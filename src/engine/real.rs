use super::{throw, BigInt};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering as AO};
pub static BITS: AtomicI32 = AtomicI32::new(96);
pub const MAX_EXACT_BITS: u64 = 200000;
pub fn bits() -> i32 {
    BITS.load(AO::Relaxed)
}

// a real number: exact rational N/D
#[derive(Clone, Debug)]
pub struct Real {
    pub n: BigInt,
    pub d: BigInt,
    pub approx: bool,
    pub inf: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    HalfAwayFromZero,
    HalfEven,
    HalfOdd,
    HalfTowardZero,
    HalfRandom,
    HalfUp,
    HalfDown,
    TowardZero,
    AwayFromZero,
    Up,
    Down,
}

impl RoundingMode {
    pub const ALL: [RoundingMode; 11] = [
        RoundingMode::HalfAwayFromZero,
        RoundingMode::HalfEven,
        RoundingMode::HalfOdd,
        RoundingMode::HalfTowardZero,
        RoundingMode::HalfRandom,
        RoundingMode::HalfUp,
        RoundingMode::HalfDown,
        RoundingMode::TowardZero,
        RoundingMode::AwayFromZero,
        RoundingMode::Up,
        RoundingMode::Down,
    ];
}

static RNG: AtomicU64 = AtomicU64::new(0);

// xorshift, seeded from the clock on first use
pub fn rng_next(n: u64) -> u64 {
    let mut x = RNG.load(AO::Relaxed);
    if x == 0 {
        x = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(88172645463325252) | 1;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    RNG.store(x, AO::Relaxed);
    if n == 0 {
        x
    } else {
        x % n
    }
}

pub fn rng_f64() -> f64 {
    (rng_next(0) >> 11) as f64 / (1u64 << 53) as f64
}

pub fn bit_len(v: &BigInt) -> i64 {
    v.bits() as i64
}

pub fn trailing_zeros(m: &BigInt) -> i64 {
    m.trailing_zeros().map(|t| t as i64).unwrap_or(0)
}

pub fn div_round(a: &BigInt, b: &BigInt) -> BigInt {
    let (q, r) = a.div_rem(b);
    if r.abs() * 2 >= b.abs() {
        let s = a.signum() * b.signum();
        q + s
    } else {
        q
    }
}

fn shl(v: &BigInt, s: i64) -> BigInt {
    if s >= 0 {
        v << (s as usize)
    } else {
        v >> ((-s) as usize)
    }
}

pub fn big(v: i64) -> BigInt {
    BigInt::from(v)
}

impl Real {
    pub fn raw(n: BigInt, d: BigInt, approx: bool) -> Real {
        Real { n, d, approx, inf: 0 }
    }
    pub fn zero() -> Real {
        Real::raw(BigInt::zero(), BigInt::one(), false)
    }
    pub fn one() -> Real {
        Real::int(1)
    }
    pub fn two() -> Real {
        Real::int(2)
    }
    pub fn minus_one() -> Real {
        Real::int(-1)
    }
    pub fn half() -> Real {
        Real::raw(big(1), big(2), false)
    }
    pub fn pos_inf() -> Real {
        Real { n: big(1), d: BigInt::zero(), approx: false, inf: 1 }
    }
    pub fn neg_inf() -> Real {
        Real { n: big(-1), d: BigInt::zero(), approx: false, inf: -1 }
    }

    pub fn make(mut n: BigInt, mut d: BigInt, approx: bool) -> Real {
        if d.is_zero() {
            throw("Division by zero");
        }
        if d.is_negative() {
            n = -n;
            d = -d;
        }
        if approx {
            return round_to(&n, &d, bits());
        }
        let g = n.gcd(&d);
        if !g.is_one() && !g.is_zero() {
            n /= &g;
            d /= &g;
        }
        if n.is_zero() {
            d = BigInt::one();
        }
        Real::raw(n, d, false)
    }

    pub fn exact(n: BigInt, d: BigInt) -> Real {
        Real::make(n, d, false)
    }
    pub fn int(n: i64) -> Real {
        Real::raw(big(n), BigInt::one(), false)
    }
    pub fn big_int(n: BigInt) -> Real {
        Real::raw(n, BigInt::one(), false)
    }
    pub fn frac(n: i64, d: i64) -> Real {
        Real::make(big(n), big(d), false)
    }

    pub fn as_approx(&self) -> Real {
        if self.approx || self.inf != 0 {
            self.clone()
        } else {
            round_to(&self.n, &self.d, bits())
        }
    }

    pub fn is_zero(&self) -> bool {
        self.inf == 0 && self.n.is_zero()
    }
    pub fn is_int(&self) -> bool {
        self.inf == 0 && self.d.is_one()
    }
    pub fn is_exact_int(&self) -> bool {
        !self.approx && self.is_int()
    }
    pub fn is_one(&self) -> bool {
        self.inf == 0 && self.n.is_one() && self.d.is_one()
    }
    pub fn is_infinite(&self) -> bool {
        self.inf != 0
    }
    pub fn sign(&self) -> i32 {
        if self.inf != 0 {
            self.inf as i32
        } else if self.n.is_negative() {
            -1
        } else if self.n.is_zero() {
            0
        } else {
            1
        }
    }
    pub fn is_negative(&self) -> bool {
        self.sign() < 0
    }

    pub fn from_f64(v: f64) -> Real {
        if v.is_nan() {
            throw("Undefined result");
        }
        if v == f64::INFINITY {
            return Real::pos_inf();
        }
        if v == f64::NEG_INFINITY {
            return Real::neg_inf();
        }
        if v == 0.0 {
            return Real::raw(BigInt::zero(), BigInt::one(), true);
        }
        let b = v.to_bits() as i64;
        let neg = b < 0;
        let mut exp = ((b >> 52) & 0x7FF) as i64;
        let mut man = b & 0xFFFFFFFFFFFFF;
        if exp == 0 {
            exp += 1;
        } else {
            man |= 1 << 52;
        }
        exp -= 1075;
        let mut n = big(man);
        if neg {
            n = -n;
        }
        if exp >= 0 {
            Real::make(n << (exp as usize), BigInt::one(), true)
        } else {
            Real::make(n, BigInt::one() << ((-exp) as usize), true)
        }
    }

    pub fn to_f64(&self) -> f64 {
        if self.inf != 0 {
            return if self.inf > 0 { f64::INFINITY } else { f64::NEG_INFINITY };
        }
        if self.n.is_zero() {
            return 0.0;
        }
        let s = 62 - (bit_len(&self.n) - bit_len(&self.d));
        let q = if s >= 0 { (&self.n << (s as usize)) / &self.d } else { &self.n / (&self.d << ((-s) as usize)) };
        let mut r = q.to_f64().unwrap_or(f64::NAN);
        let mut ex = -s;
        while ex > 1000 {
            r *= 2f64.powi(1000);
            ex -= 1000;
        }
        while ex < -1000 {
            r *= 2f64.powi(-1000);
            ex += 1000;
        }
        r * 2f64.powi(ex as i32)
    }

    pub fn to_long(&self) -> i64 {
        match self.trunc().n.to_i64() {
            Some(v) => v,
            None => throw("Integer too large"),
        }
    }

    pub fn to_int(&self) -> i32 {
        match self.trunc().n.to_i32() {
            Some(v) => v,
            None => throw("Integer too large"),
        }
    }

    pub fn to_big(&self) -> BigInt {
        if self.inf != 0 {
            throw("Infinite value");
        }
        self.trunc().n
    }

    fn check_size(r: &Real) {
        if !r.approx && (bit_len(&r.n) as u64 > MAX_EXACT_BITS || bit_len(&r.d) as u64 > MAX_EXACT_BITS) {
            throw("Number too large");
        }
    }

    pub fn add(&self, b: &Real) -> Real {
        let a = self;
        if a.inf != 0 || b.inf != 0 {
            if a.inf != 0 && b.inf != 0 && a.inf != b.inf {
                throw("Undefined result (∞ − ∞)");
            }
            return if a.inf != 0 { a.clone() } else { b.clone() };
        }
        let ap = a.approx || b.approx;
        if a.d == b.d && !ap {
            return Real::make(&a.n + &b.n, a.d.clone(), false);
        }
        let r = Real::make(&a.n * &b.d + &b.n * &a.d, &a.d * &b.d, ap);
        Real::check_size(&r);
        r
    }

    pub fn neg(&self) -> Real {
        if self.inf != 0 {
            if self.inf > 0 {
                Real::neg_inf()
            } else {
                Real::pos_inf()
            }
        } else {
            Real::raw(-&self.n, self.d.clone(), self.approx)
        }
    }

    pub fn sub(&self, b: &Real) -> Real {
        self.add(&b.neg())
    }

    pub fn mul(&self, b: &Real) -> Real {
        let a = self;
        if a.inf != 0 || b.inf != 0 {
            if a.is_zero() || b.is_zero() {
                throw("Undefined result (0 × ∞)");
            }
            return if a.sign() * b.sign() > 0 { Real::pos_inf() } else { Real::neg_inf() };
        }
        let r = Real::make(&a.n * &b.n, &a.d * &b.d, a.approx || b.approx);
        Real::check_size(&r);
        r
    }

    pub fn div(&self, b: &Real) -> Real {
        let a = self;
        if b.is_zero() {
            throw("Division by zero");
        }
        if b.inf != 0 {
            if a.inf != 0 {
                throw("Undefined result (∞ / ∞)");
            }
            return Real::raw(BigInt::zero(), BigInt::one(), a.approx);
        }
        if a.inf != 0 {
            return if a.sign() * b.sign() > 0 { Real::pos_inf() } else { Real::neg_inf() };
        }
        let r = Real::make(&a.n * &b.d, &a.d * &b.n, a.approx || b.approx);
        Real::check_size(&r);
        r
    }

    pub fn cmp(&self, o: &Real) -> Ordering {
        if self.inf != 0 && o.inf != 0 {
            return self.inf.cmp(&o.inf);
        }
        if self.inf != 0 {
            return self.inf.cmp(&0);
        }
        if o.inf != 0 {
            return 0.cmp(&o.inf);
        }
        (&self.n * &o.d).cmp(&(&o.n * &self.d))
    }

    pub fn lt(&self, o: &Real) -> bool {
        self.cmp(o) == Ordering::Less
    }
    pub fn gt(&self, o: &Real) -> bool {
        self.cmp(o) == Ordering::Greater
    }
    pub fn le(&self, o: &Real) -> bool {
        self.cmp(o) != Ordering::Greater
    }
    pub fn ge(&self, o: &Real) -> bool {
        self.cmp(o) != Ordering::Less
    }
    pub fn eqv(&self, o: &Real) -> bool {
        self.cmp(o) == Ordering::Equal
    }

    pub fn abs(&self) -> Real {
        if self.sign() < 0 {
            self.neg()
        } else {
            self.clone()
        }
    }

    pub fn floor(&self) -> Real {
        if self.inf != 0 || self.d.is_one() {
            return self.clone();
        }
        let mut q = &self.n / &self.d;
        if self.n.is_negative() {
            q -= 1;
        }
        Real::raw(q, BigInt::one(), self.approx)
    }

    pub fn ceil(&self) -> Real {
        self.neg().floor().neg()
    }
    pub fn trunc(&self) -> Real {
        if self.sign() < 0 {
            self.ceil()
        } else {
            self.floor()
        }
    }

    pub fn round(&self, mode: RoundingMode) -> Real {
        use RoundingMode::*;
        if self.inf != 0 || self.d.is_one() {
            return self.clone();
        }
        let fl = self.floor();
        let fr = self - &fl;
        let c = fr.cmp(&Real::half());
        match mode {
            Up => return self.ceil(),
            Down => return self.floor(),
            TowardZero => return self.trunc(),
            AwayFromZero => return if self.sign() < 0 { self.floor() } else { self.ceil() },
            _ => {}
        }
        if c == Ordering::Less {
            return fl;
        }
        let up = &fl + &Real::one();
        if c == Ordering::Greater {
            return up;
        }
        match mode {
            HalfEven => {
                if fl.n.is_even() {
                    fl
                } else {
                    up
                }
            }
            HalfOdd => {
                if !fl.n.is_even() {
                    fl
                } else {
                    up
                }
            }
            HalfUp => up,
            HalfDown => fl,
            HalfTowardZero => {
                if self.sign() < 0 {
                    up
                } else {
                    fl
                }
            }
            HalfRandom => {
                if rng_next(2) == 0 {
                    fl
                } else {
                    up
                }
            }
            _ => {
                if self.sign() < 0 {
                    fl
                } else {
                    up
                }
            }
        }
    }

    pub fn round_default(&self) -> Real {
        self.round(RoundingMode::HalfAwayFromZero)
    }

    pub fn frac_part(&self) -> Real {
        self - &self.trunc()
    }

    pub fn pow_int(&self, e: &BigInt) -> Real {
        if e.is_zero() {
            if self.is_zero() {
                return Real::one();
            }
            return if self.approx { Real::one().as_approx() } else { Real::one() };
        }
        if self.inf != 0 {
            if e.is_negative() {
                return Real::zero();
            }
            return if self.inf < 0 && !e.is_even() { Real::neg_inf() } else { Real::pos_inf() };
        }
        if e.is_negative() {
            if self.is_zero() {
                throw("Division by zero");
            }
            return Real::one().div(&self.pow_int(&-e));
        }
        if self.is_zero() || self.is_one() {
            return self.clone();
        }
        if self.n == big(-1) && self.d.is_one() {
            return if e.is_even() { Real::one() } else { Real::minus_one() };
        }
        if !self.approx {
            let est = e.to_f64().unwrap_or(f64::INFINITY) * bit_len(&self.n).max(bit_len(&self.d)) as f64;
            if est > MAX_EXACT_BITS as f64 {
                return self.as_approx().pow_int(e);
            }
            let ei = match e.to_u32() {
                Some(v) => v,
                None => throw("Exponent too large"),
            };
            return Real::make(num_traits::pow(self.n.clone(), ei as usize), num_traits::pow(self.d.clone(), ei as usize), false);
        }
        let mut result = Real::one().as_approx();
        let mut b = self.clone();
        let mut ex = e.clone();
        while !ex.is_zero() {
            if !ex.is_even() {
                result = &result * &b;
            }
            ex >>= 1;
            if !ex.is_zero() {
                b = &b * &b;
            }
        }
        result
    }

    pub fn pow_i(&self, e: i64) -> Real {
        self.pow_int(&big(e))
    }

    pub fn min(a: &Real, b: &Real) -> Real {
        if a.le(b) {
            a.clone()
        } else {
            b.clone()
        }
    }
    pub fn max(a: &Real, b: &Real) -> Real {
        if a.ge(b) {
            a.clone()
        } else {
            b.clone()
        }
    }

    pub fn parse(digits: &str, radix: u32) -> Real {
        let mut n = BigInt::zero();
        let mut d = BigInt::one();
        let mut frac = false;
        for ch in digits.chars() {
            if ch == '.' {
                frac = true;
                continue;
            }
            let v = digit_value(ch);
            if v < 0 || v as u32 >= radix {
                throw(&format!("Invalid digit '{}' for base {}", ch, radix));
            }
            n = n * radix + v as u32;
            if frac {
                d *= radix;
            }
        }
        Real::make(n, d, false)
    }

    // exact nth root if the rational is a perfect power, otherwise None
    pub fn exact_root(&self, k: u32) -> Option<Real> {
        if self.approx || self.inf != 0 {
            return None;
        }
        if self.n.is_negative() && k % 2 == 0 {
            return None;
        }
        let rn = iroot(&self.n, k);
        if num_traits::pow(rn.clone(), k as usize) != self.n {
            return None;
        }
        let rd = iroot(&self.d, k);
        if num_traits::pow(rd.clone(), k as usize) != self.d {
            return None;
        }
        Some(Real::make(rn, rd, false))
    }
}

fn round_to(n: &BigInt, d: &BigInt, bits: i32) -> Real {
    if n.is_zero() {
        return Real::raw(BigInt::zero(), BigInt::one(), true);
    }
    let mut sh = bits as i64 - (bit_len(n) - bit_len(d));
    let mut m = if sh >= 0 { div_round(&(n << (sh as usize)), d) } else { div_round(n, &(d << ((-sh) as usize))) };
    if sh > 0 {
        let tz = trailing_zeros(&m);
        let cut = tz.min(sh);
        m >>= cut as usize;
        sh -= cut;
        return Real::raw(m, BigInt::one() << (sh as usize), true);
    }
    Real::raw(shl(&m, -sh), BigInt::one(), true)
}

pub fn digit_value(c: char) -> i32 {
    match c {
        '0'..='9' => c as i32 - '0' as i32,
        'a'..='z' => c as i32 - 'a' as i32 + 10,
        'A'..='Z' => c as i32 - 'A' as i32 + 10,
        '↊' => 10,
        '↋' => 11,
        _ => -1,
    }
}

pub fn isqrt(n: &BigInt) -> BigInt {
    if n.is_negative() {
        throw("Negative square root");
    }
    if *n < big(2) {
        return n.clone();
    }
    n.sqrt()
}

pub fn iroot(n: &BigInt, k: u32) -> BigInt {
    if n.is_negative() {
        return -iroot(&-n, k);
    }
    if *n < big(2) || k == 1 {
        return n.clone();
    }
    n.nth_root(k)
}

macro_rules! binop {
    ($tr:ident, $m:ident, $f:ident) => {
        impl $tr<&Real> for &Real {
            type Output = Real;
            fn $m(self, o: &Real) -> Real {
                Real::$f(self, o)
            }
        }
        impl $tr<Real> for Real {
            type Output = Real;
            fn $m(self, o: Real) -> Real {
                Real::$f(&self, &o)
            }
        }
        impl $tr<&Real> for Real {
            type Output = Real;
            fn $m(self, o: &Real) -> Real {
                Real::$f(&self, o)
            }
        }
        impl $tr<Real> for &Real {
            type Output = Real;
            fn $m(self, o: Real) -> Real {
                Real::$f(self, &o)
            }
        }
    };
}
binop!(Add, add, add);
binop!(Sub, sub, sub);
binop!(Mul, mul, mul);
binop!(Div, div, div);

impl Neg for &Real {
    type Output = Real;
    fn neg(self) -> Real {
        Real::neg(self)
    }
}
impl Neg for Real {
    type Output = Real;
    fn neg(self) -> Real {
        Real::neg(&self)
    }
}

impl From<i64> for Real {
    fn from(v: i64) -> Real {
        Real::int(v)
    }
}