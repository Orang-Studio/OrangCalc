using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Numerics;
using System.Text;
using System.Text.RegularExpressions;

namespace OrangCalc.Engine
{
    public static partial class Funcs
    {
        static Calculator C => Calculator.Current;

        static MathFunction Def(Calculator c, string names, string cat, string title, string args, FnImpl impl, string desc = "", bool ew = false)
        {
            var f = new MathFunction { Names = names.Split(',').ToList(), Category = cat, Title = title, Description = desc, Impl = impl, Elementwise = ew };
            f.Name = f.Names[0];
            int min = 0;
            f.MaxArgs = 0;
            if (args.Length > 0)
            {
                foreach (var part in args.Split(','))
                {
                    var a = part.Trim();
                    if (a == "...") { f.MaxArgs = -1; continue; }
                    string def = null;
                    bool opt = false;
                    int eq = a.IndexOf('=');
                    if (eq >= 0) { def = a.Substring(eq + 1); a = a.Substring(0, eq); opt = true; }
                    if (a.EndsWith("?")) { opt = true; a = a.TrimEnd('?'); }
                    var nt = a.Split(':');
                    var ad = new ArgDef(nt[0], nt.Length > 1 ? ParseType(nt[1]) : ArgType.Any, def);
                    if (ad.Type == ArgType.Unevaluated) f.HoldArgs.Add(f.Args.Count);
                    f.Args.Add(ad);
                    if (!opt) min = f.Args.Count;
                    if (f.MaxArgs >= 0) f.MaxArgs = f.Args.Count;
                }
            }
            f.MinArgs = min;
            c.AddFunction(f);
            return f;
        }

        static ArgType ParseType(string s)
        {
            switch (s)
            {
                case "num": return ArgType.Number;
                case "int": return ArgType.Integer;
                case "real": return ArgType.Real;
                case "nnint": return ArgType.NonNegInt;
                case "pint": return ArgType.PositiveInt;
                case "mat": return ArgType.Matrix;
                case "vec": return ArgType.Vector;
                case "text": return ArgType.Text;
                case "sym": return ArgType.Symbol;
                case "bool": return ArgType.Boolean;
                case "date": return ArgType.Date;
                case "angle": return ArgType.Angle;
                case "hold": return ArgType.Unevaluated;
                default: return ArgType.Any;
            }
        }

        static bool IsN(Expr e, out Cx v) { v = (e as NumE)?.V; return v != null; }
        static bool IsR(Expr e, out Real r) { r = (e is NumE n && n.V.IsReal) ? n.V.Re : null; return r != null; }
        static bool IsI(Expr e, out BigInteger i)
        {
            i = 0;
            if (e is NumE n && n.V.IsReal && n.V.Re.IsInt) { i = n.V.Re.N; return true; }
            return false;
        }
        static Expr Num(Real r) => new NumE(new Cx(r));
        static Expr Num(Cx v) => new NumE(v);
        static Expr Num(BigInteger i) => new NumE(new Cx(Real.Int(i)));
        static Expr Num(double d) => new NumE(new Cx(Real.FromDouble(d)));
        static Expr NumC(System.Numerics.Complex z) => new NumE(Cx.FromComplex(z).Clean());
        static Expr Bool(bool b) => b ? Expr.One : Expr.Zero;

        static BigInteger Int(Expr e, string fn)
        {
            if (IsI(e, out var i)) return i;
            throw new CalcException(fn + "() requires integer arguments");
        }

        static int SmallInt(Expr e, string fn)
        {
            var i = Int(e, fn);
            if (BigInteger.Abs(i) > int.MaxValue) throw new CalcException(fn + "() argument too large");
            return (int)i;
        }

        static double D(Expr e, string fn)
        {
            if (IsR(e, out var r)) return r.ToDouble();
            throw new CalcException(fn + "() requires real arguments");
        }

        static string Str(Expr e)
        {
            if (e is StrE s) return s.S;
            if (e is SymE sy) return sy.Name;
            if (e is VarE v) return v.V.Name;
            if (e is UnitE u) return u.U.Name;
            return Formatter.PlainText(e);
        }

        // numeric unary function: exact attempt first, then approximation outside the exact phase
        static FnImpl U(Func<Cx, Expr> exact, Func<Cx, Cx> approx) => (c, a) =>
        {
            if (!(a[0] is NumE n)) return null;
            if (exact != null && !n.V.Approx) { var r = exact(n.V); if (r != null) return r; }
            if (c.ExactPhase && !n.V.Approx) return null;
            var res = approx(n.V.Approx ? n.V : n.V.AsApprox());
            return new NumE(res.Clean());
        };

        static FnImpl RD(Func<double, double> f, Func<Cx, Expr> exact = null) => U(exact, v =>
        {
            if (!v.IsReal) throw new CalcException("Complex argument is not supported here");
            return new Cx(Real.FromDouble(f(v.Re.ToDouble())));
        });

        static FnImpl CD(Func<System.Numerics.Complex, System.Numerics.Complex> f) => U(null, v => Cx.FromComplex(f(v.ToComplex())));

        static FnImpl R2(Func<double, double, double> f) => (c, a) =>
        {
            if (!IsR(a[0], out var x) || !IsR(a[1], out var y)) return null;
            if (c.ExactPhase && !x.Approx && !y.Approx) return null;
            return Num(f(x.ToDouble(), y.ToDouble()));
        };

        public static readonly Dictionary<string, Expr> AngleUnitsExact = new Dictionary<string, Expr>();

        public static Expr Pi => C.VarRef(C.FindVariable("pi"));

        // converts an argument in the default angle unit, or with an angle unit, to radians
        public static Expr ToRadians(Expr x)
        {
            if (x.HasUnits)
            {
                Simp.SplitCoef(x, out var cf, out var rest);
                Simp.SplitUnits(rest, out var plain, out var units);
                if (units is UnitE ue && ue.P == null)
                {
                    Expr fac = null;
                    switch (ue.U.Name)
                    {
                        case "rad": fac = Expr.One; break;
                        case "°": fac = Simp.Div(Pi, Expr.N(180)); break;
                        case "arcmin": fac = Simp.Div(Pi, Expr.N(10800)); break;
                        case "arcsec": fac = Simp.Div(Pi, Expr.N(648000)); break;
                        case "gon": fac = Simp.Div(Pi, Expr.N(200)); break;
                        case "tr": fac = Simp.Mul(Expr.Two, Pi); break;
                    }
                    if (fac != null) return Simp.Mul(new NumE(cf), plain, fac);
                }
                if (Units.TryQuantity(x, out var v, out var d, out var r2) && d.SameAs(new Dim(9))) return Simp.Mul(new NumE(v), r2);
                throw new CalcException("Invalid unit for an angle");
            }
            switch (C.Opts.Angle)
            {
                case AngleMode.Degrees: return Simp.Mul(x, Simp.Div(Pi, Expr.N(180)));
                case AngleMode.Gradians: return Simp.Mul(x, Simp.Div(Pi, Expr.N(200)));
                default: return x;
            }
        }

        public static Expr FromRadians(Expr r)
        {
            switch (C.Opts.Angle)
            {
                case AngleMode.Degrees: return Simp.Mul(r, Simp.Div(Expr.N(180), Pi));
                case AngleMode.Gradians: return Simp.Mul(r, Simp.Div(Expr.N(200), Pi));
                default: return r;
            }
        }

        // rational multiple of pi, or null
        static Real PiMultiple(Expr e)
        {
            if (e.IsZero) return Real.Zero;
            if (e is VarE v && v.V.Name == "pi") return Real.One;
            if (e is MulE m && m.Factors.Count == 2 && m.Factors[0] is NumE n && n.V.IsRational && m.Factors[1] is VarE pv && pv.V.Name == "pi") return n.V.Re;
            return null;
        }

        static readonly Dictionary<int, Expr> sinTable = new Dictionary<int, Expr>();

        static Expr SinTwelfths(int k)
        {
            k = ((k % 24) + 24) % 24;
            if (sinTable.TryGetValue(k, out var v)) return v;
            string[] baseVals = { "0", "(sqrt(6)-sqrt(2))/4", "1/2", "sqrt(2)/2", "sqrt(3)/2", "(sqrt(6)+sqrt(2))/4", "1" };
            int kk = k % 12;
            int idx = kk <= 6 ? kk : 12 - kk;
            var save = C.ExactPhase;
            C.ExactPhase = true;
            try
            {
                var e = C.Ev.Eval(Parser.ParseText(baseVals[idx], C));
                if (k >= 12) e = Simp.Neg(e);
                sinTable[k] = e;
                return e;
            }
            finally { C.ExactPhase = save; }
        }

        static Expr ExactTrig(string fn, Expr rad)
        {
            var q = PiMultiple(rad);
            if (q == null) return null;
            var t = q * Real.Int(12);
            if (!t.IsInt) return null;
            int k = (int)(t.N % 24);
            switch (fn)
            {
                case "sin": return SinTwelfths(k);
                case "cos": return SinTwelfths(k + 6);
                case "tan":
                    {
                        var s = SinTwelfths(k); var c = SinTwelfths(k + 6);
                        if (c.IsZero) throw new CalcException("tan() is undefined at this angle");
                        return Simp.Div(s, c);
                    }
            }
            return null;
        }

        public static Expr ExactTrigPublic(string fn, Expr rad) => ExactTrig(fn, rad);

        static Expr ExactInverseTrig(string fn, Expr v)
        {
            int lo, hi;
            Func<int, Expr> val;
            switch (fn)
            {
                case "asin": lo = -6; hi = 6; val = k => SinTwelfths(k); break;
                case "acos": lo = 0; hi = 12; val = k => SinTwelfths(k + 6); break;
                case "atan":
                    lo = -5; hi = 5;
                    val = k => Simp.Div(SinTwelfths(k), SinTwelfths(k + 6));
                    break;
                default: return null;
            }
            for (int k = lo; k <= hi; k++)
            {
                if (val(k).Key == v.Key)
                    return FromRadians(Simp.Mul(Expr.N(Real.Make(k, 12)), Pi));
            }
            return null;
        }

        static FnImpl Trig(string name, Func<Cx, Cx> f) => (c, a) =>
        {
            var rad = ToRadians(a[0]);
            var ex = ExactTrig(name, rad);
            if (ex != null) return ex;
            if (rad is NumE n)
            {
                if (c.ExactPhase && !n.V.Approx) return null;
                return Num(f(n.V.AsApprox()).Clean());
            }
            if (!c.ExactPhase && !rad.HasSymbols)
            {
                var r2 = c.Ev.Eval(rad);
                if (r2 is NumE n2) return Num(f(n2.V.AsApprox()).Clean());
            }
            if (rad.Key != a[0].Key && !(rad is NumE)) return new CallE(c.Fn(name), FromRadiansKeep(a[0]));
            return null;
        };

        static Expr FromRadiansKeep(Expr e) => e;

        static FnImpl InvTrig(string name, Func<Cx, Cx> f) => (c, a) =>
        {
            var ex = ExactInverseTrig(name, a[0]);
            if (ex != null) return ex;
            if (!(a[0] is NumE n)) return null;
            if (c.ExactPhase && !n.V.Approx) return null;
            var r = f(n.V.AsApprox()).Clean();
            return c.Ev.Eval(FromRadians(Num(r)));
        };

        static Expr Reciprocal(Calculator c, string fn, Expr x)
        {
            var v = c.Ev.Call(fn, x);
            if (v is CallE) return null;
            if (v.IsZero) throw new CalcException("Division by zero");
            return Simp.Pow(v, Expr.MinusOne);
        }

        public static void Register(Calculator c)
        {
            RegisterMath(c);
            RegisterMore(c);
        }

        static void RegisterMath(Calculator c)
        {
            const string TR = "Trigonometry", EL = "Exponents & Logarithms", NT = "Number Theory", AR = "Arithmetics", RO = "Rounding", INT = "Integers", PR = "Prime Numbers", SP = "Special Functions", CX = "Complex Numbers", CB = "Combinatorics";

            Def(c, "sin", TR, "Sine", "x:angle", Trig("sin", Cx.Sin), "Returns the sine of an angle.", true);
            Def(c, "cos", TR, "Cosine", "x:angle", Trig("cos", Cx.Cos), "Returns the cosine of an angle.", true);
            Def(c, "tan", TR, "Tangent", "x:angle", Trig("tan", Cx.Tan), "Returns the tangent of an angle.", true);
            Def(c, "cot", TR, "Cotangent", "x:angle", (cc, a) => Reciprocal(cc, "tan", a[0]), "", true);
            Def(c, "sec", TR, "Secant", "x:angle", (cc, a) => Reciprocal(cc, "cos", a[0]), "", true);
            Def(c, "csc", TR, "Cosecant", "x:angle", (cc, a) => Reciprocal(cc, "sin", a[0]), "", true);
            Def(c, "asin,arcsin", TR, "Arcsine", "x:num", InvTrig("asin", Cx.Asin), "Returns the angle whose sine is x.", true);
            Def(c, "acos,arccos", TR, "Arccosine", "x:num", InvTrig("acos", Cx.Acos), "", true);
            Def(c, "atan,arctan", TR, "Arctangent", "x:num", InvTrig("atan", Cx.Atan), "", true);
            Def(c, "acot,arccot", TR, "Arccotangent", "x:num", (cc, a) => a[0].IsZero ? cc.Ev.Eval(FromRadians(Simp.Div(Pi, Expr.Two))) : cc.Ev.Call("atan", Simp.Pow(a[0], Expr.MinusOne)), "", true);
            Def(c, "asec,arcsec_f", TR, "Arcsecant", "x:num", (cc, a) => cc.Ev.Call("acos", Simp.Pow(a[0], Expr.MinusOne)), "", true);
            Def(c, "acsc,arccsc", TR, "Arccosecant", "x:num", (cc, a) => cc.Ev.Call("asin", Simp.Pow(a[0], Expr.MinusOne)), "", true);
            Def(c, "sinh", TR, "Hyperbolic Sine", "x:num", U(v => v.IsZero ? Expr.Zero : null, Cx.Sinh), "", true);
            Def(c, "cosh", TR, "Hyperbolic Cosine", "x:num", U(v => v.IsZero ? Expr.One : null, Cx.Cosh), "", true);
            Def(c, "tanh", TR, "Hyperbolic Tangent", "x:num", U(v => v.IsZero ? Expr.Zero : null, Cx.Tanh), "", true);
            Def(c, "coth", TR, "Hyperbolic Cotangent", "x:num", (cc, a) => Reciprocal(cc, "tanh", a[0]), "", true);
            Def(c, "sech", TR, "Hyperbolic Secant", "x:num", (cc, a) => Reciprocal(cc, "cosh", a[0]), "", true);
            Def(c, "csch", TR, "Hyperbolic Cosecant", "x:num", (cc, a) => Reciprocal(cc, "sinh", a[0]), "", true);
            Def(c, "asinh,arsinh", TR, "Inverse Hyperbolic Sine", "x:num", U(v => v.IsZero ? Expr.Zero : null, Cx.Asinh), "", true);
            Def(c, "acosh,arcosh", TR, "Inverse Hyperbolic Cosine", "x:num", U(v => v.IsOne ? Expr.Zero : null, Cx.Acosh), "", true);
            Def(c, "atanh,artanh", TR, "Inverse Hyperbolic Tangent", "x:num", U(v => v.IsZero ? Expr.Zero : null, Cx.Atanh), "", true);
            Def(c, "acoth", TR, "Inverse Hyperbolic Cotangent", "x:num", (cc, a) => cc.Ev.Call("atanh", Simp.Pow(a[0], Expr.MinusOne)), "", true);
            Def(c, "asech", TR, "Inverse Hyperbolic Secant", "x:num", (cc, a) => cc.Ev.Call("acosh", Simp.Pow(a[0], Expr.MinusOne)), "", true);
            Def(c, "acsch", TR, "Inverse Hyperbolic Cosecant", "x:num", (cc, a) => cc.Ev.Call("asinh", Simp.Pow(a[0], Expr.MinusOne)), "", true);
            Def(c, "atan2", TR, "Four-quadrant Arctangent", "y:real,x:real", (cc, a) =>
            {
                if (!IsR(a[0], out var y) || !IsR(a[1], out var x)) return null;
                if (x.Sign > 0 && cc.ExactPhase && !x.Approx && !y.Approx) return cc.Ev.Call("atan", Num(y / x));
                if (x.IsZero && cc.ExactPhase) return y.Sign == 0 ? Expr.Zero : cc.Ev.Eval(FromRadians(Simp.Mul(Expr.N(Real.Frac(y.Sign, 2)), Pi)));
                if (cc.ExactPhase && !x.Approx && !y.Approx) return null;
                return cc.Ev.Eval(FromRadians(Num(RMath.Atan2(y.AsApprox(), x.AsApprox()))));
            });
            Def(c, "sinc", TR, "Cardinal Sine", "x:num", (cc, a) =>
            {
                if (a[0].IsZero) return Expr.One;
                var s = cc.Ev.Call("sin", a[0]);
                if (s is CallE) return null;
                return Simp.Div(s, ToRadians(a[0]));
            }, "sin(x)/x, unnormalized.", true);
            Def(c, "hypot", TR, "Hypotenuse", "a:num,b:num,...", (cc, a) => Simp.Pow(Simp.Add(a.Select(x => Simp.Pow(x, Expr.Two)).ToList()), Expr.Half), "Square root of the sum of squares.");
            Def(c, "deg,degrees", TR, "Radians to Degrees", "x:num", (cc, a) => Simp.Mul(a[0], Simp.Div(Expr.N(180), Pi)), "", true);
            Def(c, "radians", TR, "Degrees to Radians", "x:num", (cc, a) => Simp.Mul(a[0], Simp.Div(Pi, Expr.N(180))), "", true);

            Def(c, "exp", EL, "Exponential", "x:num", (cc, a) =>
            {
                if (a[0] is CallE lc && lc.F.Name == "ln") return lc.Args[0];
                return Simp.Pow(cc.VarRef(cc.FindVariable("e")), a[0]);
            }, "e raised to the power of x.", true);
            Def(c, "exp2", EL, "Base-2 Exponential", "x:num", (cc, a) => Simp.Pow(Expr.Two, a[0]), "", true);
            Def(c, "exp10", EL, "Base-10 Exponential", "x:num", (cc, a) => Simp.Pow(Expr.N(10), a[0]), "", true);
            Def(c, "expm1", EL, "exp(x) - 1", "x:num", U(v => v.IsZero ? Expr.Zero : null, v => Cx.Exp(v) - Cx.One), "", true);
            Def(c, "ln", EL, "Natural Logarithm", "x:num", (cc, a) =>
            {
                var x = a[0];
                if (x.IsOne) return Expr.Zero;
                if (x is VarE ve && ve.V.Name == "e") return Expr.One;
                if (x is PowE pe && pe.B is VarE pv && pv.V.Name == "e" && cc.IsKnownReal(pe.X)) return pe.X;
                if (x is NumE n)
                {
                    if (n.V.IsZero) throw new CalcException("Logarithm of zero");
                    if (n.V.IsRational && n.V.Re.Sign > 0 && !n.V.Re.D.IsOne && n.V.Re.N.IsOne && cc.ExactPhase) return Simp.Neg(cc.Ev.Call("ln", Num(Real.Int(n.V.Re.D))));
                    if (cc.ExactPhase && !n.V.Approx) return null;
                    return Num(Cx.Ln(n.V.AsApprox()).Clean());
                }
                if (x is MatE) return null;
                return null;
            }, "Natural logarithm of x.", true);
            Def(c, "log", EL, "Logarithm", "x:num,base=e", (cc, a) =>
            {
                var b = a[1];
                if (b is VarE bv && bv.V.Name == "e") return cc.Ev.Call("ln", a[0]);
                if (IsR(a[0], out var x) && IsR(b, out var br) && !x.Approx && !br.Approx && x.Sign > 0 && br.Sign > 0)
                {
                    var ex = ExactLog(x, br);
                    if (ex != null) return Num(ex);
                    if (cc.ExactPhase) return null;
                }
                var ln1 = cc.Ev.Call("ln", a[0]); var ln2 = cc.Ev.Call("ln", b);
                if (ln1 is CallE || ln2 is CallE) return null;
                return Simp.Div(ln1, ln2);
            }, "Logarithm of x with the given base.", true);
            Def(c, "log2", EL, "Base-2 Logarithm", "x:num", (cc, a) => cc.Ev.Call("log", a[0], Expr.Two) is CallE ? null : cc.Ev.Call("log", a[0], Expr.Two), "", true);
            Def(c, "log10,lg", EL, "Base-10 Logarithm", "x:num", (cc, a) => cc.Ev.Call("log", a[0], Expr.N(10)) is CallE ? null : cc.Ev.Call("log", a[0], Expr.N(10)), "", true);
            Def(c, "logn", EL, "Logarithm with Base", "x:num,base:num", (cc, a) => cc.Ev.Call("log", a[0], a[1]) is CallE ? null : cc.Ev.Call("log", a[0], a[1]), "", true);
            Def(c, "log1p", EL, "ln(1 + x)", "x:num", U(v => v.IsZero ? Expr.Zero : null, v => Cx.Ln(Cx.One + v)), "", true);
            Def(c, "sqrt", EL, "Square Root", "x:num", (cc, a) => Simp.Pow(a[0], Expr.Half), "Principal square root.", true);
            Def(c, "cbrt", EL, "Cube Root", "x:num", (cc, a) =>
            {
                if (IsR(a[0], out var r) && r.Sign < 0)
                {
                    var save = cc.Opts.RealRoots; cc.Opts.RealRoots = true;
                    try { return Simp.Pow(a[0], Expr.N(Real.Frac(1, 3))); } finally { cc.Opts.RealRoots = save; }
                }
                return Simp.Pow(a[0], Expr.N(Real.Frac(1, 3)));
            }, "Real cube root.", true);
            Def(c, "root", EL, "Nth Root", "x:num,n:num", (cc, a) =>
            {
                if (IsR(a[1], out var n) && n.IsZero) throw new CalcException("root() of degree zero");
                var save = cc.Opts.RealRoots; cc.Opts.RealRoots = true;
                try { return Simp.Pow(a[0], Simp.Pow(a[1], Expr.MinusOne)); } finally { cc.Opts.RealRoots = save; }
            }, "Real nth root of x.");
            Def(c, "sq", EL, "Square", "x", (cc, a) => Simp.Pow(a[0], Expr.Two), "", true);
            Def(c, "pow", EL, "Power", "x,y", (cc, a) => Simp.Pow(a[0], a[1]));
            Def(c, "lambertw,W", EL, "Lambert W", "x:num,branch:int=0", (cc, a) =>
            {
                if (a[0].IsZero && (a.Count < 2 || a[1].IsZero)) return Expr.Zero;
                if (!(a[0] is NumE n)) return null;
                if (cc.ExactPhase && !n.V.Approx) return null;
                return NumC(Special.LambertW(n.V.ToComplex(), a.Count > 1 ? SmallInt(a[1], "lambertw") : 0));
            }, "Inverse of x·e^x.");
            Def(c, "powertower", EL, "Power Tower", "x:num,n:nnint", (cc, a) =>
            {
                int n = SmallInt(a[1], "powertower");
                if (n > 100) throw new CalcException("powertower() height too large");
                Expr r = Expr.One;
                for (int i = 0; i < n; i++) r = Simp.Pow(a[0], r);
                return r;
            });
            Def(c, "cis", CX, "cis", "x:angle", (cc, a) =>
            {
                var cs = cc.Ev.Call("cos", a[0]); var sn = cc.Ev.Call("sin", a[0]);
                if (cs is CallE || sn is CallE) return null;
                return Simp.Add(cs, Simp.Mul(Num(Cx.I), sn));
            }, "cos(x) + i sin(x).", true);

            Def(c, "abs", AR, "Absolute Value", "x", (cc, a) =>
            {
                var x = a[0];
                if (x is NumE n) return Num(n.V.IsReal ? new Cx(n.V.Re.Abs()) : new Cx(n.V.Abs()));
                if (x is IntvE iv) return iv.Lo.Sign >= 0 ? x : iv.Hi.Sign <= 0 ? Simp.Neg(x) : new IntvE(Real.Zero, Real.Max(iv.Hi, -iv.Lo));
                if (cc.IsKnownNonNegative(x)) return x;
                if (x is MulE m && m.Factors[0] is NumE f) return Simp.Mul(Num(new Cx(f.V.Abs())), cc.Ev.Call("abs", Simp.Mul(m.Factors.Skip(1).ToList())));
                if (x.HasUnits && Units.TryQuantity(x, out var qv, out _, out var rest) && rest.IsOne && qv.IsReal)
                {
                    Simp.SplitCoef(x, out var cf, out var un);
                    return Simp.Mul(Num(new Cx(cf.Re.Abs())), un);
                }
                if (x is VarE) return x;
                return null;
            }, "Absolute value or modulus.", true);
            Def(c, "sgn,signum,sign", AR, "Signum", "x", (cc, a) =>
            {
                if (a[0] is NumE n)
                {
                    if (n.V.IsReal) return Num(Real.Int(n.V.Re.Sign));
                    if (cc.ExactPhase && !n.V.Approx) return null;
                    return Num(n.V / new Cx(n.V.Abs()));
                }
                if (cc.IsKnownPositive(a[0])) return Expr.One;
                return null;
            }, "", true);
            Def(c, "ceil,ceiling", RO, "Round Upwards", "x", (cc, a) => RoundFn(cc, a[0], r => r.Ceil()), "", true);
            Def(c, "floor", RO, "Round Downwards", "x", (cc, a) => RoundFn(cc, a[0], r => r.Floor()), "", true);
            Def(c, "trunc", RO, "Round Towards Zero", "x", (cc, a) => RoundFn(cc, a[0], r => r.Trunc()), "", true);
            Def(c, "int", RO, "Integer Part", "x", (cc, a) => RoundFn(cc, a[0], r => r.Trunc()), "", true);
            Def(c, "frac", RO, "Fractional Part", "x", (cc, a) => RoundFn(cc, a[0], r => r - r.Trunc()), "", true);
            Def(c, "round", RO, "Round", "x,decimals:int=0", (cc, a) =>
            {
                int d = a.Count > 1 ? SmallInt(a[1], "round") : 0;
                var scale = Real.Int(10).PowInt(d);
                return RoundFn(cc, a[0], r => (r * scale).Round(cc.Opts.Rounding) / scale);
            }, "Rounds to the nearest integer or to the given number of decimals.", true);
            Def(c, "roundhalfeven", RO, "Round Half to Even", "x", (cc, a) => RoundFn(cc, a[0], r => r.Round(RoundingMode.HalfEven)), "", true);
            Def(c, "roundhalfup", RO, "Round Half Up", "x", (cc, a) => RoundFn(cc, a[0], r => r.Round(RoundingMode.HalfUp)), "", true);
            Def(c, "roundhalfdown", RO, "Round Half Down", "x", (cc, a) => RoundFn(cc, a[0], r => r.Round(RoundingMode.HalfDown)), "", true);
            Def(c, "mod", AR, "Modulus (floored)", "a:real,b:real", (cc, a) =>
            {
                if (!IsR(a[0], out var x) || !IsR(a[1], out var y)) return Quantities2(cc, a, "mod");
                if (y.IsZero) throw new CalcException("Division by zero in mod()");
                return Num(x - y * (x / y).Floor());
            }, "Remainder with the sign of the divisor.");
            Def(c, "rem", AR, "Remainder (truncated)", "a:real,b:real", (cc, a) =>
            {
                if (!IsR(a[0], out var x) || !IsR(a[1], out var y)) return Quantities2(cc, a, "rem");
                if (y.IsZero) throw new CalcException("Division by zero in rem()");
                return Num(x - y * (x / y).Trunc());
            }, "Remainder with the sign of the dividend.");
            Def(c, "div,quotient", AR, "Integer Division", "a:real,b:real", (cc, a) =>
            {
                if (!IsR(a[0], out var x) || !IsR(a[1], out var y)) return null;
                if (y.IsZero) throw new CalcException("Division by zero");
                return Num((x / y).Floor());
            });
            Def(c, "gcd", INT, "Greatest Common Divisor", "a,b,...", (cc, a) =>
            {
                var items = Flatten(a);
                if (items.All(x => x is NumE n && n.V.IsRational))
                {
                    Real g = Real.Zero;
                    foreach (var x in items) g = RatGcd(g, ((NumE)x).V.Re);
                    return Num(g);
                }
                return null;
            });
            Def(c, "lcm", INT, "Least Common Multiple", "a,b,...", (cc, a) =>
            {
                var items = Flatten(a);
                if (!items.All(x => IsI(x, out _))) return null;
                BigInteger l = 1;
                foreach (var x in items)
                {
                    var v = BigInteger.Abs(((NumE)x).V.Re.N);
                    if (v.IsZero) return Expr.Zero;
                    l = l / BigInteger.GreatestCommonDivisor(l, v) * v;
                }
                return Num(l);
            });
            Def(c, "numerator,numer", AR, "Numerator", "x", (cc, a) => IsR(a[0], out var r) && !r.Approx ? Num(Real.Int(r.N)) : NumDenPart(a[0], true), "", true);
            Def(c, "denominator,denom", AR, "Denominator", "x", (cc, a) => IsR(a[0], out var r) && !r.Approx ? Num(Real.Int(r.D)) : NumDenPart(a[0], false), "", true);

            Def(c, "factorial", CB, "Factorial", "n", (cc, a) =>
            {
                if (IsI(a[0], out var n) && !((NumE)a[0]).V.Approx)
                {
                    if (n.Sign < 0) throw new CalcException("Factorial of negative integer is undefined");
                    if (n <= 100000) return Num(Primes.Factorial(n));
                }
                if (IsN(a[0], out var v))
                {
                    if (cc.ExactPhase && !v.Approx && v.IsRational && v.Re.D == 2) return HalfFactorial(cc, v.Re);
                    if (cc.ExactPhase && !v.Approx) return null;
                    if (v.IsReal) return Num(LogGammaReal(v.Re + Real.One));
                    return NumC(Special.Gamma(v.ToComplex() + 1));
                }
                return null;
            }, "n! = 1 × 2 × … × n", true);
            Def(c, "factorial2,doublefactorial", CB, "Double Factorial", "n:int", (cc, a) =>
            {
                var n = Int(a[0], "factorial2");
                if (n < -1) throw new CalcException("Double factorial undefined");
                if (n <= 0) return Expr.One;
                return Num(Primes.Factorial(n, 2));
            }, "n!! = n × (n−2) × …", true);
            Def(c, "multifactorial", CB, "Multifactorial", "n:nnint,k:pint", (cc, a) => Num(Primes.Factorial(Int(a[0], "multifactorial"), SmallInt(a[1], "multifactorial"))));
            Def(c, "binomial,choose,comb,nCr", CB, "Binomial Coefficient", "n,k:int", (cc, a) =>
            {
                if (IsI(a[0], out var n) && IsI(a[1], out var k)) return Num(Primes.Binomial(n, k));
                if (IsN(a[0], out var nv) && IsN(a[1], out var kv))
                {
                    if (cc.ExactPhase && !nv.Approx) return null;
                    var g = new Func<double, double>(Special.Gamma);
                    double nn = nv.Re.ToDouble(), kk = kv.Re.ToDouble();
                    return Num(g(nn + 1) / (g(kk + 1) * g(nn - kk + 1)));
                }
                if (IsI(a[1], out var k2) && k2 >= 0 && k2 < 50)
                {
                    Expr r = Expr.One;
                    for (int i = 0; i < (int)k2; i++) r = Simp.Mul(r, Simp.Sub(a[0], Expr.N(i)));
                    return Simp.Div(r, Num(Primes.Factorial(k2)));
                }
                return null;
            }, "Number of ways to choose k items from n.");
            Def(c, "perm,nPr", CB, "Permutations", "n:int,k:int", (cc, a) =>
            {
                var n = Int(a[0], "perm"); var k = Int(a[1], "perm");
                if (k < 0 || k > n) return Expr.Zero;
                return Num(Primes.Factorial(n) / Primes.Factorial(n - k));
            }, "n!/(n−k)!");
            Def(c, "derangements,subfactorial", CB, "Derangements", "n:nnint", (cc, a) => Num(Primes.Subfactorial(Int(a[0], "derangements"))), "", true);
            Def(c, "stirling2", CB, "Stirling Number of the Second Kind", "n:nnint,k:nnint", (cc, a) => Num(Primes.Stirling2(SmallInt(a[0], "stirling2"), SmallInt(a[1], "stirling2"))));
            Def(c, "catalan_number,catalannumber", CB, "Catalan Number", "n:nnint", (cc, a) => { var n = Int(a[0], "catalan_number"); return Num(Primes.Binomial(2 * n, n) / (n + 1)); }, "", true);
            Def(c, "partitions", CB, "Integer Partitions", "n:nnint", (cc, a) => Num(Primes.Partitions(SmallInt(a[0], "partitions"))), "", true);
            Def(c, "bell", CB, "Bell Number", "n:nnint", (cc, a) =>
            {
                int n = SmallInt(a[0], "bell");
                if (n > 2000) throw new CalcException("bell() argument too large");
                var row = new List<BigInteger> { 1 };
                for (int i = 0; i < n; i++)
                {
                    var next = new List<BigInteger> { row[row.Count - 1] };
                    foreach (var v in row) next.Add(next[next.Count - 1] + v);
                    row = next;
                }
                return Num(row[0]);
            }, "", true);

            Def(c, "isprime", PR, "Is Prime", "n:int", (cc, a) => Bool(Primes.IsPrime(Int(a[0], "isprime"))), "", true);
            Def(c, "nextprime", PR, "Next Prime", "n:int", (cc, a) => Num(Primes.NextPrime(Int(a[0], "nextprime"))), "Smallest prime greater than n.", true);
            Def(c, "prevprime", PR, "Previous Prime", "n:int", (cc, a) => Num(Primes.PrevPrime(Int(a[0], "prevprime"))), "Largest prime less than n.", true);
            Def(c, "nthprime,prime", PR, "Nth Prime", "n:pint", (cc, a) => Num(Primes.NthPrime((long)Int(a[0], "nthprime"))), "", true);
            Def(c, "primepi,primecount", PR, "Prime Counting Function", "x:real", (cc, a) => IsR(a[0], out var r) ? Num(Real.Int(Primes.PrimeCount(r.Floor().ToBigInt()))) : null, "Number of primes ≤ x.", true);
            Def(c, "primes", PR, "Primes", "n:int", (cc, a) => MatE.Vector(Primes.PrimesUpTo((long)Int(a[0], "primes")).Select(p => Num(p)).ToList()), "Vector of all primes ≤ n.");
            Def(c, "factors,primefactors", PR, "Prime Factors", "n:int", (cc, a) =>
            {
                var f = Primes.Factorize(Int(a[0], "factors"));
                var l = new List<Expr>();
                foreach (var kv in f) for (int i = 0; i < kv.Value; i++) l.Add(Num(kv.Key));
                return MatE.Vector(l);
            }, "Vector of prime factors.");
            Def(c, "divisors", INT, "Divisors", "n:int", (cc, a) => MatE.Vector(Primes.Divisors(Int(a[0], "divisors")).Select(d => Num(d)).ToList()), "Vector of positive divisors.");
            Def(c, "totient,eulerphi", INT, "Euler's Totient", "n:pint", (cc, a) => Num(Primes.Totient(Int(a[0], "totient"))), "", true);
            Def(c, "sigma,divisorsum", INT, "Divisor Function", "n:pint,k:int=1", (cc, a) =>
            {
                int k = SmallInt(a[1], "sigma");
                BigInteger s = 0;
                foreach (var d in Primes.Divisors(Int(a[0], "sigma"))) s += BigInteger.Pow(d, k);
                return Num(s);
            });
            Def(c, "even,iseven", INT, "Is Even", "n", (cc, a) => IsI(a[0], out var n) ? Bool(n.IsEven) : (a[0] is NumE ? Expr.Zero : null), "", true);
            Def(c, "odd,isodd", INT, "Is Odd", "n", (cc, a) => IsI(a[0], out var n) ? Bool(!n.IsEven) : (a[0] is NumE ? Expr.Zero : null), "", true);
            Def(c, "fibonacci,fib", INT, "Fibonacci Number", "n:int", (cc, a) => Num(Primes.Fibonacci(Int(a[0], "fibonacci"))), "", true);
            Def(c, "lucas", INT, "Lucas Number", "n:nnint", (cc, a) => { var n = Int(a[0], "lucas"); return Num(Primes.Fibonacci(n - 1) + Primes.Fibonacci(n + 1)); }, "", true);
            Def(c, "bernoulli", INT, "Bernoulli Number", "n:nnint", (cc, a) => Num(Primes.Bernoulli(SmallInt(a[0], "bernoulli"))), "", true);
            Def(c, "harmonic", INT, "Harmonic Number", "n:nnint", (cc, a) =>
            {
                int n = SmallInt(a[0], "harmonic");
                if (n > 100000) throw new CalcException("harmonic() argument too large");
                Real s = Real.Zero;
                for (int i = 1; i <= n; i++) s += Real.Make(1, i);
                return Num(s);
            }, "", true);
            Def(c, "digitsum,sumdigits", INT, "Digit Sum", "n:int,base:int=10", (cc, a) =>
            {
                var n = BigInteger.Abs(Int(a[0], "digitsum")); int b = SmallInt(a[1], "digitsum");
                BigInteger s = 0;
                while (!n.IsZero) { s += n % b; n /= b; }
                return Num(s);
            });
            Def(c, "powmod", INT, "Modular Exponentiation", "a:int,b:int,m:int", (cc, a) =>
            {
                var b = Int(a[1], "powmod"); var m = Int(a[2], "powmod");
                if (b.Sign < 0) return Num(BigInteger.ModPow(Primes.ModInverse(Int(a[0], "powmod"), m), -b, m));
                var r = BigInteger.ModPow(Int(a[0], "powmod"), b, m);
                if (r.Sign < 0) r += m;
                return Num(r);
            });
            Def(c, "modinv", INT, "Modular Inverse", "a:int,m:int", (cc, a) => Num(Primes.ModInverse(Int(a[0], "modinv"), Int(a[1], "modinv"))));
            Def(c, "isinteger", INT, "Is Integer", "x", (cc, a) => a[0] is NumE n ? Bool(n.V.IsInt) : (cc.IsKnownInteger(a[0]) ? Expr.One : null));
            Def(c, "isrational", INT, "Is Rational", "x", (cc, a) => a[0] is NumE n ? Bool(n.V.IsRational) : null);
            Def(c, "isreal", INT, "Is Real", "x", (cc, a) => a[0] is NumE n ? Bool(n.V.IsReal) : (cc.IsKnownReal(a[0]) ? Expr.One : null));
            Def(c, "isnumber", INT, "Is Number", "x", (cc, a) => Bool(a[0] is NumE));

            Def(c, "re,real", CX, "Real Part", "z", (cc, a) =>
            {
                if (a[0] is NumE n) return Num(n.V.Re);
                if (cc.IsKnownReal(a[0])) return a[0];
                return null;
            }, "", true);
            Def(c, "im,imag", CX, "Imaginary Part", "z", (cc, a) =>
            {
                if (a[0] is NumE n) return Num(n.V.Im);
                if (cc.IsKnownReal(a[0])) return Expr.Zero;
                return null;
            }, "", true);
            Def(c, "arg,angle", CX, "Argument", "z", (cc, a) =>
            {
                if (!(a[0] is NumE n)) return null;
                if (n.V.IsReal) return n.V.Re.Sign >= 0 ? Expr.Zero : cc.Ev.Eval(FromRadians(Pi));
                if (n.V.Re.IsZero) return cc.Ev.Eval(FromRadians(Simp.Mul(Expr.N(Real.Frac(n.V.Im.Sign, 2)), Pi)));
                if (cc.ExactPhase && !n.V.Approx) return cc.Ev.Call("atan2", Num(n.V.Im), Num(n.V.Re)) is CallE ? null : cc.Ev.Call("atan2", Num(n.V.Im), Num(n.V.Re));
                return cc.Ev.Eval(FromRadians(Num(n.V.AsApprox().Arg())));
            }, "Angle of a complex number.", true);
            Def(c, "conj,conjugate", CX, "Complex Conjugate", "z", (cc, a) => a[0] is NumE n ? Num(n.V.Conj()) : (cc.IsKnownReal(a[0]) ? a[0] : null), "", true);
            Def(c, "norm2,abs2", CX, "Squared Modulus", "z", (cc, a) => a[0] is NumE n ? Num(n.V.Re * n.V.Re + n.V.Im * n.V.Im) : null, "", true);

            Def(c, "gamma,Γ", SP, "Gamma Function", "x:num", (cc, a) =>
            {
                if (IsI(a[0], out var n) && !((NumE)a[0]).V.Approx)
                {
                    if (n.Sign <= 0) throw new CalcException("gamma() is undefined for non-positive integers");
                    return Num(Primes.Factorial(n - 1));
                }
                if (IsN(a[0], out var v))
                {
                    if (cc.ExactPhase && !v.Approx && v.IsRational && v.Re.D == 2) return HalfFactorial(cc, v.Re - Real.One);
                    if (cc.ExactPhase && !v.Approx) return null;
                    if (v.IsReal) return Num(LogGammaReal(v.Re));
                    return NumC(Special.Gamma(v.ToComplex()));
                }
                return null;
            }, "Γ(x) = (x−1)!", true);
            Def(c, "lgamma,loggamma", SP, "Log Gamma", "x:real", RD(Special.LogGamma), "", true);
            Def(c, "digamma,psi", SP, "Digamma Function", "x:real", RD(Special.Digamma), "", true);
            Def(c, "beta", SP, "Beta Function", "a:num,b:num", (cc, a) =>
            {
                if (IsI(a[0], out var x) && IsI(a[1], out var y) && x > 0 && y > 0 && x + y < 10000)
                    return Num(Real.Make(Primes.Factorial(x - 1) * Primes.Factorial(y - 1), Primes.Factorial(x + y - 1)));
                return R2(Special.Beta)(cc, a);
            });
            Def(c, "betainc", SP, "Regularized Incomplete Beta", "x:real,a:real,b:real", (cc, a) => { if (!IsR(a[0], out _) || cc.ExactPhase && !a.Any(z => z.IsApprox)) return null; return Num(Special.BetaInc(D(a[0], "betainc"), D(a[1], "betainc"), D(a[2], "betainc"))); });
            Def(c, "erf", SP, "Error Function", "x:real", RD(Special.Erf, v => v.IsZero ? Expr.Zero : null), "", true);
            Def(c, "erfc", SP, "Complementary Error Function", "x:real", RD(Special.Erfc, v => v.IsZero ? Expr.One : null), "", true);
            Def(c, "erfinv", SP, "Inverse Error Function", "x:real", RD(Special.ErfInv, v => v.IsZero ? Expr.Zero : null), "", true);
            Def(c, "zeta,ζ", SP, "Riemann Zeta", "s:num", (cc, a) =>
            {
                if (IsI(a[0], out var n) && !((NumE)a[0]).V.Approx)
                {
                    if (n.IsZero) return Expr.N(Real.Frac(-1, 2));
                    if (n.Sign < 0 && n.IsEven) return Expr.Zero;
                    if (n.Sign < 0) return Num(-Primes.Bernoulli((int)(1 - n)) / Real.Int(1 - n));
                    if (n > 0 && n.IsEven && n < 200)
                    {
                        int k = (int)n;
                        var b = Primes.Bernoulli(k).Abs();
                        var coef = Real.Two.PowInt(k - 1) * b / Real.Int(Primes.Factorial(k));
                        return Simp.Mul(Num(coef), Simp.Pow(Pi, Num(Real.Int(k))));
                    }
                    if (n.IsOne) throw new CalcException("zeta(1) is undefined");
                }
                if (!IsN(a[0], out var v) || cc.ExactPhase && !v.Approx) return null;
                return NumC(Special.Zeta(v.ToComplex()));
            }, "", true);
            Def(c, "polylog,Li", SP, "Polylogarithm", "s:num,z:num", (cc, a) =>
            {
                if (a[1].IsZero) return Expr.Zero;
                if (!IsN(a[0], out var s) || !IsN(a[1], out var z)) return null;
                if (cc.ExactPhase && !s.Approx && !z.Approx) return null;
                return NumC(Special.Polylog(s.ToComplex(), z.ToComplex()));
            });
            Def(c, "Ei,expint", SP, "Exponential Integral", "x:real", RD(Special.Ei), "", true);
            Def(c, "E1", SP, "Exponential Integral E1", "x:real", RD(Special.E1), "", true);
            Def(c, "li,logint", SP, "Logarithmic Integral", "x:real", RD(Special.Li, v => v.IsZero ? Expr.Zero : null), "", true);
            Def(c, "Si,sinint", SP, "Sine Integral", "x:real", RD(x => { Special.SiCi(x, out var s, out _); return s; }, v => v.IsZero ? Expr.Zero : null), "", true);
            Def(c, "Ci,cosint", SP, "Cosine Integral", "x:real", RD(x => { Special.SiCi(x, out _, out var ci); return ci; }), "", true);
            Def(c, "Shi,sinhint", SP, "Hyperbolic Sine Integral", "x:real", RD(Special.Shi, v => v.IsZero ? Expr.Zero : null), "", true);
            Def(c, "Chi,coshint", SP, "Hyperbolic Cosine Integral", "x:real", RD(Special.Chi), "", true);
            Def(c, "besselj", SP, "Bessel Function of the First Kind", "n:int,x:real", (cc, a) => { if (!IsR(a[1], out var x) || cc.ExactPhase && !x.Approx && !x.IsZero) return null; return Num(Special.BesselJ(SmallInt(a[0], "besselj"), x.ToDouble())); });
            Def(c, "bessely", SP, "Bessel Function of the Second Kind", "n:int,x:real", (cc, a) => { if (!IsR(a[1], out var x) || cc.ExactPhase && !x.Approx) return null; return Num(Special.BesselY(SmallInt(a[0], "bessely"), x.ToDouble())); });
            Def(c, "airy,airyai", SP, "Airy Function", "x:real", RD(Special.AiryAi), "", true);
            Def(c, "fresnels", SP, "Fresnel Integral S", "x:real", RD(x => { Special.Fresnel(x, out var s, out _); return s; }, v => v.IsZero ? Expr.Zero : null), "", true);
            Def(c, "fresnelc", SP, "Fresnel Integral C", "x:real", RD(x => { Special.Fresnel(x, out _, out var cc2); return cc2; }, v => v.IsZero ? Expr.Zero : null), "", true);
            Def(c, "igamma,uppergamma", SP, "Upper Incomplete Gamma", "a:real,x:real", R2(Special.UpperIncGamma));
            Def(c, "lowergamma", SP, "Lower Incomplete Gamma", "a:real,x:real", R2(Special.LowerIncGamma));
            Def(c, "heaviside,theta", "Step Functions", "Heaviside Step Function", "x:real", (cc, a) => IsR(a[0], out var r) ? (r.Sign > 0 ? Expr.One : r.Sign < 0 ? Expr.Zero : Expr.Half) : null, "", true);
            Def(c, "dirac,delta", "Step Functions", "Dirac Delta", "x:real", (cc, a) => IsR(a[0], out var r) ? (r.IsZero ? Expr.N(Real.PosInf) : Expr.Zero) : null, "", true);
            Def(c, "ramp", "Step Functions", "Ramp Function", "x:real", (cc, a) => IsR(a[0], out var r) ? Num(Real.Max(r, Real.Zero)) : null, "", true);
            Def(c, "rect", "Step Functions", "Rectangular Function", "x:real", (cc, a) => IsR(a[0], out var r) ? (r.Abs() < Real.Half ? Expr.One : r.Abs().EqualsValue(Real.Half) ? Expr.Half : Expr.Zero) : null, "", true);
            Def(c, "tri,triangle", "Step Functions", "Triangular Function", "x:real", (cc, a) => IsR(a[0], out var r) ? Num(Real.Max(Real.Zero, Real.One - r.Abs())) : null, "", true);
        }

        static Expr RoundFn(Calculator c, Expr x, Func<Real, Real> f)
        {
            if (x is NumE n)
            {
                if (n.V.IsReal) { var r = f(n.V.Re); return Num(r.Approx && r.IsInt ? Real.Int(r.N) : r); }
                return Num(new Cx(f(n.V.Re), f(n.V.Im)));
            }
            if (x is IntvE iv) return Intervals.FromMidRad(f(iv.Mid), Real.Zero);
            if (x.HasUnits && Units.TryQuantity(x, out _, out _, out var rest) && rest.IsOne)
            {
                Simp.SplitCoef(x, out var cf, out var un);
                if (cf.IsReal) return Simp.Mul(Num(f(cf.Re)), un);
            }
            if (!c.ExactPhase || x.HasSymbols) return null;
            var save = c.ExactPhase; c.ExactPhase = false;
            try
            {
                var ap = c.Ev.Eval(x);
                if (ap is NumE an && an.V.IsReal)
                {
                    var r = f(an.V.Re);
                    var nearest = r.Round();
                    return Num(Real.Int(nearest.N) == null ? r : (r - nearest).Abs() < Real.Frac(1, 1000000000) ? Real.Int(nearest.ToBigInt()) : r);
                }
            }
            finally { c.ExactPhase = save; }
            return null;
        }

        static Expr Quantities2(Calculator c, List<Expr> a, string fn)
        {
            if (Units.TryQuantity(a[0], out var v1, out var d1, out var r1) && Units.TryQuantity(a[1], out var v2, out var d2, out var r2) && r1.IsOne && r2.IsOne && d1.SameAs(d2) && v1.IsReal && v2.IsReal && !d1.IsNone)
            {
                var res = c.Ev.Call(fn, Num(v1.Re), Num(v2.Re));
                return Units.Convert(Simp.Mul(res, Units.BaseUnits(d1)), Simp.SplitUnitsOf(a[0]));
            }
            return null;
        }

        static Expr NumDenPart(Expr e, bool num)
        {
            if (!e.HasSymbols) return null;
            Simp.NumDen(e, out var n, out var d);
            return num ? n : d;
        }

        static Real RatGcd(Real a, Real b)
        {
            if (a.IsZero) return b.Abs();
            if (b.IsZero) return a.Abs();
            var n = BigInteger.GreatestCommonDivisor(a.N * b.D, b.N * a.D);
            return Real.Make(n, a.D * b.D);
        }

        static Real LogGammaReal(Real x)
        {
            double d = x.ToDouble();
            if (d > 170) return RMath.Exp(Real.FromDouble(Special.LogGamma(d)));
            return Real.FromDouble(Special.Gamma(d));
        }

        // gamma at half integers: (n - 1/2)! has a closed form with sqrt(pi)
        static Expr HalfFactorial(Calculator c, Real x)
        {
            var n = (x + Real.Half);
            if (!n.IsInt || n.Abs() > Real.Int(5000)) return null;
            int k = n.ToInt();
            Real coef = Real.One;
            if (k >= 0) for (int i = 1; i <= k; i++) coef *= Real.Make(2 * i - 1, 2);
            else for (int i = 0; i < -k; i++) coef /= Real.Make(-2 * i - 1, 2);
            return Simp.Mul(Num(coef), Simp.Pow(Pi, Expr.Half));
        }

        static Real ExactLog(Real x, Real b)
        {
            if (b.IsOne) throw new CalcException("Logarithm base can not be one");
            if (x.IsOne) return Real.Zero;
            bool inv = false;
            var bb = b;
            if (bb < Real.One) { bb = Real.One / bb; inv = true; }
            var xx = x;
            bool neg = false;
            if (xx < Real.One) { xx = Real.One / xx; neg = true; }
            var p = bb;
            for (int k = 1; k <= 4000; k++)
            {
                if (p.EqualsValue(xx)) { var r = Real.Int(k); if (neg) r = -r; if (inv) r = -r; return r; }
                if (p > xx) break;
                p *= bb;
            }
            for (int q = 2; q <= 6; q++)
            {
                var root = bb.ExactRoot(q);
                if (root == null) continue;
                var rr = ExactLog(xx, root);
                if (rr != null) { var r = rr / Real.Int(q); if (neg) r = -r; if (inv) r = -r; return r; }
            }
            return null;
        }

        public static List<Expr> Flatten(List<Expr> args)
        {
            var l = new List<Expr>();
            foreach (var a in args)
            {
                if (a is MatE m) l.AddRange(m.All);
                else l.Add(a);
            }
            return l;
        }

        public static string ToRoman(BigInteger n)
        {
            if (n <= 0 || n >= 4000000) return null;
            var sb = new StringBuilder();
            int v = (int)n;
            if (v >= 4000)
            {
                var th = ToRoman(v / 1000);
                foreach (char ch in th) { sb.Append(ch); sb.Append('̅'); }
                v %= 1000;
            }
            int[] vals = { 1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1 };
            string[] syms = { "M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I" };
            for (int i = 0; i < vals.Length; i++) while (v >= vals[i]) { sb.Append(syms[i]); v -= vals[i]; }
            return sb.ToString();
        }

        public static BigInteger FromRoman(string s)
        {
            var map = new Dictionary<char, int> { { 'I', 1 }, { 'V', 5 }, { 'X', 10 }, { 'L', 50 }, { 'C', 100 }, { 'D', 500 }, { 'M', 1000 } };
            s = s.ToUpperInvariant().Trim();
            BigInteger total = 0;
            for (int i = 0; i < s.Length; i++)
            {
                if (!map.TryGetValue(s[i], out int v)) throw new CalcException("Invalid roman numeral");
                if (i + 1 < s.Length && map.TryGetValue(s[i + 1], out int nx) && nx > v) total -= v; else total += v;
            }
            return total;
        }

        public static DateTime ParseDate(string s, out bool hasTime, out bool utc)
        {
            s = s.Trim();
            hasTime = s.Contains(":");
            utc = s.EndsWith("Z") || Regex.IsMatch(s, @"[+-]\d{2}:?\d{2}$") && s.Length > 10;
            string[] fmts = { "yyyy-MM-dd", "yyyy-MM-dd'T'HH:mm", "yyyy-MM-dd'T'HH:mm:ss", "yyyy-MM-dd HH:mm", "yyyy-MM-dd HH:mm:ss", "yyyy-MM-dd'T'H:mm", "yyyy-MM-dd H:mm", "yyyy-MM-dd'T'HH:mm:ss.FFFFFFF", "yyyy-MM-dd HH:mm:ss.FFFFFFF" };
            if (utc)
            {
                if (DateTimeOffset.TryParse(s, CultureInfo.InvariantCulture, DateTimeStyles.AssumeUniversal, out var dto)) return dto.UtcDateTime;
            }
            if (DateTime.TryParseExact(s, fmts, CultureInfo.InvariantCulture, DateTimeStyles.None, out var d)) return d;
            if (DateTime.TryParse(s, CultureInfo.InvariantCulture, DateTimeStyles.None, out d)) { hasTime = d.TimeOfDay.Ticks != 0; return d; }
            if (DateTime.TryParse(s, CultureInfo.CurrentCulture, DateTimeStyles.None, out d)) { hasTime = d.TimeOfDay.Ticks != 0; return d; }
            throw new CalcException("Invalid date \"" + s + "\"");
        }

        // best rational approximation with bounded denominator (continued fractions)
        public static Real ToFraction(Real x, long maxDen)
        {
            if (x.IsInt) return Real.Int(x.N);
            bool neg = x.Sign < 0;
            var v = x.Abs();
            BigInteger h0 = 0, h1 = 1, k0 = 1, k1 = 0;
            var r = v;
            for (int i = 0; i < 64; i++)
            {
                var a = r.Floor().ToBigInt();
                var h2 = a * h1 + h0; var k2 = a * k1 + k0;
                if (k2 > maxDen) break;
                h0 = h1; h1 = h2; k0 = k1; k1 = k2;
                var fr = r - Real.Int(a);
                if (fr.IsZero || Math.Abs((Real.Make(h1, k1) - v).ToDouble()) < 1e-15 * Math.Max(1, Math.Abs(v.ToDouble()))) break;
                r = Real.One / fr;
            }
            if (k1.IsZero) return x;
            var res = Real.Make(h1, k1);
            return neg ? -res : res;
        }
    }
}
