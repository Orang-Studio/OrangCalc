using System;
using System.Collections.Generic;
using System.Linq;
using System.Numerics;

namespace OrangCalc.Engine
{
    public static class Matrices
    {
        public static MatE Identity(int n)
        {
            var rows = new List<List<Expr>>();
            for (int i = 0; i < n; i++)
            {
                var r = new List<Expr>();
                for (int j = 0; j < n; j++) r.Add(i == j ? Expr.One : Expr.Zero);
                rows.Add(r);
            }
            return new MatE(rows, false);
        }

        public static MatE Create(int r, int c, Func<int, int, Expr> f)
        {
            var rows = new List<List<Expr>>();
            for (int i = 0; i < r; i++)
            {
                var row = new List<Expr>();
                for (int j = 0; j < c; j++) row.Add(f(i, j));
                rows.Add(row);
            }
            return new MatE(rows, false);
        }

        public static MatE MapElements(MatE m, Func<Expr, Expr> f) =>
            new MatE(m.Rows.Select(r => r.Select(f).ToList()).ToList(), m.IsVector);

        public static MatE AsMatrix(Expr e)
        {
            if (e is MatE m) return m;
            return new MatE(new List<List<Expr>> { new List<Expr> { e } }, false);
        }

        public static Expr Product(MatE a, MatE b)
        {
            if (a.IsVector && b.IsVector)
            {
                if (a.ColCount != b.ColCount) throw new CalcException("Vectors must have the same length");
                return Dot(a, b);
            }
            if (a.IsVector && !b.IsVector && a.ColCount != b.RowCount && b.ColCount == a.ColCount)
                a = new MatE(a.Rows, false);
            if (b.IsVector && !a.IsVector && a.ColCount == b.ColCount)
                b = Transpose(b);
            if (a.ColCount != b.RowCount) throw new CalcException("Matrix dimensions do not match for multiplication (" + a.RowCount + "×" + a.ColCount + " and " + b.RowCount + "×" + b.ColCount + ")");
            var rows = new List<List<Expr>>();
            for (int i = 0; i < a.RowCount; i++)
            {
                var r = new List<Expr>();
                for (int j = 0; j < b.ColCount; j++)
                {
                    var terms = new List<Expr>();
                    for (int k = 0; k < a.ColCount; k++) terms.Add(Simp.Mul(a.Rows[i][k], b.Rows[k][j]));
                    r.Add(Simp.Add(terms));
                }
                rows.Add(r);
            }
            if (rows.Count == 1 && rows[0].Count == 1 && false) return rows[0][0];
            return new MatE(rows, false);
        }

        public static Expr Dot(MatE a, MatE b)
        {
            var x = a.Items; var y = b.Items;
            if (x.Count != y.Count) throw new CalcException("Vectors must have the same length");
            return Simp.Add(x.Zip(y, (p, q) => Simp.Mul(p, q)).ToList());
        }

        public static MatE Cross(MatE a, MatE b)
        {
            var x = a.Items; var y = b.Items;
            if (x.Count != 3 || y.Count != 3) throw new CalcException("Cross product requires two vectors with three elements");
            return MatE.Vector(new List<Expr>
            {
                Simp.Sub(Simp.Mul(x[1], y[2]), Simp.Mul(x[2], y[1])),
                Simp.Sub(Simp.Mul(x[2], y[0]), Simp.Mul(x[0], y[2])),
                Simp.Sub(Simp.Mul(x[0], y[1]), Simp.Mul(x[1], y[0]))
            });
        }

        public static Expr Power(MatE m, BigInteger n)
        {
            if (m.RowCount != m.ColCount) throw new CalcException("Matrix must be square");
            if (n.Sign < 0) { m = Inverse(m); n = -n; }
            Expr result = Identity(m.RowCount), b = m;
            while (!n.IsZero)
            {
                if (!n.IsEven) result = Product((MatE)result, (MatE)b);
                n >>= 1;
                if (!n.IsZero) b = Product((MatE)b, (MatE)b);
            }
            return result;
        }

        public static MatE Transpose(MatE m)
        {
            if (m.IsVector) return Create(m.ColCount, 1, (i, j) => m.Rows[0][i]);
            return Create(m.ColCount, m.RowCount, (i, j) => m.Rows[j][i]);
        }

        static Expr[,] ToArray(MatE m)
        {
            var a = new Expr[m.RowCount, m.ColCount];
            for (int i = 0; i < m.RowCount; i++) for (int j = 0; j < m.ColCount; j++) a[i, j] = m.Rows[i][j];
            return a;
        }

        static MatE FromArray(Expr[,] a)
        {
            return Create(a.GetLength(0), a.GetLength(1), (i, j) => a[i, j]);
        }

        static bool AllNumeric(MatE m) => m.All.All(e => e is NumE);

        public static Expr Determinant(MatE m)
        {
            int n = m.RowCount;
            if (n != m.ColCount) throw new CalcException("Determinant requires a square matrix");
            if (n == 0) return Expr.One;
            if (n == 1) return m.Rows[0][0];
            if (n == 2) return Simp.Sub(Simp.Mul(m.Rows[0][0], m.Rows[1][1]), Simp.Mul(m.Rows[0][1], m.Rows[1][0]));
            if (!AllNumeric(m) && n <= 6)
            {
                var terms = new List<Expr>();
                for (int j = 0; j < n; j++)
                {
                    if (m.Rows[0][j].IsZero) continue;
                    var minor = Minor(m, 0, j);
                    var t = Simp.Mul(m.Rows[0][j], Determinant(minor));
                    terms.Add(j % 2 == 0 ? t : Simp.Neg(t));
                }
                return Simp.Add(terms);
            }
            var a = ToArray(m);
            Expr det = Expr.One;
            for (int c = 0; c < n; c++)
            {
                int piv = -1;
                for (int r = c; r < n; r++) if (!a[r, c].IsZero) { piv = r; break; }
                if (piv < 0) return Expr.Zero;
                if (piv != c)
                {
                    for (int k = 0; k < n; k++) { var t = a[c, k]; a[c, k] = a[piv, k]; a[piv, k] = t; }
                    det = Simp.Neg(det);
                }
                det = Simp.Mul(det, a[c, c]);
                for (int r = c + 1; r < n; r++)
                {
                    if (a[r, c].IsZero) continue;
                    var f = Simp.Div(a[r, c], a[c, c]);
                    for (int k = c; k < n; k++) a[r, k] = Simp.Sub(a[r, k], Simp.Mul(f, a[c, k]));
                }
            }
            return det;
        }

        public static MatE Minor(MatE m, int row, int col)
        {
            var rows = new List<List<Expr>>();
            for (int i = 0; i < m.RowCount; i++)
            {
                if (i == row) continue;
                var r = new List<Expr>();
                for (int j = 0; j < m.ColCount; j++) if (j != col) r.Add(m.Rows[i][j]);
                rows.Add(r);
            }
            return new MatE(rows, false);
        }

        public static Expr Cofactor(MatE m, int row, int col)
        {
            var d = Determinant(Minor(m, row, col));
            return (row + col) % 2 == 0 ? d : Simp.Neg(d);
        }

        public static MatE Adjugate(MatE m)
        {
            int n = m.RowCount;
            if (n != m.ColCount) throw new CalcException("Adjugate requires a square matrix");
            if (n == 1) return Identity(1);
            return Create(n, n, (i, j) => Cofactor(m, j, i));
        }

        public static MatE Inverse(MatE m)
        {
            int n = m.RowCount;
            if (n != m.ColCount) throw new CalcException("Inverse requires a square matrix");
            if (!AllNumeric(m) && n <= 4)
            {
                var det = Determinant(m);
                if (det.IsZero) throw new CalcException("The matrix is singular");
                var adj = Adjugate(m);
                return MapElements(adj, x => Simp.Div(x, det));
            }
            var a = new Expr[n, 2 * n];
            for (int i = 0; i < n; i++)
                for (int j = 0; j < 2 * n; j++)
                    a[i, j] = j < n ? m.Rows[i][j] : (j - n == i ? Expr.One : Expr.Zero);
            GaussJordan(a, n, n);
            for (int i = 0; i < n; i++) if (!a[i, i].IsOne) throw new CalcException("The matrix is singular");
            return Create(n, n, (i, j) => a[i, j + n]);
        }

        static bool IsZeroish(Expr e)
        {
            if (e.IsZero) return true;
            if (e is NumE n && n.V.Approx)
            {
                var c = n.V.ToComplex();
                return System.Numerics.Complex.Abs(c) < 1e-25;
            }
            return false;
        }

        // reduces augmented matrix in place, returns rank
        static int GaussJordan(Expr[,] a, int rows, int cols)
        {
            int rank = 0;
            int width = a.GetLength(1);
            for (int c = 0; c < cols && rank < rows; c++)
            {
                int piv = -1;
                for (int r = rank; r < rows; r++) if (!IsZeroish(a[r, c])) { piv = r; break; }
                if (piv < 0) continue;
                for (int k = 0; k < width; k++) { var t = a[rank, k]; a[rank, k] = a[piv, k]; a[piv, k] = t; }
                var pv = a[rank, c];
                for (int k = 0; k < width; k++) a[rank, k] = Simp.Div(a[rank, k], pv);
                for (int r = 0; r < rows; r++)
                {
                    if (r == rank || IsZeroish(a[r, c])) continue;
                    var f = a[r, c];
                    for (int k = 0; k < width; k++) a[r, k] = Simp.Sub(a[r, k], Simp.Mul(f, a[rank, k]));
                }
                rank++;
            }
            return rank;
        }

        public static MatE Rref(MatE m)
        {
            var a = ToArray(m);
            GaussJordan(a, m.RowCount, m.ColCount);
            return FromArray(a);
        }

        public static int Rank(MatE m)
        {
            var a = ToArray(m);
            return GaussJordan(a, m.RowCount, m.ColCount);
        }

        public static Expr Trace(MatE m)
        {
            if (m.RowCount != m.ColCount) throw new CalcException("Trace requires a square matrix");
            return Simp.Add(Enumerable.Range(0, m.RowCount).Select(i => m.Rows[i][i]).ToList());
        }

        // solves A x = b for x, A square
        public static List<Expr> Solve(MatE A, List<Expr> b)
        {
            int n = A.RowCount;
            var a = new Expr[n, A.ColCount + 1];
            for (int i = 0; i < n; i++)
            {
                for (int j = 0; j < A.ColCount; j++) a[i, j] = A.Rows[i][j];
                a[i, A.ColCount] = b[i];
            }
            int rank = GaussJordan(a, n, A.ColCount);
            if (rank < A.ColCount) throw new CalcException("The system has no unique solution");
            for (int i = rank; i < n; i++) if (!IsZeroish(a[i, A.ColCount])) throw new CalcException("The system has no solution");
            return Enumerable.Range(0, A.ColCount).Select(i => a[i, A.ColCount]).ToList();
        }

        public static MatE Kronecker(MatE a, MatE b)
        {
            return Create(a.RowCount * b.RowCount, a.ColCount * b.ColCount,
                (i, j) => Simp.Mul(a.Rows[i / b.RowCount][j / b.ColCount], b.Rows[i % b.RowCount][j % b.ColCount]));
        }

        public static MatE Hadamard(MatE a, MatE b)
        {
            if (a.RowCount != b.RowCount || a.ColCount != b.ColCount) throw new CalcException("Matrix dimensions do not match");
            return new MatE(a.Rows.Select((r, i) => r.Select((x, j) => Simp.Mul(x, b.Rows[i][j])).ToList()).ToList(), a.IsVector);
        }

        public static Expr Permanent(MatE m)
        {
            int n = m.RowCount;
            if (n != m.ColCount) throw new CalcException("Permanent requires a square matrix");
            if (n == 1) return m.Rows[0][0];
            var terms = new List<Expr>();
            for (int j = 0; j < n; j++) terms.Add(Simp.Mul(m.Rows[0][j], Permanent(Minor(m, 0, j))));
            return Simp.Add(terms);
        }

        // numeric eigenvalues via unshifted QR on complex doubles
        public static List<System.Numerics.Complex> EigenvaluesNumeric(double[,] a)
        {
            int n = a.GetLength(0);
            var h = new System.Numerics.Complex[n, n];
            for (int i = 0; i < n; i++) for (int j = 0; j < n; j++) h[i, j] = a[i, j];
            for (int iter = 0; iter < 500 * n; iter++)
            {
                var mu = h[n - 1, n - 1];
                for (int i = 0; i < n; i++) h[i, i] -= mu;
                QR(h, n, out var q, out var r);
                h = MatMulC(r, q, n);
                for (int i = 0; i < n; i++) h[i, i] += mu;
                double off = 0;
                for (int i = 1; i < n; i++) off += System.Numerics.Complex.Abs(h[i, i - 1]);
                if (off < 1e-14) break;
            }
            var res = new List<System.Numerics.Complex>();
            for (int i = 0; i < n; i++) res.Add(h[i, i]);
            return res;
        }

        static void QR(System.Numerics.Complex[,] a, int n, out System.Numerics.Complex[,] q, out System.Numerics.Complex[,] r)
        {
            q = new System.Numerics.Complex[n, n];
            r = new System.Numerics.Complex[n, n];
            var v = new System.Numerics.Complex[n, n];
            for (int i = 0; i < n; i++) for (int j = 0; j < n; j++) v[i, j] = a[i, j];
            for (int j = 0; j < n; j++)
            {
                for (int k = 0; k < j; k++)
                {
                    System.Numerics.Complex dot = 0;
                    for (int i = 0; i < n; i++) dot += System.Numerics.Complex.Conjugate(q[i, k]) * v[i, j];
                    r[k, j] = dot;
                    for (int i = 0; i < n; i++) v[i, j] -= dot * q[i, k];
                }
                double norm = 0;
                for (int i = 0; i < n; i++) norm += Math.Pow(System.Numerics.Complex.Abs(v[i, j]), 2);
                norm = Math.Sqrt(norm);
                r[j, j] = norm;
                for (int i = 0; i < n; i++) q[i, j] = norm < 1e-300 ? 0 : v[i, j] / norm;
            }
        }

        static System.Numerics.Complex[,] MatMulC(System.Numerics.Complex[,] a, System.Numerics.Complex[,] b, int n)
        {
            var c = new System.Numerics.Complex[n, n];
            for (int i = 0; i < n; i++)
                for (int j = 0; j < n; j++)
                {
                    System.Numerics.Complex s = 0;
                    for (int k = 0; k < n; k++) s += a[i, k] * b[k, j];
                    c[i, j] = s;
                }
            return c;
        }
    }
}
