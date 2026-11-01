// expression editor with completion, history navigation and RPN operator keys
use super::settings::CompletionMode;
use super::svc::Snapshot;
use crate::engine::parser::is_ident_part;
use crate::engine::PREFIXES;
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
#[derive(Clone)]
struct Completion {
    name: String,
    title: String,
    kind: &'static str,
    is_fn: bool,
    takes_args: bool,
    score: i32,
}
pub struct ExpressionBox {
    pub view: gtk::TextView,
    pub buffer: gtk::TextBuffer,
    pub frame: gtk::Frame,
    popover: gtk::Popover,
    list: gtk::ListBox,
    items: RefCell<Vec<Completion>>,
    completion_start: Cell<i32>,
    history_index: Cell<i32>,
    history_draft: RefCell<String>,
    suppress: Cell<bool>,
    pub on_calculate: RefCell<Option<Box<dyn Fn()>>>,
    pub on_rpn_operator: RefCell<Option<Box<dyn Fn(char)>>>,
    pub rpn_active: RefCell<Option<Box<dyn Fn() -> bool>>>,
    pub history: RefCell<Option<Box<dyn Fn() -> Vec<String>>>>,
    pub completion_mode: Cell<CompletionMode>,
    pub completion_min: Cell<i32>,
    pub rpn_keys: Cell<bool>,
    pub snap: RefCell<Option<Rc<RefCell<Snapshot>>>>,
}
impl ExpressionBox {
    pub fn new() -> Rc<ExpressionBox> {
        let buffer = gtk::TextBuffer::new(None);
        buffer.set_enable_undo(true);
        let view = gtk::TextView::with_buffer(&buffer);
        view.set_wrap_mode(gtk::WrapMode::WordChar);
        view.add_css_class("oc-expr");
        view.set_accepts_tab(false);
        view.set_left_margin(4);
        view.set_right_margin(4);
        let sw = gtk::ScrolledWindow::new();
        sw.set_child(Some(&view));
        sw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        sw.set_min_content_height(78);
        sw.set_max_content_height(78);
        let frame = gtk::Frame::new(None);
        frame.set_child(Some(&sw));
        frame.set_margin_start(12);
        frame.set_margin_end(12);
        frame.set_margin_top(12);

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Single);
        let lsw = gtk::ScrolledWindow::new();
        lsw.set_child(Some(&list));
        lsw.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        lsw.set_min_content_width(440);
        lsw.set_max_content_height(260);
        lsw.set_propagate_natural_height(true);
        let popover = gtk::Popover::new();
        popover.set_child(Some(&lsw));
        popover.set_autohide(false);
        popover.set_has_arrow(false);
        popover.set_can_focus(false);
        popover.set_position(gtk::PositionType::Bottom);
        popover.set_parent(&view);

        let e = Rc::new(ExpressionBox {
            view,
            buffer,
            frame,
            popover,
            list,
            items: RefCell::new(vec![]),
            completion_start: Cell::new(-1),
            history_index: Cell::new(-1),
            history_draft: RefCell::new(String::new()),
            suppress: Cell::new(false),
            on_calculate: RefCell::new(None),
            on_rpn_operator: RefCell::new(None),
            rpn_active: RefCell::new(None),
            history: RefCell::new(None),
            completion_mode: Cell::new(CompletionMode::Full),
            completion_min: Cell::new(2),
            rpn_keys: Cell::new(true),
            snap: RefCell::new(None),
        });

        let key = gtk::EventControllerKey::new();
        key.set_propagation_phase(gtk::PropagationPhase::Capture);
        let w = Rc::downgrade(&e);
        key.connect_key_pressed(move |_, k, _, m| match w.upgrade() {
            Some(e) => e.on_key(k, m),
            None => glib::Propagation::Proceed,
        });
        e.view.add_controller(key);
        let w = Rc::downgrade(&e);
        e.buffer.connect_changed(move |_| {
            if let Some(e) = w.upgrade() {
                if !e.suppress.get() {
                    e.update_completion(false);
                }
            }
        });
        let w = Rc::downgrade(&e);
        e.list.connect_row_activated(move |_, _| {
            if let Some(e) = w.upgrade() {
                e.accept_completion();
                e.view.grab_focus();
            }
        });
        let focus = gtk::EventControllerFocus::new();
        let w = Rc::downgrade(&e);
        focus.connect_leave(move |_| {
            if let Some(e) = w.upgrade() {
                e.hide_popup();
            }
        });
        e.view.add_controller(focus);
        e
    }

    pub fn text(&self) -> String {
        let (a, b) = self.buffer.bounds();
        self.buffer.text(&a, &b, false).to_string()
    }

    pub fn len(&self) -> i32 {
        self.buffer.char_count()
    }

    pub fn set_text(&self, t: &str, select_all: bool) {
        self.suppress.set(true);
        self.buffer.set_text(t);
        self.suppress.set(false);
        if select_all {
            self.select_all();
        } else {
            let end = self.buffer.end_iter();
            self.buffer.place_cursor(&end);
        }
    }

    pub fn select_all(&self) {
        let (a, b) = self.buffer.bounds();
        self.buffer.select_range(&a, &b);
    }

    pub fn cursor(&self) -> i32 {
        self.buffer.cursor_position()
    }

    pub fn selection(&self) -> Option<(i32, i32)> {
        self.buffer.selection_bounds().map(|(a, b)| (a.offset(), b.offset()))
    }

    pub fn selected_text(&self) -> String {
        self.buffer.selection_bounds().map(|(a, b)| self.buffer.text(&a, &b, false).to_string()).unwrap_or_default()
    }

    pub fn all_selected(&self) -> bool {
        self.len() > 0 && self.selection().map(|(a, b)| a == 0 && b == self.len()).unwrap_or(false)
    }

    pub fn set_cursor(&self, pos: i32) {
        let it = self.buffer.iter_at_offset(pos.clamp(0, self.len()));
        self.buffer.place_cursor(&it);
    }

    pub fn focus(&self) {
        self.view.grab_focus();
    }

    pub fn undo(&self) {
        if self.buffer.can_undo() {
            self.buffer.undo();
        }
    }

    pub fn redo(&self) {
        if self.buffer.can_redo() {
            self.buffer.redo();
        }
    }

    pub fn insert(&self, text: &str, caret_back: i32) {
        self.buffer.delete_selection(true, true);
        self.buffer.insert_at_cursor(text);
        if caret_back > 0 {
            self.set_cursor(self.cursor() - caret_back);
        }
        self.focus();
    }

    // inserts name( ) around the selection, or around everything if wrap_all
    pub fn insert_function(&self, name: &str, wrap_all: bool, extra: &str) {
        let sel = self.selected_text();
        if !sel.is_empty() {
            self.insert(&format!("{}({}{})", name, sel, extra), 0);
            return;
        }
        if wrap_all && self.len() > 0 {
            let t = self.text();
            self.set_text(&format!("{}({}{})", name, t, extra), false);
            self.focus();
            return;
        }
        self.insert(&format!("{}({})", name, extra), 1 + extra.chars().count() as i32);
    }

    pub fn backspace(&self) {
        if !self.buffer.delete_selection(true, true) {
            let mut it = self.buffer.iter_at_offset(self.cursor());
            self.buffer.backspace(&mut it, true, true);
        }
        self.focus();
    }

    pub fn delete_forward(&self) {
        if !self.buffer.delete_selection(true, true) {
            let p = self.cursor();
            if p < self.len() {
                let mut a = self.buffer.iter_at_offset(p);
                let mut b = self.buffer.iter_at_offset(p + 1);
                self.buffer.delete(&mut a, &mut b);
            }
        }
        self.focus();
    }

    pub fn move_caret(&self, delta: i32, absolute: bool) {
        let p = if absolute {
            if delta < 0 {
                0
            } else {
                self.len()
            }
        } else {
            self.cursor() + delta
        };
        self.set_cursor(p);
        self.focus();
    }

    fn on_key(&self, k: gdk::Key, m: gdk::ModifierType) -> glib::Propagation {
        let ctrl = m.contains(gdk::ModifierType::CONTROL_MASK);
        let shift = m.contains(gdk::ModifierType::SHIFT_MASK);
        if self.popover.is_visible() {
            let n = self.items.borrow().len() as i32;
            let cur = self.list.selected_row().map(|r| r.index()).unwrap_or(0);
            let sel = |i: i32| {
                if let Some(r) = self.list.row_at_index(i.clamp(0, (n - 1).max(0))) {
                    self.list.select_row(Some(&r));
                    r.grab_focus();
                    self.view.grab_focus();
                }
            };
            match k {
                gdk::Key::Down => {
                    sel(cur + 1);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Up => {
                    sel(cur - 1);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Page_Down => {
                    sel(cur + 8);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Page_Up => {
                    sel(cur - 8);
                    return glib::Propagation::Stop;
                }
                gdk::Key::Tab => {
                    self.accept_completion();
                    return glib::Propagation::Stop;
                }
                gdk::Key::Return | gdk::Key::KP_Enter if !shift && self.list.selected_row().is_some() => {
                    self.accept_completion();
                    return glib::Propagation::Stop;
                }
                gdk::Key::Escape => {
                    self.hide_popup();
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }
        match k {
            gdk::Key::Return | gdk::Key::KP_Enter if !shift => {
                self.hide_popup();
                self.history_index.set(-1);
                if let Some(f) = self.on_calculate.borrow().as_ref() {
                    f();
                }
                return glib::Propagation::Stop;
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                self.insert("\n", 0);
                return glib::Propagation::Stop;
            }
            gdk::Key::space if ctrl => {
                self.update_completion(true);
                return glib::Propagation::Stop;
            }
            gdk::Key::y | gdk::Key::Y if ctrl => {
                self.redo();
                return glib::Propagation::Stop;
            }
            gdk::Key::Up | gdk::Key::Down if !ctrl && !m.contains(gdk::ModifierType::ALT_MASK) => {
                let it = self.buffer.iter_at_offset(self.cursor());
                let line = it.line();
                let last = self.buffer.end_iter().line();
                let up = k == gdk::Key::Up;
                if (up && line == 0) || (!up && line == last) {
                    let hist = self.history.borrow().as_ref().map(|f| f()).unwrap_or_default();
                    if hist.is_empty() {
                        return glib::Propagation::Proceed;
                    }
                    if self.history_index.get() == -1 {
                        *self.history_draft.borrow_mut() = self.text();
                    }
                    let i = if up { (self.history_index.get() + 1).min(hist.len() as i32 - 1) } else { (self.history_index.get() - 1).max(-1) };
                    self.history_index.set(i);
                    let t = if i < 0 { self.history_draft.borrow().clone() } else { hist[i as usize].clone() };
                    self.set_text(&t, false);
                    return glib::Propagation::Stop;
                }
            }
            _ => {}
        }
        if let Some(ch) = k.to_unicode() {
            let rpn = self.rpn_active.borrow().as_ref().map(|f| f()).unwrap_or(false);
            if rpn && self.rpn_keys.get() && self.cursor() == self.len() && "+-*/^".contains(ch) && !ctrl {
                let t = self.text();
                let t = t.trim_end();
                let open = t.matches('(').count() > t.matches(')').count();
                if !(t.ends_with('(') || t.ends_with(',') || (ch == '-' && (t.ends_with('e') || t.ends_with('E'))) || open) {
                    if let Some(f) = self.on_rpn_operator.borrow().as_ref() {
                        f(ch);
                    }
                    return glib::Propagation::Stop;
                }
            }
        }
        glib::Propagation::Proceed
    }

    pub fn hide_popup(&self) {
        if self.popover.is_visible() {
            self.popover.popdown();
        }
        self.completion_start.set(-1);
    }

    fn current_word(&self) -> (String, i32) {
        let p = self.cursor() as usize;
        let chars: Vec<char> = self.text().chars().collect();
        let p = p.min(chars.len());
        let mut start = p;
        while start > 0 && is_ident_part(chars[start - 1]) {
            start -= 1;
        }
        while start < p && chars[start].is_ascii_digit() {
            start += 1;
        }
        (chars[start..p].iter().collect(), start as i32)
    }

    fn update_completion(&self, force: bool) {
        let mode = self.completion_mode.get();
        if mode == CompletionMode::Off && !force {
            self.hide_popup();
            return;
        }
        if !self.view.has_focus() && !force {
            self.hide_popup();
            return;
        }
        let (word, start) = self.current_word();
        let limited = matches!(mode, CompletionMode::LimitedFull | CompletionMode::LimitedStrict);
        let min = if limited { self.completion_min.get().max(3) } else { self.completion_min.get() };
        if !force && (word.chars().count() as i32) < min || (word.is_empty() && !force) {
            self.hide_popup();
            return;
        }
        let strict = matches!(mode, CompletionMode::Strict | CompletionMode::LimitedStrict);
        let items = self.matches(&word, strict);
        if items.is_empty() || (items.len() == 1 && items[0].name == word && !force) {
            self.hide_popup();
            return;
        }
        self.completion_start.set(start);
        while let Some(r) = self.list.row_at_index(0) {
            self.list.remove(&r);
        }
        for it in items.iter().take(60) {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_margin_start(6);
            row.set_margin_end(6);
            let n = gtk::Label::new(None);
            n.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(&it.name)));
            let t = gtk::Label::new(Some(&it.title));
            t.add_css_class("dim-label");
            t.set_ellipsize(gtk::pango::EllipsizeMode::End);
            t.set_hexpand(true);
            t.set_xalign(0.0);
            let k = gtk::Label::new(Some(it.kind));
            k.add_css_class("dim-label");
            k.add_css_class("caption");
            row.append(&n);
            row.append(&t);
            row.append(&k);
            self.list.append(&row);
        }
        *self.items.borrow_mut() = items.into_iter().take(60).collect();
        self.list.select_row(self.list.row_at_index(0).as_ref());
        let it = self.buffer.iter_at_offset(start);
        let rect = self.view.iter_location(&it);
        let (x, y) = self.view.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
        self.popover.set_pointing_to(Some(&gdk::Rectangle::new(x, y, 1, rect.height())));
        if !self.popover.is_visible() {
            self.popover.popup();
            self.view.grab_focus();
        }
    }

    fn matches(&self, word: &str, strict: bool) -> Vec<Completion> {
        let Some(snap) = self.snap.borrow().clone() else { return vec![] };
        let snap = snap.borrow();
        let lw = word.to_lowercase();
        let score = |name: &str, title: &str| -> i32 {
            let ln = name.to_lowercase();
            let len = name.chars().count() as i32;
            if ln == lw {
                return 100;
            }
            if name.starts_with(word) {
                return 90 - len;
            }
            if ln.starts_with(&lw) {
                return 80 - len;
            }
            if strict {
                return -1;
            }
            if ln.contains(&lw) {
                return 50 - len;
            }
            if lw.chars().count() >= 3 && title.to_lowercase().contains(&lw) {
                return 30;
            }
            -1
        };
        let mut res = vec![];
        for f in snap.functions.iter().filter(|f| f.item.active() && !f.item.hidden && !f.name().starts_with('_')) {
            let (best, bn) = f.item.names.iter().map(|n| (score(n, &f.item.title), n.clone())).max_by_key(|x| x.0).unwrap();
            if best >= 0 {
                res.push(Completion { name: bn, title: f.item.display_title().into(), kind: "function", is_fn: true, takes_args: f.max_args != 0, score: best });
            }
        }
        for v in snap.variables.iter().filter(|v| v.item.active() && !v.item.hidden) {
            let (best, bn) = v.item.names.iter().map(|n| (score(n, &v.item.title), n.clone())).max_by_key(|x| x.0).unwrap();
            if best >= 0 {
                res.push(Completion { name: bn, title: v.item.display_title().into(), kind: if v.is_unknown { "unknown" } else { "variable" }, is_fn: false, takes_args: false, score: best });
            }
        }
        for u in snap.units.iter().filter(|u| u.item.active() && !u.item.hidden) {
            let (best, bn) = u.item.names.iter().chain(std::iter::once(&u.plural)).map(|n| (score(n, &u.item.title), n.clone())).max_by_key(|x| x.0).unwrap();
            if best >= 0 {
                res.push(Completion { name: bn, title: u.item.display_title().into(), kind: if u.is_currency { "currency" } else { "unit" }, is_fn: false, takes_args: false, score: best - 2 });
            }
        }
        for p in PREFIXES.iter().filter(|p| p.name.starts_with(&lw)) {
            res.push(Completion { name: p.name.into(), title: format!("Prefix {}", p.symbol), kind: "prefix", is_fn: false, takes_args: false, score: 20 });
        }
        for kw in ["to", "where", "mod", "rem", "and", "or", "xor", "not", "per"] {
            if kw.starts_with(&lw) && kw != lw {
                res.push(Completion { name: kw.into(), title: "Operator".into(), kind: "keyword", is_fn: false, takes_args: false, score: 25 });
            }
        }
        res.sort_by(|a, b| b.score.cmp(&a.score).then(a.name.chars().count().cmp(&b.name.chars().count())).then(a.name.cmp(&b.name)));
        res
    }

    fn accept_completion(&self) {
        let idx = self.list.selected_row().map(|r| r.index()).unwrap_or(-1);
        let start = self.completion_start.get();
        let it = self.items.borrow().get(idx.max(0) as usize).cloned();
        if let (Some(it), true) = (it, idx >= 0 && start >= 0) {
            let chars: Vec<char> = self.text().chars().collect();
            let mut end = self.cursor() as usize;
            while end < chars.len() && is_ident_part(chars[end]) {
                end += 1;
            }
            let has_paren = end < chars.len() && chars[end] == '(';
            self.suppress.set(true);
            let mut a = self.buffer.iter_at_offset(start);
            let mut b = self.buffer.iter_at_offset(end as i32);
            self.buffer.delete(&mut a, &mut b);
            if it.is_fn && !has_paren {
                self.buffer.insert(&mut a, &format!("{}()", it.name));
                if it.takes_args {
                    self.set_cursor(self.cursor() - 1);
                }
            } else {
                self.buffer.insert(&mut a, &it.name);
            }
            self.suppress.set(false);
        }
        self.hide_popup();
    }
}