// arbitrary precision transcendental functions on fixed point integers (value * 2^p)
use super::real::{big, bit_len, bits, div_round, iroot, isqrt};
use super::{throw, BigInt, Real};
use num_integer::Integer;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use std::collections::HashMap;
use std::sync::Mutex;
static PI_CACHE: Mutex<Option<HashMap<i64, BigInt>>> = Mutex::new(None);
static LN2_CACHE: Mutex<Option<HashMap<i64, BigInt>>> = Mutex::new(None);

fn p() -> i64 {
    bits() as i64 + 32
}

fn fix(x: &Real, p: i64) -> BigInt {
    if p >= 0 {
        div_round(&(&x.n << (p as usize)), &x.d)
    } else {
        div_round(&x.n, &(&x.d << ((-p) as usize)))
    }
}

fn unfix(v: BigInt, p: i64) -> Real {
    if p >= 0 {
        Real::make(v, BigInt::one() << (p as usize), true)
    } else {
        Real::make(v << ((-p) as usize), BigInt::one(), true)
    }
}

fn atan_inv(n: i64, p: i64) -> BigInt {
    let x = (BigInt::one() << (p as usize)) / n;
    let n2 = big(n * n);
    let mut sum = x.clone();
    let mut term = x;
    let mut k = 1i64;
    let mut neg = true;
    while !term.is_zero() {
        term /= &n2;
        k += 2;
        if neg {
            sum -= &term / k;
        } else {
            sum += &term / k;
        }
        neg = !neg;
    }
    sum
}

pub fn pi_fix(p: i64) -> BigInt {
    let mut g = PI_CACHE.lock().unwrap();
    let m = g.get_or_insert_with(HashMap::new);
    if let Some(v) = m.get(&p) {
        return v.clone();
    }
    let gp = p + 16;
    let pi: BigInt = (atan_inv(5, gp) * 16 - atan_inv(239, gp) * 4) >> 16usize;
    m.insert(p, pi.clone());
    pi
}

fn atanh_fix(t: &BigInt, p: i64) -> BigInt {
    let t2 = (t * t) >> (p as usize);
    let mut term = t.clone();
    let mut sum = t.clone();
    let mut k = 1i64;
    while !term.is_zero() {
        term = (&term * &t2) >> (p as usize);
        k += 2;
        sum += &term / k;
    }
    sum
}

pub fn ln2_fix(p: i64) -> BigInt {
    let mut g = LN2_CACHE.lock().unwrap();
    let m = g.get_or_insert_with(HashMap::new);
    if let Some(v) = m.get(&p) {
        return v.clone();
    }
    let gp = p + 16;
    let third = (BigInt::one() << (gp as usize)) / 3;
    let r: BigInt = (atanh_fix(&third, gp) * 2) >> 16usize;
    m.insert(p, r.clone());
    r
}

pub fn pi() -> Real {
    unfix(pi_fix(p()), p())
}
pub fn e() -> Real {
    exp(&Real::one())
}

fn exp_fix_small(r: &BigInt, p: i64) -> BigInt {
    let s = ((p as f64).sqrt() as i64) / 2 + 1;
    let q = p + s + 8;
    let x: BigInt = (r << ((q - p) as usize)) >> (s as usize);
    let one = BigInt::one() << (q as usize);
    let mut sum = one.clone();
    let mut term = one;
    let mut k = 1i64;
    loop {
        term = ((&term * &x) >> (q as usize)) / k;
        if term.is_zero() {
            break;
        }
        sum += &term;
        k += 1;
    }
    for _ in 0..s {
        sum = (&sum * &sum) >> (q as usize);
    }
    sum >> ((q - p) as usize)
}

pub fn exp(x: &Real) -> Real {
    if x.is_infinite() {
        return if x.sign() > 0 { Real::pos_inf() } else { Real::zero() };
    }
    if x.is_zero() {
        return Real::one();
    }
    let xd = x.to_f64();
    if xd > 1e6 {
        throw("Overflow in exp()");
    }
    if xd < -1e6 {
        return Real::zero().as_approx();
    }
    let k = (xd / 0.6931471805599453).round() as i64;
    let p = p() + bit_len(&big(k)) + 4;
    let xx = fix(x, p) - ln2_fix(p) * k;
    let e = exp_fix_small(&xx, p);
    unfix(e, p - k)
}

pub fn ln(x: &Real) -> Real {
    if x.sign() <= 0 {
        throw("Logarithm of non-positive number");
    }
    if x.is_infinite() {
        return Real::pos_inf();
    }
    if x.is_one() {
        return Real::zero();
    }
    let p = p();
    let e = bit_len(&x.n) - bit_len(&x.d);
    let m = if p - e >= 0 { div_round(&(&x.n << ((p - e) as usize)), &x.d) } else { div_round(&x.n, &(&x.d << ((e - p) as usize))) };
    let one = BigInt::one() << (p as usize);
    let t = ((&m - &one) << (p as usize)) / (&m + &one);
    let lnm = atanh_fix(&t, p) * 2;
    unfix(lnm + ln2_fix(p) * e, p)
}

fn sin_cos_fix(r: &BigInt, p: i64) -> (BigInt, BigInt) {
    let r2 = (r * r) >> (p as usize);
    let mut term = r.clone();
    let mut s = r.clone();
    let mut k = 1i64;
    loop {
        term = -(((&term * &r2) >> (p as usize)) / ((k + 1) * (k + 2)));
        if term.is_zero() {
            break;
        }
        s += &term;
        k += 2;
    }
    let one = BigInt::one() << (p as usize);
    term = one.clone();
    let mut c = one;
    let mut k = 0i64;
    loop {
        term = -(((&term * &r2) >> (p as usize)) / ((k + 1) * (k + 2)));
        if term.is_zero() {
            break;
        }
        c += &term;
        k += 2;
    }
    (s, c)
}

fn reduce(x: &Real) -> (BigInt, i32, i64) {
    let xd = x.to_f64();
    if xd.abs() > 1e300 {
        throw("Argument too large for trigonometric function");
    }
    let kf = (xd / 1.5707963267948966).round();
    let mut k = BigInt::from_f64(kf).unwrap_or_default();
    let p = p() + bit_len(&k) + 8;
    let half_pi = pi_fix(p) >> 1usize;
    let mut r = fix(x, p) - &k * &half_pi;
    if r > &half_pi / 2u32 {
        r -= &half_pi;
        k += 1;
    } else if r < -(&half_pi / 2u32) {
        r += &half_pi;
        k -= 1;
    }
    let q: i32 = k.mod_floor(&BigInt::from(4)).to_i32().unwrap_or(0);
    (r, q, p)
}

pub fn sin(x: &Real) -> Real {
    if x.is_zero() {
        return Real::zero();
    }
    if x.is_infinite() {
        throw("sin(∞) is undefined");
    }
    let (r, q, p) = reduce(x);
    let (s, c) = sin_cos_fix(&r, p);
    match q {
        0 => unfix(s, p),
        1 => unfix(c, p),
        2 => unfix(-s, p),
        _ => unfix(-c, p),
    }
}

pub fn cos(x: &Real) -> Real {
    if x.is_zero() {
        return Real::one();
    }
    if x.is_infinite() {
        throw("cos(∞) is undefined");
    }
    let (r, q, p) = reduce(x);
    let (s, c) = sin_cos_fix(&r, p);
    match q {
        0 => unfix(c, p),
        1 => unfix(-s, p),
        2 => unfix(-c, p),
        _ => unfix(s, p),
    }
}

pub fn tan(x: &Real) -> Real {
    let c = cos(x);
    if c.is_zero() {
        throw("tan() is undefined here");
    }
    sin(x) / c
}

pub fn atan(x: &Real) -> Real {
    if x.is_zero() {
        return Real::zero();
    }
    if x.is_infinite() {
        return if x.sign() > 0 { pi() / Real::two() } else { -(pi() / Real::two()) };
    }
    let mut x = x.clone();
    let neg = x.sign() < 0;
    if neg {
        x = -x;
    }
    let inv = x.gt(&Real::one());
    if inv {
        x = Real::one() / x;
    }
    let p = p() + 8;
    let mut xx = fix(&x, p);
    let one = BigInt::one() << (p as usize);
    for _ in 0..3 {
        let root = isqrt(&((&one + ((&xx * &xx) >> (p as usize))) << (p as usize)));
        xx = (&xx << (p as usize)) / (&one + root);
    }
    let x2 = (&xx * &xx) >> (p as usize);
    let mut term = xx.clone();
    let mut sum = xx;
    let mut k = 1i64;
    loop {
        term = -((&term * &x2) >> (p as usize));
        let t = &term / (2 * k + 1);
        if t.is_zero() {
            break;
        }
        sum += t;
        k += 1;
    }
    sum <<= 3usize;
    if inv {
        sum = (pi_fix(p) >> 1usize) - sum;
    }
    if neg {
        sum = -sum;
    }
    unfix(sum, p)
}

pub fn atan2(y: &Real, x: &Real) -> Real {
    if x.is_zero() {
        if y.is_zero() {
            return Real::zero();
        }
        return if y.sign() > 0 { pi() / Real::two() } else { -(pi() / Real::two()) };
    }
    let a = atan(&(y / x));
    if x.sign() > 0 {
        return a;
    }
    if y.sign() >= 0 {
        a + pi()
    } else {
        a - pi()
    }
}

pub fn sqrt(x: &Real) -> Real {
    if x.sign() < 0 {
        throw("Square root of negative number");
    }
    if x.is_zero() || x.is_infinite() {
        return x.clone();
    }
    if let Some(ex) = x.exact_root(2) {
        return ex;
    }
    let p = p();
    unfix(isqrt(&fix(x, 2 * p)), p)
}

pub fn root(x: &Real, n: i32) -> Real {
    if n == 2 {
        return sqrt(x);
    }
    if let Some(ex) = x.exact_root(n as u32) {
        return ex;
    }
    if x.sign() < 0 {
        if n % 2 == 0 {
            throw("Even root of negative number");
        }
        return -root(&-x, n);
    }
    if x.is_zero() {
        return x.clone();
    }
    let p = p();
    let v = fix(x, p * n as i64);
    unfix(iroot(&v, n as u32), p)
}

pub fn pow(x: &Real, y: &Real) -> Real {
    if y.is_exact_int() {
        return x.pow_int(&y.n);
    }
    if y.is_int() && bit_len(&y.n) < 60 {
        return x.as_approx().pow_int(&y.n);
    }
    if x.is_zero() {
        if y.sign() > 0 {
            return Real::zero();
        }
        throw("Division by zero");
    }
    if x.sign() < 0 {
        throw("Negative base with fractional exponent");
    }
    if !y.approx && !y.d.is_one() && y.d < big(64) {
        if let Some(r) = x.exact_root(y.d.to_u32().unwrap()) {
            return r.pow_int(&y.n);
        }
    }
    exp(&(y * &ln(x)))
}

pub fn sinh(x: &Real) -> Real {
    let e = exp(x);
    (&e - &(Real::one() / &e)) / Real::two()
}
pub fn cosh(x: &Real) -> Real {
    let e = exp(x);
    (&e + &(Real::one() / &e)) / Real::two()
}
pub fn tanh(x: &Real) -> Real {
    if x.to_f64() > 200.0 {
        return Real::one().as_approx();
    }
    if x.to_f64() < -200.0 {
        return Real::minus_one().as_approx();
    }
    let e2 = exp(&(x * &Real::two()));
    (&e2 - &Real::one()) / (&e2 + &Real::one())
}

pub fn asin(x: &Real) -> Real {
    if x.abs().gt(&Real::one()) {
        throw("asin() argument out of range");
    }
    if x.abs().eqv(&Real::one()) {
        return Real::int(x.sign() as i64) * pi() / Real::two();
    }
    atan(&(x / &sqrt(&(Real::one() - x * x))))
}

pub fn acos(x: &Real) -> Real {
    pi() / Real::two() - asin(x)
}
pub fn asinh(x: &Real) -> Real {
    if x.sign() < 0 {
        -asinh(&-x)
    } else {
        ln(&(x + &sqrt(&(x * x + Real::one()))))
    }
}
pub fn acosh(x: &Real) -> Real {
    ln(&(x + &sqrt(&(x * x - Real::one()))))
}
pub fn atanh(x: &Real) -> Real {
    ln(&((Real::one() + x) / (Real::one() - x))) / Real::two()
}