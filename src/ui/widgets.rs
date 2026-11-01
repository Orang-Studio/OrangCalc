// small helpers for building GTK widgets and simple dialogs
use gtk::prelude::*;
use gtk::{gdk, glib};
pub fn label(text: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    l
}
pub fn dim(text: &str) -> gtk::Label {
    let l = label(text);
    l.add_css_class("dim-label");
    l.set_wrap(true);
    l
}
pub fn heading(text: &str) -> gtk::Label {
    let l = label(text);
    l.add_css_class("heading");
    l
}
pub fn button(text: &str, f: impl Fn() + 'static) -> gtk::Button {
    let b = gtk::Button::with_label(text);
    b.connect_clicked(move |_| f());
    b
}
pub fn vbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, spacing)
}
pub fn hbox(spacing: i32) -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, spacing)
}
pub fn margins<W: IsA<gtk::Widget>>(w: &W, m: i32) {
    w.set_margin_top(m);
    w.set_margin_bottom(m);
    w.set_margin_start(m);
    w.set_margin_end(m);
}
pub fn entry(text: &str) -> gtk::Entry {
    let e = gtk::Entry::new();
    e.set_text(text);
    e
}
pub fn combo(items: &[&str], selected: usize) -> gtk::DropDown {
    let d = gtk::DropDown::from_strings(items);
    d.set_selected(selected.min(items.len().saturating_sub(1)) as u32);
    d
}
pub fn combo_strings(items: &[String], selected: usize) -> gtk::DropDown {
    let v: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    combo(&v, selected)
}

pub fn scrolled<W: IsA<gtk::Widget>>(w: &W) -> gtk::ScrolledWindow {
    let s = gtk::ScrolledWindow::new();
    s.set_child(Some(w));
    s.set_vexpand(true);
    s.set_hexpand(true);
    s
}
pub fn copy_text(text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(d) = gdk::Display::default() {
        d.clipboard().set_text(text);
    }
}
pub fn root_window<W: IsA<gtk::Widget>>(w: &W) -> Option<gtk::Window> {
    w.root().and_then(|r| r.downcast::<gtk::Window>().ok())
}
pub fn message(parent: Option<&gtk::Window>, text: &str) {
    let d = gtk::AlertDialog::builder().message(text).modal(true).build();
    d.show(parent);
}
pub fn confirm(parent: Option<&gtk::Window>, text: &str, yes: impl FnOnce() + 'static) {
    let d = gtk::AlertDialog::builder().message(text).buttons(["Cancel", "OK"]).cancel_button(0).default_button(1).modal(true).build();
    d.choose(parent, gtk::gio::Cancellable::NONE, move |r| {
        if r == Ok(1) {
            yes();
        }
    });
}
// modal window with a content area and OK/Cancel; on_ok returns false to keep it open
pub struct Dialog {
    pub win: gtk::Window,
    pub content: gtk::Box,
    pub ok: gtk::Button,
    pub buttons: gtk::Box,
}
impl Dialog {
    pub fn new(parent: Option<&gtk::Window>, title: &str, ok_label: &str) -> Dialog {
        let win = gtk::Window::builder().title(title).modal(true).resizable(true).build();
        if let Some(p) = parent {
            win.set_transient_for(Some(p));
        }
        win.set_destroy_with_parent(true);
        let outer = vbox(12);
        margins(&outer, 16);
        let content = vbox(8);
        content.set_vexpand(true);
        outer.append(&content);
        let buttons = hbox(8);
        buttons.set_halign(gtk::Align::End);
        let cancel = gtk::Button::with_label("Cancel");
        let ok = gtk::Button::with_label(ok_label);
        ok.add_css_class("suggested-action");
        buttons.append(&cancel);
        buttons.append(&ok);
        outer.append(&buttons);
        win.set_child(Some(&outer));
        let w = win.clone();
        cancel.connect_clicked(move |_| w.close());
        let w = win.clone();
        let key = gtk::EventControllerKey::new();
        key.connect_key_pressed(move |_, k, _, _| {
            if k == gdk::Key::Escape {
                w.close();
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        win.add_controller(key);
        win.set_default_widget(Some(&ok));
        Dialog { win, content, ok, buttons }
    }
    pub fn on_ok(&self, f: impl Fn() -> bool + 'static) {
        let w = self.win.clone();
        self.ok.connect_clicked(move |_| {
            if f() {
                w.close();
            }
        });
    }
    pub fn present(&self) {
        self.win.present();
    }
}
// asks for one line of text
pub fn ask(parent: Option<&gtk::Window>, title: &str, prompt: &str, default: &str, done: impl Fn(String) + 'static) {
    let d = Dialog::new(parent, title, "OK");
    d.win.set_default_width(380);
    d.content.append(&label(prompt));
    let e = entry(default);
    e.set_activates_default(true);
    d.content.append(&e);
    let e2 = e.clone();
    d.on_ok(move || {
        done(e2.text().to_string());
        true
    });
    d.present();
    e.grab_focus();
}
// grid of label + widget rows
pub struct Form {
    pub grid: gtk::Grid,
    row: i32,
}
impl Form {
    pub fn new() -> Form {
        let grid = gtk::Grid::new();
        grid.set_row_spacing(8);
        grid.set_column_spacing(12);
        Form { grid, row: 0 }
    }
    pub fn add<W: IsA<gtk::Widget>>(&mut self, text: &str, w: &W) {
        let l = label(text);
        self.grid.attach(&l, 0, self.row, 1, 1);
        w.set_hexpand(true);
        self.grid.attach(w, 1, self.row, 1, 1);
        self.row += 1;
    }
    pub fn full<W: IsA<gtk::Widget>>(&mut self, w: &W) {
        self.grid.attach(w, 0, self.row, 2, 1);
        self.row += 1;
    }
}
pub fn save_file(parent: Option<&gtk::Window>, title: &str, name: &str, done: impl Fn(std::path::PathBuf) + 'static) {
    let d = gtk::FileDialog::builder().title(title).initial_name(name).modal(true).build();
    d.save(parent, gtk::gio::Cancellable::NONE, move |r| {
        if let Ok(f) = r {
            if let Some(p) = f.path() {
                done(p);
            }
        }
    });
}
pub fn open_file(parent: Option<&gtk::Window>, title: &str, done: impl Fn(std::path::PathBuf) + 'static) {
    let d = gtk::FileDialog::builder().title(title).modal(true).build();
    d.open(parent, gtk::gio::Cancellable::NONE, move |r| {
        if let Ok(f) = r {
            if let Some(p) = f.path() {
                done(p);
            }
        }
    });
}
pub fn words(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in s.chars() {
        if ch.is_uppercase() && prev_lower {
            out.push(' ');
        }
        prev_lower = ch.is_lowercase();
        out.push(ch);
    }
    out
}