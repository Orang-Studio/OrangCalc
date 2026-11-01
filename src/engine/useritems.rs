// user defined functions, variables, units and data sets, and their xml persistence
use super::*;
use crate::xml::Node;
use regex::Regex;
use std::sync::LazyLock;

pub fn make_function(c: &Calculator, name: &str, formula: &str, arg_names: &[String]) -> MathFunction {
    static ARG_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\([a-z])").unwrap());
    let mut count = 0usize;
    for m in ARG_RX.captures_iter(formula) {
        let ch = m[1].chars().next().unwrap();
        let idx = if ch >= 'x' { ch as usize - 'x' as usize } else { ch as usize - 'a' as usize + 3 };
        count = count.max(idx + 1);
    }
    let mut f = MathFunction {
        item: Item { name: name.into(), names: vec![name.into()], is_user: true, category: "User functions".into(), title: name.into(), ..Default::default() },
        formula: formula.into(),
        ..Default::default()
    };
    for i in 0..count {
        let an = arg_names.get(i).cloned().unwrap_or_else(|| if i < 3 { ((b'x' + i as u8) as char).to_string() } else { ((b'a' + i as u8 - 3) as char).to_string() });
        f.args.push(ArgDef::new(&an, ArgType::Any, None));
    }
    f.min_args = count as i32;
    f.max_args = count as i32;
    let parsed = parser::parse_req(formula, c);
    let _ = f.parsed.set(parsed);
    f
}

pub fn make_variable(c: &Calculator, name: &str, value_text: &str, title: &str, category: &str) -> Variable {
    let value = c.run(&parser::parse_req(value_text, c), &[]);
    Variable {
        item: Item { name: name.into(), names: vec![name.into()], is_user: true, category: category.into(), title: if title.is_empty() { name.into() } else { title.into() }, ..Default::default() },
        value_text: value_text.into(),
        ..Default::default()
    }
    .with_value(value)
}

#[allow(clippy::too_many_arguments)]
pub fn make_unit(c: &Calculator, name: &str, names: &[String], plural: &str, definition: &str, title: &str, category: &str, prefixes: PrefixUse) -> Unit {
    let mut all = vec![name.to_string()];
    for n in names {
        if !n.is_empty() && !all.contains(n) {
            all.push(n.clone());
        }
    }
    let mut u = Unit {
        item: Item {
            name: name.into(),
            names: all,
            title: if title.is_empty() { name.into() } else { title.into() },
            category: if category.is_empty() { "User units".into() } else { category.into() },
            is_user: true,
            ..Default::default()
        },
        plural: if plural.is_empty() { name.into() } else { plural.into() },
        definition: definition.into(),
        prefixes,
        system: "User".into(),
        ..Default::default()
    };
    units::resolve(c, &mut u);
    u
}
pub fn prefix_use_name(p: PrefixUse) -> String {
    format!("{:?}", p)
}
pub fn parse_prefix_use(s: &str) -> PrefixUse {
    match s {
        "Some" => PrefixUse::Some,
        "All" => PrefixUse::All,
        "Binary" => PrefixUse::Binary,
        _ => PrefixUse::None,
    }
}
fn b(v: bool) -> &'static str {
    if v {
        "1"
    } else {
        "0"
    }
}

pub fn save(c: &Calculator, root: &mut Node) {
    let d = c.defs.borrow();
    for f in d.functions.iter().filter(|x| x.item.is_user) {
        root.push(
            Node::new("function")
                .with("name", f.name())
                .with("names", f.item.names.join(","))
                .with("title", &f.item.title)
                .with("category", &f.item.category)
                .with("description", &f.item.description)
                .with("formula", &f.formula)
                .with("args", f.args.iter().map(|a| a.name.clone()).collect::<Vec<_>>().join(","))
                .with("condition", &f.condition)
                .with("active", b(f.item.active())),
        );
    }
    let o = c.opts().clone();
    for v in d.variables.iter().filter(|x| x.item.is_user) {
        let value = if v.is_unknown {
            String::new()
        } else if v.value_text.is_empty() {
            v.value.get().map(|x| formatter::ascii(x, &o)).unwrap_or_default()
        } else {
            v.value_text.clone()
        };
        root.push(
            Node::new("variable")
                .with("name", v.name())
                .with("names", v.item.names.join(","))
                .with("title", &v.item.title)
                .with("category", &v.item.category)
                .with("description", &v.item.description)
                .with("value", value)
                .with("unknown", b(v.is_unknown))
                .with("type", v.atype.name())
                .with("sign", v.asign.name())
                .with("active", b(v.item.active())),
        );
    }
    for u in d.units.iter().filter(|x| x.item.is_user) {
        root.push(
            Node::new("unit")
                .with("name", u.name())
                .with("names", u.item.names.join(","))
                .with("plural", &u.plural)
                .with("title", &u.item.title)
                .with("category", &u.item.category)
                .with("definition", &u.definition)
                .with("prefixes", prefix_use_name(u.prefixes))
                .with("active", b(u.item.active())),
        );
    }
    for ds in d.data_sets.iter().filter(|x| x.item.is_user) {
        let mut e = Node::new("dataset").with("name", &ds.item.name).with("title", &ds.item.title).with("description", &ds.item.description);
        for p in &ds.properties {
            e.push(Node::new("property").with("name", &p.name).with("title", &p.title).with("unit", &p.unit).with("text", b(p.is_text)).with("key", b(p.is_key)).with("approximate", b(p.approximate)));
        }
        for ob in &ds.objects {
            let mut oe = Node::new("object");
            for (k, v) in &ob.values {
                oe.push(Node::new("v").with("k", k).with("v", v));
            }
            e.push(oe);
        }
        root.push(e);
    }
}
fn split_names(s: &str) -> Vec<String> {
    s.split(',').filter(|n| !n.is_empty()).map(|n| n.to_string()).collect()
}
pub fn load(c: &Calculator, root: &Node) -> Vec<String> {
    let mut errors = vec![];
    for e in root.all("unit") {
        let r = attempt(|| {
            let mut u = make_unit(c, e.attr("name"), &split_names(e.attr("names")), e.attr("plural"), e.attr("definition"), e.attr("title"), e.attr("category"), parse_prefix_use(e.attr("prefixes")));
            u.item.inactive = e.attr("active") == "0";
            c.add_unit(u);
        });
        if let Err(m) = r {
            errors.push(format!("{}: {}", e.attr("name"), m));
        }
    }
    for e in root.all("variable") {
        let r = attempt(|| {
            let name = e.attr("name");
            let mut v = if e.attr("unknown") == "1" {
                Variable {
                    item: Item { name: name.into(), names: vec![name.into()], is_user: true, category: e.attr("category").into(), title: e.attr("title").into(), ..Default::default() },
                    is_unknown: true,
                    atype: AssumeType::parse(e.attr("type")).unwrap_or(AssumeType::Real),
                    asign: AssumeSign::parse(e.attr("sign")).unwrap_or(AssumeSign::Unknown),
                    ..Default::default()
                }
            } else {
                make_variable(c, name, e.attr("value"), e.attr("title"), e.attr("category"))
            };
            let names = split_names(e.attr("names"));
            if !names.is_empty() {
                v.item.names = names;
            }
            v.item.description = e.attr("description").into();
            v.item.inactive = e.attr("active") == "0";
            c.add_variable(v);
        });
        if let Err(m) = r {
            errors.push(format!("{}: {}", e.attr("name"), m));
        }
    }
    for e in root.all("function") {
        let r = attempt(|| {
            let args = split_names(e.attr("args"));
            let mut f = make_function(c, e.attr("name"), e.attr("formula"), &args);
            let names = split_names(e.attr("names"));
            if !names.is_empty() {
                f.item.names = names;
            }
            f.item.title = e.attr("title").into();
            f.item.category = e.attr("category").into();
            f.item.description = e.attr("description").into();
            f.condition = e.attr("condition").into();
            f.item.inactive = e.attr("active") == "0";
            c.add_function(f);
        });
        if let Err(m) = r {
            errors.push(format!("{}: {}", e.attr("name"), m));
        }
    }
    for e in root.all("dataset") {
        let name = e.attr("name").to_string();
        let mut ds = DataSet {
            item: Item { name: name.clone(), names: vec![name], title: e.attr("title").into(), description: e.attr("description").into(), is_user: true, category: "User data sets".into(), ..Default::default() },
            ..Default::default()
        };
        for pe in e.all("property") {
            let mut p = DataProperty::new(pe.attr("name"), pe.attr("title"), pe.attr("unit"), pe.attr("text") == "1");
            p.is_key = pe.attr("key") == "1";
            p.approximate = pe.attr("approximate") != "0";
            ds.properties.push(p);
        }
        for oe in e.all("object") {
            let mut ob = DataObject::default();
            for ve in oe.all("v") {
                ob.set(ve.attr("k"), ve.attr("v").into());
            }
            ds.objects.push(ob);
        }
        c.add_data_set(ds);
    }
    errors
}
pub fn clear_user(c: &Calculator) {
    let (fs, vs, us) = {
        let d = c.defs.borrow();
        (
            d.functions.iter().filter(|x| x.item.is_user).cloned().collect::<Vec<_>>(),
            d.variables.iter().filter(|x| x.item.is_user).cloned().collect::<Vec<_>>(),
            d.units.iter().filter(|x| x.item.is_user).cloned().collect::<Vec<_>>(),
        )
    };
    for f in fs {
        c.remove_function(&f);
    }
    for v in vs {
        c.remove_variable(&v);
    }
    for u in us {
        c.remove_unit(&u);
    }
    c.defs.borrow_mut().data_sets.retain(|d| !d.item.is_user);
}