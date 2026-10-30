using System;
using System.Collections.Generic;
using System.Numerics;

namespace OrangCalc.Engine
{
    // arbitrary precision transcendental functions on fixed point integers (value * 2^p)
    public static class RMath
    {
        static readonly Dictionary<int, BigInteger> piCache = new Dictionary<int, BigInteger>();
        static readonly Dictionary<int, BigInteger> ln2Cache = new Dictionary<int, BigInteger>();
        static readonly object sync = new object();

        static int P => Real.Bits + 32;

        static BigInteger Fix(Real x, int p)
        {
            return p >= 0 ? Real.DivRound(x.N << p, x.D) : Real.DivRound(x.N, x.D << -p);
        }

        static Real Unfix(BigInteger v, int p)
        {
            if (p >= 0) return Real.Make(v, BigInteger.One << p, true);
            return Real.Make(v << -p, 1, true);
        }

        static BigInteger AtanInv(int n, int p)
        {
            BigInteger x = (BigInteger.One << p) / n;
            BigInteger n2 = n * n, sum = x, term = x;
            int k = 1;
            bool neg = true;
            while (!term.IsZero)
            {
                term /= n2;
                k += 2;
                if (neg) sum -= term / k; else sum += term / k;
                neg = !neg;
            }
            return sum;
        }

        public static BigInteger PiFix(int p)
        {
            lock (sync)
            {
                if (piCache.TryGetValue(p, out var v)) return v;
                int g = p + 16;
                var pi = (16 * AtanInv(5, g) - 4 * AtanInv(239, g)) >> 16;
                piCache[p] = pi;
                return pi;
            }
        }

        static BigInteger AtanhFix(BigInteger t, int p)
        {
            BigInteger t2 = (t * t) >> p, term = t, sum = t;
            int k = 1;
            while (!term.IsZero)
            {
                term = (term * t2) >> p;
                k += 2;
                sum += term / k;
            }
            return sum;
        }

        public static BigInteger Ln2Fix(int p)
        {
            lock (sync)
            {
                if (ln2Cache.TryGetValue(p, out var v)) return v;
                int g = p + 16;
                var third = (BigInteger.One << g) / 3;
                var r = (2 * AtanhFix(third, g)) >> 16;
                ln2Cache[p] = r;
                return r;
            }
        }

        public static Real Pi() => Unfix(PiFix(P), P);
        public static Real Ln2() => Unfix(Ln2Fix(P), P);
        public static Real E() => Exp(Real.One);

        static BigInteger ExpFixSmall(BigInteger r, int p)
        {
            int s = (int)Math.Sqrt(p) / 2 + 1;
            int q = p + s + 8;
            BigInteger x = r << (q - p);
            x >>= s;
            BigInteger one = BigInteger.One << q, sum = one, term = one;
            for (int k = 1; ; k++)
            {
                term = ((term * x) >> q) / k;
                if (term.IsZero) break;
                sum += term;
            }
            for (int i = 0; i < s; i++) sum = (sum * sum) >> q;
            return sum >> (q - p);
        }

        public static Real Exp(Real x)
        {
            if (x.IsInfinite) return x.Sign > 0 ? Real.PosInf : Real.Zero;
            if (x.IsZero) return Real.One;
            double xd = x.ToDouble();
            if (xd > 1e6) throw new CalcException("Overflow in exp()");
            if (xd < -1e6) return Real.Zero.AsApprox();
            long k = (long)Math.Round(xd / 0.6931471805599453);
            int p = P + Real.BitLen(k) + 4;
            BigInteger X = Fix(x, p) - k * Ln2Fix(p);
            BigInteger e = ExpFixSmall(X, p);
            return Unfix(e, (int)(p - k));
        }

        public static Real Ln(Real x)
        {
            if (x.Sign <= 0) throw new CalcException("Logarithm of non-positive number");
            if (x.IsInfinite) return Real.PosInf;
            if (x.IsOne) return Real.Zero;
            int p = P;
            int e = Real.BitLen(x.N) - Real.BitLen(x.D);
            BigInteger M = (p - e) >= 0 ? Real.DivRound(x.N << (p - e), x.D) : Real.DivRound(x.N, x.D << (e - p));
            BigInteger one = BigInteger.One << p;
            BigInteger T = ((M - one) << p) / (M + one);
            BigInteger lnm = 2 * AtanhFix(T, p);
            return Unfix(lnm + e * Ln2Fix(p), p);
        }

        static void SinCosFix(BigInteger r, int p, out BigInteger s, out BigInteger c)
        {
            BigInteger r2 = (r * r) >> p;
            BigInteger term = r; s = r;
            for (int k = 1; ; k += 2)
            {
                term = -((term * r2) >> p) / ((k + 1) * (k + 2));
                if (term.IsZero) break;
                s += term;
            }
            BigInteger one = BigInteger.One << p;
            term = one; c = one;
            for (int k = 0; ; k += 2)
            {
                term = -((term * r2) >> p) / ((k + 1) * (k + 2));
                if (term.IsZero) break;
                c += term;
            }
        }

        static void Reduce(Real x, out BigInteger r, out int quadrant, out int p)
        {
            double xd = x.ToDouble();
            if (Math.Abs(xd) > 1e300) throw new CalcException("Argument too large for trigonometric function");
            BigInteger k = new BigInteger(Math.Round(xd / 1.5707963267948966));
            p = P + Real.BitLen(k) + 8;
            BigInteger halfPi = PiFix(p) >> 1;
            r = Fix(x, p) - k * halfPi;
            if (r > halfPi / 2) { r -= halfPi; k += 1; }
            else if (r < -halfPi / 2) { r += halfPi; k -= 1; }
            quadrant = (int)(((k % 4) + 4) % 4);
        }

        public static Real Sin(Real x)
        {
            if (x.IsZero) return Real.Zero;
            if (x.IsInfinite) throw new CalcException("sin(∞) is undefined");
            Reduce(x, out var r, out int q, out int p);
            SinCosFix(r, p, out var s, out var c);
            switch (q)
            {
                case 0: return Unfix(s, p);
                case 1: return Unfix(c, p);
                case 2: return Unfix(-s, p);
                default: return Unfix(-c, p);
            }
        }

        public static Real Cos(Real x)
        {
            if (x.IsZero) return Real.One;
            if (x.IsInfinite) throw new CalcException("cos(∞) is undefined");
            Reduce(x, out var r, out int q, out int p);
            SinCosFix(r, p, out var s, out var c);
            switch (q)
            {
                case 0: return Unfix(c, p);
                case 1: return Unfix(-s, p);
                case 2: return Unfix(-c, p);
                default: return Unfix(s, p);
            }
        }

        public static Real Tan(Real x)
        {
            var c = Cos(x);
            if (c.IsZero) throw new CalcException("tan() is undefined here");
            return Sin(x) / c;
        }

        public static Real Atan(Real x)
        {
            if (x.IsZero) return Real.Zero;
            if (x.IsInfinite) return x.Sign > 0 ? Pi() / Real.Two : -(Pi() / Real.Two);
            bool neg = x.Sign < 0;
            if (neg) x = -x;
            bool inv = x > Real.One;
            if (inv) x = Real.One / x;
            int p = P + 8;
            BigInteger X = Fix(x, p), one = BigInteger.One << p;
            for (int i = 0; i < 3; i++)
            {
                BigInteger root = Real.ISqrt((one + ((X * X) >> p)) << p);
                X = (X << p) / (one + root);
            }
            BigInteger x2 = (X * X) >> p, term = X, sum = X;
            for (int k = 1; ; k++)
            {
                term = -((term * x2) >> p);
                var t = term / (2 * k + 1);
                if (t.IsZero) break;
                sum += t;
            }
            sum <<= 3;
            if (inv) sum = (PiFix(p) >> 1) - sum;
            if (neg) sum = -sum;
            return Unfix(sum, p);
        }

        public static Real Atan2(Real y, Real x)
        {
            if (x.IsZero)
            {
                if (y.IsZero) return Real.Zero;
                return y.Sign > 0 ? Pi() / Real.Two : -(Pi() / Real.Two);
            }
            var a = Atan(y / x);
            if (x.Sign > 0) return a;
            return y.Sign >= 0 ? a + Pi() : a - Pi();
        }

        public static Real Sqrt(Real x)
        {
            if (x.Sign < 0) throw new CalcException("Square root of negative number");
            if (x.IsZero) return x;
            if (x.IsInfinite) return x;
            var ex = x.ExactRoot(2);
            if (ex != null) return ex;
            int p = P;
            return Unfix(Real.ISqrt(Fix(x, 2 * p)), p);
        }

        public static Real Root(Real x, int n)
        {
            if (n == 2) return Sqrt(x);
            var ex = x.ExactRoot(n);
            if (ex != null) return ex;
            if (x.Sign < 0)
            {
                if (n % 2 == 0) throw new CalcException("Even root of negative number");
                return -Root(-x, n);
            }
            if (x.IsZero) return x;
            int p = P;
            BigInteger v = Fix(x, p * n);
            return Unfix(Real.IRoot(v, n), p);
        }

        public static Real Pow(Real x, Real y)
        {
            if (y.IsExactInt) return x.PowInt(y.N);
            if (y.IsInt && Real.BitLen(y.N) < 60) return x.AsApprox().PowInt(y.N);
            if (x.IsZero)
            {
                if (y.Sign > 0) return Real.Zero;
                throw new CalcException("Division by zero");
            }
            if (x.Sign < 0) throw new CalcException("Negative base with fractional exponent");
            if (!y.Approx && !y.D.IsOne && y.D < 64)
            {
                var r = x.ExactRoot((int)y.D);
                if (r != null) return r.PowInt(y.N);
            }
            return Exp(y * Ln(x));
        }

        public static Real Sinh(Real x) { var e = Exp(x); return (e - Real.One / e) / Real.Two; }
        public static Real Cosh(Real x) { var e = Exp(x); return (e + Real.One / e) / Real.Two; }
        public static Real Tanh(Real x)
        {
            if (x.ToDouble() > 200) return Real.One.AsApprox();
            if (x.ToDouble() < -200) return Real.MinusOne.AsApprox();
            var e2 = Exp(x * Real.Two);
            return (e2 - Real.One) / (e2 + Real.One);
        }

        public static Real Asin(Real x)
        {
            if (x.Abs() > Real.One) throw new CalcException("asin() argument out of range");
            if (x.Abs().EqualsValue(Real.One)) return x.Sign * Pi() / Real.Two;
            return Atan(x / Sqrt(Real.One - x * x));
        }

        public static Real Acos(Real x) => Pi() / Real.Two - Asin(x);
        public static Real Asinh(Real x) => x.Sign < 0 ? -Asinh(-x) : Ln(x + Sqrt(x * x + Real.One));
        public static Real Acosh(Real x) => Ln(x + Sqrt(x * x - Real.One));
        public static Real Atanh(Real x) => Ln((Real.One + x) / (Real.One - x)) / Real.Two;
        public static Real Log(Real x, Real b) => Ln(x) / Ln(b);

        public static Real Hypot(Real a, Real b) => Sqrt(a * a + b * b);
    }
}
