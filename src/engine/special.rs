use super::c64::{c64, C64};
use super::throw;
use std::f64::consts::{E, PI};
const LANCZOS: [f64; 9] = [0.99999999999980993, 676.5203681218851, -1259.1392167224028, 771.32342877765313, -176.61502916214059, 12.507343278686905, -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7];
const EULER: f64 = 0.57721566490153286;

fn cr(x: f64) -> C64 {
    c64(x, 0.0)
}

pub fn gamma_c(z: C64) -> C64 {
    if z.re < 0.5 {
        return cr(PI) / ((z * PI).sin() * gamma_c(cr(1.0) - z));
    }
    let z = z - 1.0;
    let mut x = cr(LANCZOS[0]);
    for (i, l) in LANCZOS.iter().enumerate().skip(1) {
        x += cr(*l) / (z + i as f64);
    }
    let t = z + 7.5;
    t.powc(z + 0.5) * (-t).exp() * x * (2.0 * PI).sqrt()
}

pub fn gamma(x: f64) -> f64 {
    if x == x.floor() && x <= 0.0 {
        throw("gamma() is undefined for non-positive integers");
    }
    if x > 171.6 {
        return f64::INFINITY;
    }
    gamma_c(cr(x)).re
}

pub fn log_gamma(x: f64) -> f64 {
    if x <= 0.0 && x == x.floor() {
        throw("lgamma() is undefined for non-positive integers");
    }
    if x < 0.5 {
        return (PI / (PI * x).sin().abs()).ln() - log_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = LANCZOS[0];
    let t = x + 7.5;
    for (i, l) in LANCZOS.iter().enumerate().skip(1) {
        a += l / (x + i as f64);
    }
    0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

pub fn digamma(x: f64) -> f64 {
    if x <= 0.0 && x == x.floor() {
        throw("digamma() is undefined for non-positive integers");
    }
    if x < 0.0 {
        return digamma(1.0 - x) - PI / (PI * x).tan();
    }
    let mut r = 0.0;
    let mut x = x;
    while x < 6.0 {
        r -= 1.0 / x;
        x += 1.0;
    }
    let f = 1.0 / (x * x);
    r + x.ln() - 0.5 / x - f * (1.0 / 12.0 - f * (1.0 / 120.0 - f * (1.0 / 252.0 - f * (1.0 / 240.0 - f * (1.0 / 132.0)))))
}

pub fn erf(x: f64) -> f64 {
    if x < 0.0 {
        return -erf(-x);
    }
    if x > 6.0 {
        return 1.0;
    }
    if x < 2.5 {
        let (mut sum, mut term, x2) = (x, x, x * x);
        for n in 1..200 {
            term *= -x2 / n as f64;
            let add = term / (2 * n + 1) as f64;
            sum += add;
            if add.abs() < 1e-17 * sum.abs() {
                break;
            }
        }
        return 2.0 / PI.sqrt() * sum;
    }
    1.0 - erfc(x)
}

pub fn erfc(x: f64) -> f64 {
    if x < 2.5 {
        return 1.0 - erf(x);
    }
    let mut t = 0.0;
    for n in (1..=60).rev() {
        t = n as f64 / 2.0 / (x + t);
    }
    (-x * x).exp() / PI.sqrt() / (x + t)
}

pub fn erf_inv(y: f64) -> f64 {
    if y <= -1.0 || y >= 1.0 {
        throw("erfinv() argument must be in (-1, 1)");
    }
    let mut w = -((1.0 - y) * (1.0 + y)).ln();
    let mut x;
    if w < 5.0 {
        w -= 2.5;
        x = 2.81022636e-08;
        for c in [3.43273939e-07, -3.5233877e-06, -4.39150654e-06, 0.00021858087, -0.00125372503, -0.00417768164, 0.246640727, 1.50140941] {
            x = c + x * w;
        }
    } else {
        w = w.sqrt() - 3.0;
        x = -0.000200214257;
        for c in [0.000100950558, 0.00134934322, -0.00367342844, 0.00573950773, -0.0076224613, 0.00943887047, 1.00167406, 2.83297682] {
            x = c + x * w;
        }
    }
    x *= y;
    for _ in 0..3 {
        let err = erf(x) - y;
        x -= err / (2.0 / PI.sqrt() * (-x * x).exp());
    }
    x
}

fn fact(n: i32) -> f64 {
    (2..=n).fold(1.0, |r, i| r * i as f64)
}

pub fn zeta(s: C64) -> C64 {
    if s == cr(1.0) {
        throw("zeta(1) is undefined");
    }
    if s.re < 0.5 {
        return cr(2.0).powc(s) * cr(PI).powc(s - 1.0) * (s * PI / 2.0).sin() * gamma_c(cr(1.0) - s) * zeta(cr(1.0) - s);
    }
    let n = 40;
    let mut d = vec![0.0; n as usize + 1];
    for (i, di) in d.iter_mut().enumerate() {
        let mut num = 0.0;
        for j in 0..=i as i32 {
            num += fact(n + j - 1) * 4f64.powi(j) / (fact(n - j) * fact(2 * j));
        }
        *di = n as f64 * num;
    }
    let mut total = C64::default();
    for k in 0..n as usize {
        let sign = if k % 2 == 0 { 1.0 } else { -1.0 };
        total += cr(sign * (d[k] - d[n as usize])) / cr(k as f64 + 1.0).powc(s);
    }
    -total / ((cr(1.0) - cr(2.0).powc(cr(1.0) - s)) * d[n as usize])
}

pub fn polylog(s: C64, z: C64) -> C64 {
    if z.abs() <= 0.75 {
        let mut sum = C64::default();
        let mut zk = z;
        for k in 1..2000 {
            let add = zk / cr(k as f64).powc(s);
            sum += add;
            if add.abs() < 1e-17 * sum.abs() {
                break;
            }
            zk = zk * z;
        }
        return sum;
    }
    if s == cr(2.0) && z.im == 0.0 && z.re <= 1.0 {
        let x = z.re;
        let p2 = PI * PI / 6.0;
        if x == 1.0 {
            return cr(p2);
        }
        if x > 0.5 {
            return cr(p2 - x.ln() * (1.0 - x).ln()) - polylog(s, cr(1.0 - x));
        }
        if x < -1.0 {
            return cr(-p2 - 0.5 * (-x).ln().powi(2)) - polylog(s, cr(1.0 / x));
        }
        if x < -0.5 {
            return -polylog(s, cr(x / (x - 1.0))) - cr(0.5 * (1.0 - x).ln().powi(2));
        }
    }
    if s == cr(1.0) {
        return -(cr(1.0) - z).ln();
    }
    if z.im == 0.0 && z.re == 1.0 {
        return zeta(s);
    }
    let mu = z.ln();
    let mut res = gamma_c(cr(1.0) - s) * (-mu).powc(s - 1.0);
    let mut muk = cr(1.0);
    for k in 0..60 {
        if s - k as f64 != cr(1.0) {
            res += zeta(s - k as f64) * muk / fact(k);
        }
        muk = muk * mu;
    }
    res
}

pub fn ei(x: f64) -> f64 {
    if x == 0.0 {
        throw("Ei(0) is undefined");
    }
    if x < 0.0 {
        return -e1(-x);
    }
    if x < 40.0 {
        let (mut sum, mut term) = (0.0, 1.0);
        for k in 1..500 {
            term *= x / k as f64;
            let add = term / k as f64;
            sum += add;
            if add < 1e-17 * sum {
                break;
            }
        }
        return EULER + x.ln() + sum;
    }
    let (mut s, mut t) = (1.0, 1.0);
    for k in 1..40 {
        t *= k as f64 / x;
        s += t;
        if t < 1e-17 {
            break;
        }
    }
    x.exp() / x * s
}

pub fn e1(x: f64) -> f64 {
    if x <= 0.0 {
        throw("E1() requires a positive argument");
    }
    if x < 1.0 {
        let (mut sum, mut term) = (0.0, 1.0);
        for k in 1..100 {
            term *= -x / k as f64;
            sum -= term / k as f64;
        }
        return -EULER - x.ln() + sum;
    }
    let mut b = x + 1.0;
    let mut c = 1e300;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..300 {
        let an = -(i as f64) * i as f64;
        b += 2.0;
        d = 1.0 / (an * d + b);
        c = b + an / c;
        let del = c * d;
        h *= del;
        if (del - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h * (-x).exp()
}

pub fn li(x: f64) -> f64 {
    if x == 1.0 {
        return f64::NEG_INFINITY;
    }
    if x < 0.0 {
        throw("li() requires a non-negative argument");
    }
    if x == 0.0 {
        return 0.0;
    }
    ei(x.ln())
}

pub fn si_ci(x: f64) -> (f64, f64) {
    let ax = x.abs();
    let (mut si, ci);
    if ax < 4.0 {
        let (mut s, mut c, mut term) = (0.0, 0.0, ax);
        let x2 = ax * ax;
        for k in 0..60 {
            s += term / (2 * k + 1) as f64;
            term *= -x2 / ((2 * k + 2) * (2 * k + 3)) as f64;
        }
        term = -x2 / 2.0;
        for k in 1..60 {
            c += term / (2 * k) as f64;
            term *= -x2 / ((2 * k + 1) * (2 * k + 2)) as f64;
        }
        si = s;
        ci = if ax == 0.0 { f64::NEG_INFINITY } else { EULER + ax.ln() + c };
    } else {
        let mut b = c64(1.0, ax);
        let mut cc = cr(1e300);
        let mut d = cr(1.0) / b;
        let mut h = d;
        for i in 2..400 {
            let a = -((i - 1) as f64) * (i - 1) as f64;
            b = b + 2.0;
            d = cr(1.0) / (d * a + b);
            cc = b + cr(a) / cc;
            let del = cc * d;
            h = h * del;
            if (del - 1.0).abs() < 1e-16 {
                break;
            }
        }
        h = h * c64(ax.cos(), -ax.sin());
        ci = -h.re;
        si = PI / 2.0 + h.im;
    }
    if x < 0.0 {
        si = -si;
    }
    (si, ci)
}

pub fn shi(x: f64) -> f64 {
    if x.abs() > 30.0 {
        return 0.5 * (ei(x) - ei(-x));
    }
    let (mut sum, mut term, x2) = (0.0, x, x * x);
    for k in 0..200 {
        let add = term / (2 * k + 1) as f64;
        sum += add;
        if add.abs() < 1e-17 * sum.abs() {
            break;
        }
        term *= x2 / ((2 * k + 2) * (2 * k + 3)) as f64;
    }
    sum
}

pub fn chi(x: f64) -> f64 {
    if x <= 0.0 {
        throw("Chi() requires a positive argument");
    }
    0.5 * (ei(x) + ei(-x))
}

pub fn bessel_j(n: i32, x: f64) -> f64 {
    if n < 0 {
        return if n % 2 == 0 { 1.0 } else { -1.0 } * bessel_j(-n, x);
    }
    if x.abs() < 25.0 {
        let mut sum = 0.0;
        let mut term = (x / 2.0).powi(n) / fact(n);
        for k in 0..300 {
            sum += term;
            term *= -(x * x / 4.0) / ((k + 1) as f64 * (k + n + 1) as f64);
            if term.abs() < 1e-18 * sum.abs() && k > 5 {
                break;
            }
        }
        return sum;
    }
    let steps = 4000;
    let mut integral = 0.0;
    for i in 0..=steps {
        let t = PI * i as f64 / steps as f64;
        let w = if i == 0 || i == steps {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        integral += w * (n as f64 * t - x * t.sin()).cos();
    }
    integral * (PI / steps as f64 / 3.0) / PI
}

pub fn bessel_y(n: i32, x: f64) -> f64 {
    if x <= 0.0 {
        throw("bessely() requires a positive argument");
    }
    let nu = n as f64 + 1e-7;
    let nu2 = n as f64 - 1e-7;
    let y1 = (bessel_j_nonint(nu, x) * (nu * PI).cos() - bessel_j_nonint(-nu, x)) / (nu * PI).sin();
    let y2 = (bessel_j_nonint(nu2, x) * (nu2 * PI).cos() - bessel_j_nonint(-nu2, x)) / (nu2 * PI).sin();
    (y1 + y2) / 2.0
}

fn bessel_j_nonint(nu: f64, x: f64) -> f64 {
    let mut sum = 0.0;
    for k in 0..200 {
        let (lg, sg) = log_gamma_signed(k as f64 + nu + 1.0);
        let term = if k % 2 == 0 { 1.0 } else { -1.0 } * ((2.0 * k as f64 + nu) * (x / 2.0).ln() - log_gamma(k as f64 + 1.0) - lg).exp() * sg;
        sum += term;
        if term.abs() < 1e-18 * sum.abs() && k > 10 {
            break;
        }
    }
    sum
}

fn log_gamma_signed(x: f64) -> (f64, f64) {
    if x > 0.0 {
        return (log_gamma(x), 1.0);
    }
    let g = gamma(x);
    (g.abs().ln(), if g < 0.0 { -1.0 } else { 1.0 })
}

pub fn airy_ai(x: f64) -> f64 {
    if x > 8.0 {
        return (-2.0 / 3.0 * x.powf(1.5)).exp() / (2.0 * PI.sqrt() * x.powf(0.25));
    }
    let (c1, c2) = (0.355028053887817239, 0.258819403792806798);
    let (mut f, mut g, mut sf, mut sg) = (1.0, x, 1.0, x);
    let x3 = x * x * x;
    for k in 1..200 {
        let k = k as f64;
        f *= x3 / ((3.0 * k - 1.0) * (3.0 * k));
        g *= x3 / ((3.0 * k) * (3.0 * k + 1.0));
        sf += f;
        sg += g;
        if f.abs() + g.abs() < 1e-18 {
            break;
        }
    }
    c1 * sf - c2 * sg
}

pub fn lambert_w(z: C64, k: i32) -> C64 {
    let mut w = if k == 0 {
        if z.abs() < 1.0 {
            z
        } else {
            (z + 1.0).ln()
        }
    } else {
        z.ln() + c64(0.0, 2.0 * PI * k as f64)
    };
    if k == 0 && z.im == 0.0 && z.re < -0.3 && z.re >= -1.0 / E {
        w = cr(-1.0) + ((z * E + 1.0) * 2.0).sqrt();
    }
    if k == -1 && z.im == 0.0 && z.re < 0.0 && z.re >= -1.0 / E {
        w = if z.re > -0.25 { cr((-z.re).ln() - (-(-z.re).ln()).ln()) } else { cr(-1.0) - ((z * E + 1.0) * 2.0).sqrt() };
    }
    for _ in 0..100 {
        let ew = w.exp();
        let f = w * ew - z;
        let wp1 = w + 1.0;
        if wp1.abs() < 1e-300 {
            break;
        }
        let dw = f / (ew * wp1 - (w + 2.0) * f / (wp1 * 2.0));
        w -= dw;
        if dw.abs() < 1e-16 * (1.0 + w.abs()) {
            break;
        }
    }
    w
}

// regularized lower incomplete gamma
pub fn gamma_p(a: f64, x: f64) -> f64 {
    if x < 0.0 || a <= 0.0 {
        throw("Invalid arguments for incomplete gamma");
    }
    if x == 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        let (mut ap, mut sum) = (a, 1.0 / a);
        let mut del = sum;
        for _ in 0..1000 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-16 {
                break;
            }
        }
        return sum * (-x + a * x.ln() - log_gamma(a)).exp();
    }
    1.0 - gamma_q_cf(a, x)
}

fn gamma_q_cf(a: f64, x: f64) -> f64 {
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / 1e-300;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..1000 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = b + an / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-16 {
            break;
        }
    }
    (-x + a * x.ln() - log_gamma(a)).exp() * h
}

pub fn upper_inc_gamma(a: f64, x: f64) -> f64 {
    (1.0 - gamma_p(a, x)) * gamma(a)
}
pub fn lower_inc_gamma(a: f64, x: f64) -> f64 {
    gamma_p(a, x) * gamma(a)
}

pub fn beta(a: f64, b: f64) -> f64 {
    (log_gamma(a) + log_gamma(b) - log_gamma(a + b)).exp() * gamma(a).signum() * gamma(b).signum() * gamma(a + b).signum()
}

// regularized incomplete beta
pub fn beta_inc(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (log_gamma(a + b) - log_gamma(a) - log_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        return bt * beta_cf(x, a, b) / a;
    }
    1.0 - bt * beta_cf(1.0 - x, b, a) / b
}

fn beta_cf(x: f64, a: f64, b: f64) -> f64 {
    let (qab, qap, qam) = (a + b, a + 1.0, a - 1.0);
    let tiny = |v: f64| if v.abs() < 1e-300 { 1e-300 } else { v };
    let mut c = 1.0;
    let mut d = 1.0 / tiny(1.0 - qab * x / qap);
    let mut h = d;
    for m in 1..1000 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 / tiny(1.0 + aa * d);
        c = tiny(1.0 + aa / c);
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 / tiny(1.0 + aa * d);
        c = tiny(1.0 + aa / c);
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h
}

pub fn fresnel(x: f64) -> (f64, f64) {
    let (mut ss, mut cc) = (0.0, 0.0);
    let n = 2000;
    for i in 0..=n {
        let t = x * i as f64 / n as f64;
        let w = if i == 0 || i == n {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        ss += w * (PI * t * t / 2.0).sin();
        cc += w * (PI * t * t / 2.0).cos();
    }
    (ss * x / n as f64 / 3.0, cc * x / n as f64 / 3.0)
}

pub fn normal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    0.5 * (1.0 + erf((x - mu) / (sigma * 2f64.sqrt())))
}
pub fn normal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    (-0.5 * ((x - mu) / sigma).powi(2)).exp() / (sigma * (2.0 * PI).sqrt())
}

pub fn normal_inv(p: f64, mu: f64, sigma: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        throw("Probability must be in (0, 1)");
    }
    mu + sigma * 2f64.sqrt() * erf_inv(2.0 * p - 1.0)
}

pub fn student_t_cdf(t: f64, v: f64) -> f64 {
    let x = v / (v + t * t);
    let ib = beta_inc(x, v / 2.0, 0.5);
    if t >= 0.0 {
        1.0 - 0.5 * ib
    } else {
        0.5 * ib
    }
}

pub fn chi_square_cdf(x: f64, k: f64) -> f64 {
    if x <= 0.0 {
        0.0
    } else {
        gamma_p(k / 2.0, x / 2.0)
    }
}