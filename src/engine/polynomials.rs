use super::algebra as simp;
use super::c64::{c64, C64};
use super::real::big;
use super::*;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::{HashMap, HashSet};

// univariate polynomial with rational coefficients, index = degree
#[derive(Clone, Debug)]
pub struct Poly {
    pub c: Vec<Real>,
}

impl Poly {
    pub fn new(mut c: Vec<Real>) -> Poly {
        while c.len() > 1 && c.last().unwrap().is_zero() {
            c.pop();
        }
        if c.is_empty() {
            c.push(Real::zero());
        }
        Poly { c }
    }
    pub fn degree(&self) -> usize {
        self.c.len() - 1
    }
    pub fn lead(&self) -> &Real {
        self.c.last().unwrap()
    }
    pub fn is_zero(&self) -> bool {
        self.c.len() == 1 && self.c[0].is_zero()
    }
    pub fn eval(&self, x: &Real) -> Real {
        let mut r = Real::zero();
        for ci in self.c.iter().rev() {
            r = r * x + ci;
        }
        r
    }
    // (quotient, remainder)
    pub fn div_mod(a: &Poly, b: &Poly) -> (Poly, Poly) {
        let mut r = a.c.clone();
        let db = b.degree();
        let qn = if a.degree() >= db { a.degree() - db + 1 } else { 1 };
        let mut q = vec![Real::zero(); qn.max(1)];
        if a.degree() >= db {
            for i in (0..=a.degree() - db).rev() {
                let f = &r[i + db] / b.lead();
                for j in 0..=db {
                    r[i + j] = &r[i + j] - &(&f * &b.c[j]);
                }
                q[i] = f;
            }
        }
        r.truncate(db.max(1));
        (Poly::new(q), Poly::new(r))
    }
    pub fn to_expr(&self, x: &Expr) -> Expr {
        let mut terms = vec![];
        for i in (0..self.c.len()).rev() {
            if self.c[i].is_zero() {
                continue;
            }
            terms.push(simp::mul2(&num_r(self.c[i].clone()), &simp::pow(x, &num_i(i as i64))));
        }
        simp::add(terms)
    }
}

// extracts integer-power coefficients of x from an expanded expression
pub fn try_coefficients(e: &Expr, x: &str) -> Option<Vec<Expr>> {
    let ex = simp::expand(e);
    let mut map: HashMap<usize, Vec<Expr>> = HashMap::new();
    for t in simp::terms_of(&ex) {
        let mut deg = 0usize;
        let mut rest = vec![];
        for f in simp::factors_of(&t) {
            if f.as_sym() == Some(x) {
                deg += 1;
                continue;
            }
            if let E::Pow(b, px) = &f.e {
                if b.as_sym() == Some(x) {
                    match &px.e {
                        E::Num(pn) if pn.is_exact_int() && !pn.re.n.is_negative() && pn.re.n < big(1000) => {
                            deg += pn.re.n.to_usize().unwrap();
                            continue;
                        }
                        _ => return None,
                    }
                }
            }
            if f.any_of(&mut |z| z.as_sym() == Some(x)) {
                return None;
            }
            rest.push(f);
        }
        map.entry(deg).or_default().push(simp::mul(rest));
    }
    let max = map.keys().copied().max().unwrap_or(0);
    Some((0..=max).map(|i| map.remove(&i).map(simp::add).unwrap_or_else(zero)).collect())
}

pub fn try_poly(e: &Expr, x: &str) -> Option<Poly> {
    let cs = try_coefficients(e, x)?;
    let mut rs = vec![];
    for c in cs {
        match &c.e {
            E::Num(n) if n.is_rational() => rs.push(n.re.clone()),
            _ => return None,
        }
    }
    Some(Poly::new(rs))
}

fn positive_divisors(n: &BigInt) -> Vec<BigInt> {
    let n = n.abs();
    if n.is_zero() {
        return vec![BigInt::one()];
    }
    let d = primes::divisors(&n);
    if n > big(1000000000000) {
        return d.into_iter().take(2000).collect();
    }
    d
}

// rational roots via the rational root theorem
pub fn rational_roots(p: &Poly) -> Vec<Real> {
    let mut roots = vec![];
    let mut curp = integerize(p);
    while curp.degree() >= 1 && curp.c[0].is_zero() {
        roots.push(Real::zero());
        curp = Poly::new(curp.c[1..].to_vec());
    }
    if curp.degree() < 1 {
        return roots;
    }
    let ps = positive_divisors(&curp.c[0].n);
    let qs = positive_divisors(&curp.lead().n);
    if ps.len() * qs.len() > 200000 {
        return roots;
    }
    let mut found = true;
    while found && curp.degree() >= 1 {
        found = false;
        'outer: for pp in &ps {
            for qq in &qs {
                for sgn in [1, -1] {
                    let r = Real::exact(pp * sgn, qq.clone());
                    if curp.eval(&r).is_zero() {
                        curp = Poly::div_mod(&curp, &Poly::new(vec![-&r, Real::one()])).0;
                        roots.push(r);
                        found = true;
                        break 'outer;
                    }
                }
            }
        }
    }
    roots
}

fn lcm_dens(p: &Poly) -> BigInt {
    let mut l = BigInt::one();
    for c in &p.c {
        l = &l / l.gcd(&c.d) * &c.d;
    }
    l
}

fn integerize(p: &Poly) -> Poly {
    let l = Real::big_int(lcm_dens(p));
    Poly::new(p.c.iter().map(|c| c * &l).collect())
}

pub fn factorize(c: &Calculator, e: &Expr) -> Expr {
    match &e.e {
        E::Num(n) if n.is_rational() => return factor_rational(&n.re),
        E::Mat(..) => return matrices::map_elements(e, &mut |x| factorize(c, &x)),
        E::Cmp(op, l, r) => return cmp_e(*op, factorize(c, l), factorize(c, r)),
        _ => {}
    }
    let syms = calculator::symbols(e);
    if syms.is_empty() {
        return e.clone();
    }
    let (numr, den) = simp::num_den(e);
    if !den.is_one() {
        let fnum = factorize(c, &numr);
        let fd = factorize(c, &den);
        return mul_e(vec![fnum, pow_e(fd, minus_one())]);
    }
    let ex = simp::expand(e);
    if syms.len() == 1 {
        if let Some(p) = try_poly(&ex, &syms[0]) {
            if p.degree() >= 2 {
                return factor_poly(&p, &sym(&syms[0]));
            }
        }
    }
    common_factor(&ex)
}

fn prime_parts(n: &BigInt) -> Vec<Expr> {
    primes::factorize(n).into_iter().map(|(k, v)| if v == 1 { num_r(Real::big_int(k)) } else { pow_e(num_r(Real::big_int(k)), num_i(v as i64)) }).collect()
}

fn factor_rational(r: &Real) -> Expr {
    if r.is_zero() || r.abs().is_one() {
        return num_r(r.clone());
    }
    let mut parts = vec![];
    if r.sign() < 0 {
        parts.push(minus_one());
    }
    parts.extend(prime_parts(&r.n));
    let num_part = match parts.len() {
        0 => one(),
        1 => parts.pop().unwrap(),
        _ => mul_e(parts),
    };
    if r.d.is_one() {
        return num_part;
    }
    let mut dparts = prime_parts(&r.d);
    let den_part = if dparts.len() == 1 { dparts.pop().unwrap() } else { mul_e(dparts) };
    mul_e(vec![num_part, pow_e(den_part, minus_one())])
}

pub fn factor_poly(p: &Poly, x: &Expr) -> Expr {
    let mut factors = vec![];
    let l = lcm_dens(p);
    let ip = Poly::new(p.c.iter().map(|cf| cf * &Real::big_int(l.clone())).collect());
    let mut g = BigInt::zero();
    for cf in &ip.c {
        g = g.gcd(&cf.n);
    }
    if ip.lead().sign() < 0 {
        g = -g;
    }
    let content = Real::exact(g.clone(), l);
    let prim = Poly::new(ip.c.iter().map(|cf| cf / &Real::big_int(g.clone())).collect());
    let roots = rational_roots(&prim);
    let mut rest = prim;
    let mut counts: Vec<(String, Expr, i64)> = vec![];
    for r in roots {
        let lin = Poly::new(vec![Real::big_int(-&r.n), Real::big_int(r.d.clone())]);
        rest = Poly::div_mod(&rest, &lin).0;
        let fe = simp::add2(&simp::mul2(&num_r(Real::big_int(r.d.clone())), x), &num_r(Real::big_int(-&r.n)));
        let k = fe.key().to_string();
        match counts.iter_mut().find(|c| c.0 == k) {
            Some(c) => c.2 += 1,
            None => counts.push((k, fe, 1)),
        }
    }
    let mult = &content * &if rest.degree() == 0 { rest.c[0].clone() } else { Real::one() };
    if !mult.is_one() {
        factors.push(num_r(mult));
    }
    for (_, f, k) in counts {
        factors.push(if k == 1 { f } else { pow_e(f, num_i(k)) });
    }
    if rest.degree() >= 1 {
        factors.push(rest.to_expr(x));
    }
    match factors.len() {
        0 => one(),
        1 => factors.pop().unwrap(),
        _ => mul_e(factors),
    }
}

fn monic(p: &Poly) -> Poly {
    let l = p.lead().clone();
    Poly::new(p.c.iter().map(|c| c / &l).collect())
}

fn common_factor(e: &Expr) -> Expr {
    let E::Add(terms) = &e.e else { return e.clone() };
    let mut g = BigInt::zero();
    let mut l = BigInt::one();
    for t in terms {
        let (cf, _) = simp::split_coef(t);
        if !cf.is_rational() {
            return e.clone();
        }
        g = g.gcd(&cf.re.n);
        l = &l / l.gcd(&cf.re.d) * &cf.re.d;
    }
    let mut common: Vec<(String, Expr, Real)> = vec![];
    let mut first = true;
    for t in terms {
        let (_, rest) = simp::split_coef(t);
        let mut here: Vec<(String, Expr, Real)> = vec![];
        for f in simp::factors_of(&rest) {
            if f.is_one() {
                continue;
            }
            let (b, ex) = match &f.e {
                E::Pow(pb, px) if matches!(&px.e, E::Num(pn) if pn.is_rational() && pn.re.sign() > 0) => (pb.clone(), px.num().re.clone()),
                _ => (f.clone(), Real::one()),
            };
            let k = b.key().to_string();
            match here.iter_mut().find(|h| h.0 == k) {
                Some(h) => {
                    h.1 = b;
                    h.2 = ex;
                }
                None => here.push((k, b, ex)),
            }
        }
        if first {
            common = here;
            first = false;
            continue;
        }
        common.retain_mut(|cm| match here.iter().find(|h| h.0 == cm.0) {
            Some(h) => {
                cm.2 = Real::min(&cm.2, &h.2);
                true
            }
            None => false,
        });
    }
    let mut cr = Real::exact(g, l);
    if terms.iter().all(|t| simp::split_coef(t).0.re.sign() < 0) {
        cr = -cr;
    }
    let common_expr = simp::mul(common.iter().map(|v| simp::pow(&v.1, &num_r(v.2.clone()))).collect());
    if cr.is_one() && common_expr.is_one() {
        return e.clone();
    }
    let inner = simp::add(terms.iter().map(|t| simp::div(&simp::div(t, &common_expr), &num_r(cr.clone()))).collect());
    let mut parts = vec![];
    if !cr.is_one() {
        parts.push(num_r(cr));
    }
    if !common_expr.is_one() {
        parts.extend(simp::factors_of(&common_expr));
    }
    parts.push(inner);
    mul_e(parts)
}

pub fn combine_fractions(e: &Expr) -> Expr {
    e.clone()
}

fn pmul(a: &Poly, b: &Poly) -> Poly {
    let mut r = vec![Real::zero(); a.c.len() + b.c.len() - 1];
    for i in 0..a.c.len() {
        for j in 0..b.c.len() {
            r[i + j] = &r[i + j] + &(&a.c[i] * &b.c[j]);
        }
    }
    Poly::new(r)
}

// partial fraction decomposition over rational linear and quadratic factors
pub fn partial_fractions(_c: &Calculator, e: &Expr) -> Expr {
    let syms = calculator::symbols(e);
    if syms.len() != 1 {
        return e.clone();
    }
    let xs = &syms[0];
    let x = sym(xs);
    let combined = together(e);
    let (numr, den) = simp::num_den(&combined);
    let numr = simp::expand(&numr);
    let den = simp::expand(&den);
    let (Some(pn), Some(pd)) = (try_poly(&numr, xs), try_poly(&den, xs)) else { return e.clone() };
    if pd.degree() < 1 {
        return e.clone();
    }
    let (q, rem) = Poly::div_mod(&pn, &pd);
    let poly_part = q.to_expr(&x);
    if rem.is_zero() {
        return poly_part;
    }
    let lead = pd.lead().clone();
    let monic_d = monic(&pd);
    let roots = rational_roots(&monic_d);
    let mut rest = monic_d.clone();
    let mut factors: Vec<(Poly, usize)> = vec![];
    let mut groups: Vec<(String, Real, usize)> = vec![];
    for r in roots {
        let k = format!("{}/{}", r.n, r.d);
        match groups.iter_mut().find(|g| g.0 == k) {
            Some(g) => g.2 += 1,
            None => groups.push((k, r, 1)),
        }
    }
    for (_, r, cnt) in groups {
        let lin = Poly::new(vec![-&r, Real::one()]);
        for _ in 0..cnt {
            rest = Poly::div_mod(&rest, &lin).0;
        }
        factors.push((lin, cnt));
    }
    if rest.degree() >= 1 {
        factors.push((rest, 1));
    }
    let mut cols: Vec<Poly> = vec![];
    for (f, k) in &factors {
        for j in 1..=*k {
            let mut other = monic_d.clone();
            for _ in 0..j {
                other = Poly::div_mod(&other, f).0;
            }
            for d in 0..f.degree() {
                let mut xd = vec![Real::zero(); d + 1];
                xd[d] = Real::one();
                cols.push(pmul(&other, &Poly::new(xd)));
            }
        }
    }
    let n = monic_d.degree();
    let a = matrices::create(n, cols.len(), &mut |i, j| num_r(cols[j].c.get(i).cloned().unwrap_or_else(Real::zero)));
    let target: Vec<Expr> = (0..n).map(|i| num_r(rem.c.get(i).map(|v| v / &lead).unwrap_or_else(Real::zero))).collect();
    let Ok(sol) = attempt(|| matrices::solve(&a, &target)) else { return e.clone() };
    let mut terms = vec![poly_part];
    let mut idx = 0;
    for (f, k) in &factors {
        for j in 1..=*k {
            let mut num_terms = vec![];
            for d in 0..f.degree() {
                num_terms.push(simp::mul2(&sol[idx], &simp::pow(&x, &num_i(d as i64))));
                idx += 1;
            }
            let numer = simp::add(num_terms);
            if numer.is_zero() {
                continue;
            }
            let fe = f.to_expr(&x);
            terms.push(mul_e(vec![numer, pow_e(if j == 1 { fe } else { pow_e(fe, num_i(j as i64)) }, minus_one())]));
        }
    }
    terms.retain(|t| !t.is_zero());
    if terms.len() == 1 {
        terms.pop().unwrap()
    } else {
        add_e(terms)
    }
}

// brings a sum of fractions over a common denominator
pub fn together(e: &Expr) -> Expr {
    let E::Add(terms) = &e.e else { return e.clone() };
    let mut nums = vec![];
    let mut dens = vec![];
    for t in terms {
        let (n, d) = simp::num_den(t);
        nums.push(n);
        dens.push(d);
    }
    let mut common = one();
    let mut seen = HashSet::new();
    for d in &dens {
        for f in simp::factors_of(d) {
            if f.is_one() {
                continue;
            }
            if seen.insert(f.key().to_string()) {
                common = simp::mul2(&common, &f);
            }
        }
    }
    let ts: Vec<Expr> = (0..nums.len()).map(|i| simp::expand(&simp::mul2(&nums[i], &simp::div(&common, &dens[i])))).collect();
    simp::div(&simp::add(ts), &common)
}

// all complex roots numerically (Aberth method)
pub fn numeric_roots(coef: &[C64]) -> Vec<C64> {
    let mut n = coef.len() - 1;
    while n > 0 && coef[n] == C64::default() {
        n -= 1;
    }
    if n < 1 {
        return vec![];
    }
    let a: Vec<C64> = coef[..=n].iter().map(|&c| c / coef[n]).collect();
    let radius = 1.0 + a[..n].iter().map(|c| c.abs()).fold(0.0, f64::max);
    let mut z: Vec<C64> = (0..n)
        .map(|i| {
            let r = radius * 0.5 + 0.1;
            let t = 2.0 * std::f64::consts::PI * i as f64 / n as f64 + 0.4;
            c64(r * t.cos(), r * t.sin())
        })
        .collect();
    for _ in 0..500 {
        let mut max_step: f64 = 0.0;
        for i in 0..n {
            let mut p = C64::default();
            let mut dp = C64::default();
            for k in (0..=n).rev() {
                dp = dp * z[i] + p;
                p = p * z[i] + a[k];
            }
            let ratio = p / dp;
            let mut sum = C64::default();
            for j in 0..n {
                if j != i {
                    sum += c64(1.0, 0.0) / (z[i] - z[j]);
                }
            }
            let mut w = ratio / (c64(1.0, 0.0) - ratio * sum);
            if w.re.is_nan() {
                w = C64::default();
            }
            z[i] -= w;
            max_step = max_step.max(w.abs());
        }
        if max_step < 1e-15 {
            break;
        }
    }
    z
}