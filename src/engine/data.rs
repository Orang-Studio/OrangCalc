use super::algebra as simp;
use super::datatables::{ELEMENTS, PLANETS};
use super::*;

pub const ELEMENT_CLASSES: [&str; 12] = ["", "Alkali Metal", "Alkaline Earth Metal", "Lanthanide", "Actinide", "Transition Metal", "Post-transition Metal", "Metalloid", "Reactive Nonmetal", "Halogen", "Noble Gas", "Unknown Properties"];

fn prop(name: &str, title: &str, unit: &str, text: bool) -> DataProperty {
    DataProperty::new(name, title, unit, text)
}

pub fn register(c: &Calculator) {
    let mut ds = DataSet {
        item: Item { name: "atom".into(), names: vec!["atom".into(), "element".into()], title: "Periodic Table".into(), category: "Chemistry".into(), description: "Chemical elements with their physical and chemical properties.".into(), ..Default::default() },
        ..Default::default()
    };
    let exact = |mut p: DataProperty| {
        p.approximate = false;
        p
    };
    let key = |mut p: DataProperty| {
        p.is_key = true;
        p
    };
    let hidden = |mut p: DataProperty| {
        p.hidden = true;
        p.approximate = false;
        p
    };
    ds.properties = vec![
        key(exact(prop("number", "Atomic Number", "", false))),
        key(prop("symbol", "Symbol", "", true)),
        key(prop("name", "Name", "", true)),
        prop("class", "Classification", "", true),
        exact(prop("group", "Group", "", false)),
        exact(prop("period", "Period", "", false)),
        {
            let mut p = prop("weight", "Atomic Weight", "", false);
            p.description = "Standard atomic weight, or mass number of the most stable isotope".into();
            p
        },
        prop("electronegativity", "Electronegativity (Pauling)", "", false),
        prop("density", "Density", "g/cm^3", false),
        prop("melting", "Melting Point", "K", false),
        prop("boiling", "Boiling Point", "K", false),
        prop("configuration", "Electron Configuration", "", true),
        prop("discovered", "Discovered", "", true),
        prop("phase", "Phase at STP", "", true),
        hidden(prop("x_pos", "Column", "", false)),
        hidden(prop("y_pos", "Row", "", false)),
    ];
    for line in ELEMENTS {
        let p: Vec<&str> = line.split('|').collect();
        let mut o = DataObject::default();
        o.set("number", p[0].into());
        o.set("symbol", p[1].into());
        o.set("name", p[2].into());
        let cls: usize = p[3].parse().unwrap();
        o.set("class", ELEMENT_CLASSES[cls].into());
        o.set("class_id", p[3].into());
        let z: i32 = p[0].parse().unwrap();
        let group: i32 = p[4].parse().unwrap();
        let period: i32 = p[5].parse().unwrap();
        if group > 0 {
            o.set("group", p[4].into());
        }
        o.set("period", p[5].into());
        for (i, k) in [(6, "weight"), (7, "electronegativity"), (8, "density"), (9, "melting"), (10, "boiling")] {
            if !p[i].is_empty() {
                o.set(k, p[i].into());
            }
        }
        o.set("configuration", p[11].into());
        o.set("discovered", p[12].into());
        if !p[13].is_empty() {
            o.set("phase", p[13].into());
        }
        let (x, y) = if (57..=71).contains(&z) {
            (z - 57 + 3, 9)
        } else if (89..=103).contains(&z) {
            (z - 89 + 3, 10)
        } else {
            (group, period)
        };
        o.set("x_pos", x.to_string());
        o.set("y_pos", y.to_string());
        ds.objects.push(o);
    }
    c.add_data_set(ds);

    let mut ps = DataSet {
        item: Item { name: "planet".into(), names: vec!["planet".into()], title: "Planets".into(), category: "Astronomy".into(), description: "Planets of the solar system (and Pluto).".into(), ..Default::default() },
        ..Default::default()
    };
    ps.properties = vec![
        key(prop("name", "Name", "", true)),
        prop("mass", "Mass", "kg", false),
        prop("radius", "Equatorial Radius", "km", false),
        prop("density", "Mean Density", "g/cm^3", false),
        prop("gravity", "Surface Gravity", "m/s^2", false),
        prop("escape_velocity", "Escape Velocity", "km/s", false),
        prop("rotation", "Sidereal Rotation Period", "h", false),
        prop("orbit", "Orbital Period", "d", false),
        prop("distance", "Semi-major Axis", "au", false),
        prop("eccentricity", "Orbital Eccentricity", "", false),
        prop("inclination", "Orbital Inclination", "°", false),
        prop("tilt", "Axial Tilt", "°", false),
        prop("temperature", "Mean Temperature", "K", false),
        exact(prop("moons", "Known Moons", "", false)),
    ];
    const KEYS: [&str; 14] = ["name", "mass", "radius", "density", "gravity", "escape_velocity", "rotation", "orbit", "distance", "eccentricity", "inclination", "tilt", "temperature", "moons"];
    for line in PLANETS {
        let mut o = DataObject::default();
        for (k, v) in KEYS.iter().zip(line.split('|')) {
            o.set(k, v.into());
        }
        ps.objects.push(o);
    }
    c.add_data_set(ps);
}

pub fn object_name(e: &Expr) -> String {
    match &e.e {
        E::Str(s) => s.clone(),
        E::Sym(s) => s.clone(),
        E::Var(v) => v.item.names[0].clone(),
        E::Unit(u, p) => format!("{}{}", p.map(|p| p.symbol).unwrap_or(""), u.abbrev()),
        E::Mul(f) => f.iter().map(object_name).collect(),
        E::Call(f, a) if a.is_empty() => f.item.names[0].clone(),
        _ => formatter::plain_text(e),
    }
}

pub fn value(c: &Calculator, o: &DataObject, p: &DataProperty) -> Expr {
    let v = o.get(&p.name).unwrap_or_else(|| throw_s(format!("No value for {}", p.title.to_lowercase())));
    if p.is_text {
        return str_e(v);
    }
    let Ok(Some(parsed)) = attempt(|| parser::parse_text(v, c)) else { return str_e(v) };
    let mut n = c.with_exact(true, || c.ev.eval(&parsed));
    if p.approximate {
        if let E::Num(nn) = &n.e {
            n = num(nn.as_approx());
        }
    }
    if !p.unit.is_empty() {
        let u = c.ev.eval(&parser::parse_req(&p.unit, c));
        n = simp::mul2(&n, &u);
    }
    n
}

pub fn lookup(c: &Calculator, set: &str, obj: &Expr, prop: &str) -> Expr {
    let ds = c.find_data_set(set).unwrap_or_else(|| throw_s(format!("Unknown data set {}", set)));
    let key = object_name(obj);
    let mut o = ds.find(&key);
    if o.is_none() {
        if let E::Num(n) = &obj.e {
            if n.is_int() {
                let s = n.re.n.to_string();
                o = ds.objects.iter().find(|x| x.get("number") == Some(s.as_str()));
            }
        }
    }
    let o = o.unwrap_or_else(|| throw_s(format!("\"{}\" was not found in {}", key, ds.item.display_title().to_lowercase())));
    if prop.is_empty() {
        let mut parts = vec![];
        for p in ds.properties.iter().filter(|x| !x.hidden) {
            let Some(v) = o.get(&p.name) else { continue };
            let unit = if p.unit.is_empty() { String::new() } else { format!(" {}", p.unit.replace("^3", "³").replace("^2", "²")) };
            parts.push(format!("{}: {}{}", p.title, v, unit));
        }
        return str_e(&parts.join("; "));
    }
    let lp = prop.to_lowercase();
    let dp = ds.prop(prop).or_else(|| ds.properties.iter().find(|x| x.title.to_lowercase().contains(&lp))).unwrap_or_else(|| throw_s(format!("Unknown property \"{}\"", prop)));
    value(c, o, dp)
}