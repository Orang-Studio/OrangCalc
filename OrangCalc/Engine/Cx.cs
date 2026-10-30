using System;
using System.Numerics;

namespace OrangCalc.Engine
{
    public sealed class Cx
    {
        public readonly Real Re, Im;

        public static readonly Cx Zero = new Cx(Real.Zero);
        public static readonly Cx One = new Cx(Real.One);
        public static readonly Cx I = new Cx(Real.Zero, Real.One);

        public Cx(Real re) { Re = re; Im = Real.Zero; }
        public Cx(Real re, Real im) { Re = re; Im = im; }

        public static Cx Int(long v) => new Cx(Real.Int(v));
        public static implicit operator Cx(Real r) => new Cx(r);

        public bool IsReal => Im.IsZero;
        public bool IsZero => Re.IsZero && Im.IsZero;
        public bool IsOne => Re.IsOne && Im.IsZero;
        public bool Approx => Re.Approx || Im.Approx;
        public bool IsInt => IsReal && Re.IsInt;
        public bool IsExactInt => IsReal && Re.IsExactInt;
        public bool IsRational => IsReal && !Re.Approx && !Re.IsInfinite;
        public bool IsInfinite => Re.IsInfinite || Im.IsInfinite;

        public Cx AsApprox() => new Cx(Re.AsApprox(), Im.IsZero ? Im : Im.AsApprox());

        public Cx Clean()
        {
            if (!Im.IsZero && Im.Approx && !Re.IsZero)
            {
                double r = Math.Abs(Re.ToDouble()), i = Math.Abs(Im.ToDouble());
                if (i < r * Math.Pow(2, -(Real.Bits - 12))) return new Cx(Re);
            }
            if (!Re.IsZero && Re.Approx && !Im.IsZero)
            {
                double r = Math.Abs(Re.ToDouble()), i = Math.Abs(Im.ToDouble());
                if (r < i * Math.Pow(2, -(Real.Bits - 12))) return new Cx(Real.Zero, Im);
            }
            return this;
        }

        public static Cx operator +(Cx a, Cx b) => new Cx(a.Re + b.Re, a.Im + b.Im);
        public static Cx operator -(Cx a, Cx b) => new Cx(a.Re - b.Re, a.Im - b.Im);
        public static Cx operator -(Cx a) => new Cx(-a.Re, -a.Im);

        public static Cx operator *(Cx a, Cx b)
        {
            if (a.IsReal && b.IsReal) return new Cx(a.Re * b.Re);
            if (b.IsReal) return new Cx(a.Re * b.Re, a.Im * b.Re);
            if (a.IsReal) return new Cx(a.Re * b.Re, a.Re * b.Im);
            return new Cx(a.Re * b.Re - a.Im * b.Im, a.Re * b.Im + a.Im * b.Re).Clean();
        }

        public static Cx operator /(Cx a, Cx b)
        {
            if (b.IsReal) return new Cx(a.Re / b.Re, a.Im / b.Re);
            var den = b.Re * b.Re + b.Im * b.Im;
            if (den.IsZero) throw new CalcException("Division by zero");
            return new Cx((a.Re * b.Re + a.Im * b.Im) / den, (a.Im * b.Re - a.Re * b.Im) / den).Clean();
        }

        public bool EqualsValue(Cx o) => Re.EqualsValue(o.Re) && Im.EqualsValue(o.Im);

        public Cx Conj() => new Cx(Re, -Im);

        public Real Abs()
        {
            if (IsReal) return Re.Abs();
            if (Re.IsZero) return Im.Abs();
            var s = Re * Re + Im * Im;
            return RMath.Sqrt(s);
        }

        public Real Arg()
        {
            if (IsZero) return Real.Zero;
            return RMath.Atan2(Im, Re);
        }

        public Cx PowInt(BigInteger e)
        {
            if (IsReal) return new Cx(Re.PowInt(e));
            if (e.Sign < 0) return One / PowInt(-e);
            Cx result = One, b = this;
            while (!e.IsZero)
            {
                if (!e.IsEven) result *= b;
                e >>= 1;
                if (!e.IsZero) b *= b;
            }
            return result;
        }

        public static Cx Exp(Cx z)
        {
            if (z.IsReal) return new Cx(RMath.Exp(z.Re));
            var ea = RMath.Exp(z.Re);
            return new Cx(ea * RMath.Cos(z.Im), ea * RMath.Sin(z.Im)).Clean();
        }

        public static Cx Ln(Cx z)
        {
            if (z.IsZero) throw new CalcException("Logarithm of zero");
            if (z.IsReal && z.Re.Sign > 0) return new Cx(RMath.Ln(z.Re));
            if (z.IsReal) return new Cx(RMath.Ln(-z.Re), RMath.Pi());
            return new Cx(RMath.Ln(z.Abs()), z.Arg());
        }

        public static Cx Sqrt(Cx z)
        {
            if (z.IsReal)
            {
                if (z.Re.Sign >= 0) return new Cx(RMath.Sqrt(z.Re));
                return new Cx(Real.Zero, RMath.Sqrt(-z.Re));
            }
            var m = z.Abs();
            var re = RMath.Sqrt((m + z.Re) / Real.Two);
            var im = RMath.Sqrt((m - z.Re) / Real.Two);
            if (z.Im.Sign < 0) im = -im;
            return new Cx(re, im);
        }

        public static Cx Pow(Cx z, Cx w)
        {
            if (w.IsExactInt) return z.PowInt(w.Re.N);
            if (z.IsZero)
            {
                if (w.Re.Sign > 0) return Zero;
                throw new CalcException("Division by zero");
            }
            if (z.IsReal && w.IsReal)
            {
                if (z.Re.Sign > 0) return new Cx(RMath.Pow(z.Re, w.Re));
                if (!w.Re.Approx && !w.Re.D.IsEven && w.Re.D < 1000)
                {
                    var root = (-z.Re).ExactRoot((int)w.Re.D);
                    if (root != null && Calculator.Current != null && Calculator.Current.Opts.RealRoots)
                        return new Cx((-root).PowInt(w.Re.N));
                }
                if (w.Re.EqualsValue(Real.Half)) return Sqrt(z);
            }
            return Exp(w * Ln(z)).Clean();
        }

        public static Cx Sin(Cx z)
        {
            if (z.IsReal) return new Cx(RMath.Sin(z.Re));
            return new Cx(RMath.Sin(z.Re) * RMath.Cosh(z.Im), RMath.Cos(z.Re) * RMath.Sinh(z.Im)).Clean();
        }

        public static Cx Cos(Cx z)
        {
            if (z.IsReal) return new Cx(RMath.Cos(z.Re));
            return new Cx(RMath.Cos(z.Re) * RMath.Cosh(z.Im), -(RMath.Sin(z.Re) * RMath.Sinh(z.Im))).Clean();
        }

        public static Cx Tan(Cx z)
        {
            if (z.IsReal) return new Cx(RMath.Tan(z.Re));
            return Sin(z) / Cos(z);
        }

        public static Cx Sinh(Cx z) => z.IsReal ? new Cx(RMath.Sinh(z.Re)) : (Exp(z) - Exp(-z)) / Int(2);
        public static Cx Cosh(Cx z) => z.IsReal ? new Cx(RMath.Cosh(z.Re)) : (Exp(z) + Exp(-z)) / Int(2);
        public static Cx Tanh(Cx z) => z.IsReal ? new Cx(RMath.Tanh(z.Re)) : Sinh(z) / Cosh(z);

        public static Cx Asin(Cx z)
        {
            if (z.IsReal && z.Re.Abs() <= Real.One) return new Cx(RMath.Asin(z.Re));
            var iz = I * z;
            return (-I * Ln(iz + Sqrt(One - z * z))).Clean();
        }

        public static Cx Acos(Cx z)
        {
            if (z.IsReal && z.Re.Abs() <= Real.One) return new Cx(RMath.Acos(z.Re));
            return (new Cx(RMath.Pi() / Real.Two) - Asin(z)).Clean();
        }

        public static Cx Atan(Cx z)
        {
            if (z.IsReal) return new Cx(RMath.Atan(z.Re));
            if (z.Re.IsZero && z.Im.Abs().EqualsValue(Real.One)) throw new CalcException("atan() is undefined here");
            var iz = I * z;
            return (I / Int(2) * (Ln(One - iz) - Ln(One + iz))).Clean();
        }

        public static Cx Asinh(Cx z)
        {
            if (z.IsReal) return new Cx(RMath.Asinh(z.Re));
            return Ln(z + Sqrt(z * z + One)).Clean();
        }

        public static Cx Acosh(Cx z)
        {
            if (z.IsReal && z.Re >= Real.One) return new Cx(RMath.Acosh(z.Re));
            return Ln(z + Sqrt(z + One) * Sqrt(z - One)).Clean();
        }

        public static Cx Atanh(Cx z)
        {
            if (z.IsReal && z.Re.Abs() < Real.One) return new Cx(RMath.Atanh(z.Re));
            if (z.IsReal && z.Re.Abs().EqualsValue(Real.One)) throw new CalcException("atanh() is undefined here");
            return (Ln((One + z) / (One - z)) / Int(2)).Clean();
        }

        public System.Numerics.Complex ToComplex() => new System.Numerics.Complex(Re.ToDouble(), Im.ToDouble());
        public static Cx FromComplex(System.Numerics.Complex c) => new Cx(Real.FromDouble(c.Real), c.Imaginary == 0 ? Real.Zero : Real.FromDouble(c.Imaginary));
    }
}
