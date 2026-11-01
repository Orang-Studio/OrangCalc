use super::algebra as simp;
use super::*;
use std::sync::atomic::Ordering;

fn f(name: &str, a: Vec<Expr>) -> Expr {
    cur().ev.call(name, a)
}
fn f1(name: &str, a: &Expr) -> Expr {
    f(name, vec![a.clone()])
}

fn free(e: &Expr, x: &str) -> bool {
    !e.any_of(&mut |z| z.as_sym() == Some(x))
}

fn pi_e() -> Expr {
    let c = cur();
    c.var_ref(&c.variable("pi").unwrap())
}

pub fn diff(e: &Expr, x: &str) -> Expr {
    cur().check_abort();
    if free(e, x) {
        return zero();
    }
    match &e.e {
        E::Sym(s) => {
            if s == x {
                one()
            } else {
                zero()
            }
        }
        E::Add(t) => simp::add(t.iter().map(|t| diff(t, x)).collect()),
        E::Mul(m) => {
            let mut terms = vec![];
            for i in 0..m.len() {
                if free(&m[i], x) {
                    continue;
                }
                let mut fs = m.clone();
                fs[i] = diff(&m[i], x);
                terms.push(simp::mul(fs));
            }
            simp::add(terms)
        }
        E::Pow(b, px) => {
            if free(px, x) {
                return simp::mul(vec![px.clone(), simp::pow(b, &simp::sub(px, &one())), diff(b, x)]);
            }
            if free(b, x) {
                return simp::mul(vec![e.clone(), f1("ln", b), diff(px, x)]);
            }
            let lnb = f1("ln", b);
            simp::mul2(e, &simp::add2(&simp::mul2(&diff(px, x), &lnb), &simp::mul2(px, &simp::div(&diff(b, x), b))))
        }
        E::Mat(..) => matrices::map_elements(e, &mut |t| diff(&t, x)),
        E::Cmp(op, l, r) => cmp_e(*op, diff(l, x), diff(r, x)),
        E::Call(..) => diff_call(e, x),
        _ => throw_s(format!("Unable to differentiate {}", formatter::plain_text(e))),
    }
}

fn diff_call(ce: &Expr, x: &str) -> Expr {
    let E::Call(fnc, args) = &ce.e else { unreachable!() };
    let u = args.first().cloned().unwrap_or_else(zero);
    let du = if args.is_empty() { zero() } else { diff(&u, x) };
    let angle = angle_factor_inverse();
    let mh = num_r(Real::frac(-1, 2));
    let sq = |v: &Expr| simp::pow(v, &two());
    let outer = match fnc.name() {
        "sin" => simp::mul2(&f1("cos", &u), &angle),
        "cos" => simp::neg(&simp::mul2(&f1("sin", &u), &angle)),
        "tan" => simp::mul2(&simp::pow(&f1("cos", &u), &num_i(-2)), &angle),
        "cot" => simp::neg(&simp::mul2(&simp::pow(&f1("sin", &u), &num_i(-2)), &angle)),
        "sec" => simp::mul(vec![f1("sec", &u), f1("tan", &u), angle]),
        "csc" => simp::neg(&simp::mul(vec![f1("csc", &u), f1("cot", &u), angle])),
        "asin" => simp::div(&simp::pow(&simp::sub(&one(), &sq(&u)), &mh), &angle),
        "acos" => simp::neg(&simp::div(&simp::pow(&simp::sub(&one(), &sq(&u)), &mh), &angle)),
        "atan" => simp::div(&simp::pow(&simp::add2(&one(), &sq(&u)), &minus_one()), &angle),
        "sinh" => f1("cosh", &u),
        "cosh" => f1("sinh", &u),
        "tanh" => simp::pow(&f1("cosh", &u), &num_i(-2)),
        "asinh" => simp::pow(&simp::add2(&sq(&u), &one()), &mh),
        "acosh" => simp::pow(&simp::sub(&sq(&u), &one()), &mh),
        "atanh" => simp::pow(&simp::sub(&one(), &sq(&u)), &minus_one()),
        "exp" => ce.clone(),
        "ln" => simp::pow(&u, &minus_one()),
        "log" => {
            if args.len() > 1 {
                return diff(&simp::div(&f1("ln", &u), &f1("ln", &args[1])), x);
            }
            simp::pow(&u, &minus_one())
        }
        "sqrt" => simp::div(&half(), &f1("sqrt", &u)),
        "cbrt" => simp::div(&num_r(Real::frac(1, 3)), &simp::pow(&f1("cbrt", &u), &two())),
        "root" => return diff(&simp::pow(&u, &simp::pow(&args[1], &minus_one())), x),
        "abs" => f1("sgn", &u),
        "erf" => simp::mul2(&simp::div(&two(), &f1("sqrt", &pi_e())), &f1("exp", &simp::neg(&sq(&u)))),
        "gamma" => simp::mul2(&f1("gamma", &u), &f1("digamma", &u)),
        "lambertw" => simp::div(&f1("lambertw", &u), &simp::mul2(&u, &simp::add2(&one(), &f1("lambertw", &u)))),
        "Ei" => simp::div(&f1("exp", &u), &u),
        "li" => simp::pow(&f1("ln", &u), &minus_one()),
        "Si" => simp::div(&f1("sin", &u), &u),
        "Ci" => simp::div(&f1("cos", &u), &u),
        "sgn" => zero(),
        "heaviside" => f1("dirac", &u),
        _ => throw_s(format!("Unable to differentiate {}()", fnc.item.names[0])),
    };
    simp::mul2(&outer, &du)
}

fn angle_factor_inverse() -> Expr {
    let angle = cur().opts().angle;
    match angle {
        AngleMode::Degrees => simp::div(&pi_e(), &num_i(180)),
        AngleMode::Gradians => simp::div(&pi_e(), &num_i(200)),
        _ => one(),
    }
}

// symbolic antiderivative or None
pub fn integrate(e: &Expr, x: &str) -> Option<Expr> {
    cur().check_abort();
    let xs = sym(x);
    if free(e, x) {
        return Some(simp::mul2(e, &xs));
    }
    match &e.e {
        E::Sym(_) => Some(simp::div(&simp::pow(&xs, &two()), &two())),
        E::Add(t) => {
            let mut parts = vec![];
            for term in t {
                parts.push(integrate(term, x)?);
            }
            Some(simp::add(parts))
        }
        E::Mul(m) => {
            let consts: Vec<Expr> = m.iter().filter(|f| free(f, x)).cloned().collect();
            let vars: Vec<Expr> = m.iter().filter(|f| !free(f, x)).cloned().collect();
            if !consts.is_empty() {
                let inner = integrate(&simp::mul(vars), x)?;
                return Some(simp::mul2(&simp::mul(consts), &inner));
            }
            if let Some(bp) = integrate_by_parts(&vars, x) {
                return Some(bp);
            }
            let expanded = simp::expand(e);
            if expanded.is_add() {
                return integrate(&expanded, x);
            }
            rational_integral(e, x)
        }
        E::Pow(b, px) => {
            if free(px, x) {
                if let Some((a, _)) = linear(b, x) {
                    if px.is_minus_one() {
                        return Some(simp::div(&f1("ln", &f1("abs", b)), &a));
                    }
                    let n1 = simp::add2(px, &one());
                    return Some(simp::div(&simp::pow(b, &n1), &simp::mul2(&a, &n1)));
                }
            }
            if free(b, x) {
                if let Some((a2, _)) = linear(px, x) {
                    return Some(simp::div(e, &simp::mul2(&a2, &f1("ln", b))));
                }
            }
            if free(px, x) {
                if let E::Num(pn) = &px.e {
                    if pn.is_exact_int() && pn.re.sign() > 0 && pn.re.n <= real::big(20) {
                        return integrate(&simp::expand(e), x);
                    }
                }
            }
            rational_integral(e, x)
        }
        E::Call(..) => integrate_call(e, x),
        _ => None,
    }
}

fn linear(e: &Expr, x: &str) -> Option<(Expr, Expr)> {
    let cs = polynomials::try_coefficients(e, x)?;
    if cs.len() != 2 || cs[1].is_zero() {
        return None;
    }
    Some((cs[1].clone(), cs[0].clone()))
}

fn integrate_call(ce: &Expr, x: &str) -> Option<Expr> {
    let E::Call(fnc, args) = &ce.e else { return None };
    if args.is_empty() {
        return None;
    }
    let (a, _) = linear(&args[0], x)?;
    let u = &args[0];
    let ang = angle_factor_inverse();
    let sq = |v: &Expr| simp::pow(v, &two());
    let r = match fnc.name() {
        "sin" => simp::neg(&simp::div(&f1("cos", u), &ang)),
        "cos" => simp::div(&f1("sin", u), &ang),
        "tan" => simp::neg(&simp::div(&f1("ln", &f1("abs", &f1("cos", u))), &ang)),
        "sec" => simp::div(&f1("ln", &f1("abs", &simp::add2(&f1("sec", u), &f1("tan", u)))), &ang),
        "exp" => ce.clone(),
        "ln" => simp::sub(&simp::mul2(u, &f1("ln", u)), u),
        "sinh" => f1("cosh", u),
        "cosh" => f1("sinh", u),
        "tanh" => f1("ln", &f1("cosh", u)),
        "sqrt" => simp::mul2(&num_r(Real::frac(2, 3)), &simp::pow(u, &num_r(Real::frac(3, 2)))),
        "cbrt" => simp::mul2(&num_r(Real::frac(3, 4)), &simp::pow(u, &num_r(Real::frac(4, 3)))),
        "abs" => simp::div(&simp::mul2(u, &f1("abs", u)), &two()),
        "asin" => simp::add2(&simp::mul2(u, ce), &simp::div(&f1("sqrt", &simp::sub(&one(), &sq(u))), &ang)),
        "atan" => simp::sub(&simp::mul2(u, ce), &simp::div(&simp::div(&f1("ln", &simp::add2(&one(), &sq(u))), &two()), &ang)),
        "erf" => simp::add2(&simp::mul2(u, ce), &simp::div(&f1("exp", &simp::neg(&sq(u))), &f1("sqrt", &pi_e()))),
        "log" => {
            if args.len() > 1 {
                return integrate(&simp::div(&f1("ln", u), &f1("ln", &args[1])), x);
            }
            simp::sub(&simp::mul2(u, &f1("ln", u)), u)
        }
        _ => return None,
    };
    Some(simp::div(&r, &a))
}

// polynomial times exp/sin/cos via repeated integration by parts
fn integrate_by_parts(vars: &[Expr], x: &str) -> Option<Expr> {
    if vars.len() != 2 {
        return None;
    }
    let mut pi: Option<usize> = None;
    for (i, v) in vars.iter().enumerate() {
        if let Some(cs) = polynomials::try_coefficients(v, x) {
            if cs.len() > 1 && cs[1..].iter().any(|z| !z.is_zero()) {
                pi = Some(i);
            }
        }
    }
    let pi = pi?;
    let poly = &vars[pi];
    let other = &vars[1 - pi];
    let ok = match &other.e {
        E::Call(oc, oa) => ["exp", "sin", "cos", "sinh", "cosh"].contains(&oc.name()) && linear(&oa[0], x).is_some(),
        E::Pow(ob, ox) => free(ob, x) && linear(ox, x).is_some(),
        _ => false,
    };
    if !ok {
        if let E::Call(lc, la) = &other.e {
            if lc.name() == "ln" && linear(&la[0], x).is_some() {
                let ip = integrate(poly, x)?;
                let rest = integrate(&simp::expand(&simp::mul2(&ip, &diff(other, x))), x)?;
                return Some(simp::sub(&simp::mul2(&ip, other), &rest));
            }
        }
        return None;
    }
    let mut terms = vec![];
    let mut curp = poly.clone();
    let mut v = integrate(other, x)?;
    let mut sign = 1;
    for _ in 0..40 {
        if curp.is_zero() {
            break;
        }
        let t = simp::mul2(&curp, &v);
        terms.push(if sign > 0 { t } else { simp::neg(&t) });
        curp = diff(&curp, x);
        v = integrate(&v, x)?;
        sign = -sign;
    }
    Some(simp::add(terms))
}

fn rational_integral(e: &Expr, x: &str) -> Option<Expr> {
    let pf = polynomials::partial_fractions(cur(), e);
    if pf.key() != e.key() && !pf.is_mul() {
        let mut res = vec![];
        for t in simp::terms_of(&pf) {
            let r = integrate(&t, x).or_else(|| quadratic_integral(&t, x))?;
            res.push(r);
        }
        return Some(simp::add(res));
    }
    quadratic_integral(e, x)
}

// (b x + c)/(x^2 + p x + q) forms
fn quadratic_integral(e: &Expr, x: &str) -> Option<Expr> {
    let xs = sym(x);
    let (numr, den) = simp::num_den(e);
    let pn = polynomials::try_poly(&numr, x)?;
    let pd = polynomials::try_poly(&den, x)?;
    if pd.degree() == 1 && pn.degree() == 0 {
        return Some(simp::div(&simp::mul2(&num_r(pn.c[0].clone()), &f1("ln", &f1("abs", &den))), &num_r(pd.c[1].clone())));
    }
    if pd.degree() != 2 || pn.degree() > 1 {
        return None;
    }
    let (a, b2, cc) = (&pd.c[2], &pd.c[1], &pd.c[0]);
    let b = if pn.degree() == 1 { pn.c[1].clone() } else { Real::zero() };
    let c = &pn.c[0];
    let disc = Real::int(4) * a * cc - b2 * b2;
    let mut parts = vec![];
    let k1 = &b / &(Real::two() * a);
    if !k1.is_zero() {
        parts.push(simp::mul2(&num_r(k1.clone()), &f1("ln", &f1("abs", &den))));
    }
    let rem = c - &(&k1 * b2);
    if !rem.is_zero() {
        let lin = simp::add2(&simp::mul2(&num_r(Real::two() * a), &xs), &num_r(b2.clone()));
        if disc.sign() > 0 {
            let s = f1("sqrt", &num_r(disc));
            let ang = angle_factor_inverse();
            parts.push(simp::mul(vec![num_r(Real::two() * &rem), simp::pow(&s, &minus_one()), simp::mul2(&f1("atan", &simp::div(&lin, &s)), &ang)]));
        } else if disc.sign() < 0 {
            let s = f1("sqrt", &num_r(-disc));
            parts.push(simp::mul(vec![num_r(rem), simp::pow(&s, &minus_one()), f1("ln", &f1("abs", &simp::div(&simp::sub(&lin, &s), &simp::add2(&lin, &s))))]));
        } else {
            parts.push(simp::mul2(&num_r(-(Real::two() * &rem)), &simp::pow(&lin, &minus_one())));
        }
    }
    Some(simp::add(parts))
}

// adaptive Gauss-Kronrod 7-15
const XK: [f64; 8] = [0.991455371120812639, 0.949107912342758525, 0.864864423359769073, 0.741531185599394440, 0.586087235467691130, 0.405845151377397167, 0.207784955007898468, 0.0];
const WK: [f64; 8] = [0.022935322010529225, 0.063092092629978553, 0.104790010322250184, 0.140653259715525919, 0.169004726639267903, 0.190350578064785410, 0.204432940075298892, 0.209482141084727828];
const WG: [f64; 4] = [0.129484966168869693, 0.279705391489276668, 0.381830050505118945, 0.417959183673469388];

// returns (value, error estimate)
pub fn num_integrate(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> (f64, f64) {
    if a.is_infinite() || b.is_infinite() {
        if a == f64::NEG_INFINITY && b == f64::INFINITY {
            return num_integrate(
                &|t| {
                    let x = t / (1.0 - t * t);
                    f(x) * (1.0 + t * t) / (1.0 - t * t).powi(2)
                },
                -1.0 + 1e-12,
                1.0 - 1e-12,
            );
        }
        if b == f64::INFINITY {
            return num_integrate(&|t| f(a + t / (1.0 - t)) / (1.0 - t).powi(2), 0.0, 1.0 - 1e-12);
        }
        let (v, e) = num_integrate(&|t| f(b - t / (1.0 - t)) / (1.0 - t).powi(2), 0.0, 1.0 - 1e-12);
        return (-v, e);
    }
    let mut err = 0.0;
    let mut stack = vec![(a, b, 0)];
    let mut total = 0.0;
    let mut evals = 0;
    while let Some((lo, hi, d)) = stack.pop() {
        cur().check_abort();
        let (r, e) = gk(f, lo, hi);
        evals += 15;
        if r.is_nan() {
            throw("The integral could not be calculated (undefined values in range)");
        }
        if e < 1e-13 * 1f64.max(r.abs()) || d > 40 || evals > 400000 {
            total += r;
            err += e;
        } else {
            let m = (lo + hi) / 2.0;
            stack.push((lo, m, d + 1));
            stack.push((m, hi, d + 1));
        }
    }
    (total, err)
}

fn gk(f: &dyn Fn(f64) -> f64, a: f64, b: f64) -> (f64, f64) {
    let c = (a + b) / 2.0;
    let h = (b - a) / 2.0;
    let fc = f(c);
    let mut k = fc * WK[7];
    let mut g = fc * WG[3];
    for i in 0..7 {
        let dx = h * XK[i];
        let s = f(c - dx) + f(c + dx);
        k += WK[i] * s;
        if i % 2 == 1 {
            g += WG[i / 2] * s;
        }
    }
    (k * h, ((k - g) * h).abs())
}

pub fn limit(e: &Expr, x: &str, a: &Expr, dir: i32) -> Expr {
    let c = cur();
    let save = c.exact();
    let bits_save = real::bits();
    finally(
        || {
            if !matches!(&a.e, E::Num(an) if an.is_infinite()) {
                if let Ok(direct) = attempt(|| c.ev.eval_with(e, x, a.clone())) {
                    if matches!(&direct.e, E::Num(dn) if !dn.is_infinite()) && dir == 0 {
                        return direct;
                    }
                }
                let (mut numr, mut den) = simp::num_den(e);
                if !den.is_one() {
                    for _ in 0..6 {
                        let Ok((nv, dv)) = attempt(|| (c.ev.eval_with(&numr, x, a.clone()), c.ev.eval_with(&den, x, a.clone()))) else { break };
                        if !(nv.is_zero() && dv.is_zero()) {
                            break;
                        }
                        numr = diff(&numr, x);
                        den = diff(&den, x);
                        if let Ok(r) = attempt(|| c.ev.eval_with(&simp::div(&numr, &den), x, a.clone())) {
                            if matches!(&r.e, E::Num(rn) if !rn.is_infinite()) {
                                return r;
                            }
                        }
                    }
                }
            }
            c.set_exact(false);
            real::BITS.store(bits_save + 96, Ordering::Relaxed);
            let av = a.as_num().map(|n| n.re.to_f64()).unwrap_or(f64::NAN);
            if av.is_nan() {
                throw("Limit point must be a number");
            }
            let mut seq = vec![];
            for k in 2..=9 {
                let xv = if av == f64::INFINITY {
                    10f64.powi(k + 1)
                } else if av == f64::NEG_INFINITY {
                    -10f64.powi(k + 1)
                } else {
                    av + if dir < 0 { -1.0 } else { 1.0 } * 10f64.powi(-k)
                };
                seq.push(c.ev.eval_double(e, x, xv));
            }
            let last = *seq.last().unwrap();
            let prev = seq[seq.len() - 2];
            if dir == 0 && !av.is_infinite() {
                let other = c.ev.eval_double(e, x, av - 1e-9);
                if !other.is_nan() && !last.is_nan() && (other - last).abs() > 1e-4 * 1f64.max(other.abs()) {
                    throw("The limit does not exist (left and right limits differ)");
                }
            }
            if last.is_nan() {
                throw("Unable to compute the limit");
            }
            if last.abs() > 1e12 && last.abs() > prev.abs() * 10.0 {
                return num_r(if last > 0.0 { Real::pos_inf() } else { Real::neg_inf() });
            }
            if (last - prev).abs() > 1e-5 * 1f64.max(last.abs()) {
                throw("The limit does not seem to converge");
            }
            let r = (last * 1e9).round() / 1e9;
            num_r(Real::from_f64(if (r - last).abs() < 1e-9 { r } else { last }))
        },
        || {
            c.set_exact(save);
            real::BITS.store(bits_save, Ordering::Relaxed);
        },
    )
}