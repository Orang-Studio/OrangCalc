// preferences, grouped into sections
use crate::engine::*;
use crate::ui::app::{App, Page};
use crate::ui::settings::{AfterCalc, AppSettings, CompletionMode, HistoryExprMode, TitleMode};
use crate::ui::widgets::*;
use crate::ui::{dialogs, theme};
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
const NAMES: [&str; 8] = ["Look & Feel", "Behavior", "Numbers & Operators", "Result Display", "Calculation", "Units & Currencies", "Keyboard Shortcuts", "Fonts"];
const ACCENTS: [&str; 8] = ["F08A3C", "E25C4C", "D6AA3C", "68B262", "40A8A0", "4E8CDC", "9676DC", "D264A0"];
pub struct SettingsPage {
    app: Weak<App>,
    root: gtk::Box,
    sections: gtk::ListBox,
    body: gtk::Box,
}
thread_local! {
    static CURRENT: RefCell<Weak<SettingsPage>> = RefCell::new(Weak::new());
}

impl Page for SettingsPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn activated(&self) {
        let i = self.sections.selected_row().map(|r| r.index() as usize).unwrap_or(0);
        if let Some(p) = CURRENT.with(|c| c.borrow().upgrade()) {
            p.build(i);
        }
    }
}

// builds one section's form
struct Builder {
    page: Weak<SettingsPage>,
    form: gtk::Grid,
    row: i32,
}

type Get<T> = Box<dyn Fn(&App) -> T>;
type Set<T> = Box<dyn Fn(&App, T)>;

impl Builder {
    fn app(&self) -> Rc<App> {
        self.page.upgrade().unwrap().app()
    }

    fn add(&mut self, w: &impl IsA<gtk::Widget>, label: Option<&str>) {
        match label {
            Some(l) => {
                self.form.attach(&dim(l), 0, self.row, 1, 1);
                w.set_halign(gtk::Align::Start);
                self.form.attach(w, 1, self.row, 1, 1);
            }
            None => self.form.attach(w, 0, self.row, 2, 1),
        }
        self.row += 1;
    }

    fn group(&mut self, t: &str) {
        let l = heading(t);
        l.set_margin_top(16);
        self.add(&l, None);
    }

    fn note(&mut self, t: &str) {
        let l = dim(t);
        l.set_max_width_chars(80);
        self.add(&l, None);
    }

    fn changed(app: &Rc<App>, options: bool) {
        if options {
            app.options_changed();
        } else {
            app.mark_dirty();
        }
    }

    fn check(&mut self, text: &str, get: impl Fn(&App) -> bool + 'static, set: impl Fn(&Rc<App>, bool) + 'static, options: bool) {
        let app = self.app();
        let c = gtk::CheckButton::with_label(text);
        c.set_active(get(&app));
        let w = Rc::downgrade(&app);
        c.connect_toggled(move |c| {
            if let Some(a) = w.upgrade() {
                set(&a, c.is_active());
                Self::changed(&a, options);
            }
        });
        self.add(&c, None);
    }

    fn opt(&mut self, text: &str, get: fn(&CalcOptions) -> bool, set: fn(&mut CalcOptions, bool)) {
        self.check(text, move |a| get(&a.svc.opts()), move |a, v| a.svc.set_opts(|o| set(o, v)), true);
    }

    fn choice(&mut self, label: &str, items: &[&str], get: Get<usize>, set: Box<dyn Fn(&Rc<App>, usize)>, options: bool) {
        let app = self.app();
        let d = combo(items, get(&app));
        d.set_size_request(260, -1);
        let w = Rc::downgrade(&app);
        d.connect_selected_notify(move |d| {
            if let Some(a) = w.upgrade() {
                set(&a, d.selected() as usize);
                Self::changed(&a, options);
            }
        });
        self.add(&d, Some(label));
    }

    fn opt_choice(&mut self, label: &str, items: &[&str], get: fn(&CalcOptions) -> usize, set: fn(&mut CalcOptions, usize)) {
        self.choice(label, items, Box::new(move |a| get(&a.svc.opts())), Box::new(move |a, i| a.svc.set_opts(|o| set(o, i))), true);
    }

    fn num(&mut self, label: &str, min: i32, max: i32, get: Get<i32>, set: Set<i32>, options: bool) {
        let app = self.app();
        let n = gtk::SpinButton::with_range(min as f64, max as f64, 1.0);
        n.set_value(get(&app) as f64);
        let w = Rc::downgrade(&app);
        n.connect_value_changed(move |n| {
            if let Some(a) = w.upgrade() {
                set(&a, n.value() as i32);
                Self::changed(&a, options);
            }
        });
        self.add(&n, Some(label));
    }

    fn opt_num(&mut self, label: &str, min: i32, max: i32, get: fn(&CalcOptions) -> i32, set: fn(&mut CalcOptions, i32)) {
        self.num(label, min, max, Box::new(move |a| get(&a.svc.opts())), Box::new(move |a, v| a.svc.set_opts(|o| set(o, v))), true);
    }

    fn ui_num(&mut self, label: &str, min: i32, max: i32, get: fn(&AppSettings) -> i32, set: fn(&mut AppSettings, i32)) {
        self.num(label, min, max, Box::new(move |a| get(&a.settings.borrow())), Box::new(move |a, v| set(&mut a.settings.borrow_mut(), v)), false);
    }

    fn ui_check(&mut self, text: &str, get: fn(&AppSettings) -> bool, set: fn(&mut AppSettings, bool)) {
        self.check(text, move |a| get(&a.settings.borrow()), move |a, v| set(&mut a.settings.borrow_mut(), v), false);
    }
}

macro_rules! oe {
    ($b:expr, $label:expr, $field:ident, $ty:ty, $items:expr) => {
        $b.opt_choice($label, &$items, |o| o.$field.index(), |o, i| o.$field = <$ty>::from_index(i))
    };
}

impl SettingsPage {
    pub fn from_page(_p: &Rc<dyn Page>) -> Option<Rc<SettingsPage>> {
        CURRENT.with(|c| c.borrow().upgrade())
    }

    pub fn new(app: &Rc<App>) -> Rc<SettingsPage> {
        let root = vbox(10);
        margins(&root, 16);
        let header = label("Settings");
        header.add_css_class("oc-page-title");
        root.append(&header);
        let content = hbox(16);
        content.set_vexpand(true);
        let sections = gtk::ListBox::new();
        sections.add_css_class("navigation-sidebar");
        for n in NAMES {
            let l = label(n);
            l.set_margin_top(4);
            l.set_margin_bottom(4);
            sections.append(&l);
        }
        let ss = scrolled(&sections);
        ss.set_hexpand(false);
        ss.set_size_request(190, -1);
        content.append(&ss);
        let body = vbox(0);
        content.append(&scrolled(&body));
        root.append(&content);
        let page = Rc::new(SettingsPage { app: Rc::downgrade(app), root, sections, body });
        CURRENT.with(|c| *c.borrow_mut() = Rc::downgrade(&page));
        let w = Rc::downgrade(&page);
        page.sections.connect_row_selected(move |_, r| {
            if let (Some(p), Some(r)) = (w.upgrade(), r) {
                p.build(r.index() as usize);
            }
        });
        page.sections.select_row(page.sections.row_at_index(0).as_ref());
        page
    }

    fn app(&self) -> Rc<App> {
        self.app.upgrade().expect("app")
    }

    pub fn show_section(&self, i: usize) {
        self.sections.select_row(self.sections.row_at_index(i as i32).as_ref());
    }

    fn build(self: &Rc<Self>, i: usize) {
        while let Some(c) = self.body.first_child() {
            self.body.remove(&c);
        }
        let form = gtk::Grid::new();
        form.set_row_spacing(6);
        form.set_column_spacing(12);
        form.set_margin_start(8);
        form.set_margin_end(12);
        let title = label(NAMES[i]);
        title.add_css_class("title-3");
        title.set_margin_bottom(10);
        form.attach(&title, 0, 0, 2, 1);
        let mut b = Builder { page: Rc::downgrade(self), form: form.clone(), row: 1 };
        match i {
            0 => self.look_and_feel(&mut b),
            1 => self.behavior(&mut b),
            2 => self.parsing(&mut b),
            3 => self.display(&mut b),
            4 => self.calculation(&mut b),
            5 => self.units(&mut b),
            6 => self.shortcuts(&mut b),
            _ => self.fonts(&mut b),
        }
        self.body.append(&form);
    }

    fn look_and_feel(self: &Rc<Self>, b: &mut Builder) {
        b.choice(
            "Window title:",
            &["Application name", "Application name + result", "Application name + workspace", "Result only"],
            Box::new(|a| a.settings.borrow().title.index()),
            Box::new(|a, i| {
                a.settings.borrow_mut().title = TitleMode::from_index(i);
                a.update_title("");
            }),
            false,
        );
        let row = hbox(6);
        for c in ACCENTS {
            let sw = gtk::Button::new();
            let area = gtk::DrawingArea::new();
            area.set_content_width(18);
            area.set_content_height(18);
            let rgb = (u8::from_str_radix(&c[0..2], 16).unwrap() as f64 / 255.0, u8::from_str_radix(&c[2..4], 16).unwrap() as f64 / 255.0, u8::from_str_radix(&c[4..6], 16).unwrap() as f64 / 255.0);
            area.set_draw_func(move |_, cr, w, h| {
                cr.set_source_rgb(rgb.0, rgb.1, rgb.2);
                cr.rectangle(0.0, 0.0, w as f64, h as f64);
                let _ = cr.fill();
            });
            sw.set_child(Some(&area));
            let w = Rc::downgrade(self);
            sw.connect_clicked(move |_| {
                if let Some(p) = w.upgrade() {
                    p.set_accent(c);
                }
            });
            row.append(&sw);
        }
        let app = self.app();
        let cur = app.settings.borrow().accent_color.clone();
        let dacc = crate::ui::theme::DEFAULT_ACCENT;
        let cc = |r: std::ops::Range<usize>| u8::from_str_radix(&dacc[r], 16).unwrap() as f32 / 255.0;
        let rgba = gtk::gdk::RGBA::parse(format!("#{}", cur)).unwrap_or_else(|_| gtk::gdk::RGBA::new(cc(0..2), cc(2..4), cc(4..6), 1.0));
        let custom = gtk::ColorDialogButton::new(Some(gtk::ColorDialog::new()));
        custom.set_rgba(&rgba);
        let w = Rc::downgrade(self);
        custom.connect_rgba_notify(move |c| {
            if let Some(p) = w.upgrade() {
                let r = c.rgba();
                p.set_accent(&format!("{:02X}{:02X}{:02X}", (r.red() * 255.0) as u8, (r.green() * 255.0) as u8, (r.blue() * 255.0) as u8));
            }
        });
        row.append(&custom);
        b.add(&row, Some("Accent color:"));
        b.check(
            "Prefer dark theme",
            |a| a.settings.borrow().dark_theme,
            |a, v| {
                a.settings.borrow_mut().dark_theme = v;
                let acc = a.settings.borrow().accent_color.clone();
                theme::apply(&acc, v);
                a.calc().refresh_result();
                a.calc().history.refill();
            },
            false,
        );
        b.check(
            "Show status line (expression status and messages)",
            |a| a.settings.borrow().show_status,
            |a, v| {
                a.settings.borrow_mut().show_status = v;
                a.calc().apply_settings();
            },
            false,
        );
        b.check(
            "Colorize result",
            |a| a.settings.borrow().colorize_result,
            |a, v| {
                a.settings.borrow_mut().colorize_result = v;
                a.calc().apply_settings();
                a.calc().refresh_result();
            },
            false,
        );
        b.ui_check("Show tooltips on keypad buttons (applies to newly built keypads)", |s| s.tooltips, |s, v| s.tooltips = v);
        b.group("Layout");
        for (t, get, set) in [
            ("Show keypad", (|s: &AppSettings| s.keypad_visible) as fn(&AppSettings) -> bool, (|s: &mut AppSettings, v| s.keypad_visible = v) as fn(&mut AppSettings, bool)),
            ("Always show number pad", |s| s.numpad_visible, |s, v| s.numpad_visible = v),
            ("Show side panel (history, RPN stack, number bases)", |s| s.side_visible, |s, v| s.side_visible = v),
            ("Place side panel on the left", |s| s.side_left, |s, v| s.side_left = v),
        ] {
            b.check(
                t,
                move |a| get(&a.settings.borrow()),
                move |a, v| {
                    set(&mut a.settings.borrow_mut(), v);
                    a.calc().apply_layout();
                },
                false,
            );
        }
        b.note("The window size is remembered between sessions.");
    }

    fn set_accent(&self, c: &str) {
        let app = self.app();
        app.settings.borrow_mut().accent_color = c.to_string();
        let dark = app.settings.borrow().dark_theme;
        theme::apply(c, dark);
        app.mark_dirty();
    }

    fn behavior(&self, b: &mut Builder) {
        b.group("Expression");
        b.choice("Expression after calculation:", &["Keep expression", "Clear expression", "Replace with result", "Replace with result if shorter"], Box::new(|a| a.settings.borrow().expression_after.index()), Box::new(|a, i| a.settings.borrow_mut().expression_after = AfterCalc::from_index(i)), false);
        b.ui_check("Calculate as you type (shows the result in the status line)", |s| s.calc_as_you_type, |s, v| s.calc_as_you_type = v);
        b.ui_num("Calculate as you type delay (ms):", 0, 5000, |s| s.as_you_type_delay, |s, v| s.as_you_type_delay = v);
        b.ui_check("Adaptive delay", |s| s.adaptive_delay, |s, v| s.adaptive_delay = v);
        b.ui_check("Automatically calculate selection", |s| s.auto_calculate_selection, |s, v| s.auto_calculate_selection = v);
        b.choice(
            "Completion:",
            &["No completion", "Full completion", "Strict completion", "Limited full completion", "Limited strict completion"],
            Box::new(|a| a.settings.borrow().completion.index()),
            Box::new(|a, i| {
                a.settings.borrow_mut().completion = CompletionMode::from_index(i);
                a.calc().apply_settings();
            }),
            false,
        );
        b.num(
            "Minimum characters before completion:",
            1,
            10,
            Box::new(|a| a.settings.borrow().completion_min_chars),
            Box::new(|a, v| {
                a.settings.borrow_mut().completion_min_chars = v;
                a.calc().apply_settings();
            }),
            false,
        );
        b.ui_check("Close application with Escape key (when the expression is empty)", |s| s.close_with_escape, |s, v| s.close_with_escape = v);
        b.group("Result");
        b.ui_check("Automatically copy result", |s| s.auto_copy_result, |s, v| s.auto_copy_result = v);
        b.ui_check("Copy unformatted ASCII by default", |s| s.copy_ascii_default, |s, v| s.copy_ascii_default = v);
        b.ui_check("Copy unformatted ASCII without units", |s| s.copy_ascii_no_units, |s, v| s.copy_ascii_no_units = v);
        b.group("History");
        b.ui_num("Max history lines saved:", 10, 100000, |s| s.max_history, |s, v| s.max_history = v);
        b.ui_check("Clear history on exit (protected entries are kept)", |s| s.clear_history_on_exit, |s, v| s.clear_history_on_exit = v);
        b.choice(
            "Expression in history:",
            &["Entered", "Parsed", "Entered + parsed"],
            Box::new(|a| a.settings.borrow().history_expression.index()),
            Box::new(|a, i| {
                a.settings.borrow_mut().history_expression = HistoryExprMode::from_index(i);
                a.calc().history.refill();
            }),
            false,
        );
        b.group("RPN and workspaces");
        b.check(
            "Use keyboard keys for RPN operations",
            |a| a.settings.borrow().rpn_keys,
            |a, v| {
                a.settings.borrow_mut().rpn_keys = v;
                a.calc().apply_settings();
            },
            false,
        );
        b.ui_check("Ask to save the workspace on exit", |s| s.ask_save_workspace, |s, v| s.ask_save_workspace = v);
    }

    fn parsing(&self, b: &mut Builder) {
        oe!(b, "Parsing mode:", parse, ParseMode, ["Conventional", "Implicit multiplication first", "Adaptive", "Chain", "RPN syntax"]);
        b.note("Adaptive: implicit multiplication binds tighter unless separated by a space, so 1/2x = 1/(2x) but 1/2 x = x/2.");
        b.opt("Limit implicit multiplication", |o| o.limit_implicit_mult, |o, v| o.limit_implicit_mult = v);
        b.opt("Interpret unrecognized symbols as unknown variables", |o| o.unknown_as_variables, |o, v| o.unknown_as_variables = v);
        b.opt("Read precision (decimal input is approximate)", |o| o.read_precision, |o, v| o.read_precision = v);
        b.opt("Allow concise uncertainty input, e.g. 1.23(4)", |o| o.concise_uncertainty_input, |o, v| o.concise_uncertainty_input = v);
        b.opt("Simplified percentage calculation (100 + 10% = 110)", |o| o.simplified_percent, |o, v| o.simplified_percent = v);
        b.group("Separators");
        b.opt("Use comma as decimal separator (semicolon separates arguments)", |o| o.comma_decimal, |o, v| o.comma_decimal = v);
        b.opt("Ignore comma in numbers", |o| o.ignore_comma, |o, v| o.ignore_comma = v);
        b.opt("Ignore dots in numbers", |o| o.ignore_dots, |o, v| o.ignore_dots = v);
        b.group("Operators");
        b.opt("Use caret for bitwise XOR", |o| o.caret_xor, |o, v| o.caret_xor = v);
        b.opt("Use 'j' as imaginary unit", |o| o.use_j, |o, v| o.use_j = v);
        b.group("Number base");
        const BASES: [i32; 5] = [10, 2, 8, 12, 16];
        b.opt_choice("Expression base:", &["Decimal", "Binary", "Octal", "Duodecimal", "Hexadecimal"], |o| BASES.iter().position(|x| *x == o.input_base).unwrap_or(0), |o, i| o.input_base = BASES[i]);
        b.opt("Two's complement input for binary numbers", |o| o.twos_complement_input, |o, v| o.twos_complement_input = v);
        b.opt("Two's complement input for hexadecimal numbers", |o| o.hex_twos_complement_input, |o, v| o.hex_twos_complement_input = v);
    }

    fn display(&self, b: &mut Builder) {
        oe!(b, "Number display:", display, NumberDisplay, ["Normal", "Scientific", "Engineering", "Simple", "Purely scientific"]);
        b.opt_num("Precision (significant digits):", 2, 5000, |o| o.precision, |o, v| o.precision = v);
        b.opt_num("Min decimals:", 0, 100, |o| o.min_decimals, |o, v| o.min_decimals = v);
        b.opt_num("Max decimals (−1 = off):", -1, 100, |o| o.max_decimals, |o, v| o.max_decimals = v);
        b.opt_choice(
            "Rounding:",
            &["Round halfway numbers away from zero", "Round halfway numbers to even", "Round halfway numbers to odd", "Round halfway numbers toward zero", "Round halfway numbers to random", "Round halfway numbers up", "Round halfway numbers down", "Round toward zero", "Round away from zero", "Round up", "Round down"],
            |o| o.rounding.index(),
            |o, i| o.rounding = RoundingMode::from_index(i),
        );
        oe!(b, "Fractions:", fraction, FractionMode, ["Decimal", "Exact decimal", "Fraction", "Mixed fraction", "Percent", "Per mille", "Per myriad"]);
        oe!(b, "Indicate repeating decimals:", repeating, RepeatingMode, ["Off", "Ellipsis", "Overline"]);
        oe!(b, "Digit grouping:", grouping, DigitGrouping, ["Off", "Standard (thin space)", "Local (comma)"]);
        oe!(b, "Complex number form:", complex, ComplexForm, ["Rectangular", "Exponential", "Polar", "Cis", "Angle/phasor"]);
        oe!(b, "Interval display:", interval_out, IntervalDisplay, ["Adaptive", "Significant digits", "Interval", "Plus/minus", "Midpoint", "Lower", "Upper", "Concise", "Relative"]);
        b.group("Notation");
        b.opt("Use E-notation instead of 10^n", |o| o.e_notation, |o, v| o.e_notation = v);
        b.opt("Use lower case e in E-notation", |o| o.lower_case_e, |o, v| o.lower_case_e = v);
        b.opt("Use lower case letters in non-decimal numbers", |o| o.lower_case_hex, |o, v| o.lower_case_hex = v);
        b.opt("Use special duodecimal symbols (↊ ↋)", |o| o.duodecimal_symbols, |o, v| o.duodecimal_symbols = v);
        b.opt("Show ending zeroes of approximate numbers", |o| o.show_ending_zeroes, |o, v| o.show_ending_zeroes = v);
        b.opt("Use Unicode symbols (×, −, √, superscripts)", |o| o.unicode, |o, v| o.unicode = v);
        oe!(b, "Multiplication sign:", mult, MultSign, ["Dot operator (⋅)", "Multiplication sign (×)", "Asterisk (*)", "Middle dot (·)"]);
        b.opt("Use Unicode division slash in output", |o| o.unicode_division, |o, v| o.unicode_division = v);
        b.opt("Implicit multiplication in output (2x, 3π)", |o| o.implicit_mult_output, |o, v| o.implicit_mult_output = v);
        b.opt("Spell out logical operators", |o| o.spell_out_logical, |o, v| o.spell_out_logical = v);
        b.opt("Abbreviate names (unit symbols instead of names)", |o| o.abbreviate_names, |o, v| o.abbreviate_names = v);
        b.group("Number bases");
        oe!(b, "Two's complement output:", twos_out, TwosComplement, ["Off", "Binary", "Binary and hexadecimal"]);
        b.opt_num("Binary bits (0 = automatic):", 0, 4096, |o| o.binary_bits, |o, v| o.binary_bits = v);
        b.opt_num("Binary letter spacing:", 0, 16, |o| o.binary_letter_spacing, |o, v| o.binary_letter_spacing = v);
    }

    fn calculation(&self, b: &mut Builder) {
        oe!(b, "Approximation:", approx, ApproxMode, ["Automatic", "Dual (exact and approximate)", "Exact", "Approximate"]);
        oe!(b, "Angle unit:", angle, AngleMode, ["Radians", "Degrees", "Gradians", "None"]);
        oe!(b, "Result structure:", structure, StructMode, ["Simplify", "Factorize", "Expand", "None"]);
        oe!(b, "Interval calculation:", interval, IntervalCalc, ["Variance formula", "Interval arithmetic", "Simple"]);
        b.opt("Allow complex numbers", |o| o.complex_enabled, |o, v| o.complex_enabled = v);
        b.opt("Allow infinite numbers", |o| o.infinite_enabled, |o, v| o.infinite_enabled = v);
        b.opt("Use real roots for odd roots of negative numbers", |o| o.real_roots, |o, v| o.real_roots = v);
        b.opt("Enable variables and constants", |o| o.variables_enabled, |o, v| o.variables_enabled = v);
        b.opt("Enable functions", |o| o.functions_enabled, |o, v| o.functions_enabled = v);
        b.group("Default assumptions for unknown variables");
        let types: Vec<&str> = AssumeType::ALL.iter().map(|t| t.name()).collect();
        b.opt_choice("Type:", &types, |o| o.default_type.index(), |o, i| o.default_type = AssumeType::from_index(i));
        let signs: Vec<String> = AssumeSign::ALL.iter().map(|s| words(s.name())).collect();
        let signs: Vec<&str> = signs.iter().map(|s| s.as_str()).collect();
        b.opt_choice("Sign:", &signs, |o| o.default_sign.index(), |o, i| o.default_sign = AssumeSign::from_index(i));
    }

    fn units(&self, b: &mut Builder) {
        b.opt("Enable units", |o| o.units_enabled, |o, v| o.units_enabled = v);
        b.opt("Enable units in physical constants", |o| o.units_in_constants, |o, v| o.units_in_constants = v);
        oe!(b, "Automatic unit conversion:", auto_convert, UnitConversion, ["No conversion", "Optimal SI units", "Optimal units", "Base units"]);
        b.opt("Convert to mixed units (1.5 h = 1 h + 30 min)", |o| o.mixed_units, |o, v| o.mixed_units = v);
        oe!(b, "Automatic unit prefixes:", prefixes, PrefixMode, ["No prefixes", "Prefixes for some units", "Prefixes for all units", "Prefixes also for currencies"]);
        b.opt("Enable denominator prefixes", |o| o.denominator_prefixes, |o, v| o.denominator_prefixes = v);
        b.opt("Enable all SI prefixes", |o| o.all_si_prefixes, |o, v| o.all_si_prefixes = v);
        b.opt("Prefixes also for currencies", |o| o.currency_prefixes, |o, v| o.currency_prefixes = v);
        b.opt("Use binary prefixes for information units", |o| o.binary_prefixes, |o, v| o.binary_prefixes = v);
        oe!(b, "Temperature calculation:", temperature, TemperatureMode, ["Hybrid", "Absolute", "Relative"]);
        b.group("Currencies");
        const DAYS: [i32; 4] = [0, 1, 7, 30];
        b.choice(
            "Exchange rates updates:",
            &["Never", "Daily", "Weekly", "Monthly"],
            Box::new(|a| {
                let d = a.settings.borrow().rates_update_days;
                if d <= 0 {
                    0
                } else if d == 1 {
                    1
                } else if d <= 7 {
                    2
                } else {
                    3
                }
            }),
            Box::new(|a, i| a.settings.borrow_mut().rates_update_days = DAYS[i]),
            false,
        );
        let app = self.app();
        let mut cur: Vec<String> = app.snap.borrow().units.iter().filter(|u| u.is_currency).map(|u| u.name().to_string()).collect();
        cur.sort();
        let names: Vec<&str> = cur.iter().map(|s| s.as_str()).collect();
        let c2 = cur.clone();
        let c3 = cur.clone();
        b.choice("Default currency:", &names, Box::new(move |a| c2.iter().position(|x| *x == a.settings.borrow().default_currency).unwrap_or(0)), Box::new(move |a, i| a.settings.borrow_mut().default_currency = c3[i].clone()), false);
        let w = Rc::downgrade(&app);
        let btn = button("Update exchange rates now", move || {
            if let Some(a) = w.upgrade() {
                a.update_rates(true);
            }
        });
        btn.add_css_class("suggested-action");
        btn.set_halign(gtk::Align::Start);
        btn.set_margin_top(8);
        b.add(&btn, None);
        let last = app.settings.borrow().last_rates_update.clone();
        b.note(&if last.is_empty() { "Exchange rates have not been downloaded yet; built-in approximate rates are used.".to_string() } else { format!("Last update: {}. Rates come from the European Central Bank.", last.replace('T', " ")) });
    }

    fn shortcuts(self: &Rc<Self>, b: &mut Builder) {
        let app = self.app();
        let list = gtk::ListBox::new();
        list.add_css_class("rich-list");
        let fill = {
            let (list, app) = (list.clone(), Rc::downgrade(&app));
            Rc::new(move || {
                let Some(app) = app.upgrade() else { return };
                let sel = list.selected_row().map(|r| r.index());
                while let Some(r) = list.row_at_index(0) {
                    list.remove(&r);
                }
                for a in &app.actions {
                    let row = hbox(12);
                    row.set_margin_top(3);
                    row.set_margin_bottom(3);
                    let t = label(a.title);
                    t.set_hexpand(true);
                    row.append(&t);
                    let k = gtk::Label::new(Some(&app.shortcut_for(a)));
                    k.add_css_class("dim-label");
                    row.append(&k);
                    list.append(&row);
                }
                if let Some(i) = sel {
                    list.select_row(list.row_at_index(i).as_ref());
                }
            })
        };
        fill();
        let sw = scrolled(&list);
        sw.set_min_content_height(440);
        sw.set_min_content_width(620);
        b.add(&sw, None);
        let edit = {
            let (list, app, fill) = (list.clone(), Rc::downgrade(&app), fill.clone());
            Rc::new(move || {
                let (Some(app), Some(r)) = (app.upgrade(), list.selected_row()) else { return };
                let a = &app.actions[r.index() as usize];
                let (id, title, cur) = (a.id, a.title, app.shortcut_for(a));
                let (a2, fill) = (app.clone(), fill.clone());
                dialogs::shortcut_dialog(app.parent(), title, &cur, move |v| {
                    let other = a2.actions.iter().find(|x| x.id != id && !v.is_empty() && a2.shortcut_for(x) == v).map(|x| (x.id, x.title));
                    let apply = {
                        let (a3, fill, v) = (a2.clone(), fill.clone(), v.clone());
                        move |clear: Option<&'static str>| {
                            {
                                let mut s = a3.settings.borrow_mut();
                                if let Some(o) = clear {
                                    s.shortcuts.insert(o.to_string(), String::new());
                                }
                                s.shortcuts.insert(id.to_string(), v.clone());
                            }
                            a3.apply_shortcuts();
                            fill();
                            a3.mark_dirty();
                        }
                    };
                    match other {
                        Some((oid, otitle)) => confirm(a2.parent(), &format!("The key combination is already in use.\nDo you wish to replace the current action ({})?", otitle), move || apply(Some(oid))),
                        None => apply(None),
                    }
                });
            })
        };
        let e = edit.clone();
        list.connect_row_activated(move |_, _| e());
        let row = hbox(8);
        let e = edit.clone();
        let eb = button("Edit…", move || e());
        eb.add_css_class("suggested-action");
        row.append(&eb);
        let (l2, a2, f2) = (list.clone(), Rc::downgrade(&app), fill.clone());
        row.append(&button("Reset", move || {
            let (Some(app), Some(r)) = (a2.upgrade(), l2.selected_row()) else { return };
            let id = app.actions[r.index() as usize].id;
            app.settings.borrow_mut().shortcuts.remove(id);
            app.apply_shortcuts();
            f2();
            app.mark_dirty();
        }));
        let (a3, f3) = (Rc::downgrade(&app), fill.clone());
        row.append(&button("Reset all", move || {
            if let Some(app) = a3.upgrade() {
                app.settings.borrow_mut().shortcuts.clear();
                app.apply_shortcuts();
                f3();
                app.mark_dirty();
            }
        }));
        row.set_margin_top(8);
        b.add(&row, None);
    }

    fn fonts(self: &Rc<Self>, b: &mut Builder) {
        let rows: [(&str, fn(&AppSettings) -> String, fn(&mut AppSettings, String)); 5] = [
            ("Expression font:", |s| s.expression_font.clone(), |s, v| s.expression_font = v),
            ("Result font:", |s| s.result_font.clone(), |s, v| s.result_font = v),
            ("History font:", |s| s.history_font.clone(), |s, v| s.history_font = v),
            ("Keypad font:", |s| s.keypad_font.clone(), |s, v| s.keypad_font = v),
            ("Status font:", |s| s.status_font.clone(), |s, v| s.status_font = v),
        ];
        let app = self.app();
        for (label_text, get, set) in rows {
            let row = hbox(8);
            let fb = gtk::FontDialogButton::new(Some(gtk::FontDialog::new()));
            let cur = get(&app.settings.borrow());
            if !cur.is_empty() {
                fb.set_font_desc(&gtk::pango::FontDescription::from_string(&cur));
            }
            let w = Rc::downgrade(&app);
            fb.connect_font_desc_notify(move |fb| {
                if let (Some(a), Some(fd)) = (w.upgrade(), fb.font_desc()) {
                    set(&mut a.settings.borrow_mut(), fd.to_string());
                    theme::apply_fonts(&a.settings.borrow());
                    a.mark_dirty();
                }
            });
            row.append(&fb);
            let w = Rc::downgrade(self);
            let wa = Rc::downgrade(&app);
            row.append(&button("Default", move || {
                if let (Some(a), Some(p)) = (wa.upgrade(), w.upgrade()) {
                    set(&mut a.settings.borrow_mut(), String::new());
                    theme::apply_fonts(&a.settings.borrow());
                    a.mark_dirty();
                    p.build(7);
                }
            }));
            b.add(&row, Some(label_text));
}   }   }