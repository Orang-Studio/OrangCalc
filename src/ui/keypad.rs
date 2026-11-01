// on-screen keypad: General, Programming, Algebra and Custom tabs plus the number pad
use super::calcpage::CalcPage;
use super::settings::KeypadMode;
use super::widgets::*;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

type Act = Rc<dyn Fn()>;

#[derive(Clone, Copy, PartialEq)]
pub enum Style {
    Normal,
    Muted,
    Number,
    Operator,
    Accent,
}

#[derive(Clone)]
pub struct Key {
    pub button: gtk::Button,
    label: gtk::Label,
    hint: gtk::Label,
    primary: Rc<RefCell<Option<Act>>>,
    secondary: Rc<RefCell<Option<Act>>>,
}

impl Key {
    pub fn set_label(&self, l: &str) {
        self.label.set_text(l);
    }
    pub fn set_hint(&self, h: &str) {
        self.hint.set_text(h);
        self.hint.set_visible(!h.is_empty());
    }
    pub fn set_primary(&self, a: Act) {
        *self.primary.borrow_mut() = Some(a);
    }
    pub fn set_secondary(&self, a: Act) {
        *self.secondary.borrow_mut() = Some(a);
    }
}

pub struct KeyDef<'a> {
    pub label: &'a str,
    pub tip: &'a str,
    pub primary: Option<Act>,
    pub hint: &'a str,
    pub secondary: Option<Act>,
    pub tip2: &'a str,
    pub tertiary: Option<Act>,
    pub tip3: &'a str,
    pub style: Style,
}

pub fn make_key(d: KeyDef, tooltips: bool) -> Key {
    let b = gtk::Button::new();
    b.add_css_class("oc-key");
    match d.style {
        Style::Muted => b.add_css_class("flat"),
        Style::Number => b.add_css_class("number"),
        Style::Accent => {
            b.add_css_class("accent");
            b.add_css_class("suggested-action");
        }
        _ => {}
    }
    b.set_hexpand(true);
    b.set_vexpand(true);
    let bx = vbox(0);
    let hint = gtk::Label::new(Some(d.hint));
    hint.add_css_class("oc-hint");
    hint.set_xalign(1.0);
    hint.set_visible(!d.hint.is_empty());
    let label = gtk::Label::new(Some(d.label));
    label.add_css_class("oc-label");
    label.set_vexpand(true);
    bx.append(&hint);
    bx.append(&label);
    b.set_child(Some(&bx));
    let primary = Rc::new(RefCell::new(d.primary));
    let secondary = Rc::new(RefCell::new(d.secondary));
    let tertiary = Rc::new(RefCell::new(d.tertiary));
    let p = primary.clone();
    b.connect_clicked(move |_| {
        if let Some(f) = p.borrow().clone() {
            f();
        }
    });
    let g = gtk::GestureClick::new();
    g.set_button(0);
    let (p, s, t) = (primary.clone(), secondary.clone(), tertiary.clone());
    g.connect_pressed(move |g, _, _, _| {
        let f = match g.current_button() {
            3 => s.borrow().clone().or_else(|| p.borrow().clone()),
            2 => t.borrow().clone().or_else(|| s.borrow().clone()).or_else(|| p.borrow().clone()),
            _ => None,
        };
        if let Some(f) = f {
            g.set_state(gtk::EventSequenceState::Claimed);
            f();
        }
    });
    b.add_controller(g);
    if tooltips {
        let mut parts = vec![];
        if !d.tip.is_empty() {
            parts.push(d.tip.to_string());
        }
        if secondary.borrow().is_some() && !d.tip2.is_empty() {
            parts.push(format!("Right-click: {}", d.tip2));
        }
        if tertiary.borrow().is_some() && !d.tip3.is_empty() {
            parts.push(format!("Middle-click: {}", d.tip3));
        }
        if !parts.is_empty() {
            b.set_tooltip_text(Some(&parts.join("\n")));
        }
    }
    Key { button: b, label, hint, primary, secondary }
}

pub struct Keypad {
    pub root: gtk::Box,
    host: Weak<CalcPage>,
    tabs: Vec<gtk::ToggleButton>,
    mode_holder: gtk::Box,
    numpad: gtk::Grid,
    hyp: Cell<bool>,
    hyp_keys: RefCell<Vec<Key>>,
    unit_key: RefCell<Option<Key>>,
    in_base: RefCell<Vec<(gtk::ToggleButton, i32)>>,
    out_base: RefCell<Vec<(gtk::ToggleButton, i32)>>,
    hex_keys: RefCell<Vec<gtk::Button>>,
    custom_edit: Cell<bool>,
    building: Cell<bool>,
}

fn grid() -> gtk::Grid {
    let g = gtk::Grid::new();
    g.set_row_spacing(4);
    g.set_column_spacing(4);
    g.set_row_homogeneous(true);
    g.set_column_homogeneous(true);
    g.set_hexpand(true);
    g.set_vexpand(true);
    g
}

impl Keypad {
    pub fn new(host: Weak<CalcPage>) -> Rc<Keypad> {
        let root = vbox(6);
        root.set_margin_start(12);
        root.set_margin_end(12);
        root.set_margin_bottom(12);
        let tabbar = hbox(0);
        tabbar.add_css_class("linked");
        tabbar.set_halign(gtk::Align::Start);
        let mut tabs = vec![];
        for (i, n) in ["General", "Programming", "Algebra", "Custom"].iter().enumerate() {
            let t = gtk::ToggleButton::with_label(n);
            if i > 0 {
                t.set_group(Some(&tabs[0]));
            }
            tabbar.append(&t);
            tabs.push(t);
        }
        root.append(&tabbar);
        let split = hbox(10);
        split.set_vexpand(true);
        let mode_holder = hbox(0);
        mode_holder.set_hexpand(true);
        let numpad = grid();
        split.append(&mode_holder);
        split.append(&numpad);
        root.append(&split);
        let k = Rc::new(Keypad {
            root,
            host,
            tabs,
            mode_holder,
            numpad,
            hyp: Cell::new(false),
            hyp_keys: RefCell::new(vec![]),
            unit_key: RefCell::new(None),
            in_base: RefCell::new(vec![]),
            out_base: RefCell::new(vec![]),
            hex_keys: RefCell::new(vec![]),
            custom_edit: Cell::new(false),
            building: Cell::new(false),
        });
        k.build_numpad();
        for (i, t) in k.tabs.iter().enumerate() {
            let w = Rc::downgrade(&k);
            t.connect_toggled(move |t| {
                if !t.is_active() {
                    return;
                }
                if let Some(k) = w.upgrade() {
                    if k.building.get() {
                        return;
                    }
                    if let Some(h) = k.host.upgrade() {
                        h.app().settings.borrow_mut().keypad = KeypadMode::from_index(i);
                    }
                    k.build_mode();
                }
            });
        }
        k
    }

    fn h(&self) -> Option<Rc<CalcPage>> {
        self.host.upgrade()
    }

    fn act(&self, f: impl Fn(&Rc<CalcPage>) + 'static) -> Option<Act> {
        let w = self.host.clone();
        Some(Rc::new(move || {
            if let Some(h) = w.upgrade() {
                f(&h);
            }
        }))
    }

    fn ins(&self, t: &'static str) -> Option<Act> {
        self.act(move |h| h.insert(t, 0))
    }
    fn op(&self, t: &'static str) -> Option<Act> {
        self.act(move |h| h.insert_operator(t))
    }
    fn fnc(&self, n: &'static str) -> Option<Act> {
        self.act(move |h| h.insert_function(n, ""))
    }
    fn fnx(&self, n: &'static str, extra: &'static str) -> Option<Act> {
        self.act(move |h| h.insert_function(n, extra))
    }
    fn run(&self, id: &'static str) -> Option<Act> {
        self.act(move |h| h.app().run_action(id))
    }

    fn tooltips(&self) -> bool {
        self.h().map(|h| h.app().settings.borrow().tooltips).unwrap_or(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn k(&self, label: &str, p: Option<Act>, tip: &str, hint: &str, s: Option<Act>, tip2: &str, t: Option<Act>, tip3: &str, style: Style) -> Key {
        make_key(KeyDef { label, tip, primary: p, hint, secondary: s, tip2, tertiary: t, tip3, style }, self.tooltips())
    }

    fn k1(&self, label: &str, p: Option<Act>, tip: &str) -> Key {
        self.k(label, p, tip, "", None, "", None, "", Style::Normal)
    }

    fn k2(&self, label: &str, p: Option<Act>, tip: &str, hint: &str, s: Option<Act>, tip2: &str) -> Key {
        self.k(label, p, tip, hint, s, tip2, None, "", Style::Normal)
    }

    #[allow(clippy::too_many_arguments)]
    fn k3(&self, label: &str, p: Option<Act>, tip: &str, hint: &str, s: Option<Act>, tip2: &str, t: Option<Act>, tip3: &str) -> Key {
        self.k(label, p, tip, hint, s, tip2, t, tip3, Style::Normal)
    }

    pub fn select_mode(self: &Rc<Self>, m: KeypadMode) {
        if let Some(h) = self.h() {
            h.app().settings.borrow_mut().keypad = m;
        }
        let i = m.index();
        if i <= 3 {
            self.building.set(true);
            self.tabs[i].set_active(true);
            self.building.set(false);
        }
        self.build_mode();
        self.apply_numpad_visibility();
    }

    pub fn apply_numpad_visibility(&self) {
        let Some(h) = self.h() else { return };
        let app = h.app();
        let s = app.settings.borrow();
        let mode_visible = s.keypad != KeypadMode::NumberPadOnly;
        self.mode_holder.set_visible(mode_visible);
        self.numpad.set_visible(s.numpad_visible || !mode_visible);
        if let Some(p) = self.tabs.first().and_then(|t| t.parent()) {
            p.set_visible(mode_visible);
        }
    }

    pub fn build_mode(self: &Rc<Self>) {
        while let Some(c) = self.mode_holder.first_child() {
            self.mode_holder.remove(&c);
        }
        self.hyp_keys.borrow_mut().clear();
        self.in_base.borrow_mut().clear();
        self.out_base.borrow_mut().clear();
        self.hex_keys.borrow_mut().clear();
        let mode = self.h().map(|h| h.app().settings.borrow().keypad).unwrap_or(KeypadMode::General);
        let w: gtk::Widget = match mode {
            KeypadMode::Programming => self.build_programming().upcast(),
            KeypadMode::Algebra => self.build_algebra().upcast(),
            KeypadMode::Custom => self.build_custom().upcast(),
            _ => self.build_general().upcast(),
        };
        self.mode_holder.append(&w);
        let i = mode.index();
        if i <= 3 && !self.tabs[i].is_active() {
            self.building.set(true);
            self.tabs[i].set_active(true);
            self.building.set(false);
        }
        self.refresh_base_keys();
    }

    fn add_rows(g: &gtk::Grid, rows: Vec<Vec<gtk::Widget>>) {
        for (r, row) in rows.iter().enumerate() {
            for (c, w) in row.iter().enumerate() {
                g.attach(w, c as i32, r as i32, 1, 1);
            }
        }
    }

    fn build_general(self: &Rc<Self>) -> gtk::Grid {
        let g = grid();
        let hyp = gtk::ToggleButton::with_label("hyp");
        hyp.add_css_class("oc-key");
        hyp.set_hexpand(true);
        hyp.set_vexpand(true);
        hyp.set_active(self.hyp.get());
        hyp.set_tooltip_text(Some("Hyperbolic functions"));
        let w = Rc::downgrade(self);
        hyp.connect_toggled(move |t| {
            if let Some(k) = w.upgrade() {
                k.hyp.set(t.is_active());
                k.update_hyp();
            }
        });
        let sin = self.k3("sin", None, "Sine", "asin", None, "Arcsine", self.fnc("csc"), "Cosecant");
        let cos = self.k3("cos", None, "Cosine", "acos", None, "Arccosine", self.fnc("sec"), "Secant");
        let tan = self.k3("tan", None, "Tangent", "atan", None, "Arctangent", self.fnc("cot"), "Cotangent");
        *self.hyp_keys.borrow_mut() = vec![sin.clone(), cos.clone(), tan.clone()];
        self.update_hyp();
        let b = |k: Key| k.button.upcast::<gtk::Widget>();
        let row0 = vec![
            hyp.upcast(),
            b(sin),
            b(cos),
            b(tan),
            b(self.k3("ln", self.fnc("ln"), "Natural logarithm", "log₂", self.fnc("log2"), "Base-2 logarithm", self.fnx("log", ", "), "Logarithm with base")),
            b(self.k2("log", self.fnc("log10"), "Base-10 logarithm", "logₙ", self.fnx("log", ", "), "Logarithm with base")),
        ];
        let row1 = vec![
            b(self.k3("x²", self.op("^2"), "Square", "x³", self.op("^3"), "Cube", self.op("^-1"), "Reciprocal")),
            b(self.k2("xʸ", self.op("^"), "Power", "E", self.op("E"), "Exponent ×10ⁿ")),
            b(self.k3("√", self.fnc("sqrt"), "Square root", "∛", self.fnc("cbrt"), "Cube root", self.fnx("root", ", "), "Nth root")),
            b(self.k2("eˣ", self.fnc("exp"), "Exponential", "2ˣ", self.ins("2^"), "Power of two")),
            b(self.k2("10ˣ", self.ins("10^"), "Power of ten", "E", self.op("E"), "Scientific exponent")),
            b(self.k3("x!", self.op("!"), "Factorial", "x!!", self.op("!!"), "Double factorial", self.fnx("binomial", ", "), "Binomial coefficient")),
        ];
        let row2 = vec![
            b(self.k3("π", self.ins("π"), "Pi", "γ", self.ins("euler"), "Euler–Mascheroni constant", self.ins("golden"), "Golden ratio")),
            b(self.k2("e", self.ins("e"), "Euler's number", "φ", self.ins("golden"), "Golden ratio")),
            b(self.k3("i", self.ins("i"), "Imaginary unit", "∠", self.ins("∠"), "Angle (phasor) notation", self.fnc("arg"), "Argument")),
            b(self.k3("|x|", self.fnc("abs"), "Absolute value", "sgn", self.fnc("sgn"), "Signum", self.fnc("round"), "Round")),
            b(self.k2("1/x", self.op("^-1"), "Reciprocal", "mod", self.op(" mod "), "Modulo")),
            b(self.k3("%", self.ins("%"), "Percent", "‰", self.ins("‰"), "Per mille", self.run("percentage"), "Percentage calculation tool")),
        ];
        let row3 = vec![
            b(self.k3("x", self.ins("x"), "Unknown x", "y", self.ins("y"), "Unknown y", self.ins("z"), "Unknown z")),
            b(self.k3("=", self.ins("="), "Equals (equation)", "≠", self.ins("≠"), "Not equal", self.fnc("solve"), "Solve")),
            b(self.k3("Σ", self.fnx("sum", ", 1, 10, x"), "Sum", "∏", self.fnx("product", ", 1, 10, x"), "Product", self.fnc("mean"), "Mean")),
            b(self.k2("∫", self.fnx("integrate", ", x"), "Integral", "lim", self.fnx("limit", ", 0, x"), "Limit")),
            b(self.k3("d/dx", self.fnx("diff", ", x"), "Derivative", "dsolve", self.fnc("dsolve"), "Differential equation", self.fnx("extremum", ", x"), "Extrema")),
            b(self.k3("( )", self.fnc(""), "Parenthesize selection or insert parentheses", "[ ]", self.act(|h| h.insert("[]", 1)), "Vector brackets", self.act(|h| h.open_matrix_editor()), "Matrix editor")),
        ];
        let last = self.h().map(|h| h.last_unit()).unwrap_or_else(|| "m".into());
        let unit = self.k2(&last, self.act(|h| h.insert(&h.last_unit(), 0)), "Insert last used unit", "▾", None, "Units menu");
        let ub = unit.button.clone();
        unit.set_secondary(self.act(move |h| h.show_menu(&ub, h.app().units_menu())).unwrap());
        *self.unit_key.borrow_mut() = Some(unit.clone());
        let fx = self.k3("f(x)", None, "Functions menu", "var", None, "Variables menu", self.run("newfunction"), "New function");
        let fb = fx.button.clone();
        fx.set_primary(self.act(move |h| h.show_menu(&fb, h.app().functions_menu())).unwrap());
        let fb = fx.button.clone();
        fx.set_secondary(self.act(move |h| h.show_menu(&fb, h.app().variables_menu())).unwrap());
        let to = self.k2("to", None, "Conversion menu", "→", self.ins(" to "), "Insert conversion operator");
        let tb = to.button.clone();
        to.set_primary(self.act(move |h| h.show_menu(&tb, h.app().conversion_menu())).unwrap());
        let row4 = vec![
            b(self.k2("STO", self.act(|h| h.store_result()), "Store result as variable", "new", self.run("newvariable"), "New variable")),
            b(self.k3("MR", self.act(|h| h.memory_recall()), "Memory recall", "MS", self.act(|h| h.memory_store()), "Memory store", self.act(|h| h.memory_clear()), "Memory clear")),
            b(self.k3("M+", self.act(|h| h.memory_add(false)), "Memory add", "M−", self.act(|h| h.memory_add(true)), "Memory subtract", self.act(|h| h.memory_clear()), "Memory clear")),
            b(unit),
            b(fx),
            b(to),
        ];
        Self::add_rows(&g, vec![row0, row1, row2, row3, row4]);
        g
    }

    fn update_hyp(&self) {
        let keys = self.hyp_keys.borrow();
        if keys.len() < 3 {
            return;
        }
        let hyp = self.hyp.get();
        let n = if hyp { ["sinh", "cosh", "tanh"] } else { ["sin", "cos", "tan"] };
        let inv = if hyp { ["asinh", "acosh", "atanh"] } else { ["asin", "acos", "atan"] };
        for i in 0..3 {
            let k = &keys[i];
            let (f, fi) = (n[i], inv[i]);
            k.set_label(f);
            k.set_hint(fi);
            k.set_primary(self.fnc(f).unwrap());
            k.set_secondary(self.fnc(fi).unwrap());
        }
    }

    fn build_programming(self: &Rc<Self>) -> gtk::Grid {
        let g = grid();
        let b = |k: Key| k.button.upcast::<gtk::Widget>();
        let mut row0 = vec![];
        for ch in ["A", "B", "C", "D", "E", "F"] {
            let k = self.k(ch, self.ins(ch), &format!("Hexadecimal digit {}", ch), "", None, "", None, "", Style::Number);
            self.hex_keys.borrow_mut().push(k.button.clone());
            row0.push(b(k));
        }
        let row1 = vec![
            b(self.k2("AND", self.op("&"), "Bitwise AND", "&&", self.op("&&"), "Logical AND")),
            b(self.k2("OR", self.op("|"), "Bitwise OR", "||", self.op("||"), "Logical OR")),
            b(self.k1("XOR", self.op(" xor "), "Bitwise exclusive OR")),
            b(self.k2("NOT", self.ins("~"), "Bitwise NOT", "!", self.ins("!"), "Logical NOT")),
            b(self.k1("<<", self.op("<<"), "Left shift")),
            b(self.k1(">>", self.op(">>"), "Right shift")),
        ];
        let row2 = vec![
            b(self.k2("mod", self.op(" mod "), "Modulo", "rem", self.op(" rem "), "Remainder")),
            b(self.k1("div", self.op("//"), "Integer division")),
            b(self.k1("rot", self.fnx("circshift", ", 1, 8"), "Circular shift")),
            b(self.k1("cmp", self.fnx("bitcmp", ", 8"), "Bitwise complement")),
            b(self.k2("a→1", self.fnc("code"), "Character code", "1→a", self.fnc("char"), "Character from code")),
            b(self.k2("log₂", self.fnc("log2"), "Base-2 logarithm", "pop", self.fnc("bitcount"), "Count set bits")),
        ];
        let label_key = |t: &str, tip: &str| {
            let l = gtk::Label::new(Some(t));
            l.add_css_class("dim-label");
            l.set_tooltip_text(Some(tip));
            l.upcast::<gtk::Widget>()
        };
        let mut row3 = vec![label_key("in", "Input base")];
        let mut row4 = vec![label_key("out", "Result base")];
        let mut first_in: Option<gtk::ToggleButton> = None;
        let mut first_out: Option<gtk::ToggleButton> = None;
        for (bv, name) in [(2, "BIN"), (8, "OCT"), (10, "DEC"), (16, "HEX"), (12, "DUO")] {
            for input in [true, false] {
                let t = gtk::ToggleButton::with_label(name);
                t.add_css_class("oc-key");
                t.set_hexpand(true);
                t.set_vexpand(true);
                t.set_tooltip_text(Some(&if input { format!("Interpret input in base {}", bv) } else { format!("Show results in base {}", bv) }));
                let first = if input { &mut first_in } else { &mut first_out };
                match first {
                    Some(f) => t.set_group(Some(f)),
                    None => *first = Some(t.clone()),
                }
                let w = Rc::downgrade(self);
                t.connect_toggled(move |t| {
                    if !t.is_active() {
                        return;
                    }
                    let Some(k) = w.upgrade() else { return };
                    if k.building.get() {
                        return;
                    }
                    if let Some(h) = k.h() {
                        if input {
                            h.set_input_base(bv);
                        } else {
                            h.set_output_base(bv);
                        }
                    }
                });
                if input {
                    self.in_base.borrow_mut().push((t.clone(), bv));
                    row3.push(t.upcast());
                } else {
                    self.out_base.borrow_mut().push((t.clone(), bv));
                    row4.push(t.upcast());
                }
            }
        }
        Self::add_rows(&g, vec![row0, row1, row2, row3, row4]);
        g
    }

    pub fn refresh_base_keys(&self) {
        let Some(h) = self.h() else { return };
        let app = h.app();
        let (ib, ob) = {
            let o = app.svc.opts();
            (o.input_base, o.output_base)
        };
        self.building.set(true);
        for (t, b) in self.in_base.borrow().iter() {
            t.set_active(*b == ib);
        }
        for (t, b) in self.out_base.borrow().iter() {
            t.set_active(*b == ob);
        }
        self.building.set(false);
        for k in self.hex_keys.borrow().iter() {
            k.set_sensitive(ib > 10);
        }
    }

    fn build_algebra(self: &Rc<Self>) -> gtk::Grid {
        let g = grid();
        let b = |k: Key| k.button.upcast::<gtk::Widget>();
        let row0 = ["x", "y", "z", "n", "a", "b"].iter().map(|v| b(self.k(v, self.ins(v), &format!("Variable {}", v), "", None, "", None, "", Style::Number))).collect();
        let row1 = [("=", "Equals"), ("≠", "Not equal"), ("<", "Less than"), (">", "Greater than"), ("≤", "Less than or equal"), ("≥", "Greater than or equal")].iter().map(|(l, t)| b(self.k1(l, self.ins(l), t))).collect();
        let row2 = vec![
            b(self.k2("solve", self.fnx("solve", ", x"), "Solve equation", "sys", self.fnx("multisolve", ", [x, y]"), "Solve system")),
            b(self.k1("factor", self.fnc("factor"), "Factorize")),
            b(self.k1("expand", self.fnc("expand"), "Expand")),
            b(self.k1("partfr", self.fnc("partfrac"), "Partial fractions")),
            b(self.k2("simpl", self.fnc("simplify"), "Simplify", "tog", self.fnc("together"), "Combine fractions")),
            b(self.k2("/.", self.ins(" /. "), "Substitute (where)", "where", self.ins(" where "), "Where")),
        ];
        let row3 = vec![
            b(self.k1("d/dx", self.fnx("diff", ", x"), "Derivative")),
            b(self.k2("∫", self.fnx("integrate", ", x"), "Integral", "∫ab", self.fnx("integrate", ", x, 0, 1"), "Definite integral")),
            b(self.k1("lim", self.fnx("limit", ", 0, x"), "Limit")),
            b(self.k1("Σ", self.fnx("sum", ", 1, 10, x"), "Sum")),
            b(self.k1("∏", self.fnx("product", ", 1, 10, x"), "Product")),
            b(self.k1("taylor", self.fnx("taylor", ", x, 0, 5"), "Taylor polynomial")),
        ];
        let row4 = vec![
            b(self.k2("[ ]", self.act(|h| h.insert("[]", 1)), "Vector brackets", "mat", self.act(|h| h.open_matrix_editor()), "Matrix editor")),
            b(self.k1(";", self.ins("; "), "Matrix row separator")),
            b(self.k1("det", self.fnc("det"), "Determinant")),
            b(self.k1("inv", self.fnc("inverse"), "Inverse")),
            b(self.k1("ᵀ", self.fnc("transpose"), "Transpose")),
            b(self.k2("coeff", self.fnx("coeff", ", 1, x"), "Polynomial coefficient", "deg", self.fnx("degree", ", x"), "Degree")),
        ];
        Self::add_rows(&g, vec![row0, row1, row2, row3, row4]);
        g
    }

    fn build_custom(self: &Rc<Self>) -> gtk::Box {
        let outer = vbox(6);
        outer.set_hexpand(true);
        let g = grid();
        let buttons = self.h().map(|h| h.app().settings.borrow().custom_buttons.clone()).unwrap_or_default();
        let editing = self.custom_edit.get();
        for i in 0..30usize {
            let data = buttons.get(i).filter(|d| d.len() >= 2 && !d[0].is_empty()).cloned();
            let w = Rc::downgrade(self);
            let edit: Act = Rc::new(move || {
                if let Some(k) = w.upgrade() {
                    k.edit_custom(i);
                }
            });
            let k = match data {
                Some(d) => {
                    let get = |j: usize| d.get(j).cloned().unwrap_or_default();
                    let (label, a1, a2, a3) = (get(0), get(1), get(2), get(3));
                    let mk = |a: String| -> Option<Act> {
                        if a.is_empty() {
                            return None;
                        }
                        let w = Rc::downgrade(self);
                        Some(Rc::new(move || {
                            if let Some(k) = w.upgrade() {
                                k.run_custom(i, &a);
                            }
                        }))
                    };
                    self.k3(&label, mk(a1.clone()), &a1, &a2, mk(a2.clone()), &a2, mk(a3.clone()), &a3)
                }
                None => self.k("+", Some(edit.clone()), "Add a custom button", "", None, "", None, "", Style::Muted),
            };
            if editing {
                k.set_primary(edit.clone());
                k.set_secondary(edit.clone());
            }
            g.attach(&k.button, (i % 6) as i32, (i / 6) as i32, 1, 1);
        }
        outer.append(&g);
        let w = Rc::downgrade(self);
        let edit_btn = button(if editing { "Done" } else { "Edit buttons" }, move || {
            if let Some(k) = w.upgrade() {
                k.custom_edit.set(!k.custom_edit.get());
                k.build_mode();
            }
        });
        if editing {
            edit_btn.add_css_class("suggested-action");
        }
        edit_btn.set_halign(gtk::Align::Start);
        outer.append(&edit_btn);
        outer
    }

    fn run_custom(self: &Rc<Self>, idx: usize, action: &str) {
        if self.custom_edit.get() {
            self.edit_custom(idx);
            return;
        }
        let Some(h) = self.h() else { return };
        if let Some(id) = action.strip_prefix('@') {
            h.app().run_action(id);
        } else if action.ends_with("()") && action.len() > 2 {
            h.insert_function(&action[..action.len() - 2], "");
        } else {
            h.insert(action, 0);
        }
    }

    fn edit_custom(self: &Rc<Self>, idx: usize) {
        let Some(h) = self.h() else { return };
        let app = h.app();
        let cur = app.settings.borrow().custom_buttons.get(idx).cloned().unwrap_or_else(|| vec![String::new(); 4]);
        let w = Rc::downgrade(self);
        super::dialogs::custom_button(&app, &cur, move |values| {
            let Some(k) = w.upgrade() else { return };
            let Some(h) = k.h() else { return };
            let app = h.app();
            {
                let mut s = app.settings.borrow_mut();
                while s.custom_buttons.len() <= idx {
                    s.custom_buttons.push(vec![String::new(), String::new()]);
                }
                s.custom_buttons[idx] = values.unwrap_or_else(|| vec![String::new(), String::new()]);
            }
            app.save_all();
            k.build_mode();
        });
    }

    fn build_numpad(self: &Rc<Self>) {
        let g = &self.numpad;
        let b = |k: Key| k.button.upcast::<gtk::Widget>();
        let digit = |d: &'static str, sup: &'static str, frac: &'static str| {
            let tip3 = if frac.is_empty() { String::new() } else { format!("Fraction {}", frac) };
            b(self.k(d, self.ins(d), &format!("Digit {}", d), sup, self.ins(sup), &format!("Superscript {}", sup), if frac.is_empty() { None } else { self.ins(frac) }, &tip3, Style::Number))
        };
        let opk = |label: &str, p: Option<Act>, tip: &str, hint: &str, s: Option<Act>, tip2: &str, t: Option<Act>, tip3: &str| b(self.k(label, p, tip, hint, s, tip2, t, tip3, Style::Operator));
        let row0 = vec![
            b(self.k2("(", self.ins("("), "Left parenthesis", "[", self.ins("["), "Left vector bracket")),
            b(self.k2(")", self.ins(")"), "Right parenthesis", "]", self.ins("]"), "Right vector bracket")),
            b(self.k2("←", self.act(|h| h.expr.move_caret(-1, false)), "Move cursor left", "|←", self.act(|h| h.expr.move_caret(-1, true)), "Move cursor to start")),
            b(self.k2("→", self.act(|h| h.expr.move_caret(1, false)), "Move cursor right", "→|", self.act(|h| h.expr.move_caret(1, true)), "Move cursor to end")),
            opk("⌫", self.act(|h| h.expr.backspace()), "Backspace", "del", self.act(|h| h.expr.delete_forward()), "Delete", None, ""),
        ];
        let row1 = vec![
            digit("7", "⁷", "⅐"),
            digit("8", "⁸", "⅛"),
            digit("9", "⁹", "⅑"),
            opk("÷", self.op("/"), "Division", "//", self.op("//"), "Integer division", self.op(" mod "), "Modulo"),
            opk("AC", self.act(|h| h.clear_expression()), "Clear expression", "stack", self.act(|h| h.stack_op("clear")), "Clear the RPN stack", None, ""),
        ];
        let row2 = vec![
            digit("4", "⁴", "¼"),
            digit("5", "⁵", "⅕"),
            digit("6", "⁶", "⅙"),
            opk("×", self.op("*"), "Multiplication", "&", self.op("&"), "Bitwise AND", self.op("<<"), "Left shift"),
            opk("ANS", self.ins("ans"), "Previous result", "ans₂", self.ins("ans2"), "Second previous result", None, ""),
        ];
        let row3 = vec![
            digit("1", "¹", ""),
            digit("2", "²", "½"),
            digit("3", "³", "⅓"),
            opk("−", self.op("-"), "Subtraction", "±", self.act(|h| h.negate()), "Negate / plus-minus", self.ins("±"), "Uncertainty"),
            opk("EXP", self.op("E"), "×10ⁿ exponent", "^", self.op("^"), "Power", None, ""),
        ];
        let row4 = vec![
            digit("0", "⁰", "°"),
            b(self.k(".", self.ins("."), "Decimal point", "␣", self.ins(" "), "Space", self.ins("\n"), "New line", Style::Number)),
            b(self.k(",", self.ins(", "), "Argument separator", ";", self.ins("; "), "Matrix row separator", None, "", Style::Number)),
            opk("+", self.op("+"), "Addition", "|", self.op("|"), "Bitwise OR", self.op("~"), "Bitwise NOT"),
            b(self.k("=", self.act(|h| h.calculate()), "Calculate (Enter)", "to", self.ins(" to "), "Conversion", None, "", Style::Accent)),
        ];
        Self::add_rows(g, vec![row0, row1, row2, row3, row4]);
    }

    pub fn refresh_unit_key(&self) {
        if let (Some(k), Some(h)) = (self.unit_key.borrow().as_ref(), self.h()) {
            k.set_label(&h.last_unit());
}   }   }