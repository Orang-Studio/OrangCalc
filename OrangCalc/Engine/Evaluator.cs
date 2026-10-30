using System;
using System.Collections.Generic;
using System.Linq;

namespace OrangCalc.Engine
{
    public sealed class Evaluator
    {
        readonly Calculator c;
        int depth;
        public readonly Dictionary<string, Expr> Bindings = new Dictionary<string, Expr>();

        public Evaluator(Calculator calc) { c = calc; }

        public Expr Call(string fn, params Expr[] args) => EvalCall(new CallE(c.Fn(fn), args.ToList()), true);

        public Expr Eval(Expr e)
        {
            if (e == null) return null;
            if (++depth > 400) { depth = 0; throw new CalcException("Expression too deeply nested"); }
            try
            {
                c.CheckAbort();
                switch (e)
                {
                    case NumE n:
                        if (!c.Opts.ComplexEnabled && !n.V.IsReal) throw new CalcException("Complex numbers are disabled");
                        return e;
                    case SymE s:
                        if (Bindings.TryGetValue(s.Name, out var bound)) return bound;
                        return e;
                    case VarE v: return EvalVar(v);
                    case UnitE _: return e;
                    case AddE a: return Simp.Add(a.Terms.Select(Eval).ToList());
                    case MulE m: return Simp.Mul(m.Factors.Select(Eval).ToList());
                    case PowE p: return Simp.Pow(Eval(p.B), Eval(p.X));
                    case CallE ce: return EvalCall(ce, false);
                    case MatE me: return Matrices.MapElements(me, Eval);
                    case CmpE cmp: return EvalCmp(new CmpE(cmp.Op, Eval(cmp.L), Eval(cmp.R)));
                    case LogicE le: return EvalLogic(le);
                    case ArgE _: throw new CalcException("Argument placeholder outside of function");
                    default: return e;
                }
            }
            finally { depth--; }
        }

        Expr EvalVar(VarE v)
        {
            var var = v.V;
            if (var.KeepSymbolic && c.ExactPhase) return v;
            if (var.Dynamic != null) return var.Dynamic(c);
            if (var.Value == null && !string.IsNullOrEmpty(var.ValueText)) var.Value = Parser.ParseText(var.ValueText, c);
            if (var.Value == null) return v;
            var val = Eval(var.Value);
            if (var.IsConstant && var.HasUnit && !c.Opts.UnitsInConstants) val = StripUnits(val);
            if (var.IsApproximate && val is NumE vn && !vn.V.Approx && !c.ExactPhase) val = new NumE(vn.V.AsApprox());
            return val;
        }

        public static Expr StripUnits(Expr e)
        {
            if (!e.HasUnits) return e;
            return Simp.Mul(((e is MulE m) ? m.Factors : new List<Expr> { e }).Where(f => !Simp.IsUnitFactor(f)).ToList());
        }

        public Expr EvalCall(CallE ce, bool argsEvaluated)
        {
            var f = ce.F;
            var args = new List<Expr>(ce.Args.Count);
            for (int i = 0; i < ce.Args.Count; i++)
            {
                if (argsEvaluated || f.HoldAll || f.HoldArgs.Contains(i)) args.Add(ce.Args[i]);
                else args.Add(Eval(ce.Args[i]));
            }
            for (int i = args.Count; i < f.Args.Count && i < (f.MaxArgs < 0 ? f.Args.Count : f.MaxArgs); i++)
            {
                if (f.Args[i].Default == null) break;
                var d = Parser.ParseText(f.Args[i].Default, c);
                args.Add(f.HoldAll || f.HoldArgs.Contains(i) ? d : Eval(d));
            }
            if (args.Count < f.MinArgs) throw new CalcException(f.Names[0] + "() requires at least " + f.MinArgs + " argument" + (f.MinArgs == 1 ? "" : "s"));
            if (f.MaxArgs >= 0 && args.Count > f.MaxArgs) throw new CalcException("Too many arguments for " + f.Names[0] + "()");

            if (f.Elementwise && args.Count > 0 && args[0] is MatE m0)
            {
                return Matrices.MapElements(m0, x =>
                {
                    var a2 = args.ToList(); a2[0] = x;
                    return EvalCall(new CallE(f, a2), true);
                });
            }
            if (f.Elementwise && args.Count > 0 && args[0] is IntvE iv && f.Impl != null)
            {
                return Intervals.Apply(iv, r =>
                {
                    var a2 = args.ToList(); a2[0] = new NumE(new Cx(r.AsApprox()));
                    var save = c.ExactPhase; c.ExactPhase = false;
                    try
                    {
                        var res = f.Impl(c, a2);
                        if (res is NumE rn && rn.V.IsReal) return rn.V.Re;
                        throw new CalcException("Unsupported interval argument for " + f.Name + "()");
                    }
                    finally { c.ExactPhase = save; }
                });
            }
            CheckArgs(f, args);

            if (f.IsUserFormula)
            {
                if (f.Parsed == null) f.Parsed = Parser.ParseText(f.Formula, c);
                var body = f.Parsed.Map(x => x is ArgE ae ? (ae.Index < args.Count ? args[ae.Index] : throw new CalcException("Missing argument for " + f.Name + "()")) : x);
                return Eval(body);
            }
            var r = f.Impl(c, args);
            if (r == null) return new CallE(f, args);
            return r;
        }

        void CheckArgs(MathFunction f, List<Expr> args)
        {
            for (int i = 0; i < args.Count && i < f.Args.Count; i++)
            {
                var a = args[i];
                var t = f.Args[i].Type;
                if (f.HoldAll || f.HoldArgs.Contains(i)) continue;
                bool symbolic = a.HasSymbols || a is CallE || a is VarE || a is PowE && !(((PowE)a).B is UnitE);
                switch (t)
                {
                    case ArgType.Integer:
                    case ArgType.NonNegInt:
                    case ArgType.PositiveInt:
                        if (a is NumE n && !n.V.IsInt)
                            throw new CalcException("Argument " + (i + 1) + " (" + f.Args[i].Name + ") of " + f.Names[0] + "() must be an integer");
                        if (a is NumE n2 && t == ArgType.NonNegInt && n2.V.Re.Sign < 0) throw new CalcException("Argument " + (i + 1) + " of " + f.Names[0] + "() must be non-negative");
                        if (a is NumE n3 && t == ArgType.PositiveInt && n3.V.Re.Sign <= 0) throw new CalcException("Argument " + (i + 1) + " of " + f.Names[0] + "() must be positive");
                        break;
                    case ArgType.Real:
                        if (a is NumE rn && !rn.V.IsReal) throw new CalcException("Argument " + (i + 1) + " of " + f.Names[0] + "() must be real");
                        break;
                    case ArgType.Matrix:
                    case ArgType.Vector:
                        if (!(a is MatE) && !symbolic && !(a is NumE)) throw new CalcException("Argument " + (i + 1) + " of " + f.Names[0] + "() must be a " + (t == ArgType.Matrix ? "matrix" : "vector"));
                        break;
                }
            }
        }

        public Expr EvalCmp(CmpE e)
        {
            var l = e.L; var r = e.R;
            if (l is StrE ls && r is StrE rs)
            {
                bool eq = ls.S == rs.S;
                if (e.Op == CmpOp.Eq) return Bool(eq);
                if (e.Op == CmpOp.Ne) return Bool(!eq);
            }
            if (l is DateE ld && r is DateE rd) return Bool(Compare(e.Op, ld.T.CompareTo(rd.T)));
            if (l is MatE lm && r is MatE rm && (e.Op == CmpOp.Eq || e.Op == CmpOp.Ne))
            {
                bool eq = lm.Key == rm.Key;
                return Bool(e.Op == CmpOp.Eq ? eq : !eq);
            }
            var diff = Simp.Sub(l, r);
            if (diff is NumE dn)
            {
                if (!dn.V.IsReal)
                {
                    if (e.Op == CmpOp.Eq) return Bool(false);
                    if (e.Op == CmpOp.Ne) return Bool(true);
                    throw new CalcException("Complex numbers can not be ordered");
                }
                int s = dn.V.Re.Sign;
                if (dn.V.Approx && Math.Abs(dn.V.Re.ToDouble()) < 1e-30 * Math.Max(1, Math.Abs(Magnitude(l)))) s = 0;
                return Bool(Compare(e.Op, s));
            }
            if (diff is IntvE di)
            {
                if (di.Lo.Sign > 0) return Bool(Compare(e.Op, 1));
                if (di.Hi.Sign < 0) return Bool(Compare(e.Op, -1));
            }
            if (!c.ExactPhase && Units.TryQuantity(diff, out var qv, out var qd, out var qr) && qr.IsOne && qv.IsReal && l.HasUnits)
                return Bool(Compare(e.Op, qv.Re.Sign));
            if (!diff.HasSymbols && c.ExactPhase)
            {
                return e;
            }
            if (l.HasUnits && r.HasUnits && Units.TryQuantity(l, out var lv, out var ldim, out var lrest) && Units.TryQuantity(r, out var rv, out var rdim, out var rrest) && lrest.IsOne && rrest.IsOne && ldim.SameAs(rdim) && lv.IsReal && rv.IsReal)
                return Bool(Compare(e.Op, lv.Re.CompareTo(rv.Re)));
            return e;
        }

        static double Magnitude(Expr e) => e is NumE n ? Math.Abs(n.V.Re.ToDouble()) : 1;

        static bool Compare(CmpOp op, int s)
        {
            switch (op)
            {
                case CmpOp.Eq: return s == 0;
                case CmpOp.Ne: return s != 0;
                case CmpOp.Lt: return s < 0;
                case CmpOp.Gt: return s > 0;
                case CmpOp.Le: return s <= 0;
                default: return s >= 0;
            }
        }

        public static Expr Bool(bool b) => b ? Expr.One : Expr.Zero;

        public static bool? Truth(Expr e)
        {
            if (e is NumE n && n.V.IsReal) return !n.V.Re.IsZero;
            return null;
        }

        Expr EvalLogic(LogicE le)
        {
            var items = le.Items.Select(Eval).ToList();
            switch (le.Op)
            {
                case LogicOp.Not:
                    {
                        var t = Truth(items[0]);
                        if (t.HasValue) return Bool(!t.Value);
                        return new LogicE(LogicOp.Not, items);
                    }
                case LogicOp.And:
                    {
                        var rest = new List<Expr>();
                        foreach (var it in items)
                        {
                            var t = Truth(it);
                            if (t == false) return Expr.Zero;
                            if (t == null) rest.Add(it);
                        }
                        if (rest.Count == 0) return Expr.One;
                        return rest.Count == 1 ? rest[0] : new LogicE(LogicOp.And, rest);
                    }
                case LogicOp.Or:
                    {
                        var rest = new List<Expr>();
                        foreach (var it in items)
                        {
                            var t = Truth(it);
                            if (t == true) return Expr.One;
                            if (t == null) rest.Add(it);
                        }
                        if (rest.Count == 0) return Expr.Zero;
                        return rest.Count == 1 ? rest[0] : new LogicE(LogicOp.Or, rest);
                    }
                default:
                    {
                        var ts = items.Select(Truth).ToList();
                        if (ts.All(x => x.HasValue)) return Bool(ts.Count(x => x.Value) % 2 == 1);
                        return new LogicE(LogicOp.Xor, items);
                    }
            }
        }

        // evaluates e with symbol bound to value, numerically
        public Expr EvalWith(Expr e, string symbol, Expr value)
        {
            Bindings.TryGetValue(symbol, out var old);
            bool had = Bindings.ContainsKey(symbol);
            Bindings[symbol] = value;
            try { return Eval(e); }
            finally
            {
                if (had) Bindings[symbol] = old; else Bindings.Remove(symbol);
            }
        }

        public double EvalDouble(Expr e, string symbol, double x)
        {
            var save = c.ExactPhase;
            c.ExactPhase = false;
            try
            {
                var r = EvalWith(e, symbol, new NumE(new Cx(Real.FromDouble(x))));
                r = Units.TryQuantity(r, out var v, out _, out var rest) && rest.IsOne ? new NumE(v) : r;
                if (r is NumE n && n.V.IsReal) return n.V.Re.ToDouble();
                if (r is NumE nc && Math.Abs(nc.V.Im.ToDouble()) < 1e-12 * Math.Max(1, Math.Abs(nc.V.Re.ToDouble()))) return nc.V.Re.ToDouble();
                return double.NaN;
            }
            catch (CalcException) { return double.NaN; }
            finally { c.ExactPhase = save; }
        }

        public System.Numerics.Complex EvalComplex(Expr e, string symbol, double x)
        {
            var save = c.ExactPhase;
            c.ExactPhase = false;
            try
            {
                var r = EvalWith(e, symbol, new NumE(new Cx(Real.FromDouble(x))));
                if (r is NumE n) return n.V.ToComplex();
                return double.NaN;
            }
            catch (CalcException) { return double.NaN; }
            finally { c.ExactPhase = save; }
        }
    }
}
