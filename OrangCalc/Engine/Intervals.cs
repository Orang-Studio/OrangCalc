using System;
using System.Collections.Generic;
using System.Linq;

namespace OrangCalc.Engine
{
    public static class Intervals
    {
        static IntervalCalc Mode => Calculator.Current?.Opts.Interval ?? IntervalCalc.VarianceFormula;

        public static bool Get(Expr e, out Real lo, out Real hi)
        {
            if (e is IntvE i) { lo = i.Lo; hi = i.Hi; return true; }
            if (e is NumE n && n.V.IsReal) { lo = hi = n.V.Re; return true; }
            lo = hi = null;
            return false;
        }

        static Expr Make(Real lo, Real hi)
        {
            if (lo.EqualsValue(hi)) return new NumE(new Cx(lo));
            return new IntvE(lo, hi);
        }

        public static Expr FromMidRad(Real mid, Real rad) => Make(mid - rad.Abs(), mid + rad.Abs());

        public static Expr Add(Expr a, Expr b)
        {
            if (!Get(a, out var al, out var ah) || !Get(b, out var bl, out var bh)) throw new CalcException("Invalid interval operation");
            if (Mode == IntervalCalc.VarianceFormula && a is IntvE ia && b is IntvE ib)
            {
                var rad = RMath.Sqrt(ia.Rad * ia.Rad + ib.Rad * ib.Rad);
                return FromMidRad(ia.Mid + ib.Mid, rad);
            }
            return Make(al + bl, ah + bh);
        }

        public static Expr Mul(Expr a, Expr b)
        {
            if (!Get(a, out var al, out var ah) || !Get(b, out var bl, out var bh)) throw new CalcException("Invalid interval operation");
            if (Mode == IntervalCalc.VarianceFormula && a is IntvE ia && b is IntvE ib && !ia.Mid.IsZero && !ib.Mid.IsZero)
            {
                var m = ia.Mid * ib.Mid;
                var ra = ia.Rad / ia.Mid; var rb = ib.Rad / ib.Mid;
                var rel = RMath.Sqrt(ra * ra + rb * rb);
                return FromMidRad(m, (m * rel).Abs());
            }
            if (Mode == IntervalCalc.Simple && a is IntvE sa && b is IntvE sb)
            {
                var m = sa.Mid * sb.Mid;
                var rad = (sa.Rad * sb.Mid).Abs() + (sb.Rad * sa.Mid).Abs();
                return FromMidRad(m, rad);
            }
            var c = new[] { al * bl, al * bh, ah * bl, ah * bh };
            return Make(c.Aggregate(Real.Min), c.Aggregate(Real.Max));
        }

        public static Expr Inverse(Expr a)
        {
            Get(a, out var lo, out var hi);
            if (lo.Sign <= 0 && hi.Sign >= 0) throw new CalcException("Division by an interval containing zero");
            if (Mode == IntervalCalc.VarianceFormula && a is IntvE ia)
            {
                var m = Real.One / ia.Mid;
                return FromMidRad(m, (ia.Rad / (ia.Mid * ia.Mid)).Abs());
            }
            return Make(Real.One / hi, Real.One / lo);
        }

        public static Expr Pow(Expr b, Expr e)
        {
            if (e is NumE en && en.V.IsReal)
            {
                var x = en.V.Re;
                if (x.EqualsValue(Real.MinusOne)) return Inverse(b);
                if (x.Sign < 0) return Inverse(Pow(b, new NumE(new Cx(-x))));
                Get(b, out var lo, out var hi);
                if (Mode == IntervalCalc.VarianceFormula && b is IntvE ib && !ib.Mid.IsZero)
                {
                    var m = RMath.Pow(ib.Mid.Abs(), x);
                    if (ib.Mid.Sign < 0 && x.IsExactInt && !x.N.IsEven) m = -m;
                    var rad = (m * x * ib.Rad / ib.Mid).Abs();
                    return FromMidRad(m, rad);
                }
                if (x.IsExactInt && x.N.IsEven && lo.Sign < 0 && hi.Sign > 0)
                    return Make(Real.Zero, Real.Max(lo.PowInt(x.N), hi.PowInt(x.N)));
                var a1 = RMath.Pow(lo, x); var a2 = RMath.Pow(hi, x);
                return Make(Real.Min(a1, a2), Real.Max(a1, a2));
            }
            if (Get(b, out var bl, out var bh) && Get(e, out var el, out var eh))
            {
                var c = new[] { RMath.Pow(bl, el), RMath.Pow(bl, eh), RMath.Pow(bh, el), RMath.Pow(bh, eh) };
                return Make(c.Aggregate(Real.Min), c.Aggregate(Real.Max));
            }
            throw new CalcException("Unsupported interval power");
        }

        // applies a real function to an interval by sampling endpoints and interior points
        public static Expr Apply(IntvE iv, Func<Real, Real> f)
        {
            if (Mode == IntervalCalc.VarianceFormula)
            {
                var m = f(iv.Mid);
                var h = iv.Rad / Real.Int(1000);
                if (h.IsZero) return new NumE(new Cx(m));
                var deriv = (f(iv.Mid + h) - f(iv.Mid - h)) / (h * Real.Two);
                return FromMidRad(m, (deriv * iv.Rad).Abs());
            }
            Real lo = null, hi = null;
            for (int k = 0; k <= 16; k++)
            {
                var x = iv.Lo + (iv.Hi - iv.Lo) * Real.Make(k, 16);
                var y = f(x);
                lo = lo == null ? y : Real.Min(lo, y);
                hi = hi == null ? y : Real.Max(hi, y);
            }
            return Make(lo, hi);
        }
    }
}
