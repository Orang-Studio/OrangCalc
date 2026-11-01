use super::real::{big, bits};
use super::rmath as rm;
use super::{cur, throw, BigInt, Real};
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};
use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Clone, Debug)]
pub struct Cx {
    pub re: Real,
    pub im: Real,
}

impl Cx {
    pub fn new(re: Real, im: Real) -> Cx {
        Cx { re, im }
    }
    pub fn real(re: Real) -> Cx {
        Cx { re, im: Real::zero() }
    }
    pub fn zero() -> Cx {
        Cx::real(Real::zero())
    }
    pub fn one() -> Cx {
        Cx::real(Real::one())
    }
    pub fn i() -> Cx {
        Cx::new(Real::zero(), Real::one())
    }
    pub fn int(v: i64) -> Cx {
        Cx::real(Real::int(v))
    }

    pub fn is_real(&self) -> bool {
        self.im.is_zero()
    }
    pub fn is_zero(&self) -> bool {
        self.re.is_zero() && self.im.is_zero()
    }
    pub fn is_one(&self) -> bool {
        self.re.is_one() && self.im.is_zero()
    }
    pub fn approx(&self) -> bool {
        self.re.approx || self.im.approx
    }
    pub fn is_int(&self) -> bool {
        self.is_real() && self.re.is_int()
    }
    pub fn is_exact_int(&self) -> bool {
        self.is_real() && self.re.is_exact_int()
    }
    pub fn is_rational(&self) -> bool {
        self.is_real() && !self.re.approx && !self.re.is_infinite()
    }
    pub fn is_infinite(&self) -> bool {
        self.re.is_infinite() || self.im.is_infinite()
    }

    pub fn as_approx(&self) -> Cx {
        Cx::new(self.re.as_approx(), if self.im.is_zero() { self.im.clone() } else { self.im.as_approx() })
    }

    pub fn clean(self) -> Cx {
        let lim = 2f64.powi(-(bits() - 12));
        if !self.im.is_zero() && self.im.approx && !self.re.is_zero() {
            let r = self.re.to_f64().abs();
            let i = self.im.to_f64().abs();
            if i < r * lim {
                return Cx::real(self.re);
            }
        }
        if !self.re.is_zero() && self.re.approx && !self.im.is_zero() {
            let r = self.re.to_f64().abs();
            let i = self.im.to_f64().abs();
            if r < i * lim {
                return Cx::new(Real::zero(), self.im);
            }
        }
        self
    }

    pub fn eqv(&self, o: &Cx) -> bool {
        self.re.eqv(&o.re) && self.im.eqv(&o.im)
    }

    pub fn conj(&self) -> Cx {
        Cx::new(self.re.clone(), -&self.im)
    }

    pub fn abs(&self) -> Real {
        if self.is_real() {
            return self.re.abs();
        }
        if self.re.is_zero() {
            return self.im.abs();
        }
        rm::sqrt(&(&self.re * &self.re + &self.im * &self.im))
    }

    pub fn arg(&self) -> Real {
        if self.is_zero() {
            return Real::zero();
        }
        rm::atan2(&self.im, &self.re)
    }

    pub fn pow_int(&self, e: &BigInt) -> Cx {
        if self.is_real() {
            return Cx::real(self.re.pow_int(e));
        }
        if e.is_negative() {
            return Cx::one() / self.pow_int(&-e);
        }
        let mut result = Cx::one();
        let mut b = self.clone();
        let mut e = e.clone();
        while !e.is_zero() {
            if !e.is_even() {
                result = &result * &b;
            }
            e >>= 1;
            if !e.is_zero() {
                b = &b * &b;
            }
        }
        result
    }

    pub fn exp(z: &Cx) -> Cx {
        if z.is_real() {
            return Cx::real(rm::exp(&z.re));
        }
        let ea = rm::exp(&z.re);
        Cx::new(&ea * &rm::cos(&z.im), &ea * &rm::sin(&z.im)).clean()
    }

    pub fn ln(z: &Cx) -> Cx {
        if z.is_zero() {
            throw("Logarithm of zero");
        }
        if z.is_real() && z.re.sign() > 0 {
            return Cx::real(rm::ln(&z.re));
        }
        if z.is_real() {
            return Cx::new(rm::ln(&-&z.re), rm::pi());
        }
        Cx::new(rm::ln(&z.abs()), z.arg())
    }

    pub fn sqrt(z: &Cx) -> Cx {
        if z.is_real() {
            if z.re.sign() >= 0 {
                return Cx::real(rm::sqrt(&z.re));
            }
            return Cx::new(Real::zero(), rm::sqrt(&-&z.re));
        }
        let m = z.abs();
        let re = rm::sqrt(&((&m + &z.re) / Real::two()));
        let mut im = rm::sqrt(&((&m - &z.re) / Real::two()));
        if z.im.sign() < 0 {
            im = -im;
        }
        Cx::new(re, im)
    }

    pub fn pow(z: &Cx, w: &Cx) -> Cx {
        if w.is_exact_int() {
            return z.pow_int(&w.re.n);
        }
        if z.is_zero() {
            if w.re.sign() > 0 {
                return Cx::zero();
            }
            throw("Division by zero");
        }
        if z.is_real() && w.is_real() {
            if z.re.sign() > 0 {
                return Cx::real(rm::pow(&z.re, &w.re));
            }
            if !w.re.approx && !w.re.d.is_even() && w.re.d < big(1000) {
                if let Some(root) = (-&z.re).exact_root(w.re.d.to_u32().unwrap()) {
                    if cur().opts().real_roots {
                        return Cx::real((-root).pow_int(&w.re.n));
                    }
                }
            }
            if w.re.eqv(&Real::half()) {
                return Cx::sqrt(z);
            }
        }
        Cx::exp(&(w * &Cx::ln(z))).clean()
    }

    pub fn sin(z: &Cx) -> Cx {
        if z.is_real() {
            return Cx::real(rm::sin(&z.re));
        }
        Cx::new(rm::sin(&z.re) * rm::cosh(&z.im), rm::cos(&z.re) * rm::sinh(&z.im)).clean()
    }

    pub fn cos(z: &Cx) -> Cx {
        if z.is_real() {
            return Cx::real(rm::cos(&z.re));
        }
        Cx::new(rm::cos(&z.re) * rm::cosh(&z.im), -(rm::sin(&z.re) * rm::sinh(&z.im))).clean()
    }

    pub fn tan(z: &Cx) -> Cx {
        if z.is_real() {
            return Cx::real(rm::tan(&z.re));
        }
        Cx::sin(z) / Cx::cos(z)
    }

    pub fn sinh(z: &Cx) -> Cx {
        if z.is_real() {
            Cx::real(rm::sinh(&z.re))
        } else {
            (Cx::exp(z) - Cx::exp(&-z)) / Cx::int(2)
        }
    }
    pub fn cosh(z: &Cx) -> Cx {
        if z.is_real() {
            Cx::real(rm::cosh(&z.re))
        } else {
            (Cx::exp(z) + Cx::exp(&-z)) / Cx::int(2)
        }
    }
    pub fn tanh(z: &Cx) -> Cx {
        if z.is_real() {
            Cx::real(rm::tanh(&z.re))
        } else {
            Cx::sinh(z) / Cx::cosh(z)
        }
    }

    pub fn asin(z: &Cx) -> Cx {
        if z.is_real() && z.re.abs().le(&Real::one()) {
            return Cx::real(rm::asin(&z.re));
        }
        let iz = Cx::i() * z;
        (-Cx::i() * Cx::ln(&(iz + Cx::sqrt(&(Cx::one() - z * z))))).clean()
    }

    pub fn acos(z: &Cx) -> Cx {
        if z.is_real() && z.re.abs().le(&Real::one()) {
            return Cx::real(rm::acos(&z.re));
        }
        (Cx::real(rm::pi() / Real::two()) - Cx::asin(z)).clean()
    }

    pub fn atan(z: &Cx) -> Cx {
        if z.is_real() {
            return Cx::real(rm::atan(&z.re));
        }
        if z.re.is_zero() && z.im.abs().eqv(&Real::one()) {
            throw("atan() is undefined here");
        }
        let iz = Cx::i() * z;
        (Cx::i() / Cx::int(2) * (Cx::ln(&(Cx::one() - &iz)) - Cx::ln(&(Cx::one() + &iz)))).clean()
    }

    pub fn asinh(z: &Cx) -> Cx {
        if z.is_real() {
            return Cx::real(rm::asinh(&z.re));
        }
        Cx::ln(&(z + &Cx::sqrt(&(z * z + Cx::one())))).clean()
    }

    pub fn acosh(z: &Cx) -> Cx {
        if z.is_real() && z.re.ge(&Real::one()) {
            return Cx::real(rm::acosh(&z.re));
        }
        Cx::ln(&(z + &(Cx::sqrt(&(z + &Cx::one())) * Cx::sqrt(&(z - &Cx::one()))))).clean()
    }

    pub fn atanh(z: &Cx) -> Cx {
        if z.is_real() && z.re.abs().lt(&Real::one()) {
            return Cx::real(rm::atanh(&z.re));
        }
        if z.is_real() && z.re.abs().eqv(&Real::one()) {
            throw("atanh() is undefined here");
        }
        (Cx::ln(&((Cx::one() + z) / (Cx::one() - z))) / Cx::int(2)).clean()
    }

    pub fn to_c64(&self) -> (f64, f64) {
        (self.re.to_f64(), self.im.to_f64())
    }
    pub fn from_c64(re: f64, im: f64) -> Cx {
        Cx::new(Real::from_f64(re), if im == 0.0 { Real::zero() } else { Real::from_f64(im) })
    }
}

fn cadd(a: &Cx, b: &Cx) -> Cx {
    Cx::new(&a.re + &b.re, &a.im + &b.im)
}
fn csub(a: &Cx, b: &Cx) -> Cx {
    Cx::new(&a.re - &b.re, &a.im - &b.im)
}
fn cmul(a: &Cx, b: &Cx) -> Cx {
    if a.is_real() && b.is_real() {
        return Cx::real(&a.re * &b.re);
    }
    if b.is_real() {
        return Cx::new(&a.re * &b.re, &a.im * &b.re);
    }
    if a.is_real() {
        return Cx::new(&a.re * &b.re, &a.re * &b.im);
    }
    Cx::new(&a.re * &b.re - &a.im * &b.im, &a.re * &b.im + &a.im * &b.re).clean()
}
fn cdiv(a: &Cx, b: &Cx) -> Cx {
    if b.is_real() {
        return Cx::new(&a.re / &b.re, &a.im / &b.re);
    }
    let den = &b.re * &b.re + &b.im * &b.im;
    if den.is_zero() {
        throw("Division by zero");
    }
    Cx::new((&a.re * &b.re + &a.im * &b.im) / &den, (&a.im * &b.re - &a.re * &b.im) / &den).clean()
}

macro_rules! cbinop {
    ($tr:ident, $m:ident, $f:ident) => {
        impl $tr<&Cx> for &Cx {
            type Output = Cx;
            fn $m(self, o: &Cx) -> Cx {
                $f(self, o)
            }
        }
        impl $tr<Cx> for Cx {
            type Output = Cx;
            fn $m(self, o: Cx) -> Cx {
                $f(&self, &o)
            }
        }
        impl $tr<&Cx> for Cx {
            type Output = Cx;
            fn $m(self, o: &Cx) -> Cx {
                $f(&self, o)
            }
        }
        impl $tr<Cx> for &Cx {
            type Output = Cx;
            fn $m(self, o: Cx) -> Cx {
                $f(self, &o)
            }
        }
    };
}
cbinop!(Add, add, cadd);
cbinop!(Sub, sub, csub);
cbinop!(Mul, mul, cmul);
cbinop!(Div, div, cdiv);

impl Neg for &Cx {
    type Output = Cx;
    fn neg(self) -> Cx {
        Cx::new(-&self.re, -&self.im)
    }
}
impl Neg for Cx {
    type Output = Cx;
    fn neg(self) -> Cx {
        Cx::new(-&self.re, -&self.im)
    }
}

impl From<Real> for Cx {
    fn from(r: Real) -> Cx {
        Cx::real(r)
    }
}