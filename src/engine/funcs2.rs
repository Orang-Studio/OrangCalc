use super::algebra as simp;
use super::c64::c64;
use super::funcs::*;
use super::matrices as mx;
use super::real::{big, rng_f64, rng_next};
use super::special as sp;
use super::*;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, Timelike};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::sync::{Arc, Mutex};

// a plot() call stores its request here for the UI to pick up
pub static PLOT_REQUEST: Mutex<Option<(String, f64, f64)>> = Mutex::new(None);

fn items(args: &[Expr]) -> Vec<Expr> {
    flatten(args)
}

fn reals(args: &[Expr], f: &str) -> Vec<Real> {
    let l: Vec<Real> = items(args).iter().map(|e| is_r(e).cloned().unwrap_or_else(|| throw_s(format!("{}() requires real numbers", f)))).collect();
    if l.is_empty() {
        throw_s(format!("{}() requires at least one value", f));
    }
    l
}

fn all_numeric(args: &[Expr]) -> bool {
    items(args).iter().all(|e| e.is_real_num())
}

fn as_mat(e: &Expr, f: &str) -> Expr {
    match &e.e {
        E::Mat(..) => e.clone(),
        E::Num(_) => mx::as_matrix(e),
        _ => throw_s(format!("{}() requires a matrix", f)),
    }
}

fn sym_name(e: Option<&Expr>, fallback: &str) -> String {
    let Some(e) = e else { return fallback.to_string() };
    match &e.e {
        E::Sym(s) => s.clone(),
        E::Str(s) => s.clone(),
        E::Var(v) => v.name().to_string(),
        E::Unit(u, _) => u.abbrev().to_string(),
        _ => throw("Expected a variable name"),
    }
}

fn sn(e: &Expr) -> String {
    sym_name(Some(e), "x")
}

fn sum(l: Vec<Expr>) -> Expr {
    simp::add(l)
}

fn ni(v: i64) -> Expr {
    num_i(v)
}

fn hidden(c: &Calculator, names: &str, title: &str, args: &str, imp: impl Fn(&Calculator, &[Expr]) -> R + Send + Sync + 'static) {
    let mut f = build(names, "", title, args, "", false, Some(Arc::new(imp)));
    f.item.hidden = true;
    c.add_function(f);
}

#[allow(clippy::too_many_arguments)]
fn raw(c: &Calculator, names: &str, cat: &str, title: &str, args: &str, desc: &str, imp: impl Fn(&Calculator, &[Expr]) -> R + Send + Sync + 'static) {
    let mut f = build(names, cat, title, args, desc, false, Some(Arc::new(imp)));
    f.raw_args = true;
    f.item.hidden = cat.is_empty();
    c.add_function(f);
}

fn all_num(a: &[Expr]) -> bool {
    a.iter().all(|z| z.is_num())
}

fn exact_skip(c: &Calculator, a: &[Expr]) -> bool {
    c.exact() && !a.iter().any(|z| z.is_approx())
}

fn date_of(e: &Expr) -> (NaiveDateTime, bool, bool) {
    match &dates::as_date(e).e {
        E::Date(t, h, u) => (*t, *h, *u),
        _ => throw("Expected a date"),
    }
}

fn mk_date(y: i32, m: i32, d: i32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m as u32, d as u32).and_then(|x| x.and_hms_opt(0, 0, 0)).unwrap_or_else(|| throw("Invalid date"))
}

fn today() -> NaiveDateTime {
    Local::now().date_naive().and_hms_opt(0, 0, 0).unwrap()
}

fn ticks_real(d: Duration) -> Real {
    Real::exact(big(d.num_microseconds().unwrap_or(0)), big(1000000))
}

pub fn register_more(c: &Calculator) {
    const ST: &str = "Statistics";
    const MV: &str = "Matrices & Vectors";
    const AL: &str = "Algebra";
    const CA: &str = "Calculus";
    const LG: &str = "Logical";
    const BT: &str = "Bits";
    const UT: &str = "Utilities";
    const DT: &str = "Date & Time";
    const EC: &str = "Economics";
    const GE: &str = "Geometry";
    const IV: &str = "Intervals";
    const NB: &str = "Number Bases";
    const RN: &str = "Random Numbers";

    hidden(c, "_percent", "Percent", "x", |_, a| Some(simp::div(&a[0], &ni(100))));
    hidden(c, "_hold", "Hold", "x", |_, a| (a[0].is_num() || a[0].is_sym()).then(|| a[0].clone()));
    hidden(c, "_permille", "Per Mille", "x", |_, a| Some(simp::div(&a[0], &ni(1000))));
    hidden(c, "_angle", "Angle Notation", "r,angle", |cc, a| Some(simp::mul2(&a[0], &cc.ev.call("cis", vec![a[1].clone()]))));
    raw(c, "_dataset", "", "Data Set Lookup", "set,object,property=\"\"", "", |cc, a| Some(data::lookup(cc, &str_of(&a[0]), &a[1], &if a.len() > 2 { str_of(&a[2]) } else { String::new() })));

    def(c, "sum,∑", ST, "Sum", "term:hold,lower=1,upper?,var:hold=x", "Sum of term for var from lower to upper, or sum of vector elements.", false, |cc, a| {
        if a.len() < 3 {
            let v = cc.ev.eval(&a[0]);
            if v.is_mat() {
                return Some(sum(mx::items(&v)));
            }
            if a.len() == 1 {
                return Some(v);
            }
            throw("sum() requires term, lower and upper limit");
        }
        series_op(cc, a, true)
    });
    def(c, "product,∏", ST, "Product", "term:hold,lower=1,upper?,var:hold=x", "Product of term for var from lower to upper.", false, |cc, a| {
        if a.len() < 3 {
            let v = cc.ev.eval(&a[0]);
            if v.is_mat() {
                return Some(simp::mul(mx::items(&v)));
            }
            throw("product() requires term, lower and upper limit");
        }
        series_op(cc, a, false)
    });
    def(c, "total", ST, "Sum of Elements", "values:vec,...", "", false, |_, a| Some(sum(items(a))));
    def(c, "count", ST, "Number of Elements", "values,...", "", false, |_, a| Some(ni(items(a).len() as i64)));
    def(c, "mean,average,avg", ST, "Mean", "values:vec,...", "Arithmetic mean.", false, |_, a| {
        let l = items(a);
        let n = l.len() as i64;
        Some(simp::div(&sum(l), &ni(n)))
    });
    def(c, "geomean", ST, "Geometric Mean", "values:vec,...", "", false, |_, a| {
        let l = items(a);
        let n = l.len() as i64;
        Some(simp::pow(&simp::mul(l), &num_r(Real::frac(1, n))))
    });
    def(c, "harmmean", ST, "Harmonic Mean", "values:vec,...", "", false, |_, a| {
        let l = items(a);
        let n = l.len() as i64;
        Some(simp::div(&ni(n), &sum(l.iter().map(|x| simp::pow(x, &minus_one())).collect())))
    });
    def(c, "rms", ST, "Root Mean Square", "values:vec,...", "", false, |_, a| {
        let l = items(a);
        let n = l.len() as i64;
        Some(simp::pow(&simp::div(&sum(l.iter().map(|x| simp::pow(x, &two())).collect()), &ni(n)), &half()))
    });
    def(c, "median", ST, "Median", "values:vec,...", "", false, |_, a| {
        let mut l = reals(a, "median");
        l.sort_by(|x, y| x.cmp(y));
        let n = l.len();
        Some(nr(if n % 2 == 1 { l[n / 2].clone() } else { (&l[n / 2 - 1] + &l[n / 2]) / Real::two() }))
    });
    def(c, "mode", ST, "Mode", "values:vec,...", "Most frequent value.", false, |_, a| {
        let l = items(a);
        let mut groups: Vec<(String, Expr, usize)> = vec![];
        for x in l {
            match groups.iter_mut().find(|g| g.0 == x.key()) {
                Some(g) => g.2 += 1,
                None => groups.push((x.key().to_string(), x.clone(), 1)),
            }
        }
        let max = groups.iter().map(|g| g.2).max()?;
        groups.into_iter().find(|g| g.2 == max).map(|g| g.1)
    });
    def(c, "min", ST, "Minimum", "values,...", "", false, |cc, a| {
        let l = items(a);
        if !all_numeric(&l) {
            return quantity_extreme(cc, &l, -1);
        }
        let r = reals(&l, "min");
        Some(nr(r.iter().skip(1).fold(r[0].clone(), |m, x| Real::min(&m, x))))
    });
    def(c, "max", ST, "Maximum", "values,...", "", false, |cc, a| {
        let l = items(a);
        if !all_numeric(&l) {
            return quantity_extreme(cc, &l, 1);
        }
        let r = reals(&l, "max");
        Some(nr(r.iter().skip(1).fold(r[0].clone(), |m, x| Real::max(&m, x))))
    });
    def(c, "range", ST, "Range", "values:vec,...", "", false, |_, a| {
        let l = reals(a, "range");
        let mx = l.iter().skip(1).fold(l[0].clone(), |m, x| Real::max(&m, x));
        let mn = l.iter().skip(1).fold(l[0].clone(), |m, x| Real::min(&m, x));
        Some(nr(mx - mn))
    });
    def(c, "var,variance", ST, "Variance (sample)", "values:vec,...", "", false, |_, a| Some(variance(&items(a), true)));
    def(c, "varp", ST, "Variance (population)", "values:vec,...", "", false, |_, a| Some(variance(&items(a), false)));
    def(c, "stdev,stddev", ST, "Standard Deviation (sample)", "values:vec,...", "", false, |_, a| Some(simp::pow(&variance(&items(a), true), &half())));
    def(c, "stdevp", ST, "Standard Deviation (population)", "values:vec,...", "", false, |_, a| Some(simp::pow(&variance(&items(a), false), &half())));
    def(c, "percentile", ST, "Percentile", "values:vec,p:real", "", false, |_, a| {
        let mut l = reals(&a[..1], "percentile");
        l.sort_by(|x, y| x.cmp(y));
        let p = a[1].num().re.clone() / Real::int(100);
        let pos = p * Real::int(l.len() as i64 - 1);
        let lo = pos.floor().to_int();
        if lo >= l.len() as i32 - 1 {
            return Some(nr(l.last().unwrap().clone()));
        }
        if lo < 0 {
            return Some(nr(l[0].clone()));
        }
        let lo = lo as usize;
        let fr = &pos - &Real::int(lo as i64);
        Some(nr(&l[lo] + &((&l[lo + 1] - &l[lo]) * fr)))
    });
    def(c, "quartile", ST, "Quartile", "values:vec,q:int", "", false, |cc, a| Some(cc.ev.call("percentile", vec![a[0].clone(), simp::mul2(&a[1], &ni(25))])));
    def(c, "cov,covariance", ST, "Covariance", "x:vec,y:vec", "", false, |_, a| {
        let x = items(&a[..1]);
        let y = items(&a[1..2]);
        if x.len() != y.len() {
            throw("Vectors must have the same length");
        }
        let n = x.len() as i64;
        let mx = simp::div(&sum(x.clone()), &ni(n));
        let my = simp::div(&sum(y.clone()), &ni(n));
        Some(simp::div(&sum(x.iter().enumerate().map(|(i, xi)| simp::mul2(&simp::sub(xi, &mx), &simp::sub(&y[i], &my))).collect()), &ni(n - 1)))
    });
    def(c, "corr,correlation,pearson", ST, "Correlation Coefficient", "x:vec,y:vec", "", false, |cc, a| {
        let cov = cc.ev.call("cov", vec![a[0].clone(), a[1].clone()]);
        Some(simp::div(&cov, &simp::mul2(&cc.ev.call("stdev", vec![a[0].clone()]), &cc.ev.call("stdev", vec![a[1].clone()]))))
    });
    def(c, "linreg", ST, "Linear Regression", "x:vec,y:vec", "Returns [slope, intercept] of the least squares line.", false, |_, a| {
        let x = reals(&a[..1], "linreg");
        let y = reals(&a[1..2], "linreg");
        let n = x.len();
        let sx = x.iter().fold(Real::zero(), |s, v| s + v);
        let sy = y.iter().fold(Real::zero(), |s, v| s + v);
        let mut sxy = Real::zero();
        let mut sxx = Real::zero();
        for i in 0..n.min(y.len()) {
            sxy = sxy + &x[i] * &y[i];
            sxx = sxx + &x[i] * &x[i];
        }
        let nn = Real::int(n as i64);
        let slope = (&nn * &sxy - &sx * &sy) / (&nn * &sxx - &sx * &sx);
        let icpt = (&sy - &(&slope * &sx)) / nn;
        Some(vector_e(vec![nr(slope), nr(icpt)]))
    });
    def(c, "normdist,normal", ST, "Normal Distribution", "x:real,mean:real=0,sd:real=1,cumulative:bool=0", "Probability density, or cumulative probability.", false, |cc, a| {
        if !all_num(a) || exact_skip(cc, a) {
            return None;
        }
        let (x, m, s) = (dbl(&a[0], "normdist"), dbl(&a[1], "normdist"), dbl(&a[2], "normdist"));
        Some(nd(if !a[3].is_zero() { sp::normal_cdf(x, m, s) } else { sp::normal_pdf(x, m, s) }))
    });
    def(c, "norminv", ST, "Inverse Normal Distribution", "p:real,mean:real=0,sd:real=1", "", false, |cc, a| {
        if exact_skip(cc, a) {
            return None;
        }
        Some(nd(sp::normal_inv(dbl(&a[0], "norminv"), dbl(&a[1], "norminv"), dbl(&a[2], "norminv"))))
    });
    def(c, "poisson", ST, "Poisson Distribution", "k:nnint,lambda:real,cumulative:bool=0", "", false, |cc, a| {
        let k = small_int(&a[0], "poisson");
        let lam = &a[1];
        let pmf = |i: i32| simp::div(&simp::mul2(&simp::pow(lam, &ni(i as i64)), &cc.ev.call("exp", vec![simp::neg(lam)])), &nb(primes::factorial(&big(i as i64), 1)));
        if a[2].is_zero() {
            return Some(pmf(k));
        }
        Some(sum((0..=k).map(pmf).collect()))
    });
    def(c, "binomdist", ST, "Binomial Distribution", "k:nnint,n:nnint,p:real,cumulative:bool=0", "", false, |_, a| {
        let k = small_int(&a[0], "binomdist");
        let n = small_int(&a[1], "binomdist");
        let p = &a[2];
        let pmf = |i: i32| simp::mul(vec![nb(primes::binomial(&big(n as i64), &big(i as i64))), simp::pow(p, &ni(i as i64)), simp::pow(&simp::sub(&one(), p), &ni((n - i) as i64))]);
        if a[3].is_zero() {
            return Some(pmf(k));
        }
        Some(sum((0..=k).map(pmf).collect()))
    });
    def(c, "expdist", ST, "Exponential Distribution", "x:real,lambda:real,cumulative:bool=0", "", false, |cc, a| {
        let e = cc.ev.call("exp", vec![simp::neg(&simp::mul2(&a[1], &a[0]))]);
        Some(if a[2].is_zero() { simp::mul2(&a[1], &e) } else { simp::sub(&one(), &e) })
    });
    def(c, "tdist", ST, "Student's t Distribution", "t:real,df:real", "Cumulative probability.", false, |cc, a| {
        if exact_skip(cc, a) {
            return None;
        }
        Some(nd(sp::student_t_cdf(dbl(&a[0], "tdist"), dbl(&a[1], "tdist"))))
    });
    def(c, "chisqdist", ST, "Chi-squared Distribution", "x:real,k:real", "Cumulative probability.", false, |cc, a| {
        if exact_skip(cc, a) {
            return None;
        }
        Some(nd(sp::chi_square_cdf(dbl(&a[0], "chisqdist"), dbl(&a[1], "chisqdist"))))
    });
    def(c, "uniformdist", ST, "Uniform Distribution", "x:real,a:real,b:real,cumulative:bool=0", "", false, |_, a| {
        if !all_num(a) {
            return None;
        }
        let (x, lo, hi) = (&a[0].num().re, &a[1].num().re, &a[2].num().re);
        if a[3].is_zero() {
            return Some(if x.lt(lo) || x.gt(hi) { zero() } else { nr(Real::one() / (hi - lo)) });
        }
        Some(if x.lt(lo) {
            zero()
        } else if x.gt(hi) {
            one()
        } else {
            nr((x - lo) / (hi - lo))
        })
    });

    def(c, "rand,random", RN, "Random Number", "max:int=0,count:int=1", "Random number in [0, 1), or integer in [1, max].", false, |_, a| {
        let count = if a.len() > 1 { small_int(&a[1], "rand") } else { 1 };
        let mx = if !a.is_empty() { int(&a[0], "rand") } else { BigInt::zero() };
        let one_ = || if !mx.is_positive() { nd(rng_f64()) } else { nb(random_big(&mx) + 1) };
        if count == 1 {
            return Some(one_());
        }
        if count > 100000 {
            throw("Too many elements");
        }
        Some(vector_e((0..count.max(0)).map(|_| one_()).collect()))
    });
    def(c, "randint", RN, "Random Integer", "a:int,b:int", "", false, |_, a| {
        let lo = int(&a[0], "randint");
        let hi = int(&a[1], "randint");
        Some(nb(&lo + random_big(&(&hi - &lo + 1))))
    });
    def(c, "randn,randnorm", RN, "Normally Distributed Random Number", "mean:real=0,sd:real=1", "", false, |_, a| {
        let u1 = 1.0 - rng_f64();
        let u2 = rng_f64();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        Some(nd(dbl(&a[0], "randn") + dbl(&a[1], "randn") * z))
    });
    def(c, "randuniform", RN, "Uniform Random Number", "a:real,b:real", "", false, |_, a| {
        let lo = dbl(&a[0], "randuniform");
        Some(nd(lo + (dbl(&a[1], "randuniform") - lo) * rng_f64()))
    });
    def(c, "randpoisson", RN, "Poisson Random Number", "lambda:real", "", false, |_, a| {
        let l = (-dbl(&a[0], "randpoisson")).exp();
        let mut p = 1.0;
        let mut k = 0;
        loop {
            k += 1;
            p *= rng_f64();
            if !(p > l && k < 1000000) {
                break;
            }
        }
        Some(ni(k - 1))
    });
    def(c, "dice", RN, "Roll Dice", "n:pint=1,sides:pint=6", "", false, |_, a| {
        let n = small_int(&a[0], "dice");
        let s = small_int(&a[1], "dice");
        let mut t: i64 = 0;
        for _ in 0..n {
            t += rng_next(s as u64) as i64 + 1;
        }
        Some(ni(t))
    });

    def(c, "matrix", MV, "Construct Matrix", "rows:pint,columns:pint,elements:vec?", "", false, |_, a| {
        let r = small_int(&a[0], "matrix") as usize;
        let co = small_int(&a[1], "matrix") as usize;
        let el = if a.len() > 2 { items(&a[2..3]) } else { vec![] };
        Some(mx::create(r, co, &mut |i, j| el.get(i * co + j).cloned().unwrap_or_else(zero)))
    });
    def(c, "vector", MV, "Construct Vector", "elements,...", "", false, |_, a| Some(vector_e(a.to_vec())));
    def(c, "identity,eye", MV, "Identity Matrix", "n:pint", "", false, |_, a| Some(mx::identity(small_int(&a[0], "identity") as usize)));
    def(c, "zero,zeros", MV, "Zero Matrix", "rows:pint,columns:pint=0", "", false, |_, a| {
        let r = small_int(&a[0], "zero") as usize;
        let co = if a.len() > 1 && !a[1].is_zero() { small_int(&a[1], "zero") as usize } else { r };
        Some(mx::create(r, co, &mut |_, _| zero()))
    });
    def(c, "ones", MV, "Matrix of Ones", "rows:pint,columns:pint=0", "", false, |_, a| {
        let r = small_int(&a[0], "ones") as usize;
        let co = if a.len() > 1 && !a[1].is_zero() { small_int(&a[1], "ones") as usize } else { r };
        Some(mx::create(r, co, &mut |_, _| one()))
    });
    def(c, "det,determinant", MV, "Determinant", "m:mat", "", false, |_, a| match &a[0].e {
        E::Mat(..) => Some(mx::determinant(&a[0])),
        E::Num(_) => Some(a[0].clone()),
        _ => None,
    });
    def(c, "inverse,inv", MV, "Inverse Matrix", "m:mat", "", false, |_, a| Some(if a[0].is_mat() { mx::inverse(&a[0]) } else { simp::pow(&a[0], &minus_one()) }));
    def(c, "transpose", MV, "Transpose", "m:mat", "", false, |_, a| Some(if a[0].is_mat() { mx::transpose(&a[0]) } else { a[0].clone() }));
    def(c, "adj,adjoint,adjugate", MV, "Adjugate", "m:mat", "", false, |_, a| Some(mx::adjugate(&as_mat(&a[0], "adj"))));
    def(c, "cofactor", MV, "Cofactor", "m:mat,row:pint,column:pint", "", false, |_, a| Some(mx::cofactor(&as_mat(&a[0], "cofactor"), small_int(&a[1], "cofactor") as usize - 1, small_int(&a[2], "cofactor") as usize - 1)));
    def(c, "minor", MV, "Minor", "m:mat,row:pint,column:pint", "", false, |_, a| Some(mx::determinant(&mx::minor(&as_mat(&a[0], "minor"), small_int(&a[1], "minor") as usize - 1, small_int(&a[2], "minor") as usize - 1))));
    def(c, "rank", MV, "Rank", "m:mat", "", false, |_, a| Some(ni(mx::rank(&as_mat(&a[0], "rank")) as i64)));
    def(c, "rref", MV, "Reduced Row Echelon Form", "m:mat", "", false, |_, a| Some(mx::rref(&as_mat(&a[0], "rref"))));
    def(c, "trace", MV, "Trace", "m:mat", "", false, |_, a| Some(mx::trace(&as_mat(&a[0], "trace"))));
    def(c, "permanent", MV, "Permanent", "m:mat", "", false, |_, a| Some(mx::permanent(&as_mat(&a[0], "permanent"))));
    def(c, "dot", MV, "Dot Product", "a:vec,b:vec", "", false, |_, a| Some(mx::dot(&as_mat(&a[0], "dot"), &as_mat(&a[1], "dot"))));
    def(c, "cross", MV, "Cross Product", "a:vec,b:vec", "", false, |_, a| Some(mx::cross(&as_mat(&a[0], "cross"), &as_mat(&a[1], "cross"))));
    def(c, "kron,kronecker", MV, "Kronecker Product", "a:mat,b:mat", "", false, |_, a| Some(mx::kronecker(&as_mat(&a[0], "kron"), &as_mat(&a[1], "kron"))));
    def(c, "hadamard", MV, "Hadamard Product", "a:mat,b:mat", "", false, |_, a| Some(mx::hadamard(&as_mat(&a[0], "hadamard"), &as_mat(&a[1], "hadamard"))));
    def(c, "norm,magnitude", MV, "Norm", "v:vec,p:real=2", "Euclidean norm, or p-norm.", false, |cc, a| {
        let l = if a[0].is_mat() { mx::rows_of(&a[0]).iter().flatten().cloned().collect() } else { vec![a[0].clone()] };
        let p = &a[1];
        let abs = |x: &Expr| cc.ev.call("abs", vec![x.clone()]);
        if p.is_one() {
            return Some(sum(l.iter().map(abs).collect()));
        }
        if matches!(&p.e, E::Num(pn) if pn.re.is_infinite()) {
            return Some(cc.ev.call("max", vec![vector_e(l.iter().map(abs).collect())]));
        }
        if p.key() == two().key() {
            return Some(simp::pow(
                &sum(l.iter().map(|x| match &x.e {
                    E::Num(n) if !n.is_real() => nr(&n.re * &n.re + &n.im * &n.im),
                    _ => simp::pow(x, &two()),
                }).collect()),
                &half(),
            ));
        }
        Some(simp::pow(&sum(l.iter().map(|x| simp::pow(&abs(x), p)).collect()), &simp::pow(p, &minus_one())))
    });
    def(c, "normalize", MV, "Normalize Vector", "v:vec", "", false, |cc, a| {
        let n = cc.ev.call("norm", vec![a[0].clone()]);
        Some(mx::map_elements(&as_mat(&a[0], "normalize"), &mut |x| simp::div(&x, &n)))
    });
    def(c, "size,dimension,dim", MV, "Size", "m", "", false, |_, a| {
        Some(match &a[0].e {
            E::Mat(_, true) => ni(mx::col_count(&a[0]) as i64),
            E::Mat(..) => vector_e(vec![ni(mx::row_count(&a[0]) as i64), ni(mx::col_count(&a[0]) as i64)]),
            _ => one(),
        })
    });
    def(c, "rows", MV, "Number of Rows", "m:mat", "", false, |_, a| Some(ni(mx::row_count(&as_mat(&a[0], "rows")) as i64)));
    def(c, "columns,cols", MV, "Number of Columns", "m:mat", "", false, |_, a| Some(ni(mx::col_count(&as_mat(&a[0], "columns")) as i64)));
    def(c, "elements,numel", MV, "Number of Elements", "m:mat", "", false, |_, a| Some(ni(mx::rows_of(&as_mat(&a[0], "elements")).iter().map(|r| r.len()).sum::<usize>() as i64)));
    def(c, "element", MV, "Element", "m:mat,row:pint,column:pint=1", "", false, |_, a| {
        let m = as_mat(&a[0], "element");
        let r = small_int(&a[1], "element") as usize - 1;
        let co = small_int(&a[2], "element") as usize - 1;
        let rows = mx::rows_of(&m);
        if mx::is_vector(&m) {
            return Some(rows[0].get(r).cloned().unwrap_or_else(|| throw("Index out of range")));
        }
        rows.get(r).and_then(|row| row.get(co)).cloned().map(Some).unwrap_or_else(|| throw("Index out of range"))
    });
    def(c, "row", MV, "Row", "m:mat,row:pint", "", false, |_, a| {
        let m = as_mat(&a[0], "row");
        let r = small_int(&a[1], "row") as usize - 1;
        Some(vector_e(mx::rows_of(&m).get(r).cloned().unwrap_or_else(|| throw("Index out of range"))))
    });
    def(c, "column", MV, "Column", "m:mat,column:pint", "", false, |_, a| {
        let m = as_mat(&a[0], "column");
        let co = small_int(&a[1], "column") as usize - 1;
        if co >= mx::col_count(&m) {
            throw("Index out of range");
        }
        Some(vector_e(mx::rows_of(&m).iter().map(|r| r[co].clone()).collect()))
    });
    def(c, "submatrix,area", MV, "Submatrix", "m:mat,r1:pint,c1:pint,r2:pint,c2:pint", "", false, |_, a| {
        let m = as_mat(&a[0], "submatrix");
        let g = |i: usize| small_int(&a[i], "submatrix") as usize - 1;
        let (r1, c1, r2, c2) = (g(1), g(2), g(3), g(4));
        let rows = mx::rows_of(&m);
        if r2 < r1 || c2 < c1 || r2 >= rows.len() || c2 >= mx::col_count(&m) {
            throw("Index out of range");
        }
        Some(mx::create(r2 - r1 + 1, c2 - c1 + 1, &mut |i, j| rows[r1 + i][c1 + j].clone()))
    });
    def(c, "sort", MV, "Sort", "v:vec,descending:bool=0", "", false, |_, a| {
        let l = items(&a[..1]);
        let mut keyed: Vec<(Expr, f64)> = l.into_iter().map(|x| {
            let k = x.as_real().map(|r| r.to_f64()).unwrap_or(f64::NAN);
            (x, k)
        }).collect();
        keyed.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut sorted: Vec<Expr> = keyed.into_iter().map(|t| t.0).collect();
        if !a[1].is_zero() {
            sorted.reverse();
        }
        Some(vector_e(sorted))
    });
    def(c, "reverse,flip", MV, "Reverse", "v:vec", "", false, |_, a| {
        let mut l = items(&a[..1]);
        l.reverse();
        Some(vector_e(l))
    });
    def(c, "horzcat,hcat", MV, "Horizontal Concatenation", "a:mat,b:mat", "", false, |_, a| {
        let x = as_mat(&a[0], "horzcat");
        let y = as_mat(&a[1], "horzcat");
        if mx::is_vector(&x) && mx::is_vector(&y) {
            let mut l = mx::items(&x);
            l.extend(mx::items(&y));
            return Some(vector_e(l));
        }
        let (xr, yr) = (mx::rows_of(&x), mx::rows_of(&y));
        if xr.len() != yr.len() {
            throw("Row counts differ");
        }
        Some(mat_e(xr.iter().enumerate().map(|(i, r)| r.iter().chain(yr[i].iter()).cloned().collect()).collect(), false))
    });
    def(c, "vertcat,vcat", MV, "Vertical Concatenation", "a:mat,b:mat", "", false, |_, a| {
        let x = as_mat(&a[0], "vertcat");
        let y = as_mat(&a[1], "vertcat");
        if mx::col_count(&x) != mx::col_count(&y) {
            throw("Column counts differ");
        }
        Some(mat_e(mx::rows_of(&x).iter().chain(mx::rows_of(&y).iter()).cloned().collect(), false))
    });
    def(c, "genvector,seq", MV, "Generate Vector", "expression:hold,min,max,step=1,var:hold=x", "Vector of expression values for var from min to max.", false, |cc, a| {
        let v = sn(&a[4]);
        let (Some(lo), Some(hi), Some(st)) = (is_r(&a[1]), is_r(&a[2]), is_r(&a[3])) else { throw("genvector() requires numeric limits") };
        if st.is_zero() {
            throw("genvector() requires numeric limits");
        }
        let mut list = vec![];
        let mut x = lo.clone();
        while if st.sign() > 0 { x.le(hi) } else { x.ge(hi) } {
            cc.check_abort();
            list.push(cc.ev.eval_with(&a[0], &v, nr(x.clone())));
            if list.len() > 100000 {
                throw("Too many elements");
            }
            x = &x + st;
        }
        Some(vector_e(list))
    });
    def(c, "linspace", MV, "Linearly Spaced Vector", "a:real,b:real,n:pint", "", false, |_, a| {
        let n = small_int(&a[2], "linspace");
        if n == 1 {
            return Some(vector_e(vec![a[0].clone()]));
        }
        let step = simp::div(&simp::sub(&a[1], &a[0]), &ni(n as i64 - 1));
        Some(vector_e((0..n).map(|i| simp::add2(&a[0], &simp::mul2(&ni(i as i64), &step))).collect()))
    });
    def(c, "eigenvalues,eig", MV, "Eigenvalues", "m:mat", "", false, |cc, a| {
        let m = as_mat(&a[0], "eigenvalues");
        let rows = mx::rows_of(&m).clone();
        let n = rows.len();
        if n != mx::col_count(&m) {
            throw("Eigenvalues require a square matrix");
        }
        if n <= 4 && rows.iter().flatten().all(|x| matches!(&x.e, E::Num(v) if v.is_rational())) {
            let lam = sym("_lambda");
            let shifted = mx::create(n, n, &mut |i, j| if i == j { simp::sub(&rows[i][j], &lam) } else { rows[i][j].clone() });
            let cp = mx::determinant(&shifted);
            if let Ok(s) = attempt(|| solver::solve(cc, &cmp_e(CmpOp::Eq, cp.clone(), zero()), "_lambda")) {
                return Some(vector_e(s));
            }
        }
        let arr: Vec<Vec<f64>> = rows.iter().map(|r| r.iter().map(|x| dbl(&cc.ev.eval(x), "eigenvalues")).collect()).collect();
        Some(vector_e(mx::eigenvalues_numeric(&arr).into_iter().map(|z| ncd(c64((z.re * 1e12).round() / 1e12, if z.im.abs() < 1e-10 { 0.0 } else { z.im }))).collect()))
    });
    def(c, "charpoly", MV, "Characteristic Polynomial", "m:mat,var:hold=x", "", false, |_, a| {
        let m = as_mat(&a[0], "charpoly");
        let x = sym(&sn(&a[1]));
        let rows = mx::rows_of(&m);
        let shifted = mx::create(rows.len(), mx::col_count(&m), &mut |i, j| if i == j { simp::sub(&rows[i][j], &x) } else { rows[i][j].clone() });
        Some(simp::expand(&mx::determinant(&shifted)))
    });

    def(c, "solve", AL, "Solve Equation", "equation,var:hold=x", "Solves an equation for var.", false, |cc, a| {
        let v = sn(&a[1]);
        let mut sols = solver::solve(cc, &a[0], &v);
        if sols.len() == 1 {
            return sols.pop();
        }
        Some(vector_e(sols))
    });
    def(c, "solve2", AL, "Solve System of Two Equations", "eq1,eq2,x:hold=x,y:hold=y", "", false, |cc, a| Some(vector_e(solver::multi_solve(cc, &[a[0].clone(), a[1].clone()], &[sn(&a[2]), sym_name(Some(&a[3]), "y")]))));
    def(c, "multisolve", AL, "Solve System of Equations", "equations:vec,vars:hold", "", false, |cc, a| {
        let eqs = items(&a[..1]);
        let vraw = if a[1].is_mat() { mx::items(&a[1]) } else { vec![a[1].clone()] };
        Some(vector_e(solver::multi_solve(cc, &eqs, &vraw.iter().map(sn).collect::<Vec<_>>())))
    });
    def(c, "newtonsolve", AL, "Newton's Method", "f:hold,x0:real,var:hold=x", "", false, |cc, a| {
        let v = sn(&a[2]);
        let g = match &a[0].e {
            E::Cmp(_, l, r) => simp::sub(l, r),
            _ => a[0].clone(),
        };
        let r = solver::newton(&|x| cc.ev.eval_double(&g, &v, x), dbl(&a[1], "newtonsolve"));
        if r.is_nan() {
            throw("Newton's method did not converge");
        }
        Some(nd(r))
    });
    def(c, "secantsolve", AL, "Secant Method", "f:hold,x0:real,x1:real,var:hold=x", "", false, |cc, a| {
        let v = sn(&a[3]);
        let g = match &a[0].e {
            E::Cmp(_, l, r) => simp::sub(l, r),
            _ => a[0].clone(),
        };
        let r = solver::secant(&|x| cc.ev.eval_double(&g, &v, x), dbl(&a[1], "secantsolve"), dbl(&a[2], "secantsolve"));
        if r.is_nan() {
            throw("Secant method did not converge");
        }
        Some(nd(r))
    });
    def(c, "dsolve", CA, "Solve Differential Equation", "equation:hold,x0?,y0?", "Solves y' = a·y + b for y(x).", false, |cc, a| {
        let syms = calculator::symbols(&a[0]);
        let y = if syms.iter().any(|s| s == "y") { "y".to_string() } else { syms.iter().find(|s| *s != "x").cloned().unwrap_or_else(|| "y".into()) };
        let x0 = a.get(1).map(|e| cc.ev.eval(e));
        let y0 = a.get(2).map(|e| cc.ev.eval(e));
        Some(solver::dsolve(cc, &a[0], &y, "x", x0, y0))
    });
    def(c, "factor,factorize", AL, "Factorize", "x", "", false, |cc, a| Some(hold(cc, &polynomials::factorize(cc, &a[0]))));
    def(c, "expand", AL, "Expand", "x", "", false, |_, a| Some(simp::expand(&a[0])));
    def(c, "partfrac,apart", AL, "Partial Fractions", "x", "", false, |cc, a| Some(hold(cc, &polynomials::partial_fractions(cc, &a[0]))));
    def(c, "together", AL, "Combine Fractions", "x", "", false, |_, a| Some(polynomials::together(&a[0])));
    def(c, "simplify", AL, "Simplify", "x", "", false, |cc, a| Some(cc.structure(&a[0], StructMode::Simplify)));
    def(c, "coeff,coefficient", AL, "Coefficient", "p,n:nnint,var:hold=x", "", false, |_, a| {
        let cs = polynomials::try_coefficients(&a[0], &sn(&a[2])).unwrap_or_else(|| throw("Not a polynomial"));
        let n = small_int(&a[1], "coeff") as usize;
        Some(cs.get(n).cloned().unwrap_or_else(zero))
    });
    def(c, "degree", AL, "Degree", "p,var:hold=x", "", false, |_, a| {
        let cs = polynomials::try_coefficients(&a[0], &sn(&a[1])).unwrap_or_else(|| throw("Not a polynomial"));
        let mut d = cs.len() as i64 - 1;
        while d > 0 && cs[d as usize].is_zero() {
            d -= 1;
        }
        Some(ni(d))
    });
    def(c, "lcoeff", AL, "Leading Coefficient", "p,var:hold=x", "", false, |_, a| {
        let cs = polynomials::try_coefficients(&a[0], &sn(&a[1])).unwrap_or_else(|| throw("Not a polynomial"));
        Some(cs.iter().rev().find(|z| !z.is_zero()).cloned().unwrap_or_else(zero))
    });
    def(c, "tcoeff", AL, "Trailing Coefficient", "p,var:hold=x", "", false, |_, a| {
        let cs = polynomials::try_coefficients(&a[0], &sn(&a[1])).unwrap_or_else(|| throw("Not a polynomial"));
        Some(cs.iter().find(|z| !z.is_zero()).cloned().unwrap_or_else(zero))
    });
    def(c, "roots,polyroots", AL, "Polynomial Roots", "p,var:hold=x", "", false, |cc, a| Some(vector_e(solver::solve(cc, &cmp_e(CmpOp::Eq, a[0].clone(), zero()), &sn(&a[1])))));
    def(c, "replace,subs", AL, "Replace", "expression,from,to", "Replaces all occurrences of from with to.", false, |cc, a| Some(cc.ev.eval(&cc.substitute(&a[0], &a[1], &a[2]))));

    def(c, "diff,derivative,der", CA, "Derivative", "f,var:hold=x,order:nnint=1", "Derivative of f with respect to var.", false, |cc, a| {
        let v = sn(&a[1]);
        let n = small_int(&cc.ev.eval(&a[2]), "diff");
        let mut r = a[0].clone();
        for _ in 0..n {
            r = calculus::diff(&r, &v);
        }
        Some(cc.ev.eval(&r))
    });
    def(c, "integrate,integral,∫", CA, "Integral", "f:hold,var:hold=x,lower?,upper?", "Antiderivative, or definite integral when limits are given.", false, integral);
    def(c, "nintegrate", CA, "Numerical Integral", "f:hold,lower,upper,var:hold=x", "", false, |cc, a| {
        let v = sn(&a[3]);
        let f = cc.ev.eval(&a[0]);
        let r = calculus::num_integrate(&|x| cc.ev.eval_double(&f, &v, x), dbl(&a[1], "nintegrate"), dbl(&a[2], "nintegrate")).0;
        Some(nd(r))
    });
    def(c, "limit,lim", CA, "Limit", "f:hold,a,var:hold=x,direction:int=0", "Limit of f as var approaches a (direction −1 or 1 for one-sided).", false, |cc, a| {
        let f = cc.ev.eval(&a[0]);
        Some(calculus::limit(&f, &sn(&a[2]), &a[1], small_int(&a[3], "limit")))
    });
    def(c, "extremum,extrema", CA, "Extrema", "f,var:hold=x", "Points where the derivative is zero.", false, |cc, a| {
        let v = sn(&a[1]);
        let d = calculus::diff(&a[0], &v);
        Some(vector_e(solver::solve(cc, &cmp_e(CmpOp::Eq, d, zero()), &v)))
    });
    def(c, "taylor", CA, "Taylor Polynomial", "f,var:hold=x,point=0,order:nnint=5", "", false, |cc, a| {
        let v = sn(&a[1]);
        let n = small_int(&a[3], "taylor");
        let x = sym(&v);
        let mut terms = vec![];
        let mut d = a[0].clone();
        for k in 0..=n {
            let val = cc.ev.eval_with(&d, &v, a[2].clone());
            terms.push(simp::mul2(&simp::div(&val, &nb(primes::factorial(&big(k as i64), 1))), &simp::pow(&simp::sub(&x, &a[2]), &ni(k as i64))));
            d = calculus::diff(&d, &v);
        }
        Some(simp::expand(&sum(terms)))
    });

    def(c, "if", LG, "If", "condition,then:hold,else:hold=0", "Returns then if condition is true, otherwise else.", false, |cc, a| {
        let t = evaluator::truth(&a[0]).unwrap_or_else(|| throw("if() condition could not be determined"));
        Some(cc.ev.eval(if t { &a[1] } else { &a[2] }))
    });
    def(c, "and", LG, "Logical AND", "a,b,...", "", false, |cc, a| Some(cc.ev.eval(&logic_e(LogicOp::And, a.to_vec()))));
    def(c, "or", LG, "Logical OR", "a,b,...", "", false, |cc, a| Some(cc.ev.eval(&logic_e(LogicOp::Or, a.to_vec()))));
    def(c, "not", LG, "Logical NOT", "a", "", false, |cc, a| Some(cc.ev.eval(&logic_e(LogicOp::Not, a.to_vec()))));
    def(c, "xor", LG, "Exclusive OR", "a,b", "Bitwise exclusive or for integers.", false, |cc, a| {
        if let (Some(x), Some(y)) = (is_i(&a[0]), is_i(&a[1])) {
            return Some(nb(x ^ y));
        }
        Some(cc.ev.eval(&logic_e(LogicOp::Xor, a.to_vec())))
    });
    def(c, "for", LG, "For Loop", "start,var:hold,condition:hold,update:hold,expression:hold,initial=0,result:hold=y", "for(start, x, condition, update, expression, initial, y)", false, |cc, a| {
        let v = sn(&a[1]);
        let res = sym_name(Some(&a[6]), "y");
        let mut curv = a[0].clone();
        let mut acc = a[5].clone();
        for _ in 0..1000000 {
            cc.check_abort();
            {
                let mut b = cc.ev.bindings.borrow_mut();
                b.insert(v.clone(), curv.clone());
                b.insert(res.clone(), acc.clone());
            }
            let step = finally(
                || {
                    let cond = evaluator::truth(&cc.ev.eval(&a[2]));
                    if cond != Some(true) {
                        return None;
                    }
                    let nacc = cc.ev.eval(&a[4]);
                    cc.ev.bindings.borrow_mut().insert(res.clone(), nacc.clone());
                    Some((nacc, cc.ev.eval(&a[3])))
                },
                || {
                    let mut b = cc.ev.bindings.borrow_mut();
                    b.remove(&v);
                    b.remove(&res);
                },
            );
            match step {
                Some((na, nc)) => {
                    acc = na;
                    curv = nc;
                }
                None => break,
            }
        }
        Some(acc)
    });

    def(c, "bitand", BT, "Bitwise AND", "a:int,b:int", "", false, |_, a| Some(nb(is_i(&a[0])? & is_i(&a[1])?)));
    def(c, "bitor", BT, "Bitwise OR", "a:int,b:int", "", false, |_, a| Some(nb(is_i(&a[0])? | is_i(&a[1])?)));
    def(c, "bitxor", BT, "Bitwise XOR", "a:int,b:int", "", false, |_, a| Some(nb(is_i(&a[0])? ^ is_i(&a[1])?)));
    def(c, "bitnot", BT, "Bitwise NOT", "a:int", "", false, |_, a| Some(nb(-is_i(&a[0])? - 1)));
    def(c, "bitcmp", BT, "Bitwise Complement", "a:int,bits:pint=8,signed:bool=0", "", false, |_, a| {
        let x = int(&a[0], "bitcmp");
        let bits = small_int(&a[1], "bitcmp") as usize;
        let mask = (BigInt::one() << bits) - 1;
        let mut r = (-x - 1) & &mask;
        if !a[2].is_zero() && { let t: BigInt = &r >> (bits - 1); t.is_one() } {
            r -= BigInt::one() << bits;
        }
        Some(nb(r))
    });
    def(c, "shift", BT, "Bit Shift", "a:int,steps:int", "Shifts left for positive steps, right for negative.", false, |_, a| {
        let x = int(&a[0], "shift");
        let s = small_int(&a[1], "shift");
        if s.abs() > 1_000_000 {
            throw("shift() argument too large");
        }
        Some(nb(if s >= 0 { x << s as usize } else { x >> (-s) as usize }))
    });
    def(c, "circshift,rot,bitrot", BT, "Circular Shift", "a:int,steps:int,bits:pint=8", "", false, |_, a| {
        let mut x = int(&a[0], "circshift");
        let s = small_int(&a[1], "circshift");
        let bits = small_int(&a[2], "circshift");
        let mask = (BigInt::one() << bits as usize) - 1;
        x &= &mask;
        let s = ((s % bits) + bits) % bits;
        Some(nb(((&x << s as usize) | (&x >> (bits - s) as usize)) & mask))
    });
    def(c, "bitget,getbit", BT, "Get Bit", "a:int,bit:pint", "", false, |_, a| Some(nb((int(&a[0], "bitget") >> (small_int(&a[1], "bitget") as usize - 1)) & BigInt::one())));
    def(c, "bitset,setbit", BT, "Set Bit", "a:int,bit:pint,value:bool=1", "", false, |_, a| {
        let x = int(&a[0], "bitset");
        let m = BigInt::one() << (small_int(&a[1], "bitset") as usize - 1);
        Some(nb(if a[2].is_zero() { x & (-m - 1) } else { x | m }))
    });
    def(c, "bitcount,popcount", BT, "Count Set Bits", "a:int", "", false, |_, a| {
        let x = int(&a[0], "bitcount").abs();
        Some(ni(x.magnitude().count_ones() as i64))
    });

    def(c, "base", NB, "Number Base Conversion", "value,base:int", "Interprets the digits in the given base.", false, |_, a| {
        let b = small_int(&a[1], "base");
        match &a[0].e {
            E::Str(s) => Some(nr(parse_in_base(s, b))),
            E::Sym(s) => Some(nr(parse_in_base(s, b))),
            E::Num(n) if n.is_real() => Some(nr(parse_in_base(&formatter::plain_text(&a[0]), b))),
            _ => throw("base() requires text or digits"),
        }
    });
    def(c, "bin", NB, "Binary Number", "digits", "", false, |_, a| Some(nr(parse_in_base(&digit_text(&a[0]), 2))));
    def(c, "oct", NB, "Octal Number", "digits", "", false, |_, a| Some(nr(parse_in_base(&digit_text(&a[0]), 8))));
    def(c, "hex", NB, "Hexadecimal Number", "digits", "", false, |_, a| Some(nr(parse_in_base(&digit_text(&a[0]), 16))));
    def(c, "roman", NB, "Roman Numerals", "numerals", "", false, |_, a| Some(nb(from_roman(&digit_text(&a[0])))));
    def(c, "bijective", NB, "Bijective Base-26", "letters", "", false, |_, a| {
        let mut v = BigInt::zero();
        for ch in digit_text(&a[0]).to_uppercase().chars() {
            if !ch.is_ascii_uppercase() {
                throw("Invalid letter");
            }
            v = v * 26 + (ch as i64 - 'A' as i64 + 1);
        }
        Some(nb(v))
    });
    def(c, "bcd", NB, "Binary-Coded Decimal", "bits", "", false, |_, a| {
        let mut s = digit_text(&a[0]).replace(' ', "");
        if s.len() % 4 != 0 {
            s = format!("{:0>w$}", s, w = (s.len() / 4 + 1) * 4);
        }
        let mut sb = String::new();
        for i in (0..s.len()).step_by(4) {
            let d = u32::from_str_radix(&s[i..i + 4], 2).unwrap_or_else(|_| throw("Invalid BCD digit"));
            if d > 9 {
                throw("Invalid BCD digit");
            }
            sb.push_str(&d.to_string());
        }
        Some(nr(Real::parse(&sb, 10)))
    });
    def(c, "float,fp", NB, "Floating Point Value", "bits_text,bits:pint=32", "Decodes an IEEE 754 binary representation.", false, |_, a| Some(nr(floatformats::decode(&digit_text(&a[0]), floatformats::by_bits(small_int(&a[1], "float"))))));
    def(c, "floaterror", NB, "Floating Point Error", "x:real,bits:pint=32", "", false, |_, a| {
        let x = a[0].as_real()?.clone();
        let fmt = floatformats::by_bits(small_int(&a[1], "floaterror"));
        let bits = floatformats::encode(&x, fmt);
        let back = floatformats::decode(&floatformats::to_bits(&bits, fmt), fmt);
        Some(nr(back - x))
    });

    def(c, "len,length", UT, "Length", "x", "", false, |_, a| {
        Some(match &a[0].e {
            E::Str(s) => ni(s.encode_utf16().count() as i64),
            E::Mat(rows, _) => ni(rows.iter().map(|r| r.len()).sum::<usize>() as i64),
            _ => one(),
        })
    });
    def(c, "concatenate,concat", UT, "Concatenate Text", "a,b,...", "", false, |_, a| Some(str_e(&a.iter().map(str_of).collect::<String>())));
    def(c, "char,chr", UT, "Character", "code:pint", "", false, |_, a| Some(str_e(&char::from_u32(small_int(&a[0], "char") as u32).unwrap_or_else(|| throw("Invalid character code")).to_string())));
    def(c, "code,ascii,ord", UT, "Character Code", "text", "", false, |_, a| {
        let s = str_of(&a[0]);
        let cs: Vec<char> = s.chars().collect();
        if cs.is_empty() {
            throw("Empty text");
        }
        if cs.len() == 1 {
            return Some(ni(cs[0] as i64));
        }
        Some(vector_e(cs.iter().map(|&ch| ni(ch as i64)).collect()))
    });
    def(c, "uppercase,upper", UT, "Upper Case", "text", "", false, |_, a| Some(str_e(&str_of(&a[0]).to_uppercase())));
    def(c, "lowercase,lower", UT, "Lower Case", "text", "", false, |_, a| Some(str_e(&str_of(&a[0]).to_lowercase())));
    def(c, "nounit,strip_units", UT, "Strip Units", "x", "", false, |_, a| Some(evaluator::strip_units(&a[0])));
    def(c, "unit", UT, "Extract Unit", "x", "", false, |_, a| Some(simp::split_units_of(&a[0])));
    def(c, "value,unitvalue", UT, "Numerical Value", "x", "", false, |_, a| Some(num(simp::split_coef(&a[0]).0)));
    def(c, "convert", UT, "Convert Units", "x,unit", "", false, |_, a| {
        let u = simp::split_units_of(&a[1]);
        Some(units::convert(&a[0], if u.is_one() { &a[1] } else { &u }))
    });
    def(c, "plot", UT, "Plot", "expression:hold,min=0,max=10", "Opens the expression in the plot view.", false, |cc, a| {
        if cc.preview.get() {
            return None;
        }
        *PLOT_REQUEST.lock().unwrap() = Some((formatter::plain_text(&a[0]), dbl(&a[1], "plot"), dbl(&a[2], "plot")));
        Some(str_e("Plot opened"))
    });
    def(c, "isnan", UT, "Is Undefined", "x", "", false, |_, _| Some(zero()));

    def(c, "interval", IV, "Interval", "lower:real,upper:real", "Closed interval [lower, upper].", false, |_, a| {
        let lo = is_r(&a[0])?;
        let hi = is_r(&a[1])?;
        Some(if lo.eqv(hi) { nr(lo.clone()) } else { intv_e(lo.clone(), hi.clone()) })
    });
    def(c, "uncertainty", IV, "Value with Uncertainty", "value:real,error:real,relative:bool=0", "value ± error.", false, |_, a| {
        let v = is_r(&a[0])?;
        let mut err = is_r(&a[1])?.clone();
        if a.len() > 2 && !a[2].is_zero() {
            err = (v * &err).abs();
        }
        Some(intervals::from_mid_rad(v, &err))
    });
    def(c, "lowerendpoint,lower_endpoint", IV, "Lower Endpoint", "x", "", false, |_, a| Some(match &a[0].e {
        E::Intv(lo, _) => nr(lo.clone()),
        _ => a[0].clone(),
    }));
    def(c, "upperendpoint,upper_endpoint", IV, "Upper Endpoint", "x", "", false, |_, a| Some(match &a[0].e {
        E::Intv(_, hi) => nr(hi.clone()),
        _ => a[0].clone(),
    }));
    def(c, "midpoint", IV, "Midpoint", "x", "", false, |_, a| Some(match &a[0].e {
        E::Intv(lo, hi) => nr((lo + hi) / Real::two()),
        _ => a[0].clone(),
    }));
    def(c, "errorrange,radius", IV, "Interval Radius", "x", "", false, |_, a| Some(match &a[0].e {
        E::Intv(lo, hi) => nr((hi - lo) / Real::two()),
        _ => zero(),
    }));

    def(c, "now", DT, "Current Time", "", "", false, |_, _| Some(date_e(Local::now().naive_local(), true, false)));
    def(c, "today", DT, "Today", "", "", false, |_, _| Some(date_e(today(), false, false)));
    def(c, "tomorrow", DT, "Tomorrow", "", "", false, |_, _| Some(date_e(today() + Duration::days(1), false, false)));
    def(c, "yesterday", DT, "Yesterday", "", "", false, |_, _| Some(date_e(today() - Duration::days(1), false, false)));
    def(c, "date", DT, "Date", "year:int,month:int=1,day:int=1", "", false, |_, a| {
        if let E::Str(s) = &a[0].e {
            let (d, ht, u) = parse_date(s);
            return Some(date_e(d, ht, u));
        }
        Some(date_e(mk_date(small_int(&a[0], "date"), small_int(&a[1], "date"), small_int(&a[2], "date")), false, false))
    });
    def(c, "datetime", DT, "Date and Time", "year:int,month:int=1,day:int=1,hour:int=0,minute:int=0,second:real=0", "", false, |_, a| {
        let d = mk_date(small_int(&a[0], "datetime"), small_int(&a[1], "datetime"), small_int(&a[2], "datetime"));
        let d = d + Duration::hours(small_int(&a[3], "datetime") as i64) + Duration::minutes(small_int(&a[4], "datetime") as i64);
        Some(date_e(d + Duration::microseconds((dbl(&a[5], "datetime") * 1e6).round() as i64), true, false))
    });
    def(c, "year", DT, "Year", "date:date=today()", "", false, |_, a| Some(ni(dates::get(&a[0]).year() as i64)));
    def(c, "month", DT, "Month", "date:date=today()", "", false, |_, a| Some(ni(dates::get(&a[0]).month() as i64)));
    def(c, "day", DT, "Day of Month", "date:date=today()", "", false, |_, a| Some(ni(dates::get(&a[0]).day() as i64)));
    def(c, "hour", DT, "Hour", "date:date=now()", "", false, |_, a| Some(ni(dates::get(&a[0]).hour() as i64)));
    def(c, "minute", DT, "Minute", "date:date=now()", "", false, |_, a| Some(ni(dates::get(&a[0]).minute() as i64)));
    def(c, "second", DT, "Second", "date:date=now()", "", false, |_, a| Some(ni(dates::get(&a[0]).second() as i64)));
    def(c, "weekday", DT, "Day of Week", "date:date=today(),sunday_first:bool=0", "1 = Monday … 7 = Sunday.", false, |_, a| {
        let d = dates::get(&a[0]);
        let w = d.weekday().num_days_from_sunday() as i64;
        Some(ni(if a[1].is_zero() { if w == 0 { 7 } else { w } } else { w + 1 }))
    });
    def(c, "week", DT, "Week Number", "date:date=today()", "ISO 8601 week number.", false, |_, a| Some(ni(dates::get(&a[0]).iso_week().week() as i64)));
    def(c, "yearday", DT, "Day of Year", "date:date=today()", "", false, |_, a| Some(ni(dates::get(&a[0]).ordinal() as i64)));
    def(c, "days", DT, "Days Between Dates", "date1:date,date2:date", "", false, |_, a| Some(nr(Real::int((dates::get(&a[1]).date() - dates::get(&a[0]).date()).num_days()))));
    def(c, "yearfrac", DT, "Year Fraction", "date1:date,date2:date", "", false, |_, a| Some(nr(ticks_real(dates::get(&a[1]) - dates::get(&a[0])) / Real::int(86400) / Real::frac(3652425, 10000))));
    def(c, "adddays", DT, "Add Days", "date:date,days:real", "", false, |_, a| Some(dates::add_seconds(&dates::as_date(&a[0]), &(a[1].as_real()?.clone() * Real::int(86400)))));
    def(c, "addmonths", DT, "Add Months", "date:date,months:int", "", false, |_, a| {
        let (t, h, u) = date_of(&a[0]);
        Some(date_e(dates::add_months(t, small_int(&a[1], "addmonths") as i64), h, u))
    });
    def(c, "addyears", DT, "Add Years", "date:date,years:int", "", false, |_, a| {
        let (t, h, u) = date_of(&a[0]);
        Some(date_e(dates::add_months(t, small_int(&a[1], "addyears") as i64 * 12), h, u))
    });
    def(c, "timestamp,unixtime", DT, "Unix Timestamp", "date:date=now()", "", false, |_, a| {
        let (t, _, u) = date_of(&a[0]);
        let utc = if u { t } else { dates::local_to_utc(t) };
        Some(nr(ticks_real(utc - chrono::DateTime::UNIX_EPOCH.naive_utc())))
    });
    def(c, "stamptodate,fromunix", DT, "Timestamp to Date", "timestamp:real", "", false, |_, a| {
        let secs = dbl(&a[0], "stamptodate");
        let utc = chrono::DateTime::UNIX_EPOCH.naive_utc() + Duration::microseconds((secs * 1e6).round() as i64);
        Some(date_e(dates::utc_to_local(utc), true, false))
    });
    def(c, "lunarphase", DT, "Lunar Phase", "date:date=now()", "0 = new moon, 0.5 = full moon.", false, |_, a| Some(nd(dates::lunar_phase(dates::get(&a[0])))));
    def(c, "nextlunarphase", DT, "Next Lunar Phase", "phase:real=0.5,date:date=now()", "", false, |_, a| Some(date_e(dates::next_lunar_phase(dates::get(&a[1]), dbl(&a[0], "nextlunarphase")), true, false)));
    def(c, "easter", DT, "Easter Sunday", "year:int", "", false, |_, a| Some(date_e(dates::easter(small_int(&a[0], "easter")), false, false)));
    def(c, "isleapyear,leapyear", DT, "Is Leap Year", "year:int", "", false, |_, a| {
        let y = small_int(&a[0], "isleapyear");
        Some(bool_e((y % 4 == 0 && y % 100 != 0) || y % 400 == 0))
    });
    def(c, "daysinmonth", DT, "Days in Month", "year:int,month:int", "", false, |_, a| {
        let y = small_int(&a[0], "daysinmonth");
        let m = small_int(&a[1], "daysinmonth");
        let first = mk_date(y, m, 1);
        let next = dates::add_months(first, 1);
        Some(ni((next - first).num_days()))
    });

    def(c, "pv", EC, "Present Value", "rate:real,nper:real,pmt:real,fv:real=0,type:bool=0", "", false, |_, a| fin(a, |r, n, p, f, t| if r == 0.0 { -(f + p * n) } else { -(f + p * (1.0 + r * t) * ((1.0 + r).powf(n) - 1.0) / r) / (1.0 + r).powf(n) }));
    def(c, "fv", EC, "Future Value", "rate:real,nper:real,pmt:real,pv:real=0,type:bool=0", "", false, |_, a| fin(a, |r, n, p, v, t| if r == 0.0 { -(v + p * n) } else { -(v * (1.0 + r).powf(n) + p * (1.0 + r * t) * ((1.0 + r).powf(n) - 1.0) / r) }));
    def(c, "pmt", EC, "Payment", "rate:real,nper:real,pv:real,fv:real=0,type:bool=0", "", false, |_, a| fin(a, |r, n, v, f, t| if r == 0.0 { -(v + f) / n } else { -(r * (f + v * (1.0 + r).powf(n))) / ((1.0 + r * t) * ((1.0 + r).powf(n) - 1.0)) }));
    def(c, "nper", EC, "Number of Periods", "rate:real,pmt:real,pv:real,fv:real=0,type:bool=0", "", false, |_, a| fin(a, |r, p, v, f, t| if r == 0.0 { -(v + f) / p } else { ((p * (1.0 + r * t) - f * r) / (p * (1.0 + r * t) + v * r)).ln() / (1.0 + r).ln() }));
    def(c, "rate", EC, "Interest Rate", "nper:real,pmt:real,pv:real,fv:real=0,type:bool=0,guess:real=0.1", "", false, |_, a| {
        if !all_num(a) {
            return None;
        }
        let (n, p, v, f, t) = (dbl(&a[0], "rate"), dbl(&a[1], "rate"), dbl(&a[2], "rate"), dbl(&a[3], "rate"), dbl(&a[4], "rate"));
        let g = |r: f64| if r.abs() < 1e-12 { v + p * n + f } else { v * (1.0 + r).powf(n) + p * (1.0 + r * t) * ((1.0 + r).powf(n) - 1.0) / r + f };
        let res = solver::newton(&g, dbl(&a[5], "rate"));
        if res.is_nan() {
            throw("rate() did not converge");
        }
        Some(nd(res))
    });
    def(c, "npv", EC, "Net Present Value", "rate:real,values:vec", "", false, |_, a| {
        let vals = items(&a[1..]);
        Some(sum(vals.iter().enumerate().map(|(i, v)| simp::div(v, &simp::pow(&simp::add2(&one(), &a[0]), &ni(i as i64 + 1)))).collect()))
    });
    def(c, "irr", EC, "Internal Rate of Return", "values:vec,guess:real=0.1", "", false, |_, a| {
        let vals: Vec<f64> = reals(&a[..1], "irr").iter().map(|r| r.to_f64()).collect();
        let g = |r: f64| vals.iter().enumerate().map(|(i, v)| v / (1.0 + r).powi(i as i32)).sum::<f64>();
        let res = solver::newton(&g, dbl(&a[1], "irr"));
        if res.is_nan() {
            throw("irr() did not converge");
        }
        Some(nd(res))
    });
    def(c, "ipmt", EC, "Interest Payment", "rate:real,per:real,nper:real,pv:real,fv:real=0,type:bool=0", "", false, |_, a| {
        if !all_num(a) {
            return None;
        }
        let (r, per, n, v, f) = (dbl(&a[0], "ipmt"), dbl(&a[1], "ipmt"), dbl(&a[2], "ipmt"), dbl(&a[3], "ipmt"), dbl(&a[4], "ipmt"));
        let t = dbl(&a[5], "ipmt") as i32;
        let tf = t as f64;
        let p = -(r * (f + v * (1.0 + r).powf(n))) / ((1.0 + r * tf) * ((1.0 + r).powf(n) - 1.0));
        let fv_before = -(v * (1.0 + r).powf(per - 1.0) + p * (1.0 + r * tf) * ((1.0 + r).powf(per - 1.0) - 1.0) / r);
        let mut ip = fv_before * r;
        if t == 1 {
            ip = if per == 1.0 { 0.0 } else { (fv_before - p) * r };
        }
        Some(nd(ip))
    });
    def(c, "ppmt", EC, "Principal Payment", "rate:real,per:real,nper:real,pv:real,fv:real=0,type:bool=0", "", false, |cc, a| {
        let p = cc.ev.call("pmt", vec![a[0].clone(), a[2].clone(), a[3].clone(), a[4].clone(), a[5].clone()]);
        let ip = cc.ev.call("ipmt", a.to_vec());
        Some(simp::sub(&p, &ip))
    });
    def(c, "sln", EC, "Straight Line Depreciation", "cost:real,salvage:real,life:real", "", false, |_, a| Some(simp::div(&simp::sub(&a[0], &a[1]), &a[2])));
    def(c, "syd", EC, "Sum-of-Years Depreciation", "cost:real,salvage:real,life:real,period:real", "", false, |_, a| {
        Some(simp::div(&simp::mul(vec![simp::sub(&a[0], &a[1]), simp::add2(&simp::sub(&a[2], &a[3]), &one()), two()]), &simp::mul2(&a[2], &simp::add2(&a[2], &one()))))
    });
    def(c, "ddb", EC, "Double Declining Balance", "cost:real,salvage:real,life:real,period:real,factor:real=2", "", false, |_, a| {
        if !all_num(a) {
            return None;
        }
        let (cost, salv, life, per, fac) = (dbl(&a[0], "ddb"), dbl(&a[1], "ddb"), dbl(&a[2], "ddb"), dbl(&a[3], "ddb"), dbl(&a[4], "ddb"));
        let mut book = cost;
        let mut dep = 0.0;
        let mut i = 1.0;
        while i <= per {
            dep = (book * fac / life).min((book - salv).max(0.0));
            book -= dep;
            i += 1.0;
        }
        Some(nd(dep))
    });
    def(c, "effect", EC, "Effective Annual Rate", "nominal:real,periods:pint", "", false, |_, a| Some(simp::sub(&simp::pow(&simp::add2(&one(), &simp::div(&a[0], &a[1])), &a[1]), &one())));
    def(c, "nominal", EC, "Nominal Annual Rate", "effective:real,periods:pint", "", false, |_, a| Some(simp::mul2(&a[1], &simp::sub(&simp::pow(&simp::add2(&one(), &a[0]), &simp::pow(&a[1], &minus_one())), &one()))));
    def(c, "compound", EC, "Compound Interest", "principal:real,rate:real,years:real,periods:pint=1", "", false, |_, a| Some(simp::mul2(&a[0], &simp::pow(&simp::add2(&one(), &simp::div(&a[1], &a[3])), &simp::mul2(&a[3], &a[2])))));

    let m = simp::mul;
    def(c, "area_circle,circlearea", GE, "Area of Circle", "radius", "", false, move |_, a| Some(m(vec![pi(), simp::pow(&a[0], &two())])));
    def(c, "circumference", GE, "Circumference of Circle", "radius", "", false, move |_, a| Some(m(vec![two(), pi(), a[0].clone()])));
    def(c, "area_sector", GE, "Area of Circle Sector", "radius,angle:angle", "", false, move |_, a| Some(m(vec![half(), simp::pow(&a[0], &two()), to_radians(&a[1])])));
    def(c, "area_ellipse", GE, "Area of Ellipse", "a,b", "", false, move |_, a| Some(m(vec![pi(), a[0].clone(), a[1].clone()])));
    def(c, "area_rectangle", GE, "Area of Rectangle", "length,width", "", false, |_, a| Some(simp::mul2(&a[0], &a[1])));
    def(c, "perimeter_rectangle", GE, "Perimeter of Rectangle", "length,width", "", false, |_, a| Some(simp::mul2(&two(), &simp::add2(&a[0], &a[1]))));
    def(c, "area_triangle", GE, "Area of Triangle", "base,height", "", false, move |_, a| Some(m(vec![half(), a[0].clone(), a[1].clone()])));
    def(c, "heron", GE, "Area of Triangle (Heron)", "a,b,c", "", false, move |_, a| {
        let s = simp::div(&simp::add(a[..3].to_vec()), &two());
        Some(simp::pow(&m(vec![s.clone(), simp::sub(&s, &a[0]), simp::sub(&s, &a[1]), simp::sub(&s, &a[2])]), &half()))
    });
    def(c, "area_trapezoid", GE, "Area of Trapezoid", "a,b,height", "", false, move |_, a| Some(m(vec![half(), simp::add2(&a[0], &a[1]), a[2].clone()])));
    def(c, "area_parallelogram", GE, "Area of Parallelogram", "base,height", "", false, |_, a| Some(simp::mul2(&a[0], &a[1])));
    def(c, "area_polygon", GE, "Area of Regular Polygon", "sides,length", "", false, |cc, a| {
        let t = cc.ev.eval(&call_e(cc.fn_("tan"), vec![from_radians(&simp::div(&pi(), &a[0]))]));
        Some(simp::div(&simp::mul2(&a[0], &simp::pow(&a[1], &two())), &simp::mul2(&ni(4), &t)))
    });
    def(c, "volume_sphere", GE, "Volume of Sphere", "radius", "", false, move |_, a| Some(m(vec![num_r(Real::frac(4, 3)), pi(), simp::pow(&a[0], &ni(3))])));
    def(c, "area_sphere", GE, "Surface Area of Sphere", "radius", "", false, move |_, a| Some(m(vec![ni(4), pi(), simp::pow(&a[0], &two())])));
    def(c, "volume_cylinder", GE, "Volume of Cylinder", "radius,height", "", false, move |_, a| Some(m(vec![pi(), simp::pow(&a[0], &two()), a[1].clone()])));
    def(c, "area_cylinder", GE, "Surface Area of Cylinder", "radius,height", "", false, move |_, a| Some(m(vec![two(), pi(), a[0].clone(), simp::add2(&a[0], &a[1])])));
    def(c, "volume_cone", GE, "Volume of Cone", "radius,height", "", false, move |_, a| Some(m(vec![num_r(Real::frac(1, 3)), pi(), simp::pow(&a[0], &two()), a[1].clone()])));
    def(c, "area_cone", GE, "Surface Area of Cone", "radius,height", "", false, move |_, a| {
        Some(m(vec![pi(), a[0].clone(), simp::add2(&a[0], &simp::pow(&simp::add2(&simp::pow(&a[0], &two()), &simp::pow(&a[1], &two())), &half()))]))
    });
    def(c, "volume_box", GE, "Volume of Box", "length,width,height", "", false, move |_, a| Some(m(a[..3].to_vec())));
    def(c, "area_box", GE, "Surface Area of Box", "length,width,height", "", false, |_, a| {
        Some(simp::mul2(&two(), &simp::add(vec![simp::mul2(&a[0], &a[1]), simp::mul2(&a[1], &a[2]), simp::mul2(&a[0], &a[2])])))
    });
    def(c, "volume_cube", GE, "Volume of Cube", "side", "", false, |_, a| Some(simp::pow(&a[0], &ni(3))));
    def(c, "area_cube", GE, "Surface Area of Cube", "side", "", false, |_, a| Some(simp::mul2(&ni(6), &simp::pow(&a[0], &two()))));
    def(c, "volume_pyramid", GE, "Volume of Pyramid", "base_area,height", "", false, move |_, a| Some(m(vec![num_r(Real::frac(1, 3)), a[0].clone(), a[1].clone()])));
    def(c, "pythagoras", GE, "Pythagorean Theorem", "a,b", "", false, |_, a| Some(simp::pow(&simp::add2(&simp::pow(&a[0], &two()), &simp::pow(&a[1], &two())), &half())));

    raw(c, "atom,element", "Data Sets", "Element Data", "element,property=\"\"", "Looks up a property of a chemical element, e.g. atom(Fe, weight).", |cc, a| Some(data::lookup(cc, "atom", &a[0], &if a.len() > 1 { str_of(&a[1]) } else { String::new() })));
    raw(c, "planet", "Data Sets", "Planet Data", "planet,property=\"\"", "Looks up a property of a planet, e.g. planet(Mars, mass).", |cc, a| Some(data::lookup(cc, "planet", &a[0], &if a.len() > 1 { str_of(&a[1]) } else { String::new() })));
}

// sums the head directly, then estimates the tail
fn infinite_sum(c: &Calculator, term: &Expr, v: &str, start: i64) -> f64 {
    c.with_exact(false, || {
        let f = |x: f64| c.ev.eval_double(term, v, x);
        const HEAD: i64 = 2000;
        let mut acc = 0.0;
        let mut partial = vec![];
        let mut sign_changes = 0;
        let mut last = 0.0;
        for k in start..start + HEAD {
            c.check_abort();
            let t = f(k as f64);
            if t.is_nan() {
                throw("Undefined term in series");
            }
            if k > start && t.signum() != last.signum() && t != 0.0 {
                sign_changes += 1;
            }
            last = t;
            acc += t;
            partial.push(acc);
            if t == 0.0 && k > start + 50 {
                return acc;
            }
            if t.abs() < 1e-18 * 1f64.max(acc.abs()) && k > start + 20 {
                return acc;
            }
        }
        if sign_changes > HEAD / 2 {
            let mut tail: Vec<f64> = partial[partial.len() - 24..].to_vec();
            while tail.len() > 1 {
                tail = tail.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect();
            }
            return tail[0];
        }
        let n = (start + HEAD) as f64;
        let fnv = f(n);
        if fnv.abs() > f(n - 1.0).abs() * 1.0000001 {
            throw("The series does not converge");
        }
        let tail = calculus::num_integrate(&f, n, f64::INFINITY).0;
        let d1 = f(n + 0.5) - f(n - 0.5);
        acc + tail + fnv / 2.0 - d1 / 12.0
    })
}

fn hold(c: &Calculator, e: &Expr) -> Expr {
    if e.is_num() || e.is_sym() {
        e.clone()
    } else {
        call_e(c.fn_("_hold"), vec![e.clone()])
    }
}

pub fn unhold(e: &Expr) -> Expr {
    e.map(&mut |x| match &x.e {
        E::Call(f, a) if f.name() == "_hold" => a[0].clone(),
        _ => x,
    })
}

fn series_op(c: &Calculator, a: &[Expr], is_sum: bool) -> R {
    let v = sn(&a[3]);
    let lo = c.ev.eval(&a[1]);
    let hi = c.ev.eval(&a[2]);
    if let (Some(l), E::Num(hn)) = (is_i(&lo), &hi.e) {
        if hn.re.is_infinite() && is_sum {
            return Some(nd(infinite_sum(c, &c.ev.eval(&a[0]), &v, l.to_i64().unwrap_or(0))));
        }
    }
    if let (Some(from), Some(to)) = (is_i(&lo), is_i(&hi)) {
        if &to - &from > big(1000000) {
            throw("Too many terms");
        }
        let mut parts = vec![];
        let mut k = from;
        while k <= to {
            c.check_abort();
            parts.push(c.ev.eval_with(&a[0], &v, nb(k.clone())));
            k += 1;
        }
        return Some(if is_sum { simp::add(parts) } else { simp::mul(parts) });
    }
    if is_sum {
        if let Some(closed) = faulhaber(c, &a[0], &v, &lo, &hi) {
            return Some(closed);
        }
    }
    None
}

// closed form sum of a polynomial term
fn faulhaber(c: &Calculator, term: &Expr, v: &str, lo: &Expr, hi: &Expr) -> R {
    let t = c.ev.eval(term);
    let cs = polynomials::try_coefficients(&t, v)?;
    let n = sym("_n");
    let mut total = vec![];
    for (p, cp) in cs.iter().enumerate() {
        if cp.is_zero() {
            continue;
        }
        let mut parts = vec![];
        for j in 0..=p {
            let mut b = primes::bernoulli(j as i64);
            if j == 1 {
                b = -b;
            }
            parts.push(simp::mul2(&nr(Real::big_int(primes::binomial(&big(p as i64 + 1), &big(j as i64))) * b), &simp::pow(&n, &ni((p + 1 - j) as i64))));
        }
        let spoly = simp::div(&simp::add(parts), &ni(p as i64 + 1));
        total.push(simp::mul2(cp, &spoly));
    }
    let s = simp::add(total);
    let upper = c.ev.eval_with(&s, "_n", hi.clone());
    let lower = c.ev.eval_with(&s, "_n", simp::sub(lo, &one()));
    Some(simp::expand(&simp::sub(&upper, &lower)))
}

fn is_name(e: &Expr) -> bool {
    match &e.e {
        E::Sym(_) | E::Str(_) => true,
        E::Var(v) => v.is_unknown,
        _ => false,
    }
}

fn integral(c: &Calculator, a: &[Expr]) -> R {
    let (mut var_arg, mut lo_arg, mut hi_arg) = (a.get(1).cloned(), a.get(2).cloned(), a.get(3).cloned());
    // also accepts integrate(f, lower, upper, var)
    if let Some(va) = &var_arg {
        if !is_name(va) {
            let l = va.clone();
            var_arg = hi_arg;
            hi_arg = lo_arg;
            lo_arg = Some(l);
        }
    }
    let v = sym_name(var_arg.as_ref(), "x");
    let f = c.ev.eval(&a[0]);
    let lo = lo_arg.map(|x| c.ev.eval(&x));
    let hi = hi_arg.map(|x| c.ev.eval(&x));
    let anti = calculus::integrate(&f, &v);
    let (Some(lo), Some(hi)) = (lo, hi) else {
        return Some(anti.unwrap_or_else(|| throw("Unable to find an antiderivative")));
    };
    let inf = |e: &Expr| matches!(&e.e, E::Num(n) if n.is_infinite());
    if let Some(anti) = &anti {
        if !inf(&lo) && !inf(&hi) {
            if let Ok(r) = attempt(|| simp::sub(&c.ev.eval_with(anti, &v, hi.clone()), &c.ev.eval_with(anti, &v, lo.clone()))) {
                if !r.any_of(&mut |z| matches!(&z.e, E::Call(f, _) if f.name() == "abs" || f.name() == "ln") && z.has_symbols()) {
                    return Some(r);
                }
            }
        }
    }
    if lo.has_symbols() || hi.has_symbols() {
        return None;
    }
    let dlo = match &lo.e {
        E::Num(n) => n.re.to_f64(),
        _ => dbl(&c.ev.eval(&calculator::to_approx(&lo)), "integrate"),
    };
    let dhi = match &hi.e {
        E::Num(n) => n.re.to_f64(),
        _ => dbl(&c.ev.eval(&calculator::to_approx(&hi)), "integrate"),
    };
    let val = calculus::num_integrate(&|x| c.ev.eval_double(&f, &v, x), dlo, dhi).0;
    Some(nd(val))
}

fn variance(l: &[Expr], sample: bool) -> Expr {
    let n = l.len() as i64;
    if n < if sample { 2 } else { 1 } {
        throw("Not enough values");
    }
    let mean = simp::div(&simp::add(l.to_vec()), &ni(n));
    let sq: Vec<Expr> = l.iter().map(|x| simp::pow(&simp::sub(x, &mean), &two())).collect();
    simp::div(&simp::expand(&simp::add(sq)), &ni(if sample { n - 1 } else { n }))
}

fn quantity_extreme(_c: &Calculator, l: &[Expr], dir: i32) -> R {
    let mut best: Option<Expr> = None;
    let mut best_v: Option<Real> = None;
    for e in l {
        let (v, _, rest) = units::try_quantity(e)?;
        if !rest.is_one() || !v.is_real() {
            return None;
        }
        let better = match &best_v {
            None => true,
            Some(b) => {
                if dir > 0 {
                    v.re.gt(b)
                } else {
                    v.re.lt(b)
                }
            }
        };
        if better {
            best = Some(e.clone());
            best_v = Some(v.re);
        }
    }
    best
}

fn random_big(max: &BigInt) -> BigInt {
    if !max.is_positive() {
        return BigInt::zero();
    }
    if let Some(m) = max.to_u64() {
        return big(rng_next(m) as i64);
    }
    let words = (max.bits() as usize).div_ceil(64) + 1;
    let mut r = BigInt::zero();
    for _ in 0..words {
        r = (r << 64usize) + rng_next(0);
    }
    r.mod_floor(max)
}

fn digit_text(e: &Expr) -> String {
    match &e.e {
        E::Str(s) => s.clone(),
        E::Sym(s) => s.clone(),
        E::Num(n) if n.is_real() && n.re.is_int() => n.re.n.to_string(),
        _ => formatter::plain_text(e),
    }
}

pub fn parse_in_base(s: &str, b: i32) -> Real {
    let mut s = s.trim().replace([' ', '_'], "");
    let neg = s.starts_with('-') || s.starts_with('−');
    if neg {
        s = s.chars().skip(1).collect();
    }
    let lower = s.to_lowercase();
    if (b == 16 && lower.starts_with("0x")) || (b == 2 && lower.starts_with("0b")) || (b == 8 && lower.starts_with("0o")) {
        s = s[2..].to_string();
    }
    if !(2..=36).contains(&b) {
        throw("Base must be between 2 and 36");
    }
    let r = Real::parse(&s, b as u32);
    if neg {
        -r
    } else {
        r
    }
}

fn fin(a: &[Expr], f: fn(f64, f64, f64, f64, f64) -> f64) -> R {
    if !a.iter().all(|z| z.is_real_num()) {
        return None;
    }
    let mut d: Vec<f64> = a.iter().map(|z| z.num().re.to_f64()).collect();
    while d.len() < 5 {
        d.push(0.0);
    }
    Some(nd(f(d[0], d[1], d[2], d[3], d[4])))
}