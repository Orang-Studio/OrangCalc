// data set browser and editor
use crate::engine::parser::is_ident_part;
use crate::engine::*;
use crate::ui::app::{App, Page};
use crate::ui::widgets::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;
pub struct DataSetsPage {
    app: Weak<App>,
    root: gtk::Box,
    sets: gtk::ListBox,
    set_list: RefCell<Vec<Arc<DataSet>>>,
    search: gtk::SearchEntry,
    desc: gtk::Label,
    objects: gtk::ListBox,
    shown: RefCell<Vec<DataObject>>,
    props: gtk::ListBox,
    prop_rows: RefCell<Vec<DataProperty>>,
}
impl Page for DataSetsPage {
    fn widget(&self) -> gtk::Widget {
        self.root.clone().upcast()
    }
    fn definitions_changed(&self) {
        self.fill_sets();
    }
}
impl DataSetsPage {
    pub fn new(app: &Rc<App>) -> Rc<DataSetsPage> {
        let root = vbox(10);
        margins(&root, 16);
        let title = label("Data Sets");
        title.add_css_class("oc-page-title");
        root.append(&title);
        let content = hbox(12);
        content.set_vexpand(true);
        let sets = gtk::ListBox::new();
        sets.add_css_class("navigation-sidebar");
        let ss = scrolled(&sets);
        ss.set_hexpand(false);
        ss.set_size_request(200, -1);
        content.append(&ss);
        let middle = vbox(8);
        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Search objects"));
        let desc = dim("");
        let objects = gtk::ListBox::new();
        objects.add_css_class("rich-list");
        middle.append(&search);
        middle.append(&desc);
        middle.append(&scrolled(&objects));
        content.append(&middle);
        content.append(&gtk::Separator::new(gtk::Orientation::Vertical));
        let card = vbox(8);
        card.set_size_request(360, -1);
        card.set_hexpand(false);
        let props = gtk::ListBox::new();
        props.add_css_class("rich-list");
        card.append(&scrolled(&props));
        let bar = gtk::FlowBox::new();
        bar.set_selection_mode(gtk::SelectionMode::None);
        card.append(&bar);
        content.append(&card);
        root.append(&content);
        let page = Rc::new(DataSetsPage {
            app: Rc::downgrade(app),
            root,
            sets,
            set_list: RefCell::new(vec![]),
            search,
            desc,
            objects,
            shown: RefCell::new(vec![]),
            props,
            prop_rows: RefCell::new(vec![]),
        });
        let mk = |t: &str, primary: bool, f: fn(&Rc<DataSetsPage>)| {
            let w = Rc::downgrade(&page);
            let b = button(t, move || {
                if let Some(p) = w.upgrade() {
                    f(&p);
                }
            });
            if primary {
                b.add_css_class("suggested-action");
            }
            bar.insert(&b, -1);
        };
        mk("Insert value", true, |p| p.insert_prop(false));
        mk("Insert lookup", false, |p| p.insert_prop(true));
        mk("New set…", false, |p| p.edit_set(None));
        mk("Edit set…", false, |p| {
            if let Some(s) = p.set().filter(|s| s.item.is_user) {
                p.edit_set(Some(s));
            }
        });
        mk("Delete set", false, |p| {
            let Some(s) = p.set().filter(|s| s.item.is_user) else { return };
            let app = p.app();
            let a2 = app.clone();
            confirm(app.parent(), &format!("Delete data set \"{}\"?", s.item.display_title()), move || {
                let n = s.item.name.clone();
                let _ = a2.svc.run_sync(move |c| c.defs.borrow_mut().data_sets.retain(|d| d.item.name != n));
                a2.definitions_changed();
            });
        });
        let w = Rc::downgrade(&page);
        page.sets.connect_row_selected(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.fill_objects();
            }
        });
        let w = Rc::downgrade(&page);
        page.search.connect_search_changed(move |_| {
            if let Some(p) = w.upgrade() {
                p.fill_objects();
            }
        });
        let w = Rc::downgrade(&page);
        page.objects.connect_row_selected(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.fill_props();
            }
        });
        let w = Rc::downgrade(&page);
        page.props.connect_row_activated(move |_, _| {
            if let Some(p) = w.upgrade() {
                p.insert_prop(false);
            }
        });
        page.fill_sets();
        page
    }

    fn app(&self) -> Rc<App> {
        self.app.upgrade().expect("app")
    }

    fn set(&self) -> Option<Arc<DataSet>> {
        let r = self.sets.selected_row()?;
        self.set_list.borrow().get(r.index() as usize).cloned()
    }

    fn obj(&self) -> Option<DataObject> {
        let i = self.objects.selected_row().map(|r| r.index()).unwrap_or(0);
        self.shown.borrow().get(i as usize).cloned()
    }

    fn fill_sets(&self) {
        let cur = self.set().map(|s| s.item.name.clone());
        let list = self.app().snap.borrow().data_sets.clone();
        while let Some(r) = self.sets.row_at_index(0) {
            self.sets.remove(&r);
        }
        for d in &list {
            let l = label(&format!("{}{}", d.item.display_title(), if d.item.is_user { "  (user)" } else { "" }));
            l.set_margin_top(4);
            l.set_margin_bottom(4);
            self.sets.append(&l);
        }
        let idx = cur.and_then(|n| list.iter().position(|d| d.item.name == n)).unwrap_or(0);
        *self.set_list.borrow_mut() = list;
        self.sets.select_row(self.sets.row_at_index(idx as i32).as_ref());
    }

    fn fill_objects(&self) {
        while let Some(r) = self.objects.row_at_index(0) {
            self.objects.remove(&r);
        }
        let Some(d) = self.set() else { return };
        self.desc.set_text(&format!("{}{}", d.item.description, if d.item.is_user { String::new() } else { format!("   Use {}(object, property) in expressions.", d.item.name) }));
        let mut cols: Vec<DataProperty> = d.properties.iter().filter(|p| p.is_key).take(3).cloned().collect();
        if let Some(extra) = d.properties.iter().find(|p| !p.is_key && !p.hidden && !p.is_text) {
            cols.push(extra.clone());
        }
        let q = self.search.text().trim().to_lowercase();
        let shown: Vec<DataObject> = d.objects.iter().filter(|o| q.is_empty() || o.values.iter().any(|(_, v)| v.to_lowercase().contains(&q))).cloned().collect();
        for o in &shown {
            let row = hbox(12);
            row.set_margin_top(3);
            row.set_margin_bottom(3);
            for p in &cols {
                let l = label(o.get(&p.name).unwrap_or(""));
                l.set_hexpand(true);
                l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                row.append(&l);
            }
            self.objects.append(&row);
        }
        *self.shown.borrow_mut() = shown;
        self.objects.set_header_func(move |row, _| {
            if row.index() == 0 {
                let h = hbox(12);
                for c in header_titles(&cols) {
                    let l = label(&c);
                    l.add_css_class("heading");
                    l.set_hexpand(true);
                    h.append(&l);
                }
                h.set_margin_bottom(4);
                row.set_header(Some(&h));
            } else {
                row.set_header(None::<&gtk::Widget>);
            }
        });
        self.objects.select_row(self.objects.row_at_index(0).as_ref());
        self.fill_props();
    }

    fn fill_props(&self) {
        while let Some(r) = self.props.row_at_index(0) {
            self.props.remove(&r);
        }
        let (Some(o), Some(d)) = (self.obj(), self.set()) else { return };
        let mut rows = vec![];
        for p in d.properties.iter().filter(|x| !x.hidden) {
            let Some(v) = o.get(&p.name) else { continue };
            let row = hbox(10);
            row.set_margin_top(3);
            row.set_margin_bottom(3);
            let n = dim(&p.title);
            n.set_width_chars(14);
            n.set_wrap(true);
            row.append(&n);
            let val = label(&format!("{}{}", v, if p.unit.is_empty() { String::new() } else { format!(" {}", p.unit) }));
            val.set_hexpand(true);
            val.set_wrap(true);
            val.set_selectable(true);
            row.append(&val);
            self.props.append(&row);
            rows.push(p.clone());
        }
        *self.prop_rows.borrow_mut() = rows;
    }

    fn insert_prop(&self, lookup: bool) {
        let (Some(o), Some(d)) = (self.obj(), self.set()) else { return };
        let Some(p) = self.props.selected_row().and_then(|r| self.prop_rows.borrow().get(r.index() as usize).cloned()) else { return };
        let key = d.properties.iter().filter(|x| x.is_key).find_map(|x| o.get(&x.name)).unwrap_or("").to_string();
        let app = self.app();
        if lookup || p.is_text {
            let k = if key.contains(' ') { format!("\"{}\"", key) } else { key };
            app.insert_into_expression(&format!("{}({}, {})", d.item.name, k, p.name), false);
            return;
        }
        app.insert_into_expression(&format!("{}{}", o.get(&p.name).unwrap_or(""), if p.unit.is_empty() { String::new() } else { format!(" {}", p.unit) }), false);
    }

    // properties and objects
    fn edit_set(self: &Rc<Self>, d: Option<Arc<DataSet>>) {
        let app = self.app();
        let dlg = Dialog::new(app.parent(), if d.is_none() { "New Data Set" } else { "Edit Data Set" }, "OK");
        dlg.win.set_default_size(760, 620);
        let mut f = Form::new();
        let name = entry(&d.as_ref().map(|d| d.item.name.clone()).unwrap_or_default());
        let title = entry(&d.as_ref().map(|d| d.item.title.clone()).unwrap_or_default());
        let desc = entry(&d.as_ref().map(|d| d.item.description.clone()).unwrap_or_default());
        f.add("Name:", &name);
        f.add("Title:", &title);
        f.add("Description:", &desc);
        dlg.content.append(&f.grid);
        dlg.content.append(&heading("Properties (one per line: name; title; unit; text 0/1; key 0/1)"));
        let pview = gtk::TextView::new();
        pview.add_css_class("oc-mono");
        let props_text = match &d {
            Some(d) => d.properties.iter().map(|p| format!("{}; {}; {}; {}; {}", p.name, p.title, p.unit, p.is_text as i32, p.is_key as i32)).collect::<Vec<_>>().join("\n"),
            None => "name; Name; ; 1; 1".into(),
        };
        pview.buffer().set_text(&props_text);
        let ps = scrolled(&pview);
        ps.set_min_content_height(130);
        dlg.content.append(&ps);
        dlg.content.append(&heading("Objects (one per line, values in property order separated by ;)"));
        let oview = gtk::TextView::new();
        oview.add_css_class("oc-mono");
        if let Some(d) = &d {
            let t = d.objects.iter().map(|o| d.properties.iter().map(|p| o.get(&p.name).unwrap_or("").to_string()).collect::<Vec<_>>().join("; ")).collect::<Vec<_>>().join("\n");
            oview.buffer().set_text(&t);
        }
        let os = scrolled(&oview);
        os.set_min_content_height(200);
        dlg.content.append(&os);
        let err = label("");
        err.add_css_class("error");
        dlg.content.append(&err);
        let w = Rc::downgrade(self);
        dlg.on_ok(move || {
            let n = name.text().trim().to_string();
            if n.is_empty() || !n.chars().all(is_ident_part) {
                err.set_text("Invalid name.");
                return false;
            }
            let text = |v: &gtk::TextView| {
                let b = v.buffer();
                b.text(&b.start_iter(), &b.end_iter(), false).to_string()
            };
            let mut props: Vec<DataProperty> = text(&pview)
                .lines()
                .filter_map(|l| {
                    let p: Vec<String> = l.split(';').map(|s| s.trim().to_string()).collect();
                    let pn = p.first().cloned().unwrap_or_default();
                    if pn.is_empty() {
                        return None;
                    }
                    let get = |i: usize| p.get(i).cloned().unwrap_or_default();
                    let mut dp = DataProperty::new(&pn, &if get(1).is_empty() { pn.clone() } else { get(1) }, &get(2), get(3) == "1");
                    dp.is_key = get(4) == "1";
                    Some(dp)
                })
                .collect();
            if !props.iter().any(|p| p.is_key) {
                if let Some(p) = props.first_mut() {
                    p.is_key = true;
                }
            }
            let objects: Vec<DataObject> = text(&oview)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| {
                    let mut o = DataObject::default();
                    for (p, v) in props.iter().zip(l.split(';')) {
                        let v = v.trim();
                        if !v.is_empty() {
                            o.set(&p.name, v.to_string());
                        }
                    }
                    o
                })
                .filter(|o| !o.values.is_empty())
                .collect();
            let t = title.text().trim().to_string();
            let ds = DataSet {
                item: Item { name: n.clone(), names: vec![n.clone()], title: if t.is_empty() { n.clone() } else { t }, description: desc.text().to_string(), is_user: true, category: "User data sets".into(), ..Default::default() },
                properties: props,
                objects,
                ..Default::default()
            };
            let Some(p) = w.upgrade() else { return true };
            let app = p.app();
            let old = d.as_ref().map(|d| d.item.name.clone());
            let _ = app.svc.run_sync(move |c| {
                if let Some(o) = old {
                    c.defs.borrow_mut().data_sets.retain(|x| x.item.name != o);
                }
                c.add_data_set(ds);
            });
            app.definitions_changed();
            true
        });
        dlg.present();
    }
}

fn header_titles(cols: &[DataProperty]) -> Vec<String> {
    cols.iter().map(|p| p.title.clone()).collect()
}