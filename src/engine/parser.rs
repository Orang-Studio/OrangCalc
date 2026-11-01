use super::*;
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokKind {
    Num,
    Ident,
    Str,
    Op,
    LParen,
    RParen,
    LBrack,
    RBrack,
    Sep,
    RowSep,
    Date,
    End,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokKind,
    pub text: String,
    pub value: Option<Expr>,
    pub space_before: bool,
}

const OPS: &[&str] = &[
    ":=", "**", "//", "<<", ">>", "<=", ">=", "!=", "==", "&&", "||", "->", "!!", "+", "-", "*", "/", "^", "(", ")", "=", "<", ">", "!", "%", "&", "|", "~", "×", "·", "÷", "−", "∕", "⋅", "≠", "≤", "≥", "√", "∛", "∜", "‰", "‱", "∠", "±", "→", "⊻", "¬", "∧", "∨", "∙",
];

fn superscript(c: char) -> Option<char> {
    Some(match c {
        '⁰' => '0',
        '¹' => '1',
        '²' => '2',
        '³' => '3',
        '⁴' => '4',
        '⁵' => '5',
        '⁶' => '6',
        '⁷' => '7',
        '⁸' => '8',
        '⁹' => '9',
        '⁻' => '-',
        '⁺' => '+',
        _ => return None,
    })
}

fn vulgar(c: char) -> Option<(i64, i64)> {
    Some(match c {
        '½' => (1, 2),
        '⅓' => (1, 3),
        '⅔' => (2, 3),
        '¼' => (1, 4),
        '¾' => (3, 4),
        '⅕' => (1, 5),
        '⅖' => (2, 5),
        '⅗' => (3, 5),
        '⅘' => (4, 5),
        '⅙' => (1, 6),
        '⅚' => (5, 6),
        '⅐' => (1, 7),
        '⅛' => (1, 8),
        '⅜' => (3, 8),
        '⅝' => (5, 8),
        '⅞' => (7, 8),
        '⅑' => (1, 9),
        '⅒' => (1, 10),
        _ => return None,
    })
}

pub fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

pub fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || "_°µΩ℃℉Å∞℧$€£¥₿₹₽₩¢∑∏∫ℏ∂".contains(ch)
}

pub fn is_ident_part(ch: char) -> bool {
    is_ident_start(ch) || is_digit(ch)
}

static DATE_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{4}-\d{2}-\d{2}([T ]\d{1,2}:\d{2}(:\d{2}(\.\d+)?)?)?(Z|[+-]\d{2}:?\d{2})?").unwrap());
static SEXA_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^(\d+(?:\.\d+)?)°(?:(\d+(?:\.\d+)?)['′](?:(\d+(?:\.\d+)?)["″])?)?"#).unwrap());
// the original used a (?![\d:]) lookahead; checked by hand after matching
static TIME_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+):(\d{1,2})(?::(\d{1,2}(?:\.\d+)?))?").unwrap());

pub struct Lexer<'a> {
    s: Vec<char>,
    c: &'a Calculator,
    i: usize,
    pub tokens: Vec<Token>,
}

fn substr(s: &[char], a: usize, b: usize) -> String {
    s[a..b].iter().collect()
}

impl<'a> Lexer<'a> {
    pub fn new(text: &str, c: &'a Calculator) -> Lexer<'a> {
        let mut l = Lexer { s: text.chars().collect(), c, i: 0, tokens: vec![] };
        l.run();
        l
    }

    fn comma_decimal(&self) -> bool {
        self.c.opts().comma_decimal
    }

    fn add(&mut self, k: TokKind, text: &str, space: bool, _pos: usize, v: Option<Expr>) {
        self.tokens.push(Token { kind: k, text: text.to_string(), space_before: space, value: v });
    }

    fn run(&mut self) {
        let n = self.s.len();
        while self.i < n {
            let mut space = false;
            while self.i < n && self.s[self.i].is_whitespace() {
                self.i += 1;
                space = true;
            }
            if self.i >= n {
                break;
            }
            let ch = self.s[self.i];
            let start = self.i;
            if ch == '#' {
                break;
            }
            if ch == '"' || ch == '\'' || ch == '“' || ch == '‘' {
                let close = if ch == '“' {
                    '”'
                } else if ch == '‘' {
                    '’'
                } else {
                    ch
                };
                let j = (self.i + 1..n).find(|&k| self.s[k] == close).unwrap_or(n);
                let t = substr(&self.s, self.i + 1, j);
                self.add(TokKind::Str, &t, space, start, None);
                self.i = n.min(j + 1);
                continue;
            }
            let next_digit = self.i + 1 < n && is_digit(self.s[self.i + 1]);
            if is_digit(ch) || (ch == '.' && next_digit) || (ch == ',' && self.comma_decimal() && next_digit) {
                self.lex_number(space);
                continue;
            }
            if let Some((a, b)) = vulgar(ch) {
                self.add(TokKind::Num, &ch.to_string(), space, start, Some(num_r(Real::frac(a, b))));
                self.i += 1;
                continue;
            }
            if superscript(ch).is_some() {
                let mut sb = String::new();
                while self.i < n {
                    match superscript(self.s[self.i]) {
                        Some(d) => {
                            sb.push(d);
                            self.i += 1;
                        }
                        None => break,
                    }
                }
                self.add(TokKind::Op, "^", false, start, None);
                let mut t = sb.as_str();
                if let Some(r) = t.strip_prefix('-') {
                    self.add(TokKind::Op, "-", false, start, None);
                    t = r;
                }
                if let Some(r) = t.strip_prefix('+') {
                    t = r;
                }
                let v = Real::parse(if t.is_empty() { "1" } else { t }, 10);
                let t = t.to_string();
                self.add(TokKind::Num, &t, false, start, Some(num_r(v)));
                continue;
            }
            if ch == '\\' && self.i + 1 < n && self.s[self.i + 1].is_alphabetic() {
                let a = self.s[self.i + 1];
                let idx = if ('x'..='z').contains(&a) { a as i64 - 'x' as i64 } else { a as i64 - 'a' as i64 + 3 };
                self.add(TokKind::Num, &format!("\\{}", a), space, start, Some(mk(E::Arg(idx.max(0) as usize))));
                self.i += 2;
                continue;
            }
            if is_ident_start(ch) {
                let mut j = self.i + 1;
                let single = "∑∏∫∞∂".contains(ch);
                if !single {
                    while j < n && is_ident_part(self.s[j]) && !"∞∑∏∫".contains(self.s[j]) {
                        j += 1;
                    }
                }
                let t = substr(&self.s, self.i, j);
                self.add(TokKind::Ident, &t, space, start, None);
                self.i = j;
                continue;
            }
            let simple = match ch {
                '(' => Some((TokKind::LParen, "(")),
                ')' => Some((TokKind::RParen, ")")),
                '[' | '{' => Some((TokKind::LBrack, "[")),
                ']' | '}' => Some((TokKind::RBrack, "]")),
                ';' => Some((if self.comma_decimal() { TokKind::Sep } else { TokKind::RowSep }, ";")),
                _ => None,
            };
            if let Some((k, t)) = simple {
                self.add(k, t, space, start, None);
                self.i += 1;
                continue;
            }
            if ch == ',' {
                if self.c.opts().ignore_comma {
                    self.i += 1;
                    continue;
                }
                self.add(TokKind::Sep, ",", space, start, None);
                self.i += 1;
                continue;
            }
            if ch == '′' || ch == '″' {
                self.add(TokKind::Ident, if ch == '′' { "arcmin" } else { "arcsec" }, false, start, None);
                self.i += 1;
                continue;
            }
            let mut op: Option<&str> = None;
            for o in OPS {
                let oc: Vec<char> = o.chars().collect();
                if self.i + oc.len() <= n && self.s[self.i..self.i + oc.len()] == oc[..] {
                    op = Some(o);
                    break;
                }
            }
            if let Some(o) = op {
                self.i += o.chars().count();
                self.add(TokKind::Op, normalize_op(o), space, start, None);
                continue;
            }
            throw_s(format!("Unexpected character '{}'", ch));
        }
        self.add(TokKind::End, "", false, n, None);
    }

    fn lex_number(&mut self, space: bool) {
        let n = self.s.len();
        let start = self.i;
        let rest = substr(&self.s, self.i, n);
        let input_base = self.c.opts().input_base;
        if input_base == 10 {
            if let Some(dm) = DATE_RX.find(&rest) {
                let (d, has_time, utc) = funcs::parse_date(dm.as_str());
                self.add(TokKind::Date, dm.as_str(), space, start, Some(date_e(d, has_time, utc)));
                self.i += dm.as_str().chars().count();
                return;
            }
            if let Some(sm) = SEXA_RX.captures(&rest) {
                if sm.get(2).is_some() {
                    let mut v = Real::parse(&sm[1], 10);
                    v = v + Real::parse(&sm[2], 10) / Real::int(60);
                    if let Some(g3) = sm.get(3) {
                        v = v + Real::parse(g3.as_str(), 10) / Real::int(3600);
                    }
                    let all = sm.get(0).unwrap().as_str();
                    self.i += all.chars().count();
                    self.add(TokKind::Num, all, space, start, Some(num_r(v)));
                    self.add(TokKind::Ident, "°", false, start, None);
                    return;
                }
            }
            if let Some(tm) = TIME_RX.captures(&rest) {
                let all = tm.get(0).unwrap().as_str();
                let after = rest[all.len()..].chars().next();
                let ok = !matches!(after, Some(c) if is_digit(c) || c == ':');
                if ok {
                    let mut v = Real::parse(&tm[1], 10) + Real::parse(&tm[2], 10) / Real::int(60);
                    if let Some(g3) = tm.get(3) {
                        v = v + Real::parse(g3.as_str(), 10) / Real::int(3600);
                    }
                    self.i += all.chars().count();
                    self.add(TokKind::Num, all, space, start, Some(num_r(v)));
                    return;
                }
            }
        }
        let s = &self.s;
        let mut i = self.i;
        let mut radix = input_base;
        if s[i] == '0' && i + 1 < n {
            let p = s[i + 1].to_ascii_lowercase();
            if p == 'x' && i + 2 < n && s[i + 2].is_ascii_hexdigit() {
                radix = 16;
                i += 2;
            } else if p == 'b' && i + 2 < n && (s[i + 2] == '0' || s[i + 2] == '1') {
                radix = 2;
                i += 2;
            } else if p == 'o' && i + 2 < n && is_digit(s[i + 2]) {
                radix = 8;
                i += 2;
            } else if p == 'd' && i + 2 < n && is_digit(s[i + 2]) {
                radix = 12;
                i += 2;
            }
        }
        if !(2..=36).contains(&radix) {
            radix = 10;
        }
        let (comma_decimal, ignore_dots, ignore_comma, read_precision, concise) = {
            let o = self.c.opts();
            (o.comma_decimal, o.ignore_dots, o.ignore_comma, o.read_precision, o.concise_uncertainty_input)
        };
        let mut sb = String::new();
        let mut dot = false;
        let dec_char = if comma_decimal { ',' } else { '.' };
        while i < n {
            let ch = s[i];
            if ch == dec_char && !dot && !(comma_decimal && ignore_dots && ch == '.') {
                if i + 1 < n && s[i + 1] == dec_char {
                    break;
                }
                dot = true;
                sb.push('.');
                i += 1;
                continue;
            }
            if ch == '.' && comma_decimal && ignore_dots {
                i += 1;
                continue;
            }
            if ch == ',' && !comma_decimal && ignore_comma {
                i += 1;
                continue;
            }
            if ch == ' ' && radix != 10 && i + 1 < n {
                let dv1 = real::digit_value(s[i + 1]);
                if dv1 >= 0 && dv1 < radix && !sb.is_empty() && (radix == 2 || radix == 16) {
                    i += 1;
                    continue;
                }
            }
            let dv = real::digit_value(ch);
            if radix > 10 && dv >= 10 && dv < radix && ch.is_alphabetic() {
                let mut j = i;
                while j < n && s[j].is_alphanumeric() {
                    j += 1;
                }
                if !s[i..j].iter().all(|&x| {
                    let d = real::digit_value(x);
                    d >= 0 && d < radix
                }) {
                    break;
                }
            }
            if dv < 0 || dv >= radix || (radix <= 10 && !is_digit(ch)) {
                if radix == 12 && (ch == '↊' || ch == '↋') {
                    sb.push(ch);
                    i += 1;
                    continue;
                }
                break;
            }
            sb.push(ch);
            i += 1;
        }
        let mut val = Real::parse(&sb, radix as u32);
        let approx = read_precision && dot;
        if i < n && (s[i] == 'e' || s[i] == 'E') && radix <= 10 {
            let mut j = i + 1;
            if j < n && (s[j] == '+' || s[j] == '-' || s[j] == '−') {
                j += 1;
            }
            if j < n && is_digit(s[j]) {
                let mut k = j;
                while k < n && is_digit(s[k]) {
                    k += 1;
                }
                let es = substr(s, i + 1, k).replace('−', "-");
                let ex: i64 = es.trim_start_matches('+').parse().unwrap_or_else(|_| throw("Exponent too large"));
                val = val * Real::int(10).pow_i(ex);
                i = k;
            }
        }
        let mut value = num_r(if approx { val.as_approx() } else { val.clone() });
        if concise && i < n && s[i] == '(' && dot {
            if let Some(j) = (i..n).find(|&k| s[k] == ')') {
                if j > i + 1 && s[i + 1..j].iter().all(|c| is_digit(*c)) {
                    let u = substr(s, i + 1, j);
                    let decs = sb.len() - sb.find('.').unwrap() - 1;
                    let err = Real::parse(&u, 10) / Real::int(10).pow_i(decs as i64);
                    value = intv_e(&val - &err, &val + &err);
                    i = j + 1;
                }
            }
        }
        let text = substr(s, start, i);
        self.i = i;
        self.add(TokKind::Num, &text, space, start, Some(value));
    }
}

fn normalize_op(op: &str) -> &str {
    match op {
        "×" | "·" | "⋅" | "∙" => "*",
        "÷" | "∕" => "/",
        "−" => "-",
        "**" => "^",
        "≠" => "!=",
        "≤" => "<=",
        "≥" => ">=",
        "==" => "=",
        "→" | "->" => "→",
        "∧" => "&&",
        "∨" => "||",
        "¬" => "NOT",
        _ => op,
    }
}

// raw tree builders used by the parser, no simplification
pub mod ops {
    use super::super::*;

    pub fn add(a: Expr, b: Expr) -> Expr {
        let mut l = vec![];
        match &a.e {
            E::Add(t) => l.extend(t.iter().cloned()),
            _ => l.push(a.clone()),
        }
        match &b.e {
            E::Add(t) => l.extend(t.iter().cloned()),
            _ => l.push(b.clone()),
        }
        add_e(l)
    }
    pub fn sub(a: Expr, b: Expr) -> Expr {
        add(a, neg(b))
    }
    pub fn neg(a: Expr) -> Expr {
        if let E::Num(n) = &a.e {
            return num(-n);
        }
        if let E::Mul(f) = &a.e {
            if let Some(E::Num(c)) = f.first().map(|x| &x.e) {
                let mut l = f.clone();
                l[0] = num(-c);
                if l[0].is_one() {
                    l.remove(0);
                }
                return if l.len() == 1 { l.pop().unwrap() } else { mul_e(l) };
            }
        }
        mul_e(vec![minus_one(), a])
    }
    pub fn mul(a: Expr, b: Expr) -> Expr {
        let mut l = vec![];
        match &a.e {
            E::Mul(t) => l.extend(t.iter().cloned()),
            _ => l.push(a.clone()),
        }
        match &b.e {
            E::Mul(t) => l.extend(t.iter().cloned()),
            _ => l.push(b.clone()),
        }
        mul_e(l)
    }
    pub fn div(a: Expr, b: Expr) -> Expr {
        mul(a, pow(b, minus_one()))
    }
    pub fn pow(b: Expr, e: Expr) -> Expr {
        pow_e(b, e)
    }
}

pub struct Parser<'a> {
    t: Vec<Token>,
    p: usize,
    c: &'a Calculator,
    mode: ParseMode,
    pub warnings: Vec<String>,
    pub local_symbols: HashSet<String>,
}

pub fn parse_text(text: &str, c: &Calculator) -> Option<Expr> {
    Parser::new(text, c).parse_all()
}

pub fn parse_text_w(text: &str, c: &Calculator, warnings: &mut Vec<String>) -> Option<Expr> {
    let mut ps = Parser::new(text, c);
    let e = ps.parse_all();
    warnings.extend(ps.warnings.drain(..));
    e
}

// parse that must yield an expression
pub fn parse_req(text: &str, c: &Calculator) -> Expr {
    parse_text(text, c).unwrap_or_else(|| throw("Empty expression"))
}

impl<'a> Parser<'a> {
    pub fn new(text: &str, c: &'a Calculator) -> Parser<'a> {
        let mode = c.opts().parse;
        Parser { t: Lexer::new(text, c).tokens, p: 0, c, mode, warnings: vec![], local_symbols: HashSet::new() }
    }

    fn peek(&self) -> &Token {
        &self.t[self.p]
    }
    fn next(&mut self) -> Token {
        let t = self.t[self.p].clone();
        self.p += 1;
        t
    }
    fn is_op(&self, op: &str) -> bool {
        self.peek().kind == TokKind::Op && self.peek().text == op
    }
    fn is_word(&self, w: &str) -> bool {
        self.peek().kind == TokKind::Ident && self.peek().text.eq_ignore_ascii_case(w)
    }
    fn fnc(&self, name: &str) -> FnRef {
        self.c.fn_(name)
    }
    fn call(&self, name: &str, args: Vec<Expr>) -> Expr {
        call_e(self.fnc(name), args)
    }
    fn conv(&self) -> bool {
        self.mode == ParseMode::Conventional
    }

    pub fn parse_all(&mut self) -> Option<Expr> {
        if self.peek().kind == TokKind::End {
            return None;
        }
        let e = if self.mode == ParseMode::Rpn { self.parse_rpn() } else { Some(self.parse_expr()) };
        if self.peek().kind != TokKind::End {
            if self.peek().kind == TokKind::RParen {
                self.warnings.push("Unmatched right parenthesis ignored".into());
            } else {
                throw_s(format!("Unexpected \"{}\"", self.peek().text));
            }
        }
        e
    }

    fn parse_expr(&mut self) -> Expr {
        if self.mode == ParseMode::Chain {
            self.parse_chain()
        } else {
            self.parse_or()
        }
    }

    // chain mode: operators applied left to right
    fn parse_chain(&mut self) -> Expr {
        let mut left = self.parse_unary();
        loop {
            if self.peek().kind != TokKind::Op {
                break;
            }
            let op = self.peek().text.clone();
            if op == "^" {
                self.next();
                left = ops::pow(left, self.parse_unary());
                continue;
            }
            if op == "+" || op == "-" || op == "*" || op == "/" {
                self.next();
                let r = self.parse_unary();
                left = match op.as_str() {
                    "+" => ops::add(left, r),
                    "-" => ops::sub(left, r),
                    "*" => ops::mul(left, r),
                    _ => ops::div(left, r),
                };
                continue;
            }
            break;
        }
        left
    }

    fn parse_or(&mut self) -> Expr {
        let mut l = self.parse_xor_logical();
        while self.is_op("||") || self.is_word("or") {
            self.next();
            let r = self.parse_xor_logical();
            l = logic_e(LogicOp::Or, vec![l, r]);
        }
        l
    }

    fn parse_xor_logical(&mut self) -> Expr {
        let mut l = self.parse_and();
        while self.is_word("xor") {
            self.next();
            let r = self.parse_and();
            l = self.call("xor", vec![l, r]);
        }
        l
    }

    fn parse_and(&mut self) -> Expr {
        let mut l = self.parse_not();
        while self.is_op("&&") || self.is_word("and") {
            self.next();
            let r = self.parse_not();
            l = logic_e(LogicOp::And, vec![l, r]);
        }
        l
    }

    fn parse_not(&mut self) -> Expr {
        if self.is_word("not") || self.is_op("NOT") {
            self.next();
            let r = self.parse_not();
            return logic_e(LogicOp::Not, vec![r]);
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> Expr {
        let mut l = self.parse_bit_or();
        while self.peek().kind == TokKind::Op {
            let op = match self.peek().text.as_str() {
                "=" => CmpOp::Eq,
                "!=" => CmpOp::Ne,
                "<" => CmpOp::Lt,
                ">" => CmpOp::Gt,
                "<=" => CmpOp::Le,
                ">=" => CmpOp::Ge,
                _ => return l,
            };
            self.next();
            let r = self.parse_bit_or();
            if let (E::Cmp(_, _, lr), true) = (&l.e, op != CmpOp::Eq && op != CmpOp::Ne) {
                let lr = lr.clone();
                l = logic_e(LogicOp::And, vec![l.clone(), cmp_e(op, lr, r)]);
            } else {
                l = cmp_e(op, l, r);
            }
        }
        l
    }

    fn parse_bit_or(&mut self) -> Expr {
        let mut l = self.parse_bit_xor();
        while self.is_op("|") {
            self.next();
            let r = self.parse_bit_xor();
            l = self.call("bitor", vec![l, r]);
        }
        l
    }

    fn parse_bit_xor(&mut self) -> Expr {
        let mut l = self.parse_bit_and();
        while self.is_op("⊻") {
            self.next();
            let r = self.parse_bit_and();
            l = self.call("bitxor", vec![l, r]);
        }
        l
    }

    fn parse_bit_and(&mut self) -> Expr {
        let mut l = self.parse_shift();
        while self.is_op("&") {
            self.next();
            let r = self.parse_shift();
            l = self.call("bitand", vec![l, r]);
        }
        l
    }

    fn parse_shift(&mut self) -> Expr {
        let mut l = self.parse_add();
        while self.is_op("<<") || self.is_op(">>") {
            let left = self.next().text == "<<";
            let r = self.parse_add();
            l = self.call("shift", vec![l, if left { r } else { ops::neg(r) }]);
        }
        l
    }

    fn parse_add(&mut self) -> Expr {
        let mut l = self.parse_mul();
        loop {
            if self.is_op("+") || self.is_op("-") || self.is_word("plus") || self.is_word("minus") {
                let tk = self.next();
                let minus = tk.text == "-" || tk.text.eq_ignore_ascii_case("minus");
                let r = self.parse_mul();
                if self.c.opts().simplified_percent {
                    if let Some(pct) = is_percent_term(&r) {
                        let factor = if minus { ops::sub(one(), pct) } else { ops::add(one(), pct) };
                        l = ops::mul(l, factor);
                        continue;
                    }
                }
                l = if minus { ops::sub(l, r) } else { ops::add(l, r) };
            } else if self.is_op("±") {
                self.next();
                let r = self.parse_mul();
                l = self.call("uncertainty", vec![l, r]);
            } else {
                break;
            }
        }
        l
    }

    fn starts_primary(tk: &Token) -> bool {
        match tk.kind {
            TokKind::Num | TokKind::LParen | TokKind::LBrack | TokKind::Str | TokKind::Date => true,
            TokKind::Ident => {
                let w = tk.text.to_lowercase();
                !matches!(w.as_str(), "mod" | "rem" | "and" | "or" | "xor" | "not" | "to" | "plus" | "minus" | "times" | "per" | "of" | "where")
            }
            TokKind::Op => tk.text == "√" || tk.text == "∛" || tk.text == "∜",
            _ => false,
        }
    }

    fn operand(&mut self) -> Expr {
        if self.conv() {
            self.parse_unary()
        } else {
            self.parse_implicit()
        }
    }

    fn parse_mul(&mut self) -> Expr {
        let mut l = self.operand();
        loop {
            if self.is_op("*") || self.is_word("times") || self.is_word("of") {
                self.next();
                let r = self.operand();
                l = ops::mul(l, r);
            } else if self.is_op("/") || self.is_word("per") {
                self.next();
                let r = self.operand();
                l = ops::div(l, r);
            } else if self.is_op("//") {
                self.next();
                let r = self.operand();
                l = self.call("div", vec![l, r]);
            } else if self.is_word("mod") || (self.is_op("%") && self.p + 1 < self.t.len() && Self::starts_primary(&self.t[self.p + 1]) && self.t[self.p + 1].space_before && self.t[self.p].space_before) {
                self.next();
                let r = self.operand();
                l = self.call("mod", vec![l, r]);
            } else if self.is_word("rem") {
                self.next();
                let r = self.operand();
                l = self.call("rem", vec![l, r]);
            } else if self.is_op("∠") {
                self.next();
                let ang = self.operand();
                l = self.call("_angle", vec![l, ang]);
            } else if Self::starts_primary(self.peek()) && (self.conv() || self.peek().space_before) {
                let r = self.operand();
                l = ops::mul(l, r);
            } else {
                break;
            }
        }
        l
    }

    // implicit multiplication chain, tighter than explicit operators
    fn parse_implicit(&mut self) -> Expr {
        let mut l = self.parse_unary();
        while Self::starts_primary(self.peek()) {
            if self.mode == ParseMode::Adaptive && self.peek().space_before {
                break;
            }
            if self.c.opts().limit_implicit_mult && l.is_mul() {
                break;
            }
            let r = self.parse_power_level();
            l = ops::mul(l, r);
        }
        l
    }

    fn parse_unary(&mut self) -> Expr {
        if self.is_op("-") {
            self.next();
            return ops::neg(self.parse_unary());
        }
        if self.is_op("+") {
            self.next();
            return self.parse_unary();
        }
        if self.is_op("~") {
            self.next();
            let a = self.parse_unary();
            return self.call("bitnot", vec![a]);
        }
        if self.is_op("!") || self.is_op("NOT") {
            self.next();
            let a = self.parse_unary();
            return logic_e(LogicOp::Not, vec![a]);
        }
        if self.is_op("√") {
            self.next();
            let a = self.parse_unary();
            return self.call("sqrt", vec![a]);
        }
        if self.is_op("∛") {
            self.next();
            let a = self.parse_unary();
            return self.call("cbrt", vec![a]);
        }
        if self.is_op("∜") {
            self.next();
            let a = self.parse_unary();
            return self.call("root", vec![a, num_i(4)]);
        }
        self.parse_power_level()
    }

    fn parse_power_level(&mut self) -> Expr {
        let b = self.parse_postfix();
        if self.is_op("^") {
            self.next();
            if self.c.opts().caret_xor {
                let r = self.parse_unary();
                return self.call("bitxor", vec![b, r]);
            }
            let ex = if self.is_op("-") {
                self.next();
                ops::neg(self.parse_power_exp())
            } else if self.is_op("+") {
                self.next();
                self.parse_power_exp()
            } else {
                self.parse_power_exp()
            };
            return ops::pow(b, ex);
        }
        b
    }

    fn parse_power_exp(&mut self) -> Expr {
        if self.is_op("√") || self.is_op("∛") {
            return self.parse_unary();
        }
        self.parse_power_level()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_primary();
        loop {
            if self.is_op("!!") && !self.next_starts_operand() {
                self.next();
                e = self.call("factorial2", vec![e]);
            } else if self.is_op("!") && !self.next_starts_operand() {
                self.next();
                e = self.call("factorial", vec![e]);
            } else if self.is_op("%") && !(self.p + 1 < self.t.len() && Self::starts_primary(&self.t[self.p + 1]) && self.t[self.p].space_before) {
                self.next();
                e = self.call("_percent", vec![e]);
            } else if self.is_op("‰") {
                self.next();
                e = self.call("_permille", vec![e]);
            } else if self.is_op("‱") {
                self.next();
                e = ops::div(e, num_i(10000));
            } else {
                break;
            }
        }
        e
    }

    fn next_starts_operand(&self) -> bool {
        if self.p + 1 >= self.t.len() {
            return false;
        }
        let n = &self.t[self.p + 1];
        n.kind == TokKind::Op && n.text == "="
    }

    fn parse_primary(&mut self) -> Expr {
        let tk = self.peek().clone();
        match tk.kind {
            TokKind::Num | TokKind::Date => {
                self.next();
                tk.value.unwrap()
            }
            TokKind::Str => {
                self.next();
                str_e(&tk.text)
            }
            TokKind::LParen => {
                self.next();
                if self.peek().kind == TokKind::RParen {
                    self.next();
                    return vector_e(vec![]);
                }
                let mut e = self.parse_expr();
                if self.peek().kind == TokKind::Sep {
                    let mut items = vec![e];
                    while self.peek().kind == TokKind::Sep {
                        self.next();
                        items.push(self.parse_expr());
                    }
                    e = vector_e(items);
                }
                if self.peek().kind == TokKind::RParen {
                    self.next();
                } else if self.peek().kind != TokKind::End {
                    throw("Expected )");
                }
                e
            }
            TokKind::LBrack => self.parse_matrix(),
            TokKind::Ident => self.parse_ident(),
            TokKind::End => throw("Unexpected end of expression"),
            _ => throw_s(format!("Unexpected \"{}\"", tk.text)),
        }
    }

    fn scan_separated(&self) -> bool {
        let mut dep = 0;
        for tk in &self.t[self.p..] {
            match tk.kind {
                TokKind::LParen | TokKind::LBrack => dep += 1,
                TokKind::RParen | TokKind::RBrack => {
                    if dep == 0 {
                        return false;
                    }
                    dep -= 1;
                }
                TokKind::Sep | TokKind::RowSep if dep == 0 => return true,
                TokKind::End => return false,
                _ => {}
            }
        }
        false
    }

    fn parse_matrix(&mut self) -> Expr {
        self.next();
        let separated = self.scan_separated();
        let mut rows: Vec<Vec<Expr>> = vec![];
        let mut row = vec![];
        let mut saw_row_sep = false;
        while self.peek().kind != TokKind::RBrack && self.peek().kind != TokKind::End {
            if self.peek().kind == TokKind::Sep {
                self.next();
                continue;
            }
            if self.peek().kind == TokKind::RowSep {
                self.next();
                rows.push(std::mem::take(&mut row));
                saw_row_sep = true;
                continue;
            }
            let item = if separated { self.parse_expr() } else { self.parse_matrix_item() };
            row.push(item);
        }
        if self.peek().kind == TokKind::RBrack {
            self.next();
        }
        rows.push(row);
        if !saw_row_sep {
            let items = rows.swap_remove(0);
            if !items.is_empty() {
                let vecs: Option<Vec<&Vec<Expr>>> = items
                    .iter()
                    .map(|x| match &x.e {
                        E::Mat(r, true) => Some(&r[0]),
                        _ => None,
                    })
                    .collect();
                if let Some(v) = vecs {
                    let c0 = v[0].len();
                    if v.iter().all(|r| r.len() == c0) {
                        return mat_e(v.into_iter().cloned().collect(), false);
                    }
                }
            }
            return vector_e(items);
        }
        let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        for r in rows.iter_mut() {
            while r.len() < cols {
                r.push(zero());
            }
        }
        mat_e(rows, false)
    }

    // space separated matrix items, no implicit multiplication across spaces
    fn parse_matrix_item(&mut self) -> Expr {
        if self.scan_separated() {
            return self.parse_expr();
        }
        let mut l = self.parse_unary();
        loop {
            if self.peek().kind == TokKind::Op {
                let op = self.peek().text.clone();
                let next_tight = self.p + 1 < self.t.len() && !self.t[self.p + 1].space_before;
                if (op == "+" || op == "-") && self.peek().space_before && next_tight {
                    break;
                }
                match op.as_str() {
                    "+" | "-" => {
                        self.next();
                        let r = self.parse_unary();
                        l = if op == "+" { ops::add(l, r) } else { ops::sub(l, r) };
                        continue;
                    }
                    "*" => {
                        self.next();
                        let r = self.parse_unary();
                        l = ops::mul(l, r);
                        continue;
                    }
                    "/" => {
                        self.next();
                        let r = self.parse_unary();
                        l = ops::div(l, r);
                        continue;
                    }
                    _ => break,
                }
            }
            if Self::starts_primary(self.peek()) && !self.peek().space_before {
                let r = self.parse_power_level();
                l = ops::mul(l, r);
                continue;
            }
            break;
        }
        l
    }

    fn parse_args(&mut self, f: Option<&FnRef>) -> Vec<Expr> {
        let mut args = vec![];
        self.next();
        if self.peek().kind == TokKind::RParen {
            self.next();
            return args;
        }
        loop {
            let raw = f.map(|f| f.raw_args).unwrap_or(false);
            if raw && self.peek().kind == TokKind::Ident && self.p + 1 < self.t.len() && (self.t[self.p + 1].kind == TokKind::Sep || self.t[self.p + 1].kind == TokKind::RParen) {
                let t = self.next().text;
                args.push(str_e(&t));
            } else {
                args.push(self.parse_expr());
            }
            if self.peek().kind == TokKind::Sep {
                self.next();
                continue;
            }
            if self.peek().kind == TokKind::RowSep && !self.c.opts().comma_decimal {
                self.next();
                continue;
            }
            break;
        }
        if self.peek().kind == TokKind::RParen {
            self.next();
        } else if self.peek().kind != TokKind::End {
            throw("Expected ) after function arguments");
        }
        args
    }

    fn parse_ident(&mut self) -> Expr {
        let tk = self.next();
        let id = tk.text.clone();
        let call = self.peek().kind == TokKind::LParen && !self.peek().space_before;
        if self.local_symbols.contains(&id) {
            return if call {
                let r = self.parse_primary();
                ops::mul(sym(&id), r)
            } else {
                sym(&id)
            };
        }
        let c = self.c;
        if call {
            if let Some(f) = c.find_function(&id) {
                let a = self.parse_args(Some(&f));
                return make_call(&f, a);
            }
        }
        if let Some(v) = c.find_variable(&id) {
            return c.var_ref(&v);
        }
        if !call {
            if let Some((pu, pp)) = c.find_unit(&id, true) {
                if !(Self::starts_primary(self.peek()) && c.find_function(&id).is_some() && self.peek().kind != TokKind::Ident) {
                    return unit_e(pu, pp);
                }
            }
        }
        if let Some(f) = c.find_function(&id) {
            if self.peek().kind == TokKind::LParen {
                let a = self.parse_args(Some(&f));
                return make_call(&f, a);
            }
            if Self::starts_primary(self.peek()) {
                let arg = self.parse_function_arg_no_paren();
                return make_call(&f, vec![arg]);
            }
            if f.min_args == 0 {
                return make_call(&f, vec![]);
            }
            throw_s(format!("Function {}() needs arguments", id));
        }
        if let Some((u, prefix)) = c.find_unit(&id, true) {
            return unit_e(u, prefix);
        }
        if call {
            if let Some(ds) = c.find_data_set(&id) {
                let dsf = self.fnc("_dataset");
                let mut a = vec![str_e(&ds.item.name)];
                a.extend(self.parse_args(Some(&dsf)));
                return make_call(&dsf, a);
            }
        }
        if let Some(split) = self.try_split(&id) {
            return split;
        }
        let n = id.chars().count();
        if !c.opts().unknown_as_variables && n > 1 {
            throw_s(format!("Unknown name \"{}\"", id));
        }
        let s = sym(&id);
        if call {
            let r = self.parse_primary();
            return ops::mul(s, r);
        }
        s
    }

    fn parse_function_arg_no_paren(&mut self) -> Expr {
        let mut e = self.parse_unary();
        while Self::starts_primary(self.peek()) && !self.peek().space_before && self.peek().kind != TokKind::Ident {
            let r = self.parse_power_level();
            e = ops::mul(e, r);
        }
        if self.peek().kind == TokKind::Ident && !self.peek().space_before && Self::starts_primary(self.peek()) {
            let r = self.parse_power_level();
            e = ops::mul(e, r);
        }
        e
    }

    // split "xy" into x*y or "kWh" like names into known parts
    fn try_split(&self, id: &str) -> Option<Expr> {
        let chars: Vec<char> = id.chars().collect();
        if chars.len() < 2 || chars.len() > 12 {
            return None;
        }
        let mut parts = vec![];
        if self.split_rec(&chars, 0, &mut parts) {
            return Some(if parts.len() == 1 { parts.pop().unwrap() } else { mul_e(parts) });
        }
        None
    }

    fn split_rec(&self, id: &[char], pos: usize, parts: &mut Vec<Expr>) -> bool {
        if pos == id.len() {
            return parts.len() > 1;
        }
        for len in (1..=id.len() - pos).rev() {
            if pos == 0 && len == id.len() {
                continue;
            }
            let piece: String = id[pos..pos + len].iter().collect();
            let e = if piece.chars().all(is_digit) {
                Some(num_r(Real::parse(&piece, 10)))
            } else if let Some(v) = self.c.find_variable(&piece) {
                Some(self.c.var_ref(&v))
            } else if len == 1 && self.local_symbols.contains(&piece) {
                Some(sym(&piece))
            } else {
                match self.c.find_unit(&piece, false) {
                    Some((u, pr)) if len > 1 => Some(unit_e(u, pr)),
                    _ => None,
                }
            };
            let Some(e) = e else { continue };
            parts.push(e);
            if self.split_rec(id, pos + len, parts) {
                return true;
            }
            parts.pop();
        }
        false
    }

    // reverse polish syntax: "1 2 +"
    fn parse_rpn(&mut self) -> Option<Expr> {
        let mut st: Vec<Expr> = vec![];
        while self.peek().kind != TokKind::End {
            let tk = self.peek().clone();
            if tk.kind == TokKind::Op {
                self.next();
                if st.is_empty() {
                    throw("RPN stack is empty");
                }
                match tk.text.as_str() {
                    "!" => {
                        let a = st.pop().unwrap();
                        st.push(self.call("factorial", vec![a]));
                        continue;
                    }
                    "√" => {
                        let a = st.pop().unwrap();
                        st.push(self.call("sqrt", vec![a]));
                        continue;
                    }
                    "~" => {
                        let a = st.pop().unwrap();
                        st.push(self.call("bitnot", vec![a]));
                        continue;
                    }
                    _ => {}
                }
                if st.len() < 2 {
                    if tk.text == "-" {
                        let a = st.pop().unwrap();
                        st.push(ops::neg(a));
                        continue;
                    }
                    throw("Not enough values on the RPN stack");
                }
                let b = st.pop().unwrap();
                let a = st.pop().unwrap();
                let r = match tk.text.as_str() {
                    "+" => ops::add(a, b),
                    "-" => ops::sub(a, b),
                    "*" => ops::mul(a, b),
                    "/" => ops::div(a, b),
                    "^" => ops::pow(a, b),
                    "&" => self.call("bitand", vec![a, b]),
                    "|" => self.call("bitor", vec![a, b]),
                    "=" => cmp_e(CmpOp::Eq, a, b),
                    "<" => cmp_e(CmpOp::Lt, a, b),
                    ">" => cmp_e(CmpOp::Gt, a, b),
                    _ => throw_s(format!("Unsupported RPN operator {}", tk.text)),
                };
                st.push(r);
                continue;
            }
            if tk.kind == TokKind::Ident {
                if let Some(f) = self.c.find_function(&tk.text) {
                    if !(self.p + 1 < self.t.len() && self.t[self.p + 1].kind == TokKind::LParen) {
                        self.next();
                        let n = f.min_args.max(1) as usize;
                        if st.len() < n {
                            throw("Not enough values on the RPN stack");
                        }
                        let args = st.split_off(st.len() - n);
                        st.push(call_e(f, args));
                        continue;
                    }
                }
                if tk.text == "mod" {
                    self.next();
                    let b = st.pop().unwrap_or_else(|| throw("Not enough values on the RPN stack"));
                    let a = st.pop().unwrap_or_else(|| throw("Not enough values on the RPN stack"));
                    st.push(self.call("mod", vec![a, b]));
                    continue;
                }
            }
            let e = self.parse_postfix();
            st.push(e);
        }
        st.pop()
    }
}

fn is_percent_term(e: &Expr) -> Option<Expr> {
    if let E::Call(f, a) = &e.e {
        if f.name() == "_percent" {
            return Some(ops::div(a[0].clone(), num_i(100)));
        }
        if f.name() == "_permille" {
            return Some(ops::div(a[0].clone(), num_i(1000)));
        }
    }
    None
}

fn make_call(f: &FnRef, mut args: Vec<Expr>) -> Expr {
    if f.max_args >= 0 && args.len() as i32 > f.max_args {
        if f.max_args == 1 && args.len() > 1 {
            args = vec![vector_e(args)];
        } else {
            throw_s(format!("Too many arguments for {}()", f.item.names[0]));
        }
    }
    call_e(f.clone(), args)
}