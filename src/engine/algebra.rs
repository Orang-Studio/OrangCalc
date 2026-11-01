// canonicalizing constructors used during evaluation
use super::calculator::try_cur;
use super::real::big;
use super::*;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

fn c() -> Option<&'static Calculator> {
    try_cur()
}

pub fn add2(a: &Expr, b: &Expr) -> Expr {
    add(vec![a.clone(), b.clone()])
}
pub fn sub(a: &Expr, b: &Expr) -> Expr {
    add(vec![a.clone(), neg(b)])
}
pub fn neg(a: &Expr) -> Expr {
    mul(vec![minus_one(), a.clone()])
}
pub fn mul2(a: &Expr, b: &Expr) -> Expr {
    mul(vec![a.clone(), b.clone()])
}
pub fn div(a: &Expr, b: &Expr) -> Expr {
    mul(vec![a.clone(), pow(b, &minus_one())])
}

pub fn split_coef(t: &Expr) -> (Cx, Expr) {
    match &t.e {
        E::Num(n) => (n.clone(), one()),
        E::Mul(f) if !f.is_empty() && f[0].is_num() => {
            let rest = if f.len() == 2 { f[1].clone() } else { mul_e(f[1..].to_vec()) };
            (f[0].num().clone(), rest)
        }
        _ => (Cx::one(), t.clone()),
    }
}

fn with_coef(coef: Cx, rest: &Expr) -> Expr {
    if rest.is_one() {
        return num(coef);
    }
    if coef.is_one() {
        return rest.clone();
    }
    let mut l = vec![num(coef)];
    match &rest.e {
        E::Mul(f) => l.extend(f.iter().cloned()),
        _ => l.push(rest.clone()),
    }
    mul_e(l)
}

pub fn factors_of(e: &Expr) -> Vec<Expr> {
    match &e.e {
        E::Mul(f) => f.clone(),
        _ => vec![e.clone()],
    }
}

pub fn terms_of(e: &Expr) -> Vec<Expr> {
    match &e.e {
        E::Add(t) => t.clone(),
        _ => vec![e.clone()],
    }
}

fn wrap(mut l: Vec<Expr>) -> Expr {
    match l.len() {
        0 => one(),
        1 => l.pop().unwrap(),
        _ => mul_e(l),
    }
}

// splits a term into non unit part and unit part
pub fn split_units(rest: &Expr) -> (Expr, Expr) {
    let mut p = vec![];
    let mut u = vec![];
    for f in factors_of(rest) {
        if is_unit_factor(&f) {
            u.push(f);
        } else {
            p.push(f);
        }
    }
    (wrap(p), wrap(u))
}

pub fn split_units_of(e: &Expr) -> Expr {
    let (_, rest) = split_coef(e);
    split_units(&rest).1
}

pub fn is_unit_factor(f: &Expr) -> bool {
    match &f.e {
        E::Unit(..) => true,
        E::Pow(b, x) => b.is_unit() && x.is_num(),
        _ => false,
    }
}

pub fn add(input: Vec<Expr>) -> Expr {
    let mut terms: Vec<Expr> = vec![];
    for t in input {
        match &t.e {
            E::Add(a) => terms.extend(a.iter().cloned()),
            _ => terms.push(t),
        }
    }
    if terms.is_empty() {
        return zero();
    }
    if terms.len() == 1 {
        return terms.pop().unwrap();
    }
    if terms.iter().any(|t| t.is_mat()) {
        return matrix_add(terms);
    }
    if terms.iter().any(|t| t.is_str()) {
        return str_e(&terms.iter().map(|t| t.as_str().map(|s| s.to_string()).unwrap_or_else(|| formatter::plain_text(t))).collect::<String>());
    }
    if terms.iter().any(|t| t.is_date() || matches!(&t.e, E::Mul(f) if f.iter().any(|x| x.is_date()))) {
        return date_add(terms);
    }
    if terms.iter().any(|t| t.is_intv()) {
        let mut acc: Option<Expr> = None;
        let mut others = vec![];
        for t in &terms {
            if t.is_intv() || t.is_real_num() {
                acc = Some(match acc {
                    None => t.clone(),
                    Some(a) => intervals::add(&a, t),
                });
            } else {
                others.push(t.clone());
            }
        }
        if others.is_empty() {
            return acc.unwrap();
        }
        others.push(acc.unwrap());
        terms = others;
    }

    let mut numv = Cx::zero();
    let mut had_num = false;
    let mut order: Vec<String> = vec![];
    let mut coefs: HashMap<String, Cx> = HashMap::new();
    let mut rests: HashMap<String, Expr> = HashMap::new();
    for t in &terms {
        if let E::Num(n) = &t.e {
            numv = numv + n;
            had_num = true;
            continue;
        }
        let (cf, rest) = split_coef(t);
        let k = rest.key().to_string();
        if let Some(old) = coefs.get_mut(&k) {
            *old = &*old + &cf;
        } else {
            coefs.insert(k.clone(), cf);
            rests.insert(k.clone(), rest);
            order.push(k);
        }
    }

    if c().is_some() && order.len() > 1 {
        merge_unit_terms(&mut order, &mut coefs, &mut rests);
    }

    let mut result = vec![];
    for k in &order {
        let cf = &coefs[k];
        if cf.is_zero() {
            continue;
        }
        result.push(with_coef(cf.clone(), &rests[k]));
    }
    if !numv.is_zero() || (had_num && result.is_empty()) {
        result.push(num(numv));
    }
    if result.is_empty() {
        return zero();
    }
    if result.len() == 1 {
        return result.pop().unwrap();
    }
    result.sort_by(compare_terms);
    add_e(result)
}

// terms like 1 m + 2 ft are converted to the unit of the first term
fn merge_unit_terms(order: &mut Vec<String>, coefs: &mut HashMap<String, Cx>, rests: &mut HashMap<String, Expr>) {
    let mut i = 0;
    while i < order.len() {
        let ri = rests[&order[i]].clone();
        if ri.has_units() {
            let (pi, ui) = split_units(&ri);
            if let Some((di, fi)) = units::try_dims(&ui) {
                let mut j = i + 1;
                while j < order.len() {
                    let rj = rests[&order[j]].clone();
                    if rj.has_units() {
                        let (pj, uj) = split_units(&rj);
                        if pj.key() == pi.key() {
                            if let Some((dj, fj)) = units::try_dims(&uj) {
                                if di.same_as(&dj) {
                                    let ratio = Cx::real(&fj / &fi);
                                    let v = &coefs[&order[i]] + &(&coefs[&order[j]] * &ratio);
                                    coefs.insert(order[i].clone(), v);
                                    coefs.remove(&order[j]);
                                    rests.remove(&order[j]);
                                    order.remove(j);
                                    continue;
                                }
                            }
                        }
                    }
                    j += 1;
                }
            }
        }
        i += 1;
    }
}

fn date_add(terms: Vec<Expr>) -> Expr {
    let cc = cur();
    let mut base_date: Option<Expr> = None;
    let mut sub_date: Option<Expr> = None;
    let mut seconds = Real::zero();
    let mut months: i64 = 0;
    for t in &terms {
        if t.is_date() {
            if base_date.is_none() {
                base_date = Some(t.clone());
            } else {
                throw("Two dates can not be added");
            }
            continue;
        }
        if let E::Mul(f) = &t.e {
            if f.len() == 2 && f[0].is_minus_one() && f[1].is_date() {
                if sub_date.is_some() {
                    throw("Unsupported date expression");
                }
                sub_date = Some(f[1].clone());
                continue;
            }
        }
        if let Some(r) = t.as_real() {
            seconds = seconds + r * &Real::int(86400);
            continue;
        }
        if let E::Unit(u, None) = &t.e {
            if u.name() == "yr" || u.name() == "mo" {
                months += if u.name() == "yr" { 12 } else { 1 };
                continue;
            }
        }
        if let E::Mul(f) = &t.e {
            if f.len() == 2 {
                if let (Some(n), E::Unit(u, None)) = (f[0].as_real(), &f[1].e) {
                    if n.is_int() {
                        let per = match u.name() {
                            "yr" => 12,
                            "mo" => 1,
                            "decade" => 120,
                            "century" => 1200,
                            "millennium" => 12000,
                            _ => 0,
                        };
                        if per > 0 {
                            months += n.to_int() as i64 * per;
                            continue;
                        }
                    }
                }
            }
        }
        if let Some(secs) = units::to_seconds(t) {
            seconds = seconds + secs;
            continue;
        }
        throw("Invalid date arithmetic");
    }
    if let (Some(b), Some(s)) = (&base_date, &sub_date) {
        let (E::Date(bt, bh, _), E::Date(st, sh, _)) = (&b.e, &s.e) else { unreachable!() };
        let diff = (*bt - *st).num_microseconds().unwrap_or(0);
        let s_ = Real::make(big(diff), big(1000000), false) + &seconds;
        if !bh && !sh && seconds.is_zero() {
            return mul2(&num_r(s_ / Real::int(86400)), &unit_e(cc.unit("d"), None));
        }
        return mul2(&num_r(s_), &unit_e(cc.unit("s"), None));
    }
    let Some(mut base) = base_date else { throw("Invalid date arithmetic") };
    if months != 0 {
        if let E::Date(t, h, u) = &base.e {
            base = date_e(dates::add_months(*t, months), *h, *u);
        }
    }
    dates::add_seconds(&base, &seconds)
}

fn rank(e: &Expr) -> i32 {
    match &e.e {
        E::Num(_) | E::Intv(..) => 9,
        _ => 1,
    }
}

pub fn degree(e: &Expr) -> i32 {
    match &e.e {
        E::Sym(_) | E::Var(_) | E::Call(..) => 1,
        E::Pow(b, x) => match x.as_real() {
            Some(n) => (degree(b) as f64 * n.to_f64().round()).clamp(-100.0, 100.0) as i32,
            None => degree(b),
        },
        E::Mul(f) => f.iter().map(degree).sum(),
        _ => 0,
    }
}

fn main_name(e: &Expr) -> Option<String> {
    match &e.e {
        E::Sym(s) => Some(s.clone()),
        E::Var(v) => Some(format!("~{}", v.name())),
        E::Pow(b, _) => main_name(b),
        E::Mul(f) => f.iter().find_map(main_name),
        E::Call(f, _) => Some(format!("~~{}", f.name())),
        _ => None,
    }
}

// ordinal string comparison by UTF-16 code units, as the original engine sorted
pub fn cmp_ordinal(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

pub fn compare_terms(a: &Expr, b: &Expr) -> Ordering {
    let (ra, rb) = (rank(a), rank(b));
    if ra != rb {
        return ra.cmp(&rb);
    }
    let na = main_name(a).unwrap_or_else(|| "\u{FFFF}".into());
    let nb = main_name(b).unwrap_or_else(|| "\u{FFFF}".into());
    let c = cmp_ordinal(&na, &nb);
    if c != Ordering::Equal {
        return c;
    }
    let (da, db) = (degree(a), degree(b));
    if da != db {
        return db.cmp(&da);
    }
    cmp_ordinal(a.key(), b.key())
}

fn factor_rank(e: &Expr) -> i32 {
    match &e.e {
        E::Num(_) | E::Intv(..) => 0,
        E::Mat(..) => 5,
        E::Unit(..) => 7,
        E::Pow(b, x) if b.is_unit() => {
            if x.as_real().map(|r| r.sign() < 0).unwrap_or(false) {
                8
            } else {
                7
            }
        }
        E::Add(_) => 3,
        _ => 2,
    }
}

pub fn compare_factors(a: &Expr, b: &Expr) -> Ordering {
    let (ra, rb) = (factor_rank(a), factor_rank(b));
    if ra != rb {
        return ra.cmp(&rb);
    }
    if ra == 5 {
        return Ordering::Equal;
    }
    let na = main_name(a).unwrap_or_else(|| a.key().to_string());
    let nb = main_name(b).unwrap_or_else(|| b.key().to_string());
    let c = cmp_ordinal(&na, &nb);
    if c != Ordering::Equal {
        return c;
    }
    cmp_ordinal(a.key(), b.key())
}

fn base_exp(f: &Expr) -> (Expr, Expr) {
    match &f.e {
        E::Pow(b, x) => (b.clone(), x.clone()),
        _ => (f.clone(), one()),
    }
}

pub fn mul(input: Vec<Expr>) -> Expr {
    let mut fs: Vec<Expr> = vec![];
    for f in input {
        match &f.e {
            E::Mul(m) => fs.extend(m.iter().cloned()),
            _ => fs.push(f),
        }
    }
    if fs.is_empty() {
        return one();
    }
    if fs.len() == 1 {
        return fs.pop().unwrap();
    }
    if fs.iter().any(|f| f.is_mat()) {
        return matrix_mul(fs);
    }
    if fs.iter().any(|f| f.is_date()) {
        if fs.len() == 2 && fs.iter().any(|f| f.is_minus_one()) {
            fs.sort_by_key(|f| if f.is_date() { 1 } else { 0 });
            return mul_e(fs);
        }
        throw("Dates can not be multiplied");
    }
    if fs.iter().any(|f| f.is_str()) {
        throw("Text can not be multiplied");
    }
    if let Some(idx) = fs.iter().position(|f| f.is_cmp()) {
        let others: Vec<Expr> = fs.iter().enumerate().filter(|(i, _)| *i != idx).map(|(_, f)| f.clone()).collect();
        if others.iter().all(|o| o.is_num()) {
            let k = mul(others);
            if let E::Cmp(op, l, r) = &fs[idx].e {
                return cur().ev.eval_cmp(&cmp_e(*op, mul2(&k, l), mul2(&k, r)));
            }
        }
    }
    if fs.iter().any(|f| f.is_intv()) {
        let mut acc: Option<Expr> = None;
        let mut others = vec![];
        for f in &fs {
            if f.is_intv() || f.is_real_num() {
                acc = Some(match acc {
                    None => f.clone(),
                    Some(a) => intervals::mul(&a, f),
                });
            } else {
                others.push(f.clone());
            }
        }
        let acc = acc.unwrap();
        if others.is_empty() {
            return acc;
        }
        others.insert(0, acc.clone());
        fs = others;
        if !acc.is_num() {
            return mul_e(sorted(fs));
        }
    }

    let mut coef = Cx::one();
    let mut order: Vec<String> = vec![];
    let mut bases: HashMap<String, Expr> = HashMap::new();
    let mut exps: HashMap<String, Vec<Expr>> = HashMap::new();
    for f in &fs {
        if let E::Num(n) = &f.e {
            coef = coef * n;
            continue;
        }
        let (b, e) = base_exp(f);
        let k = b.key().to_string();
        if !exps.contains_key(&k) {
            exps.insert(k.clone(), vec![]);
            bases.insert(k.clone(), b);
            order.push(k.clone());
        }
        exps.get_mut(&k).unwrap().push(e);
    }
    if coef.is_zero() && !order.iter().any(|k| bases[k].has_units()) {
        return num(coef);
    }

    let mut result = vec![];
    for k in &order {
        let b = &bases[k];
        let ex = &exps[k];
        let e = if ex.len() == 1 { ex[0].clone() } else { add(ex.clone()) };
        let pw = pow(b, &e);
        match &pw.e {
            E::Num(pn) => coef = coef * pn,
            E::Mul(pm) => {
                for x in pm {
                    if let E::Num(xn) = &x.e {
                        coef = coef * xn;
                    } else {
                        result.push(x.clone());
                    }
                }
            }
            _ => result.push(pw.clone()),
        }
    }

    result = merge_numeric_roots(result, &mut coef);
    if c().is_some() {
        result = merge_unit_factors(result, &mut coef);
    }

    if coef.is_zero() && !result.iter().any(|r| r.has_units()) {
        return num(coef);
    }
    if result.is_empty() {
        return num(coef);
    }
    if !coef.is_one() {
        result.insert(0, num(coef));
    }
    if result.len() == 1 {
        return result.pop().unwrap();
    }
    if result.len() == 2 {
        if let (E::Num(cn), E::Add(terms)) = (&result[0].e, &result[1].e) {
            if c().map(|c| c.distribute_numbers.get()).unwrap_or(false) {
                return add(terms.iter().map(|t| mul2(&num(cn.clone()), t)).collect());
            }
        }
    }
    mul_e(sorted(result))
}

fn sorted(l: Vec<Expr>) -> Vec<Expr> {
    let mut idx: Vec<usize> = (0..l.len()).collect();
    idx.sort_by(|&x, &y| compare_factors(&l[x], &l[y]).then(x.cmp(&y)));
    idx.into_iter().map(|i| l[i].clone()).collect()
}

// 2^(1/2)*3^(1/2) -> 6^(1/2)
fn merge_numeric_roots(fs: Vec<Expr>, coef: &mut Cx) -> Vec<Expr> {
    let mut groups: Vec<(String, Vec<usize>)> = vec![];
    for (i, f) in fs.iter().enumerate() {
        if let E::Pow(b, x) = &f.e {
            if matches!(&b.e, E::Num(n) if n.is_rational()) && matches!(&x.e, E::Num(n) if n.is_rational()) {
                let k = x.key().to_string();
                match groups.iter_mut().find(|g| g.0 == k) {
                    Some(g) => g.1.push(i),
                    None => groups.push((k, vec![i])),
                }
            }
        }
    }
    if !groups.iter().any(|g| g.1.len() > 1) {
        return fs;
    }
    let mut res = vec![];
    let mut used = HashSet::new();
    for (_, g) in groups.iter().filter(|g| g.1.len() > 1) {
        let mut prod = Real::one();
        for &i in g {
            if let E::Pow(b, _) = &fs[i].e {
                prod = prod * &b.num().re;
            }
            used.insert(i);
        }
        let E::Pow(_, x0) = &fs[g[0]].e else { unreachable!() };
        let pw = pow(&num_r(prod), x0);
        match &pw.e {
            E::Num(pn) => *coef = &*coef * pn,
            E::Mul(pm) => {
                for x in pm {
                    if let E::Num(xn) = &x.e {
                        *coef = &*coef * xn;
                    } else {
                        res.push(x.clone());
                    }
                }
            }
            _ => res.push(pw.clone()),
        }
    }
    for (i, f) in fs.iter().enumerate() {
        if !used.contains(&i) {
            res.push(f.clone());
        }
    }
    res
}

// different units of the same dimension in one product are converted to the first one
fn merge_unit_factors(fs: Vec<Expr>, coef: &mut Cx) -> Vec<Expr> {
    let unit_idx: Vec<usize> = (0..fs.len()).filter(|&i| is_unit_factor(&fs[i])).collect();
    if unit_idx.len() < 2 {
        return fs;
    }
    let mut changed = false;
    let mut list: Vec<Option<Expr>> = fs.iter().cloned().map(Some).collect();
    for a in 0..unit_idx.len() {
        let ia = unit_idx[a];
        let Some(la) = list[ia].clone() else { continue };
        let (ba, mut ea) = base_exp(&la);
        let E::Unit(ua, pa) = &ba.e else { continue };
        for &ib in &unit_idx[a + 1..] {
            let Some(lb) = list[ib].clone() else { continue };
            let (bb, eb) = base_exp(&lb);
            let E::Unit(ub, pb) = &bb.e else { continue };
            if !ua.dim.same_as(&ub.dim) || ua.dim.is_none() {
                continue;
            }
            let Some(ebn) = eb.as_real() else { continue };
            let ratio = units::unit_factor(ub, *pb) / units::unit_factor(ua, *pa);
            *coef = &*coef * &Cx::real(rmath::pow(&ratio, ebn));
            ea = add2(&ea, &eb);
            list[ib] = None;
            changed = true;
        }
        if changed {
            list[ia] = Some(pow(&ba, &ea));
        }
    }
    if !changed {
        return fs;
    }
    let mut res = vec![];
    for x in list.into_iter().flatten() {
        if let E::Num(xn) = &x.e {
            *coef = &*coef * xn;
            continue;
        }
        res.push(x);
    }
    res
}

pub fn pow(b: &Expr, e: &Expr) -> Expr {
    if e.is_zero() {
        if let E::Mat(rows, _) = &b.e {
            return matrices::identity(rows.len());
        }
        return one();
    }
    if e.is_one() {
        return b.clone();
    }
    if b.is_one() && !matches!(&b.e, E::Num(n) if n.approx()) {
        return b.clone();
    }
    if let (E::Num(bn), E::Num(en)) = (&b.e, &e.e) {
        return num_pow(bn, en);
    }
    if b.is_zero() && e.as_real().map(|r| r.sign() > 0).unwrap_or(false) {
        return zero();
    }
    if b.is_intv() || e.is_intv() {
        return intervals::pow(b, e);
    }
    if let E::Mat(..) = &b.e {
        if let E::Num(en) = &e.e {
            if en.is_exact_int() {
                return matrices::power(b, &en.re.n);
            }
            throw("Matrix power requires an integer exponent");
        }
        return pow_e(b.clone(), e.clone());
    }
    let cc = cur();
    if let E::Pow(pbb, pbx) = &b.e {
        if let E::Num(en) = &e.e {
            if en.is_exact_int() {
                return pow(pbb, &mul2(pbx, e));
            }
        }
        if let (E::Num(pxn), E::Num(en2)) = (&pbx.e, &e.e) {
            if pxn.is_exact_int() && en2.is_real() {
                let prod = mul2(pbx, e);
                let even_root = pxn.re.n.is_even();
                if prod.is_one() && even_root {
                    if cc.is_known_non_negative(pbb) {
                        return pbb.clone();
                    }
                    if cc.is_known_real(pbb) {
                        return cc.ev.call("abs", vec![pbb.clone()]);
                    }
                }
                if !even_root || cc.is_known_non_negative(pbb) {
                    return pow(pbb, &prod);
                }
            }
        }
        if pbb.is_unit() {
            return pow(pbb, &mul2(pbx, e));
        }
    }
    if let E::Mul(mm) = &b.e {
        if let E::Num(en) = &e.e {
            if en.is_exact_int() {
                return mul(mm.iter().map(|f| pow(f, e)).collect());
            }
        }
        let mut pos = vec![];
        let mut rest = vec![];
        for f in mm {
            if f.as_real().map(|r| r.sign() > 0).unwrap_or(false) || is_unit_factor(f) || cc.is_known_positive(f) {
                pos.push(f.clone());
            } else {
                rest.push(f.clone());
            }
        }
        if !pos.is_empty() {
            let mut parts: Vec<Expr> = pos.iter().map(|f| pow(f, e)).collect();
            if !rest.is_empty() {
                let rb = if rest.len() == 1 { rest.pop().unwrap() } else { mul_e(rest) };
                parts.push(pow_e(rb, e.clone()));
            }
            return mul(parts);
        }
    }
    if b.is_var_named("e") {
        if let E::Call(lf, la) = &e.e {
            if lf.name() == "ln" {
                return la[0].clone();
            }
        }
        if let E::Mul(em) = &e.e {
            if em.len() == 2 && em[1].is_var_named("pi") {
                if let E::Num(ec) = &em[0].e {
                    if ec.re.is_zero() && !ec.im.approx {
                        let ang = mul2(&num_r(ec.im.clone()), &em[1]);
                        let cs = funcs::exact_trig_public("cos", &ang);
                        let sn = funcs::exact_trig_public("sin", &ang);
                        if let (Some(cs), Some(sn)) = (cs, sn) {
                            return add2(&cs, &mul2(&num(Cx::i()), &sn));
                        }
                    }
                }
            }
        }
    }
    if let (E::Num(bb), E::Call(lg, la)) = (&b.e, &e.e) {
        if bb.is_rational() && bb.re.sign() > 0 && lg.name() == "log" && la.len() == 2 && la[1].same(b) {
            return la[0].clone();
        }
    }
    pow_e(b.clone(), e.clone())
}

pub fn num_pow(b: &Cx, e: &Cx) -> Expr {
    if e.is_exact_int() {
        let n = &e.re.n;
        if n.abs() > big(100000000) && !b.approx() && !(b.is_real() && (b.re.is_one() || b.re.is_zero() || b.re.eqv(&Real::minus_one()))) {
            return num(b.as_approx().pow_int(n));
        }
        if b.is_zero() && n.is_negative() {
            throw("Division by zero");
        }
        return num(b.pow_int(n));
    }
    let cc = c();
    let exact = cc.map(|c| c.exact()).unwrap_or(true);
    if exact && !b.approx() && !e.approx() && e.is_real() && b.is_real() {
        if let Some(r) = exact_rational_pow(&b.re, &e.re) {
            return r;
        }
        return pow_e(num(b.clone()), num(e.clone()));
    }
    if exact && !b.approx() && !e.approx() {
        return pow_e(num(b.clone()), num(e.clone()));
    }
    if let Some(cc) = cc {
        let (complex, real_roots) = {
            let o = cc.opts();
            (o.complex_enabled, o.real_roots)
        };
        if b.is_real() && e.is_real() && b.re.sign() < 0 {
            if !complex && !(real_roots && !e.re.d.is_even()) {
                throw("Complex numbers are disabled");
            }
            if real_roots && !e.re.approx && !e.re.d.is_even() {
                let root = rmath::root(&(-&b.re).as_approx(), e.re.d.to_i32().unwrap_or(i32::MAX));
                return num_r((-root).pow_int(&e.re.n));
            }
        }
    }
    num(Cx::pow(b, e).clean())
}

// exact power of a rational to a rational exponent, extracting perfect powers
fn exact_rational_pow(b: &Real, e: &Real) -> Option<Expr> {
    if e.d > big(1000) || e.n.abs() > big(100000) {
        return None;
    }
    let q = e.d.to_u32().unwrap();
    let p = e.n.clone();
    if b.sign() < 0 {
        if q % 2 == 1 && c().map(|c| c.opts().real_roots).unwrap_or(true) {
            let r = exact_rational_pow(&-b, e)?;
            return Some(if p.is_even() { r } else { neg(&r) });
        }
        if q == 2 {
            let r = exact_rational_pow(&-b, e)?;
            let ip = Cx::i().pow_int(&p);
            return Some(mul2(&num(ip), &r));
        }
        return None;
    }
    if let Some(root) = b.exact_root(q) {
        return Some(num_r(root.pow_int(&p)));
    }
    let (mut on, mut rn) = extract_power(&b.n, q);
    let (mut od, rd) = extract_power(&b.d, q);
    if !rd.is_one() {
        let ddd = num_traits::pow(rd.clone(), (q - 1) as usize);
        rn *= ddd;
        od *= &rd;
        let (on2, rn2) = extract_power(&rn, q);
        on *= on2;
        rn = rn2;
    }
    let outer = Real::exact(on, od).pow_int(&p);
    let inner = Real::big_int(rn);
    let qb = big(q as i64);
    let mut whole = &p / &qb;
    let mut rem_p = &p - &whole * &qb;
    if rem_p.is_negative() {
        rem_p += &qb;
        whole -= 1;
    }
    let coefficient = &outer * &inner.pow_int(&whole);
    let g = rem_p.gcd(&qb);
    let ex = Real::exact(&rem_p / &g, &qb / &g);
    if inner.is_one() {
        return Some(num_r(coefficient));
    }
    let root_part = pow_e(num_r(inner), num_r(ex));
    if coefficient.is_one() {
        return Some(root_part);
    }
    Some(mul_e(vec![num_r(coefficient), root_part]))
}

fn extract_power(n: &BigInt, q: u32) -> (BigInt, BigInt) {
    let mut outer = BigInt::one();
    let mut rest = n.clone();
    if rest < big(2) {
        return (outer, rest);
    }
    for &pr in primes::SMALL.iter() {
        let pq = num_traits::pow(big(pr as i64), q as usize);
        if pq > rest {
            break;
        }
        while (&rest % &pq) == BigInt::from(0) {
            rest /= &pq;
            outer *= pr;
        }
    }
    let r = real::iroot(&rest, q);
    if num_traits::pow(r.clone(), q as usize) == rest {
        outer *= r;
        rest = BigInt::one();
    }
    (outer, rest)
}

fn matrix_add(terms: Vec<Expr>) -> Expr {
    let mut acc: Option<Expr> = None;
    let mut scalars = vec![];
    for t in terms {
        if let E::Mat(rows, vec) = &t.e {
            let Some(a) = &acc else {
                acc = Some(t.clone());
                continue;
            };
            let E::Mat(arows, avec) = &a.e else { unreachable!() };
            if rows.len() != arows.len() || rows.first().map(|r| r.len()) != arows.first().map(|r| r.len()) {
                throw("Matrix dimensions do not match for addition");
            }
            let nr: Vec<Vec<Expr>> = (0..rows.len()).map(|i| (0..rows[i].len()).map(|j| add2(&arows[i][j], &rows[i][j])).collect()).collect();
            acc = Some(mat_e(nr, *avec && *vec));
        } else {
            scalars.push(t);
        }
    }
    let mut acc = acc.unwrap();
    if !scalars.is_empty() {
        let s = add(scalars);
        acc = matrices::map_elements(&acc, &mut |x| add2(&x, &s));
    }
    acc
}

fn matrix_mul(fs: Vec<Expr>) -> Expr {
    let mut scalars = vec![];
    let mut acc: Option<Expr> = None;
    for f in fs {
        if f.is_mat() {
            acc = Some(match acc {
                None => f,
                Some(a) => matrices::product(&a, &f),
            });
        } else {
            scalars.push(f);
        }
    }
    let acc = acc.unwrap();
    if scalars.is_empty() {
        return acc;
    }
    let s = mul(scalars);
    if acc.is_mat() {
        return matrices::map_elements(&acc, &mut |x| mul2(&s, &x));
    }
    mul2(&s, &acc)
}

// expands products and integer powers of sums
pub fn expand(e: &Expr) -> Expr {
    match &e.e {
        E::Add(t) => add(t.iter().map(expand).collect()),
        E::Mat(..) | E::Cmp(..) | E::Logic(..) => e.rebuild(e.kids().iter().map(expand).collect()),
        E::Mul(m) => {
            let mut acc = one();
            for f in m.iter().map(expand) {
                acc = mul_expand(&acc, &f);
            }
            acc
        }
        E::Pow(pb, px) => {
            let b = expand(pb);
            if let (true, Some(n)) = (b.is_add(), px.as_num()) {
                if n.is_exact_int() && n.re.n.is_positive() && n.re.n <= big(64) {
                    let k = n.re.n.to_i32().unwrap();
                    let mut acc = one();
                    for _ in 0..k {
                        acc = mul_expand(&acc, &b);
                    }
                    return acc;
                }
                if n.is_exact_int() && n.re.n.is_negative() && n.re.n >= big(-64) {
                    let pos = expand(&pow(&b, &num(-n)));
                    return pow(&pos, &minus_one());
                }
            }
            pow(&b, px)
        }
        E::Call(f, a) => {
            if f.name() == "_hold" {
                e.clone()
            } else {
                call_e(f.clone(), a.iter().map(expand).collect())
            }
        }
        _ => e.clone(),
    }
}

fn mul_expand(a: &Expr, b: &Expr) -> Expr {
    let at = terms_of(a);
    let bt = terms_of(b);
    if at.len() * bt.len() > 20000 {
        throw("Expression too large to expand");
    }
    let mut res = Vec::with_capacity(at.len() * bt.len());
    for x in &at {
        for y in &bt {
            res.push(mul2(x, y));
        }
    }
    add(res)
}

// splits into numerator and denominator
pub fn num_den(e: &Expr) -> (Expr, Expr) {
    let mut n = vec![];
    let mut d = vec![];
    for f in factors_of(e) {
        match &f.e {
            E::Pow(b, x) if x.as_real().map(|r| r.sign() < 0).unwrap_or(false) => d.push(pow(b, &num(-x.num()))),
            E::Num(fv) if fv.is_rational() && !fv.re.d.is_one() => {
                n.push(num_r(Real::big_int(fv.re.n.clone())));
                d.push(num_r(Real::big_int(fv.re.d.clone())));
            }
            _ => n.push(f.clone()),
        }
    }
    (mul(n), mul(d))
}