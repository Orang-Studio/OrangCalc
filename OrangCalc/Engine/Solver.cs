using System;
using System.Collections.Generic;
using System.Linq;

namespace OrangCalc.Engine
{
    public static class Solver
    {
        static Calculator C => Calculator.Current;
        static Expr F(string n, params Expr[] a) => C.Ev.Call(n, a);
        static bool Free(Expr e, string x) => !e.Any(z => z is SymE s && s.Name == x);

        public static Expr SolveForDisplay(Calculator c, CmpE eq, string x)
        {
            var sols = Solve(c, eq, x);
            if (sols.Count == 0) throw new CalcException("No solution found");
            var X = new SymE(x);
            if (sols.Count == 1) return new CmpE(CmpOp.Eq, X, sols[0]);
            return new LogicE(LogicOp.Or, sols.Select(s => (Expr)new CmpE(CmpOp.Eq, X, s)).ToList());
        }

        public static List<Expr> Solve(Calculator c, Expr eq, string x)
        {
            Expr f;
            if (eq is CmpE ce)
            {
                if (ce.Op != CmpOp.Eq) throw new CalcException("Only equations can be solved");
                f = Simp.Sub(ce.L, ce.R);
            }
            else f = eq;
            var sols = SolveZero(f, x);
            var res = new List<Expr>();
            var seen = new HashSet<string>();
            foreach (var s in sols)
            {
                var sv = c.Ev.Eval(s);
                if (seen.Add(sv.Key)) res.Add(sv);
            }
            return res.Select((r, i) => (r, i)).OrderBy(t => t.r is NumE n && n.V.IsReal ? 0 : 1).ThenBy(t => NumericValue(t.r)).ThenBy(t => t.i).Select(t => t.r).ToList();
        }

        static List<Expr> SolveZero(Expr f, string x)
        {
            C.CheckAbort();
            var X = new SymE(x);
            if (Free(f, x))
            {
                if (f.IsZero) throw new CalcException("The equation is true for all " + x);
                throw new CalcException("The equation has no solution");
            }
            Simp.NumDen(f, out var num, out var den);
            if (!den.IsOne && !Free(den, x)) f = num;

            var ex = Simp.Expand(f);
            if (Polynomials.TryCoefficients(ex, x, out var cs) && cs.Count >= 2)
            {
                var r = SolvePolynomial(cs, x);
                if (r != null) return r;
            }

            var iso = Isolate(f, Expr.Zero, x);
            if (iso != null) return iso;
            return NumericRoots(f, x);
        }

        static List<Expr> SolvePolynomial(List<Expr> cs, string x)
        {
            int deg = cs.Count - 1;
            while (deg > 0 && cs[deg].IsZero) deg--;
            if (deg == 0) return null;
            var res = new List<Expr>();
            int zeroRoots = 0;
            while (cs[zeroRoots].IsZero) zeroRoots++;
            if (zeroRoots > 0) { res.Add(Expr.Zero); cs = cs.Skip(zeroRoots).ToList(); deg -= zeroRoots; }
            if (deg == 0) return res;
            if (deg == 1) { res.Add(Simp.Neg(Simp.Div(cs[0], cs[1]))); return res; }
            if (deg == 2)
            {
                var a = cs[2]; var b = cs[1]; var c = cs[0];
                var disc = Simp.Sub(Simp.Pow(b, Expr.Two), Simp.Mul(Expr.N(4), a, c));
                var sq = Simp.Pow(disc, Expr.Half);
                var twoA = Simp.Mul(Expr.Two, a);
                if (disc.IsZero) { res.Add(Simp.Div(Simp.Neg(b), twoA)); return res; }
                res.Add(Simp.Div(Simp.Add(Simp.Neg(b), sq), twoA));
                res.Add(Simp.Div(Simp.Sub(Simp.Neg(b), sq), twoA));
                return res;
            }
            if (cs.All(e => e is NumE n && n.V.IsRational))
            {
                var p = new Poly(cs.Select(e => ((NumE)e).V.Re).ToList());
                var rr = Polynomials.RationalRoots(p);
                var rest = p;
                foreach (var r in rr)
                {
                    res.Add(Expr.N(r));
                    rest = Poly.DivMod(rest, new Poly(new List<Real> { -r, Real.One }), out _);
                }
                if (rest.Degree >= 1)
                {
                    if (rest.Degree <= 2)
                    {
                        var sub = SolvePolynomial(rest.C.Select(z => Expr.N(z)).ToList(), x);
                        if (sub != null) res.AddRange(sub);
                    }
                    else if (rest.Degree % 2 == 0 && rest.C.Where((z, i) => i % 2 == 1).All(z => z.IsZero))
                    {
                        var half = rest.C.Where((z, i) => i % 2 == 0).Select(z => Expr.N(z)).ToList();
                        var inner = SolvePolynomial(half, x);
                        if (inner != null)
                            foreach (var s in inner)
                            {
                                var root = Simp.Pow(s, Expr.Half);
                                res.Add(root); res.Add(Simp.Neg(root));
                            }
                    }
                    else
                    {
                        var roots = Polynomials.NumericRoots(rest.C.Select(z => new System.Numerics.Complex(z.ToDouble(), 0)).ToList());
                        foreach (var z in roots)
                        {
                            var im = Math.Abs(z.Imaginary) < 1e-12 * Math.Max(1, Math.Abs(z.Real)) ? 0 : z.Imaginary;
                            res.Add(Expr.N(new Cx(Real.FromDouble(z.Real), im == 0 ? Real.Zero : Real.FromDouble(im))));
                        }
                    }
                }
                return res.OrderBy(e => e is NumE n && n.V.IsReal ? 0 : 1).ToList();
            }
            if (cs.All(e => e is NumE))
            {
                var roots = Polynomials.NumericRoots(cs.Select(e => ((NumE)e).V.ToComplex()).ToList());
                res.AddRange(roots.Select(z => Expr.N(Cx.FromComplex(z))));
                return res;
            }
            return null;
        }

        // inverts the outermost operation repeatedly while x occurs only on one branch
        static List<Expr> Isolate(Expr lhs, Expr rhs, string x)
        {
            for (int guard = 0; guard < 50; guard++)
            {
                if (lhs is SymE s && s.Name == x) return new List<Expr> { rhs };
                switch (lhs)
                {
                    case AddE a:
                        {
                            var withX = a.Terms.Where(t => !Free(t, x)).ToList();
                            if (withX.Count != 1) return null;
                            var others = a.Terms.Where(t => Free(t, x)).ToList();
                            rhs = Simp.Sub(rhs, Simp.Add(others));
                            lhs = withX[0];
                            continue;
                        }
                    case MulE m:
                        {
                            var withX = m.Factors.Where(t => !Free(t, x)).ToList();
                            if (withX.Count != 1) return null;
                            var others = m.Factors.Where(t => Free(t, x)).ToList();
                            rhs = Simp.Div(rhs, Simp.Mul(others));
                            lhs = withX[0];
                            continue;
                        }
                    case PowE p:
                        {
                            if (Free(p.X, x))
                            {
                                if (p.X is NumE n && n.V.IsExactInt && !n.V.Re.N.IsEven || !(p.X is NumE))
                                {
                                    rhs = Simp.Pow(rhs, Simp.Pow(p.X, Expr.MinusOne));
                                    lhs = p.B;
                                    continue;
                                }
                                if (p.X is NumE ne && ne.V.IsExactInt && ne.V.Re.N.IsEven)
                                {
                                    var root = Simp.Pow(rhs, Simp.Pow(p.X, Expr.MinusOne));
                                    var a1 = Isolate(p.B, root, x) ?? new List<Expr>();
                                    var a2 = Isolate(p.B, Simp.Neg(root), x) ?? new List<Expr>();
                                    return a1.Concat(a2).ToList();
                                }
                                if (p.X is NumE nr && nr.V.IsRational)
                                {
                                    rhs = Simp.Pow(rhs, Simp.Pow(p.X, Expr.MinusOne));
                                    lhs = p.B;
                                    continue;
                                }
                                return null;
                            }
                            if (Free(p.B, x))
                            {
                                rhs = Simp.Div(F("ln", rhs), F("ln", p.B));
                                lhs = p.X;
                                continue;
                            }
                            return null;
                        }
                    case CallE c when c.Args.Count >= 1 && Free(new MatE(new List<List<Expr>> { c.Args.Skip(1).ToList() }, true), x):
                        {
                            var inv = Inverse(c, rhs);
                            if (inv == null) return null;
                            rhs = inv;
                            lhs = c.Args[0];
                            continue;
                        }
                    default: return null;
                }
            }
            return null;
        }

        static Expr Inverse(CallE c, Expr y)
        {
            switch (c.F.Name)
            {
                case "sin": return F("asin", y);
                case "cos": return F("acos", y);
                case "tan": return F("atan", y);
                case "asin": return F("sin", y);
                case "acos": return F("cos", y);
                case "atan": return F("tan", y);
                case "sinh": return F("asinh", y);
                case "cosh": return F("acosh", y);
                case "tanh": return F("atanh", y);
                case "asinh": return F("sinh", y);
                case "acosh": return F("cosh", y);
                case "atanh": return F("tanh", y);
                case "exp": return F("ln", y);
                case "ln": return F("exp", y);
                case "log": return c.Args.Count > 1 ? Simp.Pow(c.Args[1], y) : F("exp", y);
                case "log10": return Simp.Pow(Expr.N(10), y);
                case "log2": return Simp.Pow(Expr.Two, y);
                case "sqrt": return Simp.Pow(y, Expr.Two);
                case "cbrt": return Simp.Pow(y, Expr.N(3));
                case "root": return Simp.Pow(y, c.Args[1]);
                case "lambertw": return Simp.Mul(y, F("exp", y));
                default: return null;
            }
        }

        // scans for sign changes and refines with bisection/secant
        static List<Expr> NumericRoots(Expr f, string x)
        {
            var save = C.ExactPhase;
            C.ExactPhase = false;
            try
            {
                var roots = new List<double>();
                Func<double, double> g = v => C.Ev.EvalDouble(f, x, v);
                var pts = new List<double>();
                for (int i = -400; i <= 400; i++) pts.Add(Math.Sign(i) * Math.Pow(Math.Abs(i) / 40.0, 3));
                double prevX = pts[0], prevY = g(prevX);
                foreach (var px in pts.Skip(1))
                {
                    C.CheckAbort();
                    double py = g(px);
                    if (!double.IsNaN(py) && !double.IsNaN(prevY))
                    {
                        if (py == 0) AddRoot(roots, px);
                        else if (Math.Sign(py) != Math.Sign(prevY) && prevY != 0)
                        {
                            double r = Bisect(g, prevX, px);
                            if (!double.IsNaN(r) && Math.Abs(g(r)) < 1e-6 * Math.Max(1, Math.Abs(r))) AddRoot(roots, r);
                        }
                    }
                    prevX = px; prevY = py;
                }
                if (roots.Count == 0)
                {
                    foreach (double start in new[] { 0.5, 1, -1, 2, 10, -10, 0.1 })
                    {
                        double r = Newton(g, start);
                        if (!double.IsNaN(r)) { AddRoot(roots, r); break; }
                    }
                }
                if (roots.Count == 0) throw new CalcException("No solution found");
                return roots.Select(r => Expr.N(Real.FromDouble(Clean(r)))).ToList();
            }
            finally { C.ExactPhase = save; }
        }

        static double Clean(double r)
        {
            double rr = Math.Round(r, 12);
            return Math.Abs(rr - r) < 1e-13 * Math.Max(1, Math.Abs(r)) ? rr : r;
        }

        static void AddRoot(List<double> roots, double r)
        {
            if (!roots.Any(q => Math.Abs(q - r) < 1e-9 * Math.Max(1, Math.Abs(r)))) roots.Add(r);
        }

        static double Bisect(Func<double, double> g, double a, double b)
        {
            double fa = g(a);
            for (int i = 0; i < 200; i++)
            {
                double m = (a + b) / 2, fm = g(m);
                if (double.IsNaN(fm)) return double.NaN;
                if (fm == 0 || (b - a) / 2 < 1e-16 * Math.Max(1, Math.Abs(m))) return m;
                if (Math.Sign(fm) == Math.Sign(fa)) { a = m; fa = fm; } else b = m;
            }
            return (a + b) / 2;
        }

        public static double Newton(Func<double, double> g, double x0)
        {
            double x = x0;
            for (int i = 0; i < 100; i++)
            {
                double fx = g(x);
                if (double.IsNaN(fx)) return double.NaN;
                if (Math.Abs(fx) < 1e-15) return x;
                double h = 1e-7 * Math.Max(1, Math.Abs(x));
                double d = (g(x + h) - g(x - h)) / (2 * h);
                if (d == 0 || double.IsNaN(d)) return double.NaN;
                double nx = x - fx / d;
                if (Math.Abs(nx - x) < 1e-15 * Math.Max(1, Math.Abs(x))) return nx;
                x = nx;
            }
            return Math.Abs(g(x)) < 1e-9 ? x : double.NaN;
        }

        public static double Secant(Func<double, double> g, double x0, double x1)
        {
            double f0 = g(x0), f1 = g(x1);
            for (int i = 0; i < 200; i++)
            {
                if (f1 == f0) break;
                double x2 = x1 - f1 * (x1 - x0) / (f1 - f0);
                x0 = x1; f0 = f1; x1 = x2; f1 = g(x1);
                if (Math.Abs(f1) < 1e-15 || Math.Abs(x1 - x0) < 1e-15 * Math.Max(1, Math.Abs(x1))) return x1;
            }
            return Math.Abs(f1) < 1e-9 ? x1 : double.NaN;
        }

        // solves a list of linear equations for the given unknowns
        public static List<Expr> MultiSolve(Calculator c, List<Expr> eqs, List<string> vars)
        {
            int n = vars.Count;
            if (eqs.Count < n) throw new CalcException("Not enough equations");
            var rows = new List<List<Expr>>();
            var rhs = new List<Expr>();
            foreach (var e in eqs.Take(Math.Max(n, eqs.Count)))
            {
                Expr f = e is CmpE ce ? Simp.Sub(ce.L, ce.R) : e;
                f = Simp.Expand(f);
                var row = new List<Expr>();
                Expr rest = f;
                foreach (var v in vars)
                {
                    if (!Polynomials.TryCoefficients(f, v, out var cs) || cs.Count > 2) return NonLinear(c, eqs, vars);
                    var coef = cs.Count == 2 ? cs[1] : Expr.Zero;
                    row.Add(coef);
                }
                var constant = f;
                foreach (var v in vars) constant = c.Ev.EvalWith(constant, v, Expr.Zero);
                rows.Add(row);
                rhs.Add(Simp.Neg(constant));
            }
            var A = new MatE(rows, false);
            if (rows.Count > n)
            {
                A = new MatE(rows.Take(n).ToList(), false);
                rhs = rhs.Take(n).ToList();
            }
            return Matrices.Solve(A, rhs);
        }

        // nonlinear systems by substitution of one solved variable at a time
        static List<Expr> NonLinear(Calculator c, List<Expr> eqs, List<string> vars)
        {
            if (vars.Count == 1) return Solve(c, eqs[0], vars[0]).Take(1).ToList();
            for (int i = 0; i < eqs.Count; i++)
            {
                List<Expr> sol;
                try { sol = Solve(c, eqs[i], vars[0]); } catch (CalcException) { continue; }
                foreach (var s in sol)
                {
                    try
                    {
                        var rest = eqs.Where((e, k) => k != i).Select(e => c.Ev.Eval(c.Substitute(e, new SymE(vars[0]), s))).ToList();
                        var sub = MultiSolve(c, rest, vars.Skip(1).ToList());
                        var first = s;
                        for (int k = 1; k < vars.Count; k++) first = c.Ev.Eval(c.Substitute(first, new SymE(vars[k]), sub[k - 1]));
                        return new List<Expr> { first }.Concat(sub).ToList();
                    }
                    catch (CalcException) { }
                }
            }
            throw new CalcException("Unable to solve the system");
        }

        // first order linear ODE with constant coefficients: y' = a y + b
        public static Expr DSolve(Calculator c, Expr eq, string y, string x, Expr x0, Expr y0)
        {
            var dy = new SymE("_dy");
            var marked = c.Ev.Eval(eq.Map(e => e is CallE ce && ce.F.Name == "diff" ? dy : e));
            var Y = new SymE(y);
            Expr f = marked is CmpE cm ? Simp.Sub(cm.L, cm.R) : marked;
            f = Simp.Expand(f);
            if (!Polynomials.TryCoefficients(f, "_dy", out var cd) || cd.Count != 2) throw new CalcException("Only first order linear differential equations are supported");
            var k = cd[1];
            var rest = Simp.Neg(Simp.Div(cd[0], k));
            rest = Simp.Expand(rest);
            if (!Polynomials.TryCoefficients(rest, y, out var cy) || cy.Count > 2) throw new CalcException("Only linear differential equations are supported");
            var a = cy.Count == 2 ? cy[1] : Expr.Zero;
            var b = cy[0];
            var X = new SymE(x);
            var Cc = new SymE("C");
            Expr sol;
            if (a.IsZero) sol = Simp.Add(Calculus.Integrate(b, x) ?? throw new CalcException("Unable to integrate"), Cc);
            else
            {
                if (!Free(a, x)) throw new CalcException("Only constant coefficients are supported");
                var ex = F("exp", Simp.Mul(a, X));
                var ib = Calculus.Integrate(Simp.Mul(b, F("exp", Simp.Neg(Simp.Mul(a, X)))), x);
                if (ib == null) throw new CalcException("Unable to integrate");
                sol = Simp.Mul(ex, Simp.Add(ib, Cc));
            }
            if (x0 != null && y0 != null)
            {
                var atX0 = c.Ev.EvalWith(sol, x, x0);
                var cs = Solve(c, new CmpE(CmpOp.Eq, atX0, y0), "C");
                if (cs.Count > 0) sol = c.Ev.Eval(c.Substitute(sol, Cc, cs[0]));
            }
            return Simp.Expand(sol);
        }

        public static Expr NumberOrNull(Expr e) => e is NumE ? e : null;

        static double NumericValue(Expr e)
        {
            if (e is NumE n && n.V.IsReal) return n.V.Re.ToDouble();
            try
            {
                var save = C.ExactPhase; C.ExactPhase = false;
                try { var a = C.Ev.Eval(e); return a is NumE an && an.V.IsReal ? an.V.Re.ToDouble() : double.MaxValue; }
                finally { C.ExactPhase = save; }
            }
            catch (CalcException) { return double.MaxValue; }
        }
    }
}
