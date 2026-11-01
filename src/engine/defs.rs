use super::{Calculator, Expr, Real};
use std::collections::HashSet;
use std::sync::{Arc, LazyLock, OnceLock};

#[derive(Clone, Default, Debug)]
pub struct Item {
    pub name: String,
    pub names: Vec<String>,
    pub title: String,
    pub category: String,
    pub description: String,
    pub is_user: bool,
    pub hidden: bool,
    pub inactive: bool,
}

impl Item {
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.name
        } else {
            &self.title
        }
    }
    pub fn has_name(&self, n: &str) -> bool {
        self.names.iter().any(|x| x == n)
    }
    pub fn active(&self) -> bool {
        !self.inactive
    }
}

pub type DynFn = fn(&Calculator) -> Expr;

#[derive(Clone)]
pub struct Variable {
    pub item: Item,
    pub value: OnceLock<Expr>,
    pub value_text: String,
    pub is_constant: bool,
    pub is_approximate: bool,
    pub is_unknown: bool,
    pub atype: super::AssumeType,
    pub asign: super::AssumeSign,
    pub dynamic: Option<DynFn>,
    pub keep_symbolic: bool,
    pub has_unit: bool,
}

impl Default for Variable {
    fn default() -> Self {
        Variable {
            item: Item::default(),
            value: OnceLock::new(),
            value_text: String::new(),
            is_constant: false,
            is_approximate: false,
            is_unknown: false,
            atype: super::AssumeType::Real,
            asign: super::AssumeSign::Unknown,
            dynamic: None,
            keep_symbolic: false,
            has_unit: false,
        }
    }
}

impl Variable {
    pub fn name(&self) -> &str {
        &self.item.name
    }
    pub fn with_value(mut self, v: Expr) -> Self {
        self.value = OnceLock::new();
        let _ = self.value.set(v);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixUse {
    None,
    Some,
    All,
    Binary,
}

#[derive(Debug)]
pub struct Prefix {
    pub name: &'static str,
    pub symbol: &'static str,
    pub value: Real,
    pub exp: i32,
    pub binary: bool,
    pub common: bool,
}

impl Prefix {
    fn new(name: &'static str, symbol: &'static str, exp: i32, binary: bool, common: bool) -> Prefix {
        let value = if binary { Real::int(2).pow_i(exp as i64) } else { Real::int(10).pow_i(exp as i64) };
        Prefix { name, symbol, value, exp, binary, common }
    }
}

pub static PREFIXES: LazyLock<Vec<Prefix>> = LazyLock::new(|| {
    let p = Prefix::new;
    vec![
        p("quecto", "q", -30, false, false),
        p("ronto", "r", -27, false, false),
        p("yocto", "y", -24, false, true),
        p("zepto", "z", -21, false, true),
        p("atto", "a", -18, false, true),
        p("femto", "f", -15, false, true),
        p("pico", "p", -12, false, true),
        p("nano", "n", -9, false, true),
        p("micro", "µ", -6, false, true),
        p("milli", "m", -3, false, true),
        p("centi", "c", -2, false, false),
        p("deci", "d", -1, false, false),
        p("deca", "da", 1, false, false),
        p("hecto", "h", 2, false, false),
        p("kilo", "k", 3, false, true),
        p("mega", "M", 6, false, true),
        p("giga", "G", 9, false, true),
        p("tera", "T", 12, false, true),
        p("peta", "P", 15, false, true),
        p("exa", "E", 18, false, true),
        p("zetta", "Z", 21, false, true),
        p("yotta", "Y", 24, false, true),
        p("ronna", "R", 27, false, false),
        p("quetta", "Q", 30, false, false),
        p("kibi", "Ki", 10, true, true),
        p("mebi", "Mi", 20, true, true),
        p("gibi", "Gi", 30, true, true),
        p("tebi", "Ti", 40, true, true),
        p("pebi", "Pi", 50, true, true),
        p("exbi", "Ei", 60, true, true),
        p("zebi", "Zi", 70, true, true),
        p("yobi", "Yi", 80, true, true),
    ]
});

pub type PrefixRef = &'static Prefix;

pub fn prefix_by_name(n: &str) -> Option<PrefixRef> {
    PREFIXES.iter().find(|p| p.name == n || p.symbol == n)
}

pub const DIM_COUNT: usize = 10;
// exponents of m, kg, s, A, K, mol, cd, bit, currency, rad
pub const DIM_SYMBOLS: [&str; DIM_COUNT] = ["m", "kg", "s", "A", "K", "mol", "cd", "bit", "EUR", "rad"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dim {
    pub e: [i32; DIM_COUNT],
}

impl Dim {
    pub fn none() -> Dim {
        Dim::default()
    }
    pub fn base(index: usize) -> Dim {
        let mut d = Dim::default();
        d.e[index] = 1;
        d
    }
    pub fn is_none(&self) -> bool {
        self.e.iter().all(|x| *x == 0)
    }
    pub fn mul(&self, o: &Dim) -> Dim {
        let mut r = Dim::default();
        for i in 0..DIM_COUNT {
            r.e[i] = self.e[i] + o.e[i];
        }
        r
    }
    pub fn pow(&self, p: i32) -> Dim {
        let mut r = Dim::default();
        for i in 0..DIM_COUNT {
            r.e[i] = self.e[i] * p;
        }
        r
    }
    pub fn same_as(&self, o: &Dim) -> bool {
        self.e == o.e
    }
    pub fn complexity(&self) -> i32 {
        self.e.iter().map(|x| x.abs()).sum()
    }
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub item: Item,
    pub plural: String,
    pub factor: Real,
    pub offset: Real,
    pub dim: Dim,
    pub prefixes: PrefixUse,
    pub definition: String,
    pub system: String,
    pub is_currency: bool,
    pub is_base: bool,
    pub approx_factor: bool,
}

impl Default for Unit {
    fn default() -> Self {
        Unit {
            item: Item::default(),
            plural: String::new(),
            factor: Real::one(),
            offset: Real::zero(),
            dim: Dim::default(),
            prefixes: PrefixUse::None,
            definition: String::new(),
            system: String::new(),
            is_currency: false,
            is_base: false,
            approx_factor: false,
        }
    }
}

impl Unit {
    pub fn name(&self) -> &str {
        &self.item.name
    }
    pub fn abbrev(&self) -> &str {
        self.item.names.first().map(|s| s.as_str()).unwrap_or(&self.item.name)
    }
    pub fn has_offset(&self) -> bool {
        !self.offset.is_zero()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgType {
    Any,
    Number,
    Integer,
    Real,
    NonNegInt,
    PositiveInt,
    Matrix,
    Vector,
    Text,
    Symbol,
    Boolean,
    Date,
    Angle,
    Unevaluated,
}

#[derive(Clone, Debug)]
pub struct ArgDef {
    pub name: String,
    pub ty: ArgType,
    pub default: Option<String>,
}

impl ArgDef {
    pub fn new(name: &str, ty: ArgType, def: Option<&str>) -> ArgDef {
        ArgDef { name: name.to_string(), ty, default: def.map(|s| s.to_string()) }
    }
}

pub type FnImpl = Arc<dyn Fn(&Calculator, &[Expr]) -> Option<Expr> + Send + Sync>;

#[derive(Clone)]
pub struct MathFunction {
    pub item: Item,
    pub args: Vec<ArgDef>,
    pub min_args: i32,
    pub max_args: i32,
    pub imp: Option<FnImpl>,
    pub formula: String,
    pub parsed: OnceLock<Expr>,
    pub condition: String,
    pub example: String,
    pub elementwise: bool,
    pub hold_all: bool,
    pub hold_args: HashSet<usize>,
    pub raw_args: bool,
}

impl Default for MathFunction {
    fn default() -> Self {
        MathFunction {
            item: Item::default(),
            args: vec![],
            min_args: 0,
            max_args: 0,
            imp: None,
            formula: String::new(),
            parsed: OnceLock::new(),
            condition: String::new(),
            example: String::new(),
            elementwise: false,
            hold_all: false,
            hold_args: HashSet::new(),
            raw_args: false,
        }
    }
}

impl MathFunction {
    pub fn name(&self) -> &str {
        &self.item.name
    }
    pub fn is_user_formula(&self) -> bool {
        self.imp.is_none()
    }
    pub fn holds(&self, i: usize) -> bool {
        self.hold_all || self.hold_args.contains(&i)
    }
    pub fn signature(&self) -> String {
        let args: Vec<String> = self.args.iter().enumerate().map(|(i, a)| if i as i32 >= self.min_args { format!("[{}]", a.name) } else { a.name.clone() }).collect();
        format!("{}({}{})", self.item.names[0], args.join(", "), if self.max_args < 0 { ", …" } else { "" })
    }
}

#[derive(Clone, Debug)]
pub struct DataProperty {
    pub name: String,
    pub title: String,
    pub unit: String,
    pub description: String,
    pub is_key: bool,
    pub is_text: bool,
    pub approximate: bool,
    pub hidden: bool,
}

impl DataProperty {
    pub fn new(name: &str, title: &str, unit: &str, text: bool) -> DataProperty {
        DataProperty { name: name.into(), title: title.into(), unit: unit.into(), description: String::new(), is_key: false, is_text: text, approximate: true, hidden: false }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataObject {
    // keys stored lowercase for case insensitive lookup, original order kept
    pub values: Vec<(String, String)>,
}

impl DataObject {
    pub fn get(&self, p: &str) -> Option<&str> {
        self.values.iter().find(|(k, _)| k.eq_ignore_ascii_case(p)).map(|(_, v)| v.as_str())
    }
    pub fn set(&mut self, p: &str, v: String) {
        if let Some(e) = self.values.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(p)) {
            e.1 = v;
        } else {
            self.values.push((p.to_string(), v));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataSet {
    pub item: Item,
    pub properties: Vec<DataProperty>,
    pub objects: Vec<DataObject>,
}

impl DataSet {
    pub fn prop(&self, n: &str) -> Option<&DataProperty> {
        self.properties.iter().find(|p| p.name.eq_ignore_ascii_case(n) || p.title.eq_ignore_ascii_case(n))
    }
    pub fn find(&self, key: &str) -> Option<&DataObject> {
        for o in &self.objects {
            for p in self.properties.iter().filter(|p| p.is_key) {
                if let Some(v) = o.get(&p.name) {
                    if v.eq_ignore_ascii_case(key) {
                        return Some(o);
                    }
                }
            }
        }
        None
    }
}

pub type VarRef = Arc<Variable>;
pub type UnitRef = Arc<Unit>;
pub type FnRef = Arc<MathFunction>;