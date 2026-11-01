use crate::engine::formatter::{self, OutTok};
use crate::engine::*;
use gtk::glib;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
type Job = Box<dyn FnOnce(&'static Calculator) + Send>;
#[derive(Clone)]
pub struct Svc(Rc<SvcInner>);
pub struct SvcInner {
    tx: mpsc::Sender<Job>,
    abort: Arc<AtomicBool>,
    pub busy: Arc<AtomicBool>,
    // the ui owns the options; they are pushed to engine before each job
    pub opts: RefCell<CalcOptions>,
    pushed: Cell<u64>,
    version: Cell<u64>,
}
// read-only view of the engine defs
#[derive(Clone, Default)]
pub struct Snapshot {
    pub functions: Vec<FnRef>,
    pub variables: Vec<VarRef>,
    pub units: Vec<UnitRef>,
    pub data_sets: Vec<Arc<DataSet>>,
}
impl Snapshot {
    pub fn function(&self, n: &str) -> Option<FnRef> {
        self.functions.iter().find(|f| f.item.has_name(n) && f.item.active()).cloned()
    }
    pub fn variable(&self, n: &str) -> Option<VarRef> {
        self.variables.iter().find(|v| v.item.has_name(n) && v.item.active()).cloned()
    }
    pub fn unit(&self, n: &str) -> Option<UnitRef> {
        self.units.iter().find(|u| (u.item.has_name(n) || u.plural == n) && u.item.active()).cloned()
    }
}
impl std::ops::Deref for Svc {
    type Target = SvcInner;
    fn deref(&self) -> &SvcInner {
        &self.0
    }
}
impl Svc {
    pub fn start(opts: CalcOptions, init: impl FnOnce(&'static Calculator) + Send + 'static) -> Svc {
        let (tx, rx) = mpsc::channel::<Job>();
        let busy = Arc::new(AtomicBool::new(false));
        let (abort_tx, abort_rx) = mpsc::channel();
        let b2 = busy.clone();
        std::thread::Builder::new()
            .name("OrangCalc engine".into())
            .stack_size(256 * 1024 * 1024)
            .spawn(move || {
                let c = Calculator::create();
                let _ = abort_tx.send(c.abort.clone());
                let _ = attempt_all(|| init(c));
                for job in rx {
                    b2.store(true, Ordering::Relaxed);
                    let _ = attempt_all(|| job(c));
                    b2.store(false, Ordering::Relaxed);
                }
            })
            .expect("engine thread");
        let abort = abort_rx.recv().expect("engine start");
        Svc(Rc::new(SvcInner { tx, abort, busy, opts: RefCell::new(opts), pushed: Cell::new(u64::MAX), version: Cell::new(0) }))
    }

    pub fn opts(&self) -> std::cell::Ref<'_, CalcOptions> {
        self.opts.borrow()
    }

    pub fn set_opts(&self, f: impl FnOnce(&mut CalcOptions)) {
        f(&mut self.opts.borrow_mut());
        self.version.set(self.version.get() + 1);
    }

    fn send(&self, job: Job) {
        let v = self.version.get();
        if self.pushed.get() != v {
            let o = self.opts.borrow().clone();
            let _ = self.tx.send(Box::new(move |c| {
                c.set_opts(o);
                c.apply_precision();
            }));
            self.pushed.set(v);
        }
        let _ = self.tx.send(job);
    }

    // runs f on the engine thread and hands the result to done on the main thread
    pub fn run<T: Send + 'static>(&self, f: impl FnOnce(&'static Calculator) -> T + Send + 'static, done: impl FnOnce(T) + 'static) {
        let done = glib::thread_guard::ThreadGuard::new(done);
        self.send(Box::new(move |c| {
            let r = attempt_all(|| f(c));
            glib::MainContext::default().invoke(move || {
                if let Ok(v) = r {
                    (done.into_inner())(v);
                }
            });
        }));
    }
    // fire and forget
    pub fn post(&self, f: impl FnOnce(&'static Calculator) + Send + 'static) {
        self.send(Box::new(f));
    }
    // blocks until the engine has run f; for quick queries only
    pub fn run_sync<T: Send + 'static>(&self, f: impl FnOnce(&'static Calculator) -> T + Send + 'static) -> Result<T, String> {
        let (tx, rx) = mpsc::channel();
        self.send(Box::new(move |c| {
            let _ = tx.send(attempt_all(|| f(c)));
        }));
        rx.recv().map_err(|e| e.to_string())?
    }
    pub fn snapshot(&self) -> Snapshot {
        self.run_sync(|c| {
            let d = c.defs.borrow();
            Snapshot { functions: d.functions.clone(), variables: d.variables.clone(), units: d.units.clone(), data_sets: d.data_sets.clone() }
        })
        .unwrap_or_default()
    }
    pub fn abort(&self) {
        self.abort.store(true, Ordering::Relaxed);
    }

    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }
}

// formatted result ready for display, produced on the worker thread
#[derive(Clone, Default)]
pub struct Display {
    pub input: String,
    pub parsed_text: String,
    pub tokens: Vec<OutTok>,
    pub exact_tokens: Option<Vec<OutTok>>,
    pub text: String,
    pub ascii: String,
    pub ascii_no_units: String,
    pub exact_text: String,
    pub approx: bool,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub matrix: Option<Vec<Vec<String>>>,
    pub value: Option<Expr>,
    pub assigned: Option<String>,
    pub empty: bool,
    pub comment: String,
    pub plot: Option<(String, f64, f64)>,
}
fn join(t: &[OutTok]) -> String {
    t.iter().map(|x| x.text.as_str()).collect()
}
impl Display {
    pub fn from(r: &CalcResult, c: &Calculator) -> Display {
        let mut d = Display { input: r.input.clone(), error: r.error.clone(), warnings: r.warnings.clone(), assigned: r.assigned.clone(), comment: r.comment.clone(), ..Default::default() };
        d.plot = funcs2::PLOT_REQUEST.lock().unwrap().take();
        let po = &r.print;
        if let Some(p) = &r.parsed {
            let mut pp = c.opts().clone();
            pp.output_base = 10;
            d.parsed_text = attempt(|| formatter::format(p, &pp).0).unwrap_or_else(|_| r.input.clone());
        }
        if r.error.is_some() {
            return d;
        }
        let Some(result) = &r.result else {
            d.empty = true;
            return d;
        };
        d.value = Some(funcs::unhold(result));
        let res = attempt(|| {
            let (toks, rounded) = formatter::tokens(result, po);
            d.text = join(&toks);
            d.tokens = toks;
            d.approx = rounded || result.is_approx();
            d.ascii = formatter::ascii(result, po);
            d.ascii_no_units = formatter::ascii(&evaluator::strip_units(&funcs::unhold(result)), po);
            if r.exact.is_none() && c.opts().approx == ApproxMode::Dual && formatter::has_non_terminating(result) && !result.has_symbols() {
                let mut fo = po.clone();
                fo.fraction = FractionMode::Fraction;
                let et = formatter::tokens(result, &fo).0;
                d.exact_text = join(&et);
                if d.exact_text == d.text {
                    d.exact_text.clear();
                } else {
                    d.exact_tokens = Some(et);
                    d.approx = true;
                }
            }
            if let Some(ex) = &r.exact {
                let et = formatter::tokens(ex, po).0;
                d.exact_text = join(&et);
                if d.exact_text == d.text {
                    d.exact_text.clear();
                } else {
                    d.exact_tokens = Some(et);
                }
            }
            if let E::Mat(rows, false) = &result.e {
                if rows.len() > 1 && rows.len() <= 16 && rows[0].len() <= 16 {
                    d.matrix = Some(rows.iter().map(|row| row.iter().map(|x| formatter::format(x, po).0).collect()).collect());
                }
            }
        });
        if let Err(e) = res {
            d.error = Some(e);
        }
        d
    }
}