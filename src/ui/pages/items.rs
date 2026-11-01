// browser for functions, variables and units, with details and a unit converter
use crate::engine::algebra as simp;
use crate::engine::*;
use crate::ui::app::{App, Page};
use crate::ui::svc::Display;
use crate::ui::widgets::*;
use crate::ui::{dialogs, theme};
use gtk::glib;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
#[derive(Clone, Copy, PartialEq)]
pub enum ItemKind {
    Function,
    Variable,
    Unit,
}
#[derive(Clone)]
enum ItemRef {
    F(FnRef),
    V(VarRef),
    U(UnitRef),
}
impl ItemRef {
    fn item(&self) -> &Item {
        match self {
            ItemRef::F(f) => &f.item,
            ItemRef::V(v) => &v.item,
            ItemRef::U(u) => &u.item,
        }
    }
    fn key(&self) -> String {
        match self {
            ItemRef::U(u) => u.abbrev().to_string(),
            _ => self.item().names[0].clone(),
        }
    }
}
pub struct ItemsPage {
    app: Weak<App>,
    kind: ItemKind,
    root: gtk::Box,
    categories: gtk::ListBox,
    cat_names: RefCell<Vec<String>>,
    search: gtk::SearchEntry,
    list: gtk::ListBox,
    current: RefCell<Vec<ItemRef>>,
    details: gtk::Box,
    insert_btn: gtk::Button,
    edit_btn: gtk::Button,
    delete_btn: gtk::Button,
    active_btn: gtk::Button,
    fav_btn: gtk::Button,
    conv: RefCell<Option<Converter>>,
}
struct Converter {
    value: gtk::Entry,
    from: gtk::Label,
    to: gtk::DropDown,
    targets: RefCell<Vec<UnitRef>>,
    result: gtk::Entry,
}
fn type_name(t: ArgType) -> &'static str {
    match t {
        ArgType::Number => "number",
        ArgType::Integer => "integer",
        ArgType::Real => "real number",
        ArgType::NonNegInt => "non-negative integer",
        ArgType::PositiveInt => "positive integer",
        ArgType::Matrix => "matrix",
        ArgType::Vector => "vector",
        ArgType::Text => "text",
        ArgType::Boolean => "boolean (0 or 1)",
        ArgType::Date => "date",
        ArgType::Angle => "angle",
        ArgType::Unevaluated => "expression or variable name",
        _ => "free value",
}   }
impl Page for ItemsPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn activated(&self) {
        if self.list.selected_row().is_none() {
            self.list.select_row(self.list.row_at_index(0).as_ref());
        }
        self.show_details();
    }
    fn definitions_changed(&self) {
        self.fill_categories();
        self.fill_list();
    }
    fn options_changed(&self) {
        self.show_details();
    }
}

impl ItemsPage {
    pub fn new(app: &Rc<App>, kind: ItemKind) -> Rc<ItemsPage> {
        let root = vbox(10);
        margins(&root, 16);
        let title = label(match kind {
            ItemKind::Function => "Functions",
            ItemKind::Variable => "Variables and Constants",
            ItemKind::Unit => "Units",
        });
        title.add_css_class("oc-page-title");
        root.append(&title);
        let content = hbox(12);
        content.set_vexpand(true);
        let categories = gtk::ListBox::new();
        categories.add_css_class("navigation-sidebar");
        let cs = scrolled(&categories);
        cs.set_hexpand(false);
        cs.set_size_request(200, -1);
        content.append(&cs);
        let middle = vbox(8);
        middle.set_hexpand(true);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search"));
        middle.append(&search);
        let list = gtk::ListBox::new();
        list.add_css_class("rich-list");
        middle.append(&scrolled(&list));
        content.append(&middle);
        let detail_card = vbox(8);
        detail_card.set_size_request(340, -1);
        detail_card.set_hexpand(false);
        let details = vbox(4);
        margins(&details, 6);
        let ds = scrolled(&details);
        detail_card.append(&ds);
        let actions = gtk::FlowBox::new();
        actions.set_selection_mode(gtk::SelectionMode::None);
        actions.set_max_children_per_line(4);
        detail_card.append(&actions);
        content.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        content.append(&detail_card);
        root.append(&content);
        let page = Rc::new(ItemsPage {
            app: Rc::downgrade(app),
            kind,
            root,
            categories,
            cat_names: RefCell::new(vec![]),
            search,
            list,
            current: RefCell::new(vec![]),
            details,
            insert_btn: gtk::Button::with_label(if kind == ItemKind::Function { "Insert…" } else { "Insert" }),
            edit_btn: gtk::Button::with_label("Edit…"),
            delete_btn: gtk::Button::with_label("Delete"),
            active_btn: gtk::Button::with_label("Deactivate"),
            fav_btn: gtk::Button::with_label("Favorite"),
            conv: RefCell::new(None),
        });



        page.insert_btn.add_css_class("suggested-action");
        let new_btn = gtk::Button::with_label("New…");
        for b in [&page.insert_btn, &new_btn, &page.edit_btn, &page.delete_btn, &page.active_btn, &page.fav_btn] {
            actions.insert(b, -1);
        }
        let w = Rc::downgrade(&page);
        let on = move |f: fn(&Rc<ItemsPage>)| {
            let w = w.clone();
            move |_: &gtk::Button| {
                if let Some(p) = w.upgrade() {
                    f(&p);
                }
            }
        };
        page.insert_btn.connect_clicked(on(|p| p.insert_selected()));
        page.edit_btn.connect_clicked(on(|p| p.edit_selected()));
        page.delete_btn.connect_clicked(on(|p| p.delete_selected()));
        page.active_btn.connect_clicked(on(|p| p.toggle_active()));
        page.fav_btn.connect_clicked(on(|p| p.toggle_favorite()));
        new_btn.connect_clicked(on(|p| {
            let a = p.app();
            match p.kind {
                ItemKind::Function => a.run_action("newfunction"),
                ItemKind::Variable => a.run_action("newvariable"),
                ItemKind::Unit => a.run_action("newunit"),
            }
        }));
        if kind == ItemKind::Variable {
            let nu = gtk::Button::with_label("New Unknown…");
            nu.connect_clicked(on(|p| p.app().run_action("newunknown")));
            actions.insert(&nu, -1);
        }
        if kind == ItemKind::Unit {
            page.root.append(&page.build_converter());
        }
        let w = Rc::downgrade(&page);
        page.categories.connect_row_selected(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.fill_list();
            }
        });
        let w = Rc::downgrade(&page);
        page.search.connect_search_changed(move |_| {
            if let Some(p) = w.upgrade() {
                p.fill_list();
            }
        });
        let w = Rc::downgrade(&page);
        page.list.connect_row_selected(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.show_details();
            }
        });
        let w = Rc::downgrade(&page);
        page.list.connect_row_activated(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.insert_selected();
            }
        });



        page.fill_categories();
        page
    }

    fn app(&self) -> Rc<App> {
        self.app.upgrade().expect("app")
    }



    fn build_converter(self: &Rc<Self>) -> gtk::Widget {
        let frame = gtk::Frame::new(None);
        let row = hbox(8);
        margins(&row, 10);
        let value = entry("1");
        value.set_width_chars(10);
        let from = gtk::Label::new(Some("m"));
        from.add_css_class("heading");
        let to = gtk::DropDown::from_strings(&[]);
        to.set_hexpand(true);
        let result = gtk::Entry::new();
        result.set_editable(false);
        result.set_hexpand(true);
        row.append(&label("Convert"));
        row.append(&value);
        row.append(&from);
        row.append(&label("to"));
        row.append(&to);
        row.append(&label("="));
        row.append(&result);
        let w = Rc::downgrade(self);



        let go = button("Convert", move || {
            if let Some(p) = w.upgrade() {
                p.convert();
            }
        });



        go.add_css_class("suggested-action");
        let w = Rc::downgrade(self);
        let swap = button("Swap", move || {
            if let Some(p) = w.upgrade() {
                p.swap_units();
            }
        });
        let w = Rc::downgrade(self);
        let ins = button("Insert", move || {
            if let Some(p) = w.upgrade() {
                let t = p.conv.borrow().as_ref().map(|c| c.result.text().to_string()).unwrap_or_default();
                if !t.is_empty() {
                    p.app().insert_into_expression(&t, false);
                }
            }
        });
        row.append(&go);
        row.append(&swap);
        row.append(&ins);
        let w = Rc::downgrade(self);
        value.connect_activate(move |_| {
            if let Some(p) = w.upgrade() {
                p.convert();
            }
        });
        let w = Rc::downgrade(self);
        to.connect_selected_notify(move |_| {
            if let Some(p) = w.upgrade() {
                p.convert();
            }
        });
        frame.set_child(Some(&row));
        *self.conv.borrow_mut() = Some(Converter { value, from, to, targets: RefCell::new(vec![]), result });
        frame.upcast()
    }

    fn fill_converter_targets(&self, u: &UnitRef) {
        let conv = self.conv.borrow();
        let Some(c) = conv.as_ref() else { return };


        if c.from.text() == u.abbrev() && !c.targets.borrow().is_empty() {
            return;
        }


        c.from.set_text(u.abbrev());
        let mut same: Vec<UnitRef> = self.app().snap.borrow().units.iter().filter(|x| x.item.active() && x.name() != u.name() && x.dim.same_as(&u.dim)).cloned().collect();
        same.sort_by(|a, b| a.item.display_title().cmp(b.item.display_title()));
        let names: Vec<String> = same.iter().map(|x| format!("{} ({})", x.item.display_title(), x.abbrev())).collect();
        let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        *c.targets.borrow_mut() = same;
        c.to.set_model(Some(&gtk::StringList::new(&refs)));
        c.to.set_selected(0);
        drop(conv);
        self.convert();
    }

    fn convert(&self) {
        let conv = self.conv.borrow();
        let Some(c) = conv.as_ref() else { return };
        let Some(target) = c.targets.borrow().get(c.to.selected() as usize).cloned() else { return };
        let expr = format!("({}) {} to {}", c.value.text(), c.from.text(), target.abbrev());
        let result = c.result.clone();
        self.app().svc.run(move |calc| Display::from(&calc.calculate(&expr, 3000, true), calc), move |d| result.set_text(&d.error.unwrap_or(d.text)));
    }

    fn swap_units(&self) {
        let (target, from_name) = {
            let conv = self.conv.borrow();
            let Some(c) = conv.as_ref() else { return };
            let t = c.targets.borrow().get(c.to.selected() as usize).cloned();
            (t, c.from.text().to_string())
        };
        let Some(target) = target else { return };
        if let Some(c) = self.conv.borrow().as_ref() {
            c.targets.borrow_mut().clear();
        }
        self.fill_converter_targets(&target);
        let conv = self.conv.borrow();
        if let Some(c) = conv.as_ref() {
            let idx = c.targets.borrow().iter().position(|u| u.abbrev() == from_name);
            if let Some(i) = idx {
                c.to.set_selected(i as u32);
            }
        }
    }

    fn all(&self) -> Vec<ItemRef> {
        let app = self.app();
        let snap = app.snap.borrow();
        match self.kind {
            ItemKind::Function => snap.functions.iter().filter(|f| !f.name().starts_with('_') && !f.item.hidden).cloned().map(ItemRef::F).collect(),
            ItemKind::Variable => snap.variables.iter().filter(|v| !v.item.hidden).cloned().map(ItemRef::V).collect(),
            ItemKind::Unit => snap.units.iter().filter(|u| !u.item.hidden).cloned().map(ItemRef::U).collect(),
        }
    }

    fn favorites(&self) -> Vec<String> {
        let app = self.app();
        let s = app.settings.borrow();
        match self.kind {
            ItemKind::Function => s.favorite_functions.clone(),
            ItemKind::Variable => s.favorite_variables.clone(),
            ItemKind::Unit => s.favorite_units.clone(),
        }
    }

    fn fill_categories(&self) {
        let sel = self.categories.selected_row().and_then(|r| self.cat_names.borrow().get(r.index() as usize).cloned());
        let all = self.all();
        let mut names = vec!["All".to_string(), "Favorites".to_string()];
        if all.iter().any(|i| i.item().is_user) {
            names.push(format!(
                "User {}",
                match self.kind {
                    ItemKind::Function => "functions",
                    ItemKind::Variable => "variables",
                    ItemKind::Unit => "units",
                }
            ));
        }
        if all.iter().any(|i| !i.item().active()) {
            names.push("Inactive".into());
        }
        let mut cats: Vec<String> = all.iter().map(|i| if i.item().category.is_empty() { "Other".to_string() } else { i.item().category.clone() }).collect();
        cats.sort();
        cats.dedup();
        names.extend(cats);
        while let Some(r) = self.categories.row_at_index(0) {
            self.categories.remove(&r);
        }
        for n in &names {
            let l = label(n);
            l.set_margin_top(4);
            l.set_margin_bottom(4);
            self.categories.append(&l);
        }
        let idx = sel.and_then(|s| names.iter().position(|n| *n == s)).unwrap_or(0);
        *self.cat_names.borrow_mut() = names;
        self.categories.select_row(self.categories.row_at_index(idx as i32).as_ref());
    }

    fn value_text(&self, v: &Variable) -> String {
        if v.is_unknown {
            return format!("unknown ({})", v.atype.name().to_lowercase());
        }
        if v.dynamic.is_some() && v.value_text.is_empty() {
            return if v.keep_symbolic { "constant".into() } else { "dynamic".into() };
        }
        v.value_text.clone()
    }

    fn fill_list(&self) {
        let cat = self.categories.selected_row().and_then(|r| self.cat_names.borrow().get(r.index() as usize).cloned()).unwrap_or_else(|| "All".into());
        let q = self.search.text().trim().to_lowercase();
        let favs = self.favorites();
        let mut items: Vec<ItemRef> = self
            .all()
            .into_iter()
            .filter(|i| {
                let it = i.item();
                let in_cat = match cat.as_str() {
                    "All" => true,
                    "Favorites" => favs.contains(&i.key()),
                    "Inactive" => !it.active(),
                    c if c.starts_with("User ") => it.is_user,
                    c => (if it.category.is_empty() { "Other" } else { &it.category }) == c,
                };
                in_cat && (q.is_empty() || it.display_title().to_lowercase().contains(&q) || it.names.iter().any(|n| n.to_lowercase().contains(&q)) || it.description.to_lowercase().contains(&q))
            })
            .collect();
        items.sort_by(|a, b| a.item().display_title().cmp(b.item().display_title()));
        while let Some(r) = self.list.row_at_index(0) {
            self.list.remove(&r);
        }
        for i in &items {
            let row = hbox(12);
            row.set_margin_top(4);
            row.set_margin_bottom(4);
            row.set_margin_start(6);
            row.set_margin_end(6);
            let it = i.item();
            let t = label(&format!("{}{}", it.display_title(), if it.active() { "" } else { "  (inactive)" }));
            t.set_hexpand(true);
            t.set_ellipsize(gtk::pango::EllipsizeMode::End);
            row.append(&t);
            let names = match i {
                ItemRef::F(f) => f.signature(),
                _ => it.names.iter().take(3).cloned().collect::<Vec<_>>().join(", "),
            };
            let n = gtk::Label::new(Some(&names));
            n.add_css_class("dim-label");
            n.add_css_class("oc-mono");
            n.set_ellipsize(gtk::pango::EllipsizeMode::End);
            n.set_max_width_chars(28);
            row.append(&n);
            let value = match i {
                ItemRef::V(v) => Some(self.value_text(v)),
                ItemRef::U(u) => Some(if u.is_base {
                    "base unit".to_string()
                } else if u.definition.starts_with('@') {
                    "offset scale".into()
                } else {
                    u.definition.clone()
                }),
                _ => None,
            };
            if let Some(v) = value {
                let l = gtk::Label::new(Some(&v));
                l.add_css_class("dim-label");
                l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                l.set_max_width_chars(22);
                l.set_width_chars(16);
                l.set_xalign(1.0);
                row.append(&l);
            }
            self.list.append(&row);
        }
        *self.current.borrow_mut() = items;
        self.list.select_row(self.list.row_at_index(0).as_ref());
        self.show_details();
    }

    fn selected(&self) -> Option<ItemRef> {
        let r = self.list.selected_row()?;
        self.current.borrow().get(r.index() as usize).cloned()
    }

    fn add_line(&self, markup: &str) -> gtk::Label {
        let l = gtk::Label::new(None);
        l.set_markup(markup);
        l.set_xalign(0.0);
        l.set_wrap(true);
        l.set_selectable(true);
        self.details.append(&l);
        l
    }

    fn show_details(&self) {
        while let Some(c) = self.details.first_child() {
            self.details.remove(&c);
        }
        let it = self.selected();
        for b in [&self.insert_btn, &self.edit_btn, &self.active_btn, &self.fav_btn] {
            b.set_sensitive(it.is_some());
        }
        self.delete_btn.set_sensitive(it.as_ref().map(|i| i.item().is_user).unwrap_or(false));
        let Some(it) = it else { return };
        let item = it.item().clone();
        self.active_btn.set_label(if item.active() { "Deactivate" } else { "Activate" });
        self.fav_btn.set_label(if self.favorites().contains(&it.key()) { "Unfavorite" } else { "Favorite" });
        let esc = |s: &str| glib::markup_escape_text(s).to_string();
        let dark = theme::is_dark();
        let color = |t| theme::token_color(t, dark);
        use crate::engine::formatter::TokType;
        self.add_line(&format!("<span size=\"large\" weight=\"bold\">{}</span>", esc(item.display_title())));
        match &it {
            ItemRef::F(f) => {
                self.add_line(&format!("<tt><span foreground=\"{}\">{}</span></tt>\n", color(TokType::Function), esc(&f.signature())));
                if !item.description.is_empty() {
                    self.add_line(&format!("{}\n", esc(&item.description)));
                }
                if !f.args.is_empty() {
                    let mut s = String::from("<b>Arguments</b>\n");
                    for (i, a) in f.args.iter().enumerate() {
                        let opt = if i as i32 >= f.min_args {
                            match &a.default {
                                Some(d) => format!(" (default {})", d),
                                None => " (optional)".into(),
                            }
                        } else {
                            String::new()
                        };
                        s += &format!("  {}. {}: {}{}\n", i + 1, esc(&a.name), type_name(a.ty), esc(&opt));
                    }
                    if f.max_args < 0 {
                        s += "  … more arguments allowed\n";
                    }
                    self.add_line(&s);
                }
                if item.is_user {
                    self.add_line(&format!("<tt>Expression: {}</tt>", esc(&f.formula)));
                }
                if !f.example.is_empty() {
                    self.add_line(&format!("<span alpha=\"70%\">Example: {}</span>", esc(&f.example)));
                }
                self.add_line(&format!("<span alpha=\"70%\">Names: {}</span>", esc(&item.names.join(", "))));
            }
            ItemRef::V(v) => {
                self.add_line(&format!("<tt><span foreground=\"{}\">{}</span></tt>\n", color(TokType::Variable), esc(&item.names.join(", "))));
                if v.is_unknown {
                    let sign = if v.asign != AssumeSign::Unknown { format!(", {}", words(v.asign.name()).to_lowercase()) } else { String::new() };
                    self.add_line(&format!("Unknown variable, assumed {}{}.", v.atype.name().to_lowercase(), sign));
                } else {
                    let val = self.add_line("<span alpha=\"70%\">Value:</span> …");
                    let (name, approx, o) = (item.names[0].clone(), v.is_approximate, self.app().svc.opts().clone());
                    self.app().svc.run(
                        move |c| match attempt(|| c.eval_text(&name)) {
                            Ok(e) => {
                                let (t, r) = formatter::format(&e, &o);
                                if approx || r {
                                    format!("≈ {}", t)
                                } else {
                                    t
                                }
                            }
                            Err(m) => m,
                        },
                        move |t| val.set_markup(&format!("<span alpha=\"70%\">Value:</span> <b>{}</b>", glib::markup_escape_text(&t))),
                    );
                    if !v.value_text.is_empty() {
                        self.add_line(&format!("<tt><span alpha=\"70%\">Definition: {}</span></tt>", esc(&v.value_text)));
                    }
                }
                if !item.description.is_empty() {
                    self.add_line(&format!("\n{}", esc(&item.description)));
                }
            }
            ItemRef::U(u) => {
                let mut names = item.names.join(", ");
                if !u.plural.is_empty() && !item.names.contains(&u.plural) {
                    names += &format!(", {}", u.plural);
                }
                self.add_line(&format!("<tt><span foreground=\"{}\">{}</span></tt>\n", color(TokType::Unit), esc(&names)));
                if u.is_base {
                    self.add_line(&format!("SI base unit of {}.", esc(&item.category.to_lowercase())));
                } else if u.has_offset() {
                    let (factor, offset) = (u.factor.clone(), u.offset.clone());
                    let l = self.add_line("");
                    self.app().svc.run(
                        move |_| {
                            let f = if factor.is_one() { String::new() } else { format!("{} × ", formatter::plain_text(&num_r(factor))) };
                            format!("Temperature scale: K = {}value + {}", f, formatter::plain_text(&num_r(offset)))
                        },
                        move |t| l.set_text(&t),
                    );
                } else {
                    self.add_line(&esc(&format!("1 {} = {}", u.abbrev(), u.definition)));
                    let l = self.add_line("");
                    let (u2, o) = (u.clone(), self.app().svc.opts().clone());
                    self.app().svc.run(
                        move |_| attempt(|| format!("1 {} = {} (base units)", u2.abbrev(), formatter::format(&simp::mul2(&num_r(u2.factor.clone()), &units::base_units(&u2.dim)), &o).0)).unwrap_or_default(),
                        move |t| l.set_markup(&format!("<span alpha=\"70%\">{}</span>", glib::markup_escape_text(&t))),
                    );
                }
                self.add_line(&format!("\n<span alpha=\"70%\">System: {}\nPrefixes: {}</span>", esc(&u.system), format!("{:?}", u.prefixes).to_lowercase()));
                self.fill_converter_targets(u);
            }
        }
        self.add_line(&format!("<span alpha=\"55%\">Category: {}{}</span>", esc(&item.category), if item.is_user { " (user defined)" } else { "" }));
    }

    fn insert_selected(&self) {
        let Some(it) = self.selected() else { return };
        let app = self.app();
        match &it {
            ItemRef::F(f) => dialogs::insert_function(&app, f),
            ItemRef::U(u) => {
                App::add_recent(&mut app.settings.borrow_mut().recent_units, u.abbrev());
                app.insert_into_expression(u.abbrev(), false);
            }
            ItemRef::V(v) => {
                App::add_recent(&mut app.settings.borrow_mut().recent_variables, &v.item.names[0]);
                app.insert_into_expression(&v.item.names[0], false);
            }
        }
    }

    fn edit_selected(&self) {
        let Some(it) = self.selected() else { return };
        let app = self.app();
        match it {
            ItemRef::F(f) => dialogs::function_edit(&app, Some(f), ""),
            ItemRef::V(v) if v.is_unknown => dialogs::unknown_edit(&app, Some(v)),
            ItemRef::V(v) => dialogs::variable_edit(&app, Some(v), ""),
            ItemRef::U(u) => dialogs::unit_edit(&app, Some(u)),
        }
    }

    fn delete_selected(self: &Rc<Self>) {
        let Some(it) = self.selected().filter(|i| i.item().is_user) else { return };
        let app = self.app();
        let a2 = app.clone();
        confirm(app.parent(), &format!("Delete \"{}\"?", it.item().display_title()), move || {
            let it = it.clone();
            let _ = a2.svc.run_sync(move |c| match &it {
                ItemRef::F(f) => c.remove_function(f),
                ItemRef::V(v) => c.remove_variable(v),
                ItemRef::U(u) => c.remove_unit(u),
            });
            a2.definitions_changed();
        });
    }

    fn toggle_active(&self) {
        let Some(it) = self.selected() else { return };
        let app = self.app();
        let _ = app.svc.run_sync(move |c| match it {
            ItemRef::F(f) => {
                let mut n = (*f).clone();
                n.item.inactive = !n.item.inactive;
                c.add_function(n);
            }
            ItemRef::V(v) => {
                let mut n = (*v).clone();
                n.item.inactive = !n.item.inactive;
                c.add_variable(n);
            }
            ItemRef::U(u) => {
                let mut n = (*u).clone();
                n.item.inactive = !n.item.inactive;
                c.add_unit(n);
            }
        });
        app.definitions_changed();
    }

    fn toggle_favorite(&self) {
        let Some(it) = self.selected() else { return };
        let app = self.app();
        let k = it.key();
        {
            let mut s = app.settings.borrow_mut();
            let list = match self.kind {
                ItemKind::Function => &mut s.favorite_functions,
                ItemKind::Variable => &mut s.favorite_variables,
                ItemKind::Unit => &mut s.favorite_units,
            };
            if let Some(i) = list.iter().position(|x| *x == k) {
                list.remove(i);
            } else {
                list.push(k);
            }
        }
        app.mark_dirty();
        app.rebuild_item_menus();
        self.show_details();
    }
}