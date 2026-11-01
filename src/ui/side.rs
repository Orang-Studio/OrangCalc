// side panel
use super::app::App;
use super::settings::{HistoryEntry, HistoryExprMode};
use super::theme;
use super::widgets::*;
use crate::engine::{BigInt, Expr};
use gtk::prelude::*;
use gtk::{gio, glib};
use num_traits::{One, Signed, Zero};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
pub struct HistoryPanel {
    pub root: gtk::Box,
    pub entries: RefCell<Vec<HistoryEntry>>,
    shown: RefCell<Vec<usize>>,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    date_filter: RefCell<Option<chrono::NaiveDate>>,
    app: std::rc::Weak<App>,
    pub colorize: Cell<bool>,
    pub on_insert_text: RefCell<Option<Box<dyn Fn(&str)>>>,
    pub on_insert_value: RefCell<Option<Box<dyn Fn(&HistoryEntry)>>>,
    pub on_changed: RefCell<Option<Box<dyn Fn()>>>,
}
impl HistoryPanel {
    pub fn new(entries: Vec<HistoryEntry>, app: std::rc::Weak<App>) -> Rc<HistoryPanel> {
        let root = vbox(6);
        margins(&root, 8);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search history"));
        root.append(&search);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Multiple);
        list.add_css_class("rich-list");
        root.append(&scrolled(&list));
        let p = Rc::new(HistoryPanel {
            root,
            entries: RefCell::new(entries),
            shown: RefCell::new(vec![]),
            list,
            search,
            date_filter: RefCell::new(None),
            app,
            colorize: Cell::new(true),
            on_insert_text: RefCell::new(None),
            on_insert_value: RefCell::new(None),
            on_changed: RefCell::new(None),
        });
        let w = Rc::downgrade(&p);
        p.search.connect_search_changed(move |_| {
            if let Some(p) = w.upgrade() {
                p.refill();
            }
        });
        let w = Rc::downgrade(&p);
        p.list.connect_row_activated(move |_, row| {
            if let Some(p) = w.upgrade() {
                if let Some(h) = p.entry_at(row.index()) {
                    if let Some(f) = p.on_insert_value.borrow().as_ref() {
                        f(&h);
                    }
                }
            }
        });
        p.install_menu();
        p.refill();
        p
    }

    fn entry_at(&self, row: i32) -> Option<HistoryEntry> {
        let idx = *self.shown.borrow().get(row as usize)?;
        self.entries.borrow().get(idx).cloned()
    }

    fn selected(&self) -> Vec<usize> {
        let shown = self.shown.borrow();
        self.list.selected_rows().iter().filter_map(|r| shown.get(r.index() as usize).copied()).collect()
    }

    fn changed(&self) {
        if let Some(f) = self.on_changed.borrow().as_ref() {
            f();
        }
    }

    fn install_menu(self: &Rc<Self>) {
        let group = gio::SimpleActionGroup::new();
        let add = |name: &str, f: Box<dyn Fn(&Rc<HistoryPanel>)>| {
            let a = gio::SimpleAction::new(name, None);
            let w = Rc::downgrade(self);
            a.connect_activate(move |_, _| {
                if let Some(p) = w.upgrade() {
                    f(&p);
                }
            });
            group.add_action(&a);
        };
        add(
            "insertvalue",
            Box::new(|p| {
                if let Some(h) = p.selected().first().and_then(|i| p.entries.borrow().get(*i).cloned()) {
                    if let Some(f) = p.on_insert_value.borrow().as_ref() {
                        f(&h);
                    }
                }
            }),
        );
        add(
            "inserttext",
            Box::new(|p| {
                if let Some(h) = p.selected().first().and_then(|i| p.entries.borrow().get(*i).cloned()) {
                    if let Some(f) = p.on_insert_text.borrow().as_ref() {
                        f(&h.expression);
                    }
                }
            }),
        );
        add("copy", Box::new(|p| p.copy_selected(false)));
        add("copyascii", Box::new(|p| p.copy_selected(true)));
        add(
            "copyexpr",
            Box::new(|p| {
                if let Some(h) = p.selected().first().and_then(|i| p.entries.borrow().get(*i).cloned()) {
                    copy_text(&h.expression);
                }
            }),
        );
        add("comment", Box::new(|p| p.edit_comment()));
        add(
            "protect",
            Box::new(|p| {
                let sel = p.selected();
                {
                    let mut e = p.entries.borrow_mut();
                    let on = sel.iter().any(|i| !e[*i].protected);
                    for i in &sel {
                        e[*i].protected = on;
                    }
                }
                p.refill();
                p.changed();
            }),
        );
        add(
            "movetop",
            Box::new(|p| {
                let sel = p.selected();
                {
                    let mut e = p.entries.borrow_mut();
                    let moved: Vec<HistoryEntry> = sel.iter().map(|i| e[*i].clone()).collect();
                    let mut keep: Vec<HistoryEntry> = e.iter().enumerate().filter(|(i, _)| !sel.contains(i)).map(|(_, h)| h.clone()).collect();
                    keep.extend(moved);
                    *e = keep;
                }
                p.refill();
                p.changed();
            }),
        );
        add(
            "remove",
            Box::new(|p| {
                let sel = p.selected();
                {
                    let mut e = p.entries.borrow_mut();
                    let mut i = 0;
                    e.retain(|_| {
                        let keep = !sel.contains(&i);
                        i += 1;
                        keep
                    });
                }
                p.refill();
                p.changed();
            }),
        );
        add("clear", Box::new(|p| p.clear_unprotected()));
        add("search", Box::new(|p| {
            p.search.grab_focus();
        }));
        add("searchdate", Box::new(|p| p.search_by_date()));
        add(
            "alldates",
            Box::new(|p| {
                *p.date_filter.borrow_mut() = None;
                p.refill();
            }),
        );
        add("selectall", Box::new(|p| p.list.select_all()));
        self.list.insert_action_group("hist", Some(&group));
        let menu = gio::Menu::new();
        let s1 = gio::Menu::new();
        s1.append(Some("Insert Value"), Some("hist.insertvalue"));
        s1.append(Some("Insert Text"), Some("hist.inserttext"));
        menu.append_section(None, &s1);
        let s2 = gio::Menu::new();
        s2.append(Some("Copy"), Some("hist.copy"));
        s2.append(Some("Copy Unformatted ASCII"), Some("hist.copyascii"));
        s2.append(Some("Copy Expression"), Some("hist.copyexpr"));
        menu.append_section(None, &s2);
        let s3 = gio::Menu::new();
        s3.append(Some("Add/Edit Comment…"), Some("hist.comment"));
        s3.append(Some("Protect"), Some("hist.protect"));
        s3.append(Some("Move to Top"), Some("hist.movetop"));
        s3.append(Some("Remove"), Some("hist.remove"));
        s3.append(Some("Clear"), Some("hist.clear"));
        menu.append_section(None, &s3);
        let s4 = gio::Menu::new();
        s4.append(Some("Search…"), Some("hist.search"));
        s4.append(Some("Search by Date…"), Some("hist.searchdate"));
        s4.append(Some("Show All Dates"), Some("hist.alldates"));
        s4.append(Some("Select All"), Some("hist.selectall"));
        menu.append_section(None, &s4);
        let pop = gtk::PopoverMenu::from_model(Some(&menu));
        pop.set_parent(&self.list);
        pop.set_has_arrow(false);
        let g = gtk::GestureClick::new();
        g.set_button(3);
        let w = Rc::downgrade(self);
        g.connect_pressed(move |_, _, x, y| {
            let Some(p) = w.upgrade() else { return };
            if let Some(row) = p.list.row_at_y(y as i32) {
                if !row.is_selected() {
                    p.list.unselect_all();
                    p.list.select_row(Some(&row));
                }
            }
            pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            pop.popup();
        });
        self.list.add_controller(g);
    }

    fn copy_selected(&self, ascii: bool) {
        let e = self.entries.borrow();
        let parts: Vec<String> = self
            .selected()
            .iter()
            .map(|i| {
                let h = &e[*i];
                if h.is_error || h.is_message {
                    format!("{}  {}", h.expression, h.result)
                } else {
                    format!("{} = {}", h.expression, if ascii { &h.result_ascii } else { &h.result })
                }
            })
            .collect();
        copy_text(&parts.join("\n"));
    }

    fn edit_comment(self: &Rc<Self>) {
        let Some(idx) = self.selected().first().copied() else { return };
        let cur = self.entries.borrow()[idx].comment.clone();
        let w = Rc::downgrade(self);
        ask(root_window(&self.root).as_ref(), "Comment", "Comment:", &cur, move |t| {
            if let Some(p) = w.upgrade() {
                if let Some(h) = p.entries.borrow_mut().get_mut(idx) {
                    h.comment = t.trim().to_string();
                }
                p.refill();
                p.changed();
            }
        });
    }

    fn search_by_date(self: &Rc<Self>) {
        let w = Rc::downgrade(self);
        super::dialogs::date_dialog(root_window(&self.root).as_ref(), "Search by Date", chrono::Local::now().naive_local(), false, move |d| {
            if let Some(p) = w.upgrade() {
                *p.date_filter.borrow_mut() = Some(d.date());
                p.refill();
            }
        });
    }

    pub fn clear_unprotected(&self) {
        self.entries.borrow_mut().retain(|h| h.protected);
        self.refill();
        self.changed();
    }

    pub fn add(&self, h: HistoryEntry) {
        let max = self.app.upgrade().map(|a| a.settings.borrow().max_history).unwrap_or(300).max(10) as usize;
        {
            let mut e = self.entries.borrow_mut();
            e.push(h);
            while e.len() > max {
                match e.iter().position(|x| !x.protected) {
                    Some(i) => {
                        e.remove(i);
                    }
                    None => break,
                }
            }
        }
        self.refill();
    }

    fn expr_text(&self, h: &HistoryEntry) -> String {
        match self.app.upgrade().map(|a| a.settings.borrow().history_expression).unwrap_or(HistoryExprMode::Entered) {
            HistoryExprMode::Parsed if !h.parsed.is_empty() => h.parsed.clone(),
            HistoryExprMode::EnteredAndParsed if !h.parsed.is_empty() && h.parsed != h.expression => format!("{}  ⇒  {}", h.expression, h.parsed),
            _ => h.expression.clone(),
        }
    }

    fn result_markup(&self, h: &HistoryEntry) -> String {
        let esc = |s: &str| glib::markup_escape_text(s).to_string();
        if h.is_error {
            return format!("<span foreground=\"{}\">{}</span>", theme::token_color(crate::engine::formatter::TokType::Error, theme::is_dark()), esc(&h.result));
        }
        if h.is_message {
            return esc(&h.result);
        }
        if !h.exact_text.is_empty() {
            return format!("= {}  ≈ {}", esc(&h.exact_text), esc(&h.result));
        }
        let prefix = if h.approx { "≈ " } else { "= " };
        if self.colorize.get() && !h.tokens.is_empty() {
            return format!("<span alpha=\"60%\">{}</span>{}", prefix, theme::markup_tokens(&h.tokens, true));
        }
        format!("{}{}", prefix, esc(&h.result))
    }

    pub fn refill(&self) {
        let q = self.search.text().trim().to_lowercase();
        let df = *self.date_filter.borrow();
        let entries = self.entries.borrow();
        let shown: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, h)| (q.is_empty() || h.expression.to_lowercase().contains(&q) || h.result.to_lowercase().contains(&q) || h.comment.to_lowercase().contains(&q)) && df.map(|d| h.time.date() == d).unwrap_or(true))
            .map(|(i, _)| i)
            .collect();
        while let Some(r) = self.list.row_at_index(0) {
            self.list.remove(&r);
        }
        for &i in &shown {
            let h = &entries[i];
            let b = vbox(2);
            b.set_margin_top(4);
            b.set_margin_bottom(4);
            b.set_margin_start(6);
            b.set_margin_end(6);
            if h.protected {
                b.add_css_class("oc-protected");
            }
            let e = label(&self.expr_text(h));
            e.add_css_class("oc-history-expr");
            e.set_wrap(true);
            e.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            b.append(&e);
            let r = gtk::Label::new(None);
            r.set_markup(&self.result_markup(h));
            r.set_xalign(1.0);
            r.set_wrap(true);
            r.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            r.add_css_class("oc-history-result");
            b.append(&r);
            if !h.comment.is_empty() {
                let c = label(&h.comment);
                c.add_css_class("oc-history-comment");
                c.set_wrap(true);
                b.append(&c);
            }
            self.list.append(&b);
        }
        *self.shown.borrow_mut() = shown;
        let list = self.list.clone();
        glib::idle_add_local_once(move || {
            if let Some(adj) = list.parent().and_then(|v| v.parent()).and_then(|s| s.downcast::<gtk::ScrolledWindow>().ok()).map(|s| s.vadjustment()) {
                adj.set_value(adj.upper());
            }
        });
    }
}

pub struct RpnPanel {
    pub root: gtk::Box,
    list: gtk::ListBox,
    pub on_op: RefCell<Option<Box<dyn Fn(&str)>>>,
    pub on_edit: RefCell<Option<Box<dyn Fn(usize, String)>>>,
    pub texts: RefCell<Vec<String>>,
}

impl RpnPanel {
    pub fn new() -> Rc<RpnPanel> {
        let root = vbox(6);
        margins(&root, 8);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        root.append(&scrolled(&list));
        let bar = gtk::Grid::new();
        bar.set_row_spacing(4);
        bar.set_column_spacing(4);
        bar.set_column_homogeneous(true);
        let p = Rc::new(RpnPanel { root, list, on_op: RefCell::new(None), on_edit: RefCell::new(None), texts: RefCell::new(vec![]) });
        let defs = [
            ("↑", "up", "Rotate the stack or move the selected register up"),
            ("↓", "down", "Rotate the stack or move the selected register down"),
            ("⇅", "swap", "Swap the top two values or move the selected value to the top of the stack"),
            ("copy", "copy", "Copy the selected or top value to the top of the stack"),
            ("lastx", "lastx", "Enter the top value from before the last numeric operation"),
            ("drop", "delete", "Delete the top or selected value"),
            ("±", "negate", "Negate the top value"),
            ("clear", "clear", "Clear the RPN stack"),
        ];
        for (i, (l, op, tip)) in defs.iter().enumerate() {
            let w = Rc::downgrade(&p);
            let op = op.to_string();
            let b = button(l, move || {
                if let Some(p) = w.upgrade() {
                    if let Some(f) = p.on_op.borrow().as_ref() {
                        f(&op);
                    }
                }
            });
            b.set_tooltip_text(Some(tip));
            bar.attach(&b, (i % 4) as i32, (i / 4) as i32, 1, 1);
        }
        p.root.append(&bar);
        let w = Rc::downgrade(&p);
        p.list.connect_row_activated(move |_, row| {
            let Some(p) = w.upgrade() else { return };
            let n = p.texts.borrow().len();
            let idx = n - 1 - row.index() as usize;
            let cur = p.texts.borrow()[row.index() as usize].clone();
            let w2 = Rc::downgrade(&p);
            ask(root_window(&p.root).as_ref(), "Edit Register", &format!("Value of register {}:", idx + 1), &cur, move |t| {
                if let Some(p) = w2.upgrade() {
                    if let Some(f) = p.on_edit.borrow().as_ref() {
                        f(idx, t);
                    }
                }
            });
        });
        p
    }

    // -1
    pub fn selected_register(&self) -> i32 {
        match self.list.selected_row() {
            Some(r) => self.texts.borrow().len() as i32 - 1 - r.index(),
            None => -1,
        }
    }

    pub fn set_texts(&self, texts: Vec<String>) {
        while let Some(r) = self.list.row_at_index(0) {
            self.list.remove(&r);
        }
        let n = texts.len();
        for (i, t) in texts.iter().enumerate() {
            let row = hbox(8);
            row.set_margin_top(4);
            row.set_margin_bottom(4);
            row.set_margin_start(6);
            row.set_margin_end(6);
            let reg = gtk::Label::new(Some(&(n - i).to_string()));
            reg.add_css_class("dim-label");
            row.append(&reg);
            let v = gtk::Label::new(Some(t));
            v.set_hexpand(true);
            v.set_xalign(1.0);
            v.set_ellipsize(gtk::pango::EllipsizeMode::End);
            v.add_css_class("oc-history-result");
            row.append(&v);
            self.list.append(&row);
        }
        *self.texts.borrow_mut() = texts;
    }
}

pub struct BasesPanel {
    pub root: gtk::Box,
    bin: gtk::Label,
    oct: gtk::Label,
    dec: gtk::Label,
    hex: gtk::Label,
    width: gtk::DropDown,
    signed: gtk::CheckButton,
    bits: gtk::Grid,
    bit_buttons: RefCell<Vec<gtk::ToggleButton>>,
    value: RefCell<Option<BigInt>>,
    me: RefCell<std::rc::Weak<BasesPanel>>,
    rendering: Cell<bool>,
    pub on_edit: RefCell<Option<Box<dyn Fn(BigInt)>>>,
}

const WIDTHS: [usize; 4] = [8, 16, 32, 64];

fn to_base(v: &BigInt, b: u32) -> String {
    v.to_str_radix(b).to_uppercase()
}

impl BasesPanel {
    pub fn new() -> Rc<BasesPanel> {
        let root = vbox(8);
        margins(&root, 12);
        let grid = gtk::Grid::new();
        grid.set_row_spacing(8);
        grid.set_column_spacing(10);
        let mk = |name: &str, r: i32| {
            let n = dim(name);
            grid.attach(&n, 0, r, 1, 1);
            let v = label("-");
            v.add_css_class("oc-mono");
            v.set_wrap(true);
            v.set_wrap_mode(gtk::pango::WrapMode::Char);
            v.set_hexpand(true);
            v.set_selectable(true);
            grid.attach(&v, 1, r, 1, 1);
            v
        };
        let bin = mk("Binary:", 0);
        let oct = mk("Octal:", 1);
        let dec = mk("Decimal:", 2);
        let hex = mk("Hexadecimal:", 3);
        root.append(&grid);
        let opts = hbox(10);
        let width = combo(&["8 bits", "16 bits", "32 bits", "64 bits"], 3);
        let signed = gtk::CheckButton::with_label("Signed");
        signed.set_active(true);
        opts.append(&width);
        opts.append(&signed);
        root.append(&opts);
        let bits = gtk::Grid::new();
        bits.set_row_spacing(2);
        bits.set_column_spacing(2);
        root.append(&bits);
        let note = dim("Click a bit to toggle it. Click a value to select and copy it.");
        root.append(&note);
        let p = Rc::new(BasesPanel {
            root,
            bin,
            oct,
            dec,
            hex,
            width,
            signed,
            bits,
            bit_buttons: RefCell::new(vec![]),
            value: RefCell::new(None),
            me: RefCell::new(std::rc::Weak::new()),
            rendering: Cell::new(false),
            on_edit: RefCell::new(None),
        });
        *p.me.borrow_mut() = Rc::downgrade(&p);
        let w = Rc::downgrade(&p);
        p.width.connect_selected_notify(move |_| {
            if let Some(p) = w.upgrade() {
                if !p.rendering.get() {
                    p.render();
                }
            }
        });
        let w = Rc::downgrade(&p);
        p.signed.connect_toggled(move |_| {
            if let Some(p) = w.upgrade() {
                p.render();
            }
        });
        p.render();
        p
    }
    fn bit_width(&self) -> usize {
        WIDTHS[(self.width.selected() as usize).min(3)]
    }
    pub fn set_value(&self, e: Option<&Expr>) {
        let v = e.and_then(|e| e.as_real()).filter(|r| r.is_int() && r.n.bits() <= 64).map(|r| r.n.clone());
        *self.value.borrow_mut() = v;
        self.render();
    }
    fn toggle_bit(&self, i: usize) {
        let w = self.bit_width();
        let value = self.value.borrow().clone().unwrap_or_default();
        let mask = (BigInt::one() << w) - 1;
        let mut u = value & &mask;
        u ^= BigInt::one() << i;
        let nv = if self.signed.is_active() && ((&u >> (w - 1)) & BigInt::one()).is_one() { u - (BigInt::one() << w) } else { u };
        if let Some(f) = self.on_edit.borrow().as_ref() {
            f(nv);
        }
    }
    fn render(&self) {
        self.rendering.set(true);
        let value = self.value.borrow().clone();
        let mut w = self.bit_width();
        let u = match &value {
            None => {
                for l in [&self.bin, &self.oct, &self.dec, &self.hex] {
                    l.set_text("-");
                }
                BigInt::zero()
            }
            Some(v) => {
                let signed = self.signed.is_active();
                while w < 64 && (*v >= (BigInt::one() << (if signed { w - 1 } else { w })) || *v < -(BigInt::one() << (w - 1))) {
                    w *= 2;
                }
                if w != self.bit_width() {
                    self.width.set_selected(WIDTHS.iter().position(|x| *x == w).unwrap_or(3) as u32);
                }
                let mask = (BigInt::one() << w) - 1;
                let u = v & &mask;
                let b = format!("{:0>w$}", to_base(&u, 2), w = w);
                let grouped: Vec<String> = (0..w / 4).map(|i| b[i * 4..i * 4 + 4].to_string()).collect();
                self.bin.set_text(&grouped.join(" "));
                self.oct.set_text(&format!("{}{}", if v.is_negative() { "−" } else { "" }, to_base(&v.abs(), 8)));
                self.dec.set_text(&v.to_string());
                self.hex.set_text(&if v.is_negative() { format!("−{}  ({})", to_base(&v.abs(), 16), to_base(&u, 16)) } else { to_base(v, 16) });
                u
            }
        };
        // rebuild bit buttons
        if self.bit_buttons.borrow().len() != w {
            while let Some(c) = self.bits.first_child() {
                self.bits.remove(&c);
            }
            let mut v = vec![];
            for bit in 0..w {
                let t = gtk::ToggleButton::with_label("0");
                t.add_css_class("oc-bit");
                t.add_css_class("flat");
                t.set_tooltip_text(Some(&format!("Bit {}", bit)));
                let index = w - 1 - bit;
                let col = (index % 16) as i32;
                let row = (index / 16) as i32;
                t.set_margin_start(if col % 4 == 0 && col > 0 { 4 } else { 0 });
                self.bits.attach(&t, col, row, 1, 1);
                let me = self.me.borrow().clone();
                t.connect_clicked(move |_| {
                    if let Some(p) = me.upgrade() {
                        if !p.rendering.get() {
                            p.toggle_bit(bit);
                        }
                    }
                });
                v.push(t);
            }
            *self.bit_buttons.borrow_mut() = v;
        }
        for (bit, t) in self.bit_buttons.borrow().iter().enumerate() {
            let on = ((&u >> bit) & BigInt::one()).is_one();
            t.set_active(on);
            t.set_label(if on { "1" } else { "0" });
        }
        self.rendering.set(false);
}   }