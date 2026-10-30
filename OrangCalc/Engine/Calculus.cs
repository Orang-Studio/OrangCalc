using System;
using System.Collections.Generic;
using System.Linq;

namespace OrangCalc.Engine
{
    public static class Calculus
    {
        static Calculator C => Calculator.Current;
        static Expr F(string name, params Expr[] a) => C.Ev.Call(name, a);

        static bool Free(Expr e, string x) => !e.Any(z => z is SymE s && s.Name == x);

        public static Expr Diff(Expr e, string x)
        {
            C.CheckAbort();
            if (Free(e, x)) return Expr.Zero;
            switch (e)
            {
                case SymE s: return s.Name == x ? Expr.One : Expr.Zero;
                case AddE a: return Simp.Add(a.Terms.Select(t => Diff(t, x)).ToList());
                case MulE m:
                    {
                        var terms = new List<Expr>();
                        for (int i = 0; i < m.Factors.Count; i++)
                        {
                            if (Free(m.Factors[i], x)) continue;
                            var fs = m.Factors.ToList();
                            fs[i] = Diff(m.Factors[i], x);
                            terms.Add(Simp.Mul(fs));
                        }
                        return Simp.Add(terms);
                    }
                case PowE p:
                    {
                        if (Free(p.X, x))
                            return Simp.Mul(p.X, Simp.Pow(p.B, Simp.Sub(p.X, Expr.One)), Diff(p.B, x));
                        if (Free(p.B, x))
                            return Simp.Mul(e, F("ln", p.B), Diff(p.X, x));
                        var lnb = F("ln", p.B);
                        return Simp.Mul(e, Simp.Add(Simp.Mul(Diff(p.X, x), lnb), Simp.Mul(p.X, Simp.Div(Diff(p.B, x), p.B))));
                    }
                case MatE me: return Matrices.MapElements(me, t => Diff(t, x));
                case CmpE ce: return new CmpE(ce.Op, Diff(ce.L, x), Diff(ce.R, x));
                case CallE c: return DiffCall(c, x);
            }
            throw new CalcException("Unable to differentiate " + Formatter.PlainText(e));
        }

        static Expr DiffCall(CallE c, string x)
        {
            var u = c.Args.Count > 0 ? c.Args[0] : null;
            var du = u != null ? Diff(u, x) : Expr.Zero;
            Expr outer;
            Expr angle = AngleFactorInverse();
            switch (c.F.Name)
            {
                case "sin": outer = Simp.Mul(F("cos", u), angle); break;
                case "cos": outer = Simp.Neg(Simp.Mul(F("sin", u), angle)); break;
                case "tan": outer = Simp.Mul(Simp.Pow(F("cos", u), Expr.N(-2)), angle); break;
                case "cot": outer = Simp.Neg(Simp.Mul(Simp.Pow(F("sin", u), Expr.N(-2)), angle)); break;
                case "sec": outer = Simp.Mul(F("sec", u), F("tan", u), angle); break;
                case "csc": outer = Simp.Neg(Simp.Mul(F("csc", u), F("cot", u), angle)); break;
                case "asin": outer = Simp.Div(Simp.Pow(Simp.Sub(Expr.One, Simp.Pow(u, Expr.Two)), Expr.N(Real.Frac(-1, 2))), angle); break;
                case "acos": outer = Simp.Neg(Simp.Div(Simp.Pow(Simp.Sub(Expr.One, Simp.Pow(u, Expr.Two)), Expr.N(Real.Frac(-1, 2))), angle)); break;
                case "atan": outer = Simp.Div(Simp.Pow(Simp.Add(Expr.One, Simp.Pow(u, Expr.Two)), Expr.MinusOne), angle); break;
                case "sinh": outer = F("cosh", u); break;
                case "cosh": outer = F("sinh", u); break;
                case "tanh": outer = Simp.Pow(F("cosh", u), Expr.N(-2)); break;
                case "asinh": outer = Simp.Pow(Simp.Add(Simp.Pow(u, Expr.Two), Expr.One), Expr.N(Real.Frac(-1, 2))); break;
                case "acosh": outer = Simp.Pow(Simp.Sub(Simp.Pow(u, Expr.Two), Expr.One), Expr.N(Real.Frac(-1, 2))); break;
                case "atanh": outer = Simp.Pow(Simp.Sub(Expr.One, Simp.Pow(u, Expr.Two)), Expr.MinusOne); break;
                case "exp": outer = c; break;
                case "ln": outer = Simp.Pow(u, Expr.MinusOne); break;
                case "log":
                    if (c.Args.Count > 1) return Diff(Simp.Div(F("ln", u), F("ln", c.Args[1])), x);
                    outer = Simp.Pow(u, Expr.MinusOne); break;
                case "sqrt": outer = Simp.Div(Expr.Half, F("sqrt", u)); break;
                case "cbrt": outer = Simp.Div(Expr.N(Real.Frac(1, 3)), Simp.Pow(F("cbrt", u), Expr.Two)); break;
                case "root": return Diff(Simp.Pow(u, Simp.Pow(c.Args[1], Expr.MinusOne)), x);
                case "abs": outer = F("sgn", u); break;
                case "erf": outer = Simp.Mul(Simp.Div(Expr.Two, F("sqrt", C.VarRef(C.FindVariable("pi")))), F("exp", Simp.Neg(Simp.Pow(u, Expr.Two)))); break;
                case "gamma": outer = Simp.Mul(F("gamma", u), F("digamma", u)); break;
                case "lambertw": outer = Simp.Div(F("lambertw", u), Simp.Mul(u, Simp.Add(Expr.One, F("lambertw", u)))); break;
                case "Ei": outer = Simp.Div(F("exp", u), u); break;
                case "li": outer = Simp.Pow(F("ln", u), Expr.MinusOne); break;
                case "Si": outer = Simp.Div(F("sin", u), u); break;
                case "Ci": outer = Simp.Div(F("cos", u), u); break;
                case "sgn": outer = Expr.Zero; break;
                case "heaviside": outer = F("dirac", u); break;
                default: throw new CalcException("Unable to differentiate " + c.F.Names[0] + "()");
            }
            return Simp.Mul(outer, du);
        }

        static Expr AngleFactorInverse()
        {
            var pi = C.VarRef(C.FindVariable("pi"));
            switch (C.Opts.Angle)
            {
                case AngleMode.Degrees: return Simp.Div(pi, Expr.N(180));
                case AngleMode.Gradians: return Simp.Div(pi, Expr.N(200));
                default: return Expr.One;
            }
        }

        // symbolic antiderivative or null
        public static Expr Integrate(Expr e, string x)
        {
            C.CheckAbort();
            var X = new SymE(x);
            if (Free(e, x)) return Simp.Mul(e, X);
            switch (e)
            {
                case SymE _: return Simp.Div(Simp.Pow(X, Expr.Two), Expr.Two);
                case AddE a:
                    {
                        var parts = new List<Expr>();
                        foreach (var t in a.Terms)
                        {
                            var r = Integrate(t, x);
                            if (r == null) return null;
                            parts.Add(r);
                        }
                        return Simp.Add(parts);
                    }
                case MulE m:
                    {
                        var consts = m.Factors.Where(f => Free(f, x)).ToList();
                        var vars = m.Factors.Where(f => !Free(f, x)).ToList();
                        if (consts.Count > 0)
                        {
                            var inner = Integrate(Simp.Mul(vars), x);
                            return inner == null ? null : Simp.Mul(Simp.Mul(consts), inner);
                        }
                        var byParts = IntegrateByParts(vars, x);
                        if (byParts != null) return byParts;
                        var expanded = Simp.Expand(e);
                        if (expanded is AddE) return Integrate(expanded, x);
                        return RationalIntegral(e, x);
                    }
                case PowE p:
                    {
                        if (Free(p.X, x) && Linear(p.B, x, out var a, out _))
                        {
                            if (p.X.IsMinusOne) return Simp.Div(F("ln", F("abs", p.B)), a);
                            var n1 = Simp.Add(p.X, Expr.One);
                            return Simp.Div(Simp.Pow(p.B, n1), Simp.Mul(a, n1));
                        }
                        if (Free(p.B, x) && Linear(p.X, x, out var a2, out _))
                            return Simp.Div(e, Simp.Mul(a2, F("ln", p.B)));
                        if (Free(p.X, x) && p.X is NumE pn && pn.V.IsExactInt && pn.V.Re.N.Sign > 0 && pn.V.Re.N <= 20)
                            return Integrate(Simp.Expand(e), x);
                        return RationalIntegral(e, x);
                    }
                case CallE c: return IntegrateCall(c, x);
            }
            return null;
        }

        static bool Linear(Expr e, string x, out Expr a, out Expr b)
        {
            a = b = null;
            if (!Polynomials.TryCoefficients(e, x, out var cs) || cs.Count != 2) return false;
            a = cs[1]; b = cs[0];
            return !a.IsZero;
        }

        static Expr IntegrateCall(CallE c, string x)
        {
            if (c.Args.Count == 0 || !Linear(c.Args[0], x, out var a, out _)) return null;
            var u = c.Args[0];
            Expr r;
            Expr ang = AngleFactorInverse();
            switch (c.F.Name)
            {
                case "sin": r = Simp.Neg(Simp.Div(F("cos", u), ang)); break;
                case "cos": r = Simp.Div(F("sin", u), ang); break;
                case "tan": r = Simp.Neg(Simp.Div(F("ln", F("abs", F("cos", u))), ang)); break;
                case "sec": r = Simp.Div(F("ln", F("abs", Simp.Add(F("sec", u), F("tan", u)))), ang); break;
                case "exp": r = c; break;
                case "ln": r = Simp.Sub(Simp.Mul(u, F("ln", u)), u); break;
                case "sinh": r = F("cosh", u); break;
                case "cosh": r = F("sinh", u); break;
                case "tanh": r = F("ln", F("cosh", u)); break;
                case "sqrt": r = Simp.Mul(Expr.N(Real.Frac(2, 3)), Simp.Pow(u, Expr.N(Real.Frac(3, 2)))); break;
                case "cbrt": r = Simp.Mul(Expr.N(Real.Frac(3, 4)), Simp.Pow(u, Expr.N(Real.Frac(4, 3)))); break;
                case "abs": r = Simp.Div(Simp.Mul(u, F("abs", u)), Expr.Two); break;
                case "asin": r = Simp.Add(Simp.Mul(u, c), Simp.Div(F("sqrt", Simp.Sub(Expr.One, Simp.Pow(u, Expr.Two))), ang)); break;
                case "atan": r = Simp.Sub(Simp.Mul(u, c), Simp.Div(Simp.Div(F("ln", Simp.Add(Expr.One, Simp.Pow(u, Expr.Two))), Expr.Two), ang)); break;
                case "erf": r = Simp.Add(Simp.Mul(u, c), Simp.Div(F("exp", Simp.Neg(Simp.Pow(u, Expr.Two))), F("sqrt", C.VarRef(C.FindVariable("pi"))))); break;
                case "log":
                    if (c.Args.Count > 1) return Integrate(Simp.Div(F("ln", u), F("ln", c.Args[1])), x);
                    r = Simp.Sub(Simp.Mul(u, F("ln", u)), u); break;
                default: return null;
            }
            return Simp.Div(r, a);
        }

        // polynomial times exp/sin/cos via repeated integration by parts
        static Expr IntegrateByParts(List<Expr> vars, string x)
        {
            if (vars.Count != 2) return null;
            int pi = -1;
            for (int i = 0; i < 2; i++) if (Polynomials.TryCoefficients(vars[i], x, out var cs) && cs.Count > 1 && cs.Skip(1).Any(z => !z.IsZero)) pi = i;
            if (pi < 0) return null;
            var poly = vars[pi]; var other = vars[1 - pi];
            bool ok = other is CallE oc && (oc.F.Name == "exp" || oc.F.Name == "sin" || oc.F.Name == "cos" || oc.F.Name == "sinh" || oc.F.Name == "cosh") && Linear(oc.Args[0], x, out _, out _)
                || other is PowE op && Free(op.B, x) && Linear(op.X, x, out _, out _);
            if (!ok)
            {
                if (other is CallE lc && lc.F.Name == "ln" && Linear(lc.Args[0], x, out _, out _))
                {
                    var ip = Integrate(poly, x);
                    if (ip == null) return null;
                    var rest = Integrate(Simp.Expand(Simp.Mul(ip, Diff(other, x))), x);
                    return rest == null ? null : Simp.Sub(Simp.Mul(ip, other), rest);
                }
                return null;
            }
            var terms = new List<Expr>();
            var cur = poly;
            var v = Integrate(other, x);
            if (v == null) return null;
            int sign = 1;
            for (int k = 0; k < 40 && !cur.IsZero; k++)
            {
                terms.Add(sign > 0 ? Simp.Mul(cur, v) : Simp.Neg(Simp.Mul(cur, v)));
                cur = Diff(cur, x);
                v = Integrate(v, x);
                if (v == null) return null;
                sign = -sign;
            }
            return Simp.Add(terms);
        }

        static Expr RationalIntegral(Expr e, string x)
        {
            var X = new SymE(x);
            var pf = Polynomials.PartialFractions(C, e);
            if (pf.Key != e.Key && !(pf is MulE))
            {
                var terms = pf is AddE a ? a.Terms : new List<Expr> { pf };
                var res = new List<Expr>();
                foreach (var t in terms)
                {
                    var r = Integrate(t, x);
                    if (r == null) r = QuadraticIntegral(t, x);
                    if (r == null) return null;
                    res.Add(r);
                }
                return Simp.Add(res);
            }
            return QuadraticIntegral(e, x);
        }

        // (b x + c)/(x^2 + p x + q) forms
        static Expr QuadraticIntegral(Expr e, string x)
        {
            var X = new SymE(x);
            Simp.NumDen(e, out var num, out var den);
            if (!Polynomials.TryPoly(num, x, out var pn) || !Polynomials.TryPoly(den, x, out var pd)) return null;
            if (pd.Degree == 1 && pn.Degree == 0)
                return Simp.Div(Simp.Mul(Expr.N(pn.C[0]), F("ln", F("abs", den))), Expr.N(pd.C[1]));
            if (pd.Degree != 2 || pn.Degree > 1) return null;
            var A = pd.C[2]; var B = pd.C[1]; var Cc = pd.C[0];
            var b = pn.Degree == 1 ? pn.C[1] : Real.Zero; var c = pn.C[0];
            var disc = Real.Int(4) * A * Cc - B * B;
            var parts = new List<Expr>();
            var k1 = b / (Real.Two * A);
            if (!k1.IsZero) parts.Add(Simp.Mul(Expr.N(k1), F("ln", F("abs", den))));
            var rem = c - k1 * B;
            if (!rem.IsZero)
            {
                if (disc.Sign > 0)
                {
                    var s = F("sqrt", Expr.N(disc));
                    var ang = AngleFactorInverse();
                    parts.Add(Simp.Mul(Expr.N(Real.Two * rem), Simp.Pow(s, Expr.MinusOne), Simp.Mul(F("atan", Simp.Div(Simp.Add(Simp.Mul(Expr.N(Real.Two * A), X), Expr.N(B)), s)), ang)));
                }
                else if (disc.Sign < 0)
                {
                    var s = F("sqrt", Expr.N(-disc));
                    var lin = Simp.Add(Simp.Mul(Expr.N(Real.Two * A), X), Expr.N(B));
                    parts.Add(Simp.Mul(Expr.N(rem), Simp.Pow(s, Expr.MinusOne), F("ln", F("abs", Simp.Div(Simp.Sub(lin, s), Simp.Add(lin, s))))));
                }
                else
                {
                    var lin = Simp.Add(Simp.Mul(Expr.N(Real.Two * A), X), Expr.N(B));
                    parts.Add(Simp.Mul(Expr.N(-Real.Two * rem), Simp.Pow(lin, Expr.MinusOne)));
                }
            }
            return Simp.Add(parts);
        }

        // adaptive Gauss-Kronrod 7-15
        static readonly double[] XK = { 0.991455371120812639, 0.949107912342758525, 0.864864423359769073, 0.741531185599394440, 0.586087235467691130, 0.405845151377397167, 0.207784955007898468, 0 };
        static readonly double[] WK = { 0.022935322010529225, 0.063092092629978553, 0.104790010322250184, 0.140653259715525919, 0.169004726639267903, 0.190350578064785410, 0.204432940075298892, 0.209482141084727828 };
        static readonly double[] WG = { 0.129484966168869693, 0.279705391489276668, 0.381830050505118945, 0.417959183673469388 };

        public static double NumIntegrate(Func<double, double> f, double a, double b, out double err)
        {
            if (double.IsInfinity(a) || double.IsInfinity(b))
            {
                if (double.IsNegativeInfinity(a) && double.IsPositiveInfinity(b))
                    return NumIntegrate(t => { double x = t / (1 - t * t); return f(x) * (1 + t * t) / Math.Pow(1 - t * t, 2); }, -1 + 1e-12, 1 - 1e-12, out err);
                if (double.IsPositiveInfinity(b))
                    return NumIntegrate(t => { double x = a + t / (1 - t); return f(x) / Math.Pow(1 - t, 2); }, 0, 1 - 1e-12, out err);
                return -NumIntegrate(t => { double x = b - t / (1 - t); return f(x) / Math.Pow(1 - t, 2); }, 0, 1 - 1e-12, out err);
            }
            err = 0;
            var stack = new Stack<(double a, double b, int depth)>();
            stack.Push((a, b, 0));
            double total = 0;
            int evals = 0;
            while (stack.Count > 0)
            {
                C.CheckAbort();
                var (lo, hi, d) = stack.Pop();
                double r = GK(f, lo, hi, out double e);
                evals += 15;
                if (double.IsNaN(r)) throw new CalcException("The integral could not be calculated (undefined values in range)");
                if (e < 1e-13 * Math.Max(1, Math.Abs(r)) || d > 40 || evals > 400000)
                {
                    total += r; err += e;
                }
                else
                {
                    double m = (lo + hi) / 2;
                    stack.Push((lo, m, d + 1)); stack.Push((m, hi, d + 1));
                }
            }
            return total;
        }

        static double GK(Func<double, double> f, double a, double b, out double err)
        {
            double c = (a + b) / 2, h = (b - a) / 2;
            double fc = f(c);
            double k = fc * WK[7], g = fc * WG[3];
            for (int i = 0; i < 7; i++)
            {
                double dx = h * XK[i];
                double f1 = f(c - dx), f2 = f(c + dx);
                k += WK[i] * (f1 + f2);
                if (i % 2 == 1) g += WG[i / 2] * (f1 + f2);
            }
            err = Math.Abs((k - g) * h);
            return k * h;
        }

        public static Expr Limit(Expr e, string x, Expr a, int dir)
        {
            var save = C.ExactPhase;
            int bitsSave = Real.Bits;
            try
            {
                if (!(a is NumE an && an.V.IsInfinite))
                {
                    try
                    {
                        var direct = C.Ev.EvalWith(e, x, a);
                        if (direct is NumE dn && !dn.V.IsInfinite && dir == 0) return direct;
                    }
                    catch (CalcException) { }
                    Simp.NumDen(e, out var num, out var den);
                    if (!den.IsOne)
                    {
                        for (int k = 0; k < 6; k++)
                        {
                            Expr nv, dv;
                            try { nv = C.Ev.EvalWith(num, x, a); dv = C.Ev.EvalWith(den, x, a); }
                            catch (CalcException) { break; }
                            if (!(nv.IsZero && dv.IsZero)) break;
                            num = Diff(num, x); den = Diff(den, x);
                            try
                            {
                                var r = C.Ev.EvalWith(Simp.Div(num, den), x, a);
                                if (r is NumE rn && !rn.V.IsInfinite) return r;
                            }
                            catch (CalcException) { }
                        }
                    }
                }
                C.ExactPhase = false;
                Real.Bits += 96;
                double av = a is NumE ann ? ann.V.Re.ToDouble() : double.NaN;
                if (double.IsNaN(av)) throw new CalcException("Limit point must be a number");
                var seq = new List<double>();
                for (int k = 2; k <= 9; k++)
                {
                    double xv;
                    if (double.IsPositiveInfinity(av)) xv = Math.Pow(10, k + 1);
                    else if (double.IsNegativeInfinity(av)) xv = -Math.Pow(10, k + 1);
                    else xv = av + (dir < 0 ? -1 : 1) * Math.Pow(10, -k);
                    seq.Add(C.Ev.EvalDouble(e, x, xv));
                }
                if (dir == 0 && !double.IsInfinity(av))
                {
                    var other = C.Ev.EvalDouble(e, x, av - 1e-9);
                    if (!double.IsNaN(other) && !double.IsNaN(seq.Last()) && Math.Abs(other - seq.Last()) > 1e-4 * Math.Max(1, Math.Abs(other))) throw new CalcException("The limit does not exist (left and right limits differ)");
                }
                double last = seq.Last(), prev = seq[seq.Count - 2];
                if (double.IsNaN(last)) throw new CalcException("Unable to compute the limit");
                if (Math.Abs(last) > 1e12 && Math.Abs(last) > Math.Abs(prev) * 10) return Expr.N(last > 0 ? Real.PosInf : Real.NegInf);
                if (Math.Abs(last - prev) > 1e-5 * Math.Max(1, Math.Abs(last))) throw new CalcException("The limit does not seem to converge");
                return Expr.N(Real.FromDouble(Math.Round(last, 10) == Math.Round(last, 10) ? RoundNice(last) : last));
            }
            finally { C.ExactPhase = save; Real.Bits = bitsSave; }
        }

        static double RoundNice(double v)
        {
            double r = Math.Round(v, 9);
            return Math.Abs(r - v) < 1e-9 ? r : v;
        }
    }
}
