use super::algebra as simp;
use super::c64::{c64, C64};
use super::real::big;
use super::special as sp;
use super::*;
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

pub use super::funcs2::unhold;
pub type R = Option<Expr>;

pub fn parse_type(s: &str) -> ArgType {
    match s {
        "num" => ArgType::Number,
        "int" => ArgType::Integer,
        "real" => ArgType::Real,
        "nnint" => ArgType::NonNegInt,
        "pint" => ArgType::PositiveInt,
        "mat" => ArgType::Matrix,
        "vec" => ArgType::Vector,
        "text" => ArgType::Text,
        "sym" => ArgType::Symbol,
        "bool" => ArgType::Boolean,
        "date" => ArgType::Date,
        "angle" => ArgType::Angle,
        "hold" => ArgType::Unevaluated,
        _ => ArgType::Any,
    }
}

// builds a function from a compact argument spec like "x:num,base=e,..."
pub fn build(names: &str, cat: &str, title: &str, args: &str, desc: &str, ew: bool, imp: Option<FnImpl>) -> MathFunction {
    let mut f = MathFunction {
        item: Item { names: names.split(',').map(|s| s.to_string()).collect(), category: cat.into(), title: title.into(), description: desc.into(), ..Default::default() },
        imp,
        elementwise: ew,
        ..Default::default()
    };
    f.item.name = f.item.names[0].clone();
    let mut min = 0;
    f.max_args = 0;
    if !args.is_empty() {
        for part in args.split(',') {
            let mut a = part.trim().to_string();
            if a == "..." {
                f.max_args = -1;
                continue;
            }
            let mut def = None;
            let mut opt = false;
            if let Some(eq) = a.find('=') {
                def = Some(a[eq + 1..].to_string());
                a = a[..eq].to_string();
                opt = true;
            }
            if a.ends_with('?') {
                opt = true;
                a = a.trim_end_matches('?').to_string();
            }
            let nt: Vec<&str> = a.split(':').collect();
            let ty = if nt.len() > 1 { parse_type(nt[1]) } else { ArgType::Any };
            if ty == ArgType::Unevaluated {
                f.hold_args.insert(f.args.len());
            }
            f.args.push(ArgDef { name: nt[0].to_string(), ty, default: def });
            if !opt {
                min = f.args.len() as i32;
            }
            if f.max_args >= 0 {
                f.max_args = f.args.len() as i32;
            }
        }
    }
    f.min_args = min;
    f
}

#[allow(clippy::too_many_arguments)]
pub fn def(c: &Calculator, names: &str, cat: &str, title: &str, args: &str, desc: &str, ew: bool, imp: impl Fn(&Calculator, &[Expr]) -> R + Send + Sync + 'static) -> FnRef {
    c.add_function(build(names, cat, title, args, desc, ew, Some(Arc::new(imp))))
}

pub fn def_imp(c: &Calculator, names: &str, cat: &str, title: &str, args: &str, desc: &str, ew: bool, imp: FnImpl) -> FnRef {
    c.add_function(build(names, cat, title, args, desc, ew, Some(imp)))
}

pub fn is_n(e: &Expr) -> Option<&Cx> {
    e.as_num()
}
pub fn is_r(e: &Expr) -> Option<&Real> {
    e.as_real()
}
pub fn is_i(e: &Expr) -> Option<BigInt> {
    match &e.e {
        E::Num(n) if n.is_real() && n.re.is_int() => Some(n.re.n.clone()),
        _ => None,
    }
}
pub fn nr(r: Real) -> Expr {
    num_r(r)
}
pub fn nb(i: BigInt) -> Expr {
    num_r(Real::big_int(i))
}
pub fn nd(d: f64) -> Expr {
    num_r(Real::from_f64(d))
}
pub fn ncd(z: C64) -> Expr {
    num(Cx::from_c64(z.re, z.im).clean())
}
pub fn to_c(v: &Cx) -> C64 {
    let (re, im) = v.to_c64();
    c64(re, im)
}
pub fn bool_e(b: bool) -> Expr {
    evaluator::bool_e(b)
}

pub fn int(e: &Expr, f: &str) -> BigInt {
    is_i(e).unwrap_or_else(|| throw_s(format!("{}() requires integer arguments", f)))
}

pub fn small_int(e: &Expr, f: &str) -> i32 {
    int(e, f).to_i32().unwrap_or_else(|| throw_s(format!("{}() argument too large", f)))
}

pub fn dbl(e: &Expr, f: &str) -> f64 {
    is_r(e).map(|r| r.to_f64()).unwrap_or_else(|| throw_s(format!("{}() requires real arguments", f)))
}

pub fn str_of(e: &Expr) -> String {
    match &e.e {
        E::Str(s) => s.clone(),
        E::Sym(s) => s.clone(),
        E::Var(v) => v.name().to_string(),
        E::Unit(u, _) => u.name().to_string(),
        _ => formatter::plain_text(e),
    }
}

type ExactFn = fn(&Cx) -> R;

// numeric unary function: exact attempt first, then approximation outside the exact phase
pub fn u(exact: Option<ExactFn>, approx: fn(&Cx) -> Cx) -> FnImpl {
    Arc::new(move |c: &Calculator, a: &[Expr]| {
        let n = a[0].as_num()?;
        if let Some(ex) = exact {
            if !n.approx() {
                if let Some(r) = ex(n) {
                    return Some(r);
                }
            }
        }
        if c.exact() && !n.approx() {
            return None;
        }
        let res = approx(&if n.approx() { n.clone() } else { n.as_approx() });
        Some(num(res.clean()))
    })
}

pub fn rd(f: fn(f64) -> f64, exact: Option<ExactFn>) -> FnImpl {
    Arc::new(move |c: &Calculator, a: &[Expr]| {
        let n = a[0].as_num()?;
        if let Some(ex) = exact {
            if !n.approx() {
                if let Some(r) = ex(n) {
                    return Some(r);
                }
            }
        }
        if c.exact() && !n.approx() {
            return None;
        }
        if !n.is_real() {
            throw("Complex argument is not supported here");
        }
        Some(num_r(Real::from_f64(f(n.re.to_f64()))))
    })
}

pub fn r2(f: fn(f64, f64) -> f64) -> FnImpl {
    Arc::new(move |c: &Calculator, a: &[Expr]| r2_call(c, a, f))
}

pub fn r2_call(c: &Calculator, a: &[Expr], f: fn(f64, f64) -> f64) -> R {
    let x = is_r(&a[0])?;
    let y = is_r(&a[1])?;
    if c.exact() && !x.approx && !y.approx {
        return None;
    }
    Some(nd(f(x.to_f64(), y.to_f64())))
}

fn zero_if_zero(v: &Cx) -> R {
    if v.is_zero() {
        Some(zero())
    } else {
        None
    }
}
fn one_if_zero(v: &Cx) -> R {
    if v.is_zero() {
        Some(one())
    } else {
        None
    }
}

pub fn pi() -> Expr {
    let c = cur();
    c.var_ref(&c.find_variable("pi").unwrap_or_else(|| c.variable("pi").unwrap()))
}

fn pi_over(n: i64) -> Expr {
    simp::div(&pi(), &num_i(n))
}

// converts an argument in the default angle unit, or with an angle unit, to radians
pub fn to_radians(x: &Expr) -> Expr {
    if x.has_units() {
        let (cf, rest) = simp::split_coef(x);
        let (plain, units) = simp::split_units(&rest);
        if let E::Unit(u, None) = &units.e {
            let fac = match u.name() {
                "rad" => Some(one()),
                "°" => Some(pi_over(180)),
                "arcmin" => Some(pi_over(10800)),
                "arcsec" => Some(pi_over(648000)),
                "gon" => Some(pi_over(200)),
                "tr" => Some(simp::mul2(&two(), &pi())),
                _ => None,
            };
            if let Some(fac) = fac {
                return simp::mul(vec![num(cf), plain, fac]);
            }
        }
        if let Some((v, d, r2)) = units::try_quantity(x) {
            if d.same_as(&Dim::base(9)) {
                return simp::mul2(&num(v), &r2);
            }
        }
        throw("Invalid unit for an angle");
    }
    let angle = cur().opts().angle;
    match angle {
        AngleMode::Degrees => simp::mul2(x, &pi_over(180)),
        AngleMode::Gradians => simp::mul2(x, &pi_over(200)),
        _ => x.clone(),
    }
}

pub fn from_radians(r: &Expr) -> Expr {
    let angle = cur().opts().angle;
    match angle {
        AngleMode::Degrees => simp::mul2(r, &simp::div(&num_i(180), &pi())),
        AngleMode::Gradians => simp::mul2(r, &simp::div(&num_i(200), &pi())),
        _ => r.clone(),
    }
}

// rational multiple of pi, or None
fn pi_multiple(e: &Expr) -> Option<Real> {
    if e.is_zero() {
        return Some(Real::zero());
    }
    if e.is_var_named("pi") {
        return Some(Real::one());
    }
    if let E::Mul(f) = &e.e {
        if f.len() == 2 && f[1].is_var_named("pi") {
            if let E::Num(n) = &f[0].e {
                if n.is_rational() {
                    return Some(n.re.clone());
                }
            }
        }
    }
    None
}

static SIN_TABLE: LazyLock<Mutex<HashMap<i32, Expr>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn sin_twelfths(k: i32) -> Expr {
    let k = ((k % 24) + 24) % 24;
    if let Some(v) = SIN_TABLE.lock().unwrap().get(&k) {
        return v.clone();
    }
    const BASE: [&str; 7] = ["0", "(sqrt(6)-sqrt(2))/4", "1/2", "sqrt(2)/2", "sqrt(3)/2", "(sqrt(6)+sqrt(2))/4", "1"];
    let kk = k % 12;
    let idx = if kk <= 6 { kk } else { 12 - kk } as usize;
    let c = cur();
    let mut e = c.with_exact(true, || c.ev.eval(&parser::parse_req(BASE[idx], c)));
    if k >= 12 {
        e = simp::neg(&e);
    }
    SIN_TABLE.lock().unwrap().insert(k, e.clone());
    e
}

fn exact_trig(f: &str, rad: &Expr) -> R {
    let q = pi_multiple(rad)?;
    let t = q * Real::int(12);
    if !t.is_int() {
        return None;
    }
    let k: i32 = (&t.n % BigInt::from(24)).to_i32().unwrap();
    match f {
        "sin" => Some(sin_twelfths(k)),
        "cos" => Some(sin_twelfths(k + 6)),
        "tan" => {
            let s = sin_twelfths(k);
            let co = sin_twelfths(k + 6);
            if co.is_zero() {
                throw("tan() is undefined at this angle");
            }
            Some(simp::div(&s, &co))
        }
        _ => None,
    }
}

pub fn exact_trig_public(f: &str, rad: &Expr) -> R {
    exact_trig(f, rad)
}

fn exact_inverse_trig(f: &str, v: &Expr) -> R {
    let (lo, hi) = match f {
        "asin" => (-6, 6),
        "acos" => (0, 12),
        "atan" => (-5, 5),
        _ => return None,
    };
    for k in lo..=hi {
        let val = match f {
            "asin" => sin_twelfths(k),
            "acos" => sin_twelfths(k + 6),
            _ => simp::div(&sin_twelfths(k), &sin_twelfths(k + 6)),
        };
        if val.key() == v.key() {
            return Some(from_radians(&simp::mul2(&num_r(Real::frac(k as i64, 12)), &pi())));
        }
    }
    None
}

fn trig(name: &'static str, f: fn(&Cx) -> Cx) -> FnImpl {
    Arc::new(move |c: &Calculator, a: &[Expr]| {
        let rad = to_radians(&a[0]);
        if let Some(ex) = exact_trig(name, &rad) {
            return Some(ex);
        }
        if let E::Num(n) = &rad.e {
            if c.exact() && !n.approx() {
                return None;
            }
            return Some(num(f(&n.as_approx()).clean()));
        }
        if !c.exact() && !rad.has_symbols() {
            let r2 = c.ev.eval(&rad);
            if let E::Num(n2) = &r2.e {
                return Some(num(f(&n2.as_approx()).clean()));
            }
        }
        if rad.key() != a[0].key() && !rad.is_num() {
            return Some(call_e(c.fn_(name), vec![a[0].clone()]));
        }
        None
    })
}

fn inv_trig(name: &'static str, f: fn(&Cx) -> Cx) -> FnImpl {
    Arc::new(move |c: &Calculator, a: &[Expr]| {
        if let Some(ex) = exact_inverse_trig(name, &a[0]) {
            return Some(ex);
        }
        let n = a[0].as_num()?;
        if c.exact() && !n.approx() {
            return None;
        }
        let r = f(&n.as_approx()).clean();
        Some(c.ev.eval(&from_radians(&num(r))))
    })
}

fn reciprocal(c: &Calculator, f: &str, x: &Expr) -> R {
    let v = c.ev.call(f, vec![x.clone()]);
    if v.is_call() {
        return None;
    }
    if v.is_zero() {
        throw("Division by zero");
    }
    Some(simp::pow(&v, &minus_one()))
}

// calls f and returns None when the result stays an unevaluated call
fn call_or_none(c: &Calculator, f: &str, args: Vec<Expr>) -> R {
    let r = c.ev.call(f, args);
    if r.is_call() {
        None
    } else {
        Some(r)
    }
}

pub fn register(c: &Calculator) {
    register_math(c);
    funcs2::register_more(c);
}

fn register_math(c: &Calculator) {
    const TR: &str = "Trigonometry";
    const EL: &str = "Exponents & Logarithms";
    const AR: &str = "Arithmetics";
    const RO: &str = "Rounding";
    const INT: &str = "Integers";
    const PR: &str = "Prime Numbers";
    const SP: &str = "Special Functions";
    const CX: &str = "Complex Numbers";
    const CB: &str = "Combinatorics";
    const ST: &str = "Step Functions";

    def_imp(c, "sin", TR, "Sine", "x:angle", "Returns the sine of an angle.", true, trig("sin", Cx::sin));
    def_imp(c, "cos", TR, "Cosine", "x:angle", "Returns the cosine of an angle.", true, trig("cos", Cx::cos));
    def_imp(c, "tan", TR, "Tangent", "x:angle", "Returns the tangent of an angle.", true, trig("tan", Cx::tan));
    def(c, "cot", TR, "Cotangent", "x:angle", "", true, |cc, a| reciprocal(cc, "tan", &a[0]));
    def(c, "sec", TR, "Secant", "x:angle", "", true, |cc, a| reciprocal(cc, "cos", &a[0]));
    def(c, "csc", TR, "Cosecant", "x:angle", "", true, |cc, a| reciprocal(cc, "sin", &a[0]));
    def_imp(c, "asin,arcsin", TR, "Arcsine", "x:num", "Returns the angle whose sine is x.", true, inv_trig("asin", Cx::asin));
    def_imp(c, "acos,arccos", TR, "Arccosine", "x:num", "", true, inv_trig("acos", Cx::acos));
    def_imp(c, "atan,arctan", TR, "Arctangent", "x:num", "", true, inv_trig("atan", Cx::atan));
    def(c, "acot,arccot", TR, "Arccotangent", "x:num", "", true, |cc, a| {
        Some(if a[0].is_zero() { cc.ev.eval(&from_radians(&simp::div(&pi(), &two()))) } else { cc.ev.call("atan", vec![simp::pow(&a[0], &minus_one())]) })
    });
    def(c, "asec,arcsec_f", TR, "Arcsecant", "x:num", "", true, |cc, a| Some(cc.ev.call("acos", vec![simp::pow(&a[0], &minus_one())])));
    def(c, "acsc,arccsc", TR, "Arccosecant", "x:num", "", true, |cc, a| Some(cc.ev.call("asin", vec![simp::pow(&a[0], &minus_one())])));
    def_imp(c, "sinh", TR, "Hyperbolic Sine", "x:num", "", true, u(Some(zero_if_zero), Cx::sinh));
    def_imp(c, "cosh", TR, "Hyperbolic Cosine", "x:num", "", true, u(Some(one_if_zero), Cx::cosh));
    def_imp(c, "tanh", TR, "Hyperbolic Tangent", "x:num", "", true, u(Some(zero_if_zero), Cx::tanh));
    def(c, "coth", TR, "Hyperbolic Cotangent", "x:num", "", true, |cc, a| reciprocal(cc, "tanh", &a[0]));
    def(c, "sech", TR, "Hyperbolic Secant", "x:num", "", true, |cc, a| reciprocal(cc, "cosh", &a[0]));
    def(c, "csch", TR, "Hyperbolic Cosecant", "x:num", "", true, |cc, a| reciprocal(cc, "sinh", &a[0]));
    def_imp(c, "asinh,arsinh", TR, "Inverse Hyperbolic Sine", "x:num", "", true, u(Some(zero_if_zero), Cx::asinh));
    def_imp(c, "acosh,arcosh", TR, "Inverse Hyperbolic Cosine", "x:num", "", true, u(Some(|v| if v.is_one() { Some(zero()) } else { None }), Cx::acosh));
    def_imp(c, "atanh,artanh", TR, "Inverse Hyperbolic Tangent", "x:num", "", true, u(Some(zero_if_zero), Cx::atanh));
    def(c, "acoth", TR, "Inverse Hyperbolic Cotangent", "x:num", "", true, |cc, a| Some(cc.ev.call("atanh", vec![simp::pow(&a[0], &minus_one())])));
    def(c, "asech", TR, "Inverse Hyperbolic Secant", "x:num", "", true, |cc, a| Some(cc.ev.call("acosh", vec![simp::pow(&a[0], &minus_one())])));
    def(c, "acsch", TR, "Inverse Hyperbolic Cosecant", "x:num", "", true, |cc, a| Some(cc.ev.call("asinh", vec![simp::pow(&a[0], &minus_one())])));
    def(c, "atan2", TR, "Four-quadrant Arctangent", "y:real,x:real", "", false, |cc, a| {
        let y = is_r(&a[0])?;
        let x = is_r(&a[1])?;
        if x.sign() > 0 && cc.exact() && !x.approx && !y.approx {
            return Some(cc.ev.call("atan", vec![nr(y / x)]));
        }
        if x.is_zero() && cc.exact() {
            return Some(if y.sign() == 0 { zero() } else { cc.ev.eval(&from_radians(&simp::mul2(&num_r(Real::frac(y.sign() as i64, 2)), &pi()))) });
        }
        if cc.exact() && !x.approx && !y.approx {
            return None;
        }
        Some(cc.ev.eval(&from_radians(&nr(rmath::atan2(&y.as_approx(), &x.as_approx())))))
    });
    def(c, "sinc", TR, "Cardinal Sine", "x:num", "sin(x)/x, unnormalized.", true, |cc, a| {
        if a[0].is_zero() {
            return Some(one());
        }
        let s = call_or_none(cc, "sin", vec![a[0].clone()])?;
        Some(simp::div(&s, &to_radians(&a[0])))
    });
    def(c, "hypot", TR, "Hypotenuse", "a:num,b:num,...", "Square root of the sum of squares.", false, |_, a| Some(simp::pow(&simp::add(a.iter().map(|x| simp::pow(x, &two())).collect()), &half())));
    def(c, "deg,degrees", TR, "Radians to Degrees", "x:num", "", true, |_, a| Some(simp::mul2(&a[0], &simp::div(&num_i(180), &pi()))));
    def(c, "radians", TR, "Degrees to Radians", "x:num", "", true, |_, a| Some(simp::mul2(&a[0], &pi_over(180))));

    def(c, "exp", EL, "Exponential", "x:num", "e raised to the power of x.", true, |cc, a| {
        if let E::Call(f, la) = &a[0].e {
            if f.name() == "ln" {
                return Some(la[0].clone());
            }
        }
        Some(simp::pow(&cc.var_ref(&cc.variable("e").unwrap()), &a[0]))
    });
    def(c, "exp2", EL, "Base-2 Exponential", "x:num", "", true, |_, a| Some(simp::pow(&two(), &a[0])));
    def(c, "exp10", EL, "Base-10 Exponential", "x:num", "", true, |_, a| Some(simp::pow(&num_i(10), &a[0])));
    def_imp(c, "expm1", EL, "exp(x) - 1", "x:num", "", true, u(Some(zero_if_zero), |v| Cx::exp(v) - Cx::one()));
    def(c, "ln", EL, "Natural Logarithm", "x:num", "Natural logarithm of x.", true, |cc, a| {
        let x = &a[0];
        if x.is_one() {
            return Some(zero());
        }
        if x.is_var_named("e") {
            return Some(one());
        }
        if let E::Pow(b, px) = &x.e {
            if b.is_var_named("e") && cc.is_known_real(px) {
                return Some(px.clone());
            }
        }
        if let E::Num(n) = &x.e {
            if n.is_zero() {
                throw("Logarithm of zero");
            }
            if n.is_rational() && n.re.sign() > 0 && !n.re.d.is_one() && n.re.n.is_one() && cc.exact() {
                return Some(simp::neg(&cc.ev.call("ln", vec![nr(Real::big_int(n.re.d.clone()))])));
            }
            if cc.exact() && !n.approx() {
                return None;
            }
            return Some(num(Cx::ln(&n.as_approx()).clean()));
        }
        None
    });
    def(c, "log", EL, "Logarithm", "x:num,base=e", "Logarithm of x with the given base.", true, |cc, a| {
        let b = &a[1];
        if b.is_var_named("e") {
            return Some(cc.ev.call("ln", vec![a[0].clone()]));
        }
        if let (Some(x), Some(br)) = (is_r(&a[0]), is_r(b)) {
            if !x.approx && !br.approx && x.sign() > 0 && br.sign() > 0 {
                if let Some(ex) = exact_log(x, br) {
                    return Some(nr(ex));
                }
                if cc.exact() {
                    return None;
                }
            }
        }
        let ln1 = cc.ev.call("ln", vec![a[0].clone()]);
        let ln2 = cc.ev.call("ln", vec![b.clone()]);
        if ln1.is_call() || ln2.is_call() {
            return None;
        }
        Some(simp::div(&ln1, &ln2))
    });
    def(c, "log2", EL, "Base-2 Logarithm", "x:num", "", true, |cc, a| call_or_none(cc, "log", vec![a[0].clone(), two()]));
    def(c, "log10,lg", EL, "Base-10 Logarithm", "x:num", "", true, |cc, a| call_or_none(cc, "log", vec![a[0].clone(), num_i(10)]));
    def(c, "logn", EL, "Logarithm with Base", "x:num,base:num", "", true, |cc, a| call_or_none(cc, "log", vec![a[0].clone(), a[1].clone()]));
    def_imp(c, "log1p", EL, "ln(1 + x)", "x:num", "", true, u(Some(zero_if_zero), |v| Cx::ln(&(Cx::one() + v))));
    def(c, "sqrt", EL, "Square Root", "x:num", "Principal square root.", true, |_, a| Some(simp::pow(&a[0], &half())));
    def(c, "cbrt", EL, "Cube Root", "x:num", "Real cube root.", true, |cc, a| {
        if is_r(&a[0]).map(|r| r.sign() < 0).unwrap_or(false) {
            return Some(with_real_roots(cc, || simp::pow(&a[0], &num_r(Real::frac(1, 3)))));
        }
        Some(simp::pow(&a[0], &num_r(Real::frac(1, 3))))
    });
    def(c, "root", EL, "Nth Root", "x:num,n:num", "Real nth root of x.", false, |cc, a| {
        if is_r(&a[1]).map(|n| n.is_zero()).unwrap_or(false) {
            throw("root() of degree zero");
        }
        Some(with_real_roots(cc, || simp::pow(&a[0], &simp::pow(&a[1], &minus_one()))))
    });
    def(c, "sq", EL, "Square", "x", "", true, |_, a| Some(simp::pow(&a[0], &two())));
    def(c, "pow", EL, "Power", "x,y", "", false, |_, a| Some(simp::pow(&a[0], &a[1])));
    def(c, "lambertw,W", EL, "Lambert W", "x:num,branch:int=0", "Inverse of x·e^x.", false, |cc, a| {
        if a[0].is_zero() && (a.len() < 2 || a[1].is_zero()) {
            return Some(zero());
        }
        let n = a[0].as_num()?;
        if cc.exact() && !n.approx() {
            return None;
        }
        let br = if a.len() > 1 { small_int(&a[1], "lambertw") } else { 0 };
        Some(ncd(sp::lambert_w(to_c(n), br)))
    });
    def(c, "powertower", EL, "Power Tower", "x:num,n:nnint", "", false, |_, a| {
        let n = small_int(&a[1], "powertower");
        if n > 100 {
            throw("powertower() height too large");
        }
        let mut r = one();
        for _ in 0..n {
            r = simp::pow(&a[0], &r);
        }
        Some(r)
    });
    def(c, "cis", CX, "cis", "x:angle", "cos(x) + i sin(x).", true, |cc, a| {
        let cs = call_or_none(cc, "cos", vec![a[0].clone()])?;
        let sn = call_or_none(cc, "sin", vec![a[0].clone()])?;
        Some(simp::add2(&cs, &simp::mul2(&num(Cx::i()), &sn)))
    });

    def(c, "abs", AR, "Absolute Value", "x", "Absolute value or modulus.", true, |cc, a| {
        let x = &a[0];
        match &x.e {
            E::Num(n) => return Some(num(if n.is_real() { Cx::real(n.re.abs()) } else { Cx::real(n.abs()) })),
            E::Intv(lo, hi) => {
                return Some(if lo.sign() >= 0 {
                    x.clone()
                } else if hi.sign() <= 0 {
                    simp::neg(x)
                } else {
                    intv_e(Real::zero(), Real::max(hi, &-lo))
                })
            }
            _ => {}
        }
        if cc.is_known_non_negative(x) {
            return Some(x.clone());
        }
        if let E::Mul(m) = &x.e {
            if let E::Num(f) = &m[0].e {
                return Some(simp::mul2(&num(Cx::real(f.abs())), &cc.ev.call("abs", vec![simp::mul(m[1..].to_vec())])));
            }
        }
        if x.has_units() {
            if let Some((qv, _, rest)) = units::try_quantity(x) {
                if rest.is_one() && qv.is_real() {
                    let (cf, un) = simp::split_coef(x);
                    return Some(simp::mul2(&num(Cx::real(cf.re.abs())), &un));
                }
            }
        }
        if matches!(x.e, E::Var(_)) {
            return Some(x.clone());
        }
        None
    });
    def(c, "sgn,signum,sign", AR, "Signum", "x", "", true, |cc, a| {
        if let E::Num(n) = &a[0].e {
            if n.is_real() {
                return Some(nr(Real::int(n.re.sign() as i64)));
            }
            if cc.exact() && !n.approx() {
                return None;
            }
            return Some(num(n / &Cx::real(n.abs())));
        }
        if cc.is_known_positive(&a[0]) {
            return Some(one());
        }
        None
    });
    def(c, "ceil,ceiling", RO, "Round Upwards", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r.ceil()));
    def(c, "floor", RO, "Round Downwards", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r.floor()));
    def(c, "trunc", RO, "Round Towards Zero", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r.trunc()));
    def(c, "int", RO, "Integer Part", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r.trunc()));
    def(c, "frac", RO, "Fractional Part", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r - &r.trunc()));
    def(c, "round", RO, "Round", "x,decimals:int=0", "Rounds to the nearest integer or to the given number of decimals.", true, |cc, a| {
        let d = if a.len() > 1 { small_int(&a[1], "round") } else { 0 };
        let scale = Real::int(10).pow_i(d as i64);
        let mode = cc.opts().rounding;
        round_fn(cc, &a[0], &|r| (r * &scale).round(mode) / &scale)
    });
    def(c, "roundhalfeven", RO, "Round Half to Even", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r.round(RoundingMode::HalfEven)));
    def(c, "roundhalfup", RO, "Round Half Up", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r.round(RoundingMode::HalfUp)));
    def(c, "roundhalfdown", RO, "Round Half Down", "x", "", true, |cc, a| round_fn(cc, &a[0], &|r| r.round(RoundingMode::HalfDown)));
    def(c, "mod", AR, "Modulus (floored)", "a:real,b:real", "Remainder with the sign of the divisor.", false, |cc, a| {
        let (Some(x), Some(y)) = (is_r(&a[0]), is_r(&a[1])) else { return quantities2(cc, a, "mod") };
        if y.is_zero() {
            throw("Division by zero in mod()");
        }
        Some(nr(x - &(y * &(x / y).floor())))
    });
    def(c, "rem", AR, "Remainder (truncated)", "a:real,b:real", "Remainder with the sign of the dividend.", false, |cc, a| {
        let (Some(x), Some(y)) = (is_r(&a[0]), is_r(&a[1])) else { return quantities2(cc, a, "rem") };
        if y.is_zero() {
            throw("Division by zero in rem()");
        }
        Some(nr(x - &(y * &(x / y).trunc())))
    });
    def(c, "div,quotient", AR, "Integer Division", "a:real,b:real", "", false, |_, a| {
        let x = is_r(&a[0])?;
        let y = is_r(&a[1])?;
        if y.is_zero() {
            throw("Division by zero");
        }
        Some(nr((x / y).floor()))
    });
    def(c, "gcd", INT, "Greatest Common Divisor", "a,b,...", "", false, |_, a| {
        let items = flatten(a);
        if items.iter().all(|x| matches!(&x.e, E::Num(n) if n.is_rational())) {
            let mut g = Real::zero();
            for x in &items {
                g = rat_gcd(&g, &x.num().re);
            }
            return Some(nr(g));
        }
        None
    });
    def(c, "lcm", INT, "Least Common Multiple", "a,b,...", "", false, |_, a| {
        let items = flatten(a);
        if !items.iter().all(|x| is_i(x).is_some()) {
            return None;
        }
        let mut l = BigInt::one();
        for x in &items {
            let v = x.num().re.n.abs();
            if v.is_zero() {
                return Some(zero());
            }
            l = &l / l.gcd(&v) * v;
        }
        Some(nb(l))
    });
    def(c, "numerator,numer", AR, "Numerator", "x", "", true, |_, a| match is_r(&a[0]) {
        Some(r) if !r.approx => Some(nb(r.n.clone())),
        _ => num_den_part(&a[0], true),
    });
    def(c, "denominator,denom", AR, "Denominator", "x", "", true, |_, a| match is_r(&a[0]) {
        Some(r) if !r.approx => Some(nb(r.d.clone())),
        _ => num_den_part(&a[0], false),
    });

    def(c, "factorial", CB, "Factorial", "n", "n! = 1 × 2 × … × n", true, |cc, a| {
        if let Some(n) = is_i(&a[0]) {
            if !a[0].num().approx() {
                if n.is_negative() {
                    throw("Factorial of negative integer is undefined");
                }
                if n <= big(100000) {
                    return Some(nb(primes::factorial(&n, 1)));
                }
            }
        }
        let v = is_n(&a[0])?;
        if cc.exact() && !v.approx() && v.is_rational() && v.re.d == big(2) {
            return half_factorial(&v.re);
        }
        if cc.exact() && !v.approx() {
            return None;
        }
        if v.is_real() {
            return Some(nr(log_gamma_real(&(&v.re + &Real::one()))));
        }
        Some(ncd(sp::gamma_c(to_c(v) + 1.0)))
    });
    def(c, "factorial2,doublefactorial", CB, "Double Factorial", "n:int", "n!! = n × (n−2) × …", true, |_, a| {
        let n = int(&a[0], "factorial2");
        if n < big(-1) {
            throw("Double factorial undefined");
        }
        if !n.is_positive() {
            return Some(one());
        }
        Some(nb(primes::factorial(&n, 2)))
    });
    def(c, "multifactorial", CB, "Multifactorial", "n:nnint,k:pint", "", false, |_, a| Some(nb(primes::factorial(&int(&a[0], "multifactorial"), small_int(&a[1], "multifactorial")))));
    def(c, "binomial,choose,comb,nCr", CB, "Binomial Coefficient", "n,k:int", "Number of ways to choose k items from n.", false, |cc, a| {
        if let (Some(n), Some(k)) = (is_i(&a[0]), is_i(&a[1])) {
            return Some(nb(primes::binomial(&n, &k)));
        }
        if let (Some(nv), Some(kv)) = (is_n(&a[0]), is_n(&a[1])) {
            if cc.exact() && !nv.approx() {
                return None;
            }
            let (n, k) = (nv.re.to_f64(), kv.re.to_f64());
            return Some(nd(sp::gamma(n + 1.0) / (sp::gamma(k + 1.0) * sp::gamma(n - k + 1.0))));
        }
        if let Some(k2) = is_i(&a[1]) {
            if !k2.is_negative() && k2 < big(50) {
                let mut r = one();
                for i in 0..k2.to_i64().unwrap() {
                    r = simp::mul2(&r, &simp::sub(&a[0], &num_i(i)));
                }
                return Some(simp::div(&r, &nb(primes::factorial(&k2, 1))));
            }
        }
        None
    });
    def(c, "perm,nPr", CB, "Permutations", "n:int,k:int", "n!/(n−k)!", false, |_, a| {
        let n = int(&a[0], "perm");
        let k = int(&a[1], "perm");
        if k.is_negative() || k > n {
            return Some(zero());
        }
        Some(nb(primes::factorial(&n, 1) / primes::factorial(&(&n - &k), 1)))
    });
    def(c, "derangements,subfactorial", CB, "Derangements", "n:nnint", "", true, |_, a| Some(nb(primes::subfactorial(&int(&a[0], "derangements")))));
    def(c, "stirling2", CB, "Stirling Number of the Second Kind", "n:nnint,k:nnint", "", false, |_, a| Some(nb(primes::stirling2(small_int(&a[0], "stirling2") as i64, small_int(&a[1], "stirling2") as i64))));
    def(c, "catalan_number,catalannumber", CB, "Catalan Number", "n:nnint", "", true, |_, a| {
        let n = int(&a[0], "catalan_number");
        Some(nb(primes::binomial(&(&n * 2), &n) / (&n + 1)))
    });
    def(c, "partitions", CB, "Integer Partitions", "n:nnint", "", true, |_, a| Some(nb(primes::partitions(small_int(&a[0], "partitions") as i64))));
    def(c, "bell", CB, "Bell Number", "n:nnint", "", true, |_, a| {
        let n = small_int(&a[0], "bell");
        if n > 2000 {
            throw("bell() argument too large");
        }
        let mut row = vec![BigInt::one()];
        for _ in 0..n {
            let mut next = vec![row.last().unwrap().clone()];
            for v in &row {
                let t = next.last().unwrap() + v;
                next.push(t);
            }
            row = next;
        }
        Some(nb(row[0].clone()))
    });

    def(c, "isprime", PR, "Is Prime", "n:int", "", true, |_, a| Some(bool_e(primes::is_prime(&int(&a[0], "isprime")))));
    def(c, "nextprime", PR, "Next Prime", "n:int", "Smallest prime greater than n.", true, |_, a| Some(nb(primes::next_prime(&int(&a[0], "nextprime")))));
    def(c, "prevprime", PR, "Previous Prime", "n:int", "Largest prime less than n.", true, |_, a| Some(nb(primes::prev_prime(&int(&a[0], "prevprime")))));
    def(c, "nthprime,prime", PR, "Nth Prime", "n:pint", "", true, |_, a| Some(nb(primes::nth_prime(int(&a[0], "nthprime").to_i64().unwrap_or(i64::MAX)))));
    def(c, "primepi,primecount", PR, "Prime Counting Function", "x:real", "Number of primes ≤ x.", true, |_, a| is_r(&a[0]).map(|r| nr(Real::int(primes::prime_count(&r.floor().to_big())))));
    def(c, "primes", PR, "Primes", "n:int", "Vector of all primes ≤ n.", false, |_, a| Some(vector_e(primes::primes_up_to(int(&a[0], "primes").to_i64().unwrap_or(i64::MAX)).into_iter().map(nb).collect())));
    def(c, "factors,primefactors", PR, "Prime Factors", "n:int", "Vector of prime factors.", false, |_, a| {
        let f = primes::factorize(&int(&a[0], "factors"));
        let mut l = vec![];
        for (k, v) in f {
            for _ in 0..v {
                l.push(nb(k.clone()));
            }
        }
        Some(vector_e(l))
    });
    def(c, "divisors", INT, "Divisors", "n:int", "Vector of positive divisors.", false, |_, a| Some(vector_e(primes::divisors(&int(&a[0], "divisors")).into_iter().map(nb).collect())));
    def(c, "totient,eulerphi", INT, "Euler's Totient", "n:pint", "", true, |_, a| Some(nb(primes::totient(&int(&a[0], "totient")))));
    def(c, "sigma,divisorsum", INT, "Divisor Function", "n:pint,k:int=1", "", false, |_, a| {
        let k = small_int(&a[1], "sigma");
        let mut s = BigInt::zero();
        for d in primes::divisors(&int(&a[0], "sigma")) {
            s += num_traits::pow(d, k.max(0) as usize);
        }
        Some(nb(s))
    });
    def(c, "even,iseven", INT, "Is Even", "n", "", true, |_, a| match is_i(&a[0]) {
        Some(n) => Some(bool_e(n.is_even())),
        None => a[0].is_num().then(zero),
    });
    def(c, "odd,isodd", INT, "Is Odd", "n", "", true, |_, a| match is_i(&a[0]) {
        Some(n) => Some(bool_e(!n.is_even())),
        None => a[0].is_num().then(zero),
    });
    def(c, "fibonacci,fib", INT, "Fibonacci Number", "n:int", "", true, |_, a| Some(nb(primes::fibonacci(&int(&a[0], "fibonacci")))));
    def(c, "lucas", INT, "Lucas Number", "n:nnint", "", true, |_, a| {
        let n = int(&a[0], "lucas");
        Some(nb(primes::fibonacci(&(&n - 1)) + primes::fibonacci(&(&n + 1))))
    });
    def(c, "bernoulli", INT, "Bernoulli Number", "n:nnint", "", true, |_, a| Some(nr(primes::bernoulli(small_int(&a[0], "bernoulli") as i64))));
    def(c, "harmonic", INT, "Harmonic Number", "n:nnint", "", true, |_, a| {
        let n = small_int(&a[0], "harmonic");
        if n > 100000 {
            throw("harmonic() argument too large");
        }
        let mut s = Real::zero();
        for i in 1..=n {
            s = s + Real::frac(1, i as i64);
        }
        Some(nr(s))
    });
    def(c, "digitsum,sumdigits", INT, "Digit Sum", "n:int,base:int=10", "", false, |_, a| {
        let mut n = int(&a[0], "digitsum").abs();
        let b = small_int(&a[1], "digitsum");
        if b < 2 {
            throw("digitsum() base must be at least 2");
        }
        let mut s = BigInt::zero();
        while !n.is_zero() {
            s += &n % b;
            n /= b;
        }
        Some(nb(s))
    });
    def(c, "powmod", INT, "Modular Exponentiation", "a:int,b:int,m:int", "", false, |_, a| {
        let b = int(&a[1], "powmod");
        let m = int(&a[2], "powmod");
        if m.is_zero() {
            throw("Division by zero");
        }
        if b.is_negative() {
            return Some(nb(primes::mod_inverse(&int(&a[0], "powmod"), &m).modpow(&-b, &m)));
        }
        let mut r = int(&a[0], "powmod").modpow(&b, &m);
        if r.is_negative() {
            r += &m;
        }
        Some(nb(r))
    });
    def(c, "modinv", INT, "Modular Inverse", "a:int,m:int", "", false, |_, a| Some(nb(primes::mod_inverse(&int(&a[0], "modinv"), &int(&a[1], "modinv")))));
    def(c, "isinteger", INT, "Is Integer", "x", "", false, |cc, a| match &a[0].e {
        E::Num(n) => Some(bool_e(n.is_int())),
        _ => cc.is_known_integer(&a[0]).then(one),
    });
    def(c, "isrational", INT, "Is Rational", "x", "", false, |_, a| a[0].as_num().map(|n| bool_e(n.is_rational())));
    def(c, "isreal", INT, "Is Real", "x", "", false, |cc, a| match &a[0].e {
        E::Num(n) => Some(bool_e(n.is_real())),
        _ => cc.is_known_real(&a[0]).then(one),
    });
    def(c, "isnumber", INT, "Is Number", "x", "", false, |_, a| Some(bool_e(a[0].is_num())));

    def(c, "re,real", CX, "Real Part", "z", "", true, |cc, a| match &a[0].e {
        E::Num(n) => Some(nr(n.re.clone())),
        _ => cc.is_known_real(&a[0]).then(|| a[0].clone()),
    });
    def(c, "im,imag", CX, "Imaginary Part", "z", "", true, |cc, a| match &a[0].e {
        E::Num(n) => Some(nr(n.im.clone())),
        _ => cc.is_known_real(&a[0]).then(zero),
    });
    def(c, "arg,angle", CX, "Argument", "z", "Angle of a complex number.", true, |cc, a| {
        let n = a[0].as_num()?;
        if n.is_real() {
            return Some(if n.re.sign() >= 0 { zero() } else { cc.ev.eval(&from_radians(&pi())) });
        }
        if n.re.is_zero() {
            return Some(cc.ev.eval(&from_radians(&simp::mul2(&num_r(Real::frac(n.im.sign() as i64, 2)), &pi()))));
        }
        if cc.exact() && !n.approx() {
            return call_or_none(cc, "atan2", vec![nr(n.im.clone()), nr(n.re.clone())]);
        }
        Some(cc.ev.eval(&from_radians(&nr(n.as_approx().arg()))))
    });
    def(c, "conj,conjugate", CX, "Complex Conjugate", "z", "", true, |cc, a| match &a[0].e {
        E::Num(n) => Some(num(n.conj())),
        _ => cc.is_known_real(&a[0]).then(|| a[0].clone()),
    });
    def(c, "norm2,abs2", CX, "Squared Modulus", "z", "", true, |_, a| a[0].as_num().map(|n| nr(&n.re * &n.re + &n.im * &n.im)));

    def(c, "gamma,Γ", SP, "Gamma Function", "x:num", "Γ(x) = (x−1)!", true, |cc, a| {
        if let Some(n) = is_i(&a[0]) {
            if !a[0].num().approx() {
                if !n.is_positive() {
                    throw("gamma() is undefined for non-positive integers");
                }
                return Some(nb(primes::factorial(&(n - 1), 1)));
            }
        }
        let v = is_n(&a[0])?;
        if cc.exact() && !v.approx() && v.is_rational() && v.re.d == big(2) {
            return half_factorial(&(&v.re - &Real::one()));
        }
        if cc.exact() && !v.approx() {
            return None;
        }
        if v.is_real() {
            return Some(nr(log_gamma_real(&v.re)));
        }
        Some(ncd(sp::gamma_c(to_c(v))))
    });
    def_imp(c, "lgamma,loggamma", SP, "Log Gamma", "x:real", "", true, rd(sp::log_gamma, None));
    def_imp(c, "digamma,psi", SP, "Digamma Function", "x:real", "", true, rd(sp::digamma, None));
    def(c, "beta", SP, "Beta Function", "a:num,b:num", "", false, |cc, a| {
        if let (Some(x), Some(y)) = (is_i(&a[0]), is_i(&a[1])) {
            if x.is_positive() && y.is_positive() && &x + &y < big(10000) {
                return Some(nr(Real::exact(primes::factorial(&(&x - 1), 1) * primes::factorial(&(&y - 1), 1), primes::factorial(&(&x + &y - 1), 1))));
            }
        }
        r2_call(cc, a, sp::beta)
    });
    def(c, "betainc", SP, "Regularized Incomplete Beta", "x:real,a:real,b:real", "", false, |cc, a| {
        is_r(&a[0])?;
        if cc.exact() && !a.iter().any(|z| z.is_approx()) {
            return None;
        }
        Some(nd(sp::beta_inc(dbl(&a[0], "betainc"), dbl(&a[1], "betainc"), dbl(&a[2], "betainc"))))
    });
    def_imp(c, "erf", SP, "Error Function", "x:real", "", true, rd(sp::erf, Some(zero_if_zero)));
    def_imp(c, "erfc", SP, "Complementary Error Function", "x:real", "", true, rd(sp::erfc, Some(one_if_zero)));
    def_imp(c, "erfinv", SP, "Inverse Error Function", "x:real", "", true, rd(sp::erf_inv, Some(zero_if_zero)));
    def(c, "zeta,ζ", SP, "Riemann Zeta", "s:num", "", true, |cc, a| {
        if let Some(n) = is_i(&a[0]) {
            if !a[0].num().approx() {
                if n.is_zero() {
                    return Some(num_r(Real::frac(-1, 2)));
                }
                if n.is_negative() && n.is_even() {
                    return Some(zero());
                }
                if n.is_negative() {
                    let m: i64 = (BigInt::one() - &n).to_i64().unwrap_or(i64::MAX);
                    return Some(nr(-primes::bernoulli(m) / Real::int(m)));
                }
                if n.is_even() && n < big(200) {
                    let k = n.to_i64().unwrap();
                    let b = primes::bernoulli(k).abs();
                    let coef = Real::two().pow_i(k - 1) * b / Real::big_int(primes::factorial(&big(k), 1));
                    return Some(simp::mul2(&nr(coef), &simp::pow(&pi(), &nr(Real::int(k)))));
                }
                if n.is_one() {
                    throw("zeta(1) is undefined");
                }
            }
        }
        let v = is_n(&a[0])?;
        if cc.exact() && !v.approx() {
            return None;
        }
        Some(ncd(sp::zeta(to_c(v))))
    });
    def(c, "polylog,Li", SP, "Polylogarithm", "s:num,z:num", "", false, |cc, a| {
        if a[1].is_zero() {
            return Some(zero());
        }
        let s = is_n(&a[0])?;
        let z = is_n(&a[1])?;
        if cc.exact() && !s.approx() && !z.approx() {
            return None;
        }
        Some(ncd(sp::polylog(to_c(s), to_c(z))))
    });
    def_imp(c, "Ei,expint", SP, "Exponential Integral", "x:real", "", true, rd(sp::ei, None));
    def_imp(c, "E1", SP, "Exponential Integral E1", "x:real", "", true, rd(sp::e1, None));
    def_imp(c, "li,logint", SP, "Logarithmic Integral", "x:real", "", true, rd(sp::li, Some(zero_if_zero)));
    def_imp(c, "Si,sinint", SP, "Sine Integral", "x:real", "", true, rd(|x| sp::si_ci(x).0, Some(zero_if_zero)));
    def_imp(c, "Ci,cosint", SP, "Cosine Integral", "x:real", "", true, rd(|x| sp::si_ci(x).1, None));
    def_imp(c, "Shi,sinhint", SP, "Hyperbolic Sine Integral", "x:real", "", true, rd(sp::shi, Some(zero_if_zero)));
    def_imp(c, "Chi,coshint", SP, "Hyperbolic Cosine Integral", "x:real", "", true, rd(sp::chi, None));
    def(c, "besselj", SP, "Bessel Function of the First Kind", "n:int,x:real", "", false, |cc, a| {
        let x = is_r(&a[1])?;
        if cc.exact() && !x.approx && !x.is_zero() {
            return None;
        }
        Some(nd(sp::bessel_j(small_int(&a[0], "besselj"), x.to_f64())))
    });
    def(c, "bessely", SP, "Bessel Function of the Second Kind", "n:int,x:real", "", false, |cc, a| {
        let x = is_r(&a[1])?;
        if cc.exact() && !x.approx {
            return None;
        }
        Some(nd(sp::bessel_y(small_int(&a[0], "bessely"), x.to_f64())))
    });
    def_imp(c, "airy,airyai", SP, "Airy Function", "x:real", "", true, rd(sp::airy_ai, None));
    def_imp(c, "fresnels", SP, "Fresnel Integral S", "x:real", "", true, rd(|x| sp::fresnel(x).0, Some(zero_if_zero)));
    def_imp(c, "fresnelc", SP, "Fresnel Integral C", "x:real", "", true, rd(|x| sp::fresnel(x).1, Some(zero_if_zero)));
    def_imp(c, "igamma,uppergamma", SP, "Upper Incomplete Gamma", "a:real,x:real", "", false, r2(sp::upper_inc_gamma));
    def_imp(c, "lowergamma", SP, "Lower Incomplete Gamma", "a:real,x:real", "", false, r2(sp::lower_inc_gamma));
    def(c, "heaviside,theta", ST, "Heaviside Step Function", "x:real", "", true, |_, a| {
        is_r(&a[0]).map(|r| match r.sign() {
            1 => one(),
            -1 => zero(),
            _ => half(),
        })
    });
    def(c, "dirac,delta", ST, "Dirac Delta", "x:real", "", true, |_, a| is_r(&a[0]).map(|r| if r.is_zero() { num_r(Real::pos_inf()) } else { zero() }));
    def(c, "ramp", ST, "Ramp Function", "x:real", "", true, |_, a| is_r(&a[0]).map(|r| nr(Real::max(r, &Real::zero()))));
    def(c, "rect", ST, "Rectangular Function", "x:real", "", true, |_, a| {
        is_r(&a[0]).map(|r| {
            let ab = r.abs();
            if ab.lt(&Real::half()) {
                one()
            } else if ab.eqv(&Real::half()) {
                half()
            } else {
                zero()
            }
        })
    });
    def(c, "tri,triangle", ST, "Triangular Function", "x:real", "", true, |_, a| is_r(&a[0]).map(|r| nr(Real::max(&Real::zero(), &(Real::one() - r.abs())))));
}

fn with_real_roots(c: &Calculator, f: impl FnOnce() -> Expr) -> Expr {
    let save = c.opts().real_roots;
    c.opts_mut().real_roots = true;
    finally(f, || c.opts_mut().real_roots = save)
}

pub fn round_fn(c: &Calculator, x: &Expr, f: &dyn Fn(&Real) -> Real) -> R {
    match &x.e {
        E::Num(n) => {
            if n.is_real() {
                let r = f(&n.re);
                return Some(nr(if r.approx && r.is_int() { Real::big_int(r.n.clone()) } else { r }));
            }
            return Some(num(Cx::new(f(&n.re), f(&n.im))));
        }
        E::Intv(lo, hi) => return Some(intervals::from_mid_rad(&f(&((lo + hi) / Real::two())), &Real::zero())),
        _ => {}
    }
    if x.has_units() {
        if let Some((_, _, rest)) = units::try_quantity(x) {
            if rest.is_one() {
                let (cf, un) = simp::split_coef(x);
                if cf.is_real() {
                    return Some(simp::mul2(&nr(f(&cf.re)), &un));
                }
            }
        }
    }
    if !c.exact() || x.has_symbols() {
        return None;
    }
    c.with_exact(false, || {
        let ap = c.ev.eval(x);
        let an = ap.as_real()?;
        let r = f(an);
        let nearest = r.round_default();
        Some(nr(if (&r - &nearest).abs().lt(&Real::frac(1, 1000000000)) { Real::big_int(nearest.to_big()) } else { r }))
    })
}

fn quantities2(c: &Calculator, a: &[Expr], f: &str) -> R {
    let (v1, d1, r1) = units::try_quantity(&a[0])?;
    let (v2, d2, r2) = units::try_quantity(&a[1])?;
    if r1.is_one() && r2.is_one() && d1.same_as(&d2) && v1.is_real() && v2.is_real() && !d1.is_none() {
        let res = c.ev.call(f, vec![nr(v1.re), nr(v2.re)]);
        return Some(units::convert(&simp::mul2(&res, &units::base_units(&d1)), &simp::split_units_of(&a[0])));
    }
    None
}

fn num_den_part(e: &Expr, numer: bool) -> R {
    if !e.has_symbols() {
        return None;
    }
    let (n, d) = simp::num_den(e);
    Some(if numer { n } else { d })
}

fn rat_gcd(a: &Real, b: &Real) -> Real {
    if a.is_zero() {
        return b.abs();
    }
    if b.is_zero() {
        return a.abs();
    }
    let n = (&a.n * &b.d).gcd(&(&b.n * &a.d));
    Real::exact(n, &a.d * &b.d)
}

fn log_gamma_real(x: &Real) -> Real {
    let d = x.to_f64();
    if d > 170.0 {
        return rmath::exp(&Real::from_f64(sp::log_gamma(d)));
    }
    Real::from_f64(sp::gamma(d))
}

// gamma at half integers: (n - 1/2)! has a closed form with sqrt(pi)
fn half_factorial(x: &Real) -> R {
    let n = x + &Real::half();
    if !n.is_int() || n.abs().gt(&Real::int(5000)) {
        return None;
    }
    let k = n.to_int();
    let mut coef = Real::one();
    if k >= 0 {
        for i in 1..=k {
            coef = coef * Real::frac(2 * i as i64 - 1, 2);
        }
    } else {
        for i in 0..-k {
            coef = coef / Real::frac(-2 * i as i64 - 1, 2);
        }
    }
    Some(simp::mul2(&nr(coef), &simp::pow(&pi(), &half())))
}

fn exact_log(x: &Real, b: &Real) -> Option<Real> {
    if b.is_one() {
        throw("Logarithm base can not be one");
    }
    if x.is_one() {
        return Some(Real::zero());
    }
    let mut inv = false;
    let mut bb = b.clone();
    if bb.lt(&Real::one()) {
        bb = Real::one() / bb;
        inv = true;
    }
    let mut xx = x.clone();
    let mut neg = false;
    if xx.lt(&Real::one()) {
        xx = Real::one() / xx;
        neg = true;
    }
    let fix = |mut r: Real| {
        if neg {
            r = -r;
        }
        if inv {
            r = -r;
        }
        r
    };
    let mut p = bb.clone();
    for k in 1..=4000 {
        if p.eqv(&xx) {
            return Some(fix(Real::int(k)));
        }
        if p.gt(&xx) {
            break;
        }
        p = &p * &bb;
    }
    for q in 2..=6 {
        let Some(root) = bb.exact_root(q) else { continue };
        if let Some(rr) = exact_log(&xx, &root) {
            return Some(fix(rr / Real::int(q as i64)));
        }
    }
    None
}

pub fn flatten(args: &[Expr]) -> Vec<Expr> {
    let mut l = vec![];
    for a in args {
        match &a.e {
            E::Mat(rows, _) => l.extend(rows.iter().flatten().cloned()),
            _ => l.push(a.clone()),
        }
    }
    l
}

pub fn to_roman(n: &BigInt) -> Option<String> {
    if !n.is_positive() || *n >= big(4000000) {
        return None;
    }
    let mut sb = String::new();
    let mut v = n.to_i64().unwrap();
    if v >= 4000 {
        let th = to_roman(&big(v / 1000)).unwrap();
        for ch in th.chars() {
            sb.push(ch);
            sb.push('\u{0305}');
        }
        v %= 1000;
    }
    const VALS: [i64; 13] = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1];
    const SYMS: [&str; 13] = ["M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I"];
    for i in 0..VALS.len() {
        while v >= VALS[i] {
            sb.push_str(SYMS[i]);
            v -= VALS[i];
        }
    }
    Some(sb)
}

pub fn from_roman(s: &str) -> BigInt {
    let val = |c: char| match c {
        'I' => Some(1),
        'V' => Some(5),
        'X' => Some(10),
        'L' => Some(50),
        'C' => Some(100),
        'D' => Some(500),
        'M' => Some(1000),
        _ => None,
    };
    let cs: Vec<char> = s.trim().to_uppercase().chars().collect();
    let mut total = BigInt::zero();
    for i in 0..cs.len() {
        let v = val(cs[i]).unwrap_or_else(|| throw("Invalid roman numeral"));
        match cs.get(i + 1).and_then(|&c| val(c)) {
            Some(nx) if nx > v => total -= v,
            _ => total += v,
        }
    }
    total
}

pub fn parse_date(s: &str) -> (NaiveDateTime, bool, bool) {
    static OFF_RX: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"[+-]\d{2}:?\d{2}$").unwrap());
    let s = s.trim();
    let mut has_time = s.contains(':');
    let utc = s.ends_with('Z') || (OFF_RX.is_match(s) && s.len() > 10);
    if utc {
        let t = s.replacen(' ', "T", 1);
        for f in ["%Y-%m-%dT%H:%M:%S%.f%#z", "%Y-%m-%dT%H:%M%#z", "%Y-%m-%dT%H:%M:%S%.fZ", "%Y-%m-%dT%H:%MZ"] {
            if let Ok(d) = DateTime::parse_from_str(&t, f) {
                return (d.naive_utc(), has_time, true);
            }
            if let Ok(d) = NaiveDateTime::parse_from_str(&t, f) {
                return (d, has_time, true);
            }
        }
        if let Ok(d) = NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d") {
            return (d.and_hms_opt(0, 0, 0).unwrap(), false, true);
        }
    }
    for f in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M"] {
        if let Ok(d) = NaiveDateTime::parse_from_str(s, f) {
            return (d, has_time, false);
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        has_time = false;
        return (d.and_hms_opt(0, 0, 0).unwrap(), has_time, false);
    }
    throw_s(format!("Invalid date \"{}\"", s))
}

// best rational approximation with bounded denominator (continued fractions)
pub fn to_fraction(x: &Real, max_den: i64) -> Real {
    if x.is_int() {
        return Real::big_int(x.n.clone());
    }
    let neg = x.sign() < 0;
    let v = x.abs();
    let (mut h0, mut h1, mut k0, mut k1) = (BigInt::zero(), BigInt::one(), BigInt::one(), BigInt::zero());
    let mut r = v.clone();
    let md = big(max_den);
    for _ in 0..64 {
        let a = r.floor().to_big();
        let h2 = &a * &h1 + &h0;
        let k2 = &a * &k1 + &k0;
        if k2 > md {
            break;
        }
        h0 = std::mem::replace(&mut h1, h2);
        k0 = std::mem::replace(&mut k1, k2);
        let fr = &r - &Real::big_int(a);
        if fr.is_zero() || (Real::exact(h1.clone(), k1.clone()) - &v).to_f64().abs() < 1e-15 * 1f64.max(v.to_f64().abs()) {
            break;
        }
        r = Real::one() / fr;
    }
    if k1.is_zero() {
        return x.clone();
    }
    let res = Real::exact(h1, k1);
    if neg {
        -res
    } else {
        res
    }
}