use super::algebra as simp;
use super::*;
const TABLE: &[[&str; 6]] = &[
    ["m,meter,meters,metre,metres", "Meter", "Length", "#0", "all", "SI"],
    ["g,gram,grams,gramme", "Gram", "Mass", "#1", "all", "SI"],
    ["s,second,seconds,sec,secs", "Second", "Time", "#2", "all", "SI"],
    ["A,ampere,amperes,amp,amps", "Ampere", "Current", "#3", "all", "SI"],
    ["K,kelvin,kelvins", "Kelvin", "Temperature", "#4", "all", "SI"],
    ["mol,mole,moles", "Mole", "Amount of Substance", "#5", "all", "SI"],
    ["cd,candela,candelas", "Candela", "Luminous Intensity", "#6", "all", "SI"],
    ["bit,bits", "Bit", "Information", "#7", "binary", "SI"],
    ["rad,radian,radians", "Radian", "Angle", "#9", "some", "SI"],
    ["in,inch,inches", "Inch", "Length", "0.0254 m", "none", "Imperial"],
    ["ft,foot,feet", "Foot", "Length", "0.3048 m", "none", "Imperial"],
    ["yd,yard,yards", "Yard", "Length", "0.9144 m", "none", "Imperial"],
    ["mi,mile,miles", "Mile", "Length", "1609.344 m", "none", "Imperial"],
    ["nmi,nauticalmile,nauticalmiles,NM", "Nautical Mile", "Length", "1852 m", "none", "Other"],
    ["au,astronomicalunit,AU", "Astronomical Unit", "Length", "149597870700 m", "none", "Astronomical"],
    ["ly,lightyear,lightyears", "Light Year", "Length", "9460730472580800 m", "some", "Astronomical"],
    ["pc,parsec,parsecs", "Parsec", "Length", "648000/pi au", "some", "Astronomical"],
    ["Å,angstrom,angstroms,Ang", "Ångström", "Length", "1e-10 m", "none", "Other"],
    ["micron,microns", "Micron", "Length", "1e-6 m", "none", "Other"],
    ["mil,thou", "Mil", "Length", "0.0000254 m", "none", "Imperial"],
    ["pt,point,points", "Point (typography)", "Length", "1/72 in", "none", "Other"],
    ["pica,picas", "Pica", "Length", "1/6 in", "none", "Other"],
    ["hand,hands", "Hand", "Length", "4 in", "none", "Imperial"],
    ["ftm,fathom,fathoms", "Fathom", "Length", "6 ft", "none", "Imperial"],
    ["ch,chain,chains", "Chain", "Length", "66 ft", "none", "Imperial"],
    ["fur,furlong,furlongs", "Furlong", "Length", "660 ft", "none", "Imperial"],
    ["rd,rod,rods", "Rod", "Length", "16.5 ft", "none", "Imperial"],
    ["lea,league,leagues", "League", "Length", "3 mi", "none", "Imperial"],
    ["cubit,cubits", "Cubit", "Length", "18 in", "none", "Other"],
    ["lP,planck_length", "Planck Length", "Length", "1.616255e-35 m", "none", "Natural"],
    ["ha,hectare,hectares", "Hectare", "Area", "10000 m^2", "none", "SI"],
    ["are,ares", "Are", "Area", "100 m^2", "none", "SI"],
    ["ac,acre,acres", "Acre", "Area", "4046.8564224 m^2", "none", "Imperial"],
    ["barn,barns", "Barn", "Area", "1e-28 m^2", "all", "Other"],
    ["L,l,liter,liters,litre,litres", "Liter", "Volume", "0.001 m^3", "all", "SI"],
    ["gal,gallon,gallons", "Gallon (US)", "Volume", "3.785411784 L", "none", "US"],
    ["galUK,impgal,imperialgallon", "Gallon (imperial)", "Volume", "4.54609 L", "none", "Imperial"],
    ["qt,quart,quarts", "Quart (US)", "Volume", "1/4 gal", "none", "US"],
    ["pint,pints", "Pint (US)", "Volume", "1/8 gal", "none", "US"],
    ["pintUK,imppint", "Pint (imperial)", "Volume", "1/8 galUK", "none", "Imperial"],
    ["cup,cups", "Cup (US)", "Volume", "1/16 gal", "none", "US"],
    ["floz,fluidounce,fluidounces", "Fluid Ounce (US)", "Volume", "1/128 gal", "none", "US"],
    ["tbsp,tablespoon,tablespoons", "Tablespoon", "Volume", "1/2 floz", "none", "US"],
    ["tsp,teaspoon,teaspoons", "Teaspoon", "Volume", "1/6 floz", "none", "US"],
    ["bbl,barrel,barrels", "Barrel (oil)", "Volume", "42 gal", "none", "US"],
    ["cc", "Cubic Centimeter", "Volume", "1e-6 m^3", "none", "Other"],
    ["bu,bushel,bushels", "Bushel (US)", "Volume", "35.23907016688 L", "none", "US"],
    ["t,tonne,tonnes,metricton", "Tonne", "Mass", "1000000 g", "some", "SI"],
    ["lb,lbs,pound,pounds", "Pound", "Mass", "453.59237 g", "none", "Imperial"],
    ["oz,ounce,ounces", "Ounce", "Mass", "1/16 lb", "none", "Imperial"],
    ["st,stone,stones", "Stone", "Mass", "14 lb", "none", "Imperial"],
    ["gr,grain,grains", "Grain", "Mass", "1/7000 lb", "none", "Imperial"],
    ["ct,carat,carats", "Carat", "Mass", "0.2 g", "none", "Other"],
    ["ozt,troyounce", "Troy Ounce", "Mass", "31.1034768 g", "none", "Other"],
    ["ton,tons,shortton", "Short Ton", "Mass", "2000 lb", "none", "US"],
    ["longton,longtons", "Long Ton", "Mass", "2240 lb", "none", "Imperial"],
    ["slug,slugs", "Slug", "Mass", "14593.90294 g", "none", "Imperial"],
    ["u,Da,dalton,daltons,amu", "Atomic Mass Unit", "Mass", "1.66053906660e-24 g", "all", "Other"],
    ["min,minute,minutes", "Minute", "Time", "60 s", "none", "Other"],
    ["h,hr,hour,hours", "Hour", "Time", "60 min", "none", "Other"],
    ["d,day,days", "Day", "Time", "24 h", "none", "Other"],
    ["wk,week,weeks", "Week", "Time", "7 d", "none", "Other"],
    ["fortnight,fortnights", "Fortnight", "Time", "14 d", "none", "Other"],
    ["yr,year,years", "Year", "Time", "365.2425 d", "some", "Other"],
    ["mo,month,months", "Month", "Time", "1/12 yr", "none", "Other"],
    ["decade,decades", "Decade", "Time", "10 yr", "none", "Other"],
    ["century,centuries", "Century", "Time", "100 yr", "none", "Other"],
    ["millennium,millennia", "Millennium", "Time", "1000 yr", "none", "Other"],
    ["tP,planck_time", "Planck Time", "Time", "5.391247e-44 s", "none", "Natural"],
    ["kph,kmh", "Kilometers per Hour", "Speed", "1000 m/h", "none", "Other"],
    ["mph", "Miles per Hour", "Speed", "1 mi/h", "none", "Imperial"],
    ["kn,knot,knots,kt", "Knot", "Speed", "1 nmi/h", "none", "Other"],
    ["fps", "Feet per Second", "Speed", "1 ft/s", "none", "Imperial"],
    ["Gal,galileo", "Gal", "Acceleration", "0.01 m/s^2", "all", "CGS"],
    ["gn,gravity,gee", "Standard Gravity", "Acceleration", "9.80665 m/s^2", "none", "Other"],
    ["N,newton,newtons", "Newton", "Force", "1000 g*m/s^2", "all", "SI"],
    ["dyn,dyne,dynes", "Dyne", "Force", "1e-5 N", "all", "CGS"],
    ["lbf,poundforce", "Pound-force", "Force", "4.4482216152605 N", "none", "Imperial"],
    ["kgf,kilopond,kp", "Kilogram-force", "Force", "9.80665 N", "none", "Other"],
    ["J,joule,joules", "Joule", "Energy", "1 N*m", "all", "SI"],
    ["Wh,watthour,watthours", "Watt Hour", "Energy", "3600 J", "all", "Other"],
    ["cal,calorie,calories", "Calorie", "Energy", "4.184 J", "all", "Other"],
    ["Cal,kcal_food,foodcalorie", "Calorie (food)", "Energy", "4184 J", "none", "Other"],
    ["eV,electronvolt,electronvolts", "Electronvolt", "Energy", "1.602176634e-19 J", "all", "Other"],
    ["erg,ergs", "Erg", "Energy", "1e-7 J", "all", "CGS"],
    ["BTU,btu", "British Thermal Unit", "Energy", "1055.05585262 J", "none", "Imperial"],
    ["thm,therm,therms", "Therm", "Energy", "105480400 J", "none", "US"],
    ["Eh,hartree", "Hartree", "Energy", "4.3597447222071e-18 J", "none", "Natural"],
    ["tTNT,tonTNT", "Ton of TNT", "Energy", "4.184e9 J", "some", "Other"],
    ["W,watt,watts", "Watt", "Power", "1 J/s", "all", "SI"],
    ["hp,horsepower", "Horsepower", "Power", "745.69987158227022 W", "none", "Imperial"],
    ["PS,metrichorsepower", "Metric Horsepower", "Power", "735.49875 W", "none", "Other"],
    ["Pa,pascal,pascals", "Pascal", "Pressure", "1 N/m^2", "all", "SI"],
    ["bar,bars", "Bar", "Pressure", "100000 Pa", "all", "Other"],
    ["atm,atmosphere,atmospheres", "Atmosphere", "Pressure", "101325 Pa", "none", "Other"],
    ["Torr,torr", "Torr", "Pressure", "101325/760 Pa", "some", "Other"],
    ["mmHg", "Millimeter of Mercury", "Pressure", "133.322387415 Pa", "none", "Other"],
    ["inHg", "Inch of Mercury", "Pressure", "3386.389 Pa", "none", "Imperial"],
    ["psi", "Pound per Square Inch", "Pressure", "1 lbf/in^2", "some", "Imperial"],
    ["C,coulomb,coulombs", "Coulomb", "Electric Charge", "1 A*s", "all", "SI"],
    ["Ah,amperehour", "Ampere Hour", "Electric Charge", "3600 C", "all", "Other"],
    ["V,volt,volts", "Volt", "Electric Potential", "1 W/A", "all", "SI"],
    ["Ω,ohm,ohms", "Ohm", "Electric Resistance", "1 V/A", "all", "SI"],
    ["S,siemens,mho", "Siemens", "Electric Conductance", "1 A/V", "all", "SI"],
    ["F,farad,farads", "Farad", "Capacitance", "1 C/V", "all", "SI"],
    ["Wb,weber,webers", "Weber", "Magnetic Flux", "1 V*s", "all", "SI"],
    ["T,tesla,teslas", "Tesla", "Magnetic Flux Density", "1 Wb/m^2", "all", "SI"],
    ["H,henry,henries,henrys", "Henry", "Inductance", "1 Wb/A", "all", "SI"],
    ["G,gauss", "Gauss", "Magnetic Flux Density", "1e-4 T", "all", "CGS"],
    ["Mx,maxwell", "Maxwell", "Magnetic Flux", "1e-8 Wb", "all", "CGS"],
    ["°C,oC,celsius,degC,℃", "Degree Celsius", "Temperature", "@C", "none", "SI"],
    ["°F,oF,fahrenheit,degF,℉", "Degree Fahrenheit", "Temperature", "@F", "none", "Imperial"],
    ["°R,oR,rankine,degR", "Degree Rankine", "Temperature", "5/9 K", "none", "Imperial"],
    ["Hz,hertz", "Hertz", "Frequency", "1 s^-1", "all", "SI"],
    ["rpm", "Revolutions per Minute", "Frequency", "1 min^-1", "none", "Other"],
    ["Bq,becquerel,becquerels", "Becquerel", "Radioactivity", "1 s^-1", "all", "SI"],
    ["Ci,curie,curies", "Curie", "Radioactivity", "3.7e10 Bq", "all", "Other"],
    ["Gy,gray,grays", "Gray", "Absorbed Dose", "1 J/kg", "all", "SI"],
    ["Sv,sievert,sieverts", "Sievert", "Equivalent Dose", "1 J/kg", "all", "SI"],
    ["R_roentgen,roentgen", "Roentgen", "Exposure", "2.58e-4 C/kg", "none", "Other"],
    ["kat,katal", "Katal", "Catalytic Activity", "1 mol/s", "all", "SI"],
    ["lm,lumen,lumens", "Lumen", "Luminous Flux", "1 cd", "all", "SI"],
    ["lx,lux", "Lux", "Illuminance", "1 lm/m^2", "all", "SI"],
    ["°,deg,degree,degrees", "Degree", "Angle", "pi/180 rad", "none", "Other"],
    ["arcmin,arcminute,arcminutes", "Arcminute", "Angle", "1/60 deg", "none", "Other"],
    ["arcsec,arcsecond,arcseconds", "Arcsecond", "Angle", "1/60 arcmin", "some", "Other"],
    ["gon,grad,gradian,gradians", "Gradian", "Angle", "pi/200 rad", "none", "Other"],
    ["tr,turn,turns,rev,revolution", "Turn", "Angle", "2*pi rad", "none", "Other"],
    ["sr,steradian,steradians", "Steradian", "Solid Angle", "=1", "none", "SI"],
    ["B,byte,bytes,octet", "Byte", "Information", "8 bit", "binary", "Other"],
    ["nibble,nibbles", "Nibble", "Information", "4 bit", "none", "Other"],
    ["bps", "Bits per Second", "Data Rate", "1 bit/s", "all", "Other"],
    ["P,poise", "Poise", "Dynamic Viscosity", "0.1 Pa*s", "all", "CGS"],
    ["St,stokes", "Stokes", "Kinematic Viscosity", "1e-4 m^2/s", "all", "CGS"],
    ["M_molar,molar", "Molar", "Concentration", "1000 mol/m^3", "all", "Other"],
];

pub const CURRENCIES: &[[&str; 3]] = &[
    ["EUR", "Euro", "€"],
    ["USD", "US Dollar", "$"],
    ["JPY", "Japanese Yen", "¥"],
    ["BGN", "Bulgarian Lev", ""],
    ["CZK", "Czech Koruna", ""],
    ["DKK", "Danish Krone", ""],
    ["GBP", "British Pound", "£"],
    ["HUF", "Hungarian Forint", ""],
    ["PLN", "Polish Złoty", "zł"],
    ["RON", "Romanian Leu", ""],
    ["SEK", "Swedish Krona", ""],
    ["CHF", "Swiss Franc", ""],
    ["ISK", "Icelandic Króna", ""],
    ["NOK", "Norwegian Krone", ""],
    ["TRY", "Turkish Lira", "₺"],
    ["AUD", "Australian Dollar", ""],
    ["BRL", "Brazilian Real", ""],
    ["CAD", "Canadian Dollar", ""],
    ["CNY", "Chinese Yuan", ""],
    ["HKD", "Hong Kong Dollar", ""],
    ["IDR", "Indonesian Rupiah", ""],
    ["ILS", "Israeli New Shekel", "₪"],
    ["INR", "Indian Rupee", "₹"],
    ["KRW", "South Korean Won", "₩"],
    ["MXN", "Mexican Peso", ""],
    ["MYR", "Malaysian Ringgit", ""],
    ["NZD", "New Zealand Dollar", ""],
    ["PHP", "Philippine Peso", "₱"],
    ["SGD", "Singapore Dollar", ""],
    ["THB", "Thai Baht", "฿"],
    ["ZAR", "South African Rand", ""],
    ["BTC", "Bitcoin", "₿"],
];

// fallback rates per euro, used until a download succeeds
pub const DEFAULT_RATES: &[(&str, f64)] = &[
    ("USD", 1.08),
    ("JPY", 162.0),
    ("BGN", 1.9558),
    ("CZK", 25.2),
    ("DKK", 7.46),
    ("GBP", 0.85),
    ("HUF", 395.0),
    ("PLN", 4.30),
    ("RON", 4.97),
    ("SEK", 11.4),
    ("CHF", 0.95),
    ("ISK", 150.0),
    ("NOK", 11.6),
    ("TRY", 36.0),
    ("AUD", 1.65),
    ("BRL", 6.0),
    ("CAD", 1.48),
    ("CNY", 7.8),
    ("HKD", 8.4),
    ("IDR", 17000.0),
    ("ILS", 4.0),
    ("INR", 90.0),
    ("KRW", 1480.0),
    ("MXN", 20.0),
    ("MYR", 4.9),
    ("NZD", 1.8),
    ("PHP", 61.0),
    ("SGD", 1.45),
    ("THB", 38.0),
    ("ZAR", 19.5),
    ("BTC", 0.000016),
];

pub const MIXED_CHAINS: &[&[&str]] = &[&["yr", "d", "h", "min", "s"], &["mi", "yd", "ft", "in"], &["lb", "oz"], &["st", "lb"], &["°", "arcmin", "arcsec"]];

pub fn unit_factor(u: &Unit, p: Option<PrefixRef>) -> Real {
    match p {
        None => u.factor.clone(),
        Some(p) => &u.factor * &p.value,
    }
}

pub fn fmt_f64(v: f64) -> String {
    format!("{}", v)
}

pub fn load(c: &Calculator) {
    for row in TABLE {
        let names: Vec<String> = row[0].split(',').map(|s| s.to_string()).collect();
        let plural = if names.len() > 2 {
            names[2].clone()
        } else if names.len() > 1 {
            names[1].clone()
        } else {
            names[0].clone()
        };
        let mut u = Unit {
            item: Item { name: names[0].clone(), names, title: row[1].into(), category: row[2].into(), ..Default::default() },
            definition: row[3].into(),
            system: row[5].into(),
            plural,
            prefixes: match row[4] {
                "all" => PrefixUse::All,
                "some" => PrefixUse::Some,
                "binary" => PrefixUse::Binary,
                _ => PrefixUse::None,
            },
            ..Default::default()
        };
        resolve(c, &mut u);
        c.add_unit(u);
    }
    for cur in CURRENCIES {
        let mut u = Unit {
            item: Item { name: cur[0].into(), names: vec![cur[0].into()], title: cur[1].into(), category: "Currency".into(), ..Default::default() },
            is_currency: true,
            system: "Currency".into(),
            plural: cur[0].into(),
            dim: Dim::base(8),
            ..Default::default()
        };
        if !cur[2].is_empty() {
            u.item.names.push(cur[2].into());
        }
        if cur[0] == "EUR" {
            u.factor = Real::one();
            u.is_base = true;
            u.definition = "#8".into();
        } else {
            let rate = DEFAULT_RATES.iter().find(|r| r.0 == cur[0]).map(|r| r.1).unwrap_or(1.0);
            u.factor = Real::one() / Real::from_f64(rate);
            u.approx_factor = true;
            u.definition = format!("1/{} EUR", fmt_f64(rate));
        }
        c.add_unit(u);
    }
}

pub fn set_rates(c: &Calculator, rates: &[(String, f64)]) {
    for (k, v) in rates {
        let existing = c.defs.borrow().units.iter().find(|x| x.is_currency && x.name() == k).cloned();
        let mut u = match existing {
            Some(u) => (*u).clone(),
            None => Unit {
                item: Item { name: k.clone(), names: vec![k.clone()], title: k.clone(), category: "Currency".into(), ..Default::default() },
                is_currency: true,
                system: "Currency".into(),
                plural: k.clone(),
                dim: Dim::base(8),
                ..Default::default()
            },
        };
        if *v > 0.0 {
            u.factor = Real::one() / Real::from_f64(*v);
            u.approx_factor = true;
            u.definition = format!("1/{} EUR", fmt_f64(*v));
        }
        c.add_unit(u);
    }
}

pub fn resolve(c: &Calculator, u: &mut Unit) {
    let def = u.definition.clone();
    if let Some(idx) = def.strip_prefix('#') {
        u.dim = Dim::base(idx.parse().unwrap_or(0));
        u.factor = if u.name() == "g" { Real::frac(1, 1000) } else { Real::one() };
        u.is_base = true;
        return;
    }
    match def.as_str() {
        "@C" => {
            u.dim = Dim::base(4);
            u.factor = Real::one();
            u.offset = Real::frac(27315, 100);
            return;
        }
        "@F" => {
            u.dim = Dim::base(4);
            u.factor = Real::frac(5, 9);
            u.offset = Real::frac(45967, 180);
            return;
        }
        "=1" => {
            u.dim = Dim::none();
            u.factor = Real::one();
            return;
        }
        _ => {}
    }
    c.with_exact(false, || {
        let e = c.ev.eval(&parser::parse_req(&def, c));
        match try_quantity(&e) {
            Some((v, d, rest)) if rest.is_one() && v.is_real() => {
                u.approx_factor = v.re.approx;
                u.factor = v.re;
                u.dim = d;
            }
            _ => throw_s(format!("Invalid unit definition for {}: {}", u.name(), def)),
        }
    });
}

pub fn try_dims(units: &Expr) -> Option<(Dim, Real)> {
    let mut dim = Dim::none();
    let mut factor = Real::one();
    for f in simp::factors_of(units) {
        if f.is_one() {
            continue;
        }
        match &f.e {
            E::Unit(u, p) => {
                dim = dim.mul(&u.dim);
                factor = factor * unit_factor(u, *p);
            }
            E::Pow(b, x) => {
                let (E::Unit(pu, pp), Some(pn)) = (&b.e, x.as_real()) else { return None };
                if !pn.is_int() {
                    return None;
                }
                let k = pn.to_int();
                dim = dim.mul(&pu.dim.pow(k));
                factor = factor * unit_factor(pu, *pp).pow_i(k as i64);
            }
            _ => return None,
        }
    }
    Some((dim, factor))
}

// value * units * rest; units given as dimension and SI factor folded into value
pub fn try_quantity(e: &Expr) -> Option<(Cx, Dim, Expr)> {
    let mut value = Cx::one();
    let mut dim = Dim::none();
    let mut others = vec![];
    for f in simp::factors_of(e) {
        if let E::Num(n) = &f.e {
            value = value * n;
            continue;
        }
        if simp::is_unit_factor(&f) {
            let (d, fac) = try_dims(&f)?;
            dim = dim.mul(&d);
            value = value * Cx::real(fac);
            continue;
        }
        if f.has_units() {
            return None;
        }
        others.push(f);
    }
    let rest = match others.len() {
        0 => one(),
        1 => others.pop().unwrap(),
        _ => mul_e(others),
    };
    Some((value, dim, rest))
}

pub fn to_seconds(t: &Expr) -> Option<Real> {
    let (v, d, rest) = try_quantity(t)?;
    if !rest.is_one() || !v.is_real() || !d.same_as(&Dim::base(2)) {
        return None;
    }
    Some(v.re)
}

pub fn base_units(d: &Dim) -> Expr {
    let c = cur();
    let mut fs = vec![];
    for i in 0..DIM_COUNT {
        if d.e[i] == 0 {
            continue;
        }
        let u = if i == 1 {
            unit_e(c.unit_named("g").unwrap(), prefix_by_name("kilo"))
        } else {
            unit_e(c.unit_named(DIM_SYMBOLS[i]).unwrap(), None)
        };
        fs.push(if d.e[i] == 1 { u } else { pow_e(u, num_i(d.e[i] as i64)) });
    }
    match fs.len() {
        0 => one(),
        1 => fs.pop().unwrap(),
        _ => mul_e(fs),
    }
}

fn units_factor(units: &Expr) -> Real {
    try_dims(units).map(|x| x.1).unwrap_or_else(Real::one)
}

pub fn complexity(units: &Expr) -> i32 {
    let mut c = 0;
    for f in simp::factors_of(units) {
        match &f.e {
            E::Unit(..) => c += 1,
            E::Pow(_, x) => {
                if let Some(n) = x.as_real() {
                    c += 1.max(n.to_f64().abs() as i32);
                }
            }
            _ => {}
        }
    }
    c
}

fn is_temperature(units: &Expr) -> Option<UnitRef> {
    match &units.e {
        E::Unit(u, None) if u.dim.same_as(&Dim::base(4)) => Some(u.clone()),
        _ => None,
    }
}

// converts e to the units of target
pub fn convert(e: &Expr, target: &Expr) -> Expr {
    match &e.e {
        E::Add(t) => return simp::add(t.iter().map(|x| convert(x, target)).collect()),
        E::Mat(..) => return matrices::map_elements(e, &mut |x| convert(&x, target)),
        E::Cmp(op, l, r) => return cmp_e(*op, convert(l, target), convert(r, target)),
        _ => {}
    }
    let Some((tv, td, _)) = try_quantity(target) else { throw("Invalid conversion target") };
    let (_, t_all) = simp::split_coef(target);
    let (_, t_units) = simp::split_units(&t_all);
    let Some((ev, ed, erest)) = try_quantity(e) else { return e.clone() };
    let (ecoef, e_all) = simp::split_coef(e);
    let (_, e_units) = simp::split_units(&e_all);

    if cur().opts().temperature != TemperatureMode::Relative && ecoef.is_real() {
        if let (Some(from_t), Some(to_t)) = (is_temperature(&e_units), is_temperature(&t_units)) {
            let kelvin = &ecoef.re * &from_t.factor + &from_t.offset;
            let outv = (kelvin - &to_t.offset) / &to_t.factor;
            return simp::mul2(&simp::mul2(&num_r(outv), &erest), &t_units);
        }
    }

    if ed.same_as(&td) {
        let val = &ev / &tv;
        return simp::mul(vec![num(val), erest, t_units]);
    }
    if ed.same_as(&td.pow(-1)) && !ed.is_none() {
        let val = Cx::one() / (&ev * &tv);
        let rest_inv = simp::pow(&erest, &minus_one());
        return simp::mul(vec![num(val), rest_inv, t_units]);
    }
    // partial conversion only when the target is part of the value, like J to N
    for i in 0..DIM_COUNT {
        if td.e[i] != 0 && (td.e[i].signum() != ed.e[i].signum() || td.e[i].abs() > ed.e[i].abs()) {
            return e.clone();
        }
    }
    let rem_dim = ed.mul(&td.pow(-1));
    let rem_units = best_units(&rem_dim).unwrap_or_else(|| base_units(&rem_dim));
    let rem_factor = units_factor(&rem_units);
    let value = &ev / &(&tv * &Cx::real(rem_factor));
    simp::mul(vec![num(value), erest, t_units, rem_units])
}

pub fn to_base(e: &Expr) -> Expr {
    match &e.e {
        E::Add(t) => return simp::add(t.iter().map(to_base).collect()),
        E::Mat(..) => return matrices::map_elements(e, &mut |x| to_base(&x)),
        _ => {}
    }
    if !e.has_units() {
        return e.clone();
    }
    let Some((v, d, rest)) = try_quantity(e) else { return e.clone() };
    let (coef, all) = simp::split_coef(e);
    let (_, units) = simp::split_units(&all);
    if let Some(tu) = is_temperature(&units) {
        if tu.has_offset() && cur().opts().temperature != TemperatureMode::Relative && coef.is_real() {
            return simp::mul(vec![num_r(&coef.re * &tu.factor + &tu.offset), rest, base_units(&d)]);
        }
    }
    simp::mul(vec![num(v), rest, base_units(&d)])
}

const DERIVED_PREFERRED: &[&str] = &["N", "J", "W", "Pa", "C", "V", "Ω", "F", "H", "Wb", "T", "S", "Gy", "kat", "lx"];

// finds the simplest unit expression for a dimension
pub fn best_units(d: &Dim) -> Option<Expr> {
    if d.is_none() {
        return Some(one());
    }
    let c = cur();
    let mut best: Option<Expr> = None;
    let mut best_c = i32::MAX;
    let mut consider = |cand: Expr| {
        let Some((cd, _)) = try_dims(&cand) else { return };
        if !cd.same_as(d) {
            return;
        }
        let cc = complexity(&cand);
        if cc < best_c {
            best = Some(cand);
            best_c = cc;
        }
    };
    consider(base_units(d));
    for n in DERIVED_PREFERRED {
        let Some((u, _)) = c.find_unit(n, false) else { continue };
        if u.name() == "Gy" && !d.same_as(&u.dim) {
            continue;
        }
        let ue = unit_e(u, None);
        consider(ue.clone());
        for b in 0..DIM_COUNT {
            if b == 9 || b == 8 {
                continue;
            }
            for k in -3..=3 {
                if k == 0 {
                    continue;
                }
                let bexpr = base_units(&Dim::base(b).pow(k));
                consider(simp::mul2(&ue, &bexpr));
            }
        }
    }
    best
}
pub fn optimal(e: &Expr) -> Expr {
    match &e.e {
        E::Add(t) => return simp::add(t.iter().map(optimal).collect()),
        E::Mat(..) => return matrices::map_elements(e, &mut |x| optimal(&x)),
        E::Cmp(op, l, r) => return cmp_e(*op, optimal(l), optimal(r)),
        _ => {}
    }
    if !e.has_units() {
        return e.clone();
    }
    let Some((v, d, rest)) = try_quantity(e) else { return e.clone() };
    let (_, all) = simp::split_coef(e);
    let (_, units) = simp::split_units(&all);
    if units.is_one() {
        return e.clone();
    }
    let curc = complexity(&units);
    if d.is_none() {
        return simp::mul2(&num(v), &rest);
    }
    if is_currency_only(&units) {
        return e.clone();
    }
    let Some(best) = best_units(&d) else { return e.clone() };
    let bc = complexity(&best);
    let cancels = sum_of_parts(&units) > d.complexity();
    if bc < curc && (curc >= 3 || cancels) {
        let f = units_factor(&best);
        return simp::mul(vec![num(&v / &Cx::real(f)), rest, best]);
    }
    e.clone()
}

fn is_currency_only(units: &Expr) -> bool {
    simp::factors_of(units).iter().all(|f| match &f.e {
        E::Unit(u, _) => u.is_currency,
        E::Pow(b, _) => matches!(&b.e, E::Unit(u, _) if u.is_currency),
        _ => false,
    })
}

fn sum_of_parts(units: &Expr) -> i32 {
    let mut s = 0;
    for f in simp::factors_of(units) {
        match &f.e {
            E::Unit(u, _) => s += u.dim.complexity(),
            E::Pow(b, x) => {
                if let (E::Unit(pu, _), Some(n)) = (&b.e, x.as_real()) {
                    s += pu.dim.complexity() * n.to_f64().abs() as i32;
                }
            }
            _ => {}
        }
    }
    s
}
pub fn optimize_prefix(e: &Expr, force: bool) -> Expr {
    if let E::Add(t) = &e.e {
        return simp::add(t.iter().map(|x| optimize_prefix(x, force)).collect());
    }
    let E::Mul(m) = &e.e else { return e.clone() };
    if m.len() < 2 {
        return e.clone();
    }
    let Some(n) = m[0].as_real() else { return e.clone() };
    if n.is_zero() {
        return e.clone();
    }
    let Some(ui) = (1..m.len()).find(|&i| simp::is_unit_factor(&m[i])) else { return e.clone() };
    let unit_count = m.iter().filter(|x| simp::is_unit_factor(x)).count();
    let f = &m[ui];
    let (u_expr, pw) = match &f.e {
        E::Pow(b, x) => (b.clone(), x.num().re.to_f64().round() as i32),
        _ => (f.clone(), 1),
    };
    let E::Unit(uu, up) = &u_expr.e else { return e.clone() };
    let o = cur().opts().clone();
    if pw <= 0 && !o.denominator_prefixes {
        return e.clone();
    }
    if uu.prefixes == PrefixUse::None || uu.has_offset() {
        return e.clone();
    }
    if uu.is_currency && !o.currency_prefixes {
        return e.clone();
    }
    if !force && o.prefixes == PrefixMode::None {
        return e.clone();
    }
    if !force && o.prefixes == PrefixMode::Some && up.is_none() && uu.prefixes != PrefixUse::All && uu.prefixes != PrefixUse::Binary {
        return e.clone();
    }
    let mut min_exp = -30;
    let mut max_exp = 30;
    if !force && o.prefixes == PrefixMode::Some {
        if unit_count != 1 || pw != 1 {
            return e.clone();
        }
        match uu.name() {
            "m" | "g" => {
                min_exp = -12;
                max_exp = 3;
            }
            "s" => {
                min_exp = -15;
                max_exp = 0;
            }
            "L" => {
                min_exp = -6;
                max_exp = 0;
            }
            "B" | "bit" => {
                min_exp = 0;
                max_exp = 24;
            }
            "W" | "J" | "V" | "Hz" | "N" | "Pa" | "A" | "Ω" | "F" | "C" | "H" | "T" | "Wh" | "eV" | "S" | "Wb" | "bps" => {
                min_exp = -12;
                max_exp = 12;
            }
            _ => {
                if up.is_none() {
                    return e.clone();
                }
            }
        }
    }
    let v = n.to_f64().abs();
    if v.is_infinite() || v == 0.0 {
        return e.clone();
    }
    let base_factor = up.map(|p| p.value.clone()).unwrap_or_else(Real::one);
    let total = v * base_factor.to_f64().powi(pw);
    let binary = uu.prefixes == PrefixUse::Binary && (o.binary_prefixes || up.map(|p| p.binary).unwrap_or(false));
    let cands: Vec<Option<PrefixRef>> = PREFIXES
        .iter()
        .filter(|p| if binary { p.binary } else { !p.binary })
        .filter(|p| p.common || o.all_si_prefixes || up.map(|x| std::ptr::eq(x, *p)).unwrap_or(false))
        .filter(|p| p.binary || (p.exp >= min_exp && p.exp <= max_exp))
        .map(Some)
        .chain(std::iter::once(None))
        .collect();
    let mut best: Option<PrefixRef> = None;
    let mut best_score = f64::MAX;
    for p in cands {
        let pv = p.map(|p| p.value.to_f64()).unwrap_or(1.0);
        let val = total / pv.powi(pw);
        let limit = if binary { 1024.0 } else { 1000.0 };
        let mut score = if val >= 1.0 && val < limit {
            0.0
        } else if val < 1.0 {
            1.0 / val
        } else {
            val / limit
        };
        if uu.name() == "g" && p.is_none() && total >= 1000.0 {
            score += 1e-9;
        }
        if score < best_score - 1e-12 {
            best_score = score;
            best = p;
        }
    }
    if uu.name() == "s" && best.map(|b| b.exp > 0).unwrap_or(false) {
        return e.clone();
    }
    if uu.name() == "L" && best.map(|b| b.exp > 0).unwrap_or(false) {
        best = None;
    }
    if best.map(|b| b.name) == up.map(|p| p.name) {
        return e.clone();
    }
    let nu = unit_e(uu.clone(), best);
    let ratio = rmath::pow(&(&base_factor / &best.map(|b| b.value.clone()).unwrap_or_else(Real::one)), &Real::int(pw as i64));
    let mut fs = m.clone();
    fs[0] = num_r(n * &ratio);
    fs[ui] = if pw == 1 { nu } else { pow_e(nu, num_i(pw as i64)) };
    mul_e(fs)
}

// 1.5 h -> 1 h + 30 min
pub fn to_mixed(e: &Expr, force: bool) -> Expr {
    let E::Mul(m) = &e.e else { return e.clone() };
    if m.len() != 2 {
        return e.clone();
    }
    let (Some(n), E::Unit(u, None)) = (m[0].as_real(), &m[1].e) else { return e.clone() };
    let Some(chain) = MIXED_CHAINS.iter().find(|ch| ch.contains(&u.name())) else { return e.clone() };
    let mut val = n.clone();
    if val.is_int() && !force {
        return e.clone();
    }
    let neg = val.sign() < 0;
    if neg {
        val = -val;
    }
    if !force && val.lt(&Real::one()) {
        return e.clone();
    }
    let c = cur();
    let start = chain.iter().position(|x| *x == u.name()).unwrap();
    let mut parts = vec![];
    let mut remaining = &val * &u.factor;
    let approx = val.approx;
    for i in start..chain.len() {
        let cu = c.unit_named(chain[i]).unwrap();
        let amount = &remaining / &cu.factor;
        if i == chain.len() - 1 {
            if approx && amount.to_f64().abs() < 1e-9 {
                break;
            }
            if !amount.is_zero() {
                parts.push(simp::mul2(&num_r(amount), &unit_e(cu, None)));
            }
            break;
        }
        let mut whole = amount.floor();
        let mut frac_part = &amount - &whole;
        if approx && (1.0 - frac_part.to_f64()).abs() < 1e-9 {
            whole = whole + Real::one();
            frac_part = Real::zero();
        }
        if !whole.is_zero() {
            parts.push(simp::mul2(&num_r(Real::big_int(whole.to_big())), &unit_e(cu.clone(), None)));
        }
        remaining = &frac_part * &cu.factor;
        if remaining.is_zero() || (approx && frac_part.to_f64().abs() < 1e-9) {
            break;
        }
    }
    if parts.len() <= 1 {
        return e.clone();
    }
    if neg {
        parts = parts.iter().map(simp::neg).collect();
    }
    add_e(parts)
}