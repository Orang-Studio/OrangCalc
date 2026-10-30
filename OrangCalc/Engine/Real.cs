using System;
using System.Collections.Generic;
using System.Globalization;
using System.Numerics;

namespace OrangCalc.Engine
{
    public class CalcException : Exception
    {
        public CalcException(string msg) : base(msg) { }
    }

    // a real number: exact rational N/D, or an approximate dyadic rational when Approx is set
    public sealed class Real : IComparable<Real>
    {
        public readonly BigInteger N, D;
        public readonly bool Approx;
        public readonly int Inf;

        public static int Bits = 96;
        public static int MaxExactBits = 200000;

        public static readonly Real Zero = new Real(0, 1, false);
        public static readonly Real One = new Real(1, 1, false);
        public static readonly Real Two = new Real(2, 1, false);
        public static readonly Real MinusOne = new Real(-1, 1, false);
        public static readonly Real Half = new Real(1, 2, false);
        public static readonly Real PosInf = new Real(1);
        public static readonly Real NegInf = new Real(-1);

        Real(int inf) { N = inf; D = 0; Inf = inf; Approx = false; }

        Real(BigInteger n, BigInteger d, bool approx)
        {
            N = n; D = d; Approx = approx;
        }

        public static Real Make(BigInteger n, BigInteger d, bool approx = false)
        {
            if (d.IsZero) throw new CalcException("Division by zero");
            if (d.Sign < 0) { n = -n; d = -d; }
            if (approx) return RoundTo(n, d, Bits);
            var g = BigInteger.GreatestCommonDivisor(n, d);
            if (!g.IsOne && !g.IsZero) { n /= g; d /= g; }
            if (n.IsZero) d = 1;
            return new Real(n, d, false);
        }

        public static Real Int(BigInteger n) => new Real(n, 1, false);
        public static Real Int(long n) => new Real(n, 1, false);
        public static Real Frac(long n, long d) => Make(n, d);

        public Real AsApprox() => Approx || Inf != 0 ? this : RoundTo(N, D, Bits);

        static Real RoundTo(BigInteger n, BigInteger d, int bits)
        {
            if (n.IsZero) return new Real(0, 1, true);
            int sh = bits - (BitLen(BigInteger.Abs(n)) - BitLen(d));
            BigInteger m;
            if (sh >= 0) m = DivRound(n << sh, d);
            else m = DivRound(n, d << -sh);
            if (sh > 0)
            {
                int tz = TrailingZeros(m);
                int cut = Math.Min(tz, sh);
                m >>= cut; sh -= cut;
                return new Real(m, BigInteger.One << sh, true);
            }
            return new Real(m << -sh, 1, true);
        }

        internal static int TrailingZeros(BigInteger m)
        {
            if (m.IsZero) return 0;
            int c = 0;
            m = BigInteger.Abs(m);
            while (m.IsEven) { m >>= 1; c++; if (c > 100000) break; }
            return c;
        }

        internal static BigInteger DivRound(BigInteger a, BigInteger b)
        {
            var q = BigInteger.DivRem(a, b, out var r);
            if (BigInteger.Abs(r) * 2 >= BigInteger.Abs(b))
                q += (a.Sign * b.Sign);
            return q;
        }

        internal static int BitLen(BigInteger v)
        {
            if (v.Sign < 0) v = -v;
            if (v.IsZero) return 0;
            byte[] b = v.ToByteArray();
            int top = b.Length - 1;
            while (top > 0 && b[top] == 0) top--;
            int bits = top * 8;
            byte t = b[top];
            while (t != 0) { bits++; t >>= 1; }
            return bits;
        }

        public bool IsZero => Inf == 0 && N.IsZero;
        public bool IsInt => Inf == 0 && D.IsOne;
        public bool IsExactInt => !Approx && IsInt;
        public bool IsOne => Inf == 0 && N.IsOne && D.IsOne;
        public bool IsInfinite => Inf != 0;
        public int Sign => Inf != 0 ? Inf : N.Sign;
        public bool IsNegative => Sign < 0;

        public static Real FromDouble(double v)
        {
            if (double.IsNaN(v)) throw new CalcException("Undefined result");
            if (double.IsPositiveInfinity(v)) return PosInf;
            if (double.IsNegativeInfinity(v)) return NegInf;
            if (v == 0) return new Real(0, 1, true);
            long bitsv = BitConverter.DoubleToInt64Bits(v);
            bool neg = bitsv < 0;
            int exp = (int)((bitsv >> 52) & 0x7FF);
            long man = bitsv & 0xFFFFFFFFFFFFFL;
            if (exp == 0) exp++; else man |= 1L << 52;
            exp -= 1075;
            BigInteger n = man;
            if (neg) n = -n;
            if (exp >= 0) return Make(n << exp, 1, true);
            return Make(n, BigInteger.One << -exp, true);
        }

        public double ToDouble()
        {
            if (Inf != 0) return Inf > 0 ? double.PositiveInfinity : double.NegativeInfinity;
            if (N.IsZero) return 0;
            int s = 62 - (BitLen(N) - BitLen(D));
            BigInteger q = s >= 0 ? (N << s) / D : N / (D << -s);
            double r = (double)q;
            int ex = -s;
            while (ex > 1000) { r *= Math.Pow(2, 1000); ex -= 1000; }
            while (ex < -1000) { r *= Math.Pow(2, -1000); ex += 1000; }
            return r * Math.Pow(2, ex);
        }

        public long ToLong()
        {
            var t = Trunc();
            if (t.N > long.MaxValue || t.N < long.MinValue) throw new CalcException("Integer too large");
            return (long)t.N;
        }

        public int ToInt()
        {
            var t = Trunc();
            if (t.N > int.MaxValue || t.N < int.MinValue) throw new CalcException("Integer too large");
            return (int)t.N;
        }

        public BigInteger ToBigInt()
        {
            if (Inf != 0) throw new CalcException("Infinite value");
            return Trunc().N;
        }

        static void CheckSize(Real r)
        {
            if (!r.Approx && (BitLen(r.N) > MaxExactBits || BitLen(r.D) > MaxExactBits))
                throw new CalcException("Number too large");
        }

        public static Real operator +(Real a, Real b)
        {
            if (a.Inf != 0 || b.Inf != 0)
            {
                if (a.Inf != 0 && b.Inf != 0 && a.Inf != b.Inf) throw new CalcException("Undefined result (∞ − ∞)");
                return a.Inf != 0 ? a : b;
            }
            bool ap = a.Approx || b.Approx;
            if (a.D == b.D && !ap) return Make(a.N + b.N, a.D);
            var r = Make(a.N * b.D + b.N * a.D, a.D * b.D, ap);
            CheckSize(r);
            return r;
        }

        public static Real operator -(Real a) => a.Inf != 0 ? (a.Inf > 0 ? NegInf : PosInf) : new Real(-a.N, a.D, a.Approx);
        public static Real operator -(Real a, Real b) => a + (-b);

        public static Real operator *(Real a, Real b)
        {
            if (a.Inf != 0 || b.Inf != 0)
            {
                if (a.IsZero || b.IsZero) throw new CalcException("Undefined result (0 × ∞)");
                return a.Sign * b.Sign > 0 ? PosInf : NegInf;
            }
            var r = Make(a.N * b.N, a.D * b.D, a.Approx || b.Approx);
            CheckSize(r);
            return r;
        }

        public static Real operator /(Real a, Real b)
        {
            if (b.IsZero) throw new CalcException("Division by zero");
            if (b.Inf != 0)
            {
                if (a.Inf != 0) throw new CalcException("Undefined result (∞ / ∞)");
                return new Real(0, 1, a.Approx);
            }
            if (a.Inf != 0) return a.Sign * b.Sign > 0 ? PosInf : NegInf;
            var r = Make(a.N * b.D, a.D * b.N, a.Approx || b.Approx);
            CheckSize(r);
            return r;
        }

        public static implicit operator Real(long v) => Int(v);

        public int CompareTo(Real o)
        {
            if (Inf != 0 && o.Inf != 0) return Inf.CompareTo(o.Inf);
            if (Inf != 0) return Inf;
            if (o.Inf != 0) return -o.Inf;
            return (N * o.D).CompareTo(o.N * D);
        }

        public static bool operator <(Real a, Real b) => a.CompareTo(b) < 0;
        public static bool operator >(Real a, Real b) => a.CompareTo(b) > 0;
        public static bool operator <=(Real a, Real b) => a.CompareTo(b) <= 0;
        public static bool operator >=(Real a, Real b) => a.CompareTo(b) >= 0;
        public bool EqualsValue(Real o) => CompareTo(o) == 0;

        public Real Abs() => Sign < 0 ? -this : this;

        public Real Floor()
        {
            if (Inf != 0 || D.IsOne) return this;
            var q = BigInteger.Divide(N, D);
            if (N.Sign < 0) q -= 1;
            return new Real(q, 1, Approx);
        }

        public Real Ceil() => -((-this).Floor());
        public Real Trunc() => Sign < 0 ? Ceil() : Floor();

        public Real Round(RoundingMode mode = RoundingMode.HalfAwayFromZero)
        {
            if (Inf != 0 || D.IsOne) return this;
            var fl = Floor();
            var fr = this - fl;
            int c = fr.CompareTo(Half);
            switch (mode)
            {
                case RoundingMode.Up: return Ceil();
                case RoundingMode.Down: return Floor();
                case RoundingMode.TowardZero: return Trunc();
                case RoundingMode.AwayFromZero: return Sign < 0 ? Floor() : Ceil();
            }
            if (c < 0) return fl;
            if (c > 0) return fl + One;
            switch (mode)
            {
                case RoundingMode.HalfEven: return fl.N.IsEven ? fl : fl + One;
                case RoundingMode.HalfOdd: return !fl.N.IsEven ? fl : fl + One;
                case RoundingMode.HalfUp: return fl + One;
                case RoundingMode.HalfDown: return fl;
                case RoundingMode.HalfTowardZero: return Sign < 0 ? fl + One : fl;
                case RoundingMode.HalfRandom: return Rng.Next(2) == 0 ? fl : fl + One;
                default: return Sign < 0 ? fl : fl + One;
            }
        }

        public static readonly Random Rng = new Random();

        public Real Frac() => this - Trunc();

        public Real PowInt(BigInteger e)
        {
            if (e.IsZero)
            {
                if (IsZero) return One;
                return Approx ? One.AsApprox() : One;
            }
            if (Inf != 0)
            {
                if (e.Sign < 0) return Zero;
                return (Inf < 0 && !e.IsEven) ? NegInf : PosInf;
            }
            if (e.Sign < 0)
            {
                if (IsZero) throw new CalcException("Division by zero");
                return One / PowInt(-e);
            }
            if (IsZero || IsOne) return this;
            if (N == -1 && D.IsOne) return e.IsEven ? One : MinusOne;
            if (!Approx)
            {
                double est = (double)e * Math.Max(BitLen(N), BitLen(D));
                if (est > MaxExactBits) return AsApprox().PowInt(e);
                if (e > int.MaxValue) throw new CalcException("Exponent too large");
                int ei = (int)e;
                return Make(BigInteger.Pow(N, ei), BigInteger.Pow(D, ei));
            }
            Real result = One.AsApprox(), b = this;
            var ex = e;
            while (!ex.IsZero)
            {
                if (!ex.IsEven) result *= b;
                ex >>= 1;
                if (!ex.IsZero) b *= b;
            }
            return result;
        }

        public override string ToString() => ToDouble().ToString("R", CultureInfo.InvariantCulture);

        public static Real Min(Real a, Real b) => a <= b ? a : b;
        public static Real Max(Real a, Real b) => a >= b ? a : b;

        public static Real Parse(string digits, int radix = 10)
        {
            BigInteger n = 0, d = 1;
            bool frac = false;
            foreach (char ch in digits)
            {
                if (ch == '.') { frac = true; continue; }
                int v = DigitValue(ch);
                if (v < 0 || v >= radix) throw new CalcException("Invalid digit '" + ch + "' for base " + radix);
                n = n * radix + v;
                if (frac) d *= radix;
            }
            return Make(n, d);
        }

        public static int DigitValue(char c)
        {
            if (c >= '0' && c <= '9') return c - '0';
            if (c >= 'a' && c <= 'z') return c - 'a' + 10;
            if (c >= 'A' && c <= 'Z') return c - 'A' + 10;
            if (c == '↊' || c == 'X') return 10;
            if (c == '↋') return 11;
            return -1;
        }

        public static BigInteger ISqrt(BigInteger n)
        {
            if (n.Sign < 0) throw new CalcException("Negative square root");
            if (n < 2) return n;
            int bl = BitLen(n);
            BigInteger x = BigInteger.One << ((bl + 1) / 2);
            while (true)
            {
                var y = (x + n / x) >> 1;
                if (y >= x) return x;
                x = y;
            }
        }

        public static BigInteger IRoot(BigInteger n, int k)
        {
            if (n.Sign < 0) return -IRoot(-n, k);
            if (n < 2 || k == 1) return n;
            int bl = BitLen(n);
            BigInteger x = BigInteger.One << (bl / k + 1);
            while (true)
            {
                var y = ((k - 1) * x + n / BigInteger.Pow(x, k - 1)) / k;
                if (y >= x) break;
                x = y;
            }
            while (BigInteger.Pow(x + 1, k) <= n) x++;
            while (BigInteger.Pow(x, k) > n) x--;
            return x;
        }

        // exact nth root if the rational is a perfect power, otherwise null
        public Real ExactRoot(int k)
        {
            if (Approx || Inf != 0) return null;
            if (N.Sign < 0 && k % 2 == 0) return null;
            var rn = IRoot(N, k);
            if (BigInteger.Pow(rn, k) != N) return null;
            var rd = IRoot(D, k);
            if (BigInteger.Pow(rd, k) != D) return null;
            return Make(rn, rd);
        }
    }

    public enum RoundingMode
    {
        HalfAwayFromZero, HalfEven, HalfOdd, HalfTowardZero, HalfRandom, HalfUp, HalfDown,
        TowardZero, AwayFromZero, Up, Down
    }
}
