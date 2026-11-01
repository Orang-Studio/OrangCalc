// calculator page: expression, result, mode chips, keypad and side panel
use super::app::{mode_menu, App, Page};
use super::expression::ExpressionBox;
use super::keypad::Keypad;
use super::settings::{AfterCalc, HistoryEntry, KeypadMode};
use super::side::{BasesPanel, HistoryPanel, RpnPanel};
use super::svc::Display;
use super::widgets::*;
use super::{dialogs, theme};
use crate::engine::parser::ops;
use crate::engine::*;
use gtk::prelude::*;
use gtk::{gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
pub struct CalcPage {
    app: Weak<App>,
    root: gtk::Paned,
    column: gtk::Box,
    side: gtk::Box,
    pub expr: Rc<ExpressionBox>,
    status: gtk::Label,
    abort: gtk::Button,
    parsed: gtk::Label,
    result_holder: gtk::Box,
    result_scroll: gtk::ScrolledWindow,
    chips: RefCell<HashMap<&'static str, (gtk::Label, gtk::Label)>>,
    keys_toggle: RefCell<Option<gtk::ToggleButton>>,
    panel_toggle: RefCell<Option<gtk::ToggleButton>>,
    pub keypad: Rc<Keypad>,
    side_stack: gtk::Stack,
    pub history: Rc<HistoryPanel>,
    pub rpn: Rc<RpnPanel>,
    pub bases: Rc<BasesPanel>,
    pub stack: RefCell<Vec<Expr>>,
    last_x: RefCell<Option<Expr>>,
    pub last: RefCell<Option<Display>>,
    last_calc_text: RefCell<String>,
    just_calculated: Cell<bool>,
    preview_gen: Cell<u64>,
    calc_gen: Cell<u64>,
    layouting: Cell<bool>,
}
impl Page for CalcPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn options_changed(&self) {
        self.refresh_modes();
    }
    fn definitions_changed(&self) {
        self.refresh_snapshot();
    }
}

fn needs_parens(s: &str) -> bool {
    s.char_indices().any(|(i, c)| i > 0 && " +-*/^".contains(c))
}

impl CalcPage {
    pub fn new(app: &Rc<App>, history: Vec<HistoryEntry>) -> Rc<CalcPage> {
        let expr = ExpressionBox::new();
        let status_row = hbox(6);
        status_row.set_margin_start(14);
        status_row.set_margin_end(14);
        let status = label("");
        status.add_css_class("oc-status");
        status.set_hexpand(true);
        status.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let abort = gtk::Button::with_label("Abort");
        abort.add_css_class("flat");
        abort.set_visible(false);
        status_row.append(&status);
        status_row.append(&abort);




        let result_box = vbox(2);
        result_box.set_margin_start(14);
        result_box.set_margin_end(14);
        let parsed = gtk::Label::new(None);
        parsed.set_xalign(1.0);
        parsed.add_css_class("oc-parsed");
        parsed.set_ellipsize(gtk::pango::EllipsizeMode::Start);
        parsed.set_selectable(true);
        let result_holder = vbox(0);
        result_holder.set_valign(gtk::Align::Center);
        result_holder.set_vexpand(true);
        let result_scroll = scrolled(&result_holder);
        result_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        result_scroll.set_min_content_height(110);
        result_box.append(&parsed);
        result_box.append(&result_scroll);

        let column = vbox(6);
        column.append(&expr.frame);
        column.append(&status_row);
        column.append(&result_box);

        let side = vbox(0);
        let side_stack = gtk::Stack::new();
        side_stack.set_hhomogeneous(false);
        side_stack.set_interpolate_size(true);
        let switcher = gtk::StackSwitcher::new();
        switcher.set_stack(Some(&side_stack));
        switcher.set_halign(gtk::Align::Center);
        switcher.set_margin_top(8);
        side.append(&switcher);
        side.append(&side_stack);
        side.set_size_request(260, -1);

        let history = HistoryPanel::new(history, Rc::downgrade(app));
        let rpn = RpnPanel::new();
        let bases = BasesPanel::new();
        side_stack.add_titled(&history.root, Some("0"), "History");
        side_stack.add_titled(&rpn.root, Some("1"), "Stack");
        side_stack.add_titled(&bases.root, Some("2"), "Bases");

        let root = gtk::Paned::new(gtk::Orientation::Horizontal);
        root.set_wide_handle(true);

        let page = Rc::new_cyclic(|w: &Weak<CalcPage>| CalcPage {
            app: Rc::downgrade(app),
            root,
            column,
            side,
            expr,
            status,
            abort,
            parsed,
            result_holder,
            result_scroll,
            chips: RefCell::new(HashMap::new()),
            keys_toggle: RefCell::new(None),
            panel_toggle: RefCell::new(None),
            keypad: Keypad::new(w.clone()),
            side_stack,
            history,
            rpn,
            bases,
            stack: RefCell::new(vec![]),
            last_x: RefCell::new(None),
            last: RefCell::new(None),
            last_calc_text: RefCell::new(String::new()),
            just_calculated: Cell::new(false),
            preview_gen: Cell::new(0),
            calc_gen: Cell::new(0),
            layouting: Cell::new(false),
        });
        page.column.append(&page.build_mode_strip());
        page.column.append(&page.keypad.root);
        page.keypad.build_mode();
        page.wire();
        page.apply_settings();
        page.apply_layout();
        let tab = app.settings.borrow().side_tab.clamp(0, 2);
        page.side_stack.set_visible_child_name(&tab.to_string());
        page
    }

    pub fn app(&self) -> Rc<App> {
        self.app.upgrade().expect("app")
    }

    fn wire(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        *self.expr.on_calculate.borrow_mut() = Some(Box::new(move || {
            if let Some(p) = w.upgrade() {
                p.calculate();
            }
        }));
        let w = Rc::downgrade(self);
        *self.expr.on_rpn_operator.borrow_mut() = Some(Box::new(move |ch| {
            if let Some(p) = w.upgrade() {
                p.rpn_op(&ch.to_string());
            }
        }));
        let w = Rc::downgrade(self);
        *self.expr.rpn_active.borrow_mut() = Some(Box::new(move || w.upgrade().map(|p| p.app().settings.borrow().rpn_mode).unwrap_or(false)));
        let w = Rc::downgrade(self);
        *self.expr.history.borrow_mut() = Some(Box::new(move || {
            let Some(p) = w.upgrade() else { return vec![] };
            let mut seen = std::collections::HashSet::new();
            let v: Vec<String> = p.history.entries.borrow().iter().rev().map(|h| h.expression.clone()).filter(|e| seen.insert(e.clone())).collect();
            v
        }));
        *self.expr.snap.borrow_mut() = Some(Rc::new(RefCell::new(self.app().snap.borrow().clone())));
        let w = Rc::downgrade(self);
        self.expr.buffer.connect_changed(move |_| {
            if let Some(p) = w.upgrade() {
                p.on_expression_changed();
            }
        });
        // typing an operator right after a calculation continues from the answer
        let key = gtk::EventControllerKey::new();
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let w = Rc::downgrade(self);
        key.connect_key_pressed(move |_, k, _, m| {
            let Some(p) = w.upgrade() else { return glib::Propagation::Proceed };
            if k == gtk::gdk::Key::Escape {
                if p.expr.len() > 0 {
                    p.expr.set_text("", false);
                    return glib::Propagation::Stop;
                } else if p.app().settings.borrow().close_with_escape {
                    p.app().window.close();
                }
            }
            if m.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                return glib::Propagation::Proceed;
            }
            if let Some(ch) = k.to_unicode() {
                if p.just_calculated.get() && !p.app().settings.borrow().rpn_mode && "+-*/^×÷".contains(ch) && p.expr.all_selected() {
                    p.expr.set_text("ans", false);
                }
            }
            glib::Propagation::Proceed
        });
        self.expr.view.add_controller(key);
        let w = Rc::downgrade(self);
        self.abort.connect_clicked(move |_| {
            if let Some(p) = w.upgrade() {
                p.app().svc.abort();
            }
        });
        let w = Rc::downgrade(self);
        *self.history.on_insert_text.borrow_mut() = Some(Box::new(move |t| {
            if let Some(p) = w.upgrade() {
                p.insert(t, 0);
            }
        }));
        let w = Rc::downgrade(self);
        *self.history.on_insert_value.borrow_mut() = Some(Box::new(move |h| {
            if let Some(p) = w.upgrade() {
                p.insert_history_value(h);
            }
        }));
        let w = Rc::downgrade(self);
        *self.history.on_changed.borrow_mut() = Some(Box::new(move || {
            if let Some(p) = w.upgrade() {
                p.app().mark_dirty();
            }
        }));
        let w = Rc::downgrade(self);
        *self.rpn.on_op.borrow_mut() = Some(Box::new(move |op| {
            if let Some(p) = w.upgrade() {
                p.stack_op(op);
            }
        }));
        let w = Rc::downgrade(self);
        *self.rpn.on_edit.borrow_mut() = Some(Box::new(move |i, t| {
            if let Some(p) = w.upgrade() {
                p.edit_register(i, t);
            }
        }));
        let w = Rc::downgrade(self);
        *self.bases.on_edit.borrow_mut() = Some(Box::new(move |v| {
            if let Some(p) = w.upgrade() {
                let t = p.format_input_base(&v);
                p.expr.set_text(&t, false);
                p.calculate();
            }
        }));
        let w = Rc::downgrade(self);
        self.side_stack.connect_visible_child_name_notify(move |s| {
            let Some(p) = w.upgrade() else { return };
            let i: i32 = s.visible_child_name().and_then(|n| n.parse().ok()).unwrap_or(0);
            p.app().settings.borrow_mut().side_tab = i;
            if i == 2 {
                let v = p.last.borrow().as_ref().and_then(|d| d.value.clone());
                p.bases.set_value(v.as_ref());
            }
        });
        let w = Rc::downgrade(self);
        self.root.connect_position_notify(move |pane| {
            let Some(p) = w.upgrade() else { return };
            if p.layouting.get() || !pane.is_mapped() {
                return;
            }
            let app = p.app();
            let left = app.settings.borrow().side_left;
            let width = if left { pane.position() } else { pane.width() - pane.position() };
            if width > 100 {
                app.settings.borrow_mut().side_width = width;
            }
        });
        let w = Rc::downgrade(self);
        self.root.connect_map(move |_| {
            if let Some(p) = w.upgrade() {
                let p2 = p.clone();
                glib::idle_add_local_once(move || p2.apply_layout());
            }
        });
    }

    pub fn apply_settings(&self) {
        let app = self.app();
        let s = app.settings.borrow();
        self.expr.completion_mode.set(s.completion);
        self.expr.completion_min.set(s.completion_min_chars);
        self.expr.rpn_keys.set(s.rpn_keys);
        self.history.colorize.set(s.colorize_result);
        self.status.set_visible(s.show_status);
        drop(s);
        self.history.refill();
        self.refresh_rpn();
    }

    pub fn refresh_result(&self) {
        let d = self.last.borrow().clone();
        if let Some(d) = d.filter(|d| d.error.is_none() && !d.empty) {
            self.show_result(&d);
        }
    }

    pub fn refresh_snapshot(&self) {
        *self.expr.snap.borrow_mut() = Some(Rc::new(RefCell::new(self.app().snap.borrow().clone())));
    }

    pub fn apply_layout(&self) {
        let app = self.app();
        let s = app.settings.borrow().clone();
        self.layouting.set(true);
        let (start, end): (&gtk::Widget, &gtk::Widget) = if s.side_left { (self.side.upcast_ref(), self.column.upcast_ref()) } else { (self.column.upcast_ref(), self.side.upcast_ref()) };
        if self.root.start_child().as_ref() != Some(start) {
            self.root.set_start_child(None::<&gtk::Widget>);
            self.root.set_end_child(None::<&gtk::Widget>);
            self.root.set_start_child(Some(start));
            self.root.set_end_child(Some(end));
        }
        self.root.set_resize_start_child(!s.side_left);
        self.root.set_resize_end_child(s.side_left);
        self.root.set_shrink_start_child(false);
        self.root.set_shrink_end_child(false);
        self.side.set_visible(s.side_visible);
        let total = self.root.width();
        if s.side_visible && total > 0 {
            let sw = s.side_width.clamp(220, (total - 420).max(220));
            self.root.set_position(if s.side_left { sw } else { total - sw });
        }
        self.layouting.set(false);
        let kp = s.keypad_visible && s.keypad != KeypadMode::None;
        self.keypad.root.set_visible(kp);
        self.result_scroll.set_vexpand(!kp);
        self.keypad.apply_numpad_visibility();
        self.refresh_modes();
    }

    pub fn select_side(&self, i: usize) {
        self.app().settings.borrow_mut().side_visible = true;
        self.apply_layout();
        self.side_stack.set_visible_child_name(&i.to_string());
    }

    pub fn focus_expression(&self) {
        self.expr.focus();
    }

    pub fn expression_text(&self) -> String {
        self.expr.text()
    }

    pub fn last_ascii(&self) -> String {
        self.last.borrow().as_ref().filter(|d| d.value.is_some()).map(|d| d.ascii.clone()).unwrap_or_default()
    }

    pub fn last_unit(&self) -> String {
        let s = self.app().settings.borrow().last_unit_button.clone();
        if s.is_empty() {
            "m".into()
        } else {
            s
        }
    }

    pub fn show_menu(&self, anchor: &impl IsA<gtk::Widget>, menu: gio::Menu) {
        let pop = gtk::PopoverMenu::from_model(Some(&menu));
        pop.set_parent(anchor);
        pop.set_has_arrow(false);
        pop.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        pop.popup();
    }

    fn format_input_base(&self, v: &BigInt) -> String {
        let o = self.app().svc.opts().clone();
        let b = o.input_base;
        if b == 10 || !(2..=36).contains(&b) {
            return v.to_string();
        }
        let neg = v < &BigInt::from(0);
        let digits = if neg { (-v).to_str_radix(b as u32) } else { v.to_str_radix(b as u32) };
        let digits = if o.lower_case_hex { digits } else { digits.to_uppercase() };
        format!("{}{}", if neg { "-" } else { "" }, digits)
    }

    // mode strip with quick dropdown chips
    fn build_mode_strip(self: &Rc<Self>) -> gtk::Widget {
        let strip = gtk::FlowBox::new();
        strip.set_selection_mode(gtk::SelectionMode::None);
        strip.set_max_children_per_line(12);
        strip.set_column_spacing(6);
        strip.set_row_spacing(4);
        strip.set_margin_start(12);
        strip.set_margin_end(12);
        strip.set_homogeneous(false);
        let chip_content = |label: &gtk::Label, hint: &gtk::Label| {
            let b = hbox(4);
            b.append(label);
            hint.add_css_class("oc-hint");
            b.append(hint);
            b
        };
        for (id, tip) in [("angle", "Angle unit"), ("approx", "Approximation"), ("display", "Number display"), ("base", "Result base"), ("fraction", "Fractions and decimals"), ("mode", "Normal, RPN or chain mode")] {
            let l = gtk::Label::new(None);
            let h = gtk::Label::new(None);
            let mb = gtk::MenuButton::new();
            mb.add_css_class("oc-chip");
            mb.set_child(Some(&chip_content(&l, &h)));
            mb.set_menu_model(Some(&mode_menu(id)));
            mb.set_tooltip_text(Some(tip));
            strip.insert(&mb, -1);
            self.chips.borrow_mut().insert(id, (l, h));
        }
        let kp = gtk::ToggleButton::with_label("Keypad");
        kp.add_css_class("oc-chip");
        kp.set_tooltip_text(Some("Show or hide the keypad (right-click for keypad options)"));
        let w = Rc::downgrade(self);
        kp.connect_toggled(move |t| {
            let Some(p) = w.upgrade() else { return };
            let app = p.app();
            if app.settings.borrow().keypad_visible == t.is_active() {
                return;
            }
            {
                let mut s = app.settings.borrow_mut();
                s.keypad_visible = t.is_active();
                if s.keypad == KeypadMode::None {
                    s.keypad = KeypadMode::General;
                }
            }
            p.apply_layout();
        });
        let g = gtk::GestureClick::new();
        g.set_button(3);
        let w = Rc::downgrade(self);
        let kp2 = kp.clone();
        g.connect_pressed(move |_, _, _, _| {
            if let Some(p) = w.upgrade() {
                p.show_menu(&kp2, mode_menu("keys"));
            }
        });
        kp.add_controller(g);
        let sp = gtk::ToggleButton::with_label("Panel");
        sp.add_css_class("oc-chip");
        sp.set_tooltip_text(Some("Show or hide the side panel"));
        let w = Rc::downgrade(self);
        sp.connect_toggled(move |t| {
            let Some(p) = w.upgrade() else { return };
            let app = p.app();
            if app.settings.borrow().side_visible == t.is_active() {
                return;
            }
            app.settings.borrow_mut().side_visible = t.is_active();
            p.apply_layout();
        });
        strip.insert(&kp, -1);
        strip.insert(&sp, -1);
        *self.keys_toggle.borrow_mut() = Some(kp);
        *self.panel_toggle.borrow_mut() = Some(sp);
        self.refresh_modes();
        strip.upcast()
    }

    pub fn refresh_modes(&self) {
        let Some(app) = self.app.upgrade() else { return };
        let o = app.svc.opts().clone();
        let s = app.settings.borrow().clone();
        let chips = self.chips.borrow();
        if chips.is_empty() {
            return;
        }
        let set = |id: &str, l: &str, h: &str| {
            if let Some((a, b)) = chips.get(id) {
                a.set_text(l);
                b.set_text(h);
                b.set_visible(!h.is_empty());
            }
        };
        set(
            "angle",
            match o.angle {
                AngleMode::Degrees => "DEG",
                AngleMode::Gradians => "GRA",
                AngleMode::Radians => "RAD",
                AngleMode::None => "ANG –",
            },
            "",
        );
        set(
            "approx",
            match o.approx {
                ApproxMode::Auto => "AUTO",
                ApproxMode::Dual => "DUAL",
                ApproxMode::Exact => "EXACT",
                ApproxMode::Approximate => "APPROX",
            },
            "",
        );
        set(
            "display",
            match o.display {
                NumberDisplay::Scientific => "SCI",
                NumberDisplay::Engineering => "ENG",
                NumberDisplay::Simple => "SIMPLE",
                NumberDisplay::PureScientific => "PURE",
                NumberDisplay::Normal => "NORM",
            },
            &o.precision.to_string(),
        );
        set("base", &base_label(o.output_base), &if o.input_base != 10 { format!("in {}", base_label(o.input_base)) } else { String::new() });
        set(
            "fraction",
            match o.fraction {
                FractionMode::Fraction => "a/b",
                FractionMode::Combined => "a b/c",
                FractionMode::Percent => "%",
                FractionMode::Permille => "‰",
                FractionMode::Permyriad => "‱",
                FractionMode::Decimal => "0.1",
                FractionMode::DecimalExact => "0.1…",
            },
            "",
        );
        set("mode", if s.rpn_mode { "RPN" } else if o.parse == ParseMode::Chain { "CHAIN" } else { "NORMAL" }, "");
        if let Some(k) = self.keys_toggle.borrow().as_ref() {
            k.set_active(s.keypad_visible);
        }
        if let Some(k) = self.panel_toggle.borrow().as_ref() {
            k.set_active(s.side_visible);
        }
        self.keypad.refresh_base_keys();
    }

    fn set_status(&self, text: &str, error: bool) {
        self.status.set_text(text);
        if error {
            self.status.add_css_class("error");
        } else {
            self.status.remove_css_class("error");
        }
    }

    fn on_expression_changed(self: &Rc<Self>) {
        self.just_calculated.set(false);
        let gen = self.preview_gen.get() + 1;
        self.preview_gen.set(gen);
        let app = self.app();
        let s = app.settings.borrow();
        if !s.calc_as_you_type || s.rpn_mode {
            return;
        }
        let mut delay = s.as_you_type_delay;
        if s.adaptive_delay && self.expr.len() < 12 {
            delay = delay.min(120);
        }
        drop(s);
        let w = Rc::downgrade(self);
        glib::timeout_add_local_once(std::time::Duration::from_millis(delay.max(30) as u64), move || {
            if let Some(p) = w.upgrade() {
                if p.preview_gen.get() == gen {
                    p.run_preview(gen);
                }
            }
        });
    }

    fn run_preview(self: &Rc<Self>, gen: u64) {
        let text = self.expr.text().trim().to_string();
        if text.is_empty() || (text == *self.last_calc_text.borrow() && self.just_calculated.get()) {
            self.set_status("", false);
            return;
        }
        let app = self.app();
        if app.svc.is_busy() {
            app.svc.abort();
        }
        let w = Rc::downgrade(self);
        let t2 = text.clone();
        app.svc.run(
            move |c| Display::from(&c.calculate(&t2, 1500, true), c),
            move |d| {
                let Some(p) = w.upgrade() else { return };
                if p.preview_gen.get() != gen {
                    return;
                }
                if let Some(e) = &d.error {
                    p.set_status(&if d.parsed_text.is_empty() { e.clone() } else { format!("{}   {}", d.parsed_text, e) }, false);
                } else if d.empty {
                    p.set_status("", false);
                } else {
                    let body: String = if d.text.chars().count() > 200 { format!("{}…", d.text.chars().take(200).collect::<String>()) } else { d.text.clone() };
                    let res = format!("{}{}", if d.approx { "≈ " } else { "= " }, body);
                    let parsed = &d.parsed_text;
                    p.set_status(&if !parsed.is_empty() && *parsed != text { format!("{}   {}", parsed, res) } else { res }, false);
                }
            },
        );
    }

    pub fn calculate(self: &Rc<Self>) {
        let t = self.expr.text();
        self.calculate_text(&t, true);
    }

    pub fn calculate_text(self: &Rc<Self>, input: &str, add_history: bool) {
        let text = input.trim().to_string();
        self.preview_gen.set(self.preview_gen.get() + 1);
        if self.app().settings.borrow().rpn_mode {
            self.rpn_enter();
            return;
        }
        if text.is_empty() {
            return;
        }
        *self.last_calc_text.borrow_mut() = text.clone();
        let gen = self.calc_gen.get() + 1;
        self.calc_gen.set(gen);
        let w = Rc::downgrade(self);
        glib::timeout_add_local_once(std::time::Duration::from_millis(250), move || {
            if let Some(p) = w.upgrade() {
                if p.calc_gen.get() == gen && p.app().svc.is_busy() {
                    p.show_busy();
                    p.abort.set_visible(true);
                }
            }
        });
        let w = Rc::downgrade(self);
        let t2 = text.clone();
        self.app().svc.run(
            move |c| Display::from(&c.calculate(&t2, 0, false), c),
            move |d| {
                let Some(p) = w.upgrade() else { return };
                p.calc_gen.set(p.calc_gen.get() + 1);
                p.abort.set_visible(false);
                p.show_display(d, add_history);
            },
        );
    }

    fn clear_result(&self) {
        while let Some(c) = self.result_holder.first_child() {
            self.result_holder.remove(&c);
        }
    }

    fn show_busy(&self) {
        self.clear_result();
        let l = gtk::Label::new(Some("Calculating…"));
        l.add_css_class("dim-label");
        l.set_xalign(1.0);
        self.result_holder.append(&l);
    }

    fn show_error(&self, parsed: &str, error: &str) {
        self.parsed.set_text(parsed);
        self.clear_result();
        let l = gtk::Label::new(Some(error));
        l.add_css_class("oc-error");
        l.set_xalign(1.0);
        l.set_wrap(true);
        l.set_selectable(true);
        self.result_holder.append(&l);
    }

    fn show_result(&self, d: &Display) {
        self.parsed.set_text(&d.parsed_text);
        self.clear_result();
        if let Some(m) = &d.matrix {
            let row = hbox(6);
            row.set_halign(gtk::Align::End);
            let g = gtk::Grid::new();
            g.set_column_spacing(18);
            g.set_row_spacing(4);
            g.add_css_class("oc-matrix");
            for (r, cells) in m.iter().enumerate() {
                for (c, s) in cells.iter().enumerate() {
                    let l = gtk::Label::new(Some(s));
                    l.set_selectable(true);
                    g.attach(&l, c as i32, r as i32, 1, 1);
                }
            }
            let bracket = |t: &str| {
                let l = gtk::Label::new(None);
                l.set_markup(&format!("<span size=\"{}%\" alpha=\"60%\">{}</span>", 100 + 90 * m.len().min(6), t));
                l
            };
            row.append(&bracket("["));
            row.append(&g);
            row.append(&bracket("]"));
            self.result_holder.append(&row);
            return;
        }
        let colorize = self.app().settings.borrow().colorize_result;
        let mut lines = vec![];
        if let Some(ex) = &d.exact_tokens {
            lines.push(("= ", theme::markup_tokens(ex, colorize), ex.iter().map(|t| t.text.chars().count()).sum::<usize>()));
            lines.push(("≈ ", theme::markup_tokens(&d.tokens, colorize), d.text.chars().count()));
        } else {
            lines.push((if d.approx { "≈ " } else { "= " }, theme::markup_tokens(&d.tokens, colorize), d.text.chars().count()));
        }
        let total: usize = lines.iter().map(|l| l.2).sum();
        for (prefix, markup, _) in lines {
            let l = gtk::Label::new(None);
            l.set_markup(&format!("<span alpha=\"55%\">{}</span>{}", prefix, markup));
            l.set_xalign(1.0);
            l.set_wrap(true);
            l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            l.set_selectable(true);
            l.add_css_class(if total > 36 { "oc-result-small" } else { "oc-result" });
            self.result_holder.append(&l);
        }
    }

    fn show_display(self: &Rc<Self>, d: Display, add_history: bool) {
        let app = self.app();
        if let Some(e) = d.error.clone() {
            self.show_error(&d.parsed_text, &e);
            self.set_status(&e, true);
            if add_history {
                self.history.add(HistoryEntry { expression: d.input.clone(), parsed: d.parsed_text.clone(), result: e, is_error: true, ..Default::default() });
            }
            *self.last.borrow_mut() = Some(d);
            app.mark_dirty();
            return;
        }
        if d.empty {
            self.clear_result();
            self.parsed.set_text("");
            self.set_status("", false);
            *self.last.borrow_mut() = Some(d);
            return;
        }
        self.show_result(&d);
        if !d.warnings.is_empty() {
            self.set_status(&d.warnings.join("; "), true);
        } else {
            self.set_status(&d.comment, false);
        }
        let is_msg = d.value.as_ref().map(|v| v.is_str()).unwrap_or(false);
        if add_history {
            self.history.add(HistoryEntry {
                expression: d.input.clone(),
                parsed: d.parsed_text.clone(),
                result: d.text.clone(),
                result_ascii: d.ascii.clone(),
                exact_text: d.exact_text.clone(),
                tokens: d.tokens.clone(),
                approx: d.approx,
                comment: d.comment.clone(),
                is_message: is_msg,
                ..Default::default()
            });
        }
        let s = app.settings.borrow().clone();
        if self.side_stack.visible_child_name().as_deref() == Some("2") || s.keypad == KeypadMode::Programming {
            self.bases.set_value(d.value.as_ref());
        }
        let plot = d.plot.clone();
        let assigned = d.assigned.is_some();
        app.update_title(&d.text);
        let (ascii, input) = (d.ascii.clone(), d.input.clone());
        *self.last.borrow_mut() = Some(d);
        if s.auto_copy_result {
            self.copy_result(false);
        }
        if assigned {
            app.definitions_changed();
        }
        app.mark_dirty();
        match s.expression_after {
            AfterCalc::Clear => self.expr.set_text("", false),
            AfterCalc::ReplaceWithResult if !is_msg => self.expr.set_text(&ascii, true),
            AfterCalc::ReplaceIfShorter if !is_msg && ascii.chars().count() < input.chars().count() => self.expr.set_text(&ascii, true),
            _ => self.expr.select_all(),
        }
        self.just_calculated.set(true);
        self.preview_gen.set(self.preview_gen.get() + 1);
        self.expr.focus();
        if let Some((e, a, b)) = plot {
            app.open_plot(&e, a, b);
        }
    }

    fn insert_history_value(&self, h: &HistoryEntry) {
        if h.is_error || h.is_message {
            self.insert(&h.expression, 0);
            return;
        }
        let mut t = if h.result_ascii.is_empty() { h.result.clone() } else { h.result_ascii.clone() };
        if self.expr.len() > 0 && needs_parens(&t) {
            t = format!("({})", t);
        }
        self.insert(&t, 0);
    }

    pub fn copy_result(&self, ascii: bool) {
        let last = self.last.borrow();
        let Some(d) = last.as_ref().filter(|d| d.error.is_none() && !d.empty) else { return };
        let s = self.app().settings.borrow().clone();
        let t = if ascii || s.copy_ascii_default {
            if s.copy_ascii_no_units {
                d.ascii_no_units.clone()
            } else {
                d.ascii.clone()
            }
        } else {
            d.text.clone()
        };
        copy_text(&t);
        self.set_status(&format!("Copied {}", t), false);
    }

    pub fn convert_to(self: &Rc<Self>, target: &str) {
        let mut text = self.expr.text().trim().to_string();
        if text.is_empty() {
            if self.last.borrow().is_none() {
                return;
            }
            text = "ans".into();
        }
        if let Some(to) = calculator::find_top_level(&text, " to ") {
            if to > 0 && self.just_calculated.get() {
                text = text[..to].to_string();
            }
        }
        self.expr.set_text(&format!("{} to {}", text, target), false);
        self.calculate();
    }

    pub fn insert(&self, text: &str, caret_back: i32) {
        let keep = self.app().settings.borrow().expression_after == AfterCalc::Keep;
        if self.just_calculated.get() && self.expr.all_selected() && keep {
            let first = text.chars().next();
            if matches!(first, Some(c) if c.is_alphanumeric() || c == '(' || c == '.') {
                self.expr.set_text("", false);
            } else {
                self.expr.set_cursor(self.expr.len());
            }
        }
        self.just_calculated.set(false);
        self.expr.insert(text, caret_back);
    }

    pub fn insert_operator(self: &Rc<Self>, op: &str) {
        let rpn_ops = ["+", "-", "*", "/", "^", "^2", "^3", "^-1", "!", "&", "|", "<<", ">>", " mod ", "//", " xor "];
        if self.app().settings.borrow().rpn_mode && rpn_ops.contains(&op) {
            self.rpn_op(op);
            return;
        }
        if self.just_calculated.get() && self.expr.all_selected() {
            self.expr.set_text("ans", false);
        }
        self.just_calculated.set(false);
        self.expr.insert(op, 0);
    }

    pub fn insert_function(self: &Rc<Self>, name: &str, extra: &str) {
        let app = self.app();
        if app.settings.borrow().rpn_mode && !name.is_empty() && !self.stack.borrow().is_empty() && self.expr.len() == 0 {
            self.rpn_function(name);
            return;
        }
        let wrap_all = self.just_calculated.get() && self.expr.all_selected();
        self.just_calculated.set(false);
        self.expr.insert_function(name, wrap_all, extra);
        if !name.is_empty() {
            App::add_recent(&mut app.settings.borrow_mut().recent_functions, name);
        }
    }

    pub fn clear_expression(&self) {
        self.expr.set_text("", false);
        self.clear_result();
        self.parsed.set_text("");
        self.set_status("", false);
        self.expr.focus();
    }

    pub fn negate(self: &Rc<Self>) {
        if self.app().settings.borrow().rpn_mode && self.expr.len() == 0 && !self.stack.borrow().is_empty() {
            self.stack_op("negate");
            return;
        }
        let t = self.expr.text();
        if t.starts_with("-(") && t.ends_with(')') {
            self.expr.set_text(&t[2..t.len() - 1], false);
        } else if !t.is_empty() {
            self.expr.set_text(&format!("-({})", t), false);
        } else {
            self.expr.insert("-", 0);
        }
        self.expr.focus();
    }

    fn current_value(&self) -> Option<Expr> {
        self.last.borrow().as_ref().and_then(|d| d.value.clone())
    }

    pub fn memory_store(&self) {
        let Some(v) = self.current_value() else { return };
        self.app().svc.post(move |c| *c.memory.borrow_mut() = Some(v));
        self.set_status("Stored in memory", false);
    }

    pub fn memory_recall(&self) {
        self.insert("MR", 0);
    }

    pub fn memory_clear(&self) {
        self.app().svc.post(|c| *c.memory.borrow_mut() = None);
        self.set_status("Memory cleared", false);
    }

    pub fn memory_add(&self, subtract: bool) {
        let Some(v) = self.current_value() else { return };
        self.app().svc.post(move |c| {
            let cur = c.memory.borrow().clone().unwrap_or_else(zero);
            let r = c.run(&if subtract { ops::sub(cur, v) } else { ops::add(cur, v) }, &[]);
            *c.memory.borrow_mut() = Some(r);
        });
        self.set_status(if subtract { "Subtracted from memory" } else { "Added to memory" }, false);
    }

    pub fn store_result(self: &Rc<Self>) {
        let app = self.app();
        let Some(v) = self.current_value() else {
            app.run_action("newvariable");
            return;
        };
        let w = Rc::downgrade(self);
        ask(app.parent(), "Store Result", "Variable name:", "", move |name| {
            let Some(p) = w.upgrade() else { return };
            let name = name.trim().to_string();
            if name.is_empty() {
                return;
            }
            let app = p.app();
            let mut cs = name.chars();
            if !cs.next().map(parser::is_ident_start).unwrap_or(false) || !name.chars().all(parser::is_ident_part) {
                message(app.parent(), "Invalid variable name.");
                return;
            }
            let v = v.clone();
            let n2 = name.clone();
            let w2 = Rc::downgrade(&p);
            app.svc.run(
                move |c| {
                    let text = formatter::ascii(&v, &c.opts());
                    c.add_variable(Variable { item: Item { name: n2.clone(), names: vec![n2.clone()], is_user: true, category: "User variables".into(), title: n2, ..Default::default() }, value_text: text, ..Default::default() }.with_value(v));
                },
                move |_| {
                    if let Some(p) = w2.upgrade() {
                        p.app().definitions_changed();
                        p.set_status(&format!("Stored as {}", name), false);
                    }
                },
            );
        });
    }

    pub fn set_input_base(self: &Rc<Self>, b: i32) {
        let app = self.app();
        app.svc.set_opts(|o| o.input_base = b);
        app.options_changed();
    }

    pub fn set_output_base(self: &Rc<Self>, b: i32) {
        let app = self.app();
        app.svc.set_opts(|o| o.output_base = b);
        app.options_changed();
        if self.last.borrow().is_some() && self.expr.len() > 0 {
            let t = self.last_calc_text.borrow().clone();
            let t = if t.is_empty() { self.expr.text() } else { t };
            self.calculate_text(&t, false);
        }
    }

    pub fn open_matrix_editor(self: &Rc<Self>) {
        let cur = self.current_value().filter(|v| matches!(&v.e, E::Mat(..)));
        let w = Rc::downgrade(self);
        dialogs::matrix_edit(&self.app(), cur, move |text| {
            if let Some(p) = w.upgrade() {
                p.insert(&text, 0);
            }
        });
    }

    pub fn insert_date(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        dialogs::date_dialog(self.app().parent(), "Insert Date", chrono::Local::now().naive_local(), true, move |d| {
            if let Some(p) = w.upgrade() {
                use chrono::Timelike;
                let t = if d.hour() == 0 && d.minute() == 0 && d.second() == 0 { d.format("%Y-%m-%d").to_string() } else { d.format("%Y-%m-%dT%H:%M:%S").to_string() };
                p.insert(&t, 0);
            }
        });
    }

    pub fn refresh_rpn(&self) {
        let stack = self.stack.borrow().clone();
        let o = self.app().svc.opts().clone();
        let rpn = self.rpn.clone();
        self.app().svc.run(move |_| stack.iter().map(|e| attempt(|| formatter::format(e, &o).0).unwrap_or_else(|_| "?".into())).collect::<Vec<_>>(), move |t| rpn.set_texts(t));
    }

    // rpn stack operations
    fn rpn_enter(self: &Rc<Self>) {
        let text = self.expr.text().trim().to_string();
        if text.is_empty() {
            if !self.stack.borrow().is_empty() {
                self.stack_op("copy");
            }
            return;
        }
        let w = Rc::downgrade(self);
        self.app().svc.run(
            move |c| Display::from(&c.calculate(&text, 0, false), c),
            move |d| {
                let Some(p) = w.upgrade() else { return };
                if let Some(e) = &d.error {
                    p.set_status(e, true);
                    p.show_error(&d.parsed_text, e);
                    return;
                }
                if let Some(v) = &d.value {
                    p.stack.borrow_mut().push(v.clone());
                    p.refresh_rpn();
                }
                p.show_display(d, true);
                p.expr.set_text("", false);
                p.select_side(1);
            },
        );
    }

    pub fn rpn_op(self: &Rc<Self>, op: &str) {
        let pending = self.expr.text().trim().to_string();
        let st = self.stack.borrow().clone();
        let op = op.to_string();
        let w = Rc::downgrade(self);
        self.app().svc.run(
            move |c| -> (Vec<Expr>, Option<Expr>, Display) {
                let mut st = st;
                let fail = |st: Vec<Expr>, m: &str| (st, None, Display { error: Some(m.to_string()), ..Default::default() });
                if !pending.is_empty() {
                    let r = c.calculate(&pending, 0, false);
                    if r.error.is_some() {
                        return (st, None, Display::from(&r, c));
                    }
                    if let Some(v) = &r.result {
                        st.push(funcs::unhold(v));
                    }
                }
                let needs = if ["^2", "^3", "^-1", "!"].contains(&op.as_str()) { 1 } else { 2 };
                if st.len() < needs {
                    return fail(st, "Not enough values on the RPN stack");
                }
                let orig = st.clone();
                let b = st.pop().unwrap();
                let raw = if needs == 1 {
                    if op == "!" {
                        call_e(c.fn_("factorial"), vec![b.clone()])
                    } else {
                        ops::pow(b.clone(), match op.as_str() {
                            "^2" => two(),
                            "^3" => num_i(3),
                            _ => minus_one(),
                        })
                    }
                } else {
                    let a = st.pop().unwrap();
                    let f = |n: &str, args: Vec<Expr>| call_e(c.fn_(n), args);
                    match op.as_str() {
                        "-" => ops::sub(a, b.clone()),
                        "*" => ops::mul(a, b.clone()),
                        "/" => ops::div(a, b.clone()),
                        "^" => ops::pow(a, b.clone()),
                        "&" => f("bitand", vec![a, b.clone()]),
                        "|" => f("bitor", vec![a, b.clone()]),
                        " xor " => f("xor", vec![a, b.clone()]),
                        "<<" => f("shift", vec![a, b.clone()]),
                        ">>" => f("shift", vec![a, ops::neg(b.clone())]),
                        " mod " => f("mod", vec![a, b.clone()]),
                        "//" => f("div", vec![a, b.clone()]),
                        _ => ops::add(a, b.clone()),
                    }
                };
                match attempt(|| c.run(&raw, &[])) {
                    Ok(res) => {
                        let res = funcs::unhold(&res);
                        st.push(res.clone());
                        c.answers.borrow_mut().insert(0, res.clone());
                        let cr = CalcResult { input: format!("RPN {}", op.trim()), result: Some(res), print: c.opts().clone(), ..Default::default() };
                        (st, Some(b), Display::from(&cr, c))
                    }
                    Err(e) => fail(orig, &e),
                }
            },
            move |(st, lastx, d)| {
                let Some(p) = w.upgrade() else { return };
                p.expr.set_text("", false);
                *p.stack.borrow_mut() = st;
                if lastx.is_some() {
                    *p.last_x.borrow_mut() = lastx;
                }
                p.refresh_rpn();
                if let Some(e) = &d.error {
                    p.set_status(e, true);
                    return;
                }
                p.show_result(&d);
                *p.last.borrow_mut() = Some(d);
                p.select_side(1);
            },
        );
    }

    fn rpn_function(self: &Rc<Self>, name: &str) {
        let Some(f) = self.app().snap.borrow().function(name) else { return };
        let st = self.stack.borrow().clone();
        let name = name.to_string();
        let w = Rc::downgrade(self);
        self.app().svc.run(
            move |c| -> (Vec<Expr>, Display) {
                let mut st = st;
                let n = (f.min_args.min(st.len() as i32)).max(1) as usize;
                if st.len() < n {
                    return (st, Display { error: Some("Not enough values on the RPN stack".into()), ..Default::default() });
                }
                let args = st.split_off(st.len() - n);
                match attempt(|| funcs::unhold(&c.run(&call_e(f.clone(), args.clone()), &[]))) {
                    Ok(res) => {
                        st.push(res.clone());
                        let cr = CalcResult { input: name.clone(), result: Some(res), print: c.opts().clone(), ..Default::default() };
                        (st, Display::from(&cr, c))
                    }
                    Err(e) => {
                        st.extend(args);
                        (st, Display { error: Some(e), ..Default::default() })
                    }
                }
            },
            move |(st, d)| {
                let Some(p) = w.upgrade() else { return };
                *p.stack.borrow_mut() = st;
                p.refresh_rpn();
                if let Some(e) = &d.error {
                    p.set_status(e, true);
                    return;
                }
                p.show_result(&d);
                *p.last.borrow_mut() = Some(d);
            },
        );
    }

    pub fn stack_op(self: &Rc<Self>, op: &str) {
        let sel = self.rpn.selected_register();
        {
            let mut stack = self.stack.borrow_mut();
            let n = stack.len();
            let idx = |r: i32| (n as i32 - 1 - r) as usize;
            match op {
                "clear" => stack.clear(),
                "copy" => {
                    if n == 0 {
                        return;
                    }
                    let v = if sel >= 0 { stack[idx(sel)].clone() } else { stack[n - 1].clone() };
                    stack.push(v);
                }
                "delete" => {
                    if n == 0 {
                        return;
                    }
                    stack.remove(if sel >= 0 { idx(sel) } else { n - 1 });
                }
                "swap" => {
                    if n < 2 {
                        return;
                    }
                    if sel > 0 {
                        let t = stack.remove(idx(sel));
                        stack.push(t);
                    } else {
                        stack.swap(n - 1, n - 2);
                    }
                }
                "up" => {
                    if n < 2 {
                        return;
                    }
                    if sel >= 0 && (sel as usize) < n - 1 {
                        let i = idx(sel);
                        stack.swap(i, i - 1);
                    } else {
                        let t = stack.remove(0);
                        stack.push(t);
                    }
                }
                "down" => {
                    if n < 2 {
                        return;
                    }
                    if sel > 0 {
                        let i = idx(sel);
                        stack.swap(i, i + 1);
                    } else {
                        let t = stack.pop().unwrap();
                        stack.insert(0, t);
                    }
                }
                "lastx" => {
                    if let Some(x) = self.last_x.borrow().clone() {
                        stack.push(x);
                    }
                }
                "negate" => {
                    if n == 0 {
                        return;
                    }
                    let v = stack[n - 1].clone();
                    drop(stack);
                    let w = Rc::downgrade(self);
                    self.app().svc.run(
                        move |c| attempt(|| c.run(&ops::neg(v), &[])).ok(),
                        move |r| {
                            if let (Some(p), Some(r)) = (w.upgrade(), r) {
                                if let Some(last) = p.stack.borrow_mut().last_mut() {
                                    *last = r;
                                }
                                p.refresh_rpn();
                            }
                        },
                    );
                    return;
                }
                _ => {}
            }
        }
        self.refresh_rpn();
    }

    fn edit_register(self: &Rc<Self>, idx: usize, text: String) {
        let w = Rc::downgrade(self);
        self.app().svc.run(
            move |c| c.calculate(&text, 0, false),
            move |r| {
                let Some(p) = w.upgrade() else { return };
                if let Some(e) = &r.error {
                    p.set_status(e, true);
                    return;
                }
                {
                    let mut st = p.stack.borrow_mut();
                    let n = st.len();
                    if idx < n {
                        if let Some(v) = &r.result {
                            st[n - 1 - idx] = funcs::unhold(v);
                        }
                    }
                }
                p.refresh_rpn();
            },
        );
    }
}
pub fn base_label(b: i32) -> String {
    match b {
        2 => "BIN".into(),
        8 => "OCT".into(),
        10 => "DEC".into(),
        12 => "DUO".into(),
        16 => "HEX".into(),
        bases::ROMAN => "ROMAN".into(),
        bases::TIME => "TIME".into(),
        bases::SEXAGESIMAL => "SEXA".into(),
        bases::BIJECTIVE26 => "BIJ26".into(),
        bases::BCD => "BCD".into(),
        bases::UNICODE => "UNI".into(),
        bases::GOLDEN => "φ".into(),
        bases::SUPERGOLDEN => "ψ".into(),
        bases::PI_BASE => "π".into(),
        bases::E_BASE => "e".into(),
        bases::SQRT2 => "√2".into(),
        bases::FP32 => "FP32".into(),
        bases::FP64 => "FP64".into(),
        bases::FP16 => "FP16".into(),
        bases::FP80 => "FP80".into(),
        bases::FP128 => "FP128".into(),
        bases::CUSTOM => "CUSTOM".into(),
        _ => format!("B{}", b),
}   }