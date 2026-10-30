using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Numerics;
using System.Text;

namespace OrangCalc.Engine
{
    public enum TokType { Plain, Number, Operator, Variable, Function, Unit, Symbol, Paren, Text, Error }

    public struct OutTok
    {
        public string Text;
        public TokType Type;
        public OutTok(string t, TokType k) { Text = t; Type = k; }
    }

    public sealed class Formatter
    {
        readonly CalcOptions o;
        readonly List<OutTok> toks = new List<OutTok>();
        public bool Rounded;

        public Formatter(CalcOptions opts) { o = opts ?? new CalcOptions(); }

        public static string PlainText(Expr e)
        {
            var c = Calculator.Current;
            var f = new Formatter(c?.Opts);
            f.WriteTerm(e, 0);
            return f.Text;
        }

        public static string Format(Expr e, CalcOptions opts, out bool rounded)
        {
            var f = new Formatter(opts);
            f.WriteTerm(e, 0);
            rounded = f.Rounded;
            return f.Text;
        }

        public static List<OutTok> Tokens(Expr e, CalcOptions opts, out bool rounded)
        {
            var f = new Formatter(opts);
            f.WriteTerm(e, 0);
            rounded = f.Rounded;
            return f.toks;
        }

        static bool PureUnits(Expr e)
        {
            if (Simp.IsUnitFactor(e)) return true;
            return e is MulE m && m.Factors.All(Simp.IsUnitFactor);
        }

        // writes an operand that stands on its own, so bare units get a leading 1
        public void WriteTerm(Expr e, int ctx)
        {
            if (PureUnits(e))
            {
                Emit("1", TokType.Number);
                Emit(" ", TokType.Plain);
            }
            Write(e, ctx);
        }

        // ascii friendly text for copying
        public static string Ascii(Expr e, CalcOptions opts)
        {
            var po = opts.Clone();
            po.Unicode = false; po.Mult = MultSign.Asterisk; po.Grouping = DigitGrouping.None; po.Repeating = RepeatingMode.None;
            po.ImplicitMultOutput = false;
            return Format(e, po, out _);
        }

        public string Text => string.Concat(toks.Select(t => t.Text));

        void Emit(string s, TokType t) { if (!string.IsNullOrEmpty(s)) toks.Add(new OutTok(s, t)); }

        string MinusSign => o.Unicode ? "−" : "-";
        string TimesSign
        {
            get
            {
                if (!o.Unicode) return "*";
                switch (o.Mult)
                {
                    case MultSign.Dot: return "⋅";
                    case MultSign.MiddleDot: return "·";
                    case MultSign.Asterisk: return "*";
                    default: return "×";
                }
            }
        }
        string DivSign => o.Unicode && o.UnicodeDivision ? "∕" : o.Unicode && o.Div == Engine.DivSign.Division ? "÷" : "/";

        static int Prec(Expr e)
        {
            switch (e)
            {
                case LogicE _: return 1;
                case CmpE _: return 2;
                case AddE _: return 3;
                case MulE m: return 4;
                case NumE n: return n.V.IsReal ? (n.V.Re.Sign < 0 ? 4 : 10) : (n.V.Re.IsZero && n.V.Im.Sign > 0 ? 10 : 3);
                case IntvE _: return 3;
                case PowE _: return 6;
                default: return 10;
            }
        }

        void Paren(Expr e, bool p)
        {
            if (p) Emit("(", TokType.Paren);
            Write(e, 0);
            if (p) Emit(")", TokType.Paren);
        }

        public void Write(Expr e, int ctx)
        {
            bool p = Prec(e) < ctx;
            if (p) { Emit("(", TokType.Paren); Write(e, 0); Emit(")", TokType.Paren); return; }
            switch (e)
            {
                case NumE n: WriteCx(n.V); break;
                case SymE s: Emit(s.Name, TokType.Symbol); break;
                case VarE v: Emit(VarName(v.V), TokType.Variable); break;
                case UnitE u: Emit(UnitName(u, true), TokType.Unit); break;
                case AddE a: WriteAdd(a); break;
                case MulE m: WriteMul(m); break;
                case PowE pw: WritePow(pw); break;
                case CallE c: WriteCall(c); break;
                case MatE me: WriteMatrix(me); break;
                case CmpE ce: WriteCmp(ce); break;
                case LogicE le: WriteLogic(le); break;
                case StrE st: Emit(st.S, TokType.Text); break;
                case DateE d: Emit(FormatDate(d), TokType.Number); break;
                case IntvE iv: WriteInterval(iv); break;
                case ArgE ae: Emit("\\" + (char)(ae.Index < 3 ? 'x' + ae.Index : 'a' + ae.Index - 3), TokType.Symbol); break;
                default: Emit("?", TokType.Error); break;
            }
        }

        public static string FormatDate(DateE d)
        {
            if (!d.HasTime) return d.T.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture);
            string s = d.T.ToString(d.T.Second == 0 && d.T.Millisecond == 0 ? "yyyy-MM-dd'T'HH:mm" : "yyyy-MM-dd'T'HH:mm:ss", CultureInfo.InvariantCulture);
            if (d.T.Millisecond != 0) s += "." + d.T.Millisecond.ToString("000");
            return s + (d.Utc ? "Z" : "");
        }

        string VarName(Variable v)
        {
            if (o.Unicode)
            {
                switch (v.Name)
                {
                    case "pi": return "π";
                    case "euler": return "γ";
                    case "golden": return "φ";
                    case "infinity": return "∞";
                }
            }
            return v.Names.FirstOrDefault(n => o.Unicode || n.All(ch => ch < 128)) ?? v.Name;
        }

        public string UnitName(UnitE u, bool singular)
        {
            if (o.AbbreviateNames)
            {
                string ab = u.U.Abbrev;
                if (!o.Unicode && ab == "Ω") ab = "ohm";
                if (!o.Unicode && ab.StartsWith("°") && ab.Length > 1) ab = "o" + ab.Substring(1);
                if (!o.Unicode && ab == "°") ab = "deg";
                string ps = u.P == null ? "" : (u.P.Symbol == "µ" && !o.Unicode ? "u" : u.P.Symbol);
                return ps + ab;
            }
            string full = singular ? (u.U.Names.Count > 1 ? u.U.Names[1] : u.U.Name) : u.U.Plural;
            return (u.P == null ? "" : u.P.Name) + full;
        }

        void WriteAdd(AddE a)
        {
            for (int i = 0; i < a.Terms.Count; i++)
            {
                var t = a.Terms[i];
                bool neg = IsNegative(t);
                if (i > 0) Emit(neg ? " " + MinusSign + " " : " + ", TokType.Operator);
                else if (neg) Emit(MinusSign, TokType.Operator);
                var tt = neg ? Negate(t) : t;
                WriteTerm(tt, 4);
            }
        }

        static bool IsNegative(Expr t)
        {
            if (t is NumE n) return n.V.IsReal ? n.V.Re.Sign < 0 : (n.V.Re.IsZero && n.V.Im.Sign < 0);
            if (t is MulE m && m.Factors.Count > 0 && m.Factors[0] is NumE f) return f.V.IsReal ? f.V.Re.Sign < 0 : (f.V.Re.IsZero && f.V.Im.Sign < 0);
            if (t is IntvE iv) return iv.Hi.Sign < 0;
            return false;
        }

        static Expr Negate(Expr t)
        {
            if (t is NumE n) return new NumE(-n.V);
            if (t is IntvE iv) return new IntvE(-iv.Hi, -iv.Lo);
            var m = (MulE)t;
            var l = m.Factors.ToList();
            var nv = -((NumE)l[0]).V;
            if (nv.IsOne) l.RemoveAt(0); else l[0] = new NumE(nv);
            return l.Count == 1 ? l[0] : new MulE(l);
        }

        void WriteMul(MulE m)
        {
            var num = new List<Expr>(); var den = new List<Expr>();
            foreach (var f in m.Factors)
            {
                if (f is PowE pw && pw.X is NumE x && x.V.IsReal && x.V.Re.Sign < 0)
                {
                    var nx = -x.V;
                    den.Add(nx.IsOne ? pw.B : new PowE(pw.B, new NumE(nx)));
                }
                else if (f is NumE fn && fn.V.IsReal && !fn.V.Re.Approx && !fn.V.Re.D.IsOne && m.Factors.Any(q => !(q is NumE) && !Simp.IsUnitFactor(q) && !(q is MatE)))
                {
                    if (!fn.V.Re.N.IsOne || num.Count == 0) num.Add(new NumE(new Cx(Real.Int(fn.V.Re.N))));
                    den.Insert(0, new NumE(new Cx(Real.Int(fn.V.Re.D))));
                }
                else num.Add(f);
            }
            if (num.Count > 1 && num[0].IsOne) num.RemoveAt(0);
            if (num.Count > 0 && num[0] is NumE n0 && n0.V.IsReal && n0.V.Re.EqualsValue(Real.MinusOne) && num.Count > 1)
            {
                Emit(MinusSign, TokType.Operator);
                num.RemoveAt(0);
            }
            if (num.Count == 0) num.Add(Expr.One);
            WriteFactors(num);
            if (den.Count > 0)
            {
                bool unitDen = den.All(Simp.IsUnitFactor) && num.Any(Simp.IsUnitFactor);
                bool allUnitsDen = den.All(d => Simp.IsUnitFactor(d) || d is UnitE);
                Emit(DivSign, TokType.Operator);
                if (den.Count == 1) Write(den[0], 6);
                else
                {
                    Emit("(", TokType.Paren);
                    WriteFactors(den);
                    Emit(")", TokType.Paren);
                }
            }
        }

        void WriteFactors(List<Expr> fs)
        {
            for (int i = 0; i < fs.Count; i++)
            {
                var f = fs[i];
                if (i > 0)
                {
                    var prev = fs[i - 1];
                    bool prevNum = prev is NumE pn && pn.V.IsReal;
                    if (Simp.IsUnitFactor(f) && Simp.IsUnitFactor(prev)) Emit(o.Unicode ? "·" : "*", TokType.Operator);
                    else if (o.ImplicitMultOutput && (f is AddE || f is PowE fa && fa.B is AddE) && !Simp.IsUnitFactor(prev)) { }
                    else if (Simp.IsUnitFactor(f)) Emit(" ", TokType.Plain);
                    else if (o.ImplicitMultOutput && prevNum && (f is SymE || f is VarE || f is CallE || (f is PowE fp && !(fp.B is NumE)) || f is AddE || (f is PowE fq && fq.B is NumE && fq.X is NumE fx && fx.V.IsReal && fx.V.Re.EqualsValue(Real.Half))))
                    {
                        if (f is AddE) { }
                    }
                    else if (o.ImplicitMultOutput && prev is NumE pc && !pc.V.IsReal && (f is SymE || f is VarE)) { }
                    else Emit(" " + TimesSign + " ", TokType.Operator);
                }
                if (f is NumE fnum && fs.Count > 1 && i == 0 && !fnum.V.IsReal) { Write(f, 10); continue; }
                Write(f, i == 0 && f is NumE ? 4 : 5);
            }
        }

        static readonly string Sup = "⁰¹²³⁴⁵⁶⁷⁸⁹";

        static string Superscript(BigInteger n)
        {
            var s = n.ToString(CultureInfo.InvariantCulture);
            var sb = new StringBuilder();
            foreach (char ch in s) sb.Append(ch == '-' ? '⁻' : Sup[ch - '0']);
            return sb.ToString();
        }

        void WritePow(PowE p)
        {
            if (p.X is NumE x && x.V.IsReal && !x.V.Re.Approx)
            {
                var r = x.V.Re;
                if (r.EqualsValue(Real.Half) || r.EqualsValue(Real.Frac(1, 3)))
                {
                    Emit(o.Unicode ? (r.EqualsValue(Real.Half) ? "√" : "∛") : (r.EqualsValue(Real.Half) ? "sqrt" : "cbrt"), TokType.Function);
                    bool paren = !(p.B is NumE bn && bn.V.IsReal && bn.V.Re.Sign >= 0) && !(p.B is SymE) && !(p.B is VarE) || !o.Unicode;
                    Paren(p.B, paren);
                    return;
                }
                if (r.Sign < 0)
                {
                    Emit("1", TokType.Number);
                    Emit(DivSign, TokType.Operator);
                    Write(r.EqualsValue(Real.MinusOne) ? p.B : new PowE(p.B, new NumE(new Cx(-r))), 6);
                    return;
                }
                if (r.IsInt && o.Unicode && BigInteger.Abs(r.N) < 1000000)
                {
                    Write(p.B, 7);
                    Emit(Superscript(r.N), TokType.Number);
                    return;
                }
            }
            Write(p.B, 7);
            Emit("^", TokType.Operator);
            Write(p.X, 7);
        }

        void WriteCall(CallE c)
        {
            if (c.F.Name == "_hold" && c.Args.Count == 1) { Write(c.Args[0], 0); return; }
            if (c.F.Name == "abs" && c.Args.Count == 1 && o.Unicode)
            {
                Emit("|", TokType.Paren); Write(c.Args[0], 0); Emit("|", TokType.Paren);
                return;
            }
            if (c.F.Name == "factorial" && c.Args.Count == 1)
            {
                Write(c.Args[0], 9); Emit("!", TokType.Operator);
                return;
            }
            Emit(c.F.Names[0].StartsWith("_") ? c.F.Names[0].Substring(1) : c.F.Names[0], TokType.Function);
            Emit("(", TokType.Paren);
            string sep = Calculator.Current != null && Calculator.Current.Opts.CommaDecimal ? "; " : ", ";
            for (int i = 0; i < c.Args.Count; i++)
            {
                if (i > 0) Emit(sep, TokType.Operator);
                WriteTerm(c.Args[i], 0);
            }
            Emit(")", TokType.Paren);
        }

        void WriteMatrix(MatE m)
        {
            string sep = Calculator.Current != null && Calculator.Current.Opts.CommaDecimal ? "; " : ", ";
            if (m.IsVector)
            {
                Emit("[", TokType.Paren);
                for (int i = 0; i < m.ColCount; i++) { if (i > 0) Emit(sep, TokType.Operator); WriteTerm(m.Rows[0][i], 0); }
                Emit("]", TokType.Paren);
                return;
            }
            Emit("[", TokType.Paren);
            for (int r = 0; r < m.RowCount; r++)
            {
                if (r > 0) Emit(sep, TokType.Operator);
                Emit("[", TokType.Paren);
                for (int c = 0; c < m.ColCount; c++) { if (c > 0) Emit(sep, TokType.Operator); WriteTerm(m.Rows[r][c], 0); }
                Emit("]", TokType.Paren);
            }
            Emit("]", TokType.Paren);
        }

        void WriteCmp(CmpE c)
        {
            WriteTerm(c.L, 3);
            string op;
            switch (c.Op)
            {
                case CmpOp.Eq: op = " = "; break;
                case CmpOp.Ne: op = o.Unicode ? " ≠ " : " != "; break;
                case CmpOp.Lt: op = " < "; break;
                case CmpOp.Gt: op = " > "; break;
                case CmpOp.Le: op = o.Unicode ? " ≤ " : " <= "; break;
                default: op = o.Unicode ? " ≥ " : " >= "; break;
            }
            Emit(op, TokType.Operator);
            WriteTerm(c.R, 3);
        }

        void WriteLogic(LogicE l)
        {
            if (l.Op == LogicOp.Not)
            {
                Emit(o.SpellOutLogical ? "not " : "!", TokType.Operator);
                Write(l.Items[0], 9);
                return;
            }
            string op = l.Op == LogicOp.And ? (o.SpellOutLogical ? " and " : " && ") : l.Op == LogicOp.Or ? (o.SpellOutLogical ? " or " : " || ") : " xor ";
            for (int i = 0; i < l.Items.Count; i++)
            {
                if (i > 0) Emit(op, TokType.Operator);
                Write(l.Items[i], 2);
            }
        }

        void WriteInterval(IntvE iv)
        {
            var mode = o.IntervalOut;
            if (mode == IntervalDisplay.Adaptive) mode = IntervalDisplay.PlusMinus;
            switch (mode)
            {
                case IntervalDisplay.Interval:
                    Emit("interval", TokType.Function); Emit("(", TokType.Paren);
                    WriteReal(iv.Lo); Emit(", ", TokType.Operator); WriteReal(iv.Hi);
                    Emit(")", TokType.Paren);
                    break;
                case IntervalDisplay.Midpoint: WriteReal(iv.Mid); Rounded = true; break;
                case IntervalDisplay.Lower: WriteReal(iv.Lo); Rounded = true; break;
                case IntervalDisplay.Upper: WriteReal(iv.Hi); Rounded = true; break;
                case IntervalDisplay.Relative:
                    WriteReal(iv.Mid);
                    Emit("±", TokType.Operator);
                    if (!iv.Mid.IsZero) { WriteReal((iv.Rad / iv.Mid * Real.Int(100)).Abs()); Emit("%", TokType.Unit); }
                    else WriteReal(iv.Rad);
                    break;
                case IntervalDisplay.Concise:
                    {
                        string mid = FormatSig(iv.Mid, out int decs);
                        var unit = Real.Int(10).PowInt(-decs);
                        var err = (iv.Rad / unit).Ceil();
                        Emit(mid + "(" + err.ToBigInt() + ")", TokType.Number);
                        break;
                    }
                case IntervalDisplay.SignificantDigits:
                    {
                        WriteReal(iv.Mid, SigDigitsFor(iv));
                        Rounded = true;
                        break;
                    }
                default:
                    WriteReal(iv.Mid, SigDigitsFor(iv));
                    Emit(o.Unicode ? "±" : "+/-", TokType.Operator);
                    WriteReal(iv.Rad, 2);
                    break;
            }
        }

        static int SigDigitsFor(IntvE iv)
        {
            double m = Math.Abs(iv.Mid.ToDouble()), r = Math.Abs(iv.Rad.ToDouble());
            if (r == 0 || m == 0) return 10;
            int d = (int)Math.Floor(Math.Log10(m)) - (int)Math.Floor(Math.Log10(r)) + 2;
            return Math.Max(1, Math.Min(30, d));
        }

        string FormatSig(Real r, out int decimals)
        {
            var s = FormatReal(r, o.Precision);
            int dot = s.IndexOf('.');
            decimals = dot < 0 ? 0 : s.Length - dot - 1;
            return s;
        }

        void WriteReal(Real r, int sig = -1)
        {
            Emit(FormatReal(r, sig), TokType.Number);
        }

        public void WriteCx(Cx v)
        {
            if (v.IsReal) { WriteRealValue(v.Re); return; }
            switch (o.Complex)
            {
                case ComplexForm.Exponential:
                case ComplexForm.Polar:
                case ComplexForm.Cis:
                case ComplexForm.Angle:
                    WritePolar(v);
                    return;
            }
            string iName = Calculator.Current != null && Calculator.Current.Opts.UseJ ? "j" : "i";
            if (!v.Re.IsZero)
            {
                WriteRealValue(v.Re);
                Emit(v.Im.Sign < 0 ? " " + MinusSign + " " : " + ", TokType.Operator);
                var im = v.Im.Abs();
                if (!im.IsOne || im.Approx) WriteRealValue(im);
                Emit(iName, TokType.Variable);
            }
            else
            {
                if (v.Im.Sign < 0) Emit(MinusSign, TokType.Operator);
                var im = v.Im.Abs();
                if (!im.IsOne || im.Approx) WriteRealValue(im);
                Emit(iName, TokType.Variable);
            }
        }

        void WritePolar(Cx v)
        {
            var save = Calculator.Current?.ExactPhase ?? true;
            if (Calculator.Current != null) Calculator.Current.ExactPhase = false;
            try
            {
                var r = v.Abs();
                var th = v.Arg();
                string iName = Calculator.Current != null && Calculator.Current.Opts.UseJ ? "j" : "i";
                switch (o.Complex)
                {
                    case ComplexForm.Exponential:
                        if (!r.IsOne) WriteRealValue(r);
                        Emit("e", TokType.Variable); Emit("^", TokType.Operator); Emit("(", TokType.Paren);
                        WriteRealValue(th); Emit(iName, TokType.Variable); Emit(")", TokType.Paren);
                        break;
                    case ComplexForm.Polar:
                        WriteRealValue(r); Emit("(", TokType.Paren);
                        Emit("cos", TokType.Function); Emit("(", TokType.Paren); WriteRealValue(th); Emit(")", TokType.Paren);
                        Emit(" + ", TokType.Operator); Emit(iName, TokType.Variable); Emit(" ", TokType.Plain);
                        Emit("sin", TokType.Function); Emit("(", TokType.Paren); WriteRealValue(th); Emit(")", TokType.Paren);
                        Emit(")", TokType.Paren);
                        break;
                    case ComplexForm.Cis:
                        WriteRealValue(r); Emit(" cis ", TokType.Function); WriteRealValue(th);
                        break;
                    default:
                        {
                            var ang = th;
                            string unit = "";
                            var am = Calculator.Current?.Opts.Angle ?? AngleMode.Radians;
                            if (am == AngleMode.Degrees) { ang = th * Real.Int(180) / RMath.Pi(); unit = "°"; }
                            else if (am == AngleMode.Gradians) { ang = th * Real.Int(200) / RMath.Pi(); unit = " gon"; }
                            WriteRealValue(r); Emit("∠", TokType.Operator); WriteRealValue(ang); Emit(unit, TokType.Unit);
                            break;
                        }
                }
            }
            finally { if (Calculator.Current != null) Calculator.Current.ExactPhase = save; }
        }

        void WriteRealValue(Real r)
        {
            if (r.IsInfinite) { Emit(r.Sign < 0 ? MinusSign + "∞" : "∞", TokType.Number); return; }
            int b = o.OutputBase;
            if (b != 10)
            {
                string s = FormatInBase(r, b);
                if (s != null) { Emit(s, TokType.Number); return; }
            }
            if (!r.Approx && !r.IsInt)
            {
                var mode = o.Fraction;
                if (mode == FractionMode.DecimalExact && o.Approx == ApproxMode.Exact && !Terminates(r)) mode = FractionMode.Fraction;
                switch (mode)
                {
                    case FractionMode.Fraction:
                        Emit(FormatInt(r.N) + DivSign + FormatInt(r.D), TokType.Number);
                        return;
                    case FractionMode.Combined:
                        {
                            var w = r.Trunc();
                            var fr = (r - w).Abs();
                            if (w.IsZero) { Emit(FormatInt(r.N) + DivSign + FormatInt(r.D), TokType.Number); return; }
                            Emit(FormatInt(w.N), TokType.Number);
                            Emit(r.Sign < 0 ? " " + MinusSign + " " : " + ", TokType.Operator);
                            Emit(FormatInt(fr.N) + DivSign + FormatInt(fr.D), TokType.Number);
                            return;
                        }
                }
            }
            switch (o.Fraction)
            {
                case FractionMode.Percent: Emit(FormatReal(r * Real.Int(100)), TokType.Number); Emit("%", TokType.Unit); return;
                case FractionMode.Permille: Emit(FormatReal(r * Real.Int(1000)), TokType.Number); Emit("‰", TokType.Unit); return;
                case FractionMode.Permyriad: Emit(FormatReal(r * Real.Int(10000)), TokType.Number); Emit("‱", TokType.Unit); return;
            }
            Emit(FormatReal(r), TokType.Number);
        }

        public static bool Terminates(Real r)
        {
            var d = r.D;
            while ((d % 2).IsZero) d /= 2;
            while ((d % 5).IsZero) d /= 5;
            return d.IsOne;
        }

        public static bool HasNonTerminating(Expr e) => e.Any(x => x is NumE n && n.V.IsRational && !n.V.Re.IsInt && !Terminates(n.V.Re));

        string FormatInt(BigInteger n)
        {
            string s = BigInteger.Abs(n).ToString(CultureInfo.InvariantCulture);
            s = Group(s);
            return n.Sign < 0 ? MinusSign + s : s;
        }

        string Group(string intPart)
        {
            if (o.Grouping == DigitGrouping.None || intPart.Length <= 4) return intPart;
            string sep = o.Grouping == DigitGrouping.Standard ? (o.Unicode ? " " : " ") : (o.Unicode ? "," : ",");
            if (o.Grouping == DigitGrouping.Locale && Calculator.Current != null && Calculator.Current.Opts.CommaDecimal) sep = ".";
            var sb = new StringBuilder();
            int first = intPart.Length % 3;
            if (first > 0) sb.Append(intPart, 0, first);
            for (int i = first; i < intPart.Length; i += 3)
            {
                if (sb.Length > 0) sb.Append(sep);
                sb.Append(intPart, i, 3);
            }
            return sb.ToString();
        }

        string DecPoint => Calculator.Current != null && Calculator.Current.Opts.CommaDecimal ? "," : ".";

        // decimal formatting with significant digits and display mode
        public string FormatReal(Real r, int sig = -1)
        {
            if (r.IsInfinite) return r.Sign < 0 ? MinusSign + "∞" : "∞";
            if (sig < 0) sig = o.Precision;
            if (r.IsZero) return o.MinDecimals > 0 ? "0" + DecPoint + new string('0', o.MinDecimals) : "0";
            bool neg = r.Sign < 0;
            var a = r.Abs();
            string sign = neg ? MinusSign : "";

            if (a.IsInt && !a.Approx && o.Display != NumberDisplay.Scientific && o.Display != NumberDisplay.Engineering && o.Display != NumberDisplay.PureScientific)
            {
                string ints = a.N.ToString(CultureInfo.InvariantCulture);
                if (o.Display == NumberDisplay.Normal && ints.Length > Math.Max(sig, 20) * 3 && ints.Length > 100) { }
                else
                {
                    string res = Group(ints);
                    if (o.MinDecimals > 0) res += DecPoint + new string('0', o.MinDecimals);
                    return sign + res;
                }
            }

            int e10 = Exp10(a);
            bool exact = !a.Approx;
            string repeating = null;
            if (exact && !a.IsInt && (o.Fraction == FractionMode.DecimalExact) && o.Repeating != RepeatingMode.None)
            {
                repeating = RepeatingDecimal(a, sig, out string nonRep, out string intPart);
                if (repeating != null && o.Display == NumberDisplay.Normal && e10 < sig && e10 > -6)
                {
                    string ip = Group(intPart);
                    if (o.Repeating == RepeatingMode.Overline)
                    {
                        var ov = new StringBuilder();
                        foreach (char ch in repeating) { ov.Append(ch); ov.Append('̅'); }
                        return sign + ip + DecPoint + nonRep + ov;
                    }
                    return sign + ip + DecPoint + nonRep + repeating + repeating + "…";
                }
            }

            bool useExp;
            switch (o.Display)
            {
                case NumberDisplay.Scientific: useExp = e10 != 0; break;
                case NumberDisplay.PureScientific: useExp = true; break;
                case NumberDisplay.Engineering: useExp = e10 < 0 || e10 >= 3; break;
                case NumberDisplay.Simple: useExp = false; break;
                default: useExp = e10 >= sig || e10 < -6; break;
            }
            string body = null;
            int exp = 0;
            for (int attempt = 0; attempt < 2; attempt++)
            {
                exp = 0;
                if (useExp) exp = o.Display == NumberDisplay.Engineering ? (int)Math.Floor(e10 / 3.0) * 3 : e10;
                int pointPos = e10 - exp + 1;
                int fracDigits = sig - pointPos;
                if (o.MaxDecimals >= 0 && fracDigits > o.MaxDecimals) fracDigits = o.MaxDecimals;
                BigInteger m = ScaleRound(a, fracDigits - exp, out bool inexact);
                string ds = m.ToString(CultureInfo.InvariantCulture);
                int expectedLen = pointPos + fracDigits;
                if (attempt == 0 && ds.Length > expectedLen && expectedLen > 0 && !m.IsZero)
                {
                    e10++;
                    if (o.Display == NumberDisplay.Normal) useExp = e10 >= sig || e10 < -6;
                    if (o.Display == NumberDisplay.Engineering) useExp = e10 < 0 || e10 >= 3;
                    if (o.Display == NumberDisplay.Scientific) useExp = e10 != 0;
                    continue;
                }
                if (inexact) Rounded = true;
                string intStr, fracStr;
                if (fracDigits <= 0)
                {
                    intStr = (m * BigInteger.Pow(10, -fracDigits)).ToString(CultureInfo.InvariantCulture);
                    fracStr = "";
                }
                else
                {
                    if (ds.Length <= fracDigits) ds = new string('0', fracDigits - ds.Length + 1) + ds;
                    intStr = ds.Substring(0, ds.Length - fracDigits);
                    fracStr = ds.Substring(ds.Length - fracDigits);
                }
                if (!(o.ShowEndingZeroes && a.Approx)) fracStr = fracStr.TrimEnd('0');
                if (fracStr.Length < o.MinDecimals) fracStr = fracStr.PadRight(o.MinDecimals, '0');
                body = Group(intStr) + (fracStr.Length > 0 ? DecPoint + fracStr : "");
                break;
            }
            if (useExp && (exp != 0 || o.Display == NumberDisplay.PureScientific))
            {
                if (o.ENotation || !o.Unicode) body += (o.LowerCaseE ? "e" : "E") + (exp < 0 ? "-" : "") + Math.Abs(exp);
                else body += " " + TimesSign + " 10" + Superscript(exp);
            }
            if (body.Trim('0', '.', ',').Length == 0) return body;
            return sign + body;
        }

        string FormatRealForced(Real r, int sig)
        {
            var save = o.Display;
            try { o.Display = NumberDisplay.Scientific; return FormatReal(r, sig); }
            finally { o.Display = save; }
        }

        // floor(log10(a)) for a > 0
        public static int Exp10(Real a)
        {
            double d = a.ToDouble();
            int e;
            if (d > 0 && !double.IsInfinity(d) && d > 1e-300) e = (int)Math.Floor(Math.Log10(d));
            else e = (int)Math.Floor((Real.BitLen(a.N) - Real.BitLen(a.D)) * 0.30102999566398);
            var ten = Real.Int(10);
            for (int k = 0; k < 4; k++)
            {
                var lo = ten.PowInt(e);
                if (a < lo) { e--; continue; }
                if (a >= lo * ten) { e++; continue; }
                break;
            }
            return e;
        }

        BigInteger ScaleRound(Real a, int scale, out bool inexact)
        {
            BigInteger n = a.N, d = a.D;
            if (scale >= 0) n *= BigInteger.Pow(10, scale);
            else d *= BigInteger.Pow(10, -scale);
            var q = BigInteger.DivRem(n, d, out var rem);
            inexact = !rem.IsZero || a.Approx;
            if (rem.IsZero) return q;
            var x = Real.Make(n, d);
            return x.Round(o.Rounding).N;
        }

        // detects periodic decimal expansion, returns repeating digits or null
        static string RepeatingDecimal(Real a, int maxLen, out string nonRep, out string intPart)
        {
            nonRep = ""; intPart = "";
            var d = a.D;
            BigInteger d2 = d;
            int twos = 0, fives = 0;
            while ((d2 % 2).IsZero) { d2 /= 2; twos++; }
            while ((d2 % 5).IsZero) { d2 /= 5; fives++; }
            if (d2.IsOne) return null;
            int pre = Math.Max(twos, fives);
            var ip = BigInteger.DivRem(a.N, d, out var rem);
            intPart = ip.ToString(CultureInfo.InvariantCulture);
            var sb = new StringBuilder();
            for (int i = 0; i < pre; i++) { rem *= 10; sb.Append((char)('0' + (int)(rem / d))); rem %= d; }
            nonRep = sb.ToString();
            var start = rem;
            var rep = new StringBuilder();
            for (int i = 0; i < Math.Max(maxLen, 30) * 2; i++)
            {
                rem *= 10;
                rep.Append((char)('0' + (int)(rem / d)));
                rem %= d;
                if (rem == start) return rep.ToString();
            }
            return null;
        }

        static string DigitChars(bool lower, bool duo)
        {
            var s = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
            if (lower) s = s.ToLowerInvariant();
            if (duo) s = "0123456789↊↋";
            return s;
        }

        public string FormatInBase(Real r, int b)
        {
            switch (b)
            {
                case Bases.Roman: return r.IsInt ? Funcs.ToRoman(r.ToBigInt()) : null;
                case Bases.Time: return Sexagesimal(r, ":", ":", "", true);
                case Bases.Sexagesimal: return Sexagesimal(r, "°", "′", "″", false);
                case Bases.Latitude: return Sexagesimal(r.Abs(), "°", "′", "″", false) + (r.Sign < 0 ? " S" : " N");
                case Bases.Longitude: return Sexagesimal(r.Abs(), "°", "′", "″", false) + (r.Sign < 0 ? " W" : " E");
                case Bases.Bijective26: return r.IsInt && r.Sign > 0 ? Bijective(r.ToBigInt()) : null;
                case Bases.Bcd: return r.IsInt ? Bcd(r.ToBigInt()) : null;
                case Bases.Unicode:
                    if (r.IsInt && r.Sign > 0 && r.N <= 0x10FFFF) { try { return char.ConvertFromUtf32((int)r.N); } catch { return null; } }
                    return null;
                case Bases.Golden: return RealBase(r, (1 + Math.Sqrt(5)) / 2);
                case Bases.Supergolden: return RealBase(r, 1.4655712318767680267);
                case Bases.PiBase: return RealBase(r, Math.PI);
                case Bases.EBase: return RealBase(r, Math.E);
                case Bases.Sqrt2: return RealBase(r, Math.Sqrt(2));
                case Bases.Fp16: return FloatFormats.BinaryString(r, FloatFormats.Half);
                case Bases.Fp32: return FloatFormats.BinaryString(r, FloatFormats.Single);
                case Bases.Fp64: return FloatFormats.BinaryString(r, FloatFormats.Double);
                case Bases.Fp80: return FloatFormats.BinaryString(r, FloatFormats.Extended);
                case Bases.Fp128: return FloatFormats.BinaryString(r, FloatFormats.Quad);
                case Bases.Custom:
                    {
                        try
                        {
                            var c = Calculator.Current;
                            var be = c.EvalText(o.OutputBaseCustom);
                            if (be is NumE bn && bn.V.IsReal)
                            {
                                if (bn.V.Re.IsInt && !bn.V.Re.Approx) return FormatInBase(r, bn.V.Re.ToInt());
                                return RealBase(r, bn.V.Re.ToDouble());
                            }
                        }
                        catch (CalcException) { }
                        return null;
                    }
            }
            if (b == 10) return null;
            if (b >= 2 && b <= 36) return IntegerBase(r, b);
            if (b <= -2 && b >= -36) return NegativeBase(r, -b);
            if (b > 36 && b <= 1000000) return LargeBase(r, b);
            return null;
        }

        string IntegerBase(Real r, int b)
        {
            bool neg = r.Sign < 0;
            var a = r.Abs();
            var ip = a.Floor().ToBigInt();
            string digits = DigitChars(o.LowerCaseHex, b == 12 && o.DuodecimalSymbols);
            string intStr;
            if (neg && a.IsInt && (b == 2 || b == 16) && (o.TwosOut == TwosComplement.Binary && b == 2 || o.TwosOut == TwosComplement.BinaryHex))
            {
                int bits = o.BinaryBits > 0 ? o.BinaryBits : 8;
                while (BigInteger.One << (bits - 1) < ip && bits < 4096) bits *= 2;
                var tc = (BigInteger.One << bits) - ip;
                intStr = ToDigits(tc, b, digits).PadLeft(b == 2 ? bits : bits / 4, '0');
                return GroupBase(intStr, b);
            }
            intStr = ToDigits(ip, b, digits);
            if (b == 2 && o.BinaryBits > 0) intStr = intStr.PadLeft(o.BinaryBits, '0');
            var frac = a - a.Floor();
            var sb = new StringBuilder();
            if (!frac.IsZero)
            {
                int maxd = (int)Math.Ceiling(o.Precision * Math.Log(10) / Math.Log(b));
                if (o.MaxDecimals >= 0) maxd = Math.Min(maxd, o.MaxDecimals);
                for (int i = 0; i < maxd && !frac.IsZero; i++)
                {
                    frac *= Real.Int(b);
                    var dgt = frac.Floor();
                    sb.Append(digits[dgt.ToInt()]);
                    frac -= dgt;
                }
                if (!frac.IsZero) Rounded = true;
            }
            string res = GroupBase(intStr, b) + (sb.Length > 0 ? DecPoint + sb.ToString().TrimEnd('0') : "");
            return (neg ? MinusSign : "") + res;
        }

        string GroupBase(string s, int b)
        {
            int g = o.BinaryLetterSpacing;
            if (b != 2 || g <= 0 || s.Length <= g) return s;
            var sb = new StringBuilder();
            int first = s.Length % g;
            if (first > 0) sb.Append(s, 0, first);
            for (int i = first; i < s.Length; i += g)
            {
                if (sb.Length > 0) sb.Append(' ');
                sb.Append(s, i, g);
            }
            return sb.ToString();
        }

        static string ToDigits(BigInteger v, int b, string digits)
        {
            if (v.IsZero) return "0";
            var sb = new StringBuilder();
            while (!v.IsZero)
            {
                int d = (int)(v % b);
                sb.Insert(0, digits[d]);
                v /= b;
            }
            return sb.ToString();
        }

        string NegativeBase(Real r, int b)
        {
            if (!r.IsInt) return null;
            var v = r.ToBigInt();
            if (v.IsZero) return "0";
            var sb = new StringBuilder();
            string digits = DigitChars(o.LowerCaseHex, false);
            while (!v.IsZero)
            {
                var rem = (int)(v % -b);
                v /= -b;
                if (rem < 0) { rem += b; v += 1; }
                sb.Insert(0, digits[rem]);
            }
            return sb.ToString();
        }

        string LargeBase(Real r, int b)
        {
            if (!r.IsInt) return null;
            var v = BigInteger.Abs(r.ToBigInt());
            var parts = new List<string>();
            if (v.IsZero) parts.Add("0");
            while (!v.IsZero) { parts.Insert(0, (v % b).ToString(CultureInfo.InvariantCulture)); v /= b; }
            return (r.Sign < 0 ? MinusSign : "") + string.Join(":", parts);
        }

        string RealBase(Real r, double beta)
        {
            double x = Math.Abs(r.ToDouble());
            if (x == 0) return "0";
            int maxDigit = (int)Math.Ceiling(beta) - 1;
            int k = (int)Math.Floor(Math.Log(x) / Math.Log(beta) + 1e-12);
            if (k < 0) k = 0;
            var sb = new StringBuilder();
            int fracLimit = (int)Math.Ceiling(Math.Min(o.Precision, 14) * Math.Log(10) / Math.Log(beta));
            for (int p = k; p >= -fracLimit; p--)
            {
                if (p == -1) sb.Append(DecPoint);
                double bp = Math.Pow(beta, p);
                int d = (int)Math.Floor(x / bp + 1e-12);
                if (d > maxDigit) d = maxDigit;
                if (d < 0) d = 0;
                x -= d * bp;
                sb.Append((char)('0' + d));
                if (p < 0 && x < 1e-14) break;
            }
            if (x > 1e-12) Rounded = true;
            string s = sb.ToString();
            if (s.Contains(DecPoint)) s = s.TrimEnd('0').TrimEnd(DecPoint[0]);
            return (r.Sign < 0 ? MinusSign : "") + s;
        }

        string Sexagesimal(Real r, string s1, string s2, string s3, bool time)
        {
            bool neg = r.Sign < 0;
            var a = r.Abs();
            var deg = a.Floor();
            var minf = (a - deg) * Real.Int(60);
            var min = minf.Floor();
            var sec = (minf - min) * Real.Int(60);
            var secR = sec.Round();
            string secStr;
            if ((sec - secR).Abs() < Real.Frac(1, 1000000)) secStr = secR.ToBigInt().ToString();
            else { secStr = FormatReal(sec, Math.Max(3, o.Precision - 4)); Rounded = true; }
            if (secStr == "60") { secStr = "0"; min += Real.One; }
            if (min.EqualsValue(Real.Int(60))) { min = Real.Zero; deg += Real.One; }
            string res;
            if (time) res = deg.ToBigInt() + ":" + min.ToBigInt().ToString().PadLeft(2, '0') + ":" + secStr.PadLeft(2, '0');
            else res = deg.ToBigInt() + s1 + min.ToBigInt() + s2 + secStr + s3;
            return (neg ? MinusSign : "") + res;
        }

        static string Bijective(BigInteger n)
        {
            var sb = new StringBuilder();
            while (n > 0)
            {
                n -= 1;
                sb.Insert(0, (char)('A' + (int)(n % 26)));
                n /= 26;
            }
            return sb.ToString();
        }

        static string Bcd(BigInteger n)
        {
            var s = BigInteger.Abs(n).ToString(CultureInfo.InvariantCulture);
            var parts = s.Select(ch => Convert.ToString(ch - '0', 2).PadLeft(4, '0'));
            return (n.Sign < 0 ? "−" : "") + string.Join(" ", parts);
        }

        public static Expr AllBasesText(Expr e, CalcOptions print)
        {
            if (!(e is NumE n) || !n.V.IsReal) throw new CalcException("Only real numbers can be shown in all bases");
            var sb = new StringBuilder();
            foreach (var b in new[] { (2, "Binary"), (8, "Octal"), (10, "Decimal"), (16, "Hexadecimal") })
            {
                var po = print.Clone();
                po.OutputBase = b.Item1;
                sb.Append(b.Item2).Append(": ").Append(Format(e, po, out _));
                if (b.Item1 != 16) sb.Append("    ");
            }
            return new StrE(sb.ToString());
        }
    }
}
