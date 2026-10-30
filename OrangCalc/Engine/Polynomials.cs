using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;

namespace OrangCalc.Engine
{
    // univariate polynomial with rational coefficients, index = degree
    public sealed class Poly
    {
        public List<Real> C;
        public Poly(List<Real> c) { C = c; Trim(); }
        public int Degree => C.Count - 1;
        public Real Lead => C[C.Count - 1];
        public bool IsZero => C.Count == 1 && C[0].IsZero;

        void Trim()
        {
            while (C.Count > 1 && C[C.Count - 1].IsZero) C.RemoveAt(C.Count - 1);
            if (C.Count == 0) C.Add(Real.Zero);
        }

        public Real Eval(Real x)
        {
            Real r = Real.Zero;
            for (int i = C.Count - 1; i >= 0; i--) r = r * x + C[i];
            return r;
        }

        public static Poly DivMod(Poly a, Poly b, out Poly rem)
        {
            var r = a.C.ToList();
            int db = b.Degree;
            var q = new List<Real>(Enumerable.Repeat(Real.Zero, Math.Max(1, a.Degree - db + 1)));
            for (int i = a.Degree - db; i >= 0; i--)
            {
                var f = r[i + db] / b.Lead;
                q[i] = f;
                for (int j = 0; j <= db; j++) r[i + j] -= f * b.C[j];
            }
            rem = new Poly(r.Take(Math.Max(1, db)).ToList());
            return new Poly(q);
        }

        public Expr ToExpr(Expr x)
        {
            var terms = new List<Expr>();
            for (int i = C.Count - 1; i >= 0; i--)
            {
                if (C[i].IsZero) continue;
                terms.Add(Simp.Mul(new NumE(new Cx(C[i])), Simp.Pow(x, Expr.N(i))));
            }
            return Simp.Add(terms);
        }

        public Poly Derivative()
        {
            var d = new List<Real>();
            for (int i = 1; i < C.Count; i++) d.Add(C[i] * Real.Int(i));
            return new Poly(d.Count == 0 ? new List<Real> { Real.Zero } : d);
        }
    }

    public static class Polynomials
    {
        // extracts integer-power coefficients of x from an expanded expression
        public static bool TryCoefficients(Expr e, string x, out List<Expr> coefs)
        {
            coefs = new List<Expr>();
            var ex = Simp.Expand(e);
            var terms = ex is AddE a ? a.Terms : new List<Expr> { ex };
            var map = new Dictionary<int, List<Expr>>();
            foreach (var t in terms)
            {
                int deg = 0;
                var rest = new List<Expr>();
                IEnumerable<Expr> fs = t is MulE m ? m.Factors : new[] { t };
                foreach (var f in fs)
                {
                    if (f is SymE s && s.Name == x) { deg += 1; continue; }
                    if (f is PowE p && p.B is SymE ps && ps.Name == x)
                    {
                        if (p.X is NumE pn && pn.V.IsExactInt && pn.V.Re.N.Sign >= 0 && pn.V.Re.N < 1000) { deg += (int)pn.V.Re.N; continue; }
                        return false;
                    }
                    if (f.Any(z => z is SymE zs && zs.Name == x)) return false;
                    rest.Add(f);
                }
                if (!map.TryGetValue(deg, out var l)) map[deg] = l = new List<Expr>();
                l.Add(Simp.Mul(rest));
            }
            int max = map.Count == 0 ? 0 : map.Keys.Max();
            for (int i = 0; i <= max; i++) coefs.Add(map.TryGetValue(i, out var l) ? Simp.Add(l) : Expr.Zero);
            return true;
        }

        public static bool TryPoly(Expr e, string x, out Poly p)
        {
            p = null;
            if (!TryCoefficients(e, x, out var cs)) return false;
            var rs = new List<Real>();
            foreach (var c in cs)
            {
                if (c is NumE n && n.V.IsRational) rs.Add(n.V.Re);
                else return false;
            }
            p = new Poly(rs);
            return true;
        }

        static List<BigInteger> PositiveDivisors(BigInteger n)
        {
            n = BigInteger.Abs(n);
            if (n.IsZero) return new List<BigInteger> { 1 };
            if (n > BigInteger.Parse("1000000000000")) return Primes.Divisors(n).Take(2000).ToList();
            return Primes.Divisors(n);
        }

        // rational roots via the rational root theorem
        public static List<Real> RationalRoots(Poly p)
        {
            var roots = new List<Real>();
            var cur = Integerize(p);
            while (cur.Degree >= 1 && cur.C[0].IsZero) { roots.Add(Real.Zero); cur = new Poly(cur.C.Skip(1).ToList()); }
            if (cur.Degree < 1) return roots;
            var a0 = cur.C[0].N; var an = cur.Lead.N;
            var ps = PositiveDivisors(a0); var qs = PositiveDivisors(an);
            if (ps.Count * qs.Count > 200000) return roots;
            bool found = true;
            while (found && cur.Degree >= 1)
            {
                found = false;
                foreach (var pp in ps)
                {
                    foreach (var qq in qs)
                    {
                        foreach (var sgn in new[] { 1, -1 })
                        {
                            var r = Real.Make(sgn * pp, qq);
                            if (cur.Eval(r).IsZero)
                            {
                                roots.Add(r);
                                cur = Poly.DivMod(cur, new Poly(new List<Real> { -r, Real.One }), out _);
                                found = true;
                                break;
                            }
                        }
                        if (found) break;
                    }
                    if (found) break;
                }
            }
            return roots;
        }

        static Poly Integerize(Poly p)
        {
            BigInteger l = 1;
            foreach (var c in p.C) l = l / BigInteger.GreatestCommonDivisor(l, c.D) * c.D;
            return new Poly(p.C.Select(c => c * Real.Int(l)).ToList());
        }

        public static Expr Factorize(Calculator c, Expr e)
        {
            if (e is NumE n && n.V.IsRational) return FactorRational(n.V.Re);
            if (e is MatE m) return Matrices.MapElements(m, x => Factorize(c, x));
            if (e is CmpE ce) return new CmpE(ce.Op, Factorize(c, ce.L), Factorize(c, ce.R));
            var syms = Calculator.Symbols(e);
            if (syms.Count == 0) return e;
            Simp.NumDen(e, out var num, out var den);
            if (!den.IsOne)
            {
                var fn = Factorize(c, num); var fd = Factorize(c, den);
                return new MulE(new List<Expr> { fn, new PowE(fd, Expr.MinusOne) });
            }
            var ex = Simp.Expand(e);
            if (syms.Count == 1 && TryPoly(ex, syms[0], out var p) && p.Degree >= 2) return FactorPoly(p, new SymE(syms[0]));
            return CommonFactor(ex);
        }

        static Expr FactorRational(Real r)
        {
            if (r.IsZero || r.Abs().IsOne) return new NumE(new Cx(r));
            var parts = new List<Expr>();
            if (r.Sign < 0) parts.Add(Expr.MinusOne);
            foreach (var kv in Primes.Factorize(r.N))
                parts.Add(kv.Value == 1 ? Expr.N(Real.Int(kv.Key)) : new PowE(Expr.N(Real.Int(kv.Key)), Expr.N(kv.Value)));
            Expr numPart = parts.Count == 1 ? parts[0] : parts.Count == 0 ? Expr.One : new MulE(parts);
            if (r.D.IsOne) return numPart;
            var dparts = new List<Expr>();
            foreach (var kv in Primes.Factorize(r.D))
                dparts.Add(kv.Value == 1 ? Expr.N(Real.Int(kv.Key)) : new PowE(Expr.N(Real.Int(kv.Key)), Expr.N(kv.Value)));
            Expr denPart = dparts.Count == 1 ? dparts[0] : new MulE(dparts);
            return new MulE(new List<Expr> { numPart, new PowE(denPart, Expr.MinusOne) });
        }

        public static Expr FactorPoly(Poly p, Expr x)
        {
            var factors = new List<Expr>();
            BigInteger l = 1;
            foreach (var cf in p.C) l = l / BigInteger.GreatestCommonDivisor(l, cf.D) * cf.D;
            var ip = new Poly(p.C.Select(cf => cf * Real.Int(l)).ToList());
            BigInteger g = 0;
            foreach (var cf in ip.C) g = BigInteger.GreatestCommonDivisor(g, cf.N);
            if (ip.Lead.Sign < 0) g = -g;
            var content = Real.Make(g, l);
            var prim = new Poly(ip.C.Select(cf => cf / Real.Int(g)).ToList());
            var roots = RationalRoots(prim);
            var rest = prim;
            var counts = new Dictionary<string, (Expr f, int k)>();
            foreach (var r in roots)
            {
                var lin = new Poly(new List<Real> { Real.Int(-r.N), Real.Int(r.D) });
                rest = Poly.DivMod(rest, lin, out _);
                var fe = Simp.Add(Simp.Mul(Expr.N(Real.Int(r.D)), x), Expr.N(Real.Int(-r.N)));
                string k = fe.Key;
                if (counts.TryGetValue(k, out var old)) counts[k] = (old.f, old.k + 1);
                else counts[k] = (fe, 1);
            }
            var mult = content * (rest.Degree == 0 ? rest.C[0] : Real.One);
            if (!mult.IsOne) factors.Add(Expr.N(mult));
            foreach (var kv in counts.Values) factors.Add(kv.k == 1 ? kv.f : new PowE(kv.f, Expr.N(kv.k)));
            if (rest.Degree >= 1)
            {
                var sq = SquareFree(rest, x);
                factors.Add(sq);
            }
            if (factors.Count == 0) return Expr.One;
            return factors.Count == 1 ? factors[0] : new MulE(factors);
        }

        static Expr SquareFree(Poly p, Expr x) => p.ToExpr(x);

        static Poly Monic(Poly p) => new Poly(p.C.Select(c => c / p.Lead).ToList());

        public static Poly Gcd(Poly a, Poly b)
        {
            while (!b.IsZero && b.Degree >= 0)
            {
                Poly.DivMod(a, b, out var r);
                a = b; b = r;
                if (b.Degree == 0 && !b.IsZero) return new Poly(new List<Real> { Real.One });
            }
            return Monic(a);
        }

        static Expr CommonFactor(Expr e)
        {
            if (!(e is AddE a)) return e;
            BigInteger g = 0, l = 1;
            foreach (var t in a.Terms)
            {
                Simp.SplitCoef(t, out var cf, out _);
                if (!cf.IsRational) return e;
                g = BigInteger.GreatestCommonDivisor(g, cf.Re.N);
                l = l / BigInteger.GreatestCommonDivisor(l, cf.Re.D) * cf.Re.D;
            }
            var common = new Dictionary<string, (Expr b, Real exp)>();
            bool first = true;
            foreach (var t in a.Terms)
            {
                Simp.SplitCoef(t, out _, out var rest);
                IEnumerable<Expr> fs = rest is MulE m ? m.Factors : new[] { rest };
                var here = new Dictionary<string, (Expr b, Real exp)>();
                foreach (var f in fs)
                {
                    if (f.IsOne) continue;
                    Expr b = f; Real ex = Real.One;
                    if (f is PowE p && p.X is NumE pn && pn.V.IsRational && pn.V.Re.Sign > 0) { b = p.B; ex = pn.V.Re; }
                    here[b.Key] = (b, ex);
                }
                if (first) { common = here; first = false; continue; }
                foreach (var k in common.Keys.ToList())
                {
                    if (!here.TryGetValue(k, out var h)) common.Remove(k);
                    else common[k] = (common[k].b, Real.Min(common[k].exp, h.exp));
                }
            }
            var cr = Real.Make(g, l);
            if (a.Terms.All(t => { Simp.SplitCoef(t, out var cf, out _); return cf.Re.Sign < 0; })) cr = -cr;
            var commonExpr = Simp.Mul(common.Values.Select(v => Simp.Pow(v.b, Expr.N(v.exp))).ToList());
            if ((cr.IsOne) && commonExpr.IsOne) return e;
            var inner = Simp.Add(a.Terms.Select(t => Simp.Div(Simp.Div(t, commonExpr), Expr.N(cr))).ToList());
            var parts = new List<Expr>();
            if (!cr.IsOne) parts.Add(Expr.N(cr));
            if (!commonExpr.IsOne) { if (commonExpr is MulE cm) parts.AddRange(cm.Factors); else parts.Add(commonExpr); }
            parts.Add(inner);
            return new MulE(parts);
        }

        public static Expr CombineFractions(Expr e) => e;

        // partial fraction decomposition over rational linear and quadratic factors
        public static Expr PartialFractions(Calculator c, Expr e)
        {
            var syms = Calculator.Symbols(e);
            if (syms.Count != 1) return e;
            string xs = syms[0];
            var x = new SymE(xs);
            Expr num, den;
            var combined = Together(e);
            Simp.NumDen(combined, out num, out den);
            num = Simp.Expand(num); den = Simp.Expand(den);
            if (!TryPoly(num, xs, out var pn) || !TryPoly(den, xs, out var pd) || pd.Degree < 1) return e;
            var q = Poly.DivMod(pn, pd, out var rem);
            var polyPart = q.ToExpr(x);
            if (rem.IsZero) return polyPart;
            var lead = pd.Lead;
            var monicD = Monic(pd);
            var roots = RationalRoots(monicD);
            var rest = monicD;
            var factors = new List<(Poly f, int k)>();
            foreach (var g in roots.GroupBy(r => r.N + "/" + r.D))
            {
                var r = g.First();
                factors.Add((new Poly(new List<Real> { -r, Real.One }), g.Count()));
                for (int i = 0; i < g.Count(); i++) rest = Poly.DivMod(rest, new Poly(new List<Real> { -r, Real.One }), out _);
            }
            if (rest.Degree >= 1)
            {
                if (rest.Degree == 2) factors.Add((rest, 1));
                else if (rest.Degree % 2 == 0 && rest.Degree <= 8 && rest.C.Where((cf, i) => i % 2 == 1).All(cf => cf.IsZero) && false) { }
                else factors.Add((rest, 1));
            }
            var unknownCount = factors.Sum(f => f.f.Degree * f.k);
            var cols = new List<Poly>();
            var shapes = new List<(int fi, int power, int deg)>();
            for (int fi = 0; fi < factors.Count; fi++)
            {
                var (f, k) = factors[fi];
                for (int j = 1; j <= k; j++)
                {
                    var other = monicD;
                    for (int t = 0; t < j; t++) other = Poly.DivMod(other, f, out _);
                    for (int d = 0; d < f.Degree; d++)
                    {
                        var xd = new List<Real>(Enumerable.Repeat(Real.Zero, d + 1)); xd[d] = Real.One;
                        cols.Add(Mul(other, new Poly(xd)));
                        shapes.Add((fi, j, d));
                    }
                }
            }
            int n = monicD.Degree;
            var A = Matrices.Create(n, cols.Count, (i, j) => Expr.N(i < cols[j].C.Count ? cols[j].C[i] : Real.Zero));
            var target = Enumerable.Range(0, n).Select(i => Expr.N(i < rem.C.Count ? rem.C[i] / lead : Real.Zero)).ToList();
            List<Expr> sol;
            try { sol = Matrices.Solve(A, target); }
            catch (CalcException) { return e; }
            var terms = new List<Expr> { polyPart };
            int idx = 0;
            for (int fi = 0; fi < factors.Count; fi++)
            {
                var (f, k) = factors[fi];
                for (int j = 1; j <= k; j++)
                {
                    var numTerms = new List<Expr>();
                    for (int d = 0; d < f.Degree; d++, idx++) numTerms.Add(Simp.Mul(sol[idx], Simp.Pow(x, Expr.N(d))));
                    var numer = Simp.Add(numTerms);
                    if (numer.IsZero) continue;
                    var fe = f.ToExpr(x);
                    terms.Add(new MulE(new List<Expr> { numer, new PowE(j == 1 ? fe : new PowE(fe, Expr.N(j)), Expr.MinusOne) }));
                }
            }
            terms = terms.Where(t => !t.IsZero).ToList();
            return terms.Count == 1 ? terms[0] : new AddE(terms);
        }

        static Poly Mul(Poly a, Poly b)
        {
            var r = new List<Real>(Enumerable.Repeat(Real.Zero, a.C.Count + b.C.Count - 1));
            for (int i = 0; i < a.C.Count; i++) for (int j = 0; j < b.C.Count; j++) r[i + j] += a.C[i] * b.C[j];
            return new Poly(r);
        }

        // brings a sum of fractions over a common denominator
        public static Expr Together(Expr e)
        {
            if (!(e is AddE a)) return e;
            var nums = new List<Expr>(); var dens = new List<Expr>();
            foreach (var t in a.Terms)
            {
                Simp.NumDen(t, out var n, out var d);
                nums.Add(n); dens.Add(d);
            }
            Expr common = Expr.One;
            var seen = new HashSet<string>();
            foreach (var d in dens)
            {
                IEnumerable<Expr> fs = d is MulE m ? m.Factors : new[] { d };
                foreach (var f in fs) { if (f.IsOne) continue; if (seen.Add(f.Key)) common = Simp.Mul(common, f); }
            }
            var terms = new List<Expr>();
            for (int i = 0; i < nums.Count; i++) terms.Add(Simp.Expand(Simp.Mul(nums[i], Simp.Div(common, dens[i]))));
            return Simp.Div(Simp.Add(terms), common);
        }

        // all complex roots numerically (Aberth method)
        public static List<System.Numerics.Complex> NumericRoots(List<System.Numerics.Complex> coef)
        {
            int n = coef.Count - 1;
            while (n > 0 && coef[n] == 0) n--;
            var res = new List<System.Numerics.Complex>();
            if (n < 1) return res;
            var a = coef.Take(n + 1).Select(c => c / coef[n]).ToArray();
            var z = new System.Numerics.Complex[n];
            double radius = 1 + a.Take(n).Max(c => System.Numerics.Complex.Abs(c));
            for (int i = 0; i < n; i++) z[i] = System.Numerics.Complex.FromPolarCoordinates(radius * 0.5 + 0.1, 2 * Math.PI * i / n + 0.4);
            for (int iter = 0; iter < 500; iter++)
            {
                double maxStep = 0;
                for (int i = 0; i < n; i++)
                {
                    System.Numerics.Complex p = 0, dp = 0;
                    for (int k = n; k >= 0; k--) { dp = dp * z[i] + p; p = p * z[i] + a[k]; }
                    var ratio = p / dp;
                    System.Numerics.Complex sum = 0;
                    for (int j = 0; j < n; j++) if (j != i) sum += 1 / (z[i] - z[j]);
                    var w = ratio / (1 - ratio * sum);
                    if (double.IsNaN(w.Real)) w = 0;
                    z[i] -= w;
                    maxStep = Math.Max(maxStep, System.Numerics.Complex.Abs(w));
                }
                if (maxStep < 1e-15) break;
            }
            return z.ToList();
        }
    }
}
