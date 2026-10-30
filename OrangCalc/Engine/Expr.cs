using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;

namespace OrangCalc.Engine
{
    public abstract class Expr
    {
        string key;
        public string Key => key ?? (key = BuildKey());
        protected abstract string BuildKey();
        public virtual IReadOnlyList<Expr> Kids => Array.Empty<Expr>();
        public virtual Expr Rebuild(List<Expr> kids) => this;
        public override string ToString() => Formatter.PlainText(this);

        public bool Same(Expr o) => ReferenceEquals(this, o) || Key == o.Key;

        public Expr Map(Func<Expr, Expr> f)
        {
            if (Kids.Count == 0) return f(this);
            var list = new List<Expr>(Kids.Count);
            bool changed = false;
            foreach (var k in Kids)
            {
                var m = k.Map(f);
                if (!ReferenceEquals(m, k)) changed = true;
                list.Add(m);
            }
            return f(changed ? Rebuild(list) : this);
        }

        public bool Any(Func<Expr, bool> pred)
        {
            if (pred(this)) return true;
            foreach (var k in Kids) if (k.Any(pred)) return true;
            return false;
        }

        public bool Contains(Expr sub) => Any(e => e.Same(sub));
        public bool HasUnits => Any(e => e is UnitE);
        public bool HasSymbols => Any(e => e is SymE);
        public bool IsNum => this is NumE;
        public Cx Num => ((NumE)this).V;
        public bool IsZero => this is NumE n && n.V.IsZero;
        public bool IsOne => this is NumE n && n.V.IsOne;
        public bool IsMinusOne => this is NumE n && n.V.IsReal && n.V.Re.EqualsValue(Real.MinusOne);
        public bool IsRealNum => this is NumE n && n.V.IsReal;
        public bool IsInteger => this is NumE n && n.V.IsInt;
        public bool IsApprox => Any(e => e is NumE n && n.V.Approx || e is IntvE);

        public static Expr N(long v) => new NumE(Cx.Int(v));
        public static Expr N(Real v) => new NumE(new Cx(v));
        public static Expr N(Cx v) => new NumE(v);
        public static readonly Expr Zero = N(0), One = N(1), MinusOne = N(-1), Two = N(2), Half = N(Real.Half);
    }

    public sealed class NumE : Expr
    {
        public readonly Cx V;
        public NumE(Cx v) { V = v; }
        protected override string BuildKey()
        {
            if (V.IsReal) return "#" + V.Re.N + "/" + V.Re.D + (V.Re.Inf != 0 ? "inf" + V.Re.Inf : "") + (V.Re.Approx ? "~" : "");
            return "#" + V.Re.N + "/" + V.Re.D + "i" + V.Im.N + "/" + V.Im.D + (V.Approx ? "~" : "");
        }
    }

    public sealed class SymE : Expr
    {
        public readonly string Name;
        public SymE(string name) { Name = name; }
        protected override string BuildKey() => "s:" + Name;
    }

    public sealed class VarE : Expr
    {
        public readonly Variable V;
        public VarE(Variable v) { V = v; }
        protected override string BuildKey() => "v:" + V.Name;
    }

    public sealed class UnitE : Expr
    {
        public readonly Unit U;
        public readonly Prefix P;
        public UnitE(Unit u, Prefix p = null) { U = u; P = p; }
        protected override string BuildKey() => "u:" + (P != null ? P.Name + "|" : "") + U.Name;
        public Real Factor => P == null ? U.Factor : U.Factor * P.Value;
    }

    public sealed class AddE : Expr
    {
        public readonly List<Expr> Terms;
        public AddE(List<Expr> t) { Terms = t; }
        public override IReadOnlyList<Expr> Kids => Terms;
        public override Expr Rebuild(List<Expr> kids) => new AddE(kids);
        protected override string BuildKey() => "+(" + string.Join(",", Terms.Select(t => t.Key)) + ")";
    }

    public sealed class MulE : Expr
    {
        public readonly List<Expr> Factors;
        public MulE(List<Expr> f) { Factors = f; }
        public override IReadOnlyList<Expr> Kids => Factors;
        public override Expr Rebuild(List<Expr> kids) => new MulE(kids);
        protected override string BuildKey() => "*(" + string.Join(",", Factors.Select(t => t.Key)) + ")";
    }

    public sealed class PowE : Expr
    {
        public readonly Expr B, X;
        public PowE(Expr b, Expr x) { B = b; X = x; }
        public override IReadOnlyList<Expr> Kids => new[] { B, X };
        public override Expr Rebuild(List<Expr> kids) => new PowE(kids[0], kids[1]);
        protected override string BuildKey() => "^(" + B.Key + "," + X.Key + ")";
    }

    public sealed class CallE : Expr
    {
        public readonly MathFunction F;
        public readonly List<Expr> Args;
        public CallE(MathFunction f, List<Expr> a) { F = f; Args = a; }
        public CallE(MathFunction f, params Expr[] a) { F = f; Args = a.ToList(); }
        public override IReadOnlyList<Expr> Kids => Args;
        public override Expr Rebuild(List<Expr> kids) => new CallE(F, kids);
        protected override string BuildKey() => "f:" + F.Name + "(" + string.Join(",", Args.Select(t => t.Key)) + ")";
    }

    public sealed class MatE : Expr
    {
        public readonly List<List<Expr>> Rows;
        public readonly bool IsVector;
        public MatE(List<List<Expr>> rows, bool vector) { Rows = rows; IsVector = vector; }
        public static MatE Vector(List<Expr> items) => new MatE(new List<List<Expr>> { items }, true);
        public int RowCount => Rows.Count;
        public int ColCount => Rows.Count == 0 ? 0 : Rows[0].Count;
        public IEnumerable<Expr> All => Rows.SelectMany(r => r);
        public List<Expr> Items => IsVector ? Rows[0] : Rows.SelectMany(r => r).ToList();
        public override IReadOnlyList<Expr> Kids => Rows.SelectMany(r => r).ToList();
        public override Expr Rebuild(List<Expr> kids)
        {
            var rows = new List<List<Expr>>();
            int i = 0;
            foreach (var r in Rows)
            {
                var nr = new List<Expr>();
                for (int j = 0; j < r.Count; j++) nr.Add(kids[i++]);
                rows.Add(nr);
            }
            return new MatE(rows, IsVector);
        }
        protected override string BuildKey() => (IsVector ? "vec[" : "mat[") + string.Join(";", Rows.Select(r => string.Join(",", r.Select(e => e.Key)))) + "]";
    }

    public enum CmpOp { Eq, Ne, Lt, Gt, Le, Ge }

    public sealed class CmpE : Expr
    {
        public readonly CmpOp Op;
        public readonly Expr L, R;
        public CmpE(CmpOp op, Expr l, Expr r) { Op = op; L = l; R = r; }
        public override IReadOnlyList<Expr> Kids => new[] { L, R };
        public override Expr Rebuild(List<Expr> kids) => new CmpE(Op, kids[0], kids[1]);
        protected override string BuildKey() => "c" + (int)Op + "(" + L.Key + "," + R.Key + ")";
    }

    public enum LogicOp { And, Or, Xor, Not }

    public sealed class LogicE : Expr
    {
        public readonly LogicOp Op;
        public readonly List<Expr> Items;
        public LogicE(LogicOp op, List<Expr> items) { Op = op; Items = items; }
        public override IReadOnlyList<Expr> Kids => Items;
        public override Expr Rebuild(List<Expr> kids) => new LogicE(Op, kids);
        protected override string BuildKey() => "l" + (int)Op + "(" + string.Join(",", Items.Select(t => t.Key)) + ")";
    }

    public sealed class StrE : Expr
    {
        public readonly string S;
        public StrE(string s) { S = s; }
        protected override string BuildKey() => "\"" + S + "\"";
    }

    public sealed class DateE : Expr
    {
        public readonly DateTime T;
        public readonly bool HasTime;
        public readonly bool Utc;
        public DateE(DateTime t, bool hasTime, bool utc = false) { T = t; HasTime = hasTime; Utc = utc; }
        protected override string BuildKey() => "d:" + T.Ticks + (Utc ? "z" : "");
    }

    // closed interval [Lo, Hi]
    public sealed class IntvE : Expr
    {
        public readonly Real Lo, Hi;
        public IntvE(Real lo, Real hi)
        {
            if (lo > hi) { var t = lo; lo = hi; hi = t; }
            Lo = lo; Hi = hi;
        }
        public Real Mid => (Lo + Hi) / Real.Two;
        public Real Rad => (Hi - Lo) / Real.Two;
        protected override string BuildKey() => "I(" + Lo.N + "/" + Lo.D + "," + Hi.N + "/" + Hi.D + ")";
    }

    // placeholder for an argument slot in user function definitions (\x, \y ...)
    public sealed class ArgE : Expr
    {
        public readonly int Index;
        public ArgE(int i) { Index = i; }
        protected override string BuildKey() => "\\" + Index;
    }
}
