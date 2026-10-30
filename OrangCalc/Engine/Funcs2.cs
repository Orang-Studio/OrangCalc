using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Numerics;
using System.Text;

namespace OrangCalc.Engine
{
    public static partial class Funcs
    {
        public static event Action<string, double, double> PlotRequested;

        static List<Expr> Items(List<Expr> args)
        {
            var l = Flatten(args);
            return l;
        }

        static List<Real> Reals(List<Expr> args, string fn)
        {
            var l = new List<Real>();
            foreach (var e in Items(args))
            {
                if (IsR(e, out var r)) l.Add(r);
                else throw new CalcException(fn + "() requires real numbers");
            }
            if (l.Count == 0) throw new CalcException(fn + "() requires at least one value");
            return l;
        }

        static bool AllNumeric(List<Expr> args) => Items(args).All(e => e is NumE n && n.V.IsReal);

        static MatE AsMat(Expr e, string fn)
        {
            if (e is MatE m) return m;
            if (e is NumE) return Matrices.AsMatrix(e);
            throw new CalcException(fn + "() requires a matrix");
        }

        static string SymName(Expr e, string fallback = "x")
        {
            if (e == null) return fallback;
            if (e is SymE s) return s.Name;
            if (e is StrE st) return st.S;
            if (e is VarE v) return v.V.Name;
            if (e is UnitE u) return u.U.Abbrev;
            throw new CalcException("Expected a variable name");
        }

        static Expr Sum(List<Expr> l) => Simp.Add(l);

        static void RegisterMore(Calculator c)
        {
            const string ST = "Statistics", MV = "Matrices & Vectors", AL = "Algebra", CA = "Calculus", LG = "Logical", BT = "Bits", UT = "Utilities", DT = "Date & Time", EC = "Economics", GE = "Geometry", IV = "Intervals", NB = "Number Bases", RN = "Random Numbers";

            Def(c, "_percent", "", "Percent", "x", (cc, a) => Simp.Div(a[0], Expr.N(100))).Hidden = true;
            Def(c, "_hold", "", "Hold", "x", (cc, a) => a[0] is NumE || a[0] is SymE ? a[0] : null).Hidden = true;
            Def(c, "_permille", "", "Per Mille", "x", (cc, a) => Simp.Div(a[0], Expr.N(1000))).Hidden = true;
            Def(c, "_angle", "", "Angle Notation", "r,angle", (cc, a) => Simp.Mul(a[0], cc.Ev.Call("cis", a[1]))).Hidden = true;
            Def(c, "_dataset", "", "Data Set Lookup", "set,object,property=\"\"", (cc, a) => Data.Lookup(cc, Str(a[0]), a[1], a.Count > 2 ? Str(a[2]) : "")).RawArgs = true;

            Def(c, "sum,∑", ST, "Sum", "term:hold,lower=1,upper?,var:hold=x", (cc, a) =>
            {
                if (a.Count == 1 || a.Count == 2 && a[1].IsOne && false) { var v = cc.Ev.Eval(a[0]); return v is MatE vm ? Sum(vm.All.ToList()) : v; }
                if (a.Count < 3) { var v = cc.Ev.Eval(a[0]); if (v is MatE vm) return Sum(vm.All.ToList()); throw new CalcException("sum() requires term, lower and upper limit"); }
                return SeriesOp(cc, a, true);
            }, "Sum of term for var from lower to upper, or sum of vector elements.");
            Def(c, "product,∏", ST, "Product", "term:hold,lower=1,upper?,var:hold=x", (cc, a) =>
            {
                if (a.Count < 3) { var v = cc.Ev.Eval(a[0]); if (v is MatE vm) return Simp.Mul(vm.All.ToList()); throw new CalcException("product() requires term, lower and upper limit"); }
                return SeriesOp(cc, a, false);
            }, "Product of term for var from lower to upper.");
            Def(c, "total", ST, "Sum of Elements", "values:vec,...", (cc, a) => Sum(Items(a)));
            Def(c, "count", ST, "Number of Elements", "values,...", (cc, a) => Expr.N(Items(a).Count));
            Def(c, "mean,average,avg", ST, "Mean", "values:vec,...", (cc, a) => { var l = Items(a); return Simp.Div(Sum(l), Expr.N(l.Count)); }, "Arithmetic mean.");
            Def(c, "geomean", ST, "Geometric Mean", "values:vec,...", (cc, a) => { var l = Items(a); return Simp.Pow(Simp.Mul(l), Expr.N(Real.Make(1, l.Count))); });
            Def(c, "harmmean", ST, "Harmonic Mean", "values:vec,...", (cc, a) => { var l = Items(a); return Simp.Div(Expr.N(l.Count), Sum(l.Select(x => Simp.Pow(x, Expr.MinusOne)).ToList())); });
            Def(c, "rms", ST, "Root Mean Square", "values:vec,...", (cc, a) => { var l = Items(a); return Simp.Pow(Simp.Div(Sum(l.Select(x => Simp.Pow(x, Expr.Two)).ToList()), Expr.N(l.Count)), Expr.Half); });
            Def(c, "median", ST, "Median", "values:vec,...", (cc, a) =>
            {
                var l = Reals(a, "median").OrderBy(x => x).ToList();
                int n = l.Count;
                return Num(n % 2 == 1 ? l[n / 2] : (l[n / 2 - 1] + l[n / 2]) / Real.Two);
            });
            Def(c, "mode", ST, "Mode", "values:vec,...", (cc, a) =>
            {
                var g = Items(a).GroupBy(x => x.Key).OrderByDescending(x => x.Count()).First();
                return g.First();
            }, "Most frequent value.");
            Def(c, "min", ST, "Minimum", "values,...", (cc, a) =>
            {
                var l = Items(a);
                if (!AllNumeric(l)) return QuantityExtreme(cc, l, -1);
                return Num(Reals(l, "min").Aggregate(Real.Min));
            });
            Def(c, "max", ST, "Maximum", "values,...", (cc, a) =>
            {
                var l = Items(a);
                if (!AllNumeric(l)) return QuantityExtreme(cc, l, 1);
                return Num(Reals(l, "max").Aggregate(Real.Max));
            });
            Def(c, "range", ST, "Range", "values:vec,...", (cc, a) => { var l = Reals(a, "range"); return Num(l.Aggregate(Real.Max) - l.Aggregate(Real.Min)); });
            Def(c, "var,variance", ST, "Variance (sample)", "values:vec,...", (cc, a) => Variance(Items(a), true));
            Def(c, "varp", ST, "Variance (population)", "values:vec,...", (cc, a) => Variance(Items(a), false));
            Def(c, "stdev,stddev", ST, "Standard Deviation (sample)", "values:vec,...", (cc, a) => Simp.Pow(Variance(Items(a), true), Expr.Half));
            Def(c, "stdevp", ST, "Standard Deviation (population)", "values:vec,...", (cc, a) => Simp.Pow(Variance(Items(a), false), Expr.Half));
            Def(c, "percentile", ST, "Percentile", "values:vec,p:real", (cc, a) =>
            {
                var l = Reals(new List<Expr> { a[0] }, "percentile").OrderBy(x => x).ToList();
                var p = ((NumE)a[1]).V.Re / Real.Int(100);
                var pos = p * Real.Int(l.Count - 1);
                int lo = pos.Floor().ToInt();
                if (lo >= l.Count - 1) return Num(l.Last());
                if (lo < 0) return Num(l[0]);
                var fr = pos - Real.Int(lo);
                return Num(l[lo] + (l[lo + 1] - l[lo]) * fr);
            });
            Def(c, "quartile", ST, "Quartile", "values:vec,q:int", (cc, a) => cc.Ev.Call("percentile", a[0], Simp.Mul(a[1], Expr.N(25))));
            Def(c, "cov,covariance", ST, "Covariance", "x:vec,y:vec", (cc, a) =>
            {
                var x = Items(new List<Expr> { a[0] }); var y = Items(new List<Expr> { a[1] });
                if (x.Count != y.Count) throw new CalcException("Vectors must have the same length");
                var mx = Simp.Div(Sum(x), Expr.N(x.Count)); var my = Simp.Div(Sum(y), Expr.N(y.Count));
                return Simp.Div(Sum(x.Select((xi, i) => Simp.Mul(Simp.Sub(xi, mx), Simp.Sub(y[i], my))).ToList()), Expr.N(x.Count - 1));
            });
            Def(c, "corr,correlation,pearson", ST, "Correlation Coefficient", "x:vec,y:vec", (cc, a) =>
            {
                var cov = cc.Ev.Call("cov", a[0], a[1]);
                return Simp.Div(cov, Simp.Mul(cc.Ev.Call("stdev", a[0]), cc.Ev.Call("stdev", a[1])));
            });
            Def(c, "linreg", ST, "Linear Regression", "x:vec,y:vec", (cc, a) =>
            {
                var x = Reals(new List<Expr> { a[0] }, "linreg"); var y = Reals(new List<Expr> { a[1] }, "linreg");
                int n = x.Count;
                var sx = x.Aggregate((p, q) => p + q); var sy = y.Aggregate((p, q) => p + q);
                Real sxy = Real.Zero, sxx = Real.Zero;
                for (int i = 0; i < n; i++) { sxy += x[i] * y[i]; sxx += x[i] * x[i]; }
                var slope = (Real.Int(n) * sxy - sx * sy) / (Real.Int(n) * sxx - sx * sx);
                var icpt = (sy - slope * sx) / Real.Int(n);
                return MatE.Vector(new List<Expr> { Num(slope), Num(icpt) });
            }, "Returns [slope, intercept] of the least squares line.");
            Def(c, "normdist,normal", ST, "Normal Distribution", "x:real,mean:real=0,sd:real=1,cumulative:bool=0", (cc, a) =>
            {
                if (!a.All(z => z is NumE) || cc.ExactPhase && !a.Any(z => z.IsApprox)) return null;
                double x = D(a[0], "normdist"), m = D(a[1], "normdist"), s = D(a[2], "normdist");
                return Num(!a[3].IsZero ? Special.NormalCdf(x, m, s) : Special.NormalPdf(x, m, s));
            }, "Probability density, or cumulative probability.");
            Def(c, "norminv", ST, "Inverse Normal Distribution", "p:real,mean:real=0,sd:real=1", (cc, a) => { if (cc.ExactPhase && !a.Any(z => z.IsApprox)) return null; return Num(Special.NormalInv(D(a[0], "norminv"), D(a[1], "norminv"), D(a[2], "norminv"))); });
            Def(c, "poisson", ST, "Poisson Distribution", "k:nnint,lambda:real,cumulative:bool=0", (cc, a) =>
            {
                int k = SmallInt(a[0], "poisson");
                var lam = a[1];
                Func<int, Expr> pmf = i => Simp.Div(Simp.Mul(Simp.Pow(lam, Expr.N(i)), cc.Ev.Call("exp", Simp.Neg(lam))), Num(Primes.Factorial(i)));
                if (a[2].IsZero) return pmf(k);
                return Sum(Enumerable.Range(0, k + 1).Select(pmf).ToList());
            });
            Def(c, "binomdist", ST, "Binomial Distribution", "k:nnint,n:nnint,p:real,cumulative:bool=0", (cc, a) =>
            {
                int k = SmallInt(a[0], "binomdist"), n = SmallInt(a[1], "binomdist");
                var p = a[2];
                Func<int, Expr> pmf = i => Simp.Mul(Num(Primes.Binomial(n, i)), Simp.Pow(p, Expr.N(i)), Simp.Pow(Simp.Sub(Expr.One, p), Expr.N(n - i)));
                if (a[3].IsZero) return pmf(k);
                return Sum(Enumerable.Range(0, k + 1).Select(pmf).ToList());
            });
            Def(c, "expdist", ST, "Exponential Distribution", "x:real,lambda:real,cumulative:bool=0", (cc, a) =>
            {
                var e = cc.Ev.Call("exp", Simp.Neg(Simp.Mul(a[1], a[0])));
                return a[2].IsZero ? Simp.Mul(a[1], e) : Simp.Sub(Expr.One, e);
            });
            Def(c, "tdist", ST, "Student's t Distribution", "t:real,df:real", (cc, a) => { if (cc.ExactPhase && !a.Any(z => z.IsApprox)) return null; return Num(Special.StudentTCdf(D(a[0], "tdist"), D(a[1], "tdist"))); }, "Cumulative probability.");
            Def(c, "chisqdist", ST, "Chi-squared Distribution", "x:real,k:real", (cc, a) => { if (cc.ExactPhase && !a.Any(z => z.IsApprox)) return null; return Num(Special.ChiSquareCdf(D(a[0], "chisqdist"), D(a[1], "chisqdist"))); }, "Cumulative probability.");
            Def(c, "uniformdist", ST, "Uniform Distribution", "x:real,a:real,b:real,cumulative:bool=0", (cc, a) =>
            {
                if (!a.All(z => z is NumE)) return null;
                var x = ((NumE)a[0]).V.Re; var lo = ((NumE)a[1]).V.Re; var hi = ((NumE)a[2]).V.Re;
                if (a[3].IsZero) return x < lo || x > hi ? Expr.Zero : Num(Real.One / (hi - lo));
                return x < lo ? Expr.Zero : x > hi ? Expr.One : Num((x - lo) / (hi - lo));
            });

            Def(c, "rand,random", RN, "Random Number", "max:int=0,count:int=1", (cc, a) =>
            {
                int count = a.Count > 1 ? SmallInt(a[1], "rand") : 1;
                var mx = a.Count > 0 ? Int(a[0], "rand") : 0;
                Func<Expr> one = () => mx <= 0 ? Num(Real.FromDouble(Real.Rng.NextDouble())) : Num(RandomBig(mx) + 1);
                if (count == 1) return one();
                return MatE.Vector(Enumerable.Range(0, count).Select(_ => one()).ToList());
            }, "Random number in [0, 1), or integer in [1, max].");
            Def(c, "randint", RN, "Random Integer", "a:int,b:int", (cc, a) => { var lo = Int(a[0], "randint"); var hi = Int(a[1], "randint"); return Num(lo + RandomBig(hi - lo + 1)); });
            Def(c, "randn,randnorm", RN, "Normally Distributed Random Number", "mean:real=0,sd:real=1", (cc, a) =>
            {
                double u1 = 1 - Real.Rng.NextDouble(), u2 = Real.Rng.NextDouble();
                double z = Math.Sqrt(-2 * Math.Log(u1)) * Math.Cos(2 * Math.PI * u2);
                return Num(D(a[0], "randn") + D(a[1], "randn") * z);
            });
            Def(c, "randuniform", RN, "Uniform Random Number", "a:real,b:real", (cc, a) => Num(D(a[0], "randuniform") + (D(a[1], "randuniform") - D(a[0], "randuniform")) * Real.Rng.NextDouble()));
            Def(c, "randpoisson", RN, "Poisson Random Number", "lambda:real", (cc, a) =>
            {
                double l = Math.Exp(-D(a[0], "randpoisson")), p = 1; int k = 0;
                do { k++; p *= Real.Rng.NextDouble(); } while (p > l && k < 1000000);
                return Expr.N(k - 1);
            });
            Def(c, "dice", RN, "Roll Dice", "n:pint=1,sides:pint=6", (cc, a) =>
            {
                int n = SmallInt(a[0], "dice"), s = SmallInt(a[1], "dice");
                long t = 0;
                for (int i = 0; i < n; i++) t += Real.Rng.Next(1, s + 1);
                return Expr.N(t);
            });

            Def(c, "matrix", MV, "Construct Matrix", "rows:pint,columns:pint,elements:vec?", (cc, a) =>
            {
                int r = SmallInt(a[0], "matrix"), co = SmallInt(a[1], "matrix");
                var el = a.Count > 2 ? Items(new List<Expr> { a[2] }) : new List<Expr>();
                return Matrices.Create(r, co, (i, j) => i * co + j < el.Count ? el[i * co + j] : Expr.Zero);
            });
            Def(c, "vector", MV, "Construct Vector", "elements,...", (cc, a) => MatE.Vector(a.ToList()));
            Def(c, "identity,eye", MV, "Identity Matrix", "n:pint", (cc, a) => Matrices.Identity(SmallInt(a[0], "identity")));
            Def(c, "zero,zeros", MV, "Zero Matrix", "rows:pint,columns:pint=0", (cc, a) => { int r = SmallInt(a[0], "zero"); int co = a.Count > 1 && !a[1].IsZero ? SmallInt(a[1], "zero") : r; return Matrices.Create(r, co, (i, j) => Expr.Zero); });
            Def(c, "ones", MV, "Matrix of Ones", "rows:pint,columns:pint=0", (cc, a) => { int r = SmallInt(a[0], "ones"); int co = a.Count > 1 && !a[1].IsZero ? SmallInt(a[1], "ones") : r; return Matrices.Create(r, co, (i, j) => Expr.One); });
            Def(c, "det,determinant", MV, "Determinant", "m:mat", (cc, a) => a[0] is MatE m ? Matrices.Determinant(m) : (a[0] is NumE ? a[0] : null));
            Def(c, "inverse,inv", MV, "Inverse Matrix", "m:mat", (cc, a) => a[0] is MatE m ? Matrices.Inverse(m) : Simp.Pow(a[0], Expr.MinusOne));
            Def(c, "transpose", MV, "Transpose", "m:mat", (cc, a) => a[0] is MatE m ? (Expr)Matrices.Transpose(m) : a[0]);
            Def(c, "adj,adjoint,adjugate", MV, "Adjugate", "m:mat", (cc, a) => Matrices.Adjugate(AsMat(a[0], "adj")));
            Def(c, "cofactor", MV, "Cofactor", "m:mat,row:pint,column:pint", (cc, a) => Matrices.Cofactor(AsMat(a[0], "cofactor"), SmallInt(a[1], "cofactor") - 1, SmallInt(a[2], "cofactor") - 1));
            Def(c, "minor", MV, "Minor", "m:mat,row:pint,column:pint", (cc, a) => Matrices.Determinant(Matrices.Minor(AsMat(a[0], "minor"), SmallInt(a[1], "minor") - 1, SmallInt(a[2], "minor") - 1)));
            Def(c, "rank", MV, "Rank", "m:mat", (cc, a) => Expr.N(Matrices.Rank(AsMat(a[0], "rank"))));
            Def(c, "rref", MV, "Reduced Row Echelon Form", "m:mat", (cc, a) => Matrices.Rref(AsMat(a[0], "rref")));
            Def(c, "trace", MV, "Trace", "m:mat", (cc, a) => Matrices.Trace(AsMat(a[0], "trace")));
            Def(c, "permanent", MV, "Permanent", "m:mat", (cc, a) => Matrices.Permanent(AsMat(a[0], "permanent")));
            Def(c, "dot", MV, "Dot Product", "a:vec,b:vec", (cc, a) => Matrices.Dot(AsMat(a[0], "dot"), AsMat(a[1], "dot")));
            Def(c, "cross", MV, "Cross Product", "a:vec,b:vec", (cc, a) => Matrices.Cross(AsMat(a[0], "cross"), AsMat(a[1], "cross")));
            Def(c, "kron,kronecker", MV, "Kronecker Product", "a:mat,b:mat", (cc, a) => Matrices.Kronecker(AsMat(a[0], "kron"), AsMat(a[1], "kron")));
            Def(c, "hadamard", MV, "Hadamard Product", "a:mat,b:mat", (cc, a) => Matrices.Hadamard(AsMat(a[0], "hadamard"), AsMat(a[1], "hadamard")));
            Def(c, "norm,magnitude", MV, "Norm", "v:vec,p:real=2", (cc, a) =>
            {
                var l = a[0] is MatE m ? m.All.ToList() : new List<Expr> { a[0] };
                var p = a[1];
                if (p.IsOne) return Sum(l.Select(x => cc.Ev.Call("abs", x)).ToList());
                if (p is NumE pn && pn.V.Re.IsInfinite) return cc.Ev.Call("max", MatE.Vector(l.Select(x => cc.Ev.Call("abs", x)).ToList()));
                if (p.Key == Expr.Two.Key) return Simp.Pow(Sum(l.Select(x => x is NumE n && !n.V.IsReal ? Num(n.V.Re * n.V.Re + n.V.Im * n.V.Im) : Simp.Pow(x, Expr.Two)).ToList()), Expr.Half);
                return Simp.Pow(Sum(l.Select(x => Simp.Pow(cc.Ev.Call("abs", x), p)).ToList()), Simp.Pow(p, Expr.MinusOne));
            }, "Euclidean norm, or p-norm.");
            Def(c, "normalize", MV, "Normalize Vector", "v:vec", (cc, a) => { var n = cc.Ev.Call("norm", a[0]); return Matrices.MapElements(AsMat(a[0], "normalize"), x => Simp.Div(x, n)); });
            Def(c, "size,dimension,dim", MV, "Size", "m", (cc, a) => a[0] is MatE m ? (m.IsVector ? (Expr)Expr.N(m.ColCount) : MatE.Vector(new List<Expr> { Expr.N(m.RowCount), Expr.N(m.ColCount) })) : Expr.One);
            Def(c, "rows", MV, "Number of Rows", "m:mat", (cc, a) => Expr.N(AsMat(a[0], "rows").RowCount));
            Def(c, "columns,cols", MV, "Number of Columns", "m:mat", (cc, a) => Expr.N(AsMat(a[0], "columns").ColCount));
            Def(c, "elements,numel", MV, "Number of Elements", "m:mat", (cc, a) => Expr.N(AsMat(a[0], "elements").All.Count()));
            Def(c, "element", MV, "Element", "m:mat,row:pint,column:pint=1", (cc, a) =>
            {
                var m = AsMat(a[0], "element");
                int r = SmallInt(a[1], "element") - 1, co = SmallInt(a[2], "element") - 1;
                if (m.IsVector) { if (r >= m.ColCount) throw new CalcException("Index out of range"); return m.Rows[0][r]; }
                if (r >= m.RowCount || co >= m.ColCount) throw new CalcException("Index out of range");
                return m.Rows[r][co];
            });
            Def(c, "row", MV, "Row", "m:mat,row:pint", (cc, a) => { var m = AsMat(a[0], "row"); int r = SmallInt(a[1], "row") - 1; if (r >= m.RowCount) throw new CalcException("Index out of range"); return MatE.Vector(m.Rows[r].ToList()); });
            Def(c, "column", MV, "Column", "m:mat,column:pint", (cc, a) => { var m = AsMat(a[0], "column"); int co = SmallInt(a[1], "column") - 1; if (co >= m.ColCount) throw new CalcException("Index out of range"); return MatE.Vector(m.Rows.Select(r => r[co]).ToList()); });
            Def(c, "submatrix,area", MV, "Submatrix", "m:mat,r1:pint,c1:pint,r2:pint,c2:pint", (cc, a) =>
            {
                var m = AsMat(a[0], "submatrix");
                int r1 = SmallInt(a[1], "submatrix") - 1, c1 = SmallInt(a[2], "submatrix") - 1, r2 = SmallInt(a[3], "submatrix") - 1, c2 = SmallInt(a[4], "submatrix") - 1;
                return Matrices.Create(r2 - r1 + 1, c2 - c1 + 1, (i, j) => m.Rows[r1 + i][c1 + j]);
            });
            Def(c, "sort", MV, "Sort", "v:vec,descending:bool=0", (cc, a) =>
            {
                var l = Items(new List<Expr> { a[0] });
                var keyed = l.Select(x => (x, k: x is NumE n && n.V.IsReal ? n.V.Re.ToDouble() : double.NaN)).ToList();
                var sorted = keyed.OrderBy(t => t.k).Select(t => t.x).ToList();
                if (!a[1].IsZero) sorted.Reverse();
                return MatE.Vector(sorted);
            });
            Def(c, "reverse,flip", MV, "Reverse", "v:vec", (cc, a) => { var l = Items(new List<Expr> { a[0] }); l.Reverse(); return MatE.Vector(l); });
            Def(c, "horzcat,hcat", MV, "Horizontal Concatenation", "a:mat,b:mat", (cc, a) =>
            {
                var x = AsMat(a[0], "horzcat"); var y = AsMat(a[1], "horzcat");
                if (x.IsVector && y.IsVector) return MatE.Vector(x.Items.Concat(y.Items).ToList());
                if (x.RowCount != y.RowCount) throw new CalcException("Row counts differ");
                return new MatE(x.Rows.Select((r, i) => r.Concat(y.Rows[i]).ToList()).ToList(), false);
            });
            Def(c, "vertcat,vcat", MV, "Vertical Concatenation", "a:mat,b:mat", (cc, a) =>
            {
                var x = AsMat(a[0], "vertcat"); var y = AsMat(a[1], "vertcat");
                if (x.ColCount != y.ColCount) throw new CalcException("Column counts differ");
                return new MatE(x.Rows.Concat(y.Rows).Select(r => r.ToList()).ToList(), false);
            });
            Def(c, "genvector,seq", MV, "Generate Vector", "expression:hold,min,max,step=1,var:hold=x", (cc, a) =>
            {
                string v = SymName(a[4]);
                if (!IsR(a[1], out var lo) || !IsR(a[2], out var hi) || !IsR(a[3], out var st) || st.IsZero) throw new CalcException("genvector() requires numeric limits");
                var items = new List<Expr>();
                for (var x = lo; st.Sign > 0 ? x <= hi : x >= hi; x += st)
                {
                    cc.CheckAbort();
                    items.Add(cc.Ev.EvalWith(a[0], v, Num(x)));
                    if (items.Count > 100000) throw new CalcException("Too many elements");
                }
                return MatE.Vector(items);
            }, "Vector of expression values for var from min to max.");
            Def(c, "linspace", MV, "Linearly Spaced Vector", "a:real,b:real,n:pint", (cc, a) =>
            {
                int n = SmallInt(a[2], "linspace");
                if (n == 1) return MatE.Vector(new List<Expr> { a[0] });
                var step = Simp.Div(Simp.Sub(a[1], a[0]), Expr.N(n - 1));
                return MatE.Vector(Enumerable.Range(0, n).Select(i => Simp.Add(a[0], Simp.Mul(Expr.N(i), step))).ToList());
            });
            Def(c, "eigenvalues,eig", MV, "Eigenvalues", "m:mat", (cc, a) =>
            {
                var m = AsMat(a[0], "eigenvalues");
                if (m.RowCount != m.ColCount) throw new CalcException("Eigenvalues require a square matrix");
                int n = m.RowCount;
                if (n <= 4 && m.All.All(x => x is NumE nn && nn.V.IsRational))
                {
                    var lam = new SymE("_lambda");
                    var shifted = Matrices.Create(n, n, (i, j) => i == j ? Simp.Sub(m.Rows[i][j], lam) : m.Rows[i][j]);
                    var cp = Matrices.Determinant(shifted);
                    try { return MatE.Vector(Solver.Solve(cc, new CmpE(CmpOp.Eq, cp, Expr.Zero), "_lambda")); }
                    catch (CalcException) { }
                }
                var arr = new double[n, n];
                for (int i = 0; i < n; i++) for (int j = 0; j < n; j++) arr[i, j] = D(cc.Ev.Eval(m.Rows[i][j]), "eigenvalues");
                return MatE.Vector(Matrices.EigenvaluesNumeric(arr).Select(z => NumC(new System.Numerics.Complex(Math.Round(z.Real, 12), Math.Abs(z.Imaginary) < 1e-10 ? 0 : z.Imaginary))).ToList());
            });
            Def(c, "charpoly", MV, "Characteristic Polynomial", "m:mat,var:hold=x", (cc, a) =>
            {
                var m = AsMat(a[0], "charpoly");
                var x = new SymE(SymName(a[1]));
                var shifted = Matrices.Create(m.RowCount, m.ColCount, (i, j) => i == j ? Simp.Sub(m.Rows[i][j], x) : m.Rows[i][j]);
                return Simp.Expand(Matrices.Determinant(shifted));
            });

            Def(c, "solve", AL, "Solve Equation", "equation,var:hold=x", (cc, a) =>
            {
                var v = SymName(a[1]);
                var sols = Solver.Solve(cc, a[0], v);
                if (sols.Count == 1) return sols[0];
                return MatE.Vector(sols);
            }, "Solves an equation for var.");
            Def(c, "solve2", AL, "Solve System of Two Equations", "eq1,eq2,x:hold=x,y:hold=y", (cc, a) =>
                MatE.Vector(Solver.MultiSolve(cc, new List<Expr> { a[0], a[1] }, new List<string> { SymName(a[2]), SymName(a[3], "y") })));
            Def(c, "multisolve", AL, "Solve System of Equations", "equations:vec,vars:hold", (cc, a) =>
            {
                var eqs = Items(new List<Expr> { a[0] });
                var vraw = a[1] is MatE vm ? vm.Items : new List<Expr> { a[1] };
                return MatE.Vector(Solver.MultiSolve(cc, eqs, vraw.Select(z => SymName(z)).ToList()));
            });
            Def(c, "newtonsolve", AL, "Newton's Method", "f:hold,x0:real,var:hold=x", (cc, a) =>
            {
                var f = a[0] is CmpE ? a[0] : a[0];
                string v = SymName(a[2]);
                Expr g = f is CmpE ce ? Simp.Sub(ce.L, ce.R) : f;
                var r = Solver.Newton(x => cc.Ev.EvalDouble(g, v, x), D(a[1], "newtonsolve"));
                if (double.IsNaN(r)) throw new CalcException("Newton's method did not converge");
                return Num(r);
            });
            Def(c, "secantsolve", AL, "Secant Method", "f:hold,x0:real,x1:real,var:hold=x", (cc, a) =>
            {
                string v = SymName(a[3]);
                Expr g = a[0] is CmpE ce ? Simp.Sub(ce.L, ce.R) : a[0];
                var r = Solver.Secant(x => cc.Ev.EvalDouble(g, v, x), D(a[1], "secantsolve"), D(a[2], "secantsolve"));
                if (double.IsNaN(r)) throw new CalcException("Secant method did not converge");
                return Num(r);
            });
            Def(c, "dsolve", CA, "Solve Differential Equation", "equation:hold,x0?,y0?", (cc, a) =>
            {
                var syms = Calculator.Symbols(a[0]);
                string y = syms.Contains("y") ? "y" : syms.FirstOrDefault(s => s != "x") ?? "y";
                return Solver.DSolve(cc, a[0], y, "x", a.Count > 1 ? cc.Ev.Eval(a[1]) : null, a.Count > 2 ? cc.Ev.Eval(a[2]) : null);
            }, "Solves y' = a·y + b for y(x).");
            Def(c, "factor,factorize", AL, "Factorize", "x", (cc, a) => Hold(cc, Polynomials.Factorize(cc, a[0])));
            Def(c, "expand", AL, "Expand", "x", (cc, a) => Simp.Expand(a[0]));
            Def(c, "partfrac,apart", AL, "Partial Fractions", "x", (cc, a) => Hold(cc, Polynomials.PartialFractions(cc, a[0])));
            Def(c, "together", AL, "Combine Fractions", "x", (cc, a) => Polynomials.Together(a[0]));
            Def(c, "simplify", AL, "Simplify", "x", (cc, a) => cc.Structure(a[0], StructMode.Simplify));
            Def(c, "coeff,coefficient", AL, "Coefficient", "p,n:nnint,var:hold=x", (cc, a) =>
            {
                if (!Polynomials.TryCoefficients(a[0], SymName(a[2]), out var cs)) throw new CalcException("Not a polynomial");
                int n = SmallInt(a[1], "coeff");
                return n < cs.Count ? cs[n] : Expr.Zero;
            });
            Def(c, "degree", AL, "Degree", "p,var:hold=x", (cc, a) =>
            {
                if (!Polynomials.TryCoefficients(a[0], SymName(a[1]), out var cs)) throw new CalcException("Not a polynomial");
                int d = cs.Count - 1; while (d > 0 && cs[d].IsZero) d--;
                return Expr.N(d);
            });
            Def(c, "lcoeff", AL, "Leading Coefficient", "p,var:hold=x", (cc, a) =>
            {
                if (!Polynomials.TryCoefficients(a[0], SymName(a[1]), out var cs)) throw new CalcException("Not a polynomial");
                return cs.LastOrDefault(z => !z.IsZero) ?? Expr.Zero;
            });
            Def(c, "tcoeff", AL, "Trailing Coefficient", "p,var:hold=x", (cc, a) =>
            {
                if (!Polynomials.TryCoefficients(a[0], SymName(a[1]), out var cs)) throw new CalcException("Not a polynomial");
                return cs.FirstOrDefault(z => !z.IsZero) ?? Expr.Zero;
            });
            Def(c, "roots,polyroots", AL, "Polynomial Roots", "p,var:hold=x", (cc, a) => MatE.Vector(Solver.Solve(cc, new CmpE(CmpOp.Eq, a[0], Expr.Zero), SymName(a[1]))));
            Def(c, "replace,subs", AL, "Replace", "expression,from,to", (cc, a) => cc.Ev.Eval(cc.Substitute(a[0], a[1], a[2])), "Replaces all occurrences of from with to.");

            Def(c, "diff,derivative,der", CA, "Derivative", "f,var:hold=x,order:nnint=1", (cc, a) =>
            {
                string v = SymName(a[1]);
                int n = SmallInt(cc.Ev.Eval(a[2]), "diff");
                var r = a[0];
                for (int i = 0; i < n; i++) r = Calculus.Diff(r, v);
                return cc.Ev.Eval(r);
            }, "Derivative of f with respect to var.");
            Def(c, "integrate,integral,∫", CA, "Integral", "f:hold,var:hold=x,lower?,upper?", (cc, a) => Integral(cc, a), "Antiderivative, or definite integral when limits are given.");
            Def(c, "nintegrate", CA, "Numerical Integral", "f:hold,lower,upper,var:hold=x", (cc, a) =>
            {
                string v = SymName(a[3]);
                var f = cc.Ev.Eval(a[0]);
                var r = Calculus.NumIntegrate(x => cc.Ev.EvalDouble(f, v, x), D(a[1], "nintegrate"), D(a[2], "nintegrate"), out _);
                return Num(r);
            });
            Def(c, "limit,lim", CA, "Limit", "f:hold,a,var:hold=x,direction:int=0", (cc, a) =>
            {
                var f = cc.Ev.Eval(a[0]);
                return Calculus.Limit(f, SymName(a[2]), a[1], SmallInt(a[3], "limit"));
            }, "Limit of f as var approaches a (direction −1 or 1 for one-sided).");
            Def(c, "extremum,extrema", CA, "Extrema", "f,var:hold=x", (cc, a) =>
            {
                string v = SymName(a[1]);
                var d = Calculus.Diff(a[0], v);
                return MatE.Vector(Solver.Solve(cc, new CmpE(CmpOp.Eq, d, Expr.Zero), v));
            }, "Points where the derivative is zero.");
            Def(c, "taylor", CA, "Taylor Polynomial", "f,var:hold=x,point=0,order:nnint=5", (cc, a) =>
            {
                string v = SymName(a[1]);
                int n = SmallInt(a[3], "taylor");
                var X = new SymE(v);
                var terms = new List<Expr>();
                var d = a[0];
                for (int k = 0; k <= n; k++)
                {
                    var val = cc.Ev.EvalWith(d, v, a[2]);
                    terms.Add(Simp.Mul(Simp.Div(val, Num(Primes.Factorial(k))), Simp.Pow(Simp.Sub(X, a[2]), Expr.N(k))));
                    d = Calculus.Diff(d, v);
                }
                return Simp.Expand(Sum(terms));
            });

            Def(c, "if", LG, "If", "condition,then:hold,else:hold=0", (cc, a) =>
            {
                var t = Evaluator.Truth(a[0]);
                if (t == null) throw new CalcException("if() condition could not be determined");
                return cc.Ev.Eval(t.Value ? a[1] : a[2]);
            }, "Returns then if condition is true, otherwise else.");
            Def(c, "and", LG, "Logical AND", "a,b,...", (cc, a) => cc.Ev.Eval(new LogicE(LogicOp.And, a.ToList())));
            Def(c, "or", LG, "Logical OR", "a,b,...", (cc, a) => cc.Ev.Eval(new LogicE(LogicOp.Or, a.ToList())));
            Def(c, "not", LG, "Logical NOT", "a", (cc, a) => cc.Ev.Eval(new LogicE(LogicOp.Not, a.ToList())));
            Def(c, "xor", LG, "Exclusive OR", "a,b", (cc, a) =>
            {
                if (IsI(a[0], out var x) && IsI(a[1], out var y)) return Num(x ^ y);
                return cc.Ev.Eval(new LogicE(LogicOp.Xor, a.ToList()));
            }, "Bitwise exclusive or for integers.");
            Def(c, "for", LG, "For Loop", "start,var:hold,condition:hold,update:hold,expression:hold,initial=0,result:hold=y", (cc, a) =>
            {
                string v = SymName(a[1]); string res = SymName(a[6], "y");
                var cur = a[0]; var acc = a[5];
                for (int i = 0; i < 1000000; i++)
                {
                    cc.CheckAbort();
                    cc.Ev.Bindings[v] = cur; cc.Ev.Bindings[res] = acc;
                    try
                    {
                        var cond = Evaluator.Truth(cc.Ev.Eval(a[2]));
                        if (cond != true) break;
                        acc = cc.Ev.Eval(a[4]);
                        cc.Ev.Bindings[res] = acc;
                        cur = cc.Ev.Eval(a[3]);
                    }
                    finally { cc.Ev.Bindings.Remove(v); cc.Ev.Bindings.Remove(res); }
                }
                return acc;
            }, "for(start, x, condition, update, expression, initial, y)");

            Def(c, "bitand", BT, "Bitwise AND", "a:int,b:int", (cc, a) => IsI(a[0], out var x) && IsI(a[1], out var y) ? Num(x & y) : null);
            Def(c, "bitor", BT, "Bitwise OR", "a:int,b:int", (cc, a) => IsI(a[0], out var x) && IsI(a[1], out var y) ? Num(x | y) : null);
            Def(c, "bitxor", BT, "Bitwise XOR", "a:int,b:int", (cc, a) => IsI(a[0], out var x) && IsI(a[1], out var y) ? Num(x ^ y) : null);
            Def(c, "bitnot", BT, "Bitwise NOT", "a:int", (cc, a) => IsI(a[0], out var x) ? Num(-x - 1) : null);
            Def(c, "bitcmp", BT, "Bitwise Complement", "a:int,bits:pint=8,signed:bool=0", (cc, a) =>
            {
                var x = Int(a[0], "bitcmp"); int bits = SmallInt(a[1], "bitcmp");
                var mask = (BigInteger.One << bits) - 1;
                var r = (~x) & mask;
                if (!a[2].IsZero && (r >> (bits - 1)) == 1) r -= BigInteger.One << bits;
                return Num(r);
            });
            Def(c, "shift", BT, "Bit Shift", "a:int,steps:int", (cc, a) =>
            {
                var x = Int(a[0], "shift"); int s = SmallInt(a[1], "shift");
                return Num(s >= 0 ? x << s : x >> -s);
            }, "Shifts left for positive steps, right for negative.");
            Def(c, "circshift,rot,bitrot", BT, "Circular Shift", "a:int,steps:int,bits:pint=8", (cc, a) =>
            {
                var x = Int(a[0], "circshift"); int s = SmallInt(a[1], "circshift"), bits = SmallInt(a[2], "circshift");
                var mask = (BigInteger.One << bits) - 1;
                x &= mask;
                s = ((s % bits) + bits) % bits;
                return Num(((x << s) | (x >> (bits - s))) & mask);
            });
            Def(c, "bitget,getbit", BT, "Get Bit", "a:int,bit:pint", (cc, a) => Num((Int(a[0], "bitget") >> (SmallInt(a[1], "bitget") - 1)) & 1));
            Def(c, "bitset,setbit", BT, "Set Bit", "a:int,bit:pint,value:bool=1", (cc, a) =>
            {
                var x = Int(a[0], "bitset"); var m = BigInteger.One << (SmallInt(a[1], "bitset") - 1);
                return Num(a[2].IsZero ? x & ~m : x | m);
            });
            Def(c, "bitcount,popcount", BT, "Count Set Bits", "a:int", (cc, a) =>
            {
                var x = BigInteger.Abs(Int(a[0], "bitcount")); int n = 0;
                while (!x.IsZero) { if (!x.IsEven) n++; x >>= 1; }
                return Expr.N(n);
            });

            Def(c, "base", NB, "Number Base Conversion", "value,base:int", (cc, a) =>
            {
                int b = SmallInt(a[1], "base");
                if (a[0] is StrE s) return Num(ParseInBase(s.S, b));
                if (a[0] is SymE sy) return Num(ParseInBase(sy.Name, b));
                if (a[0] is NumE n && n.V.IsReal) return Num(ParseInBase(Formatter.PlainText(a[0]), b));
                throw new CalcException("base() requires text or digits");
            }, "Interprets the digits in the given base.");
            Def(c, "bin", NB, "Binary Number", "digits", (cc, a) => Num(ParseInBase(DigitText(a[0]), 2)));
            Def(c, "oct", NB, "Octal Number", "digits", (cc, a) => Num(ParseInBase(DigitText(a[0]), 8)));
            Def(c, "hex", NB, "Hexadecimal Number", "digits", (cc, a) => Num(ParseInBase(DigitText(a[0]), 16)));
            Def(c, "roman", NB, "Roman Numerals", "numerals", (cc, a) => Num(FromRoman(DigitText(a[0]))));
            Def(c, "bijective", NB, "Bijective Base-26", "letters", (cc, a) =>
            {
                BigInteger v = 0;
                foreach (char ch in DigitText(a[0]).ToUpperInvariant()) { if (ch < 'A' || ch > 'Z') throw new CalcException("Invalid letter"); v = v * 26 + (ch - 'A' + 1); }
                return Num(v);
            });
            Def(c, "bcd", NB, "Binary-Coded Decimal", "bits", (cc, a) =>
            {
                var s = DigitText(a[0]).Replace(" ", "");
                if (s.Length % 4 != 0) s = s.PadLeft((s.Length / 4 + 1) * 4, '0');
                var sb = new StringBuilder();
                for (int i = 0; i < s.Length; i += 4) { int d = Convert.ToInt32(s.Substring(i, 4), 2); if (d > 9) throw new CalcException("Invalid BCD digit"); sb.Append(d); }
                return Num(Real.Parse(sb.ToString()));
            });
            Def(c, "float,fp", NB, "Floating Point Value", "bits_text,bits:pint=32", (cc, a) => Num(FloatFormats.Decode(DigitText(a[0]), FloatFormats.ByBits(SmallInt(a[1], "float")))), "Decodes an IEEE 754 binary representation.");
            Def(c, "floaterror", NB, "Floating Point Error", "x:real,bits:pint=32", (cc, a) =>
            {
                var x = ((NumE)a[0]).V.Re;
                var fmt = FloatFormats.ByBits(SmallInt(a[1], "floaterror"));
                var bits = FloatFormats.Encode(x, fmt);
                var back = FloatFormats.Decode(FloatFormats.ToBits(bits, fmt), fmt);
                return Num(back - x);
            });

            Def(c, "len,length", UT, "Length", "x", (cc, a) => a[0] is StrE s ? Expr.N(s.S.Length) : a[0] is MatE m ? Expr.N(m.All.Count()) : Expr.One);
            Def(c, "concatenate,concat", UT, "Concatenate Text", "a,b,...", (cc, a) => new StrE(string.Concat(a.Select(Str))));
            Def(c, "char,chr", UT, "Character", "code:pint", (cc, a) => new StrE(char.ConvertFromUtf32(SmallInt(a[0], "char"))));
            Def(c, "code,ascii,ord", UT, "Character Code", "text", (cc, a) =>
            {
                var s = Str(a[0]);
                if (s.Length == 0) throw new CalcException("Empty text");
                if (s.Length == 1 || char.IsSurrogatePair(s, 0) && s.Length == 2) return Expr.N(char.ConvertToUtf32(s, 0));
                var codes = new List<Expr>();
                for (int i = 0; i < s.Length; i++) { codes.Add(Expr.N(char.ConvertToUtf32(s, i))); if (char.IsSurrogate(s[i])) i++; }
                return MatE.Vector(codes);
            });
            Def(c, "uppercase,upper", UT, "Upper Case", "text", (cc, a) => new StrE(Str(a[0]).ToUpperInvariant()));
            Def(c, "lowercase,lower", UT, "Lower Case", "text", (cc, a) => new StrE(Str(a[0]).ToLowerInvariant()));
            Def(c, "nounit,strip_units", UT, "Strip Units", "x", (cc, a) => Evaluator.StripUnits(a[0]));
            Def(c, "unit", UT, "Extract Unit", "x", (cc, a) => Simp.SplitUnitsOf(a[0]));
            Def(c, "value,unitvalue", UT, "Numerical Value", "x", (cc, a) => { Simp.SplitCoef(a[0], out var cf, out _); return Num(cf); });
            Def(c, "convert", UT, "Convert Units", "x,unit", (cc, a) => Units.Convert(a[0], Simp.SplitUnitsOf(a[1]).IsOne ? a[1] : Simp.SplitUnitsOf(a[1])));
            Def(c, "plot", UT, "Plot", "expression:hold,min=0,max=10", (cc, a) =>
            {
                if (cc.Preview) return null;
                PlotRequested?.Invoke(Formatter.PlainText(a[0]), D(a[1], "plot"), D(a[2], "plot"));
                return new StrE("Plot opened");
            }, "Opens the expression in the plot view.");
            Def(c, "isnan", UT, "Is Undefined", "x", (cc, a) => Expr.Zero);

            Def(c, "interval", IV, "Interval", "lower:real,upper:real", (cc, a) =>
            {
                if (!IsR(a[0], out var lo) || !IsR(a[1], out var hi)) return null;
                return lo.EqualsValue(hi) ? (Expr)Num(lo) : new IntvE(lo, hi);
            }, "Closed interval [lower, upper].");
            Def(c, "uncertainty", IV, "Value with Uncertainty", "value:real,error:real,relative:bool=0", (cc, a) =>
            {
                if (!IsR(a[0], out var v) || !IsR(a[1], out var err)) return null;
                if (a.Count > 2 && !a[2].IsZero) err = (v * err).Abs();
                return Intervals.FromMidRad(v, err);
            }, "value ± error.");
            Def(c, "lowerendpoint,lower_endpoint", IV, "Lower Endpoint", "x", (cc, a) => a[0] is IntvE iv ? Num(iv.Lo) : a[0]);
            Def(c, "upperendpoint,upper_endpoint", IV, "Upper Endpoint", "x", (cc, a) => a[0] is IntvE iv ? Num(iv.Hi) : a[0]);
            Def(c, "midpoint", IV, "Midpoint", "x", (cc, a) => a[0] is IntvE iv ? Num(iv.Mid) : a[0]);
            Def(c, "errorrange,radius", IV, "Interval Radius", "x", (cc, a) => a[0] is IntvE iv ? Num(iv.Rad) : Expr.Zero);

            Def(c, "now", DT, "Current Time", "", (cc, a) => new DateE(DateTime.Now, true));
            Def(c, "today", DT, "Today", "", (cc, a) => new DateE(DateTime.Today, false));
            Def(c, "tomorrow", DT, "Tomorrow", "", (cc, a) => new DateE(DateTime.Today.AddDays(1), false));
            Def(c, "yesterday", DT, "Yesterday", "", (cc, a) => new DateE(DateTime.Today.AddDays(-1), false));
            Def(c, "date", DT, "Date", "year:int,month:int=1,day:int=1", (cc, a) =>
            {
                if (a[0] is StrE s) { var d = ParseDate(s.S, out bool ht, out bool u); return new DateE(d, ht, u); }
                return new DateE(new DateTime(SmallInt(a[0], "date"), SmallInt(a[1], "date"), SmallInt(a[2], "date")), false);
            });
            Def(c, "datetime", DT, "Date and Time", "year:int,month:int=1,day:int=1,hour:int=0,minute:int=0,second:real=0", (cc, a) =>
            {
                var d = new DateTime(SmallInt(a[0], "datetime"), SmallInt(a[1], "datetime"), SmallInt(a[2], "datetime"), SmallInt(a[3], "datetime"), SmallInt(a[4], "datetime"), 0);
                return new DateE(d.AddSeconds(D(a[5], "datetime")), true);
            });
            Def(c, "year", DT, "Year", "date:date=today()", (cc, a) => Expr.N(Dates.Get(a[0]).Year));
            Def(c, "month", DT, "Month", "date:date=today()", (cc, a) => Expr.N(Dates.Get(a[0]).Month));
            Def(c, "day", DT, "Day of Month", "date:date=today()", (cc, a) => Expr.N(Dates.Get(a[0]).Day));
            Def(c, "hour", DT, "Hour", "date:date=now()", (cc, a) => Expr.N(Dates.Get(a[0]).Hour));
            Def(c, "minute", DT, "Minute", "date:date=now()", (cc, a) => Expr.N(Dates.Get(a[0]).Minute));
            Def(c, "second", DT, "Second", "date:date=now()", (cc, a) => Expr.N(Dates.Get(a[0]).Second));
            Def(c, "weekday", DT, "Day of Week", "date:date=today(),sunday_first:bool=0", (cc, a) =>
            {
                var d = Dates.Get(a[0]);
                int w = (int)d.DayOfWeek;
                return Expr.N(a[1].IsZero ? (w == 0 ? 7 : w) : w + 1);
            }, "1 = Monday … 7 = Sunday.");
            Def(c, "week", DT, "Week Number", "date:date=today()", (cc, a) => Expr.N(Dates.IsoWeek(Dates.Get(a[0]))), "ISO 8601 week number.");
            Def(c, "yearday", DT, "Day of Year", "date:date=today()", (cc, a) => Expr.N(Dates.Get(a[0]).DayOfYear));
            Def(c, "days", DT, "Days Between Dates", "date1:date,date2:date", (cc, a) => Num(Real.Make((Dates.Get(a[1]).Date - Dates.Get(a[0]).Date).Ticks, TimeSpan.TicksPerDay)));
            Def(c, "yearfrac", DT, "Year Fraction", "date1:date,date2:date", (cc, a) => Num(Real.Make((Dates.Get(a[1]) - Dates.Get(a[0])).Ticks, TimeSpan.TicksPerDay) / Real.Frac(3652425, 10000)));
            Def(c, "adddays", DT, "Add Days", "date:date,days:real", (cc, a) => Dates.AddSeconds((DateE)Dates.AsDate(a[0]), ((NumE)a[1]).V.Re * Real.Int(86400)));
            Def(c, "addmonths", DT, "Add Months", "date:date,months:int", (cc, a) => { var d = (DateE)Dates.AsDate(a[0]); return new DateE(d.T.AddMonths(SmallInt(a[1], "addmonths")), d.HasTime, d.Utc); });
            Def(c, "addyears", DT, "Add Years", "date:date,years:int", (cc, a) => { var d = (DateE)Dates.AsDate(a[0]); return new DateE(d.T.AddYears(SmallInt(a[1], "addyears")), d.HasTime, d.Utc); });
            Def(c, "timestamp,unixtime", DT, "Unix Timestamp", "date:date=now()", (cc, a) => Num(Real.Make((Dates.Get(a[0]).ToUniversalTime() - new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc)).Ticks, TimeSpan.TicksPerSecond)));
            Def(c, "stamptodate,fromunix", DT, "Timestamp to Date", "timestamp:real", (cc, a) => new DateE(new DateTime(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc).AddSeconds(D(a[0], "stamptodate")).ToLocalTime(), true));
            Def(c, "lunarphase", DT, "Lunar Phase", "date:date=now()", (cc, a) => Num(Dates.LunarPhase(Dates.Get(a[0]))), "0 = new moon, 0.5 = full moon.");
            Def(c, "nextlunarphase", DT, "Next Lunar Phase", "phase:real=0.5,date:date=now()", (cc, a) => new DateE(Dates.NextLunarPhase(Dates.Get(a[1]), D(a[0], "nextlunarphase")), true));
            Def(c, "easter", DT, "Easter Sunday", "year:int", (cc, a) => new DateE(Dates.Easter(SmallInt(a[0], "easter")), false));
            Def(c, "isleapyear,leapyear", DT, "Is Leap Year", "year:int", (cc, a) => Bool(DateTime.IsLeapYear(SmallInt(a[0], "isleapyear"))));
            Def(c, "daysinmonth", DT, "Days in Month", "year:int,month:int", (cc, a) => Expr.N(DateTime.DaysInMonth(SmallInt(a[0], "daysinmonth"), SmallInt(a[1], "daysinmonth"))));

            Def(c, "pv", EC, "Present Value", "rate:real,nper:real,pmt:real,fv:real=0,type:bool=0", (cc, a) => Fin(cc, a, (r, n, p, f, t) => r == 0 ? -(f + p * n) : -(f + p * (1 + r * t) * (Math.Pow(1 + r, n) - 1) / r) / Math.Pow(1 + r, n)));
            Def(c, "fv", EC, "Future Value", "rate:real,nper:real,pmt:real,pv:real=0,type:bool=0", (cc, a) => Fin(cc, a, (r, n, p, v, t) => r == 0 ? -(v + p * n) : -(v * Math.Pow(1 + r, n) + p * (1 + r * t) * (Math.Pow(1 + r, n) - 1) / r)));
            Def(c, "pmt", EC, "Payment", "rate:real,nper:real,pv:real,fv:real=0,type:bool=0", (cc, a) => Fin(cc, a, (r, n, v, f, t) => r == 0 ? -(v + f) / n : -(r * (f + v * Math.Pow(1 + r, n))) / ((1 + r * t) * (Math.Pow(1 + r, n) - 1))));
            Def(c, "nper", EC, "Number of Periods", "rate:real,pmt:real,pv:real,fv:real=0,type:bool=0", (cc, a) => Fin(cc, a, (r, p, v, f, t) => r == 0 ? -(v + f) / p : Math.Log((p * (1 + r * t) - f * r) / (p * (1 + r * t) + v * r)) / Math.Log(1 + r)));
            Def(c, "rate", EC, "Interest Rate", "nper:real,pmt:real,pv:real,fv:real=0,type:bool=0,guess:real=0.1", (cc, a) =>
            {
                if (!a.All(z => z is NumE)) return null;
                double n = D(a[0], "rate"), p = D(a[1], "rate"), v = D(a[2], "rate"), f = D(a[3], "rate"), t = D(a[4], "rate");
                Func<double, double> g = r => Math.Abs(r) < 1e-12 ? v + p * n + f : v * Math.Pow(1 + r, n) + p * (1 + r * t) * (Math.Pow(1 + r, n) - 1) / r + f;
                var res = Solver.Newton(g, D(a[5], "rate"));
                if (double.IsNaN(res)) throw new CalcException("rate() did not converge");
                return Num(res);
            });
            Def(c, "npv", EC, "Net Present Value", "rate:real,values:vec", (cc, a) =>
            {
                var vals = Items(a.Skip(1).ToList());
                return Sum(vals.Select((v, i) => Simp.Div(v, Simp.Pow(Simp.Add(Expr.One, a[0]), Expr.N(i + 1)))).ToList());
            });
            Def(c, "irr", EC, "Internal Rate of Return", "values:vec,guess:real=0.1", (cc, a) =>
            {
                var vals = Reals(new List<Expr> { a[0] }, "irr").Select(r => r.ToDouble()).ToList();
                Func<double, double> g = r => vals.Select((v, i) => v / Math.Pow(1 + r, i)).Sum();
                var res = Solver.Newton(g, D(a[1], "irr"));
                if (double.IsNaN(res)) throw new CalcException("irr() did not converge");
                return Num(res);
            });
            Def(c, "ipmt", EC, "Interest Payment", "rate:real,per:real,nper:real,pv:real,fv:real=0,type:bool=0", (cc, a) =>
            {
                if (!a.All(z => z is NumE)) return null;
                double r = D(a[0], "ipmt"), per = D(a[1], "ipmt"), n = D(a[2], "ipmt"), v = D(a[3], "ipmt"), f = D(a[4], "ipmt"); int t = (int)D(a[5], "ipmt");
                double p = -(r * (f + v * Math.Pow(1 + r, n))) / ((1 + r * t) * (Math.Pow(1 + r, n) - 1));
                double fvBefore = -(v * Math.Pow(1 + r, per - 1) + p * (1 + r * t) * (Math.Pow(1 + r, per - 1) - 1) / r);
                double ip = fvBefore * r;
                if (t == 1) ip = per == 1 ? 0 : (fvBefore - p) * r;
                return Num(ip);
            });
            Def(c, "ppmt", EC, "Principal Payment", "rate:real,per:real,nper:real,pv:real,fv:real=0,type:bool=0", (cc, a) =>
            {
                var p = cc.Ev.Call("pmt", a[0], a[2], a[3], a[4], a[5]);
                var ip = cc.Ev.Call("ipmt", a.ToArray());
                return Simp.Sub(p, ip);
            });
            Def(c, "sln", EC, "Straight Line Depreciation", "cost:real,salvage:real,life:real", (cc, a) => Simp.Div(Simp.Sub(a[0], a[1]), a[2]));
            Def(c, "syd", EC, "Sum-of-Years Depreciation", "cost:real,salvage:real,life:real,period:real", (cc, a) =>
                Simp.Div(Simp.Mul(Simp.Sub(a[0], a[1]), Simp.Add(Simp.Sub(a[2], a[3]), Expr.One), Expr.Two), Simp.Mul(a[2], Simp.Add(a[2], Expr.One))));
            Def(c, "ddb", EC, "Double Declining Balance", "cost:real,salvage:real,life:real,period:real,factor:real=2", (cc, a) =>
            {
                if (!a.All(z => z is NumE)) return null;
                double cost = D(a[0], "ddb"), salv = D(a[1], "ddb"), life = D(a[2], "ddb"), per = D(a[3], "ddb"), fac = D(a[4], "ddb");
                double book = cost, dep = 0;
                for (int i = 1; i <= per; i++) { dep = Math.Min(book * fac / life, Math.Max(0, book - salv)); book -= dep; }
                return Num(dep);
            });
            Def(c, "effect", EC, "Effective Annual Rate", "nominal:real,periods:pint", (cc, a) => Simp.Sub(Simp.Pow(Simp.Add(Expr.One, Simp.Div(a[0], a[1])), a[1]), Expr.One));
            Def(c, "nominal", EC, "Nominal Annual Rate", "effective:real,periods:pint", (cc, a) => Simp.Mul(a[1], Simp.Sub(Simp.Pow(Simp.Add(Expr.One, a[0]), Simp.Pow(a[1], Expr.MinusOne)), Expr.One)));
            Def(c, "compound", EC, "Compound Interest", "principal:real,rate:real,years:real,periods:pint=1", (cc, a) => Simp.Mul(a[0], Simp.Pow(Simp.Add(Expr.One, Simp.Div(a[1], a[3])), Simp.Mul(a[3], a[2]))));

            Def(c, "area_circle,circlearea", GE, "Area of Circle", "radius", (cc, a) => Simp.Mul(Pi, Simp.Pow(a[0], Expr.Two)));
            Def(c, "circumference", GE, "Circumference of Circle", "radius", (cc, a) => Simp.Mul(Expr.Two, Pi, a[0]));
            Def(c, "area_sector", GE, "Area of Circle Sector", "radius,angle:angle", (cc, a) => Simp.Mul(Expr.Half, Simp.Pow(a[0], Expr.Two), ToRadians(a[1])));
            Def(c, "area_ellipse", GE, "Area of Ellipse", "a,b", (cc, a) => Simp.Mul(Pi, a[0], a[1]));
            Def(c, "area_rectangle", GE, "Area of Rectangle", "length,width", (cc, a) => Simp.Mul(a[0], a[1]));
            Def(c, "perimeter_rectangle", GE, "Perimeter of Rectangle", "length,width", (cc, a) => Simp.Mul(Expr.Two, Simp.Add(a[0], a[1])));
            Def(c, "area_triangle", GE, "Area of Triangle", "base,height", (cc, a) => Simp.Mul(Expr.Half, a[0], a[1]));
            Def(c, "heron", GE, "Area of Triangle (Heron)", "a,b,c", (cc, a) =>
            {
                var s = Simp.Div(Simp.Add(a[0], a[1], a[2]), Expr.Two);
                return Simp.Pow(Simp.Mul(s, Simp.Sub(s, a[0]), Simp.Sub(s, a[1]), Simp.Sub(s, a[2])), Expr.Half);
            });
            Def(c, "area_trapezoid", GE, "Area of Trapezoid", "a,b,height", (cc, a) => Simp.Mul(Expr.Half, Simp.Add(a[0], a[1]), a[2]));
            Def(c, "area_parallelogram", GE, "Area of Parallelogram", "base,height", (cc, a) => Simp.Mul(a[0], a[1]));
            Def(c, "area_polygon", GE, "Area of Regular Polygon", "sides,length", (cc, a) => Simp.Div(Simp.Mul(a[0], Simp.Pow(a[1], Expr.Two)), Simp.Mul(Expr.N(4), cc.Ev.Eval(new CallE(cc.Fn("tan"), FromRadians(Simp.Div(Pi, a[0])))))));
            Def(c, "volume_sphere", GE, "Volume of Sphere", "radius", (cc, a) => Simp.Mul(Expr.N(Real.Frac(4, 3)), Pi, Simp.Pow(a[0], Expr.N(3))));
            Def(c, "area_sphere", GE, "Surface Area of Sphere", "radius", (cc, a) => Simp.Mul(Expr.N(4), Pi, Simp.Pow(a[0], Expr.Two)));
            Def(c, "volume_cylinder", GE, "Volume of Cylinder", "radius,height", (cc, a) => Simp.Mul(Pi, Simp.Pow(a[0], Expr.Two), a[1]));
            Def(c, "area_cylinder", GE, "Surface Area of Cylinder", "radius,height", (cc, a) => Simp.Mul(Expr.Two, Pi, a[0], Simp.Add(a[0], a[1])));
            Def(c, "volume_cone", GE, "Volume of Cone", "radius,height", (cc, a) => Simp.Mul(Expr.N(Real.Frac(1, 3)), Pi, Simp.Pow(a[0], Expr.Two), a[1]));
            Def(c, "area_cone", GE, "Surface Area of Cone", "radius,height", (cc, a) => Simp.Mul(Pi, a[0], Simp.Add(a[0], Simp.Pow(Simp.Add(Simp.Pow(a[0], Expr.Two), Simp.Pow(a[1], Expr.Two)), Expr.Half))));
            Def(c, "volume_box", GE, "Volume of Box", "length,width,height", (cc, a) => Simp.Mul(a[0], a[1], a[2]));
            Def(c, "area_box", GE, "Surface Area of Box", "length,width,height", (cc, a) => Simp.Mul(Expr.Two, Simp.Add(Simp.Mul(a[0], a[1]), Simp.Mul(a[1], a[2]), Simp.Mul(a[0], a[2]))));
            Def(c, "volume_cube", GE, "Volume of Cube", "side", (cc, a) => Simp.Pow(a[0], Expr.N(3)));
            Def(c, "area_cube", GE, "Surface Area of Cube", "side", (cc, a) => Simp.Mul(Expr.N(6), Simp.Pow(a[0], Expr.Two)));
            Def(c, "volume_pyramid", GE, "Volume of Pyramid", "base_area,height", (cc, a) => Simp.Mul(Expr.N(Real.Frac(1, 3)), a[0], a[1]));
            Def(c, "pythagoras", GE, "Pythagorean Theorem", "a,b", (cc, a) => Simp.Pow(Simp.Add(Simp.Pow(a[0], Expr.Two), Simp.Pow(a[1], Expr.Two)), Expr.Half));

            Def(c, "atom,element", "Data Sets", "Element Data", "element,property=\"\"", (cc, a) => Data.Lookup(cc, "atom", a[0], a.Count > 1 ? Str(a[1]) : ""), "Looks up a property of a chemical element, e.g. atom(Fe, weight).").RawArgs = true;
            Def(c, "planet", "Data Sets", "Planet Data", "planet,property=\"\"", (cc, a) => Data.Lookup(cc, "planet", a[0], a.Count > 1 ? Str(a[1]) : ""), "Looks up a property of a planet, e.g. planet(Mars, mass).").RawArgs = true;
        }

        // sums the head directly, then estimates the tail
        static double InfiniteSum(Calculator c, Expr term, string v, long start)
        {
            var save = c.ExactPhase; c.ExactPhase = false;
            try
            {
                Func<double, double> f = x => c.Ev.EvalDouble(term, v, x);
                const int head = 2000;
                double acc = 0;
                var partial = new List<double>();
                int signChanges = 0;
                double last = 0;
                for (long k = start; k < start + head; k++)
                {
                    c.CheckAbort();
                    double t = f(k);
                    if (double.IsNaN(t)) throw new CalcException("Undefined term in series");
                    if (k > start && Math.Sign(t) != Math.Sign(last) && t != 0) signChanges++;
                    last = t;
                    acc += t;
                    partial.Add(acc);
                    if (t == 0 && k > start + 50) return acc;
                    if (Math.Abs(t) < 1e-18 * Math.Max(1, Math.Abs(acc)) && k > start + 20) return acc;
                }
                if (signChanges > head / 2)
                {
                    var tailSums = partial.Skip(partial.Count - 24).ToList();
                    while (tailSums.Count > 1) tailSums = tailSums.Zip(tailSums.Skip(1), (p1, p2) => (p1 + p2) / 2).ToList();
                    return tailSums[0];
                }
                long n = start + head;
                double fn = f(n);
                if (Math.Abs(fn) > Math.Abs(f(n - 1)) * 1.0000001) throw new CalcException("The series does not converge");
                double tail = Calculus.NumIntegrate(f, n, double.PositiveInfinity, out _);
                double d1 = (f(n + 0.5) - f(n - 0.5));
                return acc + tail + fn / 2 - d1 / 12;
            }
            finally { c.ExactPhase = save; }
        }

        static Expr Hold(Calculator c, Expr e) => e is NumE || e is SymE ? e : new CallE(c.Fn("_hold"), e);

        public static Expr Unhold(Expr e) => e.Map(x => x is CallE ce && ce.F.Name == "_hold" ? ce.Args[0] : x);

        static Expr SeriesOp(Calculator c, List<Expr> a, bool sum)
        {
            string v = SymName(a[3]);
            var lo = c.Ev.Eval(a[1]); var hi = c.Ev.Eval(a[2]);
            if (IsI(lo, out var l) && hi is NumE hn && hn.V.Re.IsInfinite && sum)
            {
                return Num(InfiniteSum(c, c.Ev.Eval(a[0]), v, (long)l));
            }
            if (IsI(lo, out var from) && IsI(hi, out var to))
            {
                if (to - from > 1000000) throw new CalcException("Too many terms");
                var parts = new List<Expr>();
                for (var k = from; k <= to; k++)
                {
                    c.CheckAbort();
                    parts.Add(c.Ev.EvalWith(a[0], v, Num(k)));
                }
                return sum ? Simp.Add(parts) : Simp.Mul(parts);
            }
            if (sum)
            {
                var closed = Faulhaber(c, a[0], v, lo, hi);
                if (closed != null) return closed;
            }
            return null;
        }

        // closed form sum of a polynomial term
        static Expr Faulhaber(Calculator c, Expr term, string v, Expr lo, Expr hi)
        {
            var t = c.Ev.Eval(term);
            if (!Polynomials.TryCoefficients(t, v, out var cs)) return null;
            var n = new SymE("_n");
            var total = new List<Expr>();
            for (int p = 0; p < cs.Count; p++)
            {
                if (cs[p].IsZero) continue;
                var parts = new List<Expr>();
                for (int j = 0; j <= p; j++)
                {
                    var b = Primes.Bernoulli(j);
                    if (j == 1) b = -b;
                    parts.Add(Simp.Mul(Num(Real.Int(Primes.Binomial(p + 1, j)) * b), Simp.Pow(n, Expr.N(p + 1 - j))));
                }
                var sp = Simp.Div(Simp.Add(parts), Expr.N(p + 1));
                total.Add(Simp.Mul(cs[p], sp));
            }
            var S = Simp.Add(total);
            var upper = c.Ev.EvalWith(S, "_n", hi);
            var lower = c.Ev.EvalWith(S, "_n", Simp.Sub(lo, Expr.One));
            return Simp.Expand(Simp.Sub(upper, lower));
        }

        static bool IsName(Expr e) => e is SymE || e is StrE || e is VarE ve && ve.V.IsUnknown;

        static Expr Integral(Calculator c, List<Expr> a)
        {
            Expr varArg = a.Count > 1 ? a[1] : null, loArg = a.Count > 2 ? a[2] : null, hiArg = a.Count > 3 ? a[3] : null;
            // also accepts integrate(f, lower, upper, var)
            if (varArg != null && !IsName(varArg)) { var l = varArg; varArg = hiArg; hiArg = loArg; loArg = l; }
            string v = SymName(varArg);
            var f = c.Ev.Eval(a[0]);
            var lo = loArg != null ? c.Ev.Eval(loArg) : null;
            var hi = hiArg != null ? c.Ev.Eval(hiArg) : null;
            var anti = Calculus.Integrate(f, v);
            if (lo == null || hi == null)
            {
                if (anti == null) throw new CalcException("Unable to find an antiderivative");
                return anti;
            }
            if (anti != null && !(lo is NumE ln && ln.V.IsInfinite) && !(hi is NumE hn && hn.V.IsInfinite))
            {
                try
                {
                    var r = Simp.Sub(c.Ev.EvalWith(anti, v, hi), c.Ev.EvalWith(anti, v, lo));
                    if (!r.Any(z => z is CallE ce && (ce.F.Name == "abs" || ce.F.Name == "ln") && z.HasSymbols)) return r;
                }
                catch (CalcException) { }
            }
            double dlo = lo is NumE l2 ? l2.V.Re.ToDouble() : D(c.Ev.Eval(Calculator.ToApprox(lo)), "integrate");
            double dhi = hi is NumE h2 ? h2.V.Re.ToDouble() : D(c.Ev.Eval(Calculator.ToApprox(hi)), "integrate");
            if (lo.HasSymbols || hi.HasSymbols) return null;
            var val = Calculus.NumIntegrate(x => c.Ev.EvalDouble(f, v, x), dlo, dhi, out _);
            return Num(val);
        }

        static Expr Variance(List<Expr> l, bool sample)
        {
            int n = l.Count;
            if (n < (sample ? 2 : 1)) throw new CalcException("Not enough values");
            var mean = Simp.Div(Simp.Add(l), Expr.N(n));
            var sq = l.Select(x => Simp.Pow(Simp.Sub(x, mean), Expr.Two)).ToList();
            return Simp.Div(Simp.Expand(Simp.Add(sq)), Expr.N(sample ? n - 1 : n));
        }

        static Expr QuantityExtreme(Calculator c, List<Expr> l, int dir)
        {
            Expr best = null; Real bestV = null;
            foreach (var e in l)
            {
                if (!Units.TryQuantity(e, out var v, out _, out var rest) || !rest.IsOne || !v.IsReal) return null;
                if (bestV == null || (dir > 0 ? v.Re > bestV : v.Re < bestV)) { best = e; bestV = v.Re; }
            }
            return best;
        }

        static BigInteger RandomBig(BigInteger max)
        {
            if (max <= 0) return 0;
            if (max < int.MaxValue) return Real.Rng.Next((int)max);
            var bytes = max.ToByteArray();
            Real.Rng.NextBytes(bytes);
            bytes[bytes.Length - 1] &= 0x7F;
            return new BigInteger(bytes) % max;
        }

        static string DigitText(Expr e)
        {
            if (e is StrE s) return s.S;
            if (e is SymE sy) return sy.Name;
            if (e is NumE n && n.V.IsReal && n.V.Re.IsInt) return n.V.Re.N.ToString(CultureInfo.InvariantCulture);
            return Formatter.PlainText(e);
        }

        public static Real ParseInBase(string s, int b)
        {
            s = s.Trim().Replace(" ", "").Replace("_", "");
            bool neg = s.StartsWith("-") || s.StartsWith("−");
            if (neg) s = s.Substring(1);
            if (b == 16 && s.StartsWith("0x", StringComparison.OrdinalIgnoreCase)) s = s.Substring(2);
            if (b == 2 && s.StartsWith("0b", StringComparison.OrdinalIgnoreCase)) s = s.Substring(2);
            if (b == 8 && s.StartsWith("0o", StringComparison.OrdinalIgnoreCase)) s = s.Substring(2);
            if (b < 2 || b > 36) throw new CalcException("Base must be between 2 and 36");
            var r = Real.Parse(s, b);
            return neg ? -r : r;
        }

        static Expr Fin(Calculator c, List<Expr> a, Func<double, double, double, double, double, double> f)
        {
            if (!a.All(z => z is NumE n && n.V.IsReal)) return null;
            var d = a.Select(z => ((NumE)z).V.Re.ToDouble()).ToList();
            while (d.Count < 5) d.Add(0);
            return Num(f(d[0], d[1], d[2], d[3], d[4]));
        }
    }
}
