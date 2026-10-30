using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;

namespace OrangCalc.Engine
{
    public static class Primes
    {
        public static readonly int[] Small = Sieve(100000);

        static int[] Sieve(int n)
        {
            var comp = new bool[n + 1];
            var list = new List<int>();
            for (int i = 2; i <= n; i++)
            {
                if (comp[i]) continue;
                list.Add(i);
                for (long j = (long)i * i; j <= n; j += i) comp[j] = true;
            }
            return list.ToArray();
        }

        static readonly int[] MrBases = { 2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41 };

        public static bool IsPrime(BigInteger n)
        {
            if (n < 2) return false;
            if (n <= Small[Small.Length - 1]) return Array.BinarySearch(Small, (int)n) >= 0;
            foreach (int p in Small.Take(50)) if ((n % p).IsZero) return false;
            BigInteger d = n - 1;
            int s = 0;
            while (d.IsEven) { d >>= 1; s++; }
            foreach (int a in MrBases)
            {
                if (!MillerRabin(n, a, d, s)) return false;
            }
            if (Real.BitLen(n) > 82)
            {
                var rng = new Random(12345);
                for (int k = 0; k < 16; k++)
                {
                    var bytes = n.ToByteArray();
                    rng.NextBytes(bytes);
                    bytes[bytes.Length - 1] &= 0x7F;
                    var a = new BigInteger(bytes) % (n - 4) + 2;
                    if (!MillerRabin(n, a, d, s)) return false;
                }
            }
            return true;
        }

        static bool MillerRabin(BigInteger n, BigInteger a, BigInteger d, int s)
        {
            var x = BigInteger.ModPow(a, d, n);
            if (x.IsOne || x == n - 1) return true;
            for (int r = 1; r < s; r++)
            {
                x = BigInteger.ModPow(x, 2, n);
                if (x == n - 1) return true;
                if (x.IsOne) return false;
            }
            return false;
        }

        public static BigInteger NextPrime(BigInteger n)
        {
            if (n < 2) return 2;
            var c = n + 1;
            if (c.IsEven && c != 2) c++;
            while (!IsPrime(c)) c += 2;
            return c;
        }

        public static BigInteger PrevPrime(BigInteger n)
        {
            if (n <= 2) throw new CalcException("No prime below " + n);
            var c = n - 1;
            if (c == 2) return 2;
            if (c.IsEven) c--;
            while (c > 2 && !IsPrime(c)) c -= 2;
            return c;
        }

        public static BigInteger NthPrime(long n)
        {
            if (n < 1) throw new CalcException("nthprime() requires a positive integer");
            if (n <= Small.Length) return Small[n - 1];
            if (n > 5000000) throw new CalcException("nthprime() argument too large");
            double ln = Math.Log(n);
            int limit = (int)(n * (ln + Math.Log(ln)) + 10);
            var ps = Sieve(limit);
            return ps[n - 1];
        }

        public static long PrimeCount(BigInteger x)
        {
            if (x < 2) return 0;
            if (x > 100000000) throw new CalcException("primePi() argument too large");
            int n = (int)x;
            if (n <= Small[Small.Length - 1])
            {
                int idx = Array.BinarySearch(Small, n);
                return idx >= 0 ? idx + 1 : ~idx;
            }
            var comp = new System.Collections.BitArray(n + 1);
            long count = 0;
            for (int i = 2; i <= n; i++)
            {
                if (comp[i]) continue;
                count++;
                for (long j = (long)i * i; j <= n; j += i) comp[(int)j] = true;
            }
            return count;
        }

        public static List<BigInteger> PrimesUpTo(long n)
        {
            if (n > 10000000) throw new CalcException("Limit too large");
            return Sieve((int)Math.Max(2, n)).Where(p => p <= n).Select(p => (BigInteger)p).ToList();
        }

        public static SortedDictionary<BigInteger, int> Factorize(BigInteger n)
        {
            var res = new SortedDictionary<BigInteger, int>();
            if (n.Sign < 0) n = -n;
            if (n < 2) return res;
            foreach (int p in Small)
            {
                if ((BigInteger)p * p > n) break;
                while ((n % p).IsZero) { Inc(res, p); n /= p; }
            }
            if (n > 1) FactorRec(n, res);
            return res;
        }

        static void Inc(SortedDictionary<BigInteger, int> d, BigInteger k)
        {
            d.TryGetValue(k, out int v);
            d[k] = v + 1;
        }

        static void FactorRec(BigInteger n, SortedDictionary<BigInteger, int> res)
        {
            if (n.IsOne) return;
            if (IsPrime(n)) { Inc(res, n); return; }
            var d = Rho(n);
            if (d.IsOne || d == n) { Inc(res, n); return; }
            FactorRec(d, res);
            FactorRec(n / d, res);
        }

        static BigInteger Rho(BigInteger n)
        {
            if (n.IsEven) return 2;
            var start = DateTime.UtcNow;
            for (int c = 1; c < 50; c++)
            {
                BigInteger x = 2, y = 2, d = 1;
                long iter = 0;
                while (d.IsOne)
                {
                    x = (x * x + c) % n;
                    y = (y * y + c) % n;
                    y = (y * y + c) % n;
                    d = BigInteger.GreatestCommonDivisor(BigInteger.Abs(x - y), n);
                    if (++iter % 1000 == 0 && (DateTime.UtcNow - start).TotalSeconds > 20) throw new CalcException("Factorization timed out");
                }
                if (d != n) return d;
            }
            return n;
        }

        public static List<BigInteger> Divisors(BigInteger n)
        {
            var f = Factorize(n);
            var divs = new List<BigInteger> { 1 };
            foreach (var kv in f)
            {
                var cur = new List<BigInteger>();
                BigInteger pk = 1;
                for (int e = 0; e <= kv.Value; e++)
                {
                    foreach (var d in divs) cur.Add(d * pk);
                    pk *= kv.Key;
                }
                divs = cur;
                if (divs.Count > 100000) throw new CalcException("Too many divisors");
            }
            divs.Sort();
            return divs;
        }

        public static BigInteger Totient(BigInteger n)
        {
            if (n < 1) throw new CalcException("totient() requires a positive integer");
            BigInteger r = n;
            foreach (var p in Factorize(n).Keys) r = r / p * (p - 1);
            return r;
        }

        public static BigInteger Factorial(BigInteger n, int step = 1)
        {
            if (n.Sign < 0) throw new CalcException("Factorial of negative number");
            if (n > 100000) throw new CalcException("Factorial argument too large for exact calculation");
            int k = (int)n;
            if (step == 1) return ProductRange(1, k);
            BigInteger r = 1;
            for (int i = k; i > 1; i -= step) r *= i;
            return r;
        }

        static BigInteger ProductRange(int a, int b)
        {
            if (a > b) return 1;
            if (b - a < 16)
            {
                BigInteger r = 1;
                for (int i = a; i <= b; i++) r *= i;
                return r;
            }
            int m = (a + b) / 2;
            return ProductRange(a, m) * ProductRange(m + 1, b);
        }

        public static BigInteger Binomial(BigInteger n, BigInteger k)
        {
            if (k.Sign < 0) return 0;
            if (n.Sign >= 0 && k > n) return 0;
            if (n.Sign >= 0 && k > n - k) k = n - k;
            if (k > 1000000) throw new CalcException("binomial() argument too large");
            BigInteger r = 1;
            for (BigInteger i = 1; i <= k; i++) r = r * (n - k + i) / i;
            return r;
        }

        public static BigInteger Fibonacci(BigInteger n)
        {
            if (n.Sign < 0)
            {
                var f = Fibonacci(-n);
                return n.IsEven ? -f : f;
            }
            if (n > 10000000) throw new CalcException("fibonacci() argument too large");
            BigInteger a = 0, b = 1;
            int bits = Real.BitLen(n);
            for (int i = bits - 1; i >= 0; i--)
            {
                var d = a * (2 * b - a);
                var e = a * a + b * b;
                a = d; b = e;
                if (((n >> i) & 1) == 1) { var t = a + b; a = b; b = t; }
            }
            return a;
        }

        public static Real Bernoulli(int n)
        {
            if (n < 0) throw new CalcException("bernoulli() requires a non-negative integer");
            if (n == 1) return Real.Frac(1, 2);
            if (n > 1 && n % 2 == 1) return Real.Zero;
            if (n > 2000) throw new CalcException("bernoulli() argument too large");
            var a = new Real[n + 1];
            for (int m = 0; m <= n; m++)
            {
                a[m] = Real.Make(1, m + 1);
                for (int j = m; j >= 1; j--) a[j - 1] = Real.Int(j) * (a[j - 1] - a[j]);
            }
            return a[0];
        }

        public static BigInteger Subfactorial(BigInteger n)
        {
            if (n.Sign < 0) throw new CalcException("Negative argument");
            if (n > 50000) throw new CalcException("Argument too large");
            BigInteger a = 1, b = 0;
            if (n.IsZero) return 1;
            for (int i = 2; i <= n; i++) { var t = (i - 1) * (a + b); a = b; b = t; }
            return b;
        }

        public static BigInteger Partitions(int n)
        {
            if (n < 0) return 0;
            if (n > 20000) throw new CalcException("Argument too large");
            var p = new BigInteger[n + 1];
            p[0] = 1;
            for (int i = 1; i <= n; i++)
            {
                BigInteger s = 0;
                for (int k = 1; ; k++)
                {
                    int g1 = k * (3 * k - 1) / 2, g2 = k * (3 * k + 1) / 2;
                    if (g1 > i) break;
                    int sign = (k % 2 == 1) ? 1 : -1;
                    s += sign * p[i - g1];
                    if (g2 <= i) s += sign * p[i - g2];
                }
                p[i] = s;
            }
            return p[n];
        }

        public static BigInteger Stirling2(int n, int k)
        {
            if (k > n) return 0;
            BigInteger sum = 0;
            for (int j = 0; j <= k; j++)
            {
                var term = Binomial(k, j) * BigInteger.Pow(j, n);
                sum += ((k - j) % 2 == 0) ? term : -term;
            }
            return sum / Factorial(k);
        }

        public static BigInteger ModInverse(BigInteger a, BigInteger m)
        {
            BigInteger g = m, x = 0, x1 = 1, a1 = ((a % m) + m) % m;
            BigInteger b = a1;
            BigInteger old_r = b, r = m, old_s = 1, s = 0;
            while (!r.IsZero)
            {
                var q = old_r / r;
                (old_r, r) = (r, old_r - q * r);
                (old_s, s) = (s, old_s - q * s);
            }
            if (!old_r.IsOne) throw new CalcException("No modular inverse exists");
            return ((old_s % m) + m) % m;
        }
    }
}
