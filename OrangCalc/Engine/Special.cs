using System;
using System.Numerics;

namespace OrangCalc.Engine
{
    public static class Special
    {
        static readonly double[] Lanczos =
        {
            0.99999999999980993, 676.5203681218851, -1259.1392167224028, 771.32342877765313,
            -176.61502916214059, 12.507343278686905, -0.13857109526572012, 9.9843695780195716e-6, 1.5056327351493116e-7
        };

        public static Complex Gamma(Complex z)
        {
            if (z.Real < 0.5) return Math.PI / (Complex.Sin(Math.PI * z) * Gamma(1 - z));
            z -= 1;
            Complex x = Lanczos[0];
            for (int i = 1; i < 9; i++) x += Lanczos[i] / (z + i);
            var t = z + 7.5;
            return Math.Sqrt(2 * Math.PI) * Complex.Pow(t, z + 0.5) * Complex.Exp(-t) * x;
        }

        public static double Gamma(double x)
        {
            if (x == Math.Floor(x) && x <= 0) throw new CalcException("gamma() is undefined for non-positive integers");
            if (x > 171.6) return double.PositiveInfinity;
            return Gamma(new Complex(x, 0)).Real;
        }

        public static double LogGamma(double x)
        {
            if (x <= 0 && x == Math.Floor(x)) throw new CalcException("lgamma() is undefined for non-positive integers");
            if (x < 0.5) return Math.Log(Math.PI / Math.Abs(Math.Sin(Math.PI * x))) - LogGamma(1 - x);
            x -= 1;
            double a = Lanczos[0];
            double t = x + 7.5;
            for (int i = 1; i < 9; i++) a += Lanczos[i] / (x + i);
            return 0.5 * Math.Log(2 * Math.PI) + (x + 0.5) * Math.Log(t) - t + Math.Log(a);
        }

        public static double Digamma(double x)
        {
            if (x <= 0 && x == Math.Floor(x)) throw new CalcException("digamma() is undefined for non-positive integers");
            double r = 0;
            if (x < 0) return Digamma(1 - x) - Math.PI / Math.Tan(Math.PI * x);
            while (x < 6) { r -= 1 / x; x += 1; }
            double f = 1 / (x * x);
            return r + Math.Log(x) - 0.5 / x - f * (1.0 / 12 - f * (1.0 / 120 - f * (1.0 / 252 - f * (1.0 / 240 - f * (1.0 / 132)))));
        }

        public static double Erf(double x)
        {
            if (x < 0) return -Erf(-x);
            if (x > 6) return 1;
            if (x < 2.5)
            {
                double sum = x, term = x, x2 = x * x;
                for (int n = 1; n < 200; n++)
                {
                    term *= -x2 / n;
                    double add = term / (2 * n + 1);
                    sum += add;
                    if (Math.Abs(add) < 1e-17 * Math.Abs(sum)) break;
                }
                return 2 / Math.Sqrt(Math.PI) * sum;
            }
            return 1 - Erfc(x);
        }

        public static double Erfc(double x)
        {
            if (x < 2.5) return 1 - Erf(x);
            double t = 0;
            for (int n = 60; n >= 1; n--) t = n / 2.0 / (x + t);
            return Math.Exp(-x * x) / Math.Sqrt(Math.PI) / (x + t);
        }

        public static double ErfInv(double y)
        {
            if (y <= -1 || y >= 1) throw new CalcException("erfinv() argument must be in (-1, 1)");
            double w = -Math.Log((1 - y) * (1 + y)), x;
            if (w < 5)
            {
                w -= 2.5;
                x = 2.81022636e-08;
                x = 3.43273939e-07 + x * w; x = -3.5233877e-06 + x * w; x = -4.39150654e-06 + x * w; x = 0.00021858087 + x * w;
                x = -0.00125372503 + x * w; x = -0.00417768164 + x * w; x = 0.246640727 + x * w; x = 1.50140941 + x * w;
            }
            else
            {
                w = Math.Sqrt(w) - 3;
                x = -0.000200214257;
                x = 0.000100950558 + x * w; x = 0.00134934322 + x * w; x = -0.00367342844 + x * w; x = 0.00573950773 + x * w;
                x = -0.0076224613 + x * w; x = 0.00943887047 + x * w; x = 1.00167406 + x * w; x = 2.83297682 + x * w;
            }
            x *= y;
            for (int i = 0; i < 3; i++)
            {
                double err = Erf(x) - y;
                x -= err / (2 / Math.Sqrt(Math.PI) * Math.Exp(-x * x));
            }
            return x;
        }

        public static Complex Zeta(Complex s)
        {
            if (s == 1) throw new CalcException("zeta(1) is undefined");
            if (s.Real < 0.5)
            {
                return Complex.Pow(2, s) * Complex.Pow(Math.PI, s - 1) * Complex.Sin(Math.PI * s / 2) * Gamma(1 - s) * Zeta(1 - s);
            }
            int n = 40;
            var d = new double[n + 1];
            double sum = 0;
            for (int i = 0; i <= n; i++)
            {
                double num = 0;
                for (int j = 0; j <= i; j++) num += Fact(n + j - 1) * Math.Pow(4, j) / (Fact(n - j) * Fact(2 * j));
                sum = n * num;
                d[i] = sum;
            }
            Complex total = 0;
            for (int k = 0; k < n; k++)
            {
                double sign = k % 2 == 0 ? 1 : -1;
                total += sign * (d[k] - d[n]) / Complex.Pow(k + 1, s);
            }
            return -total / (d[n] * (1 - Complex.Pow(2, 1 - s)));
        }

        static double Fact(int n)
        {
            double r = 1;
            for (int i = 2; i <= n; i++) r *= i;
            return r;
        }

        public static Complex Polylog(Complex s, Complex z)
        {
            if (Complex.Abs(z) <= 0.75)
            {
                Complex sum = 0, zk = z;
                for (int k = 1; k < 2000; k++)
                {
                    var add = zk / Complex.Pow(k, s);
                    sum += add;
                    if (Complex.Abs(add) < 1e-17 * Complex.Abs(sum)) break;
                    zk *= z;
                }
                return sum;
            }
            if (s == 2 && z.Imaginary == 0 && z.Real <= 1)
            {
                double x = z.Real;
                if (x == 1) return Math.PI * Math.PI / 6;
                if (x > 0.5) return Math.PI * Math.PI / 6 - Math.Log(x) * Math.Log(1 - x) - Polylog(2, 1 - x);
                if (x < -1) return -Math.PI * Math.PI / 6 - 0.5 * Math.Pow(Math.Log(-x), 2) - Polylog(2, 1 / x);
                if (x < -0.5) return -Polylog(2, x / (x - 1)) - 0.5 * Math.Pow(Math.Log(1 - x), 2);
            }
            if (s == 1) return -Complex.Log(1 - z);
            if (z.Imaginary == 0 && z.Real == 1) return Zeta(s);
            Complex mu = Complex.Log(z);
            Complex res = Gamma(1 - s) * Complex.Pow(-mu, s - 1);
            Complex muk = 1;
            for (int k = 0; k < 60; k++)
            {
                if (s - k != 1) res += Zeta(s - k) * muk / Fact(k);
                muk *= mu;
            }
            return res;
        }

        public static double Ei(double x)
        {
            if (x == 0) throw new CalcException("Ei(0) is undefined");
            if (x < 0) return -E1(-x);
            if (x < 40)
            {
                double sum = 0, term = 1;
                for (int k = 1; k < 500; k++)
                {
                    term *= x / k;
                    double add = term / k;
                    sum += add;
                    if (add < 1e-17 * sum) break;
                }
                return 0.57721566490153286 + Math.Log(x) + sum;
            }
            double s = 1, t = 1;
            for (int k = 1; k < 40; k++) { t *= k / x; s += t; if (t < 1e-17) break; }
            return Math.Exp(x) / x * s;
        }

        public static double E1(double x)
        {
            if (x <= 0) throw new CalcException("E1() requires a positive argument");
            if (x < 1)
            {
                double sum = 0, term = 1;
                for (int k = 1; k < 100; k++)
                {
                    term *= -x / k;
                    sum -= term / k;
                }
                return -0.57721566490153286 - Math.Log(x) + sum;
            }
            double b = x + 1, c = 1e300, d = 1 / b, h = d;
            for (int i = 1; i < 300; i++)
            {
                double an = -i * (double)i;
                b += 2;
                d = 1 / (an * d + b);
                c = b + an / c;
                double del = c * d;
                h *= del;
                if (Math.Abs(del - 1) < 1e-16) break;
            }
            return h * Math.Exp(-x);
        }

        public static double Li(double x)
        {
            if (x == 1) return double.NegativeInfinity;
            if (x < 0) throw new CalcException("li() requires a non-negative argument");
            if (x == 0) return 0;
            return Ei(Math.Log(x));
        }

        public static void SiCi(double x, out double si, out double ci)
        {
            double ax = Math.Abs(x);
            if (ax < 4)
            {
                double s = 0, c = 0, term = ax;
                double x2 = ax * ax;
                for (int k = 0; k < 60; k++)
                {
                    s += term / (2 * k + 1);
                    term *= -x2 / ((2 * k + 2) * (2 * k + 3));
                }
                term = -x2 / 2;
                for (int k = 1; k < 60; k++)
                {
                    c += term / (2 * k);
                    term *= -x2 / ((2 * k + 1) * (2 * k + 2));
                }
                si = s;
                ci = ax == 0 ? double.NegativeInfinity : 0.57721566490153286 + Math.Log(ax) + c;
            }
            else
            {
                Complex b = new Complex(1, ax), cc = 1e300, d = 1 / b, h = d;
                for (int i = 2; i < 400; i++)
                {
                    double a = -(i - 1) * (double)(i - 1);
                    b += 2;
                    d = 1 / (a * d + b);
                    cc = b + a / cc;
                    var del = cc * d;
                    h *= del;
                    if (Complex.Abs(del - 1) < 1e-16) break;
                }
                h *= new Complex(Math.Cos(ax), -Math.Sin(ax));
                ci = -h.Real;
                si = Math.PI / 2 + h.Imaginary;
            }
            if (x < 0) si = -si;
        }

        public static double Shi(double x)
        {
            double sum = 0, term = x, x2 = x * x;
            if (Math.Abs(x) > 30) return 0.5 * (Ei(x) - Ei(-x));
            for (int k = 0; k < 200; k++)
            {
                double add = term / (2 * k + 1);
                sum += add;
                if (Math.Abs(add) < 1e-17 * Math.Abs(sum)) break;
                term *= x2 / ((2 * k + 2) * (2 * k + 3));
            }
            return sum;
        }

        public static double Chi(double x)
        {
            if (x <= 0) throw new CalcException("Chi() requires a positive argument");
            return 0.5 * (Ei(x) + Ei(-x));
        }

        public static double BesselJ(int n, double x)
        {
            if (n < 0) return (n % 2 == 0 ? 1 : -1) * BesselJ(-n, x);
            if (Math.Abs(x) < 25)
            {
                double sum = 0, term = Math.Pow(x / 2, n) / Fact(n);
                for (int k = 0; k < 300; k++)
                {
                    sum += term;
                    term *= -(x * x / 4) / ((k + 1) * (double)(k + n + 1));
                    if (Math.Abs(term) < 1e-18 * Math.Abs(sum) && k > 5) break;
                }
                return sum;
            }
            double integral = 0;
            int steps = 4000;
            for (int i = 0; i <= steps; i++)
            {
                double t = Math.PI * i / steps;
                double w = (i == 0 || i == steps) ? 1 : (i % 2 == 1 ? 4 : 2);
                integral += w * Math.Cos(n * t - x * Math.Sin(t));
            }
            return integral * (Math.PI / steps / 3) / Math.PI;
        }

        public static double BesselY(int n, double x)
        {
            if (x <= 0) throw new CalcException("bessely() requires a positive argument");
            double nu = n + 1e-7;
            double jnu = BesselJNonInt(nu, x), jmnu = BesselJNonInt(-nu, x);
            double nu2 = n - 1e-7;
            double y1 = (jnu * Math.Cos(nu * Math.PI) - jmnu) / Math.Sin(nu * Math.PI);
            double y2 = (BesselJNonInt(nu2, x) * Math.Cos(nu2 * Math.PI) - BesselJNonInt(-nu2, x)) / Math.Sin(nu2 * Math.PI);
            return (y1 + y2) / 2;
        }

        static double BesselJNonInt(double nu, double x)
        {
            double sum = 0;
            for (int k = 0; k < 200; k++)
            {
                double lg = LogGammaSigned(k + nu + 1, out int sg);
                double term = (k % 2 == 0 ? 1 : -1) * Math.Exp((2 * k + nu) * Math.Log(x / 2) - LogFact(k) - lg) * sg;
                sum += term;
                if (Math.Abs(term) < 1e-18 * Math.Abs(sum) && k > 10) break;
            }
            return sum;
        }

        static double LogFact(int k) => LogGamma(k + 1);

        static double LogGammaSigned(double x, out int sign)
        {
            sign = 1;
            if (x > 0) return LogGamma(x);
            double g = Gamma(x);
            sign = g < 0 ? -1 : 1;
            return Math.Log(Math.Abs(g));
        }

        public static double AiryAi(double x)
        {
            if (x > 8) return Math.Exp(-2.0 / 3 * Math.Pow(x, 1.5)) / (2 * Math.Sqrt(Math.PI) * Math.Pow(x, 0.25));
            double c1 = 0.355028053887817239, c2 = 0.258819403792806798;
            double f = 1, g = x, sf = 1, sg = x, x3 = x * x * x;
            for (int k = 1; k < 200; k++)
            {
                f *= x3 / ((3 * k - 1) * (3.0 * k));
                g *= x3 / ((3 * k) * (3.0 * k + 1));
                sf += f; sg += g;
                if (Math.Abs(f) + Math.Abs(g) < 1e-18) break;
            }
            return c1 * sf - c2 * sg;
        }

        public static Complex LambertW(Complex z, int k)
        {
            Complex w;
            if (k == 0) w = Complex.Abs(z) < 1 ? z : Complex.Log(z + 1);
            else w = Complex.Log(z) + new Complex(0, 2 * Math.PI * k);
            if (k == 0 && z.Imaginary == 0 && z.Real < -0.3 && z.Real >= -1 / Math.E)
                w = -1 + Complex.Sqrt(2 * (Math.E * z + 1));
            if (k == -1 && z.Imaginary == 0 && z.Real < 0 && z.Real >= -1 / Math.E)
                w = z.Real > -0.25 ? Math.Log(-z.Real) - Math.Log(-Math.Log(-z.Real)) : -1 - Complex.Sqrt(2 * (Math.E * z + 1));
            for (int i = 0; i < 100; i++)
            {
                var ew = Complex.Exp(w);
                var f = w * ew - z;
                var wp1 = w + 1;
                if (Complex.Abs(wp1) < 1e-300) break;
                var dw = f / (ew * wp1 - (w + 2) * f / (2 * wp1));
                w -= dw;
                if (Complex.Abs(dw) < 1e-16 * (1 + Complex.Abs(w))) break;
            }
            return w;
        }

        // regularized lower incomplete gamma
        public static double GammaP(double a, double x)
        {
            if (x < 0 || a <= 0) throw new CalcException("Invalid arguments for incomplete gamma");
            if (x == 0) return 0;
            if (x < a + 1)
            {
                double ap = a, sum = 1 / a, del = sum;
                for (int n = 0; n < 1000; n++)
                {
                    ap += 1; del *= x / ap; sum += del;
                    if (Math.Abs(del) < Math.Abs(sum) * 1e-16) break;
                }
                return sum * Math.Exp(-x + a * Math.Log(x) - LogGamma(a));
            }
            return 1 - GammaQcf(a, x);
        }

        static double GammaQcf(double a, double x)
        {
            double b = x + 1 - a, c = 1 / 1e-300, d = 1 / b, h = d;
            for (int i = 1; i < 1000; i++)
            {
                double an = -i * (i - a);
                b += 2;
                d = an * d + b; if (Math.Abs(d) < 1e-300) d = 1e-300;
                c = b + an / c; if (Math.Abs(c) < 1e-300) c = 1e-300;
                d = 1 / d;
                double del = d * c;
                h *= del;
                if (Math.Abs(del - 1) < 1e-16) break;
            }
            return Math.Exp(-x + a * Math.Log(x) - LogGamma(a)) * h;
        }

        public static double UpperIncGamma(double a, double x) => (1 - GammaP(a, x)) * Gamma(a);
        public static double LowerIncGamma(double a, double x) => GammaP(a, x) * Gamma(a);

        public static double Beta(double a, double b) => Math.Exp(LogGamma(a) + LogGamma(b) - LogGamma(a + b)) * Math.Sign(Gamma(a)) * Math.Sign(Gamma(b)) * Math.Sign(Gamma(a + b));

        // regularized incomplete beta
        public static double BetaInc(double x, double a, double b)
        {
            if (x <= 0) return 0;
            if (x >= 1) return 1;
            double bt = Math.Exp(LogGamma(a + b) - LogGamma(a) - LogGamma(b) + a * Math.Log(x) + b * Math.Log(1 - x));
            if (x < (a + 1) / (a + b + 2)) return bt * BetaCf(x, a, b) / a;
            return 1 - bt * BetaCf(1 - x, b, a) / b;
        }

        static double BetaCf(double x, double a, double b)
        {
            double qab = a + b, qap = a + 1, qam = a - 1, c = 1, d = 1 - qab * x / qap;
            if (Math.Abs(d) < 1e-300) d = 1e-300;
            d = 1 / d;
            double h = d;
            for (int m = 1; m < 1000; m++)
            {
                int m2 = 2 * m;
                double aa = m * (b - m) * x / ((qam + m2) * (a + m2));
                d = 1 + aa * d; if (Math.Abs(d) < 1e-300) d = 1e-300;
                c = 1 + aa / c; if (Math.Abs(c) < 1e-300) c = 1e-300;
                d = 1 / d; h *= d * c;
                aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
                d = 1 + aa * d; if (Math.Abs(d) < 1e-300) d = 1e-300;
                c = 1 + aa / c; if (Math.Abs(c) < 1e-300) c = 1e-300;
                d = 1 / d;
                double del = d * c;
                h *= del;
                if (Math.Abs(del - 1) < 1e-16) break;
            }
            return h;
        }

        public static void Fresnel(double x, out double s, out double c)
        {
            double ss = 0, cc = 0;
            int n = 2000;
            for (int i = 0; i <= n; i++)
            {
                double t = x * i / n;
                double w = (i == 0 || i == n) ? 1 : (i % 2 == 1 ? 4 : 2);
                ss += w * Math.Sin(Math.PI * t * t / 2);
                cc += w * Math.Cos(Math.PI * t * t / 2);
            }
            s = ss * x / n / 3; c = cc * x / n / 3;
        }

        public static double NormalCdf(double x, double mu, double sigma) => 0.5 * (1 + Erf((x - mu) / (sigma * Math.Sqrt(2))));
        public static double NormalPdf(double x, double mu, double sigma) => Math.Exp(-0.5 * Math.Pow((x - mu) / sigma, 2)) / (sigma * Math.Sqrt(2 * Math.PI));

        public static double NormalInv(double p, double mu, double sigma)
        {
            if (p <= 0 || p >= 1) throw new CalcException("Probability must be in (0, 1)");
            return mu + sigma * Math.Sqrt(2) * ErfInv(2 * p - 1);
        }

        public static double StudentTCdf(double t, double v)
        {
            double x = v / (v + t * t);
            double ib = BetaInc(x, v / 2, 0.5);
            return t >= 0 ? 1 - 0.5 * ib : 0.5 * ib;
        }

        public static double ChiSquareCdf(double x, double k) => x <= 0 ? 0 : GammaP(k / 2, x / 2);
    }
}
