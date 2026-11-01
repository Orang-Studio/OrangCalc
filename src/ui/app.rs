// main window: menus, navigation, actions, workspaces and exchange rates
use super::calcpage::CalcPage;
use super::settings::{self as st, AppSettings, KeypadMode, TitleMode};
use super::svc::{Snapshot, Svc};
use super::widgets::*;
use super::{dialogs, pages, theme};
use crate::engine::*;
use crate::xml::Node;
use gtk::prelude::*;
use gtk::{gio, gdk, glib};
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
pub const PAGE_NAMES: [&str; 9] = ["Calculator", "Functions", "Variables", "Units", "Data Sets", "Periodic Table", "Plot", "Tools", "Settings"];
pub trait Page {
    fn widget(&self) -> gtk::Widget;
    fn activated(&self) {}
    fn definitions_changed(&self) {}
    fn options_changed(&self) {}
}
pub struct ActionDef {
    pub id: &'static str,
    pub title: &'static str,
    pub default_keys: &'static str,
}
pub struct App {
    pub window: gtk::ApplicationWindow,
    pub gapp: gtk::Application,
    pub svc: Svc,
    pub settings: RefCell<AppSettings>,
    pub snap: RefCell<Snapshot>,
    stack: gtk::Stack,
    nav: gtk::ListBox,
    calc: OnceCell<Rc<CalcPage>>,
    pages: RefCell<HashMap<usize, Rc<dyn Page>>>,
    dirty: Cell<bool>,
    closing: Cell<bool>,
    last_result_title: RefCell<String>,
    pub actions: Vec<ActionDef>,
    fn_menu: gio::Menu,
    var_menu: gio::Menu,
    unit_menu: gio::Menu,
    ws_recent: gio::Menu,
    syncing: Cell<bool>,
}

macro_rules! actions {
    ($($id:literal, $title:literal, $keys:literal;)*) => { vec![$(ActionDef { id: $id, title: $title, default_keys: $keys }),*] };
}

fn action_defs() -> Vec<ActionDef> {
    actions! {
        "calculate", "Calculate expression", "";
        "clear", "Clear expression", "";
        "copy", "Copy result", "Ctrl+Shift+C" ;
        "copyascii", "Copy unformatted ASCII result", "";
        "store", "Store result as variable", "Ctrl+S";
        "newfunction", "New function", "Ctrl+Shift+F";
        "newvariable", "New variable", "Ctrl+Shift+V" ;
        "newunknown", "New unknown variable", "";
        "newunit", "New unit", "Ctrl+Shift+U";
        "newmatrix", "New matrix", "Ctrl+Shift+M";
        "insertdate", "Insert date", "Ctrl+Shift+D" ;
        "wsopen", "Open workspace", "Ctrl+O";
        "wssave", "Save workspace", "";
        "wssaveas", "Save workspace as", "";
        "wsdefault", "Open default workspace", "";
        "csvimport", "Import CSV file", "";
        "csvexport", "Export CSV file", "";
        "calculator", "Show calculator", "Ctrl+1";
        "functions", "Functions", "Ctrl+F";
        "variables", "Variables and constants", "Ctrl+Shift+K";
        "units", "Units", "Ctrl+U";
        "datasets", "Data sets", "";
        "periodic", "Periodic table", "Ctrl+Shift+P";
        "plot", "Plot functions and data", "Ctrl+P";
        "plotexpr", "Plot current expression", "";
        "tools", "Tools", "";
        "percentage", "Percentage calculation tool", "";
        "calendar", "Calendar conversion", "";
        "fpconv", "Floating point conversion", "";
        "settings", "Preferences", "Ctrl+Comma";
        "shortcuts", "Keyboard shortcuts", "";
        "rates", "Update exchange rates", "";
        "history", "Show history", "Ctrl+H";
        "stack", "Show RPN stack", "Ctrl+K";
        "bases", "Show number bases", "Ctrl+B";
        "keypad", "Toggle keypad", "Ctrl+Shift+Space";
        "sidepanel", "Toggle side panel", "Ctrl+Shift+H";
        "rpn", "RPN mode", "Ctrl+R";
        "chain", "Chain mode", "";
        "normal", "Normal mode", "";
        "degrees", "Degrees", "";
        "radians", "Radians", "";
        "gradians", "Gradians", "";
        "exact", "Exact mode", "Ctrl+E";
        "fraction", "Toggle fraction display", "";
        "scientific", "Scientific notation", "";
        "tohex", "Convert result to hexadecimal", "" ;
        "tobin", "Convert result to binary", "" ;
        "todec", "Convert result to decimal", "" ;
        "tofraction", "Convert result to fraction", "" ;
        "tofactors", "Factorize result", "";
        "expand", "Expand result", "";
        "tobase", "Convert result to base units", "";
        "tooptimal", "Convert result to optimal unit", "";
        "clearhistory", "Clear history", "";
        "clearstack", "Clear RPN stack", "";
        "swap", "RPN swap", "";
        "lastx", "RPN last x", "";
        "help", "Help", "F1";
        "about", "About", "";
        "bug", "Report a bug", "";
        "settingsfolder", "Open settings folder", "";
        "quit", "Quit", "Ctrl+Q" ;
        "undo", "Undo", "";
        "redo", "Redo", "";
    }
}

// converts "Ctrl+Shift+C" style shortcuts into GTK accelerators
pub fn to_accel(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    let mut out = String::new();
    let parts: Vec<&str> = s.split('+').collect();
    for (i, p) in parts.iter().enumerate() {
        if i + 1 < parts.len() {
            match *p {
                "Ctrl" => out += "<Control>",
                "Shift" => out += "<Shift>",
                "Alt" => out += "<Alt>",
                "Super" => out += "<Super>",
                _ => return None,
            }
        } else {
            let key = match *p {
                "Comma" => "comma".to_string(),
                "Period" => "period".to_string(),
                "Space" => "space".to_string(),
                "Enter" => "Return".to_string(),
                k if k.len() == 1 => k.to_lowercase(),
                k => k.to_string(),
            };
            out += &key;
        }
    }
    Some(out)
}

pub fn from_accel(accel: &str) -> String {
    let Some((key, mods)) = gtk::accelerator_parse(accel) else { return String::new() };
    let mut parts = vec![];
    if mods.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
        parts.push("Ctrl".to_string());
    }
    if mods.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
        parts.push("Shift".to_string());
    }
    if mods.contains(gtk::gdk::ModifierType::ALT_MASK) {
        parts.push("Alt".to_string());
    }
    let name = key.name().map(|n| n.to_string()).unwrap_or_default();
    parts.push(match name.as_str() {
        "comma" => "Comma".into(),
        "period" => "Period".into(),
        "space" => "Space".into(),
        "Return" => "Enter".into(),
        n if n.chars().count() == 1 => n.to_uppercase(),
        n => n.to_string(),
    });
    parts.join("+")
}

// option menus are backed by stateful actions so GTK shows the radio/check marks
struct OptDef {
    name: &'static str,
    get: fn(&App) -> String,
    set: fn(&App, &str),
}

macro_rules! opt_enum {
    ($name:literal, $field:ident, $ty:ty) => {
        OptDef { name: $name, get: |a| a.svc.opts().$field.name().to_string(), set: |a, v| { if let Some(x) = <$ty>::parse(v) { a.svc.set_opts(|o| o.$field = x); } } }
    };
}
macro_rules! opt_int {
    ($name:literal, $field:ident) => {
        OptDef { name: $name, get: |a| a.svc.opts().$field.to_string(), set: |a, v| { if let Ok(x) = v.parse() { a.svc.set_opts(|o| o.$field = x); } } }
    };
}

fn opt_defs() -> Vec<OptDef> {
    vec![
        opt_enum!("angle", angle, AngleMode),
        opt_enum!("approx", approx, ApproxMode),
        opt_enum!("display", display, NumberDisplay),
        opt_enum!("fractionmode", fraction, FractionMode),
        opt_enum!("repeating", repeating, RepeatingMode),
        opt_enum!("complexform", complex, ComplexForm),
        opt_enum!("intervalout", interval_out, IntervalDisplay),
        opt_enum!("intervalcalc", interval, IntervalCalc),
        opt_enum!("structure", structure, StructMode),
        opt_enum!("temperature", temperature, TemperatureMode),
        opt_enum!("parsemode", parse, ParseMode),
        opt_enum!("twos", twos_out, TwosComplement),
        opt_enum!("assumetype", default_type, AssumeType),
        opt_enum!("assumesign", default_sign, AssumeSign),
        opt_int!("precision", precision ),
        opt_int!("maxdec", max_decimals ),
        opt_int!("mindec", min_decimals ),
        opt_int!("outbase", output_base ),
        opt_int!("inbase", input_base),
        opt_int!("binbits", binary_bits),
        OptDef {
            name: "percent",
            get: |a| if a.svc.opts().simplified_percent { "simple" } else { "plain" }.into(),
            set: |a, v| a.svc.set_opts(|o| o.simplified_percent = v == "simple"),
        },
        OptDef {
            name: "calcmode",
            get: |a| {
                if a.settings.borrow().rpn_mode {
                    "RPN".into()
                } else if a.svc.opts().parse == ParseMode::Chain {
                    "Chain".into()
                } else {
                    "Normal".into()
                }
            },
            set: |a, v| {
                a.settings.borrow_mut().rpn_mode = v == "RPN";
                a.svc.set_opts(|o| {
                    if v == "Chain" {
                        o.parse = ParseMode::Chain;
                    } else if o.parse == ParseMode::Chain {
                        o.parse = ParseMode::Adaptive;
                    }
                });
                if v == "RPN" {
                    a.calc().select_side(1);
                }
            },
        },
        OptDef {
            name: "keypadmode",
            get: |a| a.settings.borrow().keypad.name().into(),
            set: |a, v| {
                if let Some(km) = KeypadMode::parse(v) {
                    {
                        let mut s = a.settings.borrow_mut();
                        s.keypad_visible = km != KeypadMode::None;
                        s.keypad = km;
                    }
                    a.calc().keypad.select_mode(if km == KeypadMode::None { KeypadMode::General } else { km });
                    a.calc().apply_layout();
                }
            },
        },
    ]
}

struct BoolDef {
    name: &'static str,
    get: fn(&App) -> bool,
    set: fn(&App, bool),
}

fn bool_defs() -> Vec<BoolDef> {
    vec![
        BoolDef { name: "complexenabled", get: |a| a.svc.opts().complex_enabled, set: |a, v| a.svc.set_opts(|o| o.complex_enabled = v) },
        BoolDef { name: "infiniteenabled", get: |a| a.svc.opts().infinite_enabled, set: |a, v| a.svc.set_opts(|o| o.infinite_enabled = v) },
        BoolDef { name: "intervalarith", get: |a| a.svc.opts().interval == IntervalCalc::IntervalArithmetic, set: |a, v| a.svc.set_opts(|o| o.interval = if v { IntervalCalc::IntervalArithmetic } else { IntervalCalc::VarianceFormula }) },
        BoolDef {
            name: "numpad",
            get: |a| a.settings.borrow().numpad_visible,
            set: |a, v| {
                a.settings.borrow_mut().numpad_visible = v;
                a.calc().apply_layout();
            },
        },
        BoolDef {
            name: "sideleft",
            get: |a| a.settings.borrow().side_left,
            set: |a, v| {
                a.settings.borrow_mut().side_left = v;
                a.calc().apply_layout();
            },
        },
        BoolDef {
            name: "topmost",
            get: |a| a.settings.borrow().always_on_top,
            set: |a, v| {
                a.settings.borrow_mut().always_on_top = v;
                message(Some(a.window.upcast_ref()), "Keeping the window above others is up to your window manager on Linux; most offer it in the title bar menu.");
            },
        },
    ]
}

fn mi(label: &str, action: &str) -> gio::MenuItem {
    gio::MenuItem::new(Some(label), Some(action))
}

fn radio(m: &gio::Menu, label: &str, action: &str, value: &str) {
    let it = gio::MenuItem::new(Some(label), None);
    it.set_action_and_target_value(Some(&format!("win.{}", action)), Some(&value.to_variant()));
    m.append_item(&it);
}

fn section(parent: &gio::Menu, items: &gio::Menu) {
    parent.append_section(None, items);
}

impl App {
    pub fn build(gapp: &gtk::Application) -> Rc<App> {
        let mut settings = AppSettings::default();
        let mut opts = CalcOptions::default();
        st::load_settings(&mut settings, &mut opts);
        theme::apply(&settings.accent_color, settings.dark_theme);
        let (err_tx, err_rx) = std::sync::mpsc::channel();
        let svc = Svc::start(opts, move |c| {
            load_rates(c);
            let _ = err_tx.send(st::load_definitions(c));
        });
        let load_errors = err_rx.recv().unwrap_or_default();

        let window = gtk::ApplicationWindow::builder().application(gapp).title("OrangCalc").default_width(settings.win_w.max(760)).default_height(settings.win_h.max(560)).build();
        if let Some(icon) = theme::app_icon() {
            window.connect_map(move |w| {
                if let Some(top) = w.surface().and_then(|s| s.downcast::<gdk::Toplevel>().ok()) {
                    top.set_icon_list(&[icon.clone()]);
                }
            });
        }
        if settings.maximized {
            window.maximize();
        }

        let stack = gtk::Stack::new();
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        let nav = gtk::ListBox::new();
        nav.add_css_class("navigation-sidebar");
        nav.add_css_class("oc-nav");
        nav.set_selection_mode(gtk::SelectionMode::Single);
        let icons = ["accessories-calculator-symbolic", "insert-object-symbolic", "x-office-spreadsheet-symbolic", "preferences-system-symbolic", "folder-documents-symbolic", "applications-science-symbolic", "utilities-system-monitor-symbolic", "applications-utilities-symbolic", "emblem-system-symbolic"];
        for (i, n) in PAGE_NAMES.iter().enumerate() {
            let row = hbox(10);
            row.append(&gtk::Image::from_icon_name(icons[i]));
            row.append(&label(n));
            nav.append(&row);
            if i == 0 || i == 3 || i == 7 {
                if let Some(r) = nav.row_at_index(i as i32) {
                    r.set_margin_bottom(8);
                }
            }
        }

        let app = Rc::new(App {
            window,
            gapp: gapp.clone(),
            svc,
            settings: RefCell::new(settings),
            snap: RefCell::new(Snapshot::default()),
            stack,
            nav,
            calc: OnceCell::new(),
            pages: RefCell::new(HashMap::new()),
            dirty: Cell::new(false),
            closing: Cell::new(false),
            last_result_title: RefCell::new(String::new()),
            actions: action_defs(),
            fn_menu: gio::Menu::new(),
            var_menu: gio::Menu::new(),
            unit_menu: gio::Menu::new(),
            ws_recent: gio::Menu::new(),
            syncing: Cell::new(false),
        });
        *app.snap.borrow_mut() = app.svc.snapshot();

        let history = st::load_history();
        let calc = CalcPage::new(&app, history);
        let _ = app.calc.set(calc.clone());
        app.stack.add_named(&calc.widget(), Some("0"));
        app.pages.borrow_mut().insert(0, calc.clone());

        let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
        let navscroll = scrolled(&app.nav);
        navscroll.set_hexpand(false);
        navscroll.set_size_request(170, -1);
        navscroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        paned.set_start_child(Some(&navscroll));
        paned.set_end_child(Some(&app.stack));
        paned.set_shrink_start_child(false);
        paned.set_resize_start_child(false);

        app.register_actions();
        app.register_option_actions();
        let menubar = gtk::PopoverMenuBar::from_model(Some(&app.build_menu()));
        let root = vbox(0);
        root.append(&menubar);
        root.append(&paned);
        app.window.set_child(Some(&root));
        app.apply_shortcuts();
        app.rebuild_item_menus();
        app.rebuild_recent_workspaces();

        let a = app.clone();
        app.nav.connect_row_selected(move |_, row| {
            if let Some(r) = row {
                a.show_page(r.index() as usize);
            }
        });
        let page = app.settings.borrow().current_page.clamp(0, 8) as usize;
        app.nav.select_row(app.nav.row_at_index(if page == 8 { 0 } else { page as i32 }).as_ref());

        let a = app.clone();
        app.window.connect_close_request(move |_| {
            a.on_close();
            glib::Propagation::Proceed
        });
        let a = app.clone();
        glib::timeout_add_seconds_local(4, move || {
            if a.dirty.get() {
                a.save_all();
            }
            glib::ControlFlow::Continue
        });

        app.window.present();
        calc.focus_expression();
        if !load_errors.is_empty() {
            message(Some(app.window.upcast_ref()), &format!("Some definitions could not be loaded:\n{}", load_errors.iter().take(10).cloned().collect::<Vec<_>>().join("\n")));
        }
        app.maybe_update_rates();
        app.update_title("");
        app
    }

    pub fn calc(&self) -> Rc<CalcPage> {
        self.calc.get().unwrap().clone()
    }

    pub fn parent(&self) -> Option<&gtk::Window> {
        Some(self.window.upcast_ref())
    }

    pub fn mark_dirty(&self) {
        self.dirty.set(true);
    }

    pub fn save_all(self: &Rc<Self>) {
        self.dirty.set(false);
        {
            let mut s = self.settings.borrow_mut();
            if !self.window.is_maximized() {
                s.win_w = self.window.width();
                s.win_h = self.window.height();
            }
            s.maximized = self.window.is_maximized();
            s.current_page = self.nav.selected_row().map(|r| r.index()).unwrap_or(0);
        }
        let s = self.settings.borrow().clone();
        st::save_settings(&s, &self.svc.opts());
        let calc = self.calc();
        let hist = calc.history.entries.borrow();
        if s.clear_history_on_exit && self.closing.get() {
            st::save_history(hist.iter().filter(|h| h.protected));
        } else {
            st::save_history(hist.iter());
        }
        self.svc.post(st::save_definitions);
    }

    fn on_close(self: &Rc<Self>) {
        self.closing.set(true);
        self.save_all();
        let _ = self.svc.run_sync(|_| ());
    }

    pub fn update_title(&self, result: &str) {
        *self.last_result_title.borrow_mut() = result.to_string();
        let s = self.settings.borrow();
        let ws = std::path::Path::new(&s.workspace).file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        let trim = |t: &str| if t.chars().count() > 80 { format!("{}…", t.chars().take(80).collect::<String>()) } else { t.to_string() };
        let t = match s.title {
            TitleMode::AppNameAndResult if !result.is_empty() => format!("OrangCalc – {}", trim(result)),
            TitleMode::AppNameAndWorkspace if !ws.is_empty() => format!("OrangCalc – {}", ws),
            TitleMode::ResultOnly if !result.is_empty() => trim(result),
            _ => "OrangCalc".to_string(),
        };
        self.window.set_title(Some(&t));
    }

    pub fn show_page(self: &Rc<Self>, i: usize) {
        if self.nav.selected_row().map(|r| r.index() as usize) != Some(i) {
            self.nav.select_row(self.nav.row_at_index(i as i32).as_ref());
            return;
        }
        let page = self.page(i);
        self.stack.set_visible_child_name(&i.to_string());
        page.activated();
        if i == 0 {
            self.calc().focus_expression();
        }
    }

    pub fn page(self: &Rc<Self>, i: usize) -> Rc<dyn Page> {
        if let Some(p) = self.pages.borrow().get(&i) {
            return p.clone();
        }
        let p: Rc<dyn Page> = match i {
            1 => pages::items::ItemsPage::new(self, pages::items::ItemKind::Function),
            2 => pages::items::ItemsPage::new(self, pages::items::ItemKind::Variable),
            3 => pages::items::ItemsPage::new(self, pages::items::ItemKind::Unit),
            4 => pages::datasets::DataSetsPage::new(self),
            5 => pages::periodic::PeriodicPage::new(self),
            6 => pages::plot::PlotPage::new(self),
            7 => pages::tools::ToolsPage::new(self),
            8 => pages::settings_page::SettingsPage::new(self),
            _ => self.calc(),
        };
        self.stack.add_named(&p.widget(), Some(&i.to_string()));
        self.pages.borrow_mut().insert(i, p.clone());
        p
    }

    pub fn insert_into_expression(self: &Rc<Self>, text: &str, calculate: bool) {
        self.show_page(0);
        let c = self.calc();
        c.insert(text, 0);
        if calculate {
            c.calculate();
        }
    }

    pub fn definitions_changed(self: &Rc<Self>) {
        *self.snap.borrow_mut() = self.svc.snapshot();
        self.rebuild_item_menus();
        let pages: Vec<Rc<dyn Page>> = self.pages.borrow().values().cloned().collect();
        for p in pages {
            p.definitions_changed();
        }
        self.mark_dirty();
    }

    pub fn options_changed(self: &Rc<Self>) {
        self.calc().refresh_modes();
        let pages: Vec<Rc<dyn Page>> = self.pages.borrow().values().cloned().collect();
        for p in pages {
            p.options_changed();
        }
        self.sync_option_actions();
        self.mark_dirty();
    }

    pub fn add_recent(list: &mut Vec<String>, name: &str) {
        list.retain(|x| x != name);
        list.insert(0, name.to_string());
        list.truncate(8);
    }

    fn add_action(self: &Rc<Self>, id: &str, f: impl Fn(&Rc<App>) + 'static) {
        let a = gio::SimpleAction::new(id, None);
        let app = Rc::downgrade(self);
        a.connect_activate(move |_, _| {
            if let Some(app) = app.upgrade() {
                f(&app);
            }
        });
        self.window.add_action(&a);
    }

    fn add_string_action(self: &Rc<Self>, id: &str, f: impl Fn(&Rc<App>, &str) + 'static) {
        let a = gio::SimpleAction::new(id, Some(glib::VariantTy::STRING));
        let app = Rc::downgrade(self);
        a.connect_activate(move |_, p| {
            if let (Some(app), Some(v)) = (app.upgrade(), p.and_then(|p| p.str().map(|s| s.to_string()))) {
                f(&app, &v);
            }
        });
        self.window.add_action(&a);
    }

    pub fn run_action(&self, id: &str) {
        gtk::prelude::ActionGroupExt::activate_action(&self.window, id, None);
    }

    fn register_actions(self: &Rc<Self>) {
        let cp = |a: &Rc<App>| a.calc();
        self.add_action("calculate", move |a| cp(a).calculate());
        self.add_action("clear", move |a| cp(a).clear_expression());
        self.add_action("copy", move |a| cp(a).copy_result(false));
        self.add_action("copyascii", move |a| cp(a).copy_result(true));
        self.add_action("store", move |a| cp(a).store_result());
        self.add_action("newfunction", |a| dialogs::function_edit(a, None, ""));
        self.add_action("newvariable", |a| {
            let init = a.calc().last_ascii();
            dialogs::variable_edit(a, None, &init);
        });
        self.add_action("newunknown", |a| dialogs::unknown_edit(a, None));
        self.add_action("newunit", |a| dialogs::unit_edit(a, None));
        self.add_action("newmatrix", move |a| cp(a).open_matrix_editor());
        self.add_action("insertdate", move |a| cp(a).insert_date());
        self.add_action("wsopen", |a| {
            let a2 = a.clone();
            open_file(a.parent(), "Open Workspace", move |p| a2.open_workspace(&p.to_string_lossy()));
        });
        self.add_action("wssave", |a| {
            let ws = a.settings.borrow().workspace.clone();
            if ws.is_empty() {
                a.save_workspace_as();
            } else {
                a.save_workspace(&ws);
            }
        });
        self.add_action("wssaveas", |a| a.save_workspace_as());
        self.add_action("wsdefault", |a| a.open_default_workspace());
        self.add_string_action("wsrecent", |a, p| a.open_workspace(p));
        self.add_action("csvimport", |a| dialogs::csv_import(a));
        self.add_action("csvexport", |a| dialogs::csv_export(a));
        for (i, id) in ["calculator", "functions", "variables", "units", "datasets", "periodic", "plot", "tools", "settings"].iter().enumerate() {
            self.add_action(id, move |a| a.show_page(i));
        }
        self.add_action("plotexpr", |a| {
            let t = a.calc().expression_text();
            a.open_plot(if t.trim().is_empty() { "x" } else { &t }, -10.0, 10.0);
        });
        for (i, id) in ["percentage", "calendar", "fpconv"].iter().enumerate() {
            self.add_action(id, move |a| {
                a.show_page(7);
                if let Some(p) = a.pages.borrow().get(&7) {
                    if let Some(t) = pages::tools::ToolsPage::from_page(p) {
                        t.select(i);
                    }
                }
            });
        }
        self.add_action("shortcuts", |a| {
            a.show_page(8);
            if let Some(p) = a.pages.borrow().get(&8) {
                if let Some(s) = pages::settings_page::SettingsPage::from_page(p) {
                    s.show_section(6);
                }
            }
        });
        self.add_action("rates", |a| a.update_rates(true));
        for (i, id) in ["history", "stack", "bases"].iter().enumerate() {
            self.add_action(id, move |a| {
                a.show_page(0);
                a.calc().select_side(i);
            });
        }
        self.add_action("keypad", |a| {
            {
                let mut s = a.settings.borrow_mut();
                s.keypad_visible = !s.keypad_visible;
            }
            a.calc().apply_layout();
        });
        self.add_action("sidepanel", |a| {
            {
                let mut s = a.settings.borrow_mut();
                s.side_visible = !s.side_visible;
            }
            a.calc().apply_layout();
        });
        self.add_action("rpn", |a| {
            let on = !a.settings.borrow().rpn_mode;
            a.settings.borrow_mut().rpn_mode = on;
            a.svc.set_opts(|o| {
                if on && o.parse == ParseMode::Chain {
                    o.parse = ParseMode::Adaptive;
                }
            });
            a.options_changed();
            if on {
                a.calc().select_side(1);
            }
        });
        self.add_action("chain", |a| {
            a.settings.borrow_mut().rpn_mode = false;
            a.svc.set_opts(|o| o.parse = if o.parse == ParseMode::Chain { ParseMode::Adaptive } else { ParseMode::Chain });
            a.options_changed();
        });
        self.add_action("normal", |a| {
            a.settings.borrow_mut().rpn_mode = false;
            a.svc.set_opts(|o| {
                if o.parse == ParseMode::Chain {
                    o.parse = ParseMode::Adaptive;
                }
            });
            a.options_changed();
        });
        for (id, m) in [("degrees", AngleMode::Degrees), ("radians", AngleMode::Radians), ("gradians", AngleMode::Gradians)] {
            self.add_action(id, move |a| {
                a.svc.set_opts(|o| o.angle = m);
                a.options_changed();
            });
        }
        self.add_action("exact", |a| {
            a.svc.set_opts(|o| o.approx = if o.approx == ApproxMode::Exact { ApproxMode::Auto } else { ApproxMode::Exact });
            a.options_changed();
        });
        self.add_action("fraction", |a| {
            a.svc.set_opts(|o| o.fraction = if o.fraction == FractionMode::Fraction { FractionMode::DecimalExact } else { FractionMode::Fraction });
            a.options_changed();
        });
        self.add_action("scientific", |a| {
            a.svc.set_opts(|o| o.display = if o.display == NumberDisplay::Scientific { NumberDisplay::Normal } else { NumberDisplay::Scientific });
            a.options_changed();
        });
        for (id, t) in [("tohex", "hex"), ("tobin", "bin"), ("todec", "dec"), ("tofraction", "fraction"), ("tofactors", "factors"), ("expand", "expand"), ("tobase", "base"), ("tooptimal", "optimal")] {
            self.add_action(id, move |a| a.calc().convert_to(t));
        }
        self.add_action("clearhistory", |a| a.calc().history.clear_unprotected());
        self.add_action("clearstack", |a| a.calc().stack_op("clear"));
        self.add_action("swap", |a| a.calc().stack_op("swap"));
        self.add_action("lastx", |a| a.calc().stack_op("lastx"));
        self.add_action("help", |a| dialogs::help(a));
        self.add_action("about", |a| dialogs::about(a));
        self.add_action("bug", |_| open_url("https://github.com/Orang-Studio/OrangCalc/issues"));
        self.add_action("settingsfolder", |_| open_url(&format!("file://{}", st::dir().display())));
        self.add_action("quit", |a| a.window.close());
        self.add_action("undo", |a| a.calc().expr.undo());
        self.add_action("redo", |a| a.calc().expr.redo());
        self.add_string_action("insert", |a, t| a.insert_into_expression(t, false));
        self.add_string_action("insertfn", |a, n| {
            let f = a.snap.borrow().function(n);
            if let Some(f) = f {
                dialogs::insert_function(a, &f);
            }
        });
        self.add_string_action("insertvar", |a, n| {
            App::add_recent(&mut a.settings.borrow_mut().recent_variables, n);
            a.insert_into_expression(n, false);
        });
        self.add_string_action("insertunit", |a, n| {
            App::add_recent(&mut a.settings.borrow_mut().recent_units, n);
            a.settings.borrow_mut().last_unit_button = n.to_string();
            a.calc().keypad.refresh_unit_key();
            a.insert_into_expression(n, false);
        });
        self.add_string_action("convert", |a, t| a.calc().convert_to(t));
        self.add_action("convertother", |a| {
            let a2 = a.clone();
            ask(a.parent(), "Convert", "Convert to (unit or format):", "", move |t| {
                if !t.trim().is_empty() {
                    a2.calc().convert_to(t.trim());
                }
            });
        });
        self.add_action("otherbase", |a| {
            let a2 = a.clone();
            ask(a.parent(), "Number Base", "Base:", "3", move |t| {
                if !t.trim().is_empty() {
                    a2.calc().convert_to(&format!("base {}", t.trim()));
                }
            });
        });
        self.add_action("othertz", |a| {
            let a2 = a.clone();
            ask(a.parent(), "Time Zone", "Offset from UTC (e.g. +3 or -5:30):", "+0", move |t| {
                if !t.trim().is_empty() {
                    a2.calc().convert_to(&format!("utc{}", t.trim()));
                }
            });
        });
        self.add_action("customprecision", |a| {
            let a2 = a.clone();
            let cur = a.svc.opts().precision;
            ask(a.parent(), "Precision", "Significant digits (2–5000):", &cur.to_string(), move |t| {
                if let Ok(v) = t.trim().parse::<i32>() {
                    if (2..=5000).contains(&v) {
                        a2.svc.set_opts(|o| o.precision = v);
                        a2.options_changed();
                    }
                }
            });
        });
        for input in [false, true] {
            self.add_action(if input { "custominbase" } else { "customoutbase" }, move |a| {
                let a2 = a.clone();
                ask(a.parent(), if input { "Expression Base" } else { "Result Base" }, "Base (2–36, negative, or an expression like 1.5 or pi):", "", move |t| {
                    let t = t.trim().to_string();
                    if t.is_empty() {
                        return;
                    }
                    match t.parse::<i32>() {
                        Ok(v) if v.abs() >= 2 => a2.svc.set_opts(|o| if input { o.input_base = v } else { o.output_base = v }),
                        _ if !input => a2.svc.set_opts(|o| {
                            o.output_base = bases::CUSTOM;
                            o.output_base_custom = t.clone();
                        }),
                        _ => {}
                    }
                    a2.options_changed();
                });
            });
        }
    }

    fn register_option_actions(self: &Rc<Self>) {
        for d in opt_defs() {
            let a = gio::SimpleAction::new_stateful(d.name, Some(glib::VariantTy::STRING), &(d.get)(self).to_variant());
            let app = Rc::downgrade(self);
            let set = d.set;
            a.connect_activate(move |act, p| {
                let (Some(app), Some(v)) = (app.upgrade(), p.and_then(|p| p.str().map(|s| s.to_string()))) else { return };
                act.set_state(&v.to_variant());
                if app.syncing.get() {
                    return;
                }
                set(&app, &v);
                app.options_changed();
            });
            self.window.add_action(&a);
        }
        for d in bool_defs() {
            let a = gio::SimpleAction::new_stateful(d.name, None, &(d.get)(self).to_variant());
            let app = Rc::downgrade(self);
            let set = d.set;
            a.connect_activate(move |act, _| {
                let Some(app) = app.upgrade() else { return };
                let v = !act.state().and_then(|s| s.get::<bool>()).unwrap_or(false);
                act.set_state(&v.to_variant());
                set(&app, v);
                app.options_changed();
            });
            self.window.add_action(&a);
        }
    }

    // keeps the menu check marks in line with options changed elsewhere
    fn sync_option_actions(self: &Rc<Self>) {
        self.syncing.set(true);
        for d in opt_defs() {
            if let Some(a) = self.window.lookup_action(d.name).and_then(|a| a.downcast::<gio::SimpleAction>().ok()) {
                a.set_state(&(d.get)(self).to_variant());
            }
        }
        for d in bool_defs() {
            if let Some(a) = self.window.lookup_action(d.name).and_then(|a| a.downcast::<gio::SimpleAction>().ok()) {
                a.set_state(&(d.get)(self).to_variant());
            }
        }
        self.syncing.set(false);
    }

    pub fn shortcut_for(&self, a: &ActionDef) -> String {
        self.settings.borrow().shortcuts.get(a.id).cloned().unwrap_or_else(|| a.default_keys.to_string())
    }

    pub fn apply_shortcuts(&self) {
        for a in &self.actions {
            let keys = self.shortcut_for(a);
            let accels: Vec<String> = to_accel(&keys).into_iter().collect();
            let refs: Vec<&str> = accels.iter().map(|s| s.as_str()).collect();
            self.gapp.set_accels_for_action(&format!("win.{}", a.id), &refs);
        }
    }

    fn build_menu(&self) -> gio::Menu {
        let bar = gio::Menu::new();
        let file = gio::Menu::new();
        let new = gio::Menu::new();
        for (l, a) in [("Function…", "newfunction"), ("Variable/Constant…", "newvariable"), ("Unknown Variable…", "newunknown"), ("Matrix…", "newmatrix"), ("Unit…", "newunit")] {
            new.append(Some(l), Some(&format!("win.{}", a)));
        }
        file.append_submenu(Some("New"), &new);
        let ws = gio::Menu::new();
        let ws1 = gio::Menu::new();
        ws1.append(Some("Open…"), Some("win.wsopen"));
        ws1.append(Some("Open Default"), Some("win.wsdefault"));
        section(&ws, &ws1);
        let ws2 = gio::Menu::new();
        ws2.append(Some("Save"), Some("win.wssave"));
        ws2.append(Some("Save As…"), Some("win.wssaveas"));
        section(&ws, &ws2);
        section(&ws, &self.ws_recent);
        file.append_submenu(Some("Workspaces"), &ws);
        let f2 = gio::Menu::new();
        f2.append(Some("Import CSV File…"), Some("win.csvimport"));
        f2.append(Some("Export CSV File…"), Some("win.csvexport"));
        section(&file, &f2);
        let f3 = gio::Menu::new();
        f3.append(Some("Store Result…"), Some("win.store"));
        section(&file, &f3);
        let f4 = gio::Menu::new();
        f4.append(Some("Quit"), Some("win.quit"));
        section(&file, &f4);
        bar.append_submenu(Some("_File"), &file);

        let edit = gio::Menu::new();
        let groups: [&[(&str, &str)]; 4] = [
            &[("Undo", "undo"), ("Redo", "redo")],
            &[("Copy Result", "copy"), ("Copy Unformatted ASCII", "copyascii")],
            &[("Insert Date…", "insertdate"), ("Insert Matrix…", "newmatrix")],
            &[("Clear Expression", "clear"), ("Clear History", "clearhistory"), ("Clear RPN Stack", "clearstack")],
        ];
        for g in groups {
            let s = gio::Menu::new();
            for (l, a) in g {
                s.append(Some(l), Some(&format!("win.{}", a)));
            }
            section(&edit, &s);
        }
        bar.append_submenu(Some("_Edit"), &edit);

        let mode = gio::Menu::new();
        let modes = gio::Menu::new();
        for id in ["mode", "angle", "approx", "display", "base", "inbase", "fraction", "complex", "assumptions", "keys"] {
            modes.append_submenu(Some(mode_title(id)), &mode_menu(id));
        }
        section(&mode, &modes);
        let m2 = gio::Menu::new();
        m2.append(Some("Keep Above Other Windows"), Some("win.topmost"));
        m2.append(Some("Keyboard Shortcuts"), Some("win.shortcuts"));
        m2.append(Some("Preferences"), Some("win.settings"));
        section(&mode, &m2);
        bar.append_submenu(Some("_Mode"), &mode);

        bar.append_submenu(Some("F_unctions"), &self.fn_menu);
        bar.append_submenu(Some("_Variables"), &self.var_menu);
        bar.append_submenu(Some("U_nits"), &self.unit_menu);

        let tools = gio::Menu::new();
        let t1 = gio::Menu::new();
        for (l, a) in [("Functions", "functions"), ("Variables and Constants", "variables"), ("Units", "units"), ("Data Sets", "datasets")] {
            t1.append(Some(l), Some(&format!("win.{}", a)));
        }
        section(&tools, &t1);
        let t2 = gio::Menu::new();
        for (l, a) in [("Plot Functions/Data", "plot"), ("Floating Point Conversion (IEEE 754)", "fpconv"), ("Calendar Conversion", "calendar"), ("Percentage Calculation Tool", "percentage"), ("Periodic Table", "periodic")] {
            t2.append(Some(l), Some(&format!("win.{}", a)));
        }
        section(&tools, &t2);
        let t3 = gio::Menu::new();
        t3.append(Some("Update Exchange Rates"), Some("win.rates"));
        section(&tools, &t3);
        let t4 = gio::Menu::new();
        t4.append(Some("Open Settings Folder"), Some("win.settingsfolder"));
        section(&tools, &t4);
        bar.append_submenu(Some("_Tools"), &tools);

        let help = gio::Menu::new();
        let h1 = gio::Menu::new();
        h1.append(Some("Help"), Some("win.help"));
        h1.append(Some("Report a Bug"), Some("win.bug"));
        section(&help, &h1);
        let h2 = gio::Menu::new();
        h2.append(Some("About OrangCalc"), Some("win.about"));
        section(&help, &h2);
        bar.append_submenu(Some("_Help"), &help);
        bar
    }

    fn rebuild_recent_workspaces(&self) {
        self.ws_recent.remove_all();
        for r in self.settings.borrow().recent_workspaces.iter().filter(|p| std::path::Path::new(p).exists()).take(8) {
            let name = std::path::Path::new(r).file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            let it = gio::MenuItem::new(Some(&name), None);
            it.set_action_and_target_value(Some("win.wsrecent"), Some(&r.to_variant()));
            self.ws_recent.append_item(&it);
        }
    }

    pub fn functions_menu(&self) -> gio::Menu {
        let snap = self.snap.borrow();
        let s = self.settings.borrow();
        let m = gio::Menu::new();
        let item = |f: &FnRef| {
            let it = gio::MenuItem::new(Some(&format!("{}  ({})", f.item.display_title(), f.item.names[0])), None);
            it.set_action_and_target_value(Some("win.insertfn"), Some(&f.item.names[0].to_variant()));
            it
        };
        let top = gio::Menu::new();
        for (title, list) in [("Favorites", &s.favorite_functions), ("Recent", &s.recent_functions)] {
            let fs: Vec<FnRef> = list.iter().filter_map(|n| snap.function(n)).collect();
            if !fs.is_empty() {
                let sub = gio::Menu::new();
                for f in &fs {
                    sub.append_item(&item(f));
                }
                top.append_submenu(Some(title), &sub);
            }
        }
        section(&m, &top);
        let cats = gio::Menu::new();
        let mut groups: std::collections::BTreeMap<String, Vec<FnRef>> = Default::default();
        for f in snap.functions.iter().filter(|f| f.item.active() && !f.item.hidden && !f.name().starts_with('_')) {
            groups.entry(if f.item.category.is_empty() { "Other".into() } else { f.item.category.clone() }).or_default().push(f.clone());
        }
        for (cat, mut fs) in groups {
            fs.sort_by(|a, b| a.item.display_title().cmp(b.item.display_title()));
            let sub = gio::Menu::new();
            for f in &fs {
                sub.append_item(&item(f));
            }
            cats.append_submenu(Some(&cat), &sub);
        }
        section(&m, &cats);
        let end = gio::Menu::new();
        end.append(Some("New Function…"), Some("win.newfunction"));
        end.append(Some("Show All Functions"), Some("win.functions"));
        section(&m, &end);
        m
    }

    pub fn variables_menu(&self) -> gio::Menu {
        let snap = self.snap.borrow();
        let s = self.settings.borrow();
        let m = gio::Menu::new();
        let item = |v: &VarRef| {
            let it = gio::MenuItem::new(Some(&format!("{}  ({})", v.item.display_title(), v.item.names[0])), None);
            it.set_action_and_target_value(Some("win.insertvar"), Some(&v.item.names[0].to_variant()));
            it
        };
        let top = gio::Menu::new();
        for (title, list) in [("Favorites", &s.favorite_variables), ("Recent", &s.recent_variables)] {
            let vs: Vec<VarRef> = list.iter().filter_map(|n| snap.variable(n)).collect();
            if !vs.is_empty() {
                let sub = gio::Menu::new();
                for v in &vs {
                    sub.append_item(&item(v));
                }
                top.append_submenu(Some(title), &sub);
            }
        }
        section(&m, &top);
        let cats = gio::Menu::new();
        let mut groups: std::collections::BTreeMap<String, Vec<VarRef>> = Default::default();
        for v in snap.variables.iter().filter(|v| v.item.active() && !v.item.hidden) {
            groups.entry(if v.item.category.is_empty() { "Other".into() } else { v.item.category.clone() }).or_default().push(v.clone());
        }
        for (cat, mut vs) in groups {
            vs.sort_by(|a, b| a.item.display_title().cmp(b.item.display_title()));
            let sub = gio::Menu::new();
            for v in &vs {
                sub.append_item(&item(v));
            }
            cats.append_submenu(Some(&cat), &sub);
        }
        section(&m, &cats);
        let end = gio::Menu::new();
        end.append(Some("New Variable…"), Some("win.newvariable"));
        end.append(Some("New Unknown Variable…"), Some("win.newunknown"));
        end.append(Some("Show All Variables"), Some("win.variables"));
        section(&m, &end);
        m
    }

    pub fn units_menu(&self) -> gio::Menu {
        let snap = self.snap.borrow();
        let s = self.settings.borrow();
        let m = gio::Menu::new();
        let item = |u: &UnitRef| {
            let it = gio::MenuItem::new(Some(&format!("{}  ({})", u.item.display_title(), u.abbrev())), None);
            it.set_action_and_target_value(Some("win.insertunit"), Some(&u.abbrev().to_variant()));
            it
        };
        let top = gio::Menu::new();
        for (title, list) in [("Favorites", &s.favorite_units), ("Recent", &s.recent_units)] {
            let us: Vec<UnitRef> = list.iter().filter_map(|n| snap.unit(n)).collect();
            if !us.is_empty() {
                let sub = gio::Menu::new();
                for u in &us {
                    sub.append_item(&item(u));
                }
                top.append_submenu(Some(title), &sub);
            }
        }
        section(&m, &top);
        let cats = gio::Menu::new();
        for (cat, us) in unit_groups(&snap) {
            let sub = gio::Menu::new();
            for u in &us {
                sub.append_item(&item(u));
            }
            cats.append_submenu(Some(&cat), &sub);
        }
        let pre = gio::Menu::new();
        for p in PREFIXES.iter() {
            let it = gio::MenuItem::new(Some(&format!("{} ({}, {}{})", p.name, p.symbol, if p.binary { "2^" } else { "10^" }, p.exp)), None);
            it.set_action_and_target_value(Some("win.insert"), Some(&p.symbol.to_variant()));
            pre.append_item(&it);
        }
        cats.append_submenu(Some("Prefixes"), &pre);
        section(&m, &cats);
        let end = gio::Menu::new();
        end.append(Some("New Unit…"), Some("win.newunit"));
        end.append(Some("Show All Units"), Some("win.units"));
        section(&m, &end);
        m
    }

    pub fn conversion_menu(&self) -> gio::Menu {
        let m = gio::Menu::new();
        let conv = |menu: &gio::Menu, label: &str, target: &str| {
            let it = gio::MenuItem::new(Some(label), None);
            it.set_action_and_target_value(Some("win.convert"), Some(&target.to_variant()));
            menu.append_item(&it);
        };
        let bases = gio::Menu::new();
        let b1 = gio::Menu::new();
        for (l, t) in [("Binary", "bin"), ("Octal", "oct"), ("Decimal", "dec"), ("Duodecimal", "duo"), ("Hexadecimal", "hex")] {
            conv(&b1, l, t);
        }
        section(&bases, &b1);
        let b2 = gio::Menu::new();
        for (l, t) in [("Sexagesimal", "sexagesimal"), ("Time Format", "time"), ("Roman Numerals", "roman"), ("Bijective Base-26", "bijective"), ("Binary-Coded Decimal", "bcd"), ("Unicode", "unicode"), ("Latitude", "latitude"), ("Longitude", "longitude")] {
            conv(&b2, l, t);
        }
        section(&bases, &b2);
        let b3 = gio::Menu::new();
        for (l, t) in [("16-bit Floating Point", "fp16"), ("32-bit Floating Point", "fp32"), ("64-bit Floating Point", "fp64"), ("80-bit (x86) Floating Point", "fp80"), ("128-bit Floating Point", "fp128")] {
            conv(&b3, l, t);
        }
        section(&bases, &b3);
        let b4 = gio::Menu::new();
        conv(&b4, "All Number Bases", "bases");
        b4.append(Some("Other Base…"), Some("win.otherbase"));
        section(&bases, &b4);
        m.append_submenu(Some("Number Base"), &bases);
        let fmt = gio::Menu::new();
        for (l, t) in [("Fraction", "fraction"), ("Mixed Fraction", "mixed"), ("Decimal Fraction", "decimals"), ("Percent", "percent"), ("Scientific Notation", "scientific"), ("Engineering Notation", "engineering"), ("Simple Notation", "simple")] {
            conv(&fmt, l, t);
        }
        m.append_submenu(Some("Number Format"), &fmt);
        let alg = gio::Menu::new();
        for (l, t) in [("Factors", "factors"), ("Expanded", "expand"), ("Expanded Partial Fractions", "partial fraction"), ("Exact", "exact"), ("Approximate", "approximate")] {
            conv(&alg, l, t);
        }
        m.append_submenu(Some("Algebra"), &alg);
        let cx = gio::Menu::new();
        for (l, t) in [("Rectangular", "rectangular"), ("Exponential", "exponential"), ("Polar", "polar"), ("Cis", "cis"), ("Angle/Phasor", "angle")] {
            conv(&cx, l, t);
        }
        m.append_submenu(Some("Complex Form"), &cx);
        let tm = gio::Menu::new();
        conv(&tm, "Calendars", "calendars");
        conv(&tm, "UTC Time Zone", "utc");
        tm.append(Some("Other Time Zone…"), Some("win.othertz"));
        m.append_submenu(Some("Date and Time"), &tm);
        let u = gio::Menu::new();
        for (l, t) in [("Optimal Unit", "optimal"), ("Base Units", "base"), ("Optimal Prefix", "prefix"), ("Mixed Units", "mixed")] {
            conv(&u, l, t);
        }
        let units = gio::Menu::new();
        for (cat, us) in unit_groups(&self.snap.borrow()) {
            let sub = gio::Menu::new();
            for un in &us {
                conv(&sub, &format!("{} ({})", un.item.display_title(), un.abbrev()), un.abbrev());
            }
            units.append_submenu(Some(&cat), &sub);
        }
        u.append_submenu(Some("Unit"), &units);
        u.append(Some("Other…"), Some("win.convertother"));
        section(&m, &u);
        m
    }

    pub fn rebuild_item_menus(&self) {
        for (dst, src) in [(&self.fn_menu, self.functions_menu()), (&self.var_menu, self.variables_menu()), (&self.unit_menu, self.units_menu())] {
            dst.remove_all();
            for i in 0..src.n_items() {
                let it = gio::MenuItem::from_model(&src, i);
                dst.append_item(&it);
            }
        }
    }

    pub fn open_plot(self: &Rc<Self>, expr: &str, a: f64, b: f64) {
        self.show_page(6);
        if let Some(p) = self.pages.borrow().get(&6) {
            if let Some(pp) = pages::plot::PlotPage::from_page(p) {
                pp.add_series(expr, a, b);
            }
        }
    }

    // workspaces: definitions, history, modes and stack in one file
    fn save_workspace_as(self: &Rc<Self>) {
        let a = self.clone();
        save_file(self.parent(), "Save Workspace", "workspace.ocws", move |p| {
            let mut s = p.to_string_lossy().to_string();
            if !s.ends_with(".ocws") {
                s += ".ocws";
            }
            a.save_workspace(&s);
        });
    }

    pub fn save_workspace(self: &Rc<Self>, path: &str) {
        let mut root = Node::new("workspace");
        let mut opts = Node::new("options");
        st::write_settings(&mut opts, None, Some(&self.svc.opts()));
        root.push(opts);
        let defs = self.svc.run_sync(|c| {
            let mut n = Node::new("definitions");
            useritems::save(c, &mut n);
            n
        });
        root.push(defs.unwrap_or_else(|_| Node::new("definitions")));
        let calc = self.calc();
        let mut hist = Node::new("history");
        st::write_history(&mut hist, calc.history.entries.borrow().iter());
        root.push(hist);
        let stack = calc.stack.borrow().clone();
        let o = self.svc.opts().clone();
        let texts = self.svc.run_sync(move |_| stack.iter().map(|e| formatter::ascii(e, &o)).collect::<Vec<_>>()).unwrap_or_default();
        let mut stn = Node::new("stack");
        for t in texts {
            let mut v = Node::new("value");
            v.text = t;
            stn.push(v);
        }
        root.push(stn);
        match std::fs::write(path, root.to_string_doc()) {
            Ok(_) => {
                {
                    let mut s = self.settings.borrow_mut();
                    s.workspace = path.to_string();
                    App::add_recent(&mut s.recent_workspaces, path);
                }
                self.rebuild_recent_workspaces();
                let t = self.last_result_title.borrow().clone();
                self.update_title(&t);
                self.mark_dirty();
            }
            Err(e) => message(self.parent(), &format!("Couldn't save workspace:\n{}", e)),
        }
    }

    pub fn open_workspace(self: &Rc<Self>, path: &str) {
        let root = match std::fs::read_to_string(path).map_err(|e| e.to_string()).and_then(|t| Node::parse(&t)) {
            Ok(r) => r,
            Err(e) => {
                message(self.parent(), &format!("Failed to open workspace:\n{}", e));
                return;
            }
        };
        if let Some(opts) = root.child("options") {
            let mut o = self.svc.opts().clone();
            st::read_settings(opts, None, Some(&mut o));
            self.svc.set_opts(|x| *x = o);
        }
        let defs = root.child("definitions").cloned();
        let _ = self.svc.run_sync(move |c| {
            useritems::clear_user(c);
            if let Some(d) = defs {
                useritems::load(c, &d);
            }
        });
        let calc = self.calc();
        {
            let mut h = calc.history.entries.borrow_mut();
            h.clear();
            if let Some(hn) = root.child("history") {
                h.extend(st::read_history(hn));
            }
        }
        calc.history.refill();
        let values: Vec<String> = root.child("stack").map(|s| s.all("value").map(|v| v.text.clone()).collect()).unwrap_or_default();
        let stack = self.svc.run_sync(move |c| values.iter().filter_map(|v| attempt(|| c.eval_text(v)).ok()).collect::<Vec<_>>()).unwrap_or_default();
        *calc.stack.borrow_mut() = stack;
        calc.refresh_rpn();
        {
            let mut s = self.settings.borrow_mut();
            s.workspace = path.to_string();
            App::add_recent(&mut s.recent_workspaces, path);
        }
        self.rebuild_recent_workspaces();
        self.options_changed();
        self.definitions_changed();
        self.update_title("");
    }

    fn open_default_workspace(self: &Rc<Self>) {
        self.settings.borrow_mut().workspace.clear();
        let _ = self.svc.run_sync(|c| {
            useritems::clear_user(c);
            st::load_definitions(c);
        });
        let calc = self.calc();
        *calc.history.entries.borrow_mut() = st::load_history();
        calc.history.refill();
        self.definitions_changed();
        self.update_title("");
    }

    fn maybe_update_rates(self: &Rc<Self>) {
        let (days, last) = {
            let s = self.settings.borrow();
            (s.rates_update_days, s.last_rates_update.clone())
        };
        if days <= 0 {
            return;
        }
        if let Ok(t) = chrono::NaiveDateTime::parse_from_str(&last, "%Y-%m-%dT%H:%M:%S") {
            if (chrono::Local::now().naive_local() - t).num_days() < days as i64 {
                return;
            }
        }
        self.update_rates(false);
    }

    // exchange rates from the European Central Bank, fetched with curl in the background
    pub fn update_rates(self: &Rc<Self>, interactive: bool) {
        let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<(String, f64)>, String>>();
        std::thread::spawn(move || {
            let _ = tx.send(download_rates());
        });
        let a = self.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(250), move || match rx.try_recv() {
            Ok(Ok(rates)) => {
                let n = rates.len();
                a.svc.post(move |c| units::set_rates(c, &rates));
                a.settings.borrow_mut().last_rates_update = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
                a.mark_dirty();
                a.definitions_changed();
                if interactive {
                    message(a.parent(), &format!("Exchange rates were updated ({} currencies).", n));
                }
                glib::ControlFlow::Break
            }
            Ok(Err(e)) => {
                if interactive {
                    message(a.parent(), &format!("Failed to download exchange rates:\n{}", e));
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => glib::ControlFlow::Break,
        });
    }
}

pub fn unit_groups(snap: &Snapshot) -> Vec<(String, Vec<UnitRef>)> {
    let mut groups: std::collections::BTreeMap<String, Vec<UnitRef>> = Default::default();
    for u in snap.units.iter().filter(|u| u.item.active() && !u.item.hidden) {
        groups.entry(u.item.category.clone()).or_default().push(u.clone());
    }
    groups
        .into_iter()
        .map(|(k, mut v)| {
            v.sort_by(|a, b| a.item.display_title().cmp(b.item.display_title()));
            (k, v)
        })
        .collect()
}

fn mode_title(id: &str) -> &'static str {
    match id {
        "mode" => "Calculation Mode",
        "angle" => "Angle Unit",
        "approx" => "Approximation",
        "display" => "Number Display",
        "base" => "Result Base",
        "inbase" => "Expression Base",
        "fraction" => "Fractions",
        "complex" => "Complex Form",
        "assumptions" => "Assumptions",
        _ => "Keypad",
    }
}

fn base_items(m: &gio::Menu, action: &str, input: bool, other_label: &str) {
    let main = gio::Menu::new();
    for (b, n) in [(2, "Binary"), (8, "Octal"), (10, "Decimal"), (16, "Hexadecimal")] {
        radio(&main, n, action, &b.to_string());
    }
    section(m, &main);
    let other = gio::Menu::new();
    let list: Vec<(i32, &str)> = if input {
        vec![(12, "Duodecimal"), (bases::ROMAN, "Roman Numerals")]
    } else {
        vec![
            (12, "Duodecimal"),
            (bases::SEXAGESIMAL, "Sexagesimal"),
            (bases::TIME, "Time Format"),
            (bases::ROMAN, "Roman Numerals"),
            (bases::UNICODE, "Unicode"),
            (bases::BIJECTIVE26, "Bijective Base-26"),
            (bases::BCD, "Binary-Coded Decimal (BCD)"),
            (bases::FP32, "Float (32-bit)"),
            (bases::FP64, "Double (64-bit)"),
            (bases::GOLDEN, "Golden Ratio (φ)"),
            (bases::SUPERGOLDEN, "Supergolden Ratio (ψ)"),
            (bases::PI_BASE, "π"),
            (bases::E_BASE, "e"),
            (bases::SQRT2, "√2"),
        ]
    };
    for (b, n) in list {
        radio(&other, n, action, &b.to_string());
    }
    other.append(Some("Custom…"), Some(if input { "win.custominbase" } else { "win.customoutbase" }));
    m.append_submenu(Some(other_label), &other);
}

pub fn mode_menu(id: &str) -> gio::Menu {
    let m = gio::Menu::new();
    match id {
        "mode" => {
            let s = gio::Menu::new();
            for (l, v) in [("Normal Mode", "Normal"), ("RPN Mode", "RPN"), ("Chain Mode", "Chain")] {
                radio(&s, l, "calcmode", v);
            }
            section(&m, &s);
            let rest = gio::Menu::new();
            let pm = gio::Menu::new();
            for (l, v) in [("Adaptive", "Adaptive"), ("Conventional", "Conventional"), ("Implicit Multiplication First", "ImplicitFirst"), ("RPN Syntax", "Rpn")] {
                radio(&pm, l, "parsemode", v);
            }
            rest.append_submenu(Some("Parsing Mode"), &pm);
            let tm = gio::Menu::new();
            for v in ["Hybrid", "Absolute", "Relative"] {
                radio(&tm, v, "temperature", v);
            }
            rest.append_submenu(Some("Temperature Calculation"), &tm);
            let pc = gio::Menu::new();
            radio(&pc, "Add percentage of original value (100 + 10% = 110)", "percent", "simple");
            radio(&pc, "Add percentage multiplied by 1/100 (100 + 10% = 100.1)", "percent", "plain");
            rest.append_submenu(Some("Percentage Interpretation"), &pc);
            let iv = gio::Menu::new();
            for (l, v) in [("Variance Formula", "VarianceFormula"), ("Interval Arithmetic", "IntervalArithmetic"), ("Simple", "Simple")] {
                radio(&iv, l, "intervalcalc", v);
            }
            rest.append_submenu(Some("Interval Calculation"), &iv);
            let stm = gio::Menu::new();
            for v in ["Simplify", "Factorize", "Expand", "None"] {
                radio(&stm, v, "structure", v);
            }
            rest.append_submenu(Some("Result Structure"), &stm);
            section(&m, &rest);
        }
        "angle" => {
            for v in ["Radians", "Degrees", "Gradians", "None"] {
                radio(&m, v, "angle", v);
            }
        }
        "approx" => {
            let s = gio::Menu::new();
            for (l, v) in [("Automatic", "Auto"), ("Dual", "Dual"), ("Exact", "Exact"), ("Approximate", "Approximate")] {
                radio(&s, l, "approx", v);
            }
            section(&m, &s);
            let s2 = gio::Menu::new();
            s2.append_item(&mi("Interval Arithmetic", "win.intervalarith"));
            section(&m, &s2);
        }
        "display" => {
            let s = gio::Menu::new();
            for (l, v) in [("Normal", "Normal"), ("Scientific", "Scientific"), ("Purely Scientific", "PureScientific"), ("Engineering", "Engineering"), ("Simple", "Simple")] {
                radio(&s, l, "display", v);
            }
            section(&m, &s);
            let s2 = gio::Menu::new();
            let prec = gio::Menu::new();
            for p in [6, 8, 10, 12, 15, 20, 30, 50, 100] {
                radio(&prec, &p.to_string(), "precision", &p.to_string());
            }
            prec.append(Some("Custom…"), Some("win.customprecision"));
            s2.append_submenu(Some("Precision"), &prec);
            let maxd = gio::Menu::new();
            radio(&maxd, "Off", "maxdec", "-1");
            for p in [0, 1, 2, 3, 4, 6, 8] {
                radio(&maxd, &p.to_string(), "maxdec", &p.to_string());
            }
            s2.append_submenu(Some("Max Decimals"), &maxd);
            let mind = gio::Menu::new();
            for p in [0, 1, 2, 3, 4, 6, 8] {
                radio(&mind, &p.to_string(), "mindec", &p.to_string());
            }
            s2.append_submenu(Some("Min Decimals"), &mind);
            let ivd = gio::Menu::new();
            for d in IntervalDisplay::ALL {
                radio(&ivd, &words(d.name()), "intervalout", d.name());
            }
            s2.append_submenu(Some("Interval Display"), &ivd);
            section(&m, &s2);
        }
        "base" => {
            base_items(&m, "outbase", false, "Other Result Bases");
            let s = gio::Menu::new();
            let tc = gio::Menu::new();
            for (l, v) in [("Off", "Off"), ("Binary", "Binary"), ("Binary and Hexadecimal", "BinaryHex")] {
                radio(&tc, l, "twos", v);
            }
            s.append_submenu(Some("Two's Complement"), &tc);
            let bits = gio::Menu::new();
            for b in [0, 8, 16, 32, 64, 128] {
                radio(&bits, if b == 0 { "Automatic".into() } else { b.to_string() }.as_str(), "binbits", &b.to_string());
            }
            s.append_submenu(Some("Binary Bits"), &bits);
            section(&m, &s);
            let s2 = gio::Menu::new();
            let ib = gio::Menu::new();
            base_items(&ib, "inbase", true, "More Input Bases");
            s2.append_submenu(Some("Expression Base"), &ib);
            section(&m, &s2);
        }
        "inbase" => base_items(&m, "inbase", true, "Other Input Bases"),
        "fraction" => {
            let s = gio::Menu::new();
            for (l, v) in [("Decimal", "Decimal"), ("Exact Decimal", "DecimalExact"), ("Fraction", "Fraction"), ("Mixed Fraction", "Combined"), ("Percent", "Percent"), ("Per Mille", "Permille"), ("Per Myriad", "Permyriad")] {
                radio(&s, l, "fractionmode", v);
            }
            section(&m, &s);
            let rep = gio::Menu::new();
            for (l, v) in [("Off", "None"), ("Ellipsis", "Ellipsis"), ("Overline", "Overline")] {
                radio(&rep, l, "repeating", v);
            }
            let s2 = gio::Menu::new();
            s2.append_submenu(Some("Indicate Repeating Decimals"), &rep);
            section(&m, &s2);
        }
        "complex" => {
            let s = gio::Menu::new();
            for (l, v) in [("Rectangular", "Rectangular"), ("Exponential", "Exponential"), ("Polar", "Polar"), ("Cis", "Cis"), ("Angle/Phasor", "Angle")] {
                radio(&s, l, "complexform", v);
            }
            section(&m, &s);
            let s2 = gio::Menu::new();
            s2.append_item(&mi("Allow Complex Numbers", "win.complexenabled"));
            s2.append_item(&mi("Allow Infinite Numbers", "win.infiniteenabled"));
            section(&m, &s2);
        }
        "assumptions" => {
            let s = gio::Menu::new();
            for t in AssumeType::ALL {
                radio(&s, t.name(), "assumetype", t.name());
            }
            section(&m, &s);
            let s2 = gio::Menu::new();
            for sg in AssumeSign::ALL {
                radio(&s2, &words(sg.name()), "assumesign", sg.name());
            }
            section(&m, &s2);
        }
        _ => {
            let s = gio::Menu::new();
            for km in KeypadMode::ALL {
                radio(&s, if *km == KeypadMode::NumberPadOnly { "Number Pad" } else { km.name() }, "keypadmode", km.name());
            }
            section(&m, &s);
            let s2 = gio::Menu::new();
            s2.append_item(&mi("Always Show Number Pad", "win.numpad"));
            s2.append_item(&mi("Side Panel on the Left", "win.sideleft"));
            section(&m, &s2);
        }
    }
    m
}

pub fn open_url(url: &str) {
    let _ = gio::AppInfo::launch_default_for_uri(url, gio::AppLaunchContext::NONE);
}

fn parse_rates(text: &str) -> Vec<(String, f64)> {
    let Ok(doc) = roxmltree::Document::parse(text) else { return vec![] };
    doc.descendants()
        .filter(|n| n.tag_name().name() == "Cube")
        .filter_map(|n| Some((n.attribute("currency")?.to_string(), n.attribute("rate")?.parse::<f64>().ok()?)))
        .collect()
}

fn load_rates(c: &Calculator) {
    if let Ok(t) = std::fs::read_to_string(st::rates_file()) {
        let rates = parse_rates(&t);
        if !rates.is_empty() {
            units::set_rates(c, &rates);
        }
    }
}

fn curl(url: &str) -> Result<String, String> {
    let out = std::process::Command::new("curl").args(["-fsSL", "--max-time", "20", "-A", "OrangCalc", url]).output().map_err(|e| format!("curl is required to download rates ({})", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// pls fetch
fn download_rates() -> Result<Vec<(String, f64)>, String> {
    let mut xml = curl("https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml")?;
    let mut rates = parse_rates(&xml);
    if rates.is_empty() {
        return Err("The rate file could not be read".into());
    }
    if let Ok(btc) = curl("https://api.coinbase.com/v2/prices/BTC-EUR/spot") {
        let re = regex::Regex::new(r#""amount"\s*:\s*"([0-9.]+)""#).unwrap();
        if let Some(p) = re.captures(&btc).and_then(|m| m[1].parse::<f64>().ok()).filter(|p| *p > 0.0) {
            let r = 1.0 / p;
            rates.push(("BTC".into(), r));
            if let Some(i) = xml.rfind("</gesmes:Envelope>") {
                xml.insert_str(i, &format!("<Cube currency=\"BTC\" rate=\"{}\"/>\n", r));
            }
        }
    }
    let _ = std::fs::write(st::rates_file(), xml);
    Ok(rates)
}