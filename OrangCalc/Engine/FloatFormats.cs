using System;
using System.Linq;
using System.Numerics;
using System.Text;

namespace OrangCalc.Engine
{
    public sealed class FloatFormat
    {
        public string Name;
        public int ExpBits, ManBits;
        public bool ExplicitInt;
        public bool Microchip;
        public int Total => 1 + ExpBits + ManBits;
        public int Bias => (1 << (ExpBits - 1)) - 1;
    }

    public static class FloatFormats
    {
        public static readonly FloatFormat Half = new FloatFormat { Name = "16-bit (half precision)", ExpBits = 5, ManBits = 10 };
        public static readonly FloatFormat Single = new FloatFormat { Name = "32-bit (single precision)", ExpBits = 8, ManBits = 23 };
        public static readonly FloatFormat Double = new FloatFormat { Name = "64-bit (double precision)", ExpBits = 11, ManBits = 52 };
        public static readonly FloatFormat Extended = new FloatFormat { Name = "80-bit (x86 extended format)", ExpBits = 15, ManBits = 64, ExplicitInt = true };
        public static readonly FloatFormat Quad = new FloatFormat { Name = "128-bit (quadruple precision)", ExpBits = 15, ManBits = 112 };
        public static readonly FloatFormat Microchip24 = new FloatFormat { Name = "Microchip 24-bit", ExpBits = 8, ManBits = 15, Microchip = true };
        public static readonly FloatFormat Microchip32 = new FloatFormat { Name = "Microchip 32-bit", ExpBits = 8, ManBits = 23, Microchip = true };
        public static readonly FloatFormat[] All = { Half, Single, Double, Extended, Quad, Microchip24, Microchip32 };

        public static FloatFormat ByBits(int bits)
        {
            switch (bits)
            {
                case 16: return Half;
                case 32: return Single;
                case 64: return Double;
                case 80: return Extended;
                case 128: return Quad;
                case 24: return Microchip24;
            }
            throw new CalcException("Unsupported floating point size (16, 24, 32, 64, 80 or 128 bits)");
        }

        static int FracBits(FloatFormat f) => f.ExplicitInt ? f.ManBits - 1 : f.ManBits;

        public static BigInteger Encode(Real x, FloatFormat f)
        {
            int fb = FracBits(f);
            int bias = f.Bias;
            int maxExp = (1 << f.ExpBits) - 1;
            bool neg = x.Sign < 0;
            BigInteger expField, frac;
            if (x.IsInfinite)
            {
                expField = maxExp; frac = f.ExplicitInt ? BigInteger.One << fb : 0;
            }
            else if (x.IsZero) { expField = 0; frac = 0; }
            else
            {
                var a = x.Abs();
                int e = Real.BitLen(a.N) - Real.BitLen(a.D);
                if (Pow2(e) > a) e--;
                if (Pow2(e + 1) <= a) e++;
                int minExp = 1 - bias;
                bool sub = e < minExp;
                if (sub) e = minExp;
                var scaled = a / Pow2(e) * Pow2(fb);
                var m = scaled.Round(RoundingMode.HalfEven).N;
                if (!sub && m >= BigInteger.One << (fb + 1)) { m >>= 1; e++; }
                if (sub && m >= BigInteger.One << fb) sub = false;
                if (e + bias >= maxExp && !f.Microchip)
                {
                    expField = maxExp; frac = f.ExplicitInt ? BigInteger.One << fb : 0;
                }
                else if (sub)
                {
                    expField = 0; frac = m;
                }
                else
                {
                    expField = e + bias;
                    frac = f.ExplicitInt ? m : m - (BigInteger.One << fb);
                }
            }
            BigInteger s = neg ? 1 : 0;
            if (f.Microchip) return (expField << (f.ManBits + 1)) | (s << f.ManBits) | frac;
            return (s << (f.ExpBits + f.ManBits)) | (expField << f.ManBits) | frac;
        }

        static Real Pow2(int e) => e >= 0 ? Real.Int(BigInteger.One << e) : Real.Make(1, BigInteger.One << -e);

        public static Real Decode(string bitsText, FloatFormat f)
        {
            var t = bitsText.Replace(" ", "").Replace("_", "");
            BigInteger v;
            if (t.StartsWith("0x", StringComparison.OrdinalIgnoreCase)) v = BigInteger.Parse("0" + t.Substring(2), System.Globalization.NumberStyles.HexNumber);
            else
            {
                if (t.Any(ch => ch != '0' && ch != '1')) throw new CalcException("Binary digits expected");
                v = 0;
                foreach (char ch in t) v = v * 2 + (ch - '0');
            }
            return Decode(v, f);
        }

        public static Real Decode(BigInteger v, FloatFormat f)
        {
            int fb = FracBits(f);
            int bias = f.Bias;
            BigInteger sign, expField, frac;
            if (f.Microchip)
            {
                frac = v & ((BigInteger.One << f.ManBits) - 1);
                sign = (v >> f.ManBits) & 1;
                expField = v >> (f.ManBits + 1);
            }
            else
            {
                frac = v & ((BigInteger.One << f.ManBits) - 1);
                expField = (v >> f.ManBits) & ((BigInteger.One << f.ExpBits) - 1);
                sign = (v >> (f.ExpBits + f.ManBits)) & 1;
            }
            int maxExp = (1 << f.ExpBits) - 1;
            if (expField == maxExp && !f.Microchip)
            {
                var fr = f.ExplicitInt ? frac & ((BigInteger.One << fb) - 1) : frac;
                if (fr.IsZero) return sign.IsOne ? Real.NegInf : Real.PosInf;
                throw new CalcException("The value is NaN (not a number)");
            }
            if (f.Microchip && expField.IsZero) return Real.Zero;
            Real val;
            if (expField.IsZero) val = Real.Int(frac) * Pow2(1 - bias - fb);
            else
            {
                var m = f.ExplicitInt ? frac : frac + (BigInteger.One << fb);
                val = Real.Int(m) * Pow2((int)expField - bias - fb);
            }
            return sign.IsOne ? -val : val;
        }

        public static string ToBits(BigInteger v, FloatFormat f)
        {
            var sb = new StringBuilder();
            for (int i = f.Total - 1; i >= 0; i--) sb.Append(((v >> i) & 1).IsZero ? '0' : '1');
            return sb.ToString();
        }

        public static string ToHex(BigInteger v, FloatFormat f)
        {
            int digits = (f.Total + 3) / 4;
            var s = v.ToString("X");
            s = s.TrimStart('0');
            return "0x" + s.PadLeft(digits, '0');
        }

        public static string BinaryString(Real r, FloatFormat f)
        {
            var bits = ToBits(Encode(r, f), f);
            if (f.Microchip) return bits.Substring(0, f.ExpBits) + " " + bits.Substring(f.ExpBits, 1) + " " + bits.Substring(f.ExpBits + 1);
            return bits.Substring(0, 1) + " " + bits.Substring(1, f.ExpBits) + " " + bits.Substring(1 + f.ExpBits);
        }
    }
}
