// periodic table with heat-map colorings, element details and a molar mass calculator
use crate::engine::data::ELEMENT_CLASSES;
use crate::engine::{DataObject, DataSet};
use crate::ui::app::{App, Page};
use crate::ui::widgets::*;
use gtk::prelude::*;
use gtk::{cairo, gdk, glib};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
const CLASS_COLORS: [(u8, u8, u8); 12] = [(84, 88, 96), (160, 78, 72), (166, 116, 60), (124, 98, 158), (156, 84, 130), (74, 106, 156), (62, 132, 140), (104, 136, 74), (68, 146, 102), (158, 148, 68), (98, 94, 168), (90, 94, 102)];
const COLORINGS: [&str; 8] = ["Color by category", "Color by phase (STP)", "Color by electronegativity", "Color by density", "Color by melting point", "Color by boiling point", "Color by atomic weight", "Color by discovery year"];
const STEPS: i32 = 7;

type Rgb = (f64, f64, f64);

fn rgb(c: (u8, u8, u8)) -> Rgb {
    (c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0)
}

fn lerp(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

// stepped scale, solid colors only
fn scale(t: f64) -> Rgb {
    let step = (t.clamp(0.0, 0.9999) * STEPS as f64).floor();
    let q = step / (STEPS - 1) as f64;
    let (low, mid, high) = (rgb((52, 84, 140)), rgb((70, 140, 110)), rgb((176, 100, 52)));
    if q < 0.5 {
        lerp(low, mid, q * 2.0)
    } else {
        lerp(mid, high, (q - 0.5) * 2.0)
    }
}

fn num(o: &DataObject, p: &str) -> Option<f64> {
    o.get(p)?.parse().ok()
}

fn pos(o: &DataObject) -> (i32, i32) {
    (o.get("x_pos").and_then(|v| v.parse().ok()).unwrap_or(0), o.get("y_pos").and_then(|v| v.parse().ok()).unwrap_or(0))
}

pub struct PeriodicPage {
    app: Weak<App>,
    root: gtk::Box,
    area: gtk::DrawingArea,
    set: Arc<DataSet>,
    selected: RefCell<Option<DataObject>>,
    hover: RefCell<Option<DataObject>>,
    coloring: Cell<usize>,
    highlight: Cell<i32>,
    filter: RefCell<String>,
    legend: gtk::Box,
    big_number: gtk::Label,
    big_symbol: gtk::Label,
    big_name: gtk::Label,
    big_class: gtk::Label,
    props: gtk::ListBox,
    prop_names: RefCell<Vec<String>>,
    formula: gtk::Entry,
    molar: gtk::Label,
}

impl Page for PeriodicPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn activated(&self) {
        self.area.grab_focus();
    }
}

impl PeriodicPage {
    pub fn new(app: &Rc<App>) -> Rc<PeriodicPage> {
        let set = app.snap.borrow().data_sets.iter().find(|d| d.item.name == "atom").cloned().unwrap_or_default();
        let root = vbox(10);
        margins(&root, 16);
        let header = hbox(10);
        let title = label("Periodic Table");
        title.add_css_class("oc-page-title");
        title.set_hexpand(true);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Find element"));
        let coloring = combo(&COLORINGS, 0);
        header.append(&title);
        header.append(&search);
        header.append(&coloring);
        root.append(&header);
        let body = hbox(12);
        body.set_vexpand(true);
        let left = vbox(8);
        left.set_hexpand(true);
        let area = gtk::DrawingArea::new();
        area.set_vexpand(true);
        area.set_hexpand(true);
        area.set_focusable(true);
        area.set_content_height(360);
        left.append(&area);
        let molar_row = hbox(8);
        let formula = entry("H2O");
        formula.add_css_class("oc-mono");
        formula.set_width_chars(16);
        let molar = gtk::Label::new(None);
        molar.add_css_class("heading");
        molar_row.append(&label("Molar mass of"));
        molar_row.append(&formula);
        molar_row.append(&molar);
        left.append(&molar_row);
        let legend = hbox(6);
        left.append(&legend);
        body.append(&left);
        let card = vbox(8);
        card.set_size_request(340, -1);
        card.set_hexpand(false);
        let top = hbox(16);
        let big_left = vbox(0);
        let big_number = dim("");
        let big_symbol = gtk::Label::new(None);
        big_symbol.add_css_class("title-1");
        big_symbol.set_xalign(0.0);
        let big_name = label("");
        big_name.add_css_class("title-4");
        big_left.append(&big_number);
        big_left.append(&big_symbol);
        big_left.append(&big_name);
        let big_class = dim("");
        big_class.set_valign(gtk::Align::Center);
        top.append(&big_left);
        top.append(&big_class);
        card.append(&top);
        let props = gtk::ListBox::new();
        props.add_css_class("rich-list");
        card.append(&scrolled(&props));
        let buttons = gtk::FlowBox::new();
        buttons.set_selection_mode(gtk::SelectionMode::None);
        card.append(&buttons);
        body.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        body.append(&card);
        root.append(&body);

        let page = Rc::new(PeriodicPage {
            app: Rc::downgrade(app),
            root,
            area,
            set,
            selected: RefCell::new(None),
            hover: RefCell::new(None),
            coloring: Cell::new(0),
            highlight: Cell::new(-1),
            filter: RefCell::new(String::new()),
            legend,
            big_number,
            big_symbol,
            big_name,
            big_class,
            props,
            prop_names: RefCell::new(vec![]),
            formula,
            molar,
        });
        let mk = |t: &str, primary: bool, f: fn(&Rc<PeriodicPage>)| {
            let w = Rc::downgrade(&page);
            let b = button(t, move || {
                if let Some(p) = w.upgrade() {
                    f(&p);
                }
            });
            if primary {
                b.add_css_class("suggested-action");
            }
            buttons.insert(&b, -1);
        };
        mk("Insert value", true, |p| p.insert_property(false));
        mk("Insert lookup", false, |p| p.insert_property(true));
        mk("Insert symbol", false, |p| {
            if let Some(o) = p.selected.borrow().clone() {
                p.app().insert_into_expression(o.get("symbol").unwrap_or(""), false);
            }
        });
        mk("Add to formula", false, |p| {
            if let Some(o) = p.selected.borrow().clone() {
                let t = format!("{}{}", p.formula.text(), o.get("symbol").unwrap_or(""));
                p.formula.set_text(&t);
            }
        });
        let w = Rc::downgrade(&page);
        let ins = button("Insert", move || {
            if let Some(p) = w.upgrade() {
                if let Ok(v) = p.molar_mass(&p.formula.text()) {
                    if v > 0.0 {
                        p.app().insert_into_expression(&format!("{} g/mol", fmt5(v)), false);
                    }
                }
            }
        });
        molar_row.append(&ins);

        let w = Rc::downgrade(&page);
        page.area.set_draw_func(move |_, cr, wd, ht| {
            if let Some(p) = w.upgrade() {
                p.draw(cr, wd as f64, ht as f64);
            }
        });
        let click = gtk::GestureClick::new();
        let w = Rc::downgrade(&page);
        click.connect_pressed(move |_, n, x, y| {
            let Some(p) = w.upgrade() else { return };
            p.area.grab_focus();
            if let Some(o) = p.hit_test(x, y) {
                if n == 2 {
                    p.insert_property(false);
                } else {
                    p.show_element(o);
                }
            }
        });
        page.area.add_controller(click);
        let motion = gtk::EventControllerMotion::new();
        let w = Rc::downgrade(&page);
        motion.connect_motion(move |_, x, y| {
            if let Some(p) = w.upgrade() {
                let h = p.hit_test(x, y);
                let changed = p.hover.borrow().as_ref().and_then(|o| o.get("number").map(|s| s.to_string())) != h.as_ref().and_then(|o| o.get("number").map(|s| s.to_string()));
                if changed {
                    p.area.set_cursor_from_name(if h.is_some() { Some("pointer") } else { None });
                    *p.hover.borrow_mut() = h;
                    p.area.queue_draw();
                }
            }
        });
        page.area.add_controller(motion);
        let key = gtk::EventControllerKey::new();
        let w = Rc::downgrade(&page);
        key.connect_key_pressed(move |_, k, _, _| match w.upgrade() {
            Some(p) => p.on_key(k),
            None => glib::Propagation::Proceed,
        });
        page.area.add_controller(key);
        let w = Rc::downgrade(&page);
        coloring.connect_selected_notify(move |c| {
            if let Some(p) = w.upgrade() {
                p.coloring.set(c.selected() as usize);
                p.highlight.set(-1);
                p.build_legend();
                p.area.queue_draw();
            }
        });
        let w = Rc::downgrade(&page);
        search.connect_search_changed(move |s| {
            let Some(p) = w.upgrade() else { return };
            let f = s.text().trim().to_string();
            *p.filter.borrow_mut() = f.clone();
            let hit = p
                .set
                .objects
                .iter()
                .find(|o| o.get("symbol").map(|x| x.eq_ignore_ascii_case(&f)).unwrap_or(false) || o.get("number") == Some(f.as_str()))
                .or_else(|| p.set.objects.iter().find(|o| f.chars().count() > 1 && o.get("name").map(|n| n.to_lowercase().starts_with(&f.to_lowercase())).unwrap_or(false)))
                .cloned();
            if let Some(h) = hit {
                p.show_element(h);
            }
            p.area.queue_draw();
        });
        let w = Rc::downgrade(&page);
        page.formula.connect_changed(move |_| {
            if let Some(p) = w.upgrade() {
                p.update_molar();
            }
        });
        let w = Rc::downgrade(&page);
        page.props.connect_row_activated(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.insert_property(false);
            }
        });
        page.build_legend();
        if let Some(first) = page.set.objects.first().cloned() {
            page.show_element(first);
        }
        page.update_molar();
        page
    }

    fn app(&self) -> Rc<App> {
        self.app.upgrade().expect("app")
    }

    fn geometry(&self, w: f64, h: f64) -> (f64, f64, f64) {
        let (w, h) = (w - 30.0, h - 22.0);
        let cell = (w / 18.0).min(h / 9.6).max(16.0);
        let ox = 26.0 + (w - cell * 18.0) / 2.0;
        let oy = 20.0 + ((h - cell * 9.45) / 2.0).max(0.0);
        (cell, ox, oy)
    }

    fn cell_rect(&self, x: i32, y: i32) -> (f64, f64, f64) {
        let (cell, ox, oy) = self.geometry(self.area.width() as f64, self.area.height() as f64);
        let gap = 3.0;
        let yy = if y <= 7 { (y - 1) as f64 * cell } else { 7.0 * cell + cell * 0.45 + (y - 9) as f64 * cell };
        (ox + (x - 1) as f64 * cell + gap / 2.0, oy + yy + gap / 2.0, cell - gap)
    }

    fn hit_test(&self, px: f64, py: f64) -> Option<DataObject> {
        self.set
            .objects
            .iter()
            .find(|o| {
                let (x, y) = pos(o);
                let (rx, ry, s) = self.cell_rect(x, y);
                px >= rx && px <= rx + s && py >= ry && py <= ry + s
            })
            .cloned()
    }

    fn on_key(&self, k: gdk::Key) -> glib::Propagation {
        let cur = self.selected.borrow().clone();
        let Some(cur) = cur else {
            if let Some(f) = self.set.objects.first().cloned() {
                self.show_element(f);
            }
            return glib::Propagation::Stop;
        };
        let z: i32 = cur.get("number").and_then(|v| v.parse().ok()).unwrap_or(1);
        let (x, y) = pos(&cur);
        let find_num = |n: i32| self.set.objects.iter().find(|o| o.get("number") == Some(n.to_string().as_str())).cloned();
        let next = match k {
            gdk::Key::Right => find_num(z + 1),
            gdk::Key::Left => find_num(z - 1),
            gdk::Key::Up | gdk::Key::Down => {
                let dy = if k == gdk::Key::Down { 1 } else { -1 };
                let mut yy = y + dy;
                let mut found = None;
                while (1..=10).contains(&yy) {
                    found = self.set.objects.iter().find(|o| pos(o) == (x, yy)).cloned();
                    if found.is_some() {
                        break;
                    }
                    yy += dy;
                }
                found
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                self.insert_property(false);
                return glib::Propagation::Stop;
            }
            _ => return glib::Propagation::Proceed,
        };
        if let Some(n) = next {
            self.show_element(n);
        }
        glib::Propagation::Stop
    }

    fn range(&self, prop: &str, log: bool) -> Option<(f64, f64)> {
        let vals: Vec<f64> = self.set.objects.iter().filter_map(|o| num(o, prop)).filter(|v| !log || *v > 0.0).map(|v| if log { v.log10() } else { v }).collect();
        if vals.is_empty() {
            return None;
        }
        Some((vals.iter().cloned().fold(f64::MAX, f64::min), vals.iter().cloned().fold(f64::MIN, f64::max)))
    }

    fn color_for(&self, o: &DataObject) -> Rgb {
        let gray = rgb(CLASS_COLORS[0]);
        match self.coloring.get() {
            0 => rgb(CLASS_COLORS[o.get("class_id").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0).min(11)]),
            1 => match o.get("phase") {
                Some("Solid") => rgb((92, 100, 112)),
                Some("Liquid") => rgb((62, 112, 160)),
                Some("Gas") => rgb((150, 110, 62)),
                _ => gray,
            },
            7 => match o.get("discovered") {
                Some("Ancient") => scale(0.0),
                Some(d) => d.parse::<f64>().map(|y| scale((y - 1600.0) / 420.0)).unwrap_or(gray),
                None => gray,
            },
            c => {
                let prop = ["", "", "electronegativity", "density", "melting", "boiling", "weight"][c];
                let log = c == 3;
                match (num(o, prop), self.range(prop, log)) {
                    (Some(v), Some((lo, hi))) => {
                        let val = if log { v.max(1e-12).log10() } else { v };
                        scale((val - lo) / (hi - lo).max(1e-12))
                    }
                    _ => gray,
                }
            }
        }
    }

    fn matches(&self, o: &DataObject) -> bool {
        let f = self.filter.borrow().to_lowercase();
        if f.is_empty() {
            return true;
        }
        o.get("symbol").map(|s| s.to_lowercase() == f).unwrap_or(false) || o.get("name").map(|n| n.to_lowercase().starts_with(&f)).unwrap_or(false) || o.get("number") == Some(f.as_str())
    }

    fn draw(&self, cr: &cairo::Context, w: f64, h: f64) {
        let (cell, ox, oy) = self.geometry(w, h);
        let fg = self.area.color();
        cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.5);
        cr.set_font_size((cell * 0.2).max(7.0));
        for g in 1..=18 {
            let t = g.to_string();
            let ext = cr.text_extents(&t).ok();
            let tw = ext.map(|e| e.width()).unwrap_or(0.0);
            cr.move_to(ox + (g - 1) as f64 * cell + (cell - tw) / 2.0, oy - 6.0);
            let _ = cr.show_text(&t);
        }
        for p in 1..=7 {
            cr.move_to(ox - 16.0, oy + (p - 1) as f64 * cell + cell / 2.0 + 4.0);
            let _ = cr.show_text(&p.to_string());
        }
        let sel = self.selected.borrow().as_ref().and_then(|o| o.get("number").map(|s| s.to_string()));
        let hov = self.hover.borrow().as_ref().and_then(|o| o.get("number").map(|s| s.to_string()));
        let hl = self.highlight.get();
        for o in &self.set.objects {
            let (x, y) = pos(o);
            let (rx, ry, s) = self.cell_rect(x, y);
            let mut c = self.color_for(o);
            let cls: i32 = o.get("class_id").and_then(|v| v.parse().ok()).unwrap_or(0);
            let dimmed = (hl >= 0 && cls != hl) || !self.matches(o);
            if dimmed {
                c = lerp(c, (0.12, 0.13, 0.15), 0.7);
            }
            let num_s = o.get("number").unwrap_or("").to_string();
            if hov.as_deref() == Some(num_s.as_str()) {
                c = lerp(c, (1.0, 1.0, 1.0), 0.15);
            }
            cr.set_source_rgb(c.0, c.1, c.2);
            cr.rectangle(rx, ry, s, s);
            let _ = cr.fill();
            if sel.as_deref() == Some(num_s.as_str()) {
                let ac = crate::ui::theme::accent_rgb();
                cr.set_source_rgb(ac.0 as f64 / 255.0, ac.1 as f64 / 255.0, ac.2 as f64 / 255.0);
                cr.set_line_width(2.5);
                cr.rectangle(rx + 1.0, ry + 1.0, s - 2.0, s - 2.0);
                let _ = cr.stroke();
            }
            let alpha = if dimmed { 0.45 } else { 1.0 };
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.8 * alpha);
            cr.set_font_size((cell * 0.17).max(6.0));
            cr.move_to(rx + 3.0, ry + cell * 0.2);
            let _ = cr.show_text(&num_s);
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            cr.set_font_size((cell * 0.36).max(8.0));
            let sym = o.get("symbol").unwrap_or("");
            let tw = cr.text_extents(sym).map(|e| e.width()).unwrap_or(0.0);
            cr.set_source_rgba(1.0, 1.0, 1.0, alpha);
            cr.move_to(rx + (s - tw) / 2.0, ry + s * 0.68);
            let _ = cr.show_text(sym);
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        }
        cr.set_source_rgba(fg.red() as f64, fg.green() as f64, fg.blue() as f64, 0.5);
        cr.set_font_size((cell * 0.2).max(7.0));
        let (lx, ly, ls) = self.cell_rect(3, 6);
        cr.move_to(lx + 2.0, ly + ls * 0.7);
        let _ = cr.show_text("57–71");
        let (lx, ly, ls) = self.cell_rect(3, 7);
        cr.move_to(lx + 2.0, ly + ls * 0.7);
        let _ = cr.show_text("89–103");
    }

    fn build_legend(self: &Rc<Self>) {
        while let Some(c) = self.legend.first_child() {
            self.legend.remove(&c);
        }
        let swatch = |c: Rgb| {
            let a = gtk::DrawingArea::new();
            a.set_content_width(14);
            a.set_content_height(14);
            a.set_valign(gtk::Align::Center);
            a.set_draw_func(move |_, cr, w, h| {
                cr.set_source_rgb(c.0, c.1, c.2);
                cr.rectangle(0.0, 0.0, w as f64, h as f64);
                let _ = cr.fill();
            });
            a
        };
        match self.coloring.get() {
            0 => {
                let flow = gtk::FlowBox::new();
                flow.set_selection_mode(gtk::SelectionMode::None);
                flow.set_max_children_per_line(6);
                flow.set_hexpand(true);
                for (i, name) in ELEMENT_CLASSES.iter().enumerate().skip(1) {
                    let b = gtk::Button::new();
                    b.add_css_class("flat");
                    let bx = hbox(6);
                    bx.append(&swatch(rgb(CLASS_COLORS[i])));
                    bx.append(&gtk::Label::new(Some(name)));
                    b.set_child(Some(&bx));
                    let w = Rc::downgrade(self);
                    b.connect_clicked(move |_| {
                        if let Some(p) = w.upgrade() {
                            p.highlight.set(if p.highlight.get() == i as i32 { -1 } else { i as i32 });
                            p.area.queue_draw();
                        }
                    });
                    flow.insert(&b, -1);
                }
                self.legend.append(&flow);
            }
            1 => {
                for (n, c) in [("Solid", (92, 100, 112)), ("Liquid", (62, 112, 160)), ("Gas", (150, 110, 62)), ("Unknown", CLASS_COLORS[0])] {
                    self.legend.append(&swatch(rgb(c)));
                    let l = dim(n);
                    l.set_margin_end(12);
                    self.legend.append(&l);
                }
            }
            c => {
                self.legend.append(&dim("low"));
                for i in 0..STEPS {
                    let s = swatch(scale((i as f64 + 0.5) / STEPS as f64));
                    s.set_content_width(30);
                    self.legend.append(&s);
                }
                self.legend.append(&dim(if c == 7 { "recent" } else { "high" }));
                self.legend.append(&dim(if c == 3 { "   (logarithmic scale, gray = no data)" } else { "   (gray = no data)" }));
            }
        }
    }

    fn show_element(&self, o: DataObject) {
        self.big_number.set_text(o.get("number").unwrap_or(""));
        self.big_symbol.set_text(o.get("symbol").unwrap_or(""));
        self.big_name.set_text(o.get("name").unwrap_or(""));
        let group = o.get("group").map(|g| format!("Group {}, ", g)).unwrap_or_default();
        self.big_class.set_text(&format!("{}\n{}Period {}", o.get("class").unwrap_or(""), group, o.get("period").unwrap_or("")));
        while let Some(r) = self.props.row_at_index(0) {
            self.props.remove(&r);
        }
        let mut names = vec![];
        for p in self.set.properties.iter().filter(|x| !x.hidden) {
            let Some(v) = o.get(&p.name) else { continue };
            let mut shown = v.to_string();
            if !p.is_text && !["number", "group", "period"].contains(&p.name.as_str()) {
                if !p.unit.is_empty() {
                    shown += &format!(" {}", p.unit.replace("^3", "³"));
                }
                if p.name == "melting" || p.name == "boiling" {
                    if let Ok(k) = v.parse::<f64>() {
                        shown += &format!("  ({} °C)", fmt2(k - 273.15));
                    }
                }
                let z: i32 = o.get("number").and_then(|x| x.parse().ok()).unwrap_or(0);
                if p.name == "weight" && (o.get("class_id") == Some("11") || (z > 83 && !v.contains('.'))) {
                    shown = format!("[{}]", v);
                }
                if p.name == "density" && o.get("phase") == Some("Gas") {
                    if let Ok(d) = v.parse::<f64>() {
                        shown += &format!("  ({} g/L)", fmt4(d * 1000.0));
                    }
                }
            }
            let row = hbox(10);
            row.set_margin_top(3);
            row.set_margin_bottom(3);
            let n = dim(&p.title);
            n.set_width_chars(16);
            n.set_wrap(true);
            row.append(&n);
            let val = label(&shown);
            val.set_hexpand(true);
            val.set_wrap(true);
            val.set_selectable(true);
            row.append(&val);
            self.props.append(&row);
            names.push(p.name.clone());
        }
        *self.prop_names.borrow_mut() = names;
        *self.selected.borrow_mut() = Some(o);
        self.area.queue_draw();
    }

    fn insert_property(&self, lookup: bool) {
        let Some(o) = self.selected.borrow().clone() else { return };
        let Some(pn) = self.props.selected_row().and_then(|r| self.prop_names.borrow().get(r.index() as usize).cloned()) else { return };
        let Some(p) = self.set.prop(&pn).cloned() else { return };
        let sym = o.get("symbol").unwrap_or("").to_string();
        let app = self.app();
        if lookup || p.is_text {
            app.insert_into_expression(&format!("atom({}, {})", sym, p.name), false);
            return;
        }
        app.insert_into_expression(&format!("{}{}", o.get(&p.name).unwrap_or(""), if p.unit.is_empty() { String::new() } else { format!(" {}", p.unit) }), false);
    }

    fn update_molar(&self) {
        match self.molar_mass(&self.formula.text()) {
            Ok(v) => {
                self.molar.set_text(&format!("= {} g/mol", fmt5(v)));
                self.molar.remove_css_class("dim-label");
            }
            Err(e) => {
                self.molar.set_text(&e);
                self.molar.add_css_class("dim-label");
            }
        }
    }

    // parses formulas like H2O, Ca(OH)2, CuSO4·5H2O, [Fe(CN)6]3
    fn molar_mass(&self, f: &str) -> Result<f64, String> {
        let f = f.replace(' ', "");
        if f.is_empty() {
            return Err("enter a formula".into());
        }
        let mut total = 0.0;
        for part in f.split(['·', '*', '.']) {
            if part.is_empty() {
                continue;
            }
            let chars: Vec<char> = part.chars().collect();
            let digits = chars.iter().take_while(|c| c.is_ascii_digit()).count();
            let (mult, body) = if digits > 0 && digits < chars.len() { (chars[..digits].iter().collect::<String>().parse::<f64>().unwrap_or(1.0), chars[digits..].to_vec()) } else { (1.0, chars) };
            let mut pos = 0;
            total += mult * self.parse_group(&body, &mut pos)?;
            if pos != body.len() {
                return Err(format!("unexpected '{}'", body[pos]));
            }
        }
        Ok(total)
    }

    fn parse_group(&self, s: &[char], pos: &mut usize) -> Result<f64, String> {
        let mut sum = 0.0;
        while *pos < s.len() {
            let ch = s[*pos];
            if ch == '(' || ch == '[' {
                *pos += 1;
                let inner = self.parse_group(s, pos)?;
                if *pos >= s.len() || (s[*pos] != ')' && s[*pos] != ']') {
                    return Err("missing closing parenthesis".into());
                }
                *pos += 1;
                sum += inner * read_count(s, pos);
            } else if ch == ')' || ch == ']' {
                return Ok(sum);
            } else if ch.is_uppercase() {
                let start = *pos;
                *pos += 1;
                while *pos < s.len() && s[*pos].is_lowercase() {
                    *pos += 1;
                }
                let sym: String = s[start..*pos].iter().collect();
                let el = self.set.objects.iter().find(|o| o.get("symbol") == Some(sym.as_str())).ok_or_else(|| format!("unknown element {}", sym))?;
                let w: f64 = el.get("weight").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                sum += w * read_count(s, pos);
            } else {
                return Err(format!("unexpected '{}'", ch));
            }
        }
        Ok(sum)
    }
}

fn read_count(s: &[char], pos: &mut usize) -> f64 {
    let start = *pos;
    while *pos < s.len() && s[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if *pos > start {
        s[start..*pos].iter().collect::<String>().parse().unwrap_or(1.0)
    } else {
        1.0
    }
}

fn trim_num(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}
fn fmt2(v: f64) -> String {
    trim_num(format!("{:.2}", v))
}
fn fmt4(v: f64) -> String {
    trim_num(format!("{:.4}", v))
}
fn fmt5(v: f64) -> String {
    trim_num(format!("{:.5}", v))
}