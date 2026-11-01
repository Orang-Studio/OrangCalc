// editors and small dialogs
use super::app::App;
use super::widgets::*;
use crate::engine::parser::{is_ident_part, is_ident_start};
use crate::engine::*;
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::cell::RefCell;
use std::rc::Rc;
fn valid_name(n: &str) -> bool {
    let mut cs = n.chars();
    cs.next().map(is_ident_start).unwrap_or(false) && n.chars().all(is_ident_part)
}
fn error_label() -> gtk::Label {
    let l = label("");
    l.add_css_class("error");
    l.set_wrap(true);
    l.set_visible(false);
    l
}
fn fail(l: &gtk::Label, msg: &str) {
    l.set_text(msg);
    l.set_visible(true);
}
fn mono(e: &gtk::Entry) {
    e.add_css_class("oc-mono");
}
pub fn custom_button(app: &Rc<App>, cur: &[String], done: impl Fn(Option<Vec<String>>) + 'static) {
    let d = Dialog::new(app.parent(), "Button Action", "Save");
    d.win.set_default_width(480);
    let get = |i: usize| cur.get(i).cloned().unwrap_or_default();
    let mut f = Form::new();
    let lbl = entry(&get(0));
    let a1 = entry(&get(1));
    let a2 = entry(&get(2));
    let a3 = entry(&get(3));
    f.add("Label:", &lbl);
    f.add("Click:", &a1);
    f.add("Right-click:", &a2);
    f.add("Middle-click:", &a3);
    d.content.append(&f.grid);
    d.content.append(&dim("Text is inserted into the expression. Use name() to insert a function, or @action for an action\n(for example @calculate, @clear, @plot, @periodic, @copy)."));
    let err = error_label();
    d.content.append(&err);
    let done = Rc::new(done);
    let dn = done.clone();
    let w = d.win.clone();
    let remove = button("Remove", move || {
        dn(None);
        w.close();
    });
    d.buttons.prepend(&remove);
    d.on_ok(move || {
        if lbl.text().trim().is_empty() || a1.text().is_empty() {
            fail(&err, "A label and a click action are required.");
            return false;
        }
        done(Some(vec![lbl.text().trim().to_string(), a1.text().to_string(), a2.text().to_string(), a3.text().to_string()]));
        true
    });
    d.present();
}

pub fn function_edit(app: &Rc<App>, f: Option<FnRef>, initial: &str) {
    let d = Dialog::new(app.parent(), if f.is_none() { "New Function" } else { "Edit Function" }, "OK");
    d.win.set_default_width(560);
    let mut form = Form::new();
    let name = entry(&f.as_ref().map(|f| f.item.names[0].clone()).unwrap_or_default());
    let title = entry(&f.as_ref().map(|f| f.item.title.clone()).unwrap_or_default());
    let category = entry(&f.as_ref().map(|f| f.item.category.clone()).unwrap_or_else(|| "User functions".into()));
    let formula = entry(&f.as_ref().map(|f| f.formula.clone()).unwrap_or_else(|| initial.to_string()));
    mono(&formula);
    let args = entry(&f.as_ref().map(|f| f.args.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(", ")).unwrap_or_else(|| "x".into()));
    let condition = entry(&f.as_ref().map(|f| f.condition.clone()).unwrap_or_default());
    let example = entry(&f.as_ref().map(|f| f.example.clone()).unwrap_or_default());
    let description = entry(&f.as_ref().map(|f| f.item.description.clone()).unwrap_or_default());
    form.add("Name:", &name);
    form.add("Title:", &title);
    form.add("Category:", &category);
    form.add("Expression:", &formula);
    form.add("Argument names:", &args);
    form.add("Condition:", &condition);
    form.add("Example:", &example);
    form.add("Description:", &description);
    d.content.append(&form.grid);
    d.content.append(&dim("Refer to arguments with \\x, \\y, \\z, then \\a, \\b … in the expression, e.g. \\x^2 + \\y."));
    let builtin = f.as_ref().map(|f| !f.item.is_user).unwrap_or(false);
    if builtin {
        formula.set_editable(false);
        name.set_editable(false);
    }
    let err = error_label();
    d.content.append(&err);
    let app = app.clone();
    d.on_ok(move || {
        let n = name.text().trim().to_string();
        if !valid_name(&n) {
            fail(&err, "Invalid name.");
            return false;
        }
        let (t, cat, desc, cond, ex) = (title.text().trim().to_string(), category.text().trim().to_string(), description.text().to_string(), condition.text().to_string(), example.text().to_string());
        let fm = formula.text().trim().to_string();
        let an: Vec<String> = args.text().split(',').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
        let old = f.clone();
        let r = app.svc.run_sync(move |c| {
            attempt(|| {
                if let Some(o) = old.as_ref().filter(|o| !o.item.is_user) {
                    let mut nf = (**o).clone();
                    nf.item.title = t;
                    nf.item.category = cat;
                    nf.item.description = desc;
                    c.add_function(nf);
                    return;
                }
                let mut nf = useritems::make_function(c, &n, &fm, &an);
                nf.item.title = if t.is_empty() { n.clone() } else { t };
                nf.item.category = if cat.is_empty() { "User functions".into() } else { cat };
                nf.item.description = desc;
                nf.condition = cond;
                nf.example = ex;
                if let Some(o) = &old {
                    c.remove_function(o);
                }
                c.add_function(nf);
            })
        });
        match r {
            Ok(Ok(())) => {
                app.definitions_changed();
                true
            }
            Ok(Err(e)) | Err(e) => {
                fail(&err, &e);
                false
            }
        }
    });
    d.present();
}

pub fn variable_edit(app: &Rc<App>, v: Option<VarRef>, initial: &str) {
    let d = Dialog::new(app.parent(), if v.is_none() { "New Variable" } else { "Edit Variable" }, "OK");
    d.win.set_default_width(520);
    let value_text = match &v {
        Some(v) if !v.value_text.is_empty() => v.value_text.clone(),
        Some(v) => {
            let vv = v.value.get().cloned();
            let o = app.svc.opts().clone();
            vv.map(|x| app.svc.run_sync(move |_| formatter::ascii(&x, &o)).unwrap_or_default()).unwrap_or_default()
        }
        None => initial.to_string(),
    };
    let mut form = Form::new();
    let name = entry(&v.as_ref().map(|v| v.item.names[0].clone()).unwrap_or_default());
    let value = entry(&value_text);
    mono(&value);
    let title = entry(&v.as_ref().map(|v| v.item.title.clone()).unwrap_or_default());
    let category = entry(&v.as_ref().map(|v| v.item.category.clone()).unwrap_or_else(|| "User variables".into()));
    let description = entry(&v.as_ref().map(|v| v.item.description.clone()).unwrap_or_default());
    form.add("Name:", &name);
    form.add("Value:", &value);
    form.add("Title:", &title);
    form.add("Category:", &category);
    form.add("Description:", &description);
    d.content.append(&form.grid);
    if v.as_ref().map(|v| !v.item.is_user).unwrap_or(false) {
        name.set_editable(false);
        value.set_editable(false);
    }
    let err = error_label();
    d.content.append(&err);
    let app = app.clone();
    d.on_ok(move || {
        let n = name.text().trim().to_string();
        if !valid_name(&n) {
            fail(&err, "Invalid name.");
            return false;
        }
        let (val, t, cat, desc) = (value.text().trim().to_string(), title.text().trim().to_string(), category.text().trim().to_string(), description.text().to_string());
















        let old = v.clone();
        let r = app.svc.run_sync(move |c| {
            attempt(|| {
                if let Some(o) = old.as_ref().filter(|o| !o.item.is_user) {
                    let mut nv = (**o).clone();
                    nv.item.title = t;
                    nv.item.category = cat;
                    nv.item.description = desc;
                    c.add_variable(nv);
                    return;
                }
                let mut nv = useritems::make_variable(c, &n, &val, &t, if cat.is_empty() { "User variables" } else { &cat });
                if let Some(x) = nv.value.get().cloned() {
                    nv = nv.with_value(funcs::unhold(&x));
                }
                nv.item.description = desc;
                if let Some(o) = &old {
                    c.remove_variable(o);
                }
                c.add_variable(nv);
            })
        });
        match r {
            Ok(Ok(())) => {
                app.definitions_changed();
                true
            }
            Ok(Err(e)) | Err(e) => {
                fail(&err, &e);
                false
            }
        }
    });
    d.present();
}

pub fn unknown_edit(app: &Rc<App>, v: Option<VarRef>) {
    let d = Dialog::new(app.parent(), if v.is_none() { "New Unknown Variable" } else { "Edit Unknown Variable" }, "OK");
    d.win.set_default_width(460);
    let mut form = Form::new();
    let name = entry(&v.as_ref().map(|v| v.name().to_string()).unwrap_or_default());
    let types: Vec<&str> = AssumeType::ALL.iter().map(|t| t.name()).collect();
    let ty = combo(&types, v.as_ref().map(|v| v.atype).unwrap_or(AssumeType::Real).index());
    let signs: Vec<String> = AssumeSign::ALL.iter().map(|s| words(s.name())).collect();
    let sign = combo_strings(&signs, v.as_ref().map(|v| v.asign).unwrap_or(AssumeSign::Unknown).index());
    let title = entry(&v.as_ref().map(|v| v.item.title.clone()).unwrap_or_default());
    form.add("Name:", &name);
    form.add("Type:", &ty);
    form.add("Sign:", &sign);
    form.add("Title:", &title);
    d.content.append(&form.grid);
    d.content.append(&dim("Unknown variables are kept symbolic. Assumptions are used when simplifying, e.g. sqrt(x^2) = x for positive x."));
    let err = error_label();
    d.content.append(&err);
    let app = app.clone();
    d.on_ok(move || {
        let n = name.text().trim().to_string();
        if !valid_name(&n) {
            fail(&err, "Invalid name.");
            return false;
        }
        let (t, at, asg) = (title.text().trim().to_string(), AssumeType::from_index(ty.selected() as usize), AssumeSign::from_index(sign.selected() as usize));
        let old = v.clone();
        let _ = app.svc.run_sync(move |c| {
            let mut nv = match old.as_ref().filter(|o| o.name() == n) {
                Some(o) => (**o).clone(),
                None => Variable { item: Item { name: n.clone(), names: vec![n.clone()], category: "Unknowns".into(), ..Default::default() }, is_unknown: true, ..Default::default() },
            };
            nv.item.is_user = true;
            nv.atype = at;
            nv.asign = asg;
            nv.item.title = if t.is_empty() { n.clone() } else { t };
            if let Some(o) = &old {
                c.remove_variable(o);
            }
            c.add_variable(nv);
        });
        app.definitions_changed();
        true
    });
    d.present();
}

pub fn unit_edit(app: &Rc<App>, u: Option<UnitRef>) {
    let d = Dialog::new(app.parent(), if u.is_none() { "New Unit" } else { "Edit Unit" }, "OK");
    d.win.set_default_width(540);
    let mut form = Form::new();
    let name = entry(&u.as_ref().map(|u| u.name().to_string()).unwrap_or_default());
    let names = entry(&u.as_ref().map(|u| u.item.names.iter().skip(1).cloned().collect::<Vec<_>>().join(", ")).unwrap_or_default());
    let plural = entry(&u.as_ref().map(|u| u.plural.clone()).unwrap_or_default());
    let def = entry(&u.as_ref().map(|u| u.definition.clone()).unwrap_or_else(|| "1 m".into()));
    mono(&def);
    let title = entry(&u.as_ref().map(|u| u.item.title.clone()).unwrap_or_default());
    let category = entry(&u.as_ref().map(|u| u.item.category.clone()).unwrap_or_else(|| "User units".into()));
    let pref = combo(&["None", "Some", "All", "Binary"], u.as_ref().map(|u| u.prefixes as usize).unwrap_or(0));
    form.add("Abbreviation:", &name);
    form.add("Other names:", &names);
    form.add("Plural:", &plural);
    form.add("Equals:", &def);
    form.add("Title:", &title);
    form.add("Category:", &category);
    form.add("Prefixes:", &pref);
    d.content.append(&form.grid);
    d.content.append(&dim("Define the unit by its value in other units, for example \"0.3048 m\" or \"1 kg*m/s^2\"."));
    if u.as_ref().map(|u| !u.item.is_user).unwrap_or(false) {
        name.set_editable(false);
        def.set_editable(false);
    }
    let err = error_label();
    d.content.append(&err);
    let app = app.clone();
    d.on_ok(move || {
        let n = name.text().trim().to_string();
        if n.is_empty() {
            fail(&err, "A name is required.");
            return false;
        }
        let others: Vec<String> = names.text().split(',').map(|x| x.trim().to_string()).collect();
        let (pl, df, t, cat) = (plural.text().trim().to_string(), def.text().trim().to_string(), title.text().trim().to_string(), category.text().trim().to_string());
        let pu = [PrefixUse::None, PrefixUse::Some, PrefixUse::All, PrefixUse::Binary][pref.selected() as usize % 4];
        let old = u.clone();
        let r = app.svc.run_sync(move |c| {
            attempt(|| {
                if let Some(o) = old.as_ref().filter(|o| !o.item.is_user) {
                    let mut nu = (**o).clone();
                    nu.item.title = t;
                    nu.item.category = cat;
                    c.add_unit(nu);
                    return;
                }
                let nu = useritems::make_unit(c, &n, &others, &pl, &df, &t, &cat, pu);
                if let Some(o) = &old {
                    c.remove_unit(o);
                }
                c.add_unit(nu);
            })
        });
        match r {
            Ok(Ok(())) => {
                app.definitions_changed();
                true
            }
            Ok(Err(e)) | Err(e) => {
                fail(&err, &e);
                false
            }
        }
    });
    d.present();
}

pub fn matrix_edit(app: &Rc<App>, initial: Option<Expr>, done: impl Fn(String) + 'static) {
    let d = Dialog::new(app.parent(), "Matrix", "Insert");
    d.win.set_default_size(620, 420);
    let cells: Vec<Vec<String>> = match &initial {
        Some(m) => {
            let o = app.svc.opts().clone();
            let m = m.clone();
            app.svc.run_sync(move |_| matrices::rows_of(&m).iter().map(|r| r.iter().map(|x| formatter::ascii(x, &o)).collect()).collect()).unwrap_or_default()
        }
        None => vec![],
    };
    let r0 = if cells.is_empty() { 3 } else { cells.len() };
    let c0 = if cells.is_empty() { 3 } else { cells[0].len() };
    let top = hbox(8);
    let rows = gtk::SpinButton::with_range(1.0, 100.0, 1.0);
    rows.set_value(r0 as f64);
    let cols = gtk::SpinButton::with_range(1.0, 100.0, 1.0);
    cols.set_value(c0 as f64);
    let vector = gtk::CheckButton::with_label("Vector");
    vector.set_active(initial.as_ref().map(matrices::is_vector).unwrap_or(false));
    top.append(&label("Rows:"));
    top.append(&rows);
    top.append(&label("Columns:"));
    top.append(&cols);
    top.append(&vector);
    d.content.append(&top);
    let grid = gtk::Grid::new();
    grid.set_row_spacing(4);
    grid.set_column_spacing(4);
    d.content.append(&scrolled(&grid));
    let entries: Rc<RefCell<Vec<Vec<gtk::Entry>>>> = Rc::new(RefCell::new(vec![]));
    let resize = {
        let grid = grid.clone();
        let entries = entries.clone();
        move |r: usize, c: usize| {
            let mut es = entries.borrow_mut();
            while es.len() > r {
                for e in es.pop().unwrap() {
                    grid.remove(&e);
                }
            }
            for row in es.iter_mut() {
                while row.len() > c {
                    grid.remove(&row.pop().unwrap());
                }
            }
            while es.len() < r {
                es.push(vec![]);
            }
            for (i, row) in es.iter_mut().enumerate() {
                while row.len() < c {
                    let e = gtk::Entry::new();
                    e.set_width_chars(8);
                    e.add_css_class("oc-mono");
                    grid.attach(&e, row.len() as i32, i as i32, 1, 1);
                    row.push(e);
                }
            }
        }
    };
    resize(r0, c0);
    for (i, r) in cells.iter().enumerate() {
        for (j, s) in r.iter().enumerate() {
            if let Some(e) = entries.borrow().get(i).and_then(|row| row.get(j)) {
                e.set_text(s);
            }
        }
    }
    let resize = Rc::new(resize);
    for sb in [&rows, &cols] {
        let (rows, cols, resize) = (rows.clone(), cols.clone(), resize.clone());
        sb.connect_value_changed(move |_| resize(rows.value() as usize, cols.value() as usize));
    }
    let rows2 = rows.clone();
    vector.connect_toggled(move |v| {
        if v.is_active() {
            rows2.set_value(1.0);
        }
    });
    // sucks asf
    d.on_ok(move || {
        let es = entries.borrow();
        let rows_text: Vec<String> = es
            .iter()
            .map(|r| {
                r.iter()
                    .map(|e| {
                        let t = e.text().trim().to_string();
                        if t.is_empty() {
                            "0".into()
                        } else {
                            t
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect();
        let text = if vector.is_active() { format!("[{}]", rows_text.join(", ")) } else { format!("[{}]", rows_text.join("; ")) };
        done(text);
        true
    });
    d.present();
}

pub fn insert_function(app: &Rc<App>, f: &FnRef) {
    App::add_recent(&mut app.settings.borrow_mut().recent_functions, &f.item.names[0]);
    app.show_page(0);
    let calc = app.calc();
    let sel = calc.expr.selected_text();
    if f.max_args == 0 {
        calc.insert(&format!("{}()", f.item.names[0]), 0);
        return;
    }
    let d = Dialog::new(app.parent(), f.item.display_title(), "Calculate");
    d.win.set_default_width(520);
    let sig = label(&f.signature());
    sig.add_css_class("oc-mono");
    sig.set_selectable(true);
    d.content.append(&sig);
    if !f.item.description.is_empty() {
        d.content.append(&dim(&f.item.description));
    }
    let n = if f.max_args < 0 { f.args.len().max(f.min_args as usize) } else { f.args.len() };
    let mut form = Form::new();
    let mut boxes = vec![];
    for i in 0..n {
        let a = &f.args[i.min(f.args.len().saturating_sub(1))];
        let mut def = if i == 0 && !sel.is_empty() { sel.clone() } else { a.default.clone().unwrap_or_default() };
        if def == "\"\"" {
            def.clear();
        }
        let e = entry(&def);
        mono(&e);
        e.set_activates_default(true);
        let mut name = a.name.replace('_', " ");
        if let Some(c) = name.get(..1) {
            name = c.to_uppercase() + &name[1..];
        }
        form.add(&format!("{}{}:", name, if i as i32 >= f.min_args { " (optional)" } else { "" }), &e);
        boxes.push(e);
    }
    d.content.append(&form.grid);
    if !f.example.is_empty() {
        d.content.append(&dim(&format!("Example: {}", f.example)));
    }
    let comma = app.svc.opts().comma_decimal;
    let fname = f.item.names[0].clone();
    let min = f.min_args as usize;
    let build = Rc::new(move |boxes: &[gtk::Entry]| {
        let mut vals: Vec<String> = boxes.iter().map(|b| b.text().trim().to_string()).collect();
        while vals.len() > min && vals.last().map(|v| v.is_empty()).unwrap_or(false) {
            vals.pop();
        }
        format!("{}({})", fname, vals.join(if comma { "; " } else { ", " }))
    });
    let first_box = boxes.first().cloned();
    let boxes = Rc::new(boxes);
    let (b2, bd, w, sel_empty) = (boxes.clone(), build.clone(), d.win.clone(), sel.is_empty());
    let calc2 = calc.clone();
    let insert_btn = button("Insert", move || {
        if !sel_empty {
            calc2.expr.buffer.delete_selection(true, true);
        }
        calc2.insert(&bd(&b2), 0);
        w.close();
    });
    d.buttons.insert_child_after(&insert_btn, d.buttons.first_child().as_ref());
    d.on_ok(move || {
        if !sel.is_empty() {
            calc.expr.buffer.delete_selection(true, true);
        }
        calc.insert(&build(&boxes), 0);
        calc.calculate();
        true
    });
    d.present();
    if let Some(b) = first_box {
        b.grab_focus();
    }
}

pub fn date_dialog(parent: Option<&gtk::Window>, title: &str, init: chrono::NaiveDateTime, with_time: bool, done: impl Fn(chrono::NaiveDateTime) + 'static) {
    use chrono::{Datelike, Timelike};
    let d = Dialog::new(parent, title, "OK");
    let cal = gtk::Calendar::new();
    if let Ok(dt) = glib::DateTime::from_local(init.year(), init.month() as i32, init.day() as i32, 0, 0, 0.0) {
        cal.select_day(&dt);
    }
    d.content.append(&cal);
    let time = entry(&if init.hour() == 0 && init.minute() == 0 && init.second() == 0 { String::new() } else { init.format("%H:%M:%S").to_string() });
    time.set_placeholder_text(Some("HH:MM:SS (optional)"));
    if with_time {
        let mut f = Form::new();
        f.add("Time:", &time);
        d.content.append(&f.grid);
    }
    d.on_ok(move || {
        let dt = cal.date();
        let mut t = chrono::NaiveDate::from_ymd_opt(dt.year(), dt.month() as u32, dt.day_of_month() as u32).unwrap_or_default().and_hms_opt(0, 0, 0).unwrap();
        let ts = time.text().trim().to_string();
        if with_time && !ts.is_empty() {
            let parsed = chrono::NaiveTime::parse_from_str(&ts, "%H:%M:%S").or_else(|_| chrono::NaiveTime::parse_from_str(&ts, "%H:%M"));
            if let Ok(tm) = parsed {
                t = t.date().and_time(tm);
            }
        }
        done(t);
        true
    });
    d.present();
}

pub fn shortcut_dialog(parent: Option<&gtk::Window>, action: &str, current: &str, done: impl Fn(String) + 'static) {
    let d = Dialog::new(parent, "Keyboard Shortcut", "Set");
    d.win.set_default_width(400);
    // some day some how the translations will be there.
    d.content.append(&dim(&format!("Press the key combination you wish to use for:\n{}", action)));
    let keys = gtk::Label::new(Some(if current.is_empty() { "(none)" } else { current }));
    keys.add_css_class("title-2");
    keys.set_margin_top(10);
    keys.set_margin_bottom(10);
    d.content.append(&keys);
    let value = Rc::new(RefCell::new(current.to_string()));
    let key = gtk::EventControllerKey::new();
    key.set_propagation_phase(gtk::PropagationPhase::Capture);
    let (v, k2) = (value.clone(), keys.clone());
    key.connect_key_pressed(move |_, k, _, m| {
        if matches!(k, gdk::Key::Control_L | gdk::Key::Control_R | gdk::Key::Shift_L | gdk::Key::Shift_R | gdk::Key::Alt_L | gdk::Key::Alt_R | gdk::Key::Super_L | gdk::Key::Super_R | gdk::Key::Escape) {
            return glib::Propagation::Proceed;
        }
        let m = m & (gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK | gdk::ModifierType::ALT_MASK);
        let accel = gtk::accelerator_name(k.to_lower(), m);
        let s = super::app::from_accel(&accel);
        k2.set_text(&s);
        *v.borrow_mut() = s;
        glib::Propagation::Stop
    });
    d.win.add_controller(key);
    let (v, k2) = (value.clone(), keys.clone());
    let clear = button("Clear", move || {
        v.borrow_mut().clear();
        k2.set_text("(none)");
    });
    clear.set_focusable(false);
    d.ok.set_focusable(false);
    d.buttons.prepend(&clear);
    d.on_ok(move || {
        done(value.borrow().clone());
        true
    });
    d.present();
}

pub fn about(app: &Rc<App>) {
    let a = gtk::AboutDialog::builder()
        .program_name("OrangCalc")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("A dark, precise calculator with arithmetic, symbolic algebra, units and currencies, plotting, data sets and a periodic table.")
        .license_type(gtk::License::Gpl30)
        .copyright("© Orang Studio 2026")
        .website("https://github.com/Orang-Studio/OrangCalc")
        .modal(true)
        .transient_for(&app.window)
        .build();
    if let Some(icon) = super::theme::app_icon() {
        a.set_logo(Some(&icon));
    }
    a.present();
}

const HELP: &[(&str, &str)] = &[
    ("Basics", "Type an expression and press Enter. Shift+Enter inserts a new line. Up and Down browse previous expressions. Ctrl+Space shows completion.\nOperators: + − × ÷ ^, implicit multiplication (2x, 5 m), ! factorial, % percent, mod, rem, // integer division.\nComparisons and logic: = ≠ < > ≤ ≥, and, or, xor, not, && ||. Bitwise: & | ~ << >> xor."),
    ("Numbers", "Decimal 1.5, scientific 1.5E3, hexadecimal 0xFF, binary 0b1010, octal 0o17, sexagesimal 5°30′15″, time 10:30, dates 2024-01-31, fractions ½, uncertainty 5±0.1 or interval(4.9, 5.1), complex 3+4i or 5∠30°."),
    ("Conversion", "Append \"to\" (or →) with a target: units (5 ft to m), bases (to hex, to bin, to oct, to base 7, to roman, to bijective, to bcd), formats (to fraction, to mixed, to scientific, to engineering, to factors, to partial fraction, to polar, to cis, to angle), time zones (to utc, to utc+3), to calendars, to bases, to fp32 / fp64."),
    ("Variables and functions", "Assign with name := value. Define functions with f(x) := x^2 + 1. Unknowns (x, y, z) are kept symbolic. ans is the last result. Substitute with \"where\": x^2 + y where x = 2, y = 3."),
    ("Algebra and calculus", "Equations with one unknown are solved automatically: x^2 = 4. Functions: solve, multisolve, factor, expand, partfrac, diff, integrate, limit, sum, product, taylor, dsolve, extremum."),
    ("Units", "Units and prefixes are written directly: 5 km/h, 3 kWh, 100 °F to °C. Currencies update from the European Central Bank. Physical constants carry units (c, planck, k_B)."),
    ("Modes", "Approximation: automatic, dual (exact and approximate), exact, approximate. Angle unit: radians, degrees, gradians. Parsing: adaptive, conventional, implicit multiplication first, chain mode and RPN mode."),
    ("Keypad", "Each key has up to three actions: click, right-click and middle-click. The small hint on a key shows the right-click action. The Custom keypad can be filled with your own buttons."),
];

pub fn help(app: &Rc<App>) {
    let win = gtk::Window::builder().title("OrangCalc Help").default_width(760).default_height(640).transient_for(&app.window).build();
    let b = vbox(6);
    margins(&b, 18);
    for (head, body) in HELP {
        let h = label(head);
        h.add_css_class("title-3");
        h.set_margin_top(10);
        b.append(&h);
        let t = label(body);
        t.set_wrap(true);
        t.set_selectable(true);
        b.append(&t);
    }
    win.set_child(Some(&scrolled(&b)));
    win.present();
}

fn csv_split(line: &str, delim: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let dc: Vec<char> = delim.chars().collect();
    let mut res = vec![];
    let mut cur = String::new();
    let mut q = false;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            q = !q;
            i += 1;
            continue;
        }
        let hit = if delim == " " { ch.is_whitespace() } else { i + dc.len() <= chars.len() && chars[i..i + dc.len()] == dc[..] };
        if !q && hit {
            res.push(cur.trim().to_string());
            cur.clear();
            i += dc.len().max(1);
            if delim == " " {
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
            }
            continue;
        }
        cur.push(ch);
        i += 1;
    }
    res.push(cur.trim().to_string());
    res
}

pub fn csv_import(app: &Rc<App>) {
    let a = app.clone();
    open_file(app.parent(), "Import CSV File", move |file| {
        let stem = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let suggested: String = stem.chars().filter(|c| is_ident_part(*c)).collect();
        let suggested = if valid_name(&suggested) { suggested } else { "data".into() };
        let d = Dialog::new(a.parent(), "Import CSV File", "Import");
        d.win.set_default_width(460);
        let mut f = Form::new();
        let name = entry(&suggested);
        let as_what = combo(&["Matrix", "Vectors (one per column)"], 0);
        let delim = combo(&["Comma", "Tabulator", "Semicolon", "Space", "Other"], 0);
        let other = entry("");
        let first = gtk::SpinButton::with_range(1.0, 1000000.0, 1.0);
        let head = gtk::CheckButton::with_label("Includes headings");
        f.add("Name:", &name);
        f.add("Import as:", &as_what);
        f.add("Delimiter:", &delim);
        f.add("Other delimiter:", &other);
        f.add("First row:", &first);
        f.full(&head);
        d.content.append(&f.grid);
        let err = error_label();
        d.content.append(&err);
        let a2 = a.clone();
        d.on_ok(move || {
            let n = name.text().trim().to_string();
            if !valid_name(&n) {
                fail(&err, "Invalid name.");
                return false;
            }
            let mut dl = [",", "\t", ";", " ", ""][delim.selected() as usize % 5].to_string();
            if dl.is_empty() {
                dl = other.text().to_string();
            }
            if dl.is_empty() {
                dl = ",".into();
            }
            let text = match std::fs::read_to_string(&file) {
                Ok(t) => t,
                Err(e) => {
                    fail(&err, &format!("Could not import from file\n{}\n{}", file.display(), e));
                    return false;
                }
            };
            let mut lines: Vec<String> = text.lines().skip((first.value() as usize).saturating_sub(1)).filter(|l| !l.trim().is_empty()).map(|l| l.to_string()).collect();
            let mut headings: Option<Vec<String>> = None;
            if head.is_active() && !lines.is_empty() {
                headings = Some(csv_split(&lines.remove(0), &dl));
            }
            let rows: Vec<Vec<String>> = lines.iter().map(|l| csv_split(l, &dl)).collect();
            let as_matrix = as_what.selected() == 0;
            let r = a2.svc.run_sync(move |c| {
                let val = |t: &str| attempt(|| funcs::unhold(&c.eval_text(if t.is_empty() { "0" } else { t }))).unwrap_or_else(|_| str_e(t));
                let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                let o = c.opts().clone();
                let mut names = vec![];
                let mut add = |vname: String, v: Expr, cat: &str| {
                    let text = formatter::ascii(&v, &o);
                    c.add_variable(Variable { item: Item { name: vname.clone(), names: vec![vname.clone()], is_user: true, category: cat.into(), title: vname.clone(), ..Default::default() }, value_text: text, ..Default::default() }.with_value(v));
                    names.push(vname);
                };
                if as_matrix {
                    let m = mat_e(rows.iter().map(|r| (0..cols).map(|j| r.get(j).map(|s| val(s)).unwrap_or_else(zero)).collect()).collect(), rows.len() == 1);
                    add(n.clone(), m, "Matrices");
                } else {
                    for j in 0..cols {
                        let h = headings.as_ref().and_then(|h| h.get(j)).cloned().unwrap_or_default();
                        let vname = if valid_name(&h) { h } else { format!("{}{}", n, j + 1) };
                        let v = vector_e(rows.iter().map(|r| r.get(j).map(|s| val(s)).unwrap_or_else(zero)).collect());
                        add(vname, v, "Vectors");
                    }
                }
                names
            });
            match r {
                Ok(names) => {
                    a2.definitions_changed();
                    message(a2.parent(), &format!("Imported {}.", names.join(", ")));
                    true
                }
                Err(e) => {
                    fail(&err, &e);
                    false
                }
            }
        });
        d.present();
    });
}

pub fn csv_export(app: &Rc<App>) {
    let d = Dialog::new(app.parent(), "Export CSV File", "Export");
    d.win.set_default_width(460);
    let mut f = Form::new();
    let src = combo(&["Current result", "Matrix/vector variable"], 0);
    let name = entry("");
    let delim = combo(&["Comma", "Tabulator", "Semicolon", "Space"], 0);
    f.add("Export:", &src);
    f.add("Variable:", &name);
    f.add("Delimiter:", &delim);
    d.content.append(&f.grid);
    let a = app.clone();
    d.on_ok(move || {
        let dl = [",", "\t", ";", " "][delim.selected() as usize % 4].to_string();
        let value = if src.selected() == 0 {
            a.calc().last.borrow().as_ref().and_then(|d| d.value.clone())
        } else {
            let n = name.text().trim().to_string();
            a.svc.run_sync(move |c| c.find_variable(&n).and_then(|v| attempt(|| c.eval_text(&v.item.names[0])).ok())).ok().flatten()
        };
        let Some(m) = value.filter(|v| v.is_mat()) else {
            message(a.parent(), "No matrix or vector variable with the entered name was found.");
            return true;
        };
        let mut o = a.svc.opts().clone();
        o.unicode = false;
        o.grouping = DigitGrouping::None;
        let text = a
            .svc
            .run_sync(move |_| {
                let rows: Vec<Vec<Expr>> = if matrices::is_vector(&m) { matrices::rows_of(&m)[0].iter().map(|x| vec![x.clone()]).collect() } else { matrices::rows_of(&m).clone() };
                rows.iter().map(|r| r.iter().map(|x| formatter::format(x, &o).0).collect::<Vec<_>>().join(&dl) + "\n").collect::<String>()
            })
            .unwrap_or_default();
        let a2 = a.clone();
        save_file(a.parent(), "Export CSV File", "data.csv", move |p| {
            if let Err(e) = std::fs::write(&p, &text) {
                message(a2.parent(), &format!("Could not export to file\n{}\n{}", p.display(), e));
            }
        });
        true
    });
    d.present();
}
