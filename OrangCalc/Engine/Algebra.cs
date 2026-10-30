using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;

namespace OrangCalc.Engine
{
    // raw tree builders used by the parser, no simplification
    public static class Ops
    {
        public static Expr Add(Expr a, Expr b)
        {
            var l = new List<Expr>();
            if (a is AddE aa) l.AddRange(aa.Terms); else l.Add(a);
            if (b is AddE bb) l.AddRange(bb.Terms); else l.Add(b);
            return new AddE(l);
        }

        public static Expr Sub(Expr a, Expr b) => Add(a, Neg(b));

        public static Expr Neg(Expr a)
        {
            if (a is NumE n) return new NumE(-n.V);
            if (a is MulE m && m.Factors.Count > 0 && m.Factors[0] is NumE f)
            {
                var l = new List<Expr>(m.Factors);
                l[0] = new NumE(-f.V);
                if (l[0].IsOne) l.RemoveAt(0);
                return l.Count == 1 ? l[0] : new MulE(l);
            }
            return new MulE(new List<Expr> { Expr.MinusOne, a });
        }

        public static Expr Mul(Expr a, Expr b)
        {
            var l = new List<Expr>();
            if (a is MulE aa) l.AddRange(aa.Factors); else l.Add(a);
            if (b is MulE bb) l.AddRange(bb.Factors); else l.Add(b);
            return new MulE(l);
        }

        public static Expr Div(Expr a, Expr b) => Mul(a, Pow(b, Expr.MinusOne));
        public static Expr Pow(Expr b, Expr e) => new PowE(b, e);
    }

    // canonicalizing constructors used during evaluation
    public static class Simp
    {
        static Calculator C => Calculator.Current;

        public static Expr Add(Expr a, Expr b) => Add(new List<Expr> { a, b });
        public static Expr Sub(Expr a, Expr b) => Add(new List<Expr> { a, Neg(b) });
        public static Expr Neg(Expr a) => Mul(new List<Expr> { Expr.MinusOne, a });
        public static Expr Mul(Expr a, Expr b) => Mul(new List<Expr> { a, b });
        public static Expr Div(Expr a, Expr b) => Mul(new List<Expr> { a, Pow(b, Expr.MinusOne) });
        public static Expr Mul(params Expr[] f) => Mul(f.ToList());
        public static Expr Add(params Expr[] f) => Add(f.ToList());

        public static Expr Num(Cx v) => new NumE(v);
        public static Expr Num(Real v) => new NumE(new Cx(v));

        public static void SplitCoef(Expr t, out Cx coef, out Expr rest)
        {
            if (t is NumE n) { coef = n.V; rest = Expr.One; return; }
            if (t is MulE m && m.Factors.Count > 0 && m.Factors[0] is NumE f)
            {
                coef = f.V;
                rest = m.Factors.Count == 2 ? m.Factors[1] : new MulE(m.Factors.Skip(1).ToList());
                return;
            }
            coef = Cx.One; rest = t;
        }

        static Expr WithCoef(Cx coef, Expr rest)
        {
            if (rest.IsOne) return new NumE(coef);
            if (coef.IsOne) return rest;
            var l = new List<Expr> { new NumE(coef) };
            if (rest is MulE m) l.AddRange(m.Factors); else l.Add(rest);
            return new MulE(l);
        }

        // splits a term into non unit part and unit part
        public static void SplitUnits(Expr rest, out Expr plain, out Expr units)
        {
            var p = new List<Expr>(); var u = new List<Expr>();
            IEnumerable<Expr> fs = rest is MulE m ? m.Factors : new[] { rest };
            foreach (var f in fs)
            {
                if (IsUnitFactor(f)) u.Add(f); else p.Add(f);
            }
            plain = p.Count == 0 ? Expr.One : p.Count == 1 ? p[0] : new MulE(p);
            units = u.Count == 0 ? Expr.One : u.Count == 1 ? u[0] : new MulE(u);
        }

        public static Expr SplitUnitsOf(Expr e)
        {
            SplitCoef(e, out _, out var rest);
            SplitUnits(rest, out _, out var u);
            return u;
        }

        public static bool IsUnitFactor(Expr f) => f is UnitE || (f is PowE pw && pw.B is UnitE && pw.X is NumE);

        public static Expr Add(List<Expr> input)
        {
            var terms = new List<Expr>();
            foreach (var t in input)
            {
                if (t is AddE a) terms.AddRange(a.Terms); else terms.Add(t);
            }
            if (terms.Count == 0) return Expr.Zero;
            if (terms.Count == 1) return terms[0];

            if (terms.Any(t => t is MatE)) return MatrixAdd(terms);
            if (terms.Any(t => t is StrE)) return new StrE(string.Concat(terms.Select(t => t is StrE s ? s.S : Formatter.PlainText(t))));
            if (terms.Any(t => t is DateE || (t is MulE m && m.Factors.Any(f => f is DateE)))) return DateAdd(terms);
            if (terms.Any(t => t is IntvE))
            {
                Expr acc = null;
                var others = new List<Expr>();
                foreach (var t in terms)
                {
                    if (t is IntvE || t is NumE n && n.V.IsReal) acc = acc == null ? t : Intervals.Add(acc, t);
                    else others.Add(t);
                }
                if (others.Count == 0) return acc;
                others.Add(acc);
                terms = others;
            }

            Cx num = Cx.Zero;
            bool hadNum = false;
            var order = new List<string>();
            var coefs = new Dictionary<string, Cx>();
            var rests = new Dictionary<string, Expr>();
            foreach (var t in terms)
            {
                if (t is NumE n) { num += n.V; hadNum = true; continue; }
                SplitCoef(t, out var cf, out var rest);
                string k = rest.Key;
                if (coefs.TryGetValue(k, out var old)) coefs[k] = old + cf;
                else { coefs[k] = cf; rests[k] = rest; order.Add(k); }
            }

            if (C != null && order.Count > 1) MergeUnitTerms(order, coefs, rests);

            var result = new List<Expr>();
            foreach (var k in order)
            {
                var cf = coefs[k];
                if (cf.IsZero && !cf.Approx) continue;
                if (cf.IsZero) continue;
                result.Add(WithCoef(cf, rests[k]));
            }
            if (!num.IsZero || (hadNum && result.Count == 0)) result.Add(new NumE(num));
            if (result.Count == 0) return Expr.Zero;
            if (result.Count == 1) return result[0];
            result.Sort(CompareTerms);
            return new AddE(result);
        }

        // terms like 1 m + 2 ft are converted to the unit of the first term
        static void MergeUnitTerms(List<string> order, Dictionary<string, Cx> coefs, Dictionary<string, Expr> rests)
        {
            for (int i = 0; i < order.Count; i++)
            {
                var ri = rests[order[i]];
                if (!ri.HasUnits) continue;
                SplitUnits(ri, out var pi, out var ui);
                if (!Units.TryDims(ui, out var di, out var fi)) continue;
                for (int j = i + 1; j < order.Count; j++)
                {
                    var rj = rests[order[j]];
                    if (!rj.HasUnits) continue;
                    SplitUnits(rj, out var pj, out var uj);
                    if (pj.Key != pi.Key) continue;
                    if (!Units.TryDims(uj, out var dj, out var fj) || !di.SameAs(dj)) continue;
                    var ratio = new Cx(fj / fi);
                    coefs[order[i]] = coefs[order[i]] + coefs[order[j]] * ratio;
                    coefs.Remove(order[j]); rests.Remove(order[j]);
                    order.RemoveAt(j); j--;
                }
            }
        }

        static Expr DateAdd(List<Expr> terms)
        {
            DateE baseDate = null;
            DateE subDate = null;
            Real seconds = Real.Zero;
            int months = 0;
            foreach (var t in terms)
            {
                if (t is DateE d)
                {
                    if (baseDate == null) baseDate = d;
                    else throw new CalcException("Two dates can not be added");
                    continue;
                }
                if (t is MulE m && m.Factors.Count == 2 && m.Factors[0].IsMinusOne && m.Factors[1] is DateE nd)
                {
                    if (subDate != null) throw new CalcException("Unsupported date expression");
                    subDate = nd;
                    continue;
                }
                if (t is NumE n && n.V.IsReal) { seconds += n.V.Re * Real.Int(86400); continue; }
                if (t is UnitE bu && bu.P == null && (bu.U.Name == "yr" || bu.U.Name == "mo")) { months += bu.U.Name == "yr" ? 12 : 1; continue; }
                if (t is MulE tm && tm.Factors.Count == 2 && tm.Factors[0] is NumE tn && tn.V.IsReal && tn.V.Re.IsInt && tm.Factors[1] is UnitE tu && tu.P == null)
                {
                    int perUnit = tu.U.Name == "yr" ? 12 : tu.U.Name == "mo" ? 1 : tu.U.Name == "decade" ? 120 : tu.U.Name == "century" ? 1200 : tu.U.Name == "millennium" ? 12000 : 0;
                    if (perUnit > 0) { months += tn.V.Re.ToInt() * perUnit; continue; }
                }
                if (Units.ToSeconds(t, out var secs)) { seconds += secs; continue; }
                throw new CalcException("Invalid date arithmetic");
            }
            if (baseDate != null && subDate != null)
            {
                var diff = (baseDate.T - subDate.T).Ticks;
                Real s = Real.Make(diff, 10000000) + seconds;
                if (!baseDate.HasTime && !subDate.HasTime && seconds.IsZero)
                    return Mul(Num(s / Real.Int(86400)), new UnitE(C.FindUnit("d", out _)));
                return Mul(Num(s), new UnitE(C.FindUnit("s", out _)));
            }
            if (baseDate == null && subDate != null) throw new CalcException("Invalid date arithmetic");
            if (baseDate == null) throw new CalcException("Invalid date arithmetic");
            if (months != 0) baseDate = new DateE(baseDate.T.AddMonths(months), baseDate.HasTime, baseDate.Utc);
            return Dates.AddSeconds(baseDate, seconds);
        }

        static int Rank(Expr e)
        {
            if (e is NumE) return 9;
            if (e is IntvE) return 9;
            return 1;
        }

        public static int Degree(Expr e)
        {
            switch (e)
            {
                case SymE _: return 1;
                case VarE _: return 1;
                case PowE p when p.X is NumE n && n.V.IsReal: return (int)Math.Max(-100, Math.Min(100, Degree(p.B) * Math.Round(n.V.Re.ToDouble())));
                case PowE p: return Degree(p.B);
                case MulE m: return m.Factors.Sum(Degree);
                case CallE _: return 1;
                case UnitE _: return 0;
                default: return 0;
            }
        }

        static string MainName(Expr e)
        {
            switch (e)
            {
                case SymE s: return s.Name;
                case VarE v: return "~" + v.V.Name;
                case PowE p: return MainName(p.B);
                case MulE m: foreach (var f in m.Factors) { var n = MainName(f); if (n != null) return n; } return null;
                case CallE c: return "~~" + c.F.Name;
                default: return null;
            }
        }

        public static int CompareTerms(Expr a, Expr b)
        {
            int ra = Rank(a), rb = Rank(b);
            if (ra != rb) return ra.CompareTo(rb);
            string na = MainName(a) ?? "￿", nb = MainName(b) ?? "￿";
            int c = string.CompareOrdinal(na, nb);
            if (c != 0) return c;
            int da = Degree(a), db = Degree(b);
            if (da != db) return db.CompareTo(da);
            return string.CompareOrdinal(a.Key, b.Key);
        }

        static int FactorRank(Expr e)
        {
            switch (e)
            {
                case NumE _: return 0;
                case IntvE _: return 0;
                case MatE _: return 5;
                case UnitE _: return 7;
                case PowE p when p.B is UnitE: return p.X is NumE n && n.V.IsReal && n.V.Re.Sign < 0 ? 8 : 7;
                case AddE _: return 3;
                default: return 2;
            }
        }

        public static int CompareFactors(Expr a, Expr b)
        {
            int ra = FactorRank(a), rb = FactorRank(b);
            if (ra != rb) return ra.CompareTo(rb);
            if (ra == 5) return 0;
            string na = MainName(a) ?? a.Key, nb = MainName(b) ?? b.Key;
            int c = string.CompareOrdinal(na, nb);
            if (c != 0) return c;
            return string.CompareOrdinal(a.Key, b.Key);
        }

        static void BaseExp(Expr f, out Expr b, out Expr e)
        {
            if (f is PowE p) { b = p.B; e = p.X; }
            else { b = f; e = Expr.One; }
        }

        public static Expr Mul(List<Expr> input)
        {
            var fs = new List<Expr>();
            foreach (var f in input)
            {
                if (f is MulE m) fs.AddRange(m.Factors); else fs.Add(f);
            }
            if (fs.Count == 0) return Expr.One;
            if (fs.Count == 1) return fs[0];

            if (fs.Any(f => f is MatE)) return MatrixMul(fs);
            if (fs.Any(f => f is DateE))
            {
                if (fs.Count == 2 && fs.Any(f => f.IsMinusOne)) return new MulE(fs.OrderBy(f => f is DateE ? 1 : 0).ToList());
                throw new CalcException("Dates can not be multiplied");
            }
            if (fs.Any(f => f is StrE)) throw new CalcException("Text can not be multiplied");
            if (fs.Any(f => f is CmpE))
            {
                int idx = fs.FindIndex(f => f is CmpE);
                var ce = (CmpE)fs[idx];
                var others = fs.Where((f, i) => i != idx).ToList();
                if (others.All(o => o is NumE))
                {
                    var k = Mul(others);
                    return Calculator.Current.Ev.EvalCmp(new CmpE(ce.Op, Mul(k, ce.L), Mul(k, ce.R)));
                }
            }
            if (fs.Any(f => f is IntvE))
            {
                Expr acc = null;
                var others = new List<Expr>();
                foreach (var f in fs)
                {
                    if (f is IntvE || f is NumE n && n.V.IsReal) acc = acc == null ? f : Intervals.Mul(acc, f);
                    else others.Add(f);
                }
                if (others.Count == 0) return acc;
                others.Insert(0, acc);
                fs = others;
                if (!(acc is NumE)) return new MulE(Sorted(fs));
            }

            Cx coef = Cx.One;
            var order = new List<string>();
            var bases = new Dictionary<string, Expr>();
            var exps = new Dictionary<string, List<Expr>>();
            foreach (var f in fs)
            {
                if (f is NumE n) { coef *= n.V; continue; }
                BaseExp(f, out var b, out var e);
                string k = b.Key;
                if (!exps.TryGetValue(k, out var lst)) { exps[k] = lst = new List<Expr>(); bases[k] = b; order.Add(k); }
                lst.Add(e);
            }
            if (coef.IsZero && !order.Any(k => bases[k].HasUnits)) return new NumE(coef);

            var result = new List<Expr>();
            var numRoots = new Dictionary<string, (Real b, Expr e)>();
            foreach (var k in order)
            {
                var b = bases[k];
                var e = exps[k].Count == 1 ? exps[k][0] : Add(exps[k]);
                var pw = Pow(b, e);
                if (pw is NumE pn) { coef *= pn.V; continue; }
                if (pw is MulE pm)
                {
                    foreach (var x in pm.Factors)
                    {
                        if (x is NumE xn) coef *= xn.V;
                        else result.Add(x);
                    }
                    continue;
                }
                result.Add(pw);
            }

            result = MergeNumericRoots(result, ref coef);
            if (C != null) result = MergeUnitFactors(result, ref coef);

            if (coef.IsZero && !result.Any(r => r.HasUnits)) return new NumE(coef);
            if (result.Count == 0) return new NumE(coef);
            if (!coef.IsOne) result.Insert(0, new NumE(coef));
            if (result.Count == 1) return result[0];
            if (result.Count == 2 && result[0] is NumE cn && result[1] is AddE add && C != null && C.DistributeNumbers)
                return Add(add.Terms.Select(t => Mul(new NumE(cn.V), t)).ToList());
            return new MulE(Sorted(result));
        }

        static List<Expr> Sorted(List<Expr> l)
        {
            var arr = l.ToList();
            var idx = Enumerable.Range(0, arr.Count).ToList();
            idx.Sort((x, y) => { int c = CompareFactors(arr[x], arr[y]); return c != 0 ? c : x.CompareTo(y); });
            return idx.Select(i => arr[i]).ToList();
        }

        // 2^(1/2)*3^(1/2) -> 6^(1/2)
        static List<Expr> MergeNumericRoots(List<Expr> fs, ref Cx coef)
        {
            var groups = new Dictionary<string, List<int>>();
            for (int i = 0; i < fs.Count; i++)
            {
                if (fs[i] is PowE p && p.B is NumE bn && bn.V.IsRational && p.X is NumE xn && xn.V.IsRational)
                {
                    string k = p.X.Key;
                    if (!groups.TryGetValue(k, out var l)) groups[k] = l = new List<int>();
                    l.Add(i);
                }
            }
            if (!groups.Values.Any(g => g.Count > 1)) return fs;
            var res = new List<Expr>();
            var used = new HashSet<int>();
            foreach (var g in groups.Values.Where(g => g.Count > 1))
            {
                Real prod = Real.One;
                foreach (int i in g) { prod *= ((NumE)((PowE)fs[i]).B).V.Re; used.Add(i); }
                var pw = Pow(new NumE(new Cx(prod)), ((PowE)fs[g[0]]).X);
                if (pw is NumE pn) coef *= pn.V;
                else if (pw is MulE pm) foreach (var x in pm.Factors) { if (x is NumE xn) coef *= xn.V; else res.Add(x); }
                else res.Add(pw);
            }
            for (int i = 0; i < fs.Count; i++) if (!used.Contains(i)) res.Add(fs[i]);
            return res;
        }

        // different units of the same dimension in one product are converted to the first one
        static List<Expr> MergeUnitFactors(List<Expr> fs, ref Cx coef)
        {
            var unitIdx = new List<int>();
            for (int i = 0; i < fs.Count; i++) if (IsUnitFactor(fs[i])) unitIdx.Add(i);
            if (unitIdx.Count < 2) return fs;
            bool changed = false;
            var list = fs.ToList();
            for (int a = 0; a < unitIdx.Count; a++)
            {
                int ia = unitIdx[a];
                if (list[ia] == null) continue;
                BaseExp(list[ia], out var ba, out var ea);
                var ua = (UnitE)ba;
                if (ua.U.HasOffset && unitIdx.Count > 1) { }
                for (int b = a + 1; b < unitIdx.Count; b++)
                {
                    int ib = unitIdx[b];
                    if (list[ib] == null) continue;
                    BaseExp(list[ib], out var bb, out var eb);
                    var ub = (UnitE)bb;
                    if (!ua.U.Dim.SameAs(ub.U.Dim) || ua.U.Dim.IsNone) continue;
                    if (!(eb is NumE ebn) || !ebn.V.IsReal) continue;
                    var ratio = ub.Factor / ua.Factor;
                    coef *= new Cx(RMath.Pow(ratio, ebn.V.Re));
                    var ne = Add(ea, eb);
                    ea = ne;
                    list[ib] = null;
                    changed = true;
                }
                if (changed) list[ia] = Pow(ba, ea);
            }
            if (!changed) return fs;
            var res = new List<Expr>();
            foreach (var x in list)
            {
                if (x == null) continue;
                if (x is NumE xn) { coef *= xn.V; continue; }
                res.Add(x);
            }
            return res;
        }

        public static Expr Pow(Expr b, Expr e)
        {
            if (e.IsZero)
            {
                if (b is MatE m) return Matrices.Identity(m.RowCount);
                return Expr.One;
            }
            if (e.IsOne) return b;
            if (b.IsOne && !(b is NumE bn0 && bn0.V.Approx)) return b;
            if (b is NumE bnum && e is NumE enm) return NumPow(bnum.V, enm.V);
            if (b.IsZero && e is NumE ez && ez.V.IsReal && ez.V.Re.Sign > 0) return Expr.Zero;
            if (b is IntvE || e is IntvE) return Intervals.Pow(b, e);
            if (b is MatE mb)
            {
                if (e is NumE en && en.V.IsExactInt) return Matrices.Power(mb, en.V.Re.N);
                if (b is MatE && e is NumE) throw new CalcException("Matrix power requires an integer exponent");
                return new PowE(b, e);
            }
            if (b is PowE pb)
            {
                if (e is NumE en && en.V.IsExactInt) return Pow(pb.B, Mul(pb.X, e));
                if (pb.X is NumE pxn && pxn.V.IsExactInt && e is NumE en2 && en2.V.IsReal)
                {
                    var prod = Mul(pb.X, e);
                    bool evenRoot = !pxn.V.Re.N.IsEven ? false : true;
                    if (prod.IsOne && evenRoot)
                    {
                        if (C.IsKnownNonNegative(pb.B)) return pb.B;
                        if (C.IsKnownReal(pb.B)) return C.Ev.Call("abs", pb.B);
                    }
                    if (!evenRoot || C.IsKnownNonNegative(pb.B)) return Pow(pb.B, prod);
                }
                if (pb.B is UnitE) return Pow(pb.B, Mul(pb.X, e));
            }
            if (b is MulE mm)
            {
                if (e is NumE en && en.V.IsExactInt)
                    return Mul(mm.Factors.Select(f => Pow(f, e)).ToList());
                var pos = new List<Expr>(); var rest = new List<Expr>();
                foreach (var f in mm.Factors)
                {
                    if ((f is NumE fn && fn.V.IsReal && fn.V.Re.Sign > 0) || IsUnitFactor(f) || C.IsKnownPositive(f)) pos.Add(f);
                    else rest.Add(f);
                }
                if (pos.Count > 0)
                {
                    var parts = pos.Select(f => Pow(f, e)).ToList();
                    if (rest.Count > 0) parts.Add(new PowE(rest.Count == 1 ? rest[0] : new MulE(rest), e));
                    return Mul(parts);
                }
            }
            if (b is VarE ve && ve.V.Name == "e" && e is CallE lc && lc.F.Name == "ln") return lc.Args[0];
            if (b is VarE ve2 && ve2.V.Name == "e" && e is MulE em && em.Factors.Count == 2 && em.Factors[0] is NumE ec && ec.V.Re.IsZero && !ec.V.Im.Approx && em.Factors[1] is VarE epi && epi.V.Name == "pi")
            {
                var ang = Mul(new NumE(new Cx(ec.V.Im)), em.Factors[1]);
                var cs = Funcs.ExactTrigPublic("cos", ang); var sn = Funcs.ExactTrigPublic("sin", ang);
                if (cs != null && sn != null) return Add(cs, Mul(new NumE(Cx.I), sn));
            }
            if (b is NumE bb && bb.V.IsRational && bb.V.Re.Sign > 0 && e is CallE lg && lg.F.Name == "log" && lg.Args.Count == 2 && lg.Args[1].Same(b)) return lg.Args[0];
            return new PowE(b, e);
        }

        public static Expr NumPow(Cx b, Cx e)
        {
            if (e.IsExactInt)
            {
                var n = e.Re.N;
                if (BigInteger.Abs(n) > 100000000 && !b.Approx && !(b.IsReal && (b.Re.IsOne || b.Re.IsZero || b.Re.EqualsValue(Real.MinusOne))))
                    return new NumE(b.AsApprox().PowInt(n));
                if (b.IsZero && n.Sign < 0) throw new CalcException("Division by zero");
                return new NumE(b.PowInt(n));
            }
            bool exact = C == null || C.ExactPhase;
            if (exact && !b.Approx && !e.Approx && e.IsReal && b.IsReal)
            {
                var r = ExactRationalPow(b.Re, e.Re);
                if (r != null) return r;
                return new PowE(new NumE(b), new NumE(e));
            }
            if (exact && !b.Approx && !e.Approx) return new PowE(new NumE(b), new NumE(e));
            if (b.IsReal && e.IsReal && b.Re.Sign < 0 && C != null && !C.Opts.ComplexEnabled && !(C.Opts.RealRoots && !e.Re.D.IsEven))
                throw new CalcException("Complex numbers are disabled");
            if (b.IsReal && e.IsReal && b.Re.Sign < 0 && C != null && C.Opts.RealRoots && !e.Re.Approx && !e.Re.D.IsEven)
            {
                var root = RMath.Root((-b.Re).AsApprox(), (int)e.Re.D);
                return new NumE(new Cx((-root).PowInt(e.Re.N)));
            }
            return new NumE(Cx.Pow(b, e).Clean());
        }

        // exact power of a rational to a rational exponent, extracting perfect powers
        static Expr ExactRationalPow(Real b, Real e)
        {
            if (e.D > 1000 || BigInteger.Abs(e.N) > 100000) return null;
            int q = (int)e.D;
            BigInteger p = e.N;
            if (b.Sign < 0)
            {
                if (q % 2 == 1 && (C == null || C.Opts.RealRoots))
                {
                    var r = ExactRationalPow(-b, e);
                    if (r == null) return null;
                    return p.IsEven ? r : Neg(r);
                }
                if (q == 2)
                {
                    var r = ExactRationalPow(-b, e);
                    if (r == null) return null;
                    var ip = Cx.I.PowInt(p);
                    return Mul(new NumE(ip), r);
                }
                return null;
            }
            var root = b.ExactRoot(q);
            if (root != null) return new NumE(new Cx(root.PowInt(p)));
            ExtractPower(b.N, q, out var on, out var rn);
            ExtractPower(b.D, q, out var od, out var rd);
            if (!rd.IsOne)
            {
                var ddd = BigInteger.Pow(rd, q - 1);
                rn *= ddd; od *= rd;
                ExtractPower(rn, q, out var on2, out var rn2);
                on *= on2; rn = rn2;
            }
            var outer = Real.Make(on, od).PowInt(p);
            Real inner = Real.Int(rn);
            BigInteger whole = BigInteger.Divide(p, q);
            BigInteger remP = p - whole * q;
            if (remP.Sign < 0) { remP += q; whole -= 1; }
            var coefficient = outer * inner.PowInt(whole);
            var g = BigInteger.GreatestCommonDivisor(remP, q);
            var ex = Real.Make(remP / g, q / g);
            if (inner.IsOne) return new NumE(new Cx(coefficient));
            var rootPart = new PowE(new NumE(new Cx(inner)), new NumE(new Cx(ex)));
            if (coefficient.IsOne) return rootPart;
            return new MulE(new List<Expr> { new NumE(new Cx(coefficient)), rootPart });
        }

        static void ExtractPower(BigInteger n, int q, out BigInteger outer, out BigInteger rest)
        {
            outer = 1; rest = n;
            if (rest < 2) return;
            foreach (int pr in Primes.Small)
            {
                BigInteger pq = BigInteger.Pow(pr, q);
                if (pq > rest) break;
                while ((rest % pq).IsZero) { rest /= pq; outer *= pr; }
            }
            var r = Real.IRoot(rest, q);
            if (BigInteger.Pow(r, q) == rest) { outer *= r; rest = 1; }
        }

        static Expr MatrixAdd(List<Expr> terms)
        {
            MatE acc = null;
            var scalars = new List<Expr>();
            foreach (var t in terms)
            {
                if (t is MatE m)
                {
                    if (acc == null) { acc = m; continue; }
                    if (m.RowCount != acc.RowCount || m.ColCount != acc.ColCount)
                        throw new CalcException("Matrix dimensions do not match for addition");
                    var rows = new List<List<Expr>>();
                    for (int i = 0; i < m.RowCount; i++)
                    {
                        var r = new List<Expr>();
                        for (int j = 0; j < m.ColCount; j++) r.Add(Add(acc.Rows[i][j], m.Rows[i][j]));
                        rows.Add(r);
                    }
                    acc = new MatE(rows, acc.IsVector && m.IsVector);
                }
                else scalars.Add(t);
            }
            if (scalars.Count > 0)
            {
                var s = Add(scalars);
                acc = Matrices.MapElements(acc, x => Add(x, s));
            }
            return acc;
        }

        static Expr MatrixMul(List<Expr> fs)
        {
            var scalars = new List<Expr>();
            Expr acc = null;
            foreach (var f in fs)
            {
                if (f is MatE m)
                {
                    if (acc == null) acc = m;
                    else acc = Matrices.Product((MatE)acc, m);
                }
                else scalars.Add(f);
            }
            if (scalars.Count == 0) return acc;
            var s = Mul(scalars);
            if (acc is MatE am) return Matrices.MapElements(am, x => Mul(s, x));
            return Mul(s, acc);
        }

        // expands products and integer powers of sums
        public static Expr Expand(Expr e)
        {
            if (e is AddE a) return Add(a.Terms.Select(Expand).ToList());
            if (e is MatE || e is CmpE || e is LogicE) return e.Rebuild(e.Kids.Select(Expand).ToList());
            if (e is MulE m)
            {
                var fs = m.Factors.Select(Expand).ToList();
                Expr acc = Expr.One;
                foreach (var f in fs) acc = MulExpand(acc, f);
                return acc;
            }
            if (e is PowE p)
            {
                var b = Expand(p.B);
                if (b is AddE && p.X is NumE n && n.V.IsExactInt && n.V.Re.N.Sign > 0 && n.V.Re.N <= 64)
                {
                    int k = (int)n.V.Re.N;
                    Expr acc = Expr.One;
                    for (int i = 0; i < k; i++) acc = MulExpand(acc, b);
                    return acc;
                }
                if (b is AddE ab && p.X is NumE nn && nn.V.IsExactInt && nn.V.Re.N.Sign < 0 && nn.V.Re.N >= -64)
                {
                    var pos = Expand(Pow(b, new NumE(-nn.V)));
                    return Pow(pos, Expr.MinusOne);
                }
                return Pow(b, p.X);
            }
            if (e is CallE c) return c.F.Name == "_hold" ? e : new CallE(c.F, c.Args.Select(Expand).ToList());
            return e;
        }

        static Expr MulExpand(Expr a, Expr b)
        {
            var at = a is AddE aa ? aa.Terms : new List<Expr> { a };
            var bt = b is AddE bb ? bb.Terms : new List<Expr> { b };
            if (at.Count * bt.Count > 20000) throw new CalcException("Expression too large to expand");
            var res = new List<Expr>();
            foreach (var x in at) foreach (var y in bt) res.Add(Mul(x, y));
            return Add(res);
        }

        // splits into numerator and denominator
        public static void NumDen(Expr e, out Expr num, out Expr den)
        {
            var n = new List<Expr>(); var d = new List<Expr>();
            IEnumerable<Expr> fs = e is MulE m ? m.Factors : new[] { e };
            foreach (var f in fs)
            {
                if (f is PowE p && p.X is NumE x && x.V.IsReal && x.V.Re.Sign < 0) d.Add(Pow(p.B, new NumE(-x.V)));
                else if (f is NumE fn && fn.V.IsRational && !fn.V.Re.D.IsOne)
                {
                    n.Add(new NumE(new Cx(Real.Int(fn.V.Re.N))));
                    d.Add(new NumE(new Cx(Real.Int(fn.V.Re.D))));
                }
                else n.Add(f);
            }
            num = Mul(n); den = Mul(d);
        }
    }
}
