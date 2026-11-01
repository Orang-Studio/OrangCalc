use super::defs::{FnRef, PrefixRef, UnitRef, VarRef};
use super::{Cx, Real};
use chrono::NaiveDateTime;
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicOp {
    And,
    Or,
    Xor,
    Not,
}

pub enum E {
    Num(Cx),
    Sym(String),
    Var(VarRef),
    Unit(UnitRef, Option<PrefixRef>),
    Add(Vec<Expr>),
    Mul(Vec<Expr>),
    Pow(Expr, Expr),
    Call(FnRef, Vec<Expr>),
    Mat(Vec<Vec<Expr>>, bool),
    Cmp(CmpOp, Expr, Expr),
    Logic(LogicOp, Vec<Expr>),
    Str(String),
    // t, has_time, utc
    Date(NaiveDateTime, bool, bool),
    // closed interval [lo, hi]
    Intv(Real, Real),
    // placeholder for an argument slot in user function definitions (\x, \y ...)
    Arg(usize),
}

pub struct Node {
    pub e: E,
    key: OnceLock<String>,
}

pub type Expr = Arc<Node>;

pub fn mk(e: E) -> Expr {
    Arc::new(Node { e, key: OnceLock::new() })
}

pub fn num(v: Cx) -> Expr {
    mk(E::Num(v))
}
pub fn num_r(v: Real) -> Expr {
    mk(E::Num(Cx::real(v)))
}
pub fn num_i(v: i64) -> Expr {
    mk(E::Num(Cx::int(v)))
}
pub fn sym(n: &str) -> Expr {
    mk(E::Sym(n.to_string()))
}
pub fn add_e(t: Vec<Expr>) -> Expr {
    mk(E::Add(t))
}
pub fn mul_e(f: Vec<Expr>) -> Expr {
    mk(E::Mul(f))
}
pub fn pow_e(b: Expr, x: Expr) -> Expr {
    mk(E::Pow(b, x))
}
pub fn call_e(f: FnRef, a: Vec<Expr>) -> Expr {
    mk(E::Call(f, a))
}
pub fn str_e(s: &str) -> Expr {
    mk(E::Str(s.to_string()))
}
pub fn unit_e(u: UnitRef, p: Option<PrefixRef>) -> Expr {
    mk(E::Unit(u, p))
}
pub fn var_e(v: VarRef) -> Expr {
    mk(E::Var(v))
}
pub fn cmp_e(op: CmpOp, l: Expr, r: Expr) -> Expr {
    mk(E::Cmp(op, l, r))
}
pub fn logic_e(op: LogicOp, items: Vec<Expr>) -> Expr {
    mk(E::Logic(op, items))
}
pub fn mat_e(rows: Vec<Vec<Expr>>, vector: bool) -> Expr {
    mk(E::Mat(rows, vector))
}
pub fn vector_e(items: Vec<Expr>) -> Expr {
    mk(E::Mat(vec![items], true))
}
pub fn intv_e(lo: Real, hi: Real) -> Expr {
    if lo.gt(&hi) {
        mk(E::Intv(hi, lo))
    } else {
        mk(E::Intv(lo, hi))
    }
}
pub fn date_e(t: NaiveDateTime, has_time: bool, utc: bool) -> Expr {
    mk(E::Date(t, has_time, utc))
}

thread_local! {
    static CONSTS: [Expr; 5] = [num_i(0), num_i(1), num_i(-1), num_i(2), num_r(Real::half())];
}
pub fn zero() -> Expr {
    CONSTS.with(|c| c[0].clone())
}
pub fn one() -> Expr {
    CONSTS.with(|c| c[1].clone())
}
pub fn minus_one() -> Expr {
    CONSTS.with(|c| c[2].clone())
}
pub fn two() -> Expr {
    CONSTS.with(|c| c[3].clone())
}
pub fn half() -> Expr {
    CONSTS.with(|c| c[4].clone())
}

fn rkey(r: &Real) -> String {
    format!("{}/{}", r.n, r.d)
}

fn join_keys(v: &[Expr], sep: &str) -> String {
    v.iter().map(|e| e.key().to_string()).collect::<Vec<_>>().join(sep)
}

impl Node {
    pub fn key(&self) -> &str {
        self.key.get_or_init(|| self.build_key())
    }

    fn build_key(&self) -> String {
        match &self.e {
            E::Num(v) => {
                if v.is_real() {
                    format!("#{}{}{}", rkey(&v.re), if v.re.inf != 0 { format!("inf{}", v.re.inf) } else { String::new() }, if v.re.approx { "~" } else { "" })
                } else {
                    format!("#{}i{}{}", rkey(&v.re), rkey(&v.im), if v.approx() { "~" } else { "" })
                }
            }
            E::Sym(n) => format!("s:{}", n),
            E::Var(v) => format!("v:{}", v.name()),
            E::Unit(u, p) => format!("u:{}{}", p.map(|p| format!("{}|", p.name)).unwrap_or_default(), u.name()),
            E::Add(t) => format!("+({})", join_keys(t, ",")),
            E::Mul(f) => format!("*({})", join_keys(f, ",")),
            E::Pow(b, x) => format!("^({},{})", b.key(), x.key()),
            E::Call(f, a) => format!("f:{}({})", f.name(), join_keys(a, ",")),
            E::Mat(rows, v) => format!("{}{}]", if *v { "vec[" } else { "mat[" }, rows.iter().map(|r| join_keys(r, ",")).collect::<Vec<_>>().join(";")),
            E::Cmp(op, l, r) => format!("c{}({},{})", *op as i32, l.key(), r.key()),
            E::Logic(op, items) => format!("l{}({})", *op as i32, join_keys(items, ",")),
            E::Str(s) => format!("\"{}\"", s),
            E::Date(t, _, utc) => format!("d:{}{}", t.and_utc().timestamp_nanos_opt().unwrap_or(0), if *utc { "z" } else { "" }),
            E::Intv(lo, hi) => format!("I({},{})", rkey(lo), rkey(hi)),
            E::Arg(i) => format!("\\{}", i),
        }
    }

    pub fn kids(&self) -> Vec<Expr> {
        match &self.e {
            E::Add(v) | E::Mul(v) | E::Call(_, v) | E::Logic(_, v) => v.clone(),
            E::Pow(a, b) | E::Cmp(_, a, b) => vec![a.clone(), b.clone()],
            E::Mat(rows, _) => rows.iter().flatten().cloned().collect(),
            _ => vec![],
        }
    }

    pub fn kid_count(&self) -> usize {
        match &self.e {
            E::Add(v) | E::Mul(v) | E::Call(_, v) | E::Logic(_, v) => v.len(),
            E::Pow(..) | E::Cmp(..) => 2,
            E::Mat(rows, _) => rows.iter().map(|r| r.len()).sum(),
            _ => 0,
        }
    }

    pub fn rebuild(self: &Expr, kids: Vec<Expr>) -> Expr {
        match &self.e {
            E::Add(_) => add_e(kids),
            E::Mul(_) => mul_e(kids),
            E::Pow(..) => pow_e(kids[0].clone(), kids[1].clone()),
            E::Call(f, _) => call_e(f.clone(), kids),
            E::Logic(op, _) => logic_e(*op, kids),
            E::Cmp(op, ..) => cmp_e(*op, kids[0].clone(), kids[1].clone()),
            E::Mat(rows, v) => {
                let mut it = kids.into_iter();
                let nr = rows.iter().map(|r| (0..r.len()).map(|_| it.next().unwrap()).collect()).collect();
                mat_e(nr, *v)
            }
            _ => self.clone(),
        }
    }

    pub fn same(&self, o: &Node) -> bool {
        std::ptr::eq(self, o) || self.key() == o.key()
    }

    pub fn any(&self, pred: &mut dyn FnMut(&Expr) -> bool, me: &Expr) -> bool {
        if pred(me) {
            return true;
        }
        match &self.e {
            E::Add(v) | E::Mul(v) | E::Call(_, v) | E::Logic(_, v) => v.iter().any(|k| k.any(pred, k)),
            E::Pow(a, b) | E::Cmp(_, a, b) => a.any(pred, a) || b.any(pred, b),
            E::Mat(rows, _) => rows.iter().flatten().any(|k| k.any(pred, k)),
            _ => false,
        }
    }

    pub fn is_num(&self) -> bool {
        matches!(self.e, E::Num(_))
    }
    pub fn num(&self) -> &Cx {
        match &self.e {
            E::Num(v) => v,
            _ => super::throw("Expected a number"),
        }
    }
    pub fn as_num(&self) -> Option<&Cx> {
        match &self.e {
            E::Num(v) => Some(v),
            _ => None,
        }
    }
    pub fn as_real(&self) -> Option<&Real> {
        match &self.e {
            E::Num(v) if v.is_real() => Some(&v.re),
            _ => None,
        }
    }
    pub fn is_zero(&self) -> bool {
        matches!(&self.e, E::Num(v) if v.is_zero())
    }
    pub fn is_one(&self) -> bool {
        matches!(&self.e, E::Num(v) if v.is_one())
    }
    pub fn is_minus_one(&self) -> bool {
        matches!(&self.e, E::Num(v) if v.is_real() && v.re.eqv(&Real::minus_one()))
    }
    pub fn is_real_num(&self) -> bool {
        matches!(&self.e, E::Num(v) if v.is_real())
    }
    pub fn is_integer(&self) -> bool {
        matches!(&self.e, E::Num(v) if v.is_int())
    }
    pub fn is_mat(&self) -> bool {
        matches!(self.e, E::Mat(..))
    }
    pub fn is_unit(&self) -> bool {
        matches!(self.e, E::Unit(..))
    }
    pub fn is_add(&self) -> bool {
        matches!(self.e, E::Add(_))
    }
    pub fn is_mul(&self) -> bool {
        matches!(self.e, E::Mul(_))
    }
    pub fn is_call(&self) -> bool {
        matches!(self.e, E::Call(..))
    }
    pub fn is_call_of(&self, name: &str) -> bool {
        matches!(&self.e, E::Call(f, _) if f.name() == name)
    }
    pub fn is_var_named(&self, name: &str) -> bool {
        matches!(&self.e, E::Var(v) if v.name() == name)
    }
    pub fn is_intv(&self) -> bool {
        matches!(self.e, E::Intv(..))
    }
    pub fn is_str(&self) -> bool {
        matches!(self.e, E::Str(_))
    }
    pub fn is_date(&self) -> bool {
        matches!(self.e, E::Date(..))
    }
    pub fn is_sym(&self) -> bool {
        matches!(self.e, E::Sym(_))
    }
    pub fn is_cmp(&self) -> bool {
        matches!(self.e, E::Cmp(..))
    }
    pub fn as_str(&self) -> Option<&str> {
        match &self.e {
            E::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_sym(&self) -> Option<&str> {
        match &self.e {
            E::Sym(s) => Some(s),
            _ => None,
        }
    }
}

// tree helpers that need the Arc handle
pub trait ExprExt {
    fn map(&self, f: &mut dyn FnMut(Expr) -> Expr) -> Expr;
    fn any_of(&self, pred: &mut dyn FnMut(&Expr) -> bool) -> bool;
    fn has_units(&self) -> bool;
    fn has_symbols(&self) -> bool;
    fn is_approx(&self) -> bool;
}

impl ExprExt for Expr {
    fn map(&self, f: &mut dyn FnMut(Expr) -> Expr) -> Expr {
        if self.kid_count() == 0 {
            return f(self.clone());
        }
        let kids = self.kids();
        let mut list = Vec::with_capacity(kids.len());
        let mut changed = false;
        for k in &kids {
            let m = k.map(f);
            if !Arc::ptr_eq(&m, k) {
                changed = true;
            }
            list.push(m);
        }
        f(if changed { self.rebuild(list) } else { self.clone() })
    }
    fn any_of(&self, pred: &mut dyn FnMut(&Expr) -> bool) -> bool {
        self.any(pred, self)
    }
    fn has_units(&self) -> bool {
        self.any_of(&mut |e| e.is_unit())
    }
    fn has_symbols(&self) -> bool {
        self.any_of(&mut |e| e.is_sym())
    }
    fn is_approx(&self) -> bool {
        self.any_of(&mut |e| match &e.e {
            E::Num(n) => n.approx(),
            E::Intv(..) => true,
            _ => false,
        })
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.key())
    }
}