use super::algebra as simp;
use super::c64::c64;
use super::polynomials::{self as poly, Poly};
use super::*;
use num_integer::Integer;
use std::collections::HashSet;
fn f(n: &str, a: &Expr) -> Expr {
    cur().ev.call(n, vec![a.clone()])
}
fn free(e: &Expr, x: &str) -> bool {
    !e.any_of(&mut |z| z.as_sym() == Some(x))
}
pub fn solve_for_display(c: &Calculator, eq: &Expr, x: &str) -> Expr {
    let sols = solve(c, eq, x);
    if sols.is_empty() {
        throw("No solution found");
    }
    let xs = sym(x);
    if sols.len() == 1 {
        return cmp_e(CmpOp::Eq, xs, sols[0].clone());
    }
    logic_e(LogicOp::Or, sols.into_iter().map(|s| cmp_e(CmpOp::Eq, xs.clone(), s)).collect())
}
pub fn solve(c: &Calculator, eq: &Expr, x: &str) -> Vec<Expr> {
    let fx = match &eq.e {
        E::Cmp(op, l, r) => {
            if *op != CmpOp::Eq {
                throw("Only equations can be solved");
            }
            simp::sub(l, r)
        }
        _ => eq.clone(),
    };
    let sols = solve_zero(&fx, x);
    let mut res: Vec<Expr> = vec![];
    let mut seen = HashSet::new();
    for s in sols {
        let sv = c.ev.eval(&s);
        if seen.insert(sv.key().to_string()) {
            res.push(sv);
        }
    }
    let mut keyed: Vec<(usize, Expr, i32, f64)> = res.into_iter().enumerate().map(|(i, r)| (i, r.clone(), if r.is_real_num() { 0 } else { 1 }, numeric_value(&r))).collect();
    keyed.sort_by(|a, b| a.2.cmp(&b.2).then(a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal)).then(a.0.cmp(&b.0)));
    keyed.into_iter().map(|t| t.1).collect()
}
fn solve_zero(fx: &Expr, x: &str) -> Vec<Expr> {
    cur().check_abort();
    if free(fx, x) {
        if fx.is_zero() {
            throw_s(format!("The equation is true for all {}", x));
        }
        throw("The equation has no solution");
    }
    let mut fx = fx.clone();
    let (numr, den) = simp::num_den(&fx);
    if !den.is_one() && !free(&den, x) {
        fx = numr;
    }
    let ex = simp::expand(&fx);
    if let Some(cs) = poly::try_coefficients(&ex, x) {
        if cs.len() >= 2 {
            if let Some(r) = solve_polynomial(cs) {
                return r;
            }
        }
    }
    if let Some(iso) = isolate(&fx, &zero(), x) {
        return iso;
    }
    numeric_roots(&fx, x)
}
fn solve_polynomial(mut cs: Vec<Expr>) -> Option<Vec<Expr>> {
    let mut deg = cs.len() - 1;
    while deg > 0 && cs[deg].is_zero() {
        deg -= 1;
    }
    if deg == 0 {
        return None;
    }
    let mut res = vec![];
    let mut zero_roots = 0;
    while cs[zero_roots].is_zero() {
        zero_roots += 1;
    }
    if zero_roots > 0 {
        res.push(zero());
        cs = cs[zero_roots..].to_vec();
        deg -= zero_roots;
    }
    if deg == 0 {
        return Some(res);
    }
    if deg == 1 {
        res.push(simp::neg(&simp::div(&cs[0], &cs[1])));
        return Some(res);
    }
    if deg == 2 {
        let (a, b, c) = (&cs[2], &cs[1], &cs[0]);
        let disc = simp::sub(&simp::pow(b, &two()), &simp::mul(vec![num_i(4), a.clone(), c.clone()]));
        let sq = simp::pow(&disc, &half());
        let two_a = simp::mul2(&two(), a);
        if disc.is_zero() {
            res.push(simp::div(&simp::neg(b), &two_a));
            return Some(res);
        }
        res.push(simp::div(&simp::add2(&simp::neg(b), &sq), &two_a));
        res.push(simp::div(&simp::sub(&simp::neg(b), &sq), &two_a));
        return Some(res);
    }
    if cs.iter().all(|e| matches!(&e.e, E::Num(n) if n.is_rational())) {
        let p = Poly::new(cs.iter().map(|e| e.num().re.clone()).collect());
        let rr = poly::rational_roots(&p);
        let mut rest = p;
        for r in rr {
            res.push(num_r(r.clone()));
            rest = Poly::div_mod(&rest, &Poly::new(vec![-r, Real::one()])).0;
        }
        if rest.degree() >= 1 {
            if rest.degree() <= 2 {
                if let Some(sub) = solve_polynomial(rest.c.iter().map(|z| num_r(z.clone())).collect()) {
                    res.extend(sub);
                }
            } else if rest.degree() % 2 == 0 && rest.c.iter().enumerate().filter(|(i, _)| i % 2 == 1).all(|(_, z)| z.is_zero()) {
                let halfc: Vec<Expr> = rest.c.iter().enumerate().filter(|(i, _)| i % 2 == 0).map(|(_, z)| num_r(z.clone())).collect();
                if let Some(inner) = solve_polynomial(halfc) {
                    for s in inner {
                        let root = simp::pow(&s, &half());
                        res.push(root.clone());
                        res.push(simp::neg(&root));
                    }
                }
            } else {
                let roots = poly::numeric_roots(&rest.c.iter().map(|z| c64(z.to_f64(), 0.0)).collect::<Vec<_>>());
                for z in roots {
                    let im = if z.im.abs() < 1e-12 * 1f64.max(z.re.abs()) { 0.0 } else { z.im };
                    res.push(num(Cx::new(Real::from_f64(z.re), if im == 0.0 { Real::zero() } else { Real::from_f64(im) })));
                }
            }
        }
        res.sort_by_key(|e| if e.is_real_num() { 0 } else { 1 });
        return Some(res);
    }
    if cs.iter().all(|e| e.is_num()) {
        let roots = poly::numeric_roots(&cs.iter().map(|e| funcs::to_c(e.num())).collect::<Vec<_>>());
        res.extend(roots.into_iter().map(|z| num(Cx::from_c64(z.re, z.im))));
        return Some(res);
    }
    None
}
fn isolate(lhs: &Expr, rhs: &Expr, x: &str) -> Option<Vec<Expr>> {
    let mut lhs = lhs.clone();
    let mut rhs = rhs.clone();
    for _ in 0..50 {
        if lhs.as_sym() == Some(x) {
            return Some(vec![rhs]);
        }
        let node = lhs.clone();
        match &node.e {
            E::Add(t) => {
                let with_x: Vec<Expr> = t.iter().filter(|t| !free(t, x)).cloned().collect();
                if with_x.len() != 1 {
                    return None;
                }
                let others: Vec<Expr> = t.iter().filter(|t| free(t, x)).cloned().collect();
                rhs = simp::sub(&rhs, &simp::add(others));
                lhs = with_x[0].clone();
            }
            E::Mul(m) => {
                let with_x: Vec<Expr> = m.iter().filter(|t| !free(t, x)).cloned().collect();
                if with_x.len() != 1 {
                    return None;
                }
                let others: Vec<Expr> = m.iter().filter(|t| free(t, x)).cloned().collect();
                rhs = simp::div(&rhs, &simp::mul(others));
                lhs = with_x[0].clone();
            }
            E::Pow(b, px) => {
                if free(px, x) {
                    let odd_int = matches!(&px.e, E::Num(n) if n.is_exact_int() && !n.re.n.is_even());
                    if odd_int || !px.is_num() {
                        rhs = simp::pow(&rhs, &simp::pow(px, &minus_one()));
                        lhs = b.clone();
                        continue;
                    }
                    if matches!(&px.e, E::Num(n) if n.is_exact_int() && n.re.n.is_even()) {
                        let root = simp::pow(&rhs, &simp::pow(px, &minus_one()));
                        let mut a1 = isolate(b, &root, x).unwrap_or_default();
                        a1.extend(isolate(b, &simp::neg(&root), x).unwrap_or_default());
                        return Some(a1);
                    }
                    if matches!(&px.e, E::Num(n) if n.is_rational()) {
                        rhs = simp::pow(&rhs, &simp::pow(px, &minus_one()));
                        lhs = b.clone();
                        continue;
                    }
                    return None;
                }
                if free(b, x) {
                    rhs = simp::div(&f("ln", &rhs), &f("ln", b));
                    lhs = px.clone();
                    continue;
                }
                return None;
            }
            E::Call(_, args) if !args.is_empty() && args[1..].iter().all(|a| free(a, x)) => {
                let inv = inverse(&lhs, &rhs)?;
                rhs = inv;
                lhs = args[0].clone();
            }
            _ => return None,
        }
    }
    None
}
fn inverse(c: &Expr, y: &Expr) -> Option<Expr> {
    let E::Call(fnc, args) = &c.e else { return None };
    Some(match fnc.name() {
        "sin" => f("asin", y),
        "cos" => f("acos", y),
        "tan" => f("atan", y),
        "asin" => f("sin", y),
        "acos" => f("cos", y),
        "atan" => f("tan", y),
        "sinh" => f("asinh", y),
        "cosh" => f("acosh", y),
        "tanh" => f("atanh", y),
        "asinh" => f("sinh", y),
        "acosh" => f("cosh", y),
        "atanh" => f("tanh", y),
        "exp" => f("ln", y),
        "ln" => f("exp", y),
        "log" => {
            if args.len() > 1 {
                simp::pow(&args[1], y)
            } else {
                f("exp", y)
            }
        }
        "log10" => simp::pow(&num_i(10), y),
        "log2" => simp::pow(&two(), y),
        "sqrt" => simp::pow(y, &two()),
        "cbrt" => simp::pow(y, &num_i(3)),
        "root" => simp::pow(y, &args[1]),
        "lambertw" => simp::mul2(y, &f("exp", y)),
        _ => return None,
    })
}
fn numeric_roots(fx: &Expr, x: &str) -> Vec<Expr> {
    let c = cur();
    c.with_exact(false, || {
        let mut roots: Vec<f64> = vec![];
        let g = |v: f64| c.ev.eval_double(fx, x, v);
        let pts: Vec<f64> = (-400..=400).map(|i: i32| (i.signum() as f64) * (i.abs() as f64 / 40.0).powi(3)).collect();
        let mut prev_x = pts[0];
        let mut prev_y = g(prev_x);
        for &px in &pts[1..] {
            c.check_abort();
            let py = g(px);
            if !py.is_nan() && !prev_y.is_nan() {
                if py == 0.0 {
                    add_root(&mut roots, px);
                } else if py.signum() != prev_y.signum() && prev_y != 0.0 {
                    let r = bisect(&g, prev_x, px);
                    if !r.is_nan() && g(r).abs() < 1e-6 * 1f64.max(r.abs()) {
                        add_root(&mut roots, r);
                    }
                }
            }
            prev_x = px;
            prev_y = py;
        }
        if roots.is_empty() {
            for start in [0.5, 1.0, -1.0, 2.0, 10.0, -10.0, 0.1] {
                let r = newton(&g, start);
                if !r.is_nan() {
                    add_root(&mut roots, r);
                    break;
                }
            }
        }
        if roots.is_empty() {
            throw("No solution found");
        }
        roots.into_iter().map(|r| num_r(Real::from_f64(clean(r)))).collect()
    })
}

fn clean(r: f64) -> f64 {
    let rr = (r * 1e12).round() / 1e12;
    if (rr - r).abs() < 1e-13 * 1f64.max(r.abs()) {
        rr
    } else {
        r
    }
}

fn add_root(roots: &mut Vec<f64>, r: f64) {
    if !roots.iter().any(|q| (q - r).abs() < 1e-9 * 1f64.max(r.abs())) {
        roots.push(r);
    }
}

fn bisect(g: &dyn Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let mut fa = g(a);
    for _ in 0..200 {
        let m = (a + b) / 2.0;
        let fm = g(m);
        if fm.is_nan() {
            return f64::NAN;
        }
        if fm == 0.0 || (b - a) / 2.0 < 1e-16 * 1f64.max(m.abs()) {
            return m;
        }
        if fm.signum() == fa.signum() {
            a = m;
            fa = fm;
        } else {
            b = m;
        }
    }
    (a + b) / 2.0
}
pub fn newton(g: &dyn Fn(f64) -> f64, x0: f64) -> f64 {
    let mut x = x0;
    for _ in 0..100 {
        let fx = g(x);
        if fx.is_nan() {
            return f64::NAN;
        }
        if fx.abs() < 1e-15 {
            return x;
        }
        let h = 1e-7 * 1f64.max(x.abs());
        let d = (g(x + h) - g(x - h)) / (2.0 * h);
        if d == 0.0 || d.is_nan() {
            return f64::NAN;
        }
        let nx = x - fx / d;
        if (nx - x).abs() < 1e-15 * 1f64.max(x.abs()) {
            return nx;
        }
        x = nx;
    }
    if g(x).abs() < 1e-9 {
        x
    } else {
        f64::NAN
    }
}
pub fn secant(g: &dyn Fn(f64) -> f64, mut x0: f64, mut x1: f64) -> f64 {
    let mut f0 = g(x0);
    let mut f1 = g(x1);
    for _ in 0..200 {
        if f1 == f0 {
            break;
        }
        let x2 = x1 - f1 * (x1 - x0) / (f1 - f0);
        x0 = x1;
        f0 = f1;
        x1 = x2;
        f1 = g(x1);
        if f1.abs() < 1e-15 || (x1 - x0).abs() < 1e-15 * 1f64.max(x1.abs()) {
            return x1;
        }
    }
    if f1.abs() < 1e-9 {
        x1
    } else {
        f64::NAN
    }
}
pub fn multi_solve(c: &Calculator, eqs: &[Expr], vars: &[String]) -> Vec<Expr> {
    let n = vars.len();
    if eqs.len() < n {
        throw("Not enough equations");
    }
    let mut rows = vec![];
    let mut rhs = vec![];
    for e in eqs {
        let fx = match &e.e {
            E::Cmp(_, l, r) => simp::sub(l, r),
            _ => e.clone(),
        };
        let fx = simp::expand(&fx);
        let mut row = vec![];
        for v in vars {
            match poly::try_coefficients(&fx, v) {
                Some(cs) if cs.len() <= 2 => row.push(if cs.len() == 2 { cs[1].clone() } else { zero() }),
                _ => return non_linear(c, eqs, vars),
            }
        }
        let mut constant = fx.clone();
        for v in vars {
            constant = c.ev.eval_with(&constant, v, zero());
        }
        rows.push(row);
        rhs.push(simp::neg(&constant));
    }
    rows.truncate(n);
    rhs.truncate(n);
    matrices::solve(&mat_e(rows, false), &rhs)
}
fn non_linear(c: &Calculator, eqs: &[Expr], vars: &[String]) -> Vec<Expr> {
    if vars.len() == 1 {
        return solve(c, &eqs[0], &vars[0]).into_iter().take(1).collect();
    }
    for i in 0..eqs.len() {
        let Ok(sol) = attempt(|| solve(c, &eqs[i], &vars[0])) else { continue };
        for s in sol {
            let r = attempt(|| {
                let rest: Vec<Expr> = eqs.iter().enumerate().filter(|(k, _)| *k != i).map(|(_, e)| c.ev.eval(&c.substitute(e, &sym(&vars[0]), &s))).collect();
                let sub = multi_solve(c, &rest, &vars[1..]);
                let mut first = s.clone();
                for k in 1..vars.len() {
                    first = c.ev.eval(&c.substitute(&first, &sym(&vars[k]), &sub[k - 1]));
                }
                let mut out = vec![first];
                out.extend(sub);
                out
            });
            if let Ok(v) = r {
                return v;
            }
        }
    }
    throw("Unable to solve the system")
}
pub fn dsolve(c: &Calculator, eq: &Expr, y: &str, x: &str, x0: Option<Expr>, y0: Option<Expr>) -> Expr {
    let dy = sym("_dy");
    let marked = c.ev.eval(&eq.map(&mut |e| if e.is_call_of("diff") { dy.clone() } else { e }));
    let fx = match &marked.e {
        E::Cmp(_, l, r) => simp::sub(l, r),
        _ => marked.clone(),
    };
    let fx = simp::expand(&fx);
    let cd = poly::try_coefficients(&fx, "_dy").filter(|cd| cd.len() == 2).unwrap_or_else(|| throw("Only first order linear differential equations are supported"));
    let k = &cd[1];
    let rest = simp::expand(&simp::neg(&simp::div(&cd[0], k)));
    let cy = poly::try_coefficients(&rest, y).filter(|cy| cy.len() <= 2).unwrap_or_else(|| throw("Only linear differential equations are supported"));
    let a = if cy.len() == 2 { cy[1].clone() } else { zero() };
    let b = cy[0].clone();
    let xs = sym(x);
    let cc = sym("C");
    let mut sol = if a.is_zero() {
        simp::add2(&calculus::integrate(&b, x).unwrap_or_else(|| throw("Unable to integrate")), &cc)
    } else {
        if !free(&a, x) {
            throw("Only constant coefficients are supported");
        }
        let ex = f("exp", &simp::mul2(&a, &xs));
        let ib = calculus::integrate(&simp::mul2(&b, &f("exp", &simp::neg(&simp::mul2(&a, &xs)))), x).unwrap_or_else(|| throw("Unable to integrate"));
        simp::mul2(&ex, &simp::add2(&ib, &cc))
    };
    if let (Some(x0), Some(y0)) = (x0, y0) {
        let at_x0 = c.ev.eval_with(&sol, x, x0);
        let cs = solve(c, &cmp_e(CmpOp::Eq, at_x0, y0), "C");
        if let Some(c0) = cs.first() {
            sol = c.ev.eval(&c.substitute(&sol, &cc, c0));
        }
    }
    simp::expand(&sol)
}
fn numeric_value(e: &Expr) -> f64 {
    if let Some(r) = e.as_real() {
        return r.to_f64();
    }
    let c = cur();
    attempt(|| c.with_exact(false, || c.ev.eval(e).as_real().map(|r| r.to_f64()).unwrap_or(f64::MAX))).unwrap_or(f64::MAX)
}