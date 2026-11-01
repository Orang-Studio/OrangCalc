use super::algebra::is_unit_factor;
use super::calculator::try_cur;
use super::real::{big, bit_len};
use super::*;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokType {
    Plain,
    Number,
    Operator,
    Variable,
    Function,
    Unit,
    Symbol,
    Paren,
    Text,
    Error,
}

#[derive(Clone, Debug)]
pub struct OutTok {
    pub text: String,
    pub ty: TokType,
}

pub struct Formatter {
    o: CalcOptions,
    toks: Vec<OutTok>,
    pub rounded: bool,
}

fn comma_decimal() -> bool {
    try_cur().map(|c| c.opts().comma_decimal).unwrap_or(false)
}
fn i_name() -> &'static str {
    if try_cur().map(|c| c.opts().use_j).unwrap_or(false) {
        "j"
    } else {
        "i"
    }
}

pub fn plain_text(e: &Expr) -> String {
    let o = try_cur().map(|c| c.opts().clone()).unwrap_or_default();
    let mut f = Formatter::new(o);
    f.write_term(e, 0);
    f.text()
}

pub fn format(e: &Expr, o: &CalcOptions) -> (String, bool) {
    let mut f = Formatter::new(o.clone());
    f.write_term(e, 0);
    (f.text(), f.rounded)
}

pub fn tokens(e: &Expr, o: &CalcOptions) -> (Vec<OutTok>, bool) {
    let mut f = Formatter::new(o.clone());
    f.write_term(e, 0);
    (f.toks, f.rounded)
}

// ascii friendly text for copying
pub fn ascii(e: &Expr, o: &CalcOptions) -> String {
    let mut po = o.clone();
    po.unicode = false;
    po.mult = MultSign::Asterisk;
    po.grouping = DigitGrouping::None;
    po.repeating = RepeatingMode::None;
    po.implicit_mult_output = false;
    format(e, &po).0
}

fn pure_units(e: &Expr) -> bool {
    if is_unit_factor(e) {
        return true;
    }
    matches!(&e.e, E::Mul(f) if f.iter().all(is_unit_factor))
}

fn prec(e: &Expr) -> i32 {
    match &e.e {
        E::Logic(..) => 1,
        E::Cmp(..) => 2,
        E::Add(_) => 3,
        E::Mul(_) => 4,
        E::Num(n) => {
            if n.is_real() {
                if n.re.sign() < 0 {
                    4
                } else {
                    10
                }
            } else if n.re.is_zero() && n.im.sign() > 0 {
                10
            } else {
                3
            }
        }
        E::Intv(..) => 3,
        E::Pow(..) => 6,
        _ => 10,
    }
}

pub fn format_date(t: &chrono::NaiveDateTime, has_time: bool, utc: bool) -> String {
    use chrono::Timelike;
    if !has_time {
        return t.format("%Y-%m-%d").to_string();
    }
    let ms = t.nanosecond() / 1_000_000;
    let mut s = if t.second() == 0 && ms == 0 { t.format("%Y-%m-%dT%H:%M").to_string() } else { t.format("%Y-%m-%dT%H:%M:%S").to_string() };
    if ms != 0 {
        s += &format!(".{:03}", ms);
    }
    if utc {
        s.push('Z');
    }
    s
}

fn is_negative(t: &Expr) -> bool {
    let neg_cx = |v: &Cx| if v.is_real() { v.re.sign() < 0 } else { v.re.is_zero() && v.im.sign() < 0 };
    match &t.e {
        E::Num(n) => neg_cx(n),
        E::Mul(f) if !f.is_empty() => matches!(&f[0].e, E::Num(n) if neg_cx(n)),
        E::Intv(_, hi) => hi.sign() < 0,
        _ => false,
    }
}

fn negate(t: &Expr) -> Expr {
    match &t.e {
        E::Num(n) => num(-n),
        E::Intv(lo, hi) => intv_e(-hi, -lo),
        E::Mul(f) => {
            let mut l = f.clone();
            let nv = -l[0].num();
            if nv.is_one() {
                l.remove(0);
            } else {
                l[0] = num(nv);
            }
            if l.len() == 1 {
                l.pop().unwrap()
            } else {
                mul_e(l)
            }
        }
        _ => t.clone(),
    }
}

const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

pub fn superscript(n: &BigInt) -> String {
    n.to_string().chars().map(|ch| if ch == '-' { '⁻' } else { SUP[ch as usize - '0' as usize] }).collect()
}

fn sig_digits_for(lo: &Real, hi: &Real) -> i32 {
    let m = ((lo + hi) / Real::two()).to_f64().abs();
    let r = ((hi - lo) / Real::two()).to_f64().abs();
    if r == 0.0 || m == 0.0 {
        return 10;
    }
    let d = m.log10().floor() as i32 - r.log10().floor() as i32 + 2;
    d.clamp(1, 30)
}

pub fn terminates(r: &Real) -> bool {
    let mut d = r.d.clone();
    while d.is_even() && !d.is_zero() {
        d /= 2;
    }
    while (&d % 5u32).is_zero() && !d.is_zero() {
        d /= 5;
    }
    d.is_one()
}

pub fn has_non_terminating(e: &Expr) -> bool {
    e.any_of(&mut |x| matches!(&x.e, E::Num(n) if n.is_rational() && !n.re.is_int() && !terminates(&n.re)))
}

// floor(log10(a)) for a > 0
pub fn exp10(a: &Real) -> i32 {
    let d = a.to_f64();
    let mut e = if d > 1e-300 && d.is_finite() { d.log10().floor() as i32 } else { ((bit_len(&a.n) - bit_len(&a.d)) as f64 * 0.30102999566398).floor() as i32 };
    let ten = Real::int(10);
    for _ in 0..4 {
        let lo = ten.pow_i(e as i64);
        if a.lt(&lo) {
            e -= 1;
            continue;
        }
        if a.ge(&(&lo * &ten)) {
            e += 1;
            continue;
        }
        break;
    }
    e
}

// detects periodic decimal expansion: (repeating, non-repeating, integer part)
fn repeating_decimal(a: &Real, max_len: i32) -> Option<(String, String, String)> {
    let d = &a.d;
    let mut d2 = d.clone();
    let (mut twos, mut fives) = (0, 0);
    while d2.is_even() {
        d2 /= 2;
        twos += 1;
    }
    while (&d2 % 5u32).is_zero() {
        d2 /= 5;
        fives += 1;
    }
    if d2.is_one() {
        return None;
    }
    let pre = twos.max(fives);
    let (ip, mut rem) = a.n.div_rem(d);
    let mut non_rep = String::new();
    for _ in 0..pre {
        rem *= 10;
        non_rep.push((b'0' + (&rem / d).to_u8().unwrap()) as char);
        rem %= d;
    }
    let start = rem.clone();
    let mut rep = String::new();
    for _ in 0..max_len.max(30) * 2 {
        rem *= 10;
        rep.push((b'0' + (&rem / d).to_u8().unwrap()) as char);
        rem %= d;
        if rem == start {
            return Some((rep, non_rep, ip.to_string()));
        }
    }
    None
}

fn digit_chars(lower: bool, duo: bool) -> Vec<char> {
    if duo {
        return "0123456789↊↋".chars().collect();
    }
    let s = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if lower {
        s.to_lowercase().chars().collect()
    } else {
        s.chars().collect()
    }
}

fn to_digits(v: &BigInt, b: u32, digits: &[char]) -> String {
    if v.is_zero() {
        return "0".into();
    }
    v.to_str_radix(b).chars().map(|c| digits[c.to_digit(36).unwrap() as usize]).collect()
}

fn bijective(n: &BigInt) -> String {
    let mut n = n.clone();
    let mut s = vec![];
    while n.is_positive() {
        n -= 1;
        s.push((b'A' + (&n % 26u32).to_u8().unwrap()) as char);
        n /= 26;
    }
    s.iter().rev().collect()
}

fn bcd(n: &BigInt) -> String {
    let s = n.abs().to_string();
    let parts: Vec<String> = s.chars().map(|ch| format!("{:04b}", ch as u32 - '0' as u32)).collect();
    format!("{}{}", if n.is_negative() { "−" } else { "" }, parts.join(" "))
}

pub fn all_bases_text(e: &Expr, print: &CalcOptions) -> Expr {
    if !e.is_real_num() {
        throw("Only real numbers can be shown in all bases");
    }
    let mut s = String::new();
    for (b, name) in [(2, "Binary"), (8, "Octal"), (10, "Decimal"), (16, "Hexadecimal")] {
        let mut po = print.clone();
        po.output_base = b;
        s += &format!("{}: {}", name, format(e, &po).0);
        if b != 16 {
            s += "    ";
        }
    }
    str_e(&s)
}

impl Formatter {
    pub fn new(o: CalcOptions) -> Formatter {
        Formatter { o, toks: vec![], rounded: false }
    }

    pub fn text(&self) -> String {
        self.toks.iter().map(|t| t.text.as_str()).collect()
    }

    fn emit(&mut self, s: &str, t: TokType) {
        if !s.is_empty() {
            self.toks.push(OutTok { text: s.to_string(), ty: t });
        }
    }

    fn minus(&self) -> &'static str {
        if self.o.unicode {
            "−"
        } else {
            "-"
        }
    }

    fn times(&self) -> &'static str {
        if !self.o.unicode {
            return "*";
        }
        match self.o.mult {
            MultSign::Dot => "⋅",
            MultSign::MiddleDot => "·",
            MultSign::Asterisk => "*",
            MultSign::Times => "×",
        }
    }

    fn div_sign(&self) -> &'static str {
        if self.o.unicode && self.o.unicode_division {
            "∕"
        } else if self.o.unicode && self.o.div == DivSign::Division {
            "÷"
        } else {
            "/"
        }
    }

    fn dec_point(&self) -> &'static str {
        if comma_decimal() {
            ","
        } else {
            "."
        }
    }

    // writes an operand that stands on its own, so bare units get a leading 1
    pub fn write_term(&mut self, e: &Expr, ctx: i32) {
        if pure_units(e) {
            self.emit("1", TokType::Number);
            self.emit(" ", TokType::Plain);
        }
        self.write(e, ctx);
    }

    fn paren(&mut self, e: &Expr, p: bool) {
        if p {
            self.emit("(", TokType::Paren);
        }
        self.write(e, 0);
        if p {
            self.emit(")", TokType::Paren);
        }
    }

    pub fn write(&mut self, e: &Expr, ctx: i32) {
        if prec(e) < ctx {
            self.paren(e, true);
            return;
        }
        match &e.e {
            E::Num(n) => self.write_cx(n),
            E::Sym(s) => self.emit(s, TokType::Symbol),
            E::Var(v) => {
                let n = self.var_name(v);
                self.emit(&n, TokType::Variable)
            }
            E::Unit(u, p) => {
                let n = self.unit_name(u, *p, true);
                self.emit(&n, TokType::Unit)
            }
            E::Add(t) => self.write_add(t),
            E::Mul(f) => self.write_mul(f),
            E::Pow(b, x) => self.write_pow(b, x),
            E::Call(f, a) => self.write_call(f, a),
            E::Mat(rows, v) => self.write_matrix(rows, *v),
            E::Cmp(op, l, r) => self.write_cmp(*op, l, r),
            E::Logic(op, items) => self.write_logic(*op, items),
            E::Str(s) => self.emit(s, TokType::Text),
            E::Date(t, h, u) => self.emit(&format_date(t, *h, *u), TokType::Number),
            E::Intv(lo, hi) => self.write_interval(lo, hi),
            E::Arg(i) => {
                let c = if *i < 3 { (b'x' + *i as u8) as char } else { (b'a' + *i as u8 - 3) as char };
                self.emit(&format!("\\{}", c), TokType::Symbol)
            }
        }
    }

    fn var_name(&self, v: &Variable) -> String {
        if self.o.unicode {
            match v.name() {
                "pi" => return "π".into(),
                "euler" => return "γ".into(),
                "golden" => return "φ".into(),
                "infinity" => return "∞".into(),
                _ => {}
            }
        }
        v.item.names.iter().find(|n| self.o.unicode || n.is_ascii()).cloned().unwrap_or_else(|| v.name().to_string())
    }

    pub fn unit_name(&self, u: &Unit, p: Option<PrefixRef>, singular: bool) -> String {
        if self.o.abbreviate_names {
            let mut ab = u.abbrev().to_string();
            if !self.o.unicode {
                if ab == "Ω" {
                    ab = "ohm".into();
                } else if ab.starts_with('°') && ab.chars().count() > 1 {
                    ab = format!("o{}", &ab['°'.len_utf8()..]);
                } else if ab == "°" {
                    ab = "deg".into();
                }
            }
            let ps = match p {
                None => "",
                Some(p) if p.symbol == "µ" && !self.o.unicode => "u",
                Some(p) => p.symbol,
            };
            return format!("{}{}", ps, ab);
        }
        let full = if singular { u.item.names.get(1).cloned().unwrap_or_else(|| u.name().to_string()) } else { u.plural.clone() };
        format!("{}{}", p.map(|p| p.name).unwrap_or(""), full)
    }

    fn write_add(&mut self, terms: &[Expr]) {
        for (i, t) in terms.iter().enumerate() {
            let neg = is_negative(t);
            if i > 0 {
                let s = if neg { format!(" {} ", self.minus()) } else { " + ".into() };
                self.emit(&s, TokType::Operator);
            } else if neg {
                let m = self.minus();
                self.emit(m, TokType::Operator);
            }
            let tt = if neg { negate(t) } else { t.clone() };
            self.write_term(&tt, 4);
        }
    }

    fn write_mul(&mut self, factors: &[Expr]) {
        let mut nums: Vec<Expr> = vec![];
        let mut den: Vec<Expr> = vec![];
        let has_symbolic = factors.iter().any(|q| !q.is_num() && !is_unit_factor(q) && !q.is_mat());
        for f in factors {
            match &f.e {
                E::Pow(b, x) if x.as_real().map(|r| r.sign() < 0).unwrap_or(false) => {
                    let nx = -x.num();
                    den.push(if nx.is_one() { b.clone() } else { pow_e(b.clone(), num(nx)) });
                }
                E::Num(fv) if fv.is_real() && !fv.re.approx && !fv.re.d.is_one() && has_symbolic => {
                    if !fv.re.n.is_one() || nums.is_empty() {
                        nums.push(num_r(Real::big_int(fv.re.n.clone())));
                    }
                    den.insert(0, num_r(Real::big_int(fv.re.d.clone())));
                }
                _ => nums.push(f.clone()),
            }
        }
        if nums.len() > 1 && nums[0].is_one() {
            nums.remove(0);
        }
        if nums.len() > 1 && nums[0].is_minus_one() {
            let m = self.minus();
            self.emit(m, TokType::Operator);
            nums.remove(0);
        }
        if nums.is_empty() {
            nums.push(one());
        }
        self.write_factors(&nums);
        if !den.is_empty() {
            let d = self.div_sign();
            self.emit(d, TokType::Operator);
            if den.len() == 1 {
                self.write(&den[0], 6);
            } else {
                self.emit("(", TokType::Paren);
                self.write_factors(&den);
                self.emit(")", TokType::Paren);
            }
        }
    }

    fn write_factors(&mut self, fs: &[Expr]) {
        let imo = self.o.implicit_mult_output;
        for (i, f) in fs.iter().enumerate() {
            if i > 0 {
                let prev = &fs[i - 1];
                let prev_num = prev.is_real_num();
                let f_add_like = f.is_add() || matches!(&f.e, E::Pow(b, _) if b.is_add());
                let implicit_after_num = matches!(&f.e, E::Sym(_) | E::Var(_) | E::Call(..) | E::Add(_))
                    || matches!(&f.e, E::Pow(b, _) if !b.is_num())
                    || matches!(&f.e, E::Pow(b, x) if b.is_num() && x.as_real().map(|r| r.eqv(&Real::half())).unwrap_or(false));
                if is_unit_factor(f) && is_unit_factor(prev) {
                    self.emit(if self.o.unicode { "·" } else { "*" }, TokType::Operator);
                } else if imo && f_add_like && !is_unit_factor(prev) {
                } else if is_unit_factor(f) {
                    self.emit(" ", TokType::Plain);
                } else if imo && prev_num && implicit_after_num {
                } else if imo && matches!(&prev.e, E::Num(pc) if !pc.is_real()) && matches!(&f.e, E::Sym(_) | E::Var(_)) {
                } else {
                    let t = format!(" {} ", self.times());
                    self.emit(&t, TokType::Operator);
                }
            }
            if let E::Num(fnum) = &f.e {
                if fs.len() > 1 && i == 0 && !fnum.is_real() {
                    self.write(f, 10);
                    continue;
                }
            }
            self.write(f, if i == 0 && f.is_num() { 4 } else { 5 });
        }
    }

    fn write_pow(&mut self, b: &Expr, x: &Expr) {
        if let E::Num(xv) = &x.e {
            if xv.is_real() && !xv.re.approx {
                let r = &xv.re;
                let is_half = r.eqv(&Real::half());
                if is_half || r.eqv(&Real::frac(1, 3)) {
                    let name = if self.o.unicode {
                        if is_half {
                            "√"
                        } else {
                            "∛"
                        }
                    } else if is_half {
                        "sqrt"
                    } else {
                        "cbrt"
                    };
                    self.emit(name, TokType::Function);
                    let simple = matches!(&b.e, E::Num(bn) if bn.is_real() && bn.re.sign() >= 0) || b.is_sym() || matches!(b.e, E::Var(_));
                    let paren = !simple || !self.o.unicode;
                    self.paren(b, paren);
                    return;
                }
                if r.sign() < 0 {
                    self.emit("1", TokType::Number);
                    let d = self.div_sign();
                    self.emit(d, TokType::Operator);
                    let inner = if r.eqv(&Real::minus_one()) { b.clone() } else { pow_e(b.clone(), num_r(-r)) };
                    self.write(&inner, 6);
                    return;
                }
                if r.is_int() && self.o.unicode && r.n.abs() < big(1000000) {
                    self.write(b, 7);
                    self.emit(&superscript(&r.n), TokType::Number);
                    return;
                }
            }
        }
        self.write(b, 7);
        self.emit("^", TokType::Operator);
        self.write(x, 7);
    }

    fn write_call(&mut self, f: &MathFunction, args: &[Expr]) {
        if f.name() == "_hold" && args.len() == 1 {
            self.write(&args[0], 0);
            return;
        }
        if f.name() == "abs" && args.len() == 1 && self.o.unicode {
            self.emit("|", TokType::Paren);
            self.write(&args[0], 0);
            self.emit("|", TokType::Paren);
            return;
        }
        if f.name() == "factorial" && args.len() == 1 {
            self.write(&args[0], 9);
            self.emit("!", TokType::Operator);
            return;
        }
        let n = &f.item.names[0];
        let n = n.strip_prefix('_').unwrap_or(n).to_string();
        self.emit(&n, TokType::Function);
        self.emit("(", TokType::Paren);
        let sep = if comma_decimal() { "; " } else { ", " };
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                self.emit(sep, TokType::Operator);
            }
            self.write_term(a, 0);
        }
        self.emit(")", TokType::Paren);
    }

    fn write_matrix(&mut self, rows: &[Vec<Expr>], vector: bool) {
        let sep = if comma_decimal() { "; " } else { ", " };
        let write_row = |me: &mut Formatter, row: &[Expr]| {
            me.emit("[", TokType::Paren);
            for (i, x) in row.iter().enumerate() {
                if i > 0 {
                    me.emit(sep, TokType::Operator);
                }
                me.write_term(x, 0);
            }
            me.emit("]", TokType::Paren);
        };
        if vector {
            write_row(self, &rows[0]);
            return;
        }
        self.emit("[", TokType::Paren);
        for (r, row) in rows.iter().enumerate() {
            if r > 0 {
                self.emit(sep, TokType::Operator);
            }
            write_row(self, row);
        }
        self.emit("]", TokType::Paren);
    }

    fn write_cmp(&mut self, op: CmpOp, l: &Expr, r: &Expr) {
        self.write_term(l, 3);
        let u = self.o.unicode;
        let s = match op {
            CmpOp::Eq => " = ",
            CmpOp::Ne => {
                if u {
                    " ≠ "
                } else {
                    " != "
                }
            }
            CmpOp::Lt => " < ",
            CmpOp::Gt => " > ",
            CmpOp::Le => {
                if u {
                    " ≤ "
                } else {
                    " <= "
                }
            }
            CmpOp::Ge => {
                if u {
                    " ≥ "
                } else {
                    " >= "
                }
            }
        };
        self.emit(s, TokType::Operator);
        self.write_term(r, 3);
    }

    fn write_logic(&mut self, op: LogicOp, items: &[Expr]) {
        let spell = self.o.spell_out_logical;
        if op == LogicOp::Not {
            self.emit(if spell { "not " } else { "!" }, TokType::Operator);
            self.write(&items[0], 9);
            return;
        }
        let s = match op {
            LogicOp::And => {
                if spell {
                    " and "
                } else {
                    " && "
                }
            }
            LogicOp::Or => {
                if spell {
                    " or "
                } else {
                    " || "
                }
            }
            _ => " xor ",
        };
        for (i, it) in items.iter().enumerate() {
            if i > 0 {
                self.emit(s, TokType::Operator);
            }
            self.write(it, 2);
        }
    }

    fn write_interval(&mut self, lo: &Real, hi: &Real) {
        let mut mode = self.o.interval_out;
        if mode == IntervalDisplay::Adaptive {
            mode = IntervalDisplay::PlusMinus;
        }
        let mid = (lo + hi) / Real::two();
        let rad = (hi - lo) / Real::two();
        match mode {
            IntervalDisplay::Interval => {
                self.emit("interval", TokType::Function);
                self.emit("(", TokType::Paren);
                self.write_real(lo, -1);
                self.emit(", ", TokType::Operator);
                self.write_real(hi, -1);
                self.emit(")", TokType::Paren);
            }
            IntervalDisplay::Midpoint => {
                self.write_real(&mid, -1);
                self.rounded = true;
            }
            IntervalDisplay::Lower => {
                self.write_real(lo, -1);
                self.rounded = true;
            }
            IntervalDisplay::Upper => {
                self.write_real(hi, -1);
                self.rounded = true;
            }
            IntervalDisplay::Relative => {
                self.write_real(&mid, -1);
                self.emit("±", TokType::Operator);
                if !mid.is_zero() {
                    self.write_real(&(&rad / &mid * Real::int(100)).abs(), -1);
                    self.emit("%", TokType::Unit);
                } else {
                    self.write_real(&rad, -1);
                }
            }
            IntervalDisplay::Concise => {
                let p = self.o.precision;
                let s = self.format_real(&mid, p);
                let decs = s.find('.').map(|d| s.len() - d - 1).unwrap_or(0);
                let unit = Real::int(10).pow_i(-(decs as i64));
                let err = (&rad / &unit).ceil();
                self.emit(&format!("{}({})", s, err.to_big()), TokType::Number);
            }
            IntervalDisplay::SignificantDigits => {
                self.write_real(&mid, sig_digits_for(lo, hi));
                self.rounded = true;
            }
            _ => {
                self.write_real(&mid, sig_digits_for(lo, hi));
                self.emit(if self.o.unicode { "±" } else { "+/-" }, TokType::Operator);
                self.write_real(&rad, 2);
            }
        }
    }

    fn write_real(&mut self, r: &Real, sig: i32) {
        let s = self.format_real(r, sig);
        self.emit(&s, TokType::Number);
    }

    pub fn write_cx(&mut self, v: &Cx) {
        if v.is_real() {
            self.write_real_value(&v.re);
            return;
        }
        if matches!(self.o.complex, ComplexForm::Exponential | ComplexForm::Polar | ComplexForm::Cis | ComplexForm::Angle) {
            self.write_polar(v);
            return;
        }
        let iname = i_name();
        if !v.re.is_zero() {
            self.write_real_value(&v.re);
            let s = if v.im.sign() < 0 { format!(" {} ", self.minus()) } else { " + ".into() };
            self.emit(&s, TokType::Operator);
        } else if v.im.sign() < 0 {
            let m = self.minus();
            self.emit(m, TokType::Operator);
        }
        let im = v.im.abs();
        if !im.is_one() || im.approx {
            self.write_real_value(&im);
        }
        self.emit(iname, TokType::Variable);
    }

    fn write_polar(&mut self, v: &Cx) {
        let c = try_cur();
        let save = c.map(|c| c.exact()).unwrap_or(true);
        if let Some(c) = c {
            c.set_exact(false);
        }
        let r = v.abs();
        let th = v.arg();
        let iname = i_name();
        match self.o.complex {
            ComplexForm::Exponential => {
                if !r.is_one() {
                    self.write_real_value(&r);
                }
                self.emit("e", TokType::Variable);
                self.emit("^", TokType::Operator);
                self.emit("(", TokType::Paren);
                self.write_real_value(&th);
                self.emit(iname, TokType::Variable);
                self.emit(")", TokType::Paren);
            }
            ComplexForm::Polar => {
                self.write_real_value(&r);
                self.emit("(", TokType::Paren);
                self.emit("cos", TokType::Function);
                self.emit("(", TokType::Paren);
                self.write_real_value(&th);
                self.emit(")", TokType::Paren);
                self.emit(" + ", TokType::Operator);
                self.emit(iname, TokType::Variable);
                self.emit(" ", TokType::Plain);
                self.emit("sin", TokType::Function);
                self.emit("(", TokType::Paren);
                self.write_real_value(&th);
                self.emit(")", TokType::Paren);
                self.emit(")", TokType::Paren);
            }
            ComplexForm::Cis => {
                self.write_real_value(&r);
                self.emit(" cis ", TokType::Function);
                self.write_real_value(&th);
            }
            _ => {
                let am = c.map(|c| c.opts().angle).unwrap_or(AngleMode::Radians);
                let (ang, unit) = match am {
                    AngleMode::Degrees => (&th * &Real::int(180) / rmath::pi(), "°"),
                    AngleMode::Gradians => (&th * &Real::int(200) / rmath::pi(), " gon"),
                    _ => (th.clone(), ""),
                };
                self.write_real_value(&r);
                self.emit("∠", TokType::Operator);
                self.write_real_value(&ang);
                self.emit(unit, TokType::Unit);
            }
        }
        if let Some(c) = c {
            c.set_exact(save);
        }
    }

    fn write_real_value(&mut self, r: &Real) {
        if r.is_infinite() {
            let s = if r.sign() < 0 { format!("{}∞", self.minus()) } else { "∞".into() };
            self.emit(&s, TokType::Number);
            return;
        }
        let b = self.o.output_base;
        if b != 10 {
            if let Some(s) = self.format_in_base(r, b) {
                self.emit(&s, TokType::Number);
                return;
            }
        }
        if !r.approx && !r.is_int() {
            let mut mode = self.o.fraction;
            if mode == FractionMode::DecimalExact && self.o.approx == ApproxMode::Exact && !terminates(r) {
                mode = FractionMode::Fraction;
            }
            let ds = self.div_sign();
            match mode {
                FractionMode::Fraction => {
                    let s = format!("{}{}{}", self.format_int(&r.n), ds, self.format_int(&r.d));
                    self.emit(&s, TokType::Number);
                    return;
                }
                FractionMode::Combined => {
                    let w = r.trunc();
                    let fr = (r - &w).abs();
                    if w.is_zero() {
                        let s = format!("{}{}{}", self.format_int(&r.n), ds, self.format_int(&r.d));
                        self.emit(&s, TokType::Number);
                        return;
                    }
                    let ws = self.format_int(&w.n);
                    self.emit(&ws, TokType::Number);
                    let op = if r.sign() < 0 { format!(" {} ", self.minus()) } else { " + ".into() };
                    self.emit(&op, TokType::Operator);
                    let s = format!("{}{}{}", self.format_int(&fr.n), ds, self.format_int(&fr.d));
                    self.emit(&s, TokType::Number);
                    return;
                }
                _ => {}
            }
        }
        let scaled = match self.o.fraction {
            FractionMode::Percent => Some((100, "%")),
            FractionMode::Permille => Some((1000, "‰")),
            FractionMode::Permyriad => Some((10000, "‱")),
            _ => None,
        };
        if let Some((k, sym)) = scaled {
            let s = self.format_real(&(r * &Real::int(k)), -1);
            self.emit(&s, TokType::Number);
            self.emit(sym, TokType::Unit);
            return;
        }
        let s = self.format_real(r, -1);
        self.emit(&s, TokType::Number);
    }

    fn format_int(&self, n: &BigInt) -> String {
        let s = self.group(&n.abs().to_string());
        if n.is_negative() {
            format!("{}{}", self.minus(), s)
        } else {
            s
        }
    }

    fn group(&self, int_part: &str) -> String {
        if self.o.grouping == DigitGrouping::None || int_part.len() <= 4 {
            return int_part.to_string();
        }
        let mut sep = if self.o.grouping == DigitGrouping::Standard { if self.o.unicode { "\u{2009}" } else { " " } } else { "," };
        if self.o.grouping == DigitGrouping::Locale && comma_decimal() {
            sep = ".";
        }
        let mut out = String::new();
        let first = int_part.len() % 3;
        if first > 0 {
            out.push_str(&int_part[..first]);
        }
        let mut i = first;
        while i < int_part.len() {
            if !out.is_empty() {
                out.push_str(sep);
            }
            out.push_str(&int_part[i..i + 3]);
            i += 3;
        }
        out
    }

    // decimal formatting with significant digits and display mode
    pub fn format_real(&mut self, r: &Real, sig: i32) -> String {
        if r.is_infinite() {
            return if r.sign() < 0 { format!("{}∞", self.minus()) } else { "∞".into() };
        }
        let sig = if sig < 0 { self.o.precision } else { sig };
        let dp = self.dec_point();
        if r.is_zero() {
            return if self.o.min_decimals > 0 { format!("0{}{}", dp, "0".repeat(self.o.min_decimals as usize)) } else { "0".into() };
        }
        let neg = r.sign() < 0;
        let a = r.abs();
        let sign = if neg { self.minus() } else { "" };
        let display = self.o.display;

        if a.is_int() && !a.approx && !matches!(display, NumberDisplay::Scientific | NumberDisplay::Engineering | NumberDisplay::PureScientific) {
            let ints = a.n.to_string();
            if !(display == NumberDisplay::Normal && ints.len() > (sig.max(20) * 3) as usize && ints.len() > 100) {
                let mut res = self.group(&ints);
                if self.o.min_decimals > 0 {
                    res += &format!("{}{}", dp, "0".repeat(self.o.min_decimals as usize));
                }
                return format!("{}{}", sign, res);
            }
        }

        let mut e10 = exp10(&a);
        let exact = !a.approx;
        if exact && !a.is_int() && self.o.fraction == FractionMode::DecimalExact && self.o.repeating != RepeatingMode::None {
            if let Some((rep, non_rep, int_part)) = repeating_decimal(&a, sig) {
                if display == NumberDisplay::Normal && e10 < sig && e10 > -6 {
                    let ip = self.group(&int_part);
                    if self.o.repeating == RepeatingMode::Overline {
                        let ov: String = rep.chars().flat_map(|ch| [ch, '\u{0305}']).collect();
                        return format!("{}{}{}{}{}", sign, ip, dp, non_rep, ov);
                    }
                    return format!("{}{}{}{}{}{}…", sign, ip, dp, non_rep, rep, rep);
                }
            }
        }

        let use_exp_for = |e10: i32| match display {
            NumberDisplay::Scientific => e10 != 0,
            NumberDisplay::PureScientific => true,
            NumberDisplay::Engineering => !(0..3).contains(&e10),
            NumberDisplay::Simple => false,
            NumberDisplay::Normal => e10 >= sig || e10 < -6,
        };
        let mut use_exp = use_exp_for(e10);
        let mut body = String::new();
        let mut exp = 0;
        for attempt in 0..2 {
            exp = 0;
            if use_exp {
                exp = if display == NumberDisplay::Engineering { (e10 as f64 / 3.0).floor() as i32 * 3 } else { e10 };
            }
            let point_pos = e10 - exp + 1;
            let mut frac_digits = sig - point_pos;
            if self.o.max_decimals >= 0 && frac_digits > self.o.max_decimals {
                frac_digits = self.o.max_decimals;
            }
            let (m, inexact) = self.scale_round(&a, frac_digits - exp);
            let mut ds = m.to_string();
            let expected_len = point_pos + frac_digits;
            if attempt == 0 && ds.len() as i32 > expected_len && expected_len > 0 && !m.is_zero() {
                e10 += 1;
                if display != NumberDisplay::PureScientific && display != NumberDisplay::Simple {
                    use_exp = use_exp_for(e10);
                }
                continue;
            }
            if inexact {
                self.rounded = true;
            }
            let (int_str, mut frac_str) = if frac_digits <= 0 {
                ((m * num_traits::pow(BigInt::from(10), (-frac_digits) as usize)).to_string(), String::new())
            } else {
                let fd = frac_digits as usize;
                if ds.len() <= fd {
                    ds = format!("{}{}", "0".repeat(fd - ds.len() + 1), ds);
                }
                (ds[..ds.len() - fd].to_string(), ds[ds.len() - fd..].to_string())
            };
            if !(self.o.show_ending_zeroes && a.approx) {
                frac_str = frac_str.trim_end_matches('0').to_string();
            }
            while (frac_str.len() as i32) < self.o.min_decimals {
                frac_str.push('0');
            }
            body = format!("{}{}", self.group(&int_str), if frac_str.is_empty() { String::new() } else { format!("{}{}", dp, frac_str) });
            break;
        }
        if use_exp && (exp != 0 || display == NumberDisplay::PureScientific) {
            if self.o.e_notation || !self.o.unicode {
                body += &format!("{}{}{}", if self.o.lower_case_e { "e" } else { "E" }, if exp < 0 { "-" } else { "" }, exp.abs());
            } else {
                body += &format!(" {} 10{}", self.times(), superscript(&BigInt::from(exp)));
            }
        }
        if body.trim_matches(|c| c == '0' || c == '.' || c == ',').is_empty() {
            return body;
        }
        format!("{}{}", sign, body)
    }

    fn scale_round(&self, a: &Real, scale: i32) -> (BigInt, bool) {
        let mut n = a.n.clone();
        let mut d = a.d.clone();
        if scale >= 0 {
            n *= num_traits::pow(BigInt::from(10), scale as usize);
        } else {
            d *= num_traits::pow(BigInt::from(10), (-scale) as usize);
        }
        let (q, rem) = n.div_rem(&d);
        let inexact = !rem.is_zero() || a.approx;
        if rem.is_zero() {
            return (q, inexact);
        }
        (Real::exact(n, d).round(self.o.rounding).n, inexact)
    }

    pub fn format_in_base(&mut self, r: &Real, b: i32) -> Option<String> {
        use bases::*;
        let m = self.minus();
        match b {
            ROMAN => return if r.is_int() { funcs::to_roman(&r.to_big()) } else { None },
            TIME => return Some(self.sexagesimal(r, ":", ":", "", true)),
            SEXAGESIMAL => return Some(self.sexagesimal(r, "°", "′", "″", false)),
            LATITUDE => return Some(format!("{}{}", self.sexagesimal(&r.abs(), "°", "′", "″", false), if r.sign() < 0 { " S" } else { " N" })),
            LONGITUDE => return Some(format!("{}{}", self.sexagesimal(&r.abs(), "°", "′", "″", false), if r.sign() < 0 { " W" } else { " E" })),
            BIJECTIVE26 => return if r.is_int() && r.sign() > 0 { Some(bijective(&r.to_big())) } else { None },
            BCD => return if r.is_int() { Some(bcd(&r.to_big())) } else { None },
            UNICODE => {
                if r.is_int() && r.sign() > 0 && r.n <= big(0x10FFFF) {
                    return char::from_u32(r.n.to_u32().unwrap()).map(|c| c.to_string());
                }
                return None;
            }
            GOLDEN => return Some(self.real_base(r, (1.0 + 5f64.sqrt()) / 2.0)),
            SUPERGOLDEN => return Some(self.real_base(r, 1.4655712318767680267)),
            PI_BASE => return Some(self.real_base(r, std::f64::consts::PI)),
            E_BASE => return Some(self.real_base(r, std::f64::consts::E)),
            SQRT2 => return Some(self.real_base(r, 2f64.sqrt())),
            FP16 => return Some(floatformats::binary_string(r, &floatformats::HALF)),
            FP32 => return Some(floatformats::binary_string(r, &floatformats::SINGLE)),
            FP64 => return Some(floatformats::binary_string(r, &floatformats::DOUBLE)),
            FP80 => return Some(floatformats::binary_string(r, &floatformats::EXTENDED)),
            FP128 => return Some(floatformats::binary_string(r, &floatformats::QUAD)),
            CUSTOM => {
                let c = try_cur()?;
                let custom = self.o.output_base_custom.clone();
                let be = attempt(|| c.eval_text(&custom)).ok()?;
                let bn = be.as_real()?.clone();
                if bn.is_int() && !bn.approx {
                    return self.format_in_base(r, bn.to_int());
                }
                return Some(self.real_base(r, bn.to_f64()));
            }
            _ => {}
        }
        if b == 10 {
            return None;
        }
        if (2..=36).contains(&b) {
            return Some(self.integer_base(r, b as u32));
        }
        if (-36..=-2).contains(&b) {
            return self.negative_base(r, -b);
        }
        if b > 36 && b <= 1000000 {
            if !r.is_int() {
                return None;
            }
            let mut v = r.to_big().abs();
            let mut parts = vec![];
            if v.is_zero() {
                parts.push("0".to_string());
            }
            while !v.is_zero() {
                parts.insert(0, (&v % b).to_string());
                v /= b;
            }
            return Some(format!("{}{}", if r.sign() < 0 { m } else { "" }, parts.join(":")));
        }
        None
    }

    fn integer_base(&mut self, r: &Real, b: u32) -> String {
        let neg = r.sign() < 0;
        let a = r.abs();
        let ip = a.floor().to_big();
        let digits = digit_chars(self.o.lower_case_hex, b == 12 && self.o.duodecimal_symbols);
        if neg && a.is_int() && (b == 2 || b == 16) && ((self.o.twos_out == TwosComplement::Binary && b == 2) || self.o.twos_out == TwosComplement::BinaryHex) {
            let mut bits = if self.o.binary_bits > 0 { self.o.binary_bits } else { 8 };
            while (BigInt::one() << (bits - 1) as usize) < ip && bits < 4096 {
                bits *= 2;
            }
            let tc = (BigInt::one() << bits as usize) - &ip;
            let w = if b == 2 { bits } else { bits / 4 } as usize;
            let s = format!("{:0>w$}", to_digits(&tc, b, &digits), w = w);
            return self.group_base(&s, b);
        }
        let mut int_str = to_digits(&ip, b, &digits);
        if b == 2 && self.o.binary_bits > 0 {
            int_str = format!("{:0>w$}", int_str, w = self.o.binary_bits as usize);
        }
        let mut frac = &a - &a.floor();
        let mut fs = String::new();
        if !frac.is_zero() {
            let mut maxd = (self.o.precision as f64 * 10f64.ln() / (b as f64).ln()).ceil() as i32;
            if self.o.max_decimals >= 0 {
                maxd = maxd.min(self.o.max_decimals);
            }
            let mut i = 0;
            while i < maxd && !frac.is_zero() {
                frac = frac * Real::int(b as i64);
                let dgt = frac.floor();
                fs.push(digits[dgt.to_int() as usize]);
                frac = frac - dgt;
                i += 1;
            }
            if !frac.is_zero() {
                self.rounded = true;
            }
        }
        let fs = fs.trim_end_matches('0');
        let res = format!("{}{}", self.group_base(&int_str, b), if fs.is_empty() { String::new() } else { format!("{}{}", self.dec_point(), fs) });
        format!("{}{}", if neg { self.minus() } else { "" }, res)
    }

    fn group_base(&self, s: &str, b: u32) -> String {
        let g = self.o.binary_letter_spacing;
        if b != 2 || g <= 0 || s.len() <= g as usize {
            return s.to_string();
        }
        let g = g as usize;
        let mut out = String::new();
        let first = s.len() % g;
        if first > 0 {
            out.push_str(&s[..first]);
        }
        let mut i = first;
        while i < s.len() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&s[i..i + g]);
            i += g;
        }
        out
    }

    fn negative_base(&self, r: &Real, b: i32) -> Option<String> {
        if !r.is_int() {
            return None;
        }
        let mut v = r.to_big();
        if v.is_zero() {
            return Some("0".into());
        }
        let digits = digit_chars(self.o.lower_case_hex, false);
        let mut s = vec![];
        let nb = BigInt::from(-b);
        while !v.is_zero() {
            let mut rem = (&v % &nb).to_i32().unwrap();
            v /= &nb;
            if rem < 0 {
                rem += b;
                v += 1;
            }
            s.push(digits[rem as usize]);
        }
        Some(s.iter().rev().collect())
    }

    fn real_base(&mut self, r: &Real, beta: f64) -> String {
        let mut x = r.to_f64().abs();
        if x == 0.0 {
            return "0".into();
        }
        let dp = self.dec_point();
        let max_digit = beta.ceil() as i32 - 1;
        let k = ((x.ln() / beta.ln() + 1e-12).floor() as i32).max(0);
        let frac_limit = ((self.o.precision.min(14) as f64) * 10f64.ln() / beta.ln()).ceil() as i32;
        let mut s = String::new();
        let mut p = k;
        while p >= -frac_limit {
            if p == -1 {
                s.push_str(dp);
            }
            let bp = beta.powi(p);
            let d = ((x / bp + 1e-12).floor() as i32).clamp(0, max_digit);
            x -= d as f64 * bp;
            s.push((b'0' + d as u8) as char);
            if p < 0 && x < 1e-14 {
                break;
            }
            p -= 1;
        }
        if x > 1e-12 {
            self.rounded = true;
        }
        if s.contains(dp) {
            s = s.trim_end_matches('0').trim_end_matches(dp).to_string();
        }
        format!("{}{}", if r.sign() < 0 { self.minus() } else { "" }, s)
    }

    fn sexagesimal(&mut self, r: &Real, s1: &str, s2: &str, s3: &str, time: bool) -> String {
        let neg = r.sign() < 0;
        let a = r.abs();
        let mut deg = a.floor();
        let minf = (&a - &deg) * Real::int(60);
        let mut min = minf.floor();
        let sec = (&minf - &min) * Real::int(60);
        let sec_r = sec.round_default();
        let mut sec_str = if (&sec - &sec_r).abs().lt(&Real::frac(1, 1000000)) {
            sec_r.to_big().to_string()
        } else {
            self.rounded = true;
            let p = (self.o.precision - 4).max(3);
            self.format_real(&sec, p)
        };
        if sec_str == "60" {
            sec_str = "0".into();
            min = min + Real::one();
        }
        if min.eqv(&Real::int(60)) {
            min = Real::zero();
            deg = deg + Real::one();
        }
        let res = if time {
            format!("{}:{:0>2}:{:0>2}", deg.to_big(), min.to_big().to_string(), sec_str)
        } else {
            format!("{}{}{}{}{}{}", deg.to_big(), s1, min.to_big(), s2, sec_str, s3)
        };
        format!("{}{}", if neg { self.minus() } else { "" }, res)
    }
}