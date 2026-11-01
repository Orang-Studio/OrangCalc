use super::algebra as simp;
use super::evaluator::Evaluator;
use super::*;
use regex::Regex;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Instant;

#[derive(Clone, Default)]
pub struct CalcResult {
    pub input: String,
    pub parsed: Option<Expr>,
    pub result: Option<Expr>,
    pub exact: Option<Expr>,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub print: CalcOptions,
    pub assigned: Option<String>,
    pub comment: String,
}

#[derive(Default)]
pub struct Defs {
    pub functions: Vec<FnRef>,
    pub variables: Vec<VarRef>,
    pub units: Vec<UnitRef>,
    pub data_sets: Vec<Arc<DataSet>>,
    fn_by_name: HashMap<String, FnRef>,
    fn_by_lower: HashMap<String, FnRef>,
    var_by_name: HashMap<String, VarRef>,
    unit_by_name: HashMap<String, UnitRef>,
}

// the calculator lives on one engine thread for the life of the program
pub struct Calculator {
    opts: RefCell<CalcOptions>,
    pub exact_phase: Cell<bool>,
    pub distribute_numbers: Cell<bool>,
    pub ev: Evaluator,
    pub abort: Arc<AtomicBool>,
    pub preview: Cell<bool>,
    pub deadline: Cell<Option<Instant>>,
    pub defs: RefCell<Defs>,
    pub answers: RefCell<Vec<Expr>>,
    pub memory: RefCell<Option<Expr>>,
    pub defs_version: Cell<u64>,
    last_exact: RefCell<Option<Expr>>,
}

thread_local! {
    static CURRENT: Cell<Option<&'static Calculator>> = const { Cell::new(None) };
}

pub fn cur() -> &'static Calculator {
    CURRENT.with(|c| c.get()).expect("no calculator on this thread")
}

pub fn try_cur() -> Option<&'static Calculator> {
    CURRENT.with(|c| c.get())
}

static ASSIGN_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)^\s*([\p{L}_][\p{L}\p{Nd}_]*)\s*(\(([^()]*)\))?\s*:=\s*(.+)$").unwrap());

impl Calculator {
    // creates the calculator for this thread; it is leaked so references can be 'static
    pub fn create() -> &'static Calculator {
        let c: &'static Calculator = Box::leak(Box::new(Calculator {
            opts: RefCell::new(CalcOptions::default()),
            exact_phase: Cell::new(true),
            distribute_numbers: Cell::new(true),
            ev: Evaluator::new(),
            abort: Arc::new(AtomicBool::new(false)),
            preview: Cell::new(false),
            deadline: Cell::new(None),
            defs: RefCell::new(Defs::default()),
            answers: RefCell::new(vec![]),
            memory: RefCell::new(None),
            defs_version: Cell::new(0),
            last_exact: RefCell::new(None),
        }));
        CURRENT.with(|x| x.set(Some(c)));
        funcs::register(c);
        constants::register(c);
        units::load(c);
        data::register(c);
        c.apply_precision();
        c
    }

    pub fn opts(&self) -> Ref<'_, CalcOptions> {
        self.opts.borrow()
    }
    pub fn opts_mut(&self) -> RefMut<'_, CalcOptions> {
        self.opts.borrow_mut()
    }
    pub fn set_opts(&self, o: CalcOptions) {
        *self.opts.borrow_mut() = o;
    }
    pub fn exact(&self) -> bool {
        self.exact_phase.get()
    }
    pub fn set_exact(&self, v: bool) {
        self.exact_phase.set(v)
    }

    // runs f with ExactPhase set to v, restoring it afterwards
    pub fn with_exact<T>(&self, v: bool, f: impl FnOnce() -> T) -> T {
        let save = self.exact();
        self.set_exact(v);
        finally(f, || self.set_exact(save))
    }

    pub fn notify_definitions_changed(&self) {
        self.defs_version.set(self.defs_version.get() + 1);
    }

    pub fn apply_precision(&self) {
        let p = self.opts().precision;
        let bits = ((p + 6) as f64 * 3.3219280948873626).ceil() as i32 + 16;
        if bits != real::bits() {
            real::BITS.store(bits, Ordering::Relaxed);
            let redo: Vec<UnitRef> = self.defs.borrow().units.iter().filter(|u| !u.is_base && !u.is_currency && u.approx_factor && u.definition.contains("pi")).cloned().collect();
            for u in redo {
                let mut nu = (*u).clone();
                if attempt(|| units::resolve(self, &mut nu)).is_ok() {
                    self.add_unit(nu);
                }
            }
        }
    }

    pub fn add_function(&self, mut f: MathFunction) -> FnRef {
        if f.item.name.is_empty() {
            f.item.name = f.item.names[0].clone();
        }
        if f.item.names.is_empty() {
            f.item.names.push(f.item.name.clone());
        }
        let old = self.defs.borrow().functions.iter().find(|x| x.name() == f.name()).cloned();
        if let Some(o) = old {
            self.remove_function(&o);
        }
        let f = Arc::new(f);
        let mut d = self.defs.borrow_mut();
        d.functions.push(f.clone());
        for n in &f.item.names {
            d.fn_by_name.insert(n.clone(), f.clone());
            d.fn_by_lower.entry(n.to_lowercase()).or_insert_with(|| f.clone());
        }
        f
    }

    pub fn remove_function(&self, f: &FnRef) {
        let mut d = self.defs.borrow_mut();
        d.functions.retain(|x| !Arc::ptr_eq(x, f));
        for n in &f.item.names {
            if d.fn_by_name.get(n).map(|x| Arc::ptr_eq(x, f)).unwrap_or(false) {
                d.fn_by_name.remove(n);
            }
            let l = n.to_lowercase();
            if d.fn_by_lower.get(&l).map(|x| Arc::ptr_eq(x, f)).unwrap_or(false) {
                d.fn_by_lower.remove(&l);
            }
        }
    }

    pub fn add_variable(&self, mut v: Variable) -> VarRef {
        if v.item.name.is_empty() {
            v.item.name = v.item.names[0].clone();
        }
        if v.item.names.is_empty() {
            v.item.names.push(v.item.name.clone());
        }
        let old = self.defs.borrow().variables.iter().find(|x| x.name() == v.name()).cloned();
        if let Some(o) = old {
            self.remove_variable(&o);
        }
        let v = Arc::new(v);
        let mut d = self.defs.borrow_mut();
        d.variables.push(v.clone());
        for n in &v.item.names {
            d.var_by_name.insert(n.clone(), v.clone());
        }
        v
    }

    pub fn remove_variable(&self, v: &VarRef) {
        let mut d = self.defs.borrow_mut();
        d.variables.retain(|x| !Arc::ptr_eq(x, v));
        for n in &v.item.names {
            if d.var_by_name.get(n).map(|x| Arc::ptr_eq(x, v)).unwrap_or(false) {
                d.var_by_name.remove(n);
            }
        }
    }

    // adds or replaces a unit, keeping its position in the list when replacing
    pub fn add_unit(&self, u: Unit) -> UnitRef {
        let u = Arc::new(u);
        let mut d = self.defs.borrow_mut();
        let pos = d.units.iter().position(|x| x.name() == u.name());
        if let Some(i) = pos {
            let old = d.units[i].clone();
            for n in old.item.names.iter().chain(std::iter::once(&old.plural)) {
                if d.unit_by_name.get(n).map(|x| Arc::ptr_eq(x, &old)).unwrap_or(false) {
                    d.unit_by_name.remove(n);
                }
            }
            d.units[i] = u.clone();
        } else {
            d.units.push(u.clone());
        }
        for n in &u.item.names {
            d.unit_by_name.insert(n.clone(), u.clone());
        }
        if !u.plural.is_empty() {
            d.unit_by_name.insert(u.plural.clone(), u.clone());
        }
        u
    }

    pub fn remove_unit(&self, u: &UnitRef) {
        let mut d = self.defs.borrow_mut();
        d.units.retain(|x| !Arc::ptr_eq(x, u));
        for n in u.item.names.iter().chain(std::iter::once(&u.plural)) {
            if d.unit_by_name.get(n).map(|x| Arc::ptr_eq(x, u)).unwrap_or(false) {
                d.unit_by_name.remove(n);
            }
        }
    }

    pub fn add_data_set(&self, ds: DataSet) {
        let mut d = self.defs.borrow_mut();
        d.data_sets.retain(|x| x.item.name != ds.item.name);
        d.data_sets.push(Arc::new(ds));
    }

    pub fn fn_(&self, name: &str) -> FnRef {
        match self.defs.borrow().fn_by_name.get(name) {
            Some(f) => f.clone(),
            None => throw_s(format!("Missing function {}", name)),
        }
    }

    pub fn find_function(&self, name: &str) -> Option<FnRef> {
        if !self.opts().functions_enabled && !name.starts_with('_') {
            return None;
        }
        let d = self.defs.borrow();
        if let Some(f) = d.fn_by_name.get(name) {
            if f.item.active() {
                return Some(f.clone());
            }
        }
        if let Some(f) = d.fn_by_lower.get(&name.to_lowercase()) {
            if f.item.active() && !f.item.is_user {
                return Some(f.clone());
            }
        }
        None
    }

    pub fn find_variable(&self, name: &str) -> Option<VarRef> {
        let d = self.defs.borrow();
        if let Some(v) = d.var_by_name.get(name) {
            if v.item.active() {
                if !self.opts().variables_enabled && !v.is_unknown && !v.name().starts_with("ans") {
                    return None;
                }
                return Some(v.clone());
            }
        }
        if name == "j" && self.opts().use_j {
            return d.var_by_name.get("i").cloned();
        }
        None
    }

    pub fn variable(&self, name: &str) -> Option<VarRef> {
        self.defs.borrow().var_by_name.get(name).cloned()
    }

    pub fn unit_named(&self, name: &str) -> Option<UnitRef> {
        self.defs.borrow().unit_by_name.get(name).cloned()
    }

    // unit lookup, also trying prefixed forms; panics with a CalcError when missing
    pub fn unit(&self, name: &str) -> UnitRef {
        self.find_unit(name, true).map(|x| x.0).unwrap_or_else(|| throw_s(format!("Missing unit {}", name)))
    }

    pub fn find_unit(&self, name: &str, allow_prefix: bool) -> Option<(UnitRef, Option<PrefixRef>)> {
        if !self.opts().units_enabled && !name.starts_with('_') {
            return None;
        }
        let name = name.replace('μ', "µ");
        let d = self.defs.borrow();
        let get = |k: &str| d.unit_by_name.get(k).filter(|u| u.item.active()).cloned();
        if let Some(u) = get(&name) {
            return Some((u, None));
        }
        if !allow_prefix {
            return None;
        }
        let nlen = name.chars().count();
        let mut by_name: Vec<&'static Prefix> = PREFIXES.iter().collect();
        by_name.sort_by_key(|p| std::cmp::Reverse(p.name.chars().count()));
        for p in &by_name {
            if nlen > p.name.chars().count() && name.starts_with(p.name) {
                let rest = &name[p.name.len()..];
                if let Some(u) = get(rest) {
                    if prefix_allowed(&u, p) && (!u.item.names.iter().any(|x| x == rest) || rest != u.abbrev()) {
                        return Some((u, Some(*p)));
                    }
                }
            }
        }
        let mut by_sym: Vec<&'static Prefix> = PREFIXES.iter().collect();
        by_sym.sort_by_key(|p| std::cmp::Reverse(p.symbol.chars().count()));
        for p in &by_sym {
            if nlen > p.symbol.chars().count() && name.starts_with(p.symbol) {
                let rest = &name[p.symbol.len()..];
                if let Some(u) = get(rest) {
                    let idx = u.item.names.iter().position(|x| x == rest);
                    if prefix_allowed(&u, p) && !u.item.names.is_empty() && (u.abbrev() == rest || (idx.map(|i| i < 2).unwrap_or(true) && rest.chars().count() <= 3)) {
                        return Some((u, Some(*p)));
                    }
                }
            }
        }
        if name.starts_with('u') && nlen > 1 {
            let rest = &name[1..];
            if let Some(u) = get(rest) {
                if u.prefixes != PrefixUse::None && u.abbrev() == rest {
                    return Some((u, prefix_by_name("micro")));
                }
            }
        }
        None
    }

    pub fn find_data_set(&self, name: &str) -> Option<Arc<DataSet>> {
        self.defs.borrow().data_sets.iter().find(|d| d.item.has_name(name)).cloned()
    }

    pub fn var_ref(&self, v: &VarRef) -> Expr {
        if v.is_unknown {
            return sym(v.name());
        }
        var_e(v.clone())
    }

    pub fn unknown_for(&self, name: &str) -> Option<VarRef> {
        self.defs.borrow().var_by_name.get(name).filter(|v| v.is_unknown).cloned()
    }

    pub fn check_abort(&self) {
        if self.abort.load(Ordering::Relaxed) {
            throw("Calculation aborted");
        }
        if let Some(d) = self.deadline.get() {
            if Instant::now() > d {
                throw("Calculation timed out");
            }
        }
    }

    fn sign_of(&self, s: &str) -> AssumeSign {
        self.unknown_for(s).map(|u| u.asign).unwrap_or(self.opts().default_sign)
    }

    fn type_of(&self, s: &str) -> AssumeType {
        self.unknown_for(s).map(|u| u.atype).unwrap_or(self.opts().default_type)
    }

    pub fn is_known_real(&self, e: &Expr) -> bool {
        match &e.e {
            E::Num(n) => n.is_real(),
            E::Sym(s) => self.type_of(s) != AssumeType::Number,
            E::Var(v) => v.name() != "i",
            E::Unit(..) => true,
            E::Add(t) => t.iter().all(|x| self.is_known_real(x)),
            E::Mul(f) => f.iter().all(|x| self.is_known_real(x)),
            E::Pow(b, x) => self.is_known_real(b) && (x.is_integer() || (self.is_known_positive(b) && self.is_known_real(x))),
            E::Call(f, a) => f.name() == "abs" || (["sin", "cos", "tan", "exp", "atan", "sinh", "cosh", "tanh"].contains(&f.name()) && a.iter().all(|x| self.is_known_real(x))),
            _ => false,
        }
    }

    pub fn is_known_positive(&self, e: &Expr) -> bool {
        match &e.e {
            E::Num(n) => n.is_real() && n.re.sign() > 0,
            E::Sym(s) => self.sign_of(s) == AssumeSign::Positive,
            E::Var(v) => ["pi", "e", "euler", "catalan", "golden", "apery"].contains(&v.name()) || (v.is_constant && v.name() != "i"),
            E::Unit(..) => true,
            E::Mul(f) => f.iter().all(|x| self.is_known_positive(x)),
            E::Add(t) => t.iter().all(|x| self.is_known_positive(x)),
            E::Pow(b, x) => self.is_known_positive(b) && self.is_known_real(x),
            E::Call(f, a) => f.name() == "exp" && self.is_known_real(&a[0]),
            _ => false,
        }
    }

    pub fn is_known_non_negative(&self, e: &Expr) -> bool {
        if self.is_known_positive(e) {
            return true;
        }
        match &e.e {
            E::Num(n) => n.is_real() && n.re.sign() >= 0,
            E::Sym(s) => matches!(self.sign_of(s), AssumeSign::Positive | AssumeSign::NonNegative),
            E::Pow(b, x) => matches!(&x.e, E::Num(n) if n.is_exact_int() && num_integer::Integer::is_even(&n.re.n)) && self.is_known_real(b),
            E::Call(f, _) => f.name() == "abs" || f.name() == "sqrt",
            E::Mul(f) => f.iter().all(|x| self.is_known_non_negative(x)),
            E::Add(t) => t.iter().all(|x| self.is_known_non_negative(x)),
            _ => false,
        }
    }

    pub fn is_known_integer(&self, e: &Expr) -> bool {
        match &e.e {
            E::Num(n) => n.is_int(),
            E::Sym(s) => matches!(self.type_of(s), AssumeType::Integer | AssumeType::Boolean),
            _ => false,
        }
    }

    // top level: assignment, conversion, substitution, evaluation, formatting choices
    pub fn calculate(&self, input: &str, timeout_ms: u64, preview: bool) -> CalcResult {
        CURRENT.with(|x| x.set(Some(cur_static(self))));
        self.abort.store(false, Ordering::Relaxed);
        self.preview.set(preview);
        self.deadline.set(if timeout_ms > 0 { Some(Instant::now() + std::time::Duration::from_millis(timeout_ms)) } else { None });
        let mut res = CalcResult { input: input.to_string(), print: self.opts().clone(), ..Default::default() };
        self.apply_precision();
        let save_exact = self.exact();
        let r = attempt_all(|| self.calculate_inner(input, preview, &mut res));
        if let Err(msg) = r {
            res.error = Some(msg);
            self.set_exact(save_exact);
            self.ev.reset();
        }
        self.deadline.set(None);
        self.preview.set(false);
        res
    }

    fn calculate_inner(&self, input: &str, preview: bool, res: &mut CalcResult) {
        let mut text = input.trim().to_string();
        if let Some(hash) = find_top_level(&text, "#") {
            res.comment = text[hash + 1..].trim().to_string();
            text = text[..hash].trim().to_string();
        }
        if text.is_empty() {
            return;
        }
        if let Some(m) = ASSIGN_RX.captures(&text) {
            if preview {
                res.result = Some(str_e(&format!("Assign {}", &m[1])));
                return;
            }
            let name = m[1].to_string();
            let args = m.get(2).map(|_| m.get(3).map(|x| x.as_str().to_string()).unwrap_or_default());
            let body = m[4].trim().to_string();
            self.do_assign(&name, args, &body, res);
            return;
        }

        let mut targets: Vec<String> = vec![];
        while let Some((idx, len)) = find_conversion(&text) {
            targets.insert(0, text[idx + len..].trim().to_string());
            text = text[..idx].trim().to_string();
        }

        let mut where_: Option<String> = None;
        if let Some(wi) = find_top_level_word(&text, "where") {
            where_ = Some(text[wi + 5..].to_string());
            text = text[..wi].to_string();
        } else if let Some(si) = find_top_level(&text, "/.") {
            where_ = Some(text[si + 2..].to_string());
            text = text[..si].to_string();
        }

        let mut warnings = vec![];
        let parsed = parser::parse_text_w(&text, self, &mut warnings);
        res.warnings.extend(warnings);
        let Some(parsed) = parsed else { return };
        res.parsed = Some(parsed.clone());

        let mut subs: Vec<(Expr, Expr)> = vec![];
        if let Some(w) = where_ {
            let sep = if self.opts().comma_decimal { ';' } else { ',' };
            for part in split_top_level(&w, sep) {
                let pe = parser::parse_req(&part, self);
                match &pe.e {
                    E::Cmp(CmpOp::Eq, l, r) => subs.push((l.clone(), r.clone())),
                    E::Logic(LogicOp::And, items) => {
                        for it in items {
                            if let E::Cmp(_, l, r) = &it.e {
                                subs.push((l.clone(), r.clone()));
                            }
                        }
                    }
                    _ => throw_s(format!("Invalid where condition: {}", part)),
                }
            }
        }

        let mut result = self.run(&parsed, &subs);
        let mut exact: Option<Expr> = None;
        for t in &targets {
            result = self.apply_target(result, t, &mut res.print, &mut exact);
        }

        if self.opts().approx == ApproxMode::Dual && exact.is_none() {
            exact = self.last_exact.borrow().clone();
        }
        if let Some(ex) = exact {
            if ex.key() != result.key() && !ex.is_approx() {
                res.exact = Some(ex);
            }
        }
        res.result = Some(result.clone());
        if !preview {
            self.store_answer(&result);
        }
    }

    // evaluation without parsing, used by calculate and tools
    pub fn run(&self, parsed: &Expr, subs: &[(Expr, Expr)]) -> Expr {
        CURRENT.with(|x| x.set(Some(cur_static(self))));
        *self.last_exact.borrow_mut() = None;
        let approx = self.opts().approx;
        self.set_exact(approx != ApproxMode::Approximate);
        let mut r = self.ev.eval(parsed);
        if !subs.is_empty() {
            for (from, to) in subs {
                let from = self.ev.eval(from);
                let to = self.ev.eval(to);
                r = self.substitute(&r, &from, &to);
            }
            r = self.ev.eval(&r);
        }
        if let E::Cmp(CmpOp::Eq, ..) = &r.e {
            let syms = symbols(&r);
            if syms.len() == 1 {
                if let Ok(s) = attempt(|| solver::solve_for_display(self, &r, &syms[0])) {
                    r = s;
                }
            }
        }
        let st = self.opts().structure;
        r = self.structure(&r, st);
        if approx != ApproxMode::Exact && self.exact() {
            if approx == ApproxMode::Dual {
                *self.last_exact.borrow_mut() = Some(r.clone());
            }
            self.set_exact(false);
            r = self.approx_pass(&r);
        }
        if approx == ApproxMode::Approximate {
            r = to_approx(&r);
        }
        r = self.auto_convert(&r);
        self.set_exact(true);
        r
    }

    fn approx_pass(&self, e: &Expr) -> Expr {
        if !e.has_symbols() {
            return self.ev.eval(e);
        }
        if let E::Pow(b, x) = &e.e {
            if b.is_var_named("e") && x.has_symbols() {
                return simp::pow(b, &self.approx_pass(x));
            }
        }
        if e.kid_count() == 0 {
            return self.ev.eval(e);
        }
        let kids: Vec<Expr> = e.kids().iter().map(|k| self.approx_pass(k)).collect();
        let rebuilt = e.rebuild(kids);
        match &rebuilt.e {
            E::Add(t) => simp::add(t.clone()),
            E::Mul(f) => simp::mul(f.clone()),
            E::Pow(b, x) => simp::pow(b, x),
            E::Call(..) => self.ev.eval_call(&rebuilt, true),
            E::Cmp(..) => self.ev.eval_cmp(&rebuilt),
            _ => rebuilt,
        }
    }

    pub fn structure(&self, r: &Expr, mode: StructMode) -> Expr {
        let mut r = r.clone();
        if mode == StructMode::Simplify || mode == StructMode::Expand {
            if r.has_symbols() || mode == StructMode::Expand {
                r = simp::expand(&r);
            }
            if r.has_symbols() {
                r = polynomials::combine_fractions(&r);
            }
        } else if mode == StructMode::Factorize {
            r = polynomials::factorize(self, &r);
        }
        r
    }

    pub fn auto_convert(&self, r: &Expr) -> Expr {
        if !r.has_units() {
            return r.clone();
        }
        let mut r = match self.opts().auto_convert {
            UnitConversion::Base => units::to_base(r),
            UnitConversion::Optimal | UnitConversion::OptimalSI => units::optimal(r),
            _ => r.clone(),
        };
        r = units::optimize_prefix(&r, false);
        if self.opts().mixed_units {
            r = units::to_mixed(&r, false);
        }
        r
    }

    pub fn substitute(&self, e: &Expr, from: &Expr, to: &Expr) -> Expr {
        e.map(&mut |x| if x.same(from) { to.clone() } else { x })
    }

    fn store_answer(&self, r: &Expr) {
        let r = funcs::unhold(r);
        let mut a = self.answers.borrow_mut();
        a.insert(0, r);
        a.truncate(20);
    }

    fn do_assign(&self, name: &str, args: Option<String>, body: &str, res: &mut CalcResult) {
        if let Some(arg_text) = args {
            let arg_names: Vec<String> = arg_text.split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
            let mut formula = body.to_string();
            for (i, a) in arg_names.iter().enumerate() {
                let ph = if i < 3 { (b'x' + i as u8) as char } else { (b'a' + i as u8 - 3) as char };
                formula = replace_word(&formula, a, &format!("\\{}", ph));
            }
            let f = useritems::make_function(self, name, &formula, &arg_names);
            self.add_function(f);
            res.assigned = Some(format!("{}({})", name, arg_names.join(", ")));
            res.result = Some(str_e(&format!("Function {}() defined", name)));
            self.notify_definitions_changed();
            return;
        }
        let parsed = parser::parse_req(body, self);
        let value = self.run(&parsed, &[]);
        let value = funcs::unhold(&value);
        let v = Variable {
            item: Item { name: name.into(), names: vec![name.into()], is_user: true, category: "User variables".into(), title: name.into(), ..Default::default() },
            value_text: formatter::plain_text(&value),
            ..Default::default()
        }
        .with_value(value.clone());
        self.add_variable(v);
        res.parsed = Some(parsed);
        res.result = Some(value);
        res.assigned = Some(name.to_string());
        self.notify_definitions_changed();
    }

    fn apply_target(&self, result: Expr, target: &str, print: &mut CalcOptions, exact: &mut Option<Expr>) -> Expr {
        let t = target.trim();
        let tl = t.to_lowercase();
        macro_rules! base {
            ($b:expr) => {{
                print.output_base = $b;
                return result;
            }};
        }
        match tl.as_str() {
            "bin" | "binary" => base!(2),
            "oct" | "octal" => base!(8),
            "dec" | "decimal" => base!(10),
            "hex" | "hexadecimal" => base!(16),
            "duo" | "duodecimal" => base!(12),
            "sexa" | "sexagesimal" | "sexa2" | "sexa3" => base!(bases::SEXAGESIMAL),
            "latitude" | "latitude2" => base!(bases::LATITUDE),
            "longitude" | "longitude2" => base!(bases::LONGITUDE),
            "roman" => base!(bases::ROMAN),
            "time" => base!(bases::TIME),
            "bijective" => base!(bases::BIJECTIVE26),
            "bcd" => base!(bases::BCD),
            "unicode" => base!(bases::UNICODE),
            "fp16" | "binary16" => base!(bases::FP16),
            "fp32" | "float" | "binary32" => base!(bases::FP32),
            "fp64" | "double" | "binary64" => base!(bases::FP64),
            "fp80" => base!(bases::FP80),
            "fp128" | "binary128" => base!(bases::FP128),
            "golden" | "φ" => base!(bases::GOLDEN),
            "supergolden" | "ψ" => base!(bases::SUPERGOLDEN),
            "pi" | "π" => base!(bases::PI_BASE),
            "e" => base!(bases::E_BASE),
            "sqrt(2)" | "√2" | "√(2)" => base!(bases::SQRT2),
            "sci" | "scientific" => {
                print.display = NumberDisplay::Scientific;
                return result;
            }
            "eng" | "engineering" => {
                print.display = NumberDisplay::Engineering;
                return result;
            }
            "simple" => {
                print.display = NumberDisplay::Simple;
                return result;
            }
            "fraction" | "frac" => {
                print.fraction = FractionMode::Fraction;
                return self.exact_again(&result, exact);
            }
            "mixed" => {
                print.fraction = FractionMode::Combined;
                return units::to_mixed(&self.exact_again(&result, exact), true);
            }
            "decimals" | "decimal fraction" => {
                print.fraction = FractionMode::Decimal;
                return result;
            }
            "percent" | "%" => {
                print.fraction = FractionMode::Percent;
                return result;
            }
            "permille" | "‰" => {
                print.fraction = FractionMode::Permille;
                return result;
            }
            "permyriad" | "‱" => {
                print.fraction = FractionMode::Permyriad;
                return result;
            }
            "factors" | "factor" | "factorize" => return polynomials::factorize(self, &self.exact_again(&result, exact)),
            "expand" => return simp::expand(&result),
            "partial fraction" | "partialfraction" | "partfrac" => return polynomials::partial_fractions(self, &self.exact_again(&result, exact)),
            "rectangular" | "cartesian" | "rect" => {
                print.complex = ComplexForm::Rectangular;
                return result;
            }
            "exponential" | "exp" => {
                print.complex = ComplexForm::Exponential;
                return result;
            }
            "polar" => {
                print.complex = ComplexForm::Polar;
                return result;
            }
            "cis" => {
                print.complex = ComplexForm::Cis;
                return result;
            }
            "angle" | "phasor" => {
                print.complex = ComplexForm::Angle;
                return result;
            }
            "optimal" => return units::optimize_prefix(&units::optimal(&result), true),
            "base" | "base units" => return units::to_base(&result),
            "prefix" => return units::optimize_prefix(&result, true),
            "utc" | "gmt" => return dates::to_utc(&result, 0),
            "calendars" => return dates::calendars_text(&result),
            "bases" => return formatter::all_bases_text(&result, print),
            "approximate" | "approx" => return to_approx(&result),
            "exact" => return self.exact_again(&result, exact),
            _ => {}
        }
        static UTC_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(utc|gmt)\s*([+-])\s*(\d{1,2})(?::?(\d{2}))?$").unwrap());
        static BASE_N: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^base\s*(-?\d+)$").unwrap());
        static BASE_X: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^base\s+(.+)$").unwrap());
        static BITS_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s*(bit|bits)$").unwrap());
        if let Some(bm) = UTC_RX.captures(&tl) {
            let h: i64 = bm[3].parse().unwrap();
            let m: i64 = bm.get(4).map(|x| x.as_str().parse().unwrap()).unwrap_or(0);
            let mut off = h * 60 + m;
            if &bm[2] == "-" {
                off = -off;
            }
            return dates::to_utc(&result, off);
        }
        if let Some(bm) = BASE_N.captures(&tl) {
            print.output_base = bm[1].parse().unwrap_or(10);
            return result;
        }
        if let Some(bm) = BASE_X.captures(&tl) {
            print.output_base = bases::CUSTOM;
            print.output_base_custom = bm[1].to_string();
            return result;
        }
        if let Some(bm) = BITS_RX.captures(&tl) {
            print.binary_bits = bm[1].parse().unwrap_or(0);
            print.output_base = 2;
            return result;
        }

        let target_expr = parser::parse_req(t, self);
        let te = self.with_exact(true, || self.ev.eval(&target_expr));
        let te = self.with_exact(false, || self.ev.eval(&te));
        self.set_exact(true);
        if te.has_units() {
            let (cf, tu) = simp::split_coef(&te);
            let mut conv = units::convert(&result, &tu);
            if !cf.is_one() {
                conv = simp::mul(vec![simp::div(&conv, &tu), num(Cx::one() / cf), tu.clone()]);
            }
            return conv;
        }
        if let (E::Num(tn), E::Num(_)) = (&te.e, &result.e) {
            if tn.is_real() && tn.re.is_exact_int() {
                return simp::div(&result, &te);
            }
        }
        throw_s(format!("Unknown conversion target \"{}\"", t))
    }

    fn exact_again(&self, result: &Expr, exact: &mut Option<Expr>) -> Expr {
        if !result.is_approx() {
            return result.clone();
        }
        if let Some(le) = self.last_exact.borrow().clone() {
            if !le.is_approx() {
                *exact = None;
                return le;
            }
        }
        rational_approx(result)
    }

    // evaluates a text expression to a number, used by tools and plotting
    pub fn eval_text(&self, text: &str) -> Expr {
        CURRENT.with(|x| x.set(Some(cur_static(self))));
        let p = parser::parse_text(text, self).unwrap_or_else(|| throw("Empty expression"));
        self.run(&p, &[])
    }
}

fn cur_static(c: &Calculator) -> &'static Calculator {
    // every Calculator is created leaked by Calculator::create
    unsafe { &*(c as *const Calculator) }
}

fn prefix_allowed(u: &Unit, p: &Prefix) -> bool {
    if u.prefixes == PrefixUse::None {
        return false;
    }
    if p.binary {
        return u.prefixes == PrefixUse::Binary;
    }
    true
}

pub fn to_approx(e: &Expr) -> Expr {
    e.map(&mut |x| match &x.e {
        E::Num(n) if !n.approx() => num(n.as_approx()),
        _ => x,
    })
}

pub fn symbols(e: &Expr) -> Vec<String> {
    let mut l: Vec<String> = vec![];
    e.any_of(&mut |x| {
        if let E::Sym(s) = &x.e {
            if !l.contains(s) {
                l.push(s.clone());
            }
        }
        false
    });
    l
}

// converts approximate numbers into nearby fractions
pub fn rational_approx(e: &Expr) -> Expr {
    e.map(&mut |x| match &x.e {
        E::Num(n) if n.is_real() && n.approx() => num_r(funcs::to_fraction(&n.re, 1000000)),
        _ => x,
    })
}

// replaces whole-word occurrences of w (not preceded by a backslash)
pub fn replace_word(s: &str, w: &str, rep: &str) -> String {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let chars: Vec<char> = s.chars().collect();
    let wc: Vec<char> = w.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if i + wc.len() <= chars.len() && chars[i..i + wc.len()] == wc[..] {
            let before_ok = i == 0 || !(is_word(chars[i - 1]) || chars[i - 1] == '\\');
            let after_ok = i + wc.len() == chars.len() || !is_word(chars[i + wc.len()]);
            if before_ok && after_ok {
                out.push_str(rep);
                i += wc.len();
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// byte index of a top level conversion arrow and its byte length
fn find_conversion(s: &str) -> Option<(usize, usize)> {
    let cs: Vec<(usize, char)> = s.char_indices().collect();
    let mut depth = 0;
    let mut q = false;
    let mut qc = '\0';
    let mut found = None;
    for k in 0..cs.len() {
        let (bi, ch) = cs[k];
        if q {
            if ch == qc {
                q = false;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            q = true;
            qc = ch;
            continue;
        }
        if ch == '(' || ch == '[' {
            depth += 1;
        } else if ch == ')' || ch == ']' {
            depth -= 1;
        }
        if depth != 0 {
            continue;
        }
        let nx = |o: usize| cs.get(k + o).map(|x| x.1);
        if ch == '→' {
            found = Some((bi, ch.len_utf8()));
        } else if ch == '-' && nx(1) == Some('>') {
            found = Some((bi, 2));
        } else if (ch == 't' || ch == 'T') && matches!(nx(1), Some('o') | Some('O')) && k > 0 && cs[k - 1].1.is_whitespace() && nx(2).map(|c| c.is_whitespace()).unwrap_or(false) {
            found = Some((bi, 2));
        }
    }
    found
}

pub fn find_top_level(s: &str, token: &str) -> Option<usize> {
    let mut depth = 0;
    let mut q = false;
    let mut qc = '\0';
    for (i, ch) in s.char_indices() {
        if q {
            if ch == qc {
                q = false;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            q = true;
            qc = ch;
            continue;
        }
        if ch == '(' || ch == '[' {
            depth += 1;
        } else if ch == ')' || ch == ']' {
            depth -= 1;
        }
        if depth == 0 && s[i..].starts_with(token) {
            return Some(i);
        }
    }
    None
}

fn find_top_level_word(s: &str, word: &str) -> Option<usize> {
    let cs: Vec<(usize, char)> = s.char_indices().collect();
    let wl = word.chars().count();
    let mut depth = 0;
    let mut k = 1;
    while k + wl <= cs.len() {
        let ch = cs[k].1;
        if ch == '(' || ch == '[' {
            depth += 1;
        } else if ch == ')' || ch == ']' {
            depth -= 1;
        }
        if depth == 0 && cs[k - 1].1.is_whitespace() {
            let piece: String = cs[k..k + wl].iter().map(|x| x.1).collect();
            if piece.eq_ignore_ascii_case(word) && (k + wl == cs.len() || cs[k + wl].1.is_whitespace()) {
                return Some(cs[k].0);
            }
        }
        k += 1;
    }
    None
}

pub fn split_top_level(s: &str, sep: char) -> Vec<String> {
    let mut res = vec![];
    let mut depth = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        if ch == '(' || ch == '[' {
            depth += 1;
        } else if ch == ')' || ch == ']' {
            depth -= 1;
        } else if ch == sep && depth == 0 {
            res.push(s[start..i].to_string());
            start = i + ch.len_utf8();
        }
    }
    res.push(s[start..].to_string());
    res.into_iter().map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}