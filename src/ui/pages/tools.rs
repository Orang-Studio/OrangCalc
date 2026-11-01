// tooling
use crate::engine::dates::{self, CalendarKind, CALENDARS, CALENDAR_NAMES};
use crate::engine::floatformats as ff;
use crate::engine::*;
use crate::ui::app::{App, Page};
use crate::ui::widgets::*;
use chrono::{Datelike, NaiveDate};
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::io::Read;
use std::rc::{Rc, Weak};
pub struct ToolsPage {
    app: Weak<App>,
    root: gtk::Box,
    stack: gtk::Stack,
}

thread_local! {
    static CURRENT: RefCell<Weak<ToolsPage>> = RefCell::new(Weak::new());
}

impl Page for ToolsPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
}

fn field(f: &mut Form, label: &str, read_only: bool) -> gtk::Entry {
    let e = gtk::Entry::new();
    e.add_css_class("oc-mono");
    e.set_editable(!read_only);
    f.add(label, &e);
    e
}

fn fmt(v: f64) -> String {
    if !v.is_finite() {
        return "-".into();
    }
    let s = format!("{:.10}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn padded(w: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    let s = scrolled(w);
    w.set_margin_top(14);
    s
}

impl ToolsPage {
    pub fn from_page(_p: &Rc<dyn Page>) -> Option<Rc<ToolsPage>> {
        CURRENT.with(|c| c.borrow().upgrade())
    }

    pub fn new(app: &Rc<App>) -> Rc<ToolsPage> {
        let root = vbox(10);
        margins(&root, 16);
        let header = label("Tools");
        header.add_css_class("oc-page-title");
        root.append(&header);
        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        let switcher = gtk::StackSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_halign(gtk::Align::Start);
        root.append(&switcher);
        root.append(&stack);
        let page = Rc::new(ToolsPage { app: Rc::downgrade(app), root, stack });
        CURRENT.with(|c| *c.borrow_mut() = Rc::downgrade(&page));
        let views: [(&str, gtk::Widget); 4] = [
            ("Percentage Calculation", page.build_percentage().upcast()),
            ("Calendar Conversion", page.build_calendar().upcast()),
            ("Floating Point Conversion (IEEE 754)", page.build_float().upcast()),
            ("Password Generator", page.build_passwords().upcast()),
        ];
        for (i, (t, w)) in views.iter().enumerate() {
            page.stack.add_titled(w, Some(&i.to_string()), t);
        }
        page
    }

    fn app(&self) -> Rc<App> {
        self.app.upgrade().expect("app")
    }

    pub fn select(&self, i: usize) {
        self.stack.set_visible_child_name(&i.to_string());
    }

    fn eval_value(&self, s: &str) -> Option<f64> {
        let s = s.trim().trim_end_matches('%').to_string();
        if s.is_empty() {
            return None;
        }
        Some(self.app().svc.run_sync(move |c| attempt(|| c.eval_text(&s)).ok().and_then(|e| e.as_real().map(|r| r.to_f64())).unwrap_or(f64::NAN)).unwrap_or(f64::NAN))
    }

    fn build_percentage(self: &Rc<Self>) -> gtk::Widget {
        let b = vbox(10);
        b.append(&dim("Enter two values, of which at most one is a percentage, and the others will be calculated for you."));
        let mut f = Form::new();
        let v1 = field(&mut f, "Value 1", false);
        let v2 = field(&mut f, "Value 2", false);
        let pct = field(&mut f, "Change from 1 to 2 (%)", false);
        f.full(&gtk::Separator::new(gtk::Orientation::Horizontal));
        let diff = field(&mut f, "Change from 1 to 2", true);
        let back = field(&mut f, "Change from 2 to 1 (%)", true);
        let cmp21 = field(&mut f, "2 compared to 1 (%)", true);
        let cmp12 = field(&mut f, "1 compared to 2 (%)", true);
        let pct_of = field(&mut f, "Value 1 is what % of value 2", true);
        f.grid.set_halign(gtk::Align::Start);
        f.grid.set_size_request(560, -1);
        b.append(&f.grid);
        let all = [v1.clone(), v2.clone(), pct.clone(), diff.clone(), back.clone(), cmp21.clone(), cmp12.clone(), pct_of.clone()];
        let w = Rc::downgrade(self);
        let calc = Rc::new(move || {
            let Some(p) = w.upgrade() else { return };
            let (mut a, mut bb, mut pc) = (p.eval_value(&v1.text()), p.eval_value(&v2.text()), p.eval_value(&pct.text()));
            match (a, bb, pc) {
                (Some(x), Some(y), _) => pc = Some((y - x) / x * 100.0),
                (Some(x), None, Some(q)) => bb = Some(x * (1.0 + q / 100.0)),
                (None, Some(y), Some(q)) => a = Some(y / (1.0 + q / 100.0)),
                _ => return,
            }
            let (x, y, q) = (a.unwrap(), bb.unwrap(), pc.unwrap());
            v1.set_text(&fmt(x));
            v2.set_text(&fmt(y));
            pct.set_text(&fmt(q));
            diff.set_text(&fmt(y - x));
            back.set_text(&fmt((x - y) / y * 100.0));
            cmp21.set_text(&fmt(y / x * 100.0));
            cmp12.set_text(&fmt(x / y * 100.0));
            pct_of.set_text(&fmt(x / y * 100.0));
        });
        for e in &all[..3] {
            let c = calc.clone();
            e.connect_activate(move |_| c());
        }
        let row = hbox(8);
        let c = calc.clone();
        let cb = button("Calculate", move || c());
        cb.add_css_class("suggested-action");
        row.append(&cb);
        row.append(&button("Clear", move || {
            for e in &all {
                e.set_text("");
            }
        }));
        b.append(&row);
        padded(&b).upcast()
    }

    fn build_calendar(self: &Rc<Self>) -> gtk::Widget {
        let outer = vbox(10);
        let g = gtk::Grid::new();
        g.set_row_spacing(6);
        g.set_column_spacing(10);
        type Row = (gtk::SpinButton, gtk::DropDown, gtk::SpinButton, gtk::Label);
        let rows: Rc<RefCell<Vec<Row>>> = Rc::new(RefCell::new(vec![]));
        let updating = Rc::new(Cell::new(false));
        let current = Rc::new(Cell::new(chrono::Local::now().date_naive()));
        let set_all: Rc<dyn Fn(NaiveDate, i32)> = {
            let (rows, updating, current) = (rows.clone(), updating.clone(), current.clone());
            Rc::new(move |gd: NaiveDate, except: i32| {
                updating.set(true);
                for (i, (d, mo, y, err)) in rows.borrow().iter().enumerate() {
                    if i as i32 == except {
                        continue;
                    }
                    match attempt(|| dates::from_gregorian(CALENDARS[i], gd)) {
                        Ok((yy, mm, dd)) => {
                            y.set_value(yy as f64);
                            mo.set_selected((mm - 1).max(0) as u32);
                            d.set_value(dd.clamp(1, 31) as f64);
                            err.set_text(&if CALENDARS[i] == CalendarKind::Chinese { dates::describe(CalendarKind::Chinese, gd).split(',').next().unwrap_or("").to_string() } else { String::new() });
                        }
                        Err(_) => err.set_text("out of range"),
                    }
                }
                updating.set(false);
                current.set(gd);
            })
        };
        for (i, name) in CALENDAR_NAMES.iter().enumerate() {
            let kind = CALENDARS[i];
            g.attach(&dim(name), 0, i as i32, 1, 1);
            let d = gtk::SpinButton::with_range(1.0, 31.0, 1.0);
            let names = dates::month_names(kind);
            let mo = combo_strings(&names, 0);
            let y = gtk::SpinButton::with_range(-9999.0, 99999.0, 1.0);
            let err = dim("");
            g.attach(&d, 1, i as i32, 1, 1);
            g.attach(&mo, 2, i as i32, 1, 1);
            g.attach(&y, 3, i as i32, 1, 1);
            g.attach(&err, 4, i as i32, 1, 1);
            let changed = {
                let (d, mo, y, err, updating, set_all) = (d.clone(), mo.clone(), y.clone(), err.clone(), updating.clone(), set_all.clone());
                Rc::new(move || {
                    if updating.get() {
                        return;
                    }
                    match attempt(|| dates::to_gregorian(kind, y.value() as i32, mo.selected() as i32 + 1, d.value() as i32)) {
                        Ok(gd) => {
                            set_all(gd, i as i32);
                            err.set_text("");
                        }
                        Err(_) => err.set_text("Calendar conversion failed"),
                    }
                })
            };
            let c = changed.clone();
            d.connect_value_changed(move |_| c());
            let c = changed.clone();
            y.connect_value_changed(move |_| c());
            let c = changed.clone();
            mo.connect_selected_notify(move |_| c());
            rows.borrow_mut().push((d, mo, y, err));
        }
        outer.append(&g);
        let bar = hbox(8);
        let sa = set_all.clone();
        bar.append(&button("Today", move || sa(chrono::Local::now().date_naive(), -1)));
        let w = Rc::downgrade(self);
        let cur = current.clone();
        let ins = button("Insert date", move || {
            if let Some(p) = w.upgrade() {
                let d = cur.get();
                p.app().insert_into_expression(&format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day()), false);
            }
        });
        ins.add_css_class("suggested-action");
        bar.append(&ins);
        outer.append(&bar);
        set_all(chrono::Local::now().date_naive(), -1);
        padded(&outer).upcast()
    }

    fn build_float(self: &Rc<Self>) -> gtk::Widget {
        let b = vbox(10);
        let mut f = Form::new();
        let names: Vec<&str> = ff::ALL.iter().map(|x| x.name).collect();
        let fmtd = combo(&names, 1);
        f.add("Format", &fmtd);
        let value = field(&mut f, "Value", false);
        let bin = field(&mut f, "Binary representation", false);
        let hex = field(&mut f, "Hexadecimal representation", false);
        let fpv = field(&mut f, "Floating point value", true);
        let err = field(&mut f, "Conversion error", true);
        let parts = field(&mut f, "Sign | exponent | significand", true);
        f.grid.set_halign(gtk::Align::Start);
        f.grid.set_size_request(640, -1);
        b.append(&f.grid);
        let busy = Rc::new(Cell::new(false));
        let w = Rc::downgrade(self);
        let from_bits = {
            let (fmtd, bin, hex, fpv, parts, busy, w) = (fmtd.clone(), bin.clone(), hex.clone(), fpv.clone(), parts.clone(), busy.clone(), w.clone());
            Rc::new(move |bits: BigInt| {
                let f = ff::ALL[fmtd.selected() as usize % ff::ALL.len()];
                busy.set(true);
                let s = ff::to_bits(&bits, f);
                bin.set_text(&s);
                hex.set_text(&ff::to_hex(&bits, f));
                let e = f.exp_bits as usize;
                parts.set_text(&if f.microchip { format!("{} | {} | {}", &s[..e], &s[e..e + 1], &s[e + 1..]) } else { format!("{} | {} | {}", &s[..1], &s[1..1 + e], &s[1 + e..]) });
                if let Some(p) = w.upgrade() {
                    let mut o = p.app().svc.opts().clone();
                    o.precision = o.precision.max(40);
                    o.fraction = FractionMode::Decimal;
                    let t = p.app().svc.run_sync(move |_| match attempt(|| ff::decode_value(&bits, f)) {
                        Ok(dv) => formatter::format(&num_r(dv), &o).0,
                        Err(m) => m,
                    });
                    fpv.set_text(&t.unwrap_or_default());
                }
                busy.set(false);
            })
        };
        let from_value = {
            let (fmtd, value, err, busy, w, from_bits) = (fmtd.clone(), value.clone(), err.clone(), busy.clone(), w.clone(), from_bits.clone());
            Rc::new(move || {
                if busy.get() {
                    return;
                }
                let text = value.text().trim().to_string();
                if text.is_empty() {
                    return;
                }
                let Some(p) = w.upgrade() else { return };
                let f = ff::ALL[fmtd.selected() as usize % ff::ALL.len()];
                let mut o = p.app().svc.opts().clone();
                o.precision = o.precision.max(20);
                let r = p.app().svc.run_sync(move |c| {
                    attempt(|| {
                        let e = c.eval_text(&text);
                        let x = e.as_real().cloned().unwrap_or_else(|| throw("Conversion error"));
                        let bits = ff::encode(&x, f);
                        let back = ff::decode_value(&bits, f);
                        let err = if x.is_infinite() { "0".to_string() } else { formatter::format(&num_r(back - x), &o).0 };
                        (bits, err)
                    })
                });
                match r {
                    Ok(Ok((bits, e))) => {
                        from_bits(bits);
                        err.set_text(&e);
                    }
                    Ok(Err(m)) | Err(m) => err.set_text(&m),
                }
            })
        };
        let fv = from_value.clone();
        value.connect_changed(move |_| fv());
        let fv = from_value.clone();
        fmtd.connect_selected_notify(move |_| fv());
        let (busy2, value2, fb, err2) = (busy.clone(), value.clone(), from_bits.clone(), err.clone());
        bin.connect_changed(move |e| {
            if busy2.get() {
                return;
            }
            let t: String = e.text().chars().filter(|c| !c.is_whitespace()).collect();
            if t.is_empty() || t.chars().any(|c| c != '0' && c != '1') {
                return;
            }
            let v = t.chars().fold(BigInt::from(0), |v, c| v * 2 + (c as i32 - '0' as i32));
            busy2.set(true);
            value2.set_text("");
            busy2.set(false);
            fb(v);
            err2.set_text("");
        });
        let (busy3, value3, fb3, err3) = (busy.clone(), value.clone(), from_bits.clone(), err.clone());
        hex.connect_changed(move |e| {
            if busy3.get() {
                return;
            }
            let mut h = e.text().trim().to_string();
            if h.to_lowercase().starts_with("0x") {
                h = h[2..].to_string();
            }
            if h.is_empty() || !h.chars().all(|c| c.is_ascii_hexdigit()) {
                return;
            }
            let Some(v) = BigInt::parse_bytes(h.as_bytes(), 16) else { return };
            busy3.set(true);
            value3.set_text("");
            busy3.set(false);
            fb3(v);
            err3.set_text("");
        });
        value.set_text("0.1");
        padded(&b).upcast()
    }

    fn build_passwords(self: &Rc<Self>) -> gtk::Widget {
        const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
        const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const DIGITS: &str = "0123456789";
        const SYMBOLS: &str = "!@#$%^&*()-_=+[]{}<>?/;:,.";
        const AMBIGUOUS: &str = "0Ooil1LI";
        let b = vbox(10);
        b.append(&dim("Generates random passwords from a cryptographically secure random source. Each password contains at least one character from every selected set."));
        let mut f = Form::new();
        let len = gtk::SpinButton::with_range(4.0, 256.0, 1.0);
        len.set_value(16.0);
        let qty = gtk::SpinButton::with_range(1.0, 20.0, 1.0);
        qty.set_value(5.0);
        f.add("Length", &len);
        f.add("Quantity", &qty);
        let sets = vbox(2);
        let mk = |t: &str| {
            let c = gtk::CheckButton::with_label(t);
            c.set_active(true);
            sets.append(&c);
            c
        };
        let lower = mk("Lowercase letters (a-z)");
        let upper = mk("Uppercase letters (A-Z)");
        let digits = mk("Digits (0-9)");
        let symbols = mk("Symbols (!@#$...)");
        let no_amb = mk("Exclude ambiguous characters (0, O, o, 1, l, I, ...)");
        f.add("Character sets", &sets);
        let output = gtk::TextView::new();
        output.set_editable(false);
        output.add_css_class("oc-mono");
        let os = scrolled(&output);
        os.set_min_content_height(110);
        f.add("Password(s)", &os);
        f.grid.set_halign(gtk::Align::Start);
        f.grid.set_size_request(560, -1);
        b.append(&f.grid);
        let entropy = dim("");
        let generate = {
            let (output, entropy) = (output.clone(), entropy.clone());
            move || {
                let mut chosen: Vec<String> = vec![];
                for (c, s) in [(&lower, LOWER), (&upper, UPPER), (&digits, DIGITS), (&symbols, SYMBOLS)] {
                    if c.is_active() {
                        chosen.push(s.into());
                    }
                }
                if no_amb.is_active() {
                    chosen = chosen.into_iter().map(|s| s.chars().filter(|c| !AMBIGUOUS.contains(*c)).collect::<String>()).filter(|s| !s.is_empty()).collect();
                }
                if chosen.is_empty() {
                    entropy.set_text("Select at least one character set.");
                    output.buffer().set_text("");
                    return;
                }
                let all: Vec<char> = chosen.concat().chars().collect();
                let n = len.value() as usize;
                let mut lines = vec![];
                for _ in 0..qty.value() as usize {
                    let mut chars: Vec<char> = vec![];
                    for s in &chosen {
                        if chars.len() < n {
                            let cs: Vec<char> = s.chars().collect();
                            chars.push(cs[secure_index(cs.len())]);
                        }
                    }
                    while chars.len() < n {
                        chars.push(all[secure_index(all.len())]);
                    }
                    for i in (1..chars.len()).rev() {
                        let j = secure_index(i + 1);
                        chars.swap(i, j);
                    }
                    lines.push(chars.into_iter().collect::<String>());
                }
                output.buffer().set_text(&lines.join("\n"));
                entropy.set_text(&format!("{} possible characters, ~{:.1} bits of entropy per password", all.len(), (all.len() as f64).log2() * n as f64));
            }
        };
        let row = hbox(8);
        let gb = button("Generate", generate);
        gb.add_css_class("suggested-action");
        row.append(&gb);
        let out = output.clone();
        row.append(&button("Copy", move || {
            let b = out.buffer();
            copy_text(&b.text(&b.start_iter(), &b.end_iter(), false));
        }));
        b.append(&row);
        b.append(&entropy);
        padded(&b).upcast()
    }
}

// uniform index from the system's secure random source
fn secure_index(n: usize) -> usize {
    if n <= 1 {
        return 0;
    }
    let threshold = (1u64 << 32) / n as u64 * n as u64;
    let mut f = std::fs::File::open("/dev/urandom").expect("/dev/urandom");
    loop {
        let mut b = [0u8; 4];
        f.read_exact(&mut b).expect("random bytes");
        let v = u32::from_le_bytes(b) as u64;
        if v < threshold {
            return (v % n as u64) as usize;
}   }   }