use super::algebra as simp;
use super::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

pub struct Evaluator {
    depth: Cell<i32>,
    pub bindings: RefCell<HashMap<String, Expr>>,
}

pub fn bool_e(b: bool) -> Expr {
    if b {
        one()
    } else {
        zero()
    }
}

pub fn truth(e: &Expr) -> Option<bool> {
    match &e.e {
        E::Num(n) if n.is_real() => Some(!n.re.is_zero()),
        _ => None,
    }
}

pub fn strip_units(e: &Expr) -> Expr {
    if !e.has_units() {
        return e.clone();
    }
    let fs: Vec<Expr> = match &e.e {
        E::Mul(f) => f.clone(),
        _ => vec![e.clone()],
    };
    simp::mul(fs.into_iter().filter(|f| !simp::is_unit_factor(f)).collect())
}

fn compare(op: CmpOp, s: i32) -> bool {
    match op {
        CmpOp::Eq => s == 0,
        CmpOp::Ne => s != 0,
        CmpOp::Lt => s < 0,
        CmpOp::Gt => s > 0,
        CmpOp::Le => s <= 0,
        CmpOp::Ge => s >= 0,
    }
}

fn magnitude(e: &Expr) -> f64 {
    match &e.e {
        E::Num(n) => n.re.to_f64().abs(),
        _ => 1.0,
    }
}

impl Evaluator {
    pub fn new() -> Evaluator {
        Evaluator { depth: Cell::new(0), bindings: RefCell::new(HashMap::new()) }
    }

    pub fn reset(&self) {
        self.depth.set(0);
        self.bindings.borrow_mut().clear();
    }

    pub fn call(&self, f: &str, args: Vec<Expr>) -> Expr {
        let c = cur();
        self.eval_call(&call_e(c.fn_(f), args), true)
    }

    pub fn eval(&self, e: &Expr) -> Expr {
        let d = self.depth.get() + 1;
        self.depth.set(d);
        if d > 400 {
            self.depth.set(0);
            throw("Expression too deeply nested");
        }
        finally(|| self.eval_inner(e), || self.depth.set((self.depth.get() - 1).max(0)))
    }

    fn eval_inner(&self, e: &Expr) -> Expr {
        let c = cur();
        c.check_abort();
        match &e.e {
            E::Num(n) => {
                if !c.opts().complex_enabled && !n.is_real() {
                    throw("Complex numbers are disabled");
                }
                e.clone()
            }
            E::Sym(s) => self.bindings.borrow().get(s).cloned().unwrap_or_else(|| e.clone()),
            E::Var(_) => self.eval_var(e),
            E::Unit(..) => e.clone(),
            E::Add(t) => simp::add(t.iter().map(|x| self.eval(x)).collect()),
            E::Mul(f) => simp::mul(f.iter().map(|x| self.eval(x)).collect()),
            E::Pow(b, x) => simp::pow(&self.eval(b), &self.eval(x)),
            E::Call(..) => self.eval_call(e, false),
            E::Mat(..) => matrices::map_elements(e, &mut |x| self.eval(&x)),
            E::Cmp(op, l, r) => self.eval_cmp(&cmp_e(*op, self.eval(l), self.eval(r))),
            E::Logic(..) => self.eval_logic(e),
            E::Arg(_) => throw("Argument placeholder outside of function"),
            _ => e.clone(),
        }
    }

    fn eval_var(&self, e: &Expr) -> Expr {
        let c = cur();
        let E::Var(var) = &e.e else { unreachable!() };
        if var.keep_symbolic && c.exact() {
            return e.clone();
        }
        if let Some(d) = var.dynamic {
            return d(c);
        }
        let value = match var.value.get() {
            Some(v) => Some(v.clone()),
            None if !var.value_text.is_empty() => {
                let p = parser::parse_text(&var.value_text, c);
                if let Some(p) = &p {
                    let _ = var.value.set(p.clone());
                }
                p
            }
            None => None,
        };
        let Some(value) = value else { return e.clone() };
        let mut val = self.eval(&value);
        if var.is_constant && var.has_unit && !c.opts().units_in_constants {
            val = strip_units(&val);
        }
        if var.is_approximate && !c.exact() {
            if let E::Num(vn) = &val.e {
                if !vn.approx() {
                    val = num(vn.as_approx());
                }
            }
        }
        val
    }

    pub fn eval_call(&self, ce: &Expr, args_evaluated: bool) -> Expr {
        let c = cur();
        let E::Call(f, cargs) = &ce.e else { unreachable!() };
        let mut args: Vec<Expr> = Vec::with_capacity(cargs.len());
        for (i, a) in cargs.iter().enumerate() {
            if args_evaluated || f.holds(i) {
                args.push(a.clone());
            } else {
                args.push(self.eval(a));
            }
        }
        let lim = if f.max_args < 0 { f.args.len() } else { (f.max_args as usize).min(f.args.len()) };
        let mut i = args.len();
        while i < lim {
            let Some(def) = &f.args[i].default else { break };
            let d = parser::parse_req(def, c);
            args.push(if f.holds(i) { d } else { self.eval(&d) });
            i += 1;
        }
        if (args.len() as i32) < f.min_args {
            throw_s(format!("{}() requires at least {} argument{}", f.item.names[0], f.min_args, if f.min_args == 1 { "" } else { "s" }));
        }
        if f.max_args >= 0 && args.len() as i32 > f.max_args {
            throw_s(format!("Too many arguments for {}()", f.item.names[0]));
        }

        if f.elementwise && !args.is_empty() {
            if let E::Mat(..) = &args[0].e {
                let m0 = args[0].clone();
                return matrices::map_elements(&m0, &mut |x| {
                    let mut a2 = args.clone();
                    a2[0] = x;
                    self.eval_call(&call_e(f.clone(), a2), true)
                });
            }
            if let (E::Intv(lo, hi), Some(imp)) = (&args[0].e, f.imp.clone()) {
                let (lo, hi) = (lo.clone(), hi.clone());
                return intervals::apply(&lo, &hi, &mut |r| {
                    let mut a2 = args.clone();
                    a2[0] = num_r(r.as_approx());
                    c.with_exact(false, || {
                        let res = imp(c, &a2);
                        match res.as_ref().and_then(|x| x.as_real()) {
                            Some(v) => v.clone(),
                            None => throw_s(format!("Unsupported interval argument for {}()", f.name())),
                        }
                    })
                });
            }
        }
        self.check_args(f, &args);

        if f.is_user_formula() {
            let body = match f.parsed.get() {
                Some(p) => p.clone(),
                None => {
                    let p = parser::parse_req(&f.formula, c);
                    let _ = f.parsed.set(p.clone());
                    p
                }
            };
            let body = body.map(&mut |x| match &x.e {
                E::Arg(i) => {
                    if *i < args.len() {
                        args[*i].clone()
                    } else {
                        throw_s(format!("Missing argument for {}()", f.name()))
                    }
                }
                _ => x,
            });
            return self.eval(&body);
        }
        let imp = f.imp.clone().unwrap();
        match imp(c, &args) {
            Some(r) => r,
            None => call_e(f.clone(), args),
        }
    }

    fn check_args(&self, f: &MathFunction, args: &[Expr]) {
        for (i, a) in args.iter().enumerate().take(f.args.len()) {
            let t = f.args[i].ty;
            if f.holds(i) {
                continue;
            }
            let symbolic = a.has_symbols() || a.is_call() || matches!(a.e, E::Var(_)) || matches!(&a.e, E::Pow(b, _) if !b.is_unit());
            let fname = &f.item.names[0];
            match t {
                ArgType::Integer | ArgType::NonNegInt | ArgType::PositiveInt => {
                    if let E::Num(n) = &a.e {
                        if !n.is_int() {
                            throw_s(format!("Argument {} ({}) of {}() must be an integer", i + 1, f.args[i].name, fname));
                        }
                        if t == ArgType::NonNegInt && n.re.sign() < 0 {
                            throw_s(format!("Argument {} of {}() must be non-negative", i + 1, fname));
                        }
                        if t == ArgType::PositiveInt && n.re.sign() <= 0 {
                            throw_s(format!("Argument {} of {}() must be positive", i + 1, fname));
                        }
                    }
                }
                ArgType::Real => {
                    if let E::Num(n) = &a.e {
                        if !n.is_real() {
                            throw_s(format!("Argument {} of {}() must be real", i + 1, fname));
                        }
                    }
                }
                ArgType::Matrix | ArgType::Vector => {
                    if !a.is_mat() && !symbolic && !a.is_num() {
                        throw_s(format!("Argument {} of {}() must be a {}", i + 1, fname, if t == ArgType::Matrix { "matrix" } else { "vector" }));
                    }
                }
                _ => {}
            }
        }
    }

    pub fn eval_cmp(&self, e: &Expr) -> Expr {
        let c = cur();
        let E::Cmp(op, l, r) = &e.e else { unreachable!() };
        let op = *op;
        if let (E::Str(ls), E::Str(rs)) = (&l.e, &r.e) {
            let eq = ls == rs;
            if op == CmpOp::Eq {
                return bool_e(eq);
            }
            if op == CmpOp::Ne {
                return bool_e(!eq);
            }
        }
        if let (E::Date(lt, ..), E::Date(rt, ..)) = (&l.e, &r.e) {
            return bool_e(compare(op, lt.cmp(rt) as i32));
        }
        if l.is_mat() && r.is_mat() && (op == CmpOp::Eq || op == CmpOp::Ne) {
            let eq = l.key() == r.key();
            return bool_e(if op == CmpOp::Eq { eq } else { !eq });
        }
        let diff = simp::sub(l, r);
        if let E::Num(dn) = &diff.e {
            if !dn.is_real() {
                if op == CmpOp::Eq {
                    return bool_e(false);
                }
                if op == CmpOp::Ne {
                    return bool_e(true);
                }
                throw("Complex numbers can not be ordered");
            }
            let mut s = dn.re.sign();
            if dn.approx() && dn.re.to_f64().abs() < 1e-30 * 1f64.max(magnitude(l).abs()) {
                s = 0;
            }
            return bool_e(compare(op, s));
        }
        if let E::Intv(lo, hi) = &diff.e {
            if lo.sign() > 0 {
                return bool_e(compare(op, 1));
            }
            if hi.sign() < 0 {
                return bool_e(compare(op, -1));
            }
        }
        if !c.exact() && l.has_units() {
            if let Some((qv, _, qr)) = units::try_quantity(&diff) {
                if qr.is_one() && qv.is_real() {
                    return bool_e(compare(op, qv.re.sign()));
                }
            }
        }
        if !diff.has_symbols() && c.exact() {
            return e.clone();
        }
        if l.has_units() && r.has_units() {
            if let (Some((lv, ld, lr)), Some((rv, rd, rr))) = (units::try_quantity(l), units::try_quantity(r)) {
                if lr.is_one() && rr.is_one() && ld.same_as(&rd) && lv.is_real() && rv.is_real() {
                    return bool_e(compare(op, lv.re.cmp(&rv.re) as i32));
                }
            }
        }
        e.clone()
    }

    fn eval_logic(&self, le: &Expr) -> Expr {
        let E::Logic(op, items) = &le.e else { unreachable!() };
        let items: Vec<Expr> = items.iter().map(|x| self.eval(x)).collect();
        match op {
            LogicOp::Not => match truth(&items[0]) {
                Some(t) => bool_e(!t),
                None => logic_e(LogicOp::Not, items),
            },
            LogicOp::And => {
                let mut rest = vec![];
                for it in items {
                    match truth(&it) {
                        Some(false) => return zero(),
                        None => rest.push(it),
                        _ => {}
                    }
                }
                if rest.is_empty() {
                    return one();
                }
                if rest.len() == 1 {
                    rest.pop().unwrap()
                } else {
                    logic_e(LogicOp::And, rest)
                }
            }
            LogicOp::Or => {
                let mut rest = vec![];
                for it in items {
                    match truth(&it) {
                        Some(true) => return one(),
                        None => rest.push(it),
                        _ => {}
                    }
                }
                if rest.is_empty() {
                    return zero();
                }
                if rest.len() == 1 {
                    rest.pop().unwrap()
                } else {
                    logic_e(LogicOp::Or, rest)
                }
            }
            LogicOp::Xor => {
                let ts: Vec<Option<bool>> = items.iter().map(truth).collect();
                if ts.iter().all(|x| x.is_some()) {
                    return bool_e(ts.iter().filter(|x| x.unwrap()).count() % 2 == 1);
                }
                logic_e(LogicOp::Xor, items)
            }
        }
    }

    // evaluates e with symbol bound to value, numerically
    pub fn eval_with(&self, e: &Expr, symbol: &str, value: Expr) -> Expr {
        let old = self.bindings.borrow_mut().insert(symbol.to_string(), value);
        finally(
            || self.eval(e),
            || {
                let mut b = self.bindings.borrow_mut();
                match old {
                    Some(o) => {
                        b.insert(symbol.to_string(), o);
                    }
                    None => {
                        b.remove(symbol);
                    }
                }
            },
        )
    }

    pub fn eval_double(&self, e: &Expr, symbol: &str, x: f64) -> f64 {
        let c = cur();
        c.with_exact(false, || {
            attempt(|| {
                let mut r = self.eval_with(e, symbol, num_r(Real::from_f64(x)));
                if let Some((v, _, rest)) = units::try_quantity(&r) {
                    if rest.is_one() {
                        r = num(v);
                    }
                }
                if let E::Num(n) = &r.e {
                    if n.is_real() {
                        return n.re.to_f64();
                    }
                    if n.im.to_f64().abs() < 1e-12 * 1f64.max(n.re.to_f64().abs()) {
                        return n.re.to_f64();
                    }
                }
                f64::NAN
            })
            .unwrap_or(f64::NAN)
        })
    }

    pub fn eval_complex(&self, e: &Expr, symbol: &str, x: f64) -> (f64, f64) {
        let c = cur();
        c.with_exact(false, || {
            attempt(|| {
                let r = self.eval_with(e, symbol, num_r(Real::from_f64(x)));
                if let E::Num(n) = &r.e {
                    return n.to_c64();
                }
                (f64::NAN, 0.0)
            })
            .unwrap_or((f64::NAN, 0.0))
        })
    }
}