use super::rmath as rm;
use super::*;
use num_integer::Integer;

fn mode() -> IntervalCalc {
    calculator::try_cur().map(|c| c.opts().interval).unwrap_or(IntervalCalc::VarianceFormula)
}

pub fn get(e: &Expr) -> Option<(Real, Real)> {
    match &e.e {
        E::Intv(lo, hi) => Some((lo.clone(), hi.clone())),
        E::Num(n) if n.is_real() => Some((n.re.clone(), n.re.clone())),
        _ => None,
    }
}

fn mid_rad(e: &Expr) -> Option<(Real, Real)> {
    match &e.e {
        E::Intv(lo, hi) => Some(((lo + hi) / Real::two(), (hi - lo) / Real::two())),
        _ => None,
    }
}

fn make(lo: Real, hi: Real) -> Expr {
    if lo.eqv(&hi) {
        return num_r(lo);
    }
    intv_e(lo, hi)
}

pub fn from_mid_rad(mid: &Real, rad: &Real) -> Expr {
    make(mid - &rad.abs(), mid + &rad.abs())
}

fn fold_min(c: &[Real]) -> Real {
    c.iter().skip(1).fold(c[0].clone(), |a, b| Real::min(&a, b))
}
fn fold_max(c: &[Real]) -> Real {
    c.iter().skip(1).fold(c[0].clone(), |a, b| Real::max(&a, b))
}

pub fn add(a: &Expr, b: &Expr) -> Expr {
    let ((al, ah), (bl, bh)) = match (get(a), get(b)) {
        (Some(x), Some(y)) => (x, y),
        _ => throw("Invalid interval operation"),
    };
    if mode() == IntervalCalc::VarianceFormula {
        if let (Some((am, ar)), Some((bm, br))) = (mid_rad(a), mid_rad(b)) {
            let rad = rm::sqrt(&(&ar * &ar + &br * &br));
            return from_mid_rad(&(am + bm), &rad);
        }
    }
    make(al + bl, ah + bh)
}

pub fn mul(a: &Expr, b: &Expr) -> Expr {
    let ((al, ah), (bl, bh)) = match (get(a), get(b)) {
        (Some(x), Some(y)) => (x, y),
        _ => throw("Invalid interval operation"),
    };
    let m = mode();
    if let (Some((am, ar)), Some((bm, br))) = (mid_rad(a), mid_rad(b)) {
        if m == IntervalCalc::VarianceFormula && !am.is_zero() && !bm.is_zero() {
            let mm = &am * &bm;
            let ra = &ar / &am;
            let rb = &br / &bm;
            let rel = rm::sqrt(&(&ra * &ra + &rb * &rb));
            return from_mid_rad(&mm, &(&mm * &rel).abs());
        }
        if m == IntervalCalc::Simple {
            let mm = &am * &bm;
            let rad = (&ar * &bm).abs() + (&br * &am).abs();
            return from_mid_rad(&mm, &rad);
        }
    }
    let c = [&al * &bl, &al * &bh, &ah * &bl, &ah * &bh];
    make(fold_min(&c), fold_max(&c))
}

pub fn inverse(a: &Expr) -> Expr {
    let (lo, hi) = get(a).unwrap_or_else(|| throw("Invalid interval operation"));
    if lo.sign() <= 0 && hi.sign() >= 0 {
        throw("Division by an interval containing zero");
    }
    if mode() == IntervalCalc::VarianceFormula {
        if let Some((mid, rad)) = mid_rad(a) {
            let m = Real::one() / &mid;
            return from_mid_rad(&m, &(&rad / &(&mid * &mid)).abs());
        }
    }
    make(Real::one() / hi, Real::one() / lo)
}

pub fn pow(b: &Expr, e: &Expr) -> Expr {
    if let Some(x) = e.as_real() {
        if x.eqv(&Real::minus_one()) {
            return inverse(b);
        }
        if x.sign() < 0 {
            return inverse(&pow(b, &num_r(-x)));
        }
        let (lo, hi) = get(b).unwrap_or_else(|| throw("Unsupported interval power"));
        if mode() == IntervalCalc::VarianceFormula {
            if let Some((mid, rad)) = mid_rad(b) {
                if !mid.is_zero() {
                    let mut m = rm::pow(&mid.abs(), x);
                    if mid.sign() < 0 && x.is_exact_int() && !x.n.is_even() {
                        m = -m;
                    }
                    let r = (&m * x * &rad / &mid).abs();
                    return from_mid_rad(&m, &r);
                }
            }
        }
        if x.is_exact_int() && x.n.is_even() && lo.sign() < 0 && hi.sign() > 0 {
            return make(Real::zero(), Real::max(&lo.pow_int(&x.n), &hi.pow_int(&x.n)));
        }
        let a1 = rm::pow(&lo, x);
        let a2 = rm::pow(&hi, x);
        return make(Real::min(&a1, &a2), Real::max(&a1, &a2));
    }
    if let (Some((bl, bh)), Some((el, eh))) = (get(b), get(e)) {
        let c = [rm::pow(&bl, &el), rm::pow(&bl, &eh), rm::pow(&bh, &el), rm::pow(&bh, &eh)];
        return make(fold_min(&c), fold_max(&c));
    }
    throw("Unsupported interval power")
}

// applies a real function to an interval by sampling endpoints and interior points
pub fn apply(lo: &Real, hi: &Real, f: &mut dyn FnMut(Real) -> Real) -> Expr {
    if mode() == IntervalCalc::VarianceFormula {
        let mid = (lo + hi) / Real::two();
        let rad = (hi - lo) / Real::two();
        let m = f(mid.clone());
        let h = &rad / &Real::int(1000);
        if h.is_zero() {
            return num_r(m);
        }
        let deriv = (f(&mid + &h) - f(&mid - &h)) / (&h * &Real::two());
        return from_mid_rad(&m, &(deriv * rad).abs());
    }
    let mut rlo: Option<Real> = None;
    let mut rhi: Option<Real> = None;
    for k in 0..=16 {
        let x = lo + &((hi - lo) * Real::frac(k, 16));
        let y = f(x);
        rlo = Some(match rlo {
            None => y.clone(),
            Some(l) => Real::min(&l, &y),
        });
        rhi = Some(match rhi {
            None => y,
            Some(h) => Real::max(&h, &y),
        });
    }
    make(rlo.unwrap(), rhi.unwrap())
}