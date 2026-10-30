using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Numerics;
using System.Text;
using System.Text.RegularExpressions;

namespace OrangCalc.Engine
{
    public enum TokKind { Num, Ident, Str, Op, LParen, RParen, LBrack, RBrack, Sep, RowSep, Date, End }

    public sealed class Token
    {
        public TokKind Kind;
        public string Text;
        public Expr Value;
        public bool SpaceBefore;
        public int Pos;
        public override string ToString() => Kind + ":" + Text;
    }

    public sealed class Lexer
    {
        readonly string s;
        readonly Calculator c;
        int i;
        public readonly List<Token> Tokens = new List<Token>();

        static readonly string[] Ops =
        {
            ":=", "**", "//", "<<", ">>", "<=", ">=", "!=", "==", "&&", "||", "->", "!!",
            "+", "-", "*", "/", "^", "(", ")", "=", "<", ">", "!", "%", "&", "|", "~",
            "×", "·", "÷", "−", "∕", "⋅", "≠", "≤", "≥", "√", "∛", "∜", "‰", "‱", "∠", "±", "→", "⊻", "¬", "∧", "∨", "∙"
        };

        static readonly Dictionary<char, string> Superscripts = new Dictionary<char, string>
        {
            {'⁰',"0"},{'¹',"1"},{'²',"2"},{'³',"3"},{'⁴',"4"},{'⁵',"5"},{'⁶',"6"},{'⁷',"7"},{'⁸',"8"},{'⁹',"9"},{'⁻',"-"},{'⁺',"+"}
        };

        static readonly Dictionary<char, Real> Vulgar = new Dictionary<char, Real>
        {
            {'½', Real.Frac(1,2)},{'⅓', Real.Frac(1,3)},{'⅔', Real.Frac(2,3)},{'¼', Real.Frac(1,4)},{'¾', Real.Frac(3,4)},
            {'⅕', Real.Frac(1,5)},{'⅖', Real.Frac(2,5)},{'⅗', Real.Frac(3,5)},{'⅘', Real.Frac(4,5)},{'⅙', Real.Frac(1,6)},
            {'⅚', Real.Frac(5,6)},{'⅐', Real.Frac(1,7)},{'⅛', Real.Frac(1,8)},{'⅜', Real.Frac(3,8)},{'⅝', Real.Frac(5,8)},
            {'⅞', Real.Frac(7,8)},{'⅑', Real.Frac(1,9)},{'⅒', Real.Frac(1,10)}
        };

        public Lexer(string text, Calculator calc)
        {
            s = text; c = calc;
            Run();
        }

        public static bool IsIdentStart(char ch) =>
            char.IsLetter(ch) || ch == '_' || ch == '°' || ch == 'µ' || ch == 'Ω' || ch == '℃' || ch == '℉' || ch == 'Å' || ch == '∞' || ch == '℧' || ch == '$' || ch == '€' || ch == '£' || ch == '¥' || ch == '₿' || ch == '₹' || ch == '₽' || ch == '₩' || ch == '¢' || ch == '∑' || ch == '∏' || ch == '∫' || ch == 'ℏ' || ch == '∂';

        public static bool IsIdentPart(char ch) => IsIdentStart(ch) || char.IsDigit(ch);

        bool CommaDecimal => c.Opts.CommaDecimal;

        void Add(TokKind k, string text, bool space, int pos, Expr v = null)
        {
            Tokens.Add(new Token { Kind = k, Text = text, SpaceBefore = space, Pos = pos, Value = v });
        }

        static readonly Regex DateRx = new Regex(@"^\d{4}-\d{2}-\d{2}([T ]\d{1,2}:\d{2}(:\d{2}(\.\d+)?)?)?(Z|[+-]\d{2}:?\d{2})?", RegexOptions.Compiled);
        static readonly Regex SexaRx = new Regex(@"^(\d+(?:\.\d+)?)°(?:(\d+(?:\.\d+)?)['′](?:(\d+(?:\.\d+)?)[""″])?)?", RegexOptions.Compiled);
        static readonly Regex TimeRx = new Regex(@"^(\d+):(\d{1,2})(?::(\d{1,2}(?:\.\d+)?))?(?![\d:])", RegexOptions.Compiled);

        void Run()
        {
            int n = s.Length;
            while (i < n)
            {
                bool space = false;
                while (i < n && char.IsWhiteSpace(s[i]) ) { i++; space = true; }
                if (i >= n) break;
                char ch = s[i];
                int start = i;
                if (ch == '#') break;
                if (ch == '"' || ch == '\'' || ch == '“' || ch == '‘')
                {
                    char close = ch == '“' ? '”' : ch == '‘' ? '’' : ch;
                    int j = s.IndexOf(close, i + 1);
                    if (j < 0) j = n;
                    Add(TokKind.Str, s.Substring(i + 1, j - i - 1), space, start);
                    i = Math.Min(n, j + 1);
                    continue;
                }
                if (char.IsDigit(ch) || (ch == '.' && i + 1 < n && char.IsDigit(s[i + 1])) || (ch == ',' && CommaDecimal && i + 1 < n && char.IsDigit(s[i + 1])))
                {
                    LexNumber(space);
                    continue;
                }
                if (Vulgar.TryGetValue(ch, out var vf))
                {
                    Add(TokKind.Num, ch.ToString(), space, start, Expr.N(vf));
                    i++;
                    continue;
                }
                if (Superscripts.ContainsKey(ch))
                {
                    var sb = new StringBuilder();
                    while (i < n && Superscripts.TryGetValue(s[i], out var d)) { sb.Append(d); i++; }
                    Add(TokKind.Op, "^", false, start);
                    var t = sb.ToString();
                    if (t.StartsWith("-")) { Add(TokKind.Op, "-", false, start); t = t.Substring(1); }
                    if (t.StartsWith("+")) t = t.Substring(1);
                    Add(TokKind.Num, t, false, start, Expr.N(Real.Parse(t.Length == 0 ? "1" : t)));
                    continue;
                }
                if (ch == '\\' && i + 1 < n && char.IsLetter(s[i + 1]))
                {
                    char a = s[i + 1];
                    int idx = a >= 'x' && a <= 'z' ? a - 'x' : a - 'a' + 3;
                    Add(TokKind.Num, "\\" + a, space, start, new ArgE(idx));
                    i += 2;
                    continue;
                }
                if (IsIdentStart(ch))
                {
                    int j = i + 1;
                    bool single = ch == '∑' || ch == '∏' || ch == '∫' || ch == '∞' || ch == '∂';
                    if (!single) while (j < n && IsIdentPart(s[j]) && s[j] != '∞' && s[j] != '∑' && s[j] != '∏' && s[j] != '∫') j++;
                    Add(TokKind.Ident, s.Substring(i, j - i), space, start);
                    i = j;
                    continue;
                }
                if (ch == '(') { Add(TokKind.LParen, "(", space, start); i++; continue; }
                if (ch == ')') { Add(TokKind.RParen, ")", space, start); i++; continue; }
                if (ch == '[' || ch == '{') { Add(TokKind.LBrack, "[", space, start); i++; continue; }
                if (ch == ']' || ch == '}') { Add(TokKind.RBrack, "]", space, start); i++; continue; }
                if (ch == ';') { Add(CommaDecimal ? TokKind.Sep : TokKind.RowSep, ";", space, start); i++; continue; }
                if (ch == ',')
                {
                    if (c.Opts.IgnoreComma) { i++; continue; }
                    Add(TokKind.Sep, ",", space, start); i++; continue;
                }
                if (ch == '′' || ch == '″')
                {
                    Add(TokKind.Ident, ch == '′' ? "arcmin" : "arcsec", false, start);
                    i++;
                    continue;
                }
                string op = null;
                foreach (var o in Ops)
                    if (string.CompareOrdinal(s, i, o, 0, o.Length) == 0) { op = o; break; }
                if (op != null)
                {
                    i += op.Length;
                    Add(TokKind.Op, NormalizeOp(op), space, start);
                    continue;
                }
                throw new CalcException("Unexpected character '" + ch + "'");
            }
            Add(TokKind.End, "", false, n);
        }

        static string NormalizeOp(string op)
        {
            switch (op)
            {
                case "×": case "·": case "⋅": case "∙": return "*";
                case "÷": case "∕": return "/";
                case "−": return "-";
                case "**": return "^";
                case "≠": return "!=";
                case "≤": return "<=";
                case "≥": return ">=";
                case "==": return "=";
                case "→": case "->": return "→";
                case "∧": return "&&";
                case "∨": return "||";
                case "¬": return "NOT";
                default: return op;
            }
        }

        void LexNumber(bool space)
        {
            int n = s.Length, start = i;
            string rest = s.Substring(i);
            var dm = DateRx.Match(rest);
            if (dm.Success && c.Opts.InputBase == 10)
            {
                var d = Funcs.ParseDate(dm.Value, out bool hasTime, out bool utc);
                Add(TokKind.Date, dm.Value, space, start, new DateE(d, hasTime, utc));
                i += dm.Length;
                return;
            }
            var sm = SexaRx.Match(rest);
            if (sm.Success && sm.Groups[2].Success && c.Opts.InputBase == 10)
            {
                Real v = ParseDec(sm.Groups[1].Value);
                v += ParseDec(sm.Groups[2].Value) / Real.Int(60);
                if (sm.Groups[3].Success) v += ParseDec(sm.Groups[3].Value) / Real.Int(3600);
                i += sm.Length;
                Add(TokKind.Num, sm.Value, space, start, Expr.N(v));
                Add(TokKind.Ident, "°", false, start);
                return;
            }
            var tm = TimeRx.Match(rest);
            if (tm.Success && c.Opts.InputBase == 10)
            {
                Real v = ParseDec(tm.Groups[1].Value) + ParseDec(tm.Groups[2].Value) / Real.Int(60);
                if (tm.Groups[3].Success) v += ParseDec(tm.Groups[3].Value) / Real.Int(3600);
                i += tm.Length;
                Add(TokKind.Num, tm.Value, space, start, Expr.N(v));
                return;
            }
            int radix = c.Opts.InputBase;
            if (s[i] == '0' && i + 1 < n)
            {
                char p = char.ToLowerInvariant(s[i + 1]);
                if (p == 'x' && i + 2 < n && Uri.IsHexDigit(s[i + 2])) { radix = 16; i += 2; }
                else if (p == 'b' && i + 2 < n && (s[i + 2] == '0' || s[i + 2] == '1')) { radix = 2; i += 2; }
                else if (p == 'o' && i + 2 < n && char.IsDigit(s[i + 2])) { radix = 8; i += 2; }
                else if (p == 'd' && i + 2 < n && char.IsDigit(s[i + 2])) { radix = 12; i += 2; }
            }
            if (radix < 2 || radix > 36) radix = 10;
            var sb = new StringBuilder();
            bool dot = false;
            char decChar = CommaDecimal ? ',' : '.';
            while (i < n)
            {
                char ch = s[i];
                if (ch == decChar && !dot && !(CommaDecimal && c.Opts.IgnoreDots && ch == '.'))
                {
                    if (i + 1 < n && s[i + 1] == decChar) break;
                    dot = true; sb.Append('.'); i++; continue;
                }
                if (ch == '.' && CommaDecimal && c.Opts.IgnoreDots) { i++; continue; }
                if (ch == ',' && !CommaDecimal && c.Opts.IgnoreComma) { i++; continue; }
                if (ch == ' ' && radix != 10 && i + 1 < n && Real.DigitValue(s[i + 1]) >= 0 && Real.DigitValue(s[i + 1]) < radix && sb.Length > 0 && (radix == 2 || radix == 16)) { i++; continue; }
                int dv = Real.DigitValue(ch);
                if (radix > 10 && dv >= 10 && dv < radix && char.IsLetter(ch))
                {
                    int j = i;
                    while (j < n && char.IsLetterOrDigit(s[j])) j++;
                    string w = s.Substring(i, j - i);
                    if (!w.All(x => Real.DigitValue(x) >= 0 && Real.DigitValue(x) < radix)) break;
                }
                if (dv < 0 || dv >= radix || (radix <= 10 && !char.IsDigit(ch)))
                {
                    if (radix == 12 && (ch == '↊' || ch == '↋')) { sb.Append(ch); i++; continue; }
                    break;
                }
                sb.Append(ch); i++;
            }
            Real val = Real.Parse(sb.ToString(), radix);
            bool approx = c.Opts.ReadPrecision && dot;
            int sigDigits = sb.ToString().Replace(".", "").TrimStart('0').Length;
            if (i < n && (s[i] == 'e' || s[i] == 'E') && radix <= 10)
            {
                int j = i + 1;
                if (j < n && (s[j] == '+' || s[j] == '-' || s[j] == '−')) j++;
                if (j < n && char.IsDigit(s[j]))
                {
                    int k = j;
                    while (k < n && char.IsDigit(s[k])) k++;
                    string es = s.Substring(i + 1, k - i - 1).Replace('−', '-');
                    int ex = int.Parse(es, CultureInfo.InvariantCulture);
                    val = val * Real.Int(10).PowInt(ex);
                    i = k;
                }
            }
            Expr value = Expr.N(approx ? val.AsApprox() : val);
            if (c.Opts.ConciseUncertaintyInput && i < n && s[i] == '(' && dot)
            {
                int j = s.IndexOf(')', i);
                if (j > i + 1 && s.Substring(i + 1, j - i - 1).All(char.IsDigit))
                {
                    string u = s.Substring(i + 1, j - i - 1);
                    int decs = sb.ToString().Length - sb.ToString().IndexOf('.') - 1;
                    Real err = Real.Parse(u) / Real.Int(10).PowInt(decs);
                    value = new IntvE(val - err, val + err);
                    i = j + 1;
                }
            }
            if (approx && value is NumE && sigDigits > 0)
            {
                var ne = (NumE)value;
                value = new NumE(ne.V);
            }
            Add(TokKind.Num, s.Substring(start, i - start), space, start, value);
        }

        Real ParseDec(string t) => Real.Parse(t);
    }

    public sealed class ParseResult
    {
        public Expr Expr;
        public List<string> Warnings = new List<string>();
    }

    public sealed class Parser
    {
        readonly List<Token> t;
        int p;
        readonly Calculator c;
        readonly ParseMode mode;
        public readonly List<string> Warnings = new List<string>();
        public HashSet<string> LocalSymbols = new HashSet<string>();

        public Parser(string text, Calculator calc)
        {
            c = calc;
            mode = calc.Opts.Parse;
            t = new Lexer(text, calc).Tokens;
        }

        public static Expr ParseText(string text, Calculator calc, List<string> warnings = null)
        {
            var ps = new Parser(text, calc);
            var e = ps.ParseAll();
            warnings?.AddRange(ps.Warnings);
            return e;
        }

        Token Peek => t[p];
        Token Next() => t[p++];
        bool IsOp(string op) => Peek.Kind == TokKind.Op && Peek.Text == op;
        bool IsWord(string w) => Peek.Kind == TokKind.Ident && string.Equals(Peek.Text, w, StringComparison.OrdinalIgnoreCase);

        void Expect(TokKind k, string what)
        {
            if (Peek.Kind != k) throw new CalcException("Expected " + what);
            p++;
        }

        public Expr ParseAll()
        {
            if (Peek.Kind == TokKind.End) return null;
            Expr e = mode == ParseMode.Rpn ? ParseRpn() : ParseExpr();
            if (Peek.Kind != TokKind.End)
            {
                if (Peek.Kind == TokKind.RParen) { Warnings.Add("Unmatched right parenthesis ignored"); }
                else throw new CalcException("Unexpected \"" + Peek.Text + "\"");
            }
            return e;
        }

        Expr ParseExpr() => mode == ParseMode.Chain ? ParseChain() : ParseOr();

        // chain mode: operators applied left to right
        Expr ParseChain()
        {
            Expr left = ParseUnary();
            while (true)
            {
                if (Peek.Kind != TokKind.Op) break;
                string op = Peek.Text;
                if (op == "^") { Next(); left = Ops.Pow(left, ParseUnary()); continue; }
                if (op == "+" || op == "-" || op == "*" || op == "/")
                {
                    Next();
                    var r = ParseUnary();
                    left = op == "+" ? Ops.Add(left, r) : op == "-" ? Ops.Sub(left, r) : op == "*" ? Ops.Mul(left, r) : Ops.Div(left, r);
                    continue;
                }
                break;
            }
            return left;
        }

        Expr ParseOr()
        {
            var l = ParseXorLogical();
            while (IsOp("||") || IsWord("or"))
            {
                Next();
                l = new LogicE(LogicOp.Or, new List<Expr> { l, ParseXorLogical() });
            }
            return l;
        }

        Expr ParseXorLogical()
        {
            var l = ParseAnd();
            while (IsWord("xor"))
            {
                Next();
                l = new CallE(c.Fn("xor"), l, ParseAnd());
            }
            return l;
        }

        Expr ParseAnd()
        {
            var l = ParseNot();
            while (IsOp("&&") || IsWord("and"))
            {
                Next();
                l = new LogicE(LogicOp.And, new List<Expr> { l, ParseNot() });
            }
            return l;
        }

        Expr ParseNot()
        {
            if (IsWord("not") || IsOp("NOT"))
            {
                Next();
                return new LogicE(LogicOp.Not, new List<Expr> { ParseNot() });
            }
            return ParseCmp();
        }

        Expr ParseCmp()
        {
            var l = ParseBitOr();
            while (Peek.Kind == TokKind.Op)
            {
                CmpOp op;
                switch (Peek.Text)
                {
                    case "=": op = CmpOp.Eq; break;
                    case "!=": op = CmpOp.Ne; break;
                    case "<": op = CmpOp.Lt; break;
                    case ">": op = CmpOp.Gt; break;
                    case "<=": op = CmpOp.Le; break;
                    case ">=": op = CmpOp.Ge; break;
                    default: return l;
                }
                Next();
                var r = ParseBitOr();
                if (l is CmpE lc && op != CmpOp.Eq && op != CmpOp.Ne)
                    l = new LogicE(LogicOp.And, new List<Expr> { l, new CmpE(op, lc.R, r) });
                else l = new CmpE(op, l, r);
            }
            return l;
        }

        Expr ParseBitOr()
        {
            var l = ParseBitXor();
            while (IsOp("|"))
            {
                Next();
                l = new CallE(c.Fn("bitor"), l, ParseBitXor());
            }
            return l;
        }

        Expr ParseBitXor()
        {
            var l = ParseBitAnd();
            while (IsOp("⊻"))
            {
                Next();
                l = new CallE(c.Fn("bitxor"), l, ParseBitAnd());
            }
            return l;
        }

        Expr ParseBitAnd()
        {
            var l = ParseShift();
            while (IsOp("&"))
            {
                Next();
                l = new CallE(c.Fn("bitand"), l, ParseShift());
            }
            return l;
        }

        Expr ParseShift()
        {
            var l = ParseAdd();
            while (IsOp("<<") || IsOp(">>"))
            {
                bool left = Next().Text == "<<";
                var r = ParseAdd();
                l = new CallE(c.Fn("shift"), l, left ? r : Ops.Neg(r));
            }
            return l;
        }

        Expr ParseAdd()
        {
            Expr l;
            l = ParseMul();
            while (true)
            {
                if (IsOp("+") || IsOp("-") || IsWord("plus") || IsWord("minus"))
                {
                    var tk = Next();
                    bool minus = tk.Text == "-" || tk.Text.Equals("minus", StringComparison.OrdinalIgnoreCase);
                    var r = ParseMul();
                    if (c.Opts.SimplifiedPercent && IsPercentTerm(r, out var pct))
                    {
                        var factor = minus ? Ops.Sub(Expr.One, pct) : Ops.Add(Expr.One, pct);
                        l = Ops.Mul(l, factor);
                    }
                    else l = minus ? Ops.Sub(l, r) : Ops.Add(l, r);
                }
                else if (IsOp("±"))
                {
                    Next();
                    var r = ParseMul();
                    l = new CallE(c.Fn("uncertainty"), l, r);
                }
                else break;
            }
            return l;
        }

        static bool IsPercentTerm(Expr e, out Expr pct)
        {
            pct = null;
            if (e is CallE ce && ce.F.Name == "_percent") { pct = Ops.Div(ce.Args[0], Expr.N(100)); return true; }
            if (e is CallE pe && pe.F.Name == "_permille") { pct = Ops.Div(pe.Args[0], Expr.N(1000)); return true; }
            return false;
        }

        bool StartsPrimary(Token tk)
        {
            switch (tk.Kind)
            {
                case TokKind.Num: case TokKind.LParen: case TokKind.LBrack: case TokKind.Str: case TokKind.Date: return true;
                case TokKind.Ident:
                    var w = tk.Text.ToLowerInvariant();
                    return !(w == "mod" || w == "rem" || w == "and" || w == "or" || w == "xor" || w == "not" || w == "to" || w == "plus" || w == "minus" || w == "times" || w == "per" || w == "of" || w == "where");
                case TokKind.Op: return tk.Text == "√" || tk.Text == "∛" || tk.Text == "∜";
            }
            return false;
        }

        Expr ParseMul()
        {
            Expr l = mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit();
            while (true)
            {
                if (IsOp("*") || IsWord("times") || IsWord("of"))
                {
                    Next();
                    l = Ops.Mul(l, mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit());
                }
                else if (IsOp("/") || IsWord("per"))
                {
                    Next();
                    l = Ops.Div(l, mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit());
                }
                else if (IsOp("//"))
                {
                    Next();
                    l = new CallE(c.Fn("div"), l, mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit());
                }
                else if (IsWord("mod") || (IsOp("%") && p + 1 < t.Count && StartsPrimary(t[p + 1]) && !t[p + 1].SpaceBefore == false && t[p].SpaceBefore))
                {
                    Next();
                    l = new CallE(c.Fn("mod"), l, mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit());
                }
                else if (IsWord("rem"))
                {
                    Next();
                    l = new CallE(c.Fn("rem"), l, mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit());
                }
                else if (IsOp("∠"))
                {
                    Next();
                    var ang = mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit();
                    l = new CallE(c.Fn("_angle"), l, ang);
                }
                else if (StartsPrimary(Peek) && (mode == ParseMode.Conventional || Peek.SpaceBefore))
                {
                    l = Ops.Mul(l, mode == ParseMode.Conventional ? ParseUnary() : ParseImplicit());
                }
                else break;
            }
            return l;
        }

        // implicit multiplication chain, tighter than explicit operators
        Expr ParseImplicit()
        {
            Expr l = ParseUnary();
            while (StartsPrimary(Peek))
            {
                if (mode == ParseMode.Adaptive && Peek.SpaceBefore) break;
                if (c.Opts.LimitImplicitMult && l is MulE) break;
                l = Ops.Mul(l, ParsePowerLevel());
            }
            return l;
        }

        Expr ParseUnary()
        {
            if (IsOp("-")) { Next(); return Ops.Neg(ParseUnary()); }
            if (IsOp("+")) { Next(); return ParseUnary(); }
            if (IsOp("~")) { Next(); return new CallE(c.Fn("bitnot"), ParseUnary()); }
            if (IsOp("!") || IsOp("NOT")) { Next(); return new LogicE(LogicOp.Not, new List<Expr> { ParseUnary() }); }
            if (IsOp("√")) { Next(); return new CallE(c.Fn("sqrt"), ParseUnary()); }
            if (IsOp("∛")) { Next(); return new CallE(c.Fn("cbrt"), ParseUnary()); }
            if (IsOp("∜")) { Next(); return new CallE(c.Fn("root"), ParseUnary(), Expr.N(4)); }
            return ParsePowerLevel();
        }

        Expr ParsePowerLevel()
        {
            var b = ParsePostfix();
            if (IsOp("^"))
            {
                Next();
                if (c.Opts.CaretXor) return new CallE(c.Fn("bitxor"), b, ParseUnary());
                Expr ex;
                if (IsOp("-")) { Next(); ex = Ops.Neg(ParsePowerExp()); }
                else if (IsOp("+")) { Next(); ex = ParsePowerExp(); }
                else ex = ParsePowerExp();
                return Ops.Pow(b, ex);
            }
            return b;
        }

        Expr ParsePowerExp()
        {
            if (IsOp("√") || IsOp("∛")) return ParseUnary();
            return ParsePowerLevel();
        }

        Expr ParsePostfix()
        {
            var e = ParsePrimary();
            while (true)
            {
                if (IsOp("!!") && !NextStartsOperand()) { Next(); e = new CallE(c.Fn("factorial2"), e); }
                else if (IsOp("!") && !NextStartsOperand()) { Next(); e = new CallE(c.Fn("factorial"), e); }
                else if (IsOp("%") && !(p + 1 < t.Count && StartsPrimary(t[p + 1]) && t[p].SpaceBefore))
                {
                    Next();
                    e = new CallE(c.Fn("_percent"), e);
                }
                else if (IsOp("‰")) { Next(); e = new CallE(c.Fn("_permille"), e); }
                else if (IsOp("‱")) { Next(); e = Ops.Div(e, Expr.N(10000)); }
                else break;
            }
            return e;
        }

        bool NextStartsOperand()
        {
            if (p + 1 >= t.Count) return false;
            var n = t[p + 1];
            return n.Kind == TokKind.Op && n.Text == "=";
        }

        Expr ParsePrimary()
        {
            var tk = Peek;
            switch (tk.Kind)
            {
                case TokKind.Num: Next(); return tk.Value;
                case TokKind.Date: Next(); return tk.Value;
                case TokKind.Str: Next(); return new StrE(tk.Text);
                case TokKind.LParen:
                    {
                        Next();
                        if (Peek.Kind == TokKind.RParen) { Next(); return MatE.Vector(new List<Expr>()); }
                        var e = ParseExpr();
                        if (Peek.Kind == TokKind.Sep)
                        {
                            var items = new List<Expr> { e };
                            while (Peek.Kind == TokKind.Sep) { Next(); items.Add(ParseExpr()); }
                            e = MatE.Vector(items);
                        }
                        if (Peek.Kind == TokKind.RParen) Next();
                        else if (Peek.Kind != TokKind.End) throw new CalcException("Expected )");
                        return e;
                    }
                case TokKind.LBrack: return ParseMatrix();
                case TokKind.Ident: return ParseIdent();
                case TokKind.End: throw new CalcException("Unexpected end of expression");
                default: throw new CalcException("Unexpected \"" + tk.Text + "\"");
            }
        }

        Expr ParseMatrix()
        {
            Next();
            bool separated = false;
            int dep = 0;
            for (int k = p; k < t.Count; k++)
            {
                var tk = t[k];
                if (tk.Kind == TokKind.LParen || tk.Kind == TokKind.LBrack) dep++;
                else if (tk.Kind == TokKind.RParen || tk.Kind == TokKind.RBrack) { if (dep == 0) break; dep--; }
                else if (dep == 0 && (tk.Kind == TokKind.Sep || tk.Kind == TokKind.RowSep)) { separated = true; break; }
                else if (tk.Kind == TokKind.End) break;
            }
            var rows = new List<List<Expr>>();
            var row = new List<Expr>();
            bool sawRowSep = false;
            while (Peek.Kind != TokKind.RBrack && Peek.Kind != TokKind.End)
            {
                if (Peek.Kind == TokKind.Sep) { Next(); continue; }
                if (Peek.Kind == TokKind.RowSep) { Next(); rows.Add(row); row = new List<Expr>(); sawRowSep = true; continue; }
                row.Add(separated ? ParseExpr() : ParseMatrixItem());
            }
            if (Peek.Kind == TokKind.RBrack) Next();
            rows.Add(row);
            if (!sawRowSep)
            {
                var items = rows[0];
                if (items.Count > 0 && items.All(x => x is MatE m && m.IsVector) && items.Select(x => ((MatE)x).ColCount).Distinct().Count() == 1)
                    return new MatE(items.Select(x => ((MatE)x).Rows[0]).ToList(), false);
                return MatE.Vector(items);
            }
            int cols = rows.Max(r => r.Count);
            foreach (var r in rows) while (r.Count < cols) r.Add(Expr.Zero);
            return new MatE(rows, false);
        }

        // space separated matrix items, no implicit multiplication across spaces
        Expr ParseMatrixItem()
        {
            int depth = 0;
            int save = p;
            bool hasSep = false;
            for (int k = p; k < t.Count; k++)
            {
                var tk = t[k];
                if (tk.Kind == TokKind.LParen || tk.Kind == TokKind.LBrack) depth++;
                else if (tk.Kind == TokKind.RParen || tk.Kind == TokKind.RBrack) { if (depth == 0) break; depth--; }
                else if (depth == 0 && (tk.Kind == TokKind.Sep || tk.Kind == TokKind.RowSep)) { hasSep = true; break; }
            }
            if (hasSep) return ParseExpr();
            var l = ParseUnary();
            while (true)
            {
                if (Peek.Kind == TokKind.Op)
                {
                    string op = Peek.Text;
                    bool nextTight = p + 1 < t.Count && !t[p + 1].SpaceBefore;
                    if ((op == "+" || op == "-") && Peek.SpaceBefore && nextTight) break;
                    if (op == "+" || op == "-") { Next(); var r = ParseUnary(); l = op == "+" ? Ops.Add(l, r) : Ops.Sub(l, r); continue; }
                    if (op == "*") { Next(); l = Ops.Mul(l, ParseUnary()); continue; }
                    if (op == "/") { Next(); l = Ops.Div(l, ParseUnary()); continue; }
                    break;
                }
                if (StartsPrimary(Peek) && !Peek.SpaceBefore) { l = Ops.Mul(l, ParsePowerLevel()); continue; }
                break;
            }
            return l;
        }

        List<Expr> ParseArgs(MathFunction f = null)
        {
            var args = new List<Expr>();
            Next();
            if (Peek.Kind == TokKind.RParen) { Next(); return args; }
            while (true)
            {
                if (f != null && f.RawArgs && Peek.Kind == TokKind.Ident && p + 1 < t.Count && (t[p + 1].Kind == TokKind.Sep || t[p + 1].Kind == TokKind.RParen))
                    args.Add(new StrE(Next().Text));
                else args.Add(ParseExpr());
                if (Peek.Kind == TokKind.Sep) { Next(); continue; }
                if (Peek.Kind == TokKind.RowSep && !c.Opts.CommaDecimal) { Next(); continue; }
                break;
            }
            if (Peek.Kind == TokKind.RParen) Next();
            else if (Peek.Kind != TokKind.End) throw new CalcException("Expected ) after function arguments");
            return args;
        }

        Expr ParseIdent()
        {
            var tk = Next();
            string id = tk.Text;
            bool call = Peek.Kind == TokKind.LParen && !Peek.SpaceBefore;
            if (LocalSymbols.Contains(id)) return call ? Ops.Mul(new SymE(id), ParsePrimary()) : new SymE(id);

            if (call)
            {
                var f = c.FindFunction(id);
                if (f != null) return MakeCall(f, ParseArgs(f));
            }
            var v = c.FindVariable(id);
            if (v != null) return c.VarRef(v);
            if (!call)
            {
                var pu = c.FindUnit(id, out var pp);
                if (pu != null && !(StartsPrimary(Peek) && c.FindFunction(id) != null && Peek.Kind != TokKind.Ident)) return new UnitE(pu, pp);
            }
            var fn = c.FindFunction(id);
            if (fn != null)
            {
                if (Peek.Kind == TokKind.LParen) return MakeCall(fn, ParseArgs(fn));
                if (StartsPrimary(Peek))
                {
                    var arg = ParseFunctionArgNoParen();
                    return MakeCall(fn, new List<Expr> { arg });
                }
                if (fn.MinArgs == 0) return MakeCall(fn, new List<Expr>());
                throw new CalcException("Function " + id + "() needs arguments");
            }
            var u = c.FindUnit(id, out var prefix);
            if (u != null) return new UnitE(u, prefix);
            var ds = c.FindDataSet(id);
            if (ds != null && call) return MakeCall(c.Fn("_dataset"), new List<Expr> { new StrE(ds.Name) }.Concat(ParseArgs(c.Fn("_dataset"))).ToList());

            var split = TrySplit(id);
            if (split != null) return split;

            if (!c.Opts.UnknownAsVariables && id.Length > 1)
                throw new CalcException("Unknown name \"" + id + "\"");
            if (id.Length > 1 && !c.Opts.UnknownAsVariables) Warnings.Add("\"" + id + "\" was interpreted as an unknown variable");
            var sym = new SymE(id);
            if (call) return Ops.Mul(sym, ParsePrimary());
            return sym;
        }

        Expr ParseFunctionArgNoParen()
        {
            var e = ParseUnary();
            while (StartsPrimary(Peek) && !Peek.SpaceBefore && Peek.Kind != TokKind.Ident) e = Ops.Mul(e, ParsePowerLevel());
            if (Peek.Kind == TokKind.Ident && !Peek.SpaceBefore && StartsPrimary(Peek)) e = Ops.Mul(e, ParsePowerLevel());
            return e;
        }

        // split "xy" into x*y or "kWh" like names into known parts
        Expr TrySplit(string id)
        {
            if (id.Length < 2 || id.Length > 12) return null;
            var parts = new List<Expr>();
            if (SplitRec(id, 0, parts)) return parts.Count == 1 ? parts[0] : new MulE(parts);
            return null;
        }

        bool SplitRec(string id, int pos, List<Expr> parts)
        {
            if (pos == id.Length) return parts.Count > 1;
            for (int len = id.Length - pos; len >= 1; len--)
            {
                if (pos == 0 && len == id.Length) continue;
                string piece = id.Substring(pos, len);
                Expr e = null;
                if (piece.All(char.IsDigit)) e = Expr.N(Real.Parse(piece));
                else
                {
                    var v = c.FindVariable(piece);
                    if (v != null) e = c.VarRef(v);
                    else if (len == 1 && LocalSymbols.Contains(piece)) e = new SymE(piece);
                    else
                    {
                        var u = c.FindUnit(piece, out var pr, false);
                        if (u != null && len > 1) e = new UnitE(u, pr);
                    }
                }
                if (e == null) continue;
                parts.Add(e);
                if (SplitRec(id, pos + len, parts)) return true;
                parts.RemoveAt(parts.Count - 1);
            }
            return false;
        }

        Expr MakeCall(MathFunction f, List<Expr> args)
        {
            if (f.MaxArgs >= 0 && args.Count > f.MaxArgs)
            {
                if (f.MaxArgs == 1 && args.Count > 1) args = new List<Expr> { MatE.Vector(args) };
                else throw new CalcException("Too many arguments for " + f.Names[0] + "()");
            }
            return new CallE(f, args);
        }

        // reverse polish syntax: "1 2 +"
        Expr ParseRpn()
        {
            var st = new Stack<Expr>();
            while (Peek.Kind != TokKind.End)
            {
                var tk = Peek;
                if (tk.Kind == TokKind.Op)
                {
                    Next();
                    if (st.Count == 0) throw new CalcException("RPN stack is empty");
                    if (tk.Text == "!") { st.Push(new CallE(c.Fn("factorial"), st.Pop())); continue; }
                    if (tk.Text == "√") { st.Push(new CallE(c.Fn("sqrt"), st.Pop())); continue; }
                    if (tk.Text == "~") { st.Push(new CallE(c.Fn("bitnot"), st.Pop())); continue; }
                    if (st.Count < 2) { if (tk.Text == "-") { st.Push(Ops.Neg(st.Pop())); continue; } throw new CalcException("Not enough values on the RPN stack"); }
                    var b = st.Pop(); var a = st.Pop();
                    switch (tk.Text)
                    {
                        case "+": st.Push(Ops.Add(a, b)); break;
                        case "-": st.Push(Ops.Sub(a, b)); break;
                        case "*": st.Push(Ops.Mul(a, b)); break;
                        case "/": st.Push(Ops.Div(a, b)); break;
                        case "^": st.Push(Ops.Pow(a, b)); break;
                        case "&": st.Push(new CallE(c.Fn("bitand"), a, b)); break;
                        case "|": st.Push(new CallE(c.Fn("bitor"), a, b)); break;
                        case "=": st.Push(new CmpE(CmpOp.Eq, a, b)); break;
                        case "<": st.Push(new CmpE(CmpOp.Lt, a, b)); break;
                        case ">": st.Push(new CmpE(CmpOp.Gt, a, b)); break;
                        default: throw new CalcException("Unsupported RPN operator " + tk.Text);
                    }
                    continue;
                }
                if (tk.Kind == TokKind.Ident)
                {
                    var f = c.FindFunction(tk.Text);
                    if (f != null && !(p + 1 < t.Count && t[p + 1].Kind == TokKind.LParen))
                    {
                        Next();
                        int n = Math.Max(1, f.MinArgs);
                        if (st.Count < n) throw new CalcException("Not enough values on the RPN stack");
                        var args = new List<Expr>();
                        for (int k = 0; k < n; k++) args.Insert(0, st.Pop());
                        st.Push(new CallE(f, args));
                        continue;
                    }
                    if (tk.Text == "mod") { Next(); var b2 = st.Pop(); var a2 = st.Pop(); st.Push(new CallE(c.Fn("mod"), a2, b2)); continue; }
                }
                st.Push(ParsePostfix());
            }
            if (st.Count == 0) return null;
            if (st.Count == 1) return st.Pop();
            var list = st.Reverse().ToList();
            return list.Last();
        }
    }
}
