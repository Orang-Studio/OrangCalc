// plain complex doubles for numeric algorithms (eigenvalues, special functions, plotting)
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct C64 {
    pub re: f64,
    pub im: f64,
}

pub const fn c64(re: f64, im: f64) -> C64 {
    C64 { re, im }
}

impl C64 {
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    pub fn conj(self) -> C64 {
        c64(self.re, -self.im)
    }
    pub fn exp(self) -> C64 {
        let e = self.re.exp();
        c64(e * self.im.cos(), e * self.im.sin())
    }
    pub fn ln(self) -> C64 {
        c64(self.abs().ln(), self.arg())
    }
    pub fn sqrt(self) -> C64 {
        let m = self.abs();
        let re = ((m + self.re) / 2.0).sqrt();
        let im = ((m - self.re) / 2.0).sqrt();
        c64(re, if self.im < 0.0 { -im } else { im })
    }
    pub fn powc(self, w: C64) -> C64 {
        if self.re == 0.0 && self.im == 0.0 {
            return if w.re == 0.0 && w.im == 0.0 { c64(1.0, 0.0) } else { c64(0.0, 0.0) };
        }
        (w * self.ln()).exp()
    }
    pub fn sin(self) -> C64 {
        c64(self.re.sin() * self.im.cosh(), self.re.cos() * self.im.sinh())
    }
}

impl Add for C64 {
    type Output = C64;
    fn add(self, o: C64) -> C64 {
        c64(self.re + o.re, self.im + o.im)
    }
}
impl Sub for C64 {
    type Output = C64;
    fn sub(self, o: C64) -> C64 {
        c64(self.re - o.re, self.im - o.im)
    }
}
impl Mul for C64 {
    type Output = C64;
    fn mul(self, o: C64) -> C64 {
        c64(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}
impl Div for C64 {
    type Output = C64;
    fn div(self, o: C64) -> C64 {
        let d = o.re * o.re + o.im * o.im;
        c64((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
    }
}
impl Neg for C64 {
    type Output = C64;
    fn neg(self) -> C64 {
        c64(-self.re, -self.im)
    }
}
impl Mul<f64> for C64 {
    type Output = C64;
    fn mul(self, o: f64) -> C64 {
        c64(self.re * o, self.im * o)
    }
}
impl Div<f64> for C64 {
    type Output = C64;
    fn div(self, o: f64) -> C64 {
        c64(self.re / o, self.im / o)
    }
}
impl Add<f64> for C64 {
    type Output = C64;
    fn add(self, o: f64) -> C64 {
        c64(self.re + o, self.im)
    }
}
impl Sub<f64> for C64 {
    type Output = C64;
    fn sub(self, o: f64) -> C64 {
        c64(self.re - o, self.im)
    }
}
impl AddAssign for C64 {
    fn add_assign(&mut self, o: C64) {
        *self = *self + o;
    }
}
impl SubAssign for C64 {
    fn sub_assign(&mut self, o: C64) {
        *self = *self - o;
    }
}
impl From<f64> for C64 {
    fn from(v: f64) -> C64 {
        c64(v, 0.0)
    }
}