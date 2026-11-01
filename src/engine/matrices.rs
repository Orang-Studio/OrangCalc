use super::algebra as simp;
use super::c64::{c64, C64};
use super::*;
use num_integer::Integer;
use num_traits::{Signed, Zero};

pub fn rows_of(e: &Expr) -> &Vec<Vec<Expr>> {
    match &e.e {
        E::Mat(r, _) => r,
        _ => throw("Expected a matrix"),
    }
}

pub fn is_vector(e: &Expr) -> bool {
    matches!(&e.e, E::Mat(_, true))
}

pub fn row_count(e: &Expr) -> usize {
    rows_of(e).len()
}

pub fn col_count(e: &Expr) -> usize {
    rows_of(e).first().map(|r| r.len()).unwrap_or(0)
}

pub fn items(e: &Expr) -> Vec<Expr> {
    match &e.e {
        E::Mat(r, true) => r[0].clone(),
        E::Mat(r, false) => r.iter().flatten().cloned().collect(),
        _ => vec![e.clone()],
    }
}

pub fn identity(n: usize) -> Expr {
    create(n, n, &mut |i, j| if i == j { one() } else { zero() })
}

pub fn create(r: usize, c: usize, f: &mut dyn FnMut(usize, usize) -> Expr) -> Expr {
    mat_e((0..r).map(|i| (0..c).map(|j| f(i, j)).collect()).collect(), false)
}

pub fn map_elements(m: &Expr, f: &mut dyn FnMut(Expr) -> Expr) -> Expr {
    let E::Mat(rows, v) = &m.e else { return f(m.clone()) };
    mat_e(rows.iter().map(|r| r.iter().map(|x| f(x.clone())).collect()).collect(), *v)
}

pub fn as_matrix(e: &Expr) -> Expr {
    if e.is_mat() {
        return e.clone();
    }
    mat_e(vec![vec![e.clone()]], false)
}

pub fn product(a: &Expr, b: &Expr) -> Expr {
    let (mut a, mut b) = (a.clone(), b.clone());
    if is_vector(&a) && is_vector(&b) {
        if col_count(&a) != col_count(&b) {
            throw("Vectors must have the same length");
        }
        return dot(&a, &b);
    }
    if is_vector(&a) && !is_vector(&b) && col_count(&a) != row_count(&b) && col_count(&b) == col_count(&a) {
        a = mat_e(rows_of(&a).clone(), false);
    }
    if is_vector(&b) && !is_vector(&a) && col_count(&a) == col_count(&b) {
        b = transpose(&b);
    }
    let (ar, br) = (rows_of(&a), rows_of(&b));
    let (an, ac, bn, bc) = (ar.len(), col_count(&a), br.len(), col_count(&b));
    if ac != bn {
        throw_s(format!("Matrix dimensions do not match for multiplication ({}×{} and {}×{})", an, ac, bn, bc));
    }
    let rows = (0..an).map(|i| (0..bc).map(|j| simp::add((0..ac).map(|k| simp::mul2(&ar[i][k], &br[k][j])).collect())).collect()).collect();
    mat_e(rows, false)
}

pub fn dot(a: &Expr, b: &Expr) -> Expr {
    let x = items(a);
    let y = items(b);
    if x.len() != y.len() {
        throw("Vectors must have the same length");
    }
    simp::add(x.iter().zip(y.iter()).map(|(p, q)| simp::mul2(p, q)).collect())
}

pub fn cross(a: &Expr, b: &Expr) -> Expr {
    let x = items(a);
    let y = items(b);
    if x.len() != 3 || y.len() != 3 {
        throw("Cross product requires two vectors with three elements");
    }
    let m = simp::mul2;
    vector_e(vec![simp::sub(&m(&x[1], &y[2]), &m(&x[2], &y[1])), simp::sub(&m(&x[2], &y[0]), &m(&x[0], &y[2])), simp::sub(&m(&x[0], &y[1]), &m(&x[1], &y[0]))])
}

pub fn power(m: &Expr, n: &BigInt) -> Expr {
    if row_count(m) != col_count(m) {
        throw("Matrix must be square");
    }
    let mut m = m.clone();
    let mut n = n.clone();
    if n.is_negative() {
        m = inverse(&m);
        n = -n;
    }
    let mut result = identity(row_count(&m));
    let mut b = m;
    while !n.is_zero() {
        if !n.is_even() {
            result = product(&result, &b);
        }
        n >>= 1;
        if !n.is_zero() {
            b = product(&b, &b);
        }
    }
    result
}

pub fn transpose(m: &Expr) -> Expr {
    let rows = rows_of(m);
    if is_vector(m) {
        return create(rows[0].len(), 1, &mut |i, _| rows[0][i].clone());
    }
    create(col_count(m), rows.len(), &mut |i, j| rows[j][i].clone())
}

fn to_array(m: &Expr) -> Vec<Vec<Expr>> {
    rows_of(m).clone()
}

fn all_numeric(m: &Expr) -> bool {
    rows_of(m).iter().flatten().all(|e| e.is_num())
}

pub fn determinant(m: &Expr) -> Expr {
    let rows = rows_of(m);
    let n = rows.len();
    if n != col_count(m) {
        throw("Determinant requires a square matrix");
    }
    if n == 0 {
        return one();
    }
    if n == 1 {
        return rows[0][0].clone();
    }
    if n == 2 {
        return simp::sub(&simp::mul2(&rows[0][0], &rows[1][1]), &simp::mul2(&rows[0][1], &rows[1][0]));
    }
    if !all_numeric(m) && n <= 6 {
        let mut terms = vec![];
        for j in 0..n {
            if rows[0][j].is_zero() {
                continue;
            }
            let t = simp::mul2(&rows[0][j], &determinant(&minor(m, 0, j)));
            terms.push(if j % 2 == 0 { t } else { simp::neg(&t) });
        }
        return simp::add(terms);
    }
    let mut a = to_array(m);
    let mut det = one();
    for c in 0..n {
        let Some(piv) = (c..n).find(|&r| !a[r][c].is_zero()) else { return zero() };
        if piv != c {
            a.swap(c, piv);
            det = simp::neg(&det);
        }
        det = simp::mul2(&det, &a[c][c]);
        for r in c + 1..n {
            if a[r][c].is_zero() {
                continue;
            }
            let f = simp::div(&a[r][c], &a[c][c]);
            for k in c..n {
                a[r][k] = simp::sub(&a[r][k], &simp::mul2(&f, &a[c][k]));
            }
        }
    }
    det
}

pub fn minor(m: &Expr, row: usize, col: usize) -> Expr {
    let rows = rows_of(m);
    let r = rows.iter().enumerate().filter(|(i, _)| *i != row).map(|(_, r)| r.iter().enumerate().filter(|(j, _)| *j != col).map(|(_, x)| x.clone()).collect()).collect();
    mat_e(r, false)
}

pub fn cofactor(m: &Expr, row: usize, col: usize) -> Expr {
    let d = determinant(&minor(m, row, col));
    if (row + col) % 2 == 0 {
        d
    } else {
        simp::neg(&d)
    }
}

pub fn adjugate(m: &Expr) -> Expr {
    let n = row_count(m);
    if n != col_count(m) {
        throw("Adjugate requires a square matrix");
    }
    if n == 1 {
        return identity(1);
    }
    create(n, n, &mut |i, j| cofactor(m, j, i))
}

pub fn inverse(m: &Expr) -> Expr {
    let n = row_count(m);
    if n != col_count(m) {
        throw("Inverse requires a square matrix");
    }
    if !all_numeric(m) && n <= 4 {
        let det = determinant(m);
        if det.is_zero() {
            throw("The matrix is singular");
        }
        return map_elements(&adjugate(m), &mut |x| simp::div(&x, &det));
    }
    let rows = rows_of(m);
    let mut a: Vec<Vec<Expr>> = (0..n).map(|i| (0..2 * n).map(|j| if j < n { rows[i][j].clone() } else if j - n == i { one() } else { zero() }).collect()).collect();
    gauss_jordan(&mut a, n, n);
    for (i, row) in a.iter().enumerate() {
        if !row[i].is_one() {
            throw("The matrix is singular");
        }
    }
    create(n, n, &mut |i, j| a[i][j + n].clone())
}

fn is_zeroish(e: &Expr) -> bool {
    if e.is_zero() {
        return true;
    }
    if let E::Num(n) = &e.e {
        if n.approx() {
            let (re, im) = n.to_c64();
            return re.hypot(im) < 1e-25;
        }
    }
    false
}

// reduces augmented matrix in place, returns rank
fn gauss_jordan(a: &mut [Vec<Expr>], rows: usize, cols: usize) -> usize {
    let mut rank = 0;
    let width = a.first().map(|r| r.len()).unwrap_or(0);
    let mut c = 0;
    while c < cols && rank < rows {
        let Some(piv) = (rank..rows).find(|&r| !is_zeroish(&a[r][c])) else {
            c += 1;
            continue;
        };
        a.swap(rank, piv);
        let pv = a[rank][c].clone();
        for k in 0..width {
            a[rank][k] = simp::div(&a[rank][k], &pv);
        }
        for r in 0..rows {
            if r == rank || is_zeroish(&a[r][c]) {
                continue;
            }
            let f = a[r][c].clone();
            for k in 0..width {
                a[r][k] = simp::sub(&a[r][k], &simp::mul2(&f, &a[rank][k]));
            }
        }
        rank += 1;
        c += 1;
    }
    rank
}

pub fn rref(m: &Expr) -> Expr {
    let mut a = to_array(m);
    gauss_jordan(&mut a, row_count(m), col_count(m));
    mat_e(a, false)
}

pub fn rank(m: &Expr) -> usize {
    let mut a = to_array(m);
    gauss_jordan(&mut a, row_count(m), col_count(m))
}

pub fn trace(m: &Expr) -> Expr {
    let rows = rows_of(m);
    if rows.len() != col_count(m) {
        throw("Trace requires a square matrix");
    }
    simp::add((0..rows.len()).map(|i| rows[i][i].clone()).collect())
}

// solves A x = b for x
pub fn solve(am: &Expr, b: &[Expr]) -> Vec<Expr> {
    let rows = rows_of(am);
    let n = rows.len();
    let cc = col_count(am);
    let mut a: Vec<Vec<Expr>> = (0..n)
        .map(|i| {
            let mut r = rows[i].clone();
            r.push(b[i].clone());
            r
        })
        .collect();
    let rank = gauss_jordan(&mut a, n, cc);
    if rank < cc {
        throw("The system has no unique solution");
    }
    for row in a.iter().skip(rank) {
        if !is_zeroish(&row[cc]) {
            throw("The system has no solution");
        }
    }
    (0..cc).map(|i| a[i][cc].clone()).collect()
}

pub fn kronecker(a: &Expr, b: &Expr) -> Expr {
    let (ar, br) = (rows_of(a), rows_of(b));
    let (bn, bc) = (br.len(), col_count(b));
    create(ar.len() * bn, col_count(a) * bc, &mut |i, j| simp::mul2(&ar[i / bn][j / bc], &br[i % bn][j % bc]))
}

pub fn hadamard(a: &Expr, b: &Expr) -> Expr {
    let (ar, br) = (rows_of(a), rows_of(b));
    if ar.len() != br.len() || col_count(a) != col_count(b) {
        throw("Matrix dimensions do not match");
    }
    mat_e(ar.iter().enumerate().map(|(i, r)| r.iter().enumerate().map(|(j, x)| simp::mul2(x, &br[i][j])).collect()).collect(), is_vector(a))
}

pub fn permanent(m: &Expr) -> Expr {
    let rows = rows_of(m);
    let n = rows.len();
    if n != col_count(m) {
        throw("Permanent requires a square matrix");
    }
    if n == 1 {
        return rows[0][0].clone();
    }
    simp::add((0..n).map(|j| simp::mul2(&rows[0][j], &permanent(&minor(m, 0, j)))).collect())
}

// numeric eigenvalues via shifted QR on complex doubles
pub fn eigenvalues_numeric(a: &[Vec<f64>]) -> Vec<C64> {
    let n = a.len();
    let mut h: Vec<Vec<C64>> = a.iter().map(|r| r.iter().map(|&x| c64(x, 0.0)).collect()).collect();
    for _ in 0..500 * n {
        let mu = h[n - 1][n - 1];
        for (i, row) in h.iter_mut().enumerate() {
            row[i] -= mu;
        }
        let (q, r) = qr(&h, n);
        h = mat_mul_c(&r, &q, n);
        for (i, row) in h.iter_mut().enumerate() {
            row[i] += mu;
        }
        let off: f64 = (1..n).map(|i| h[i][i - 1].abs()).sum();
        if off < 1e-14 {
            break;
        }
    }
    (0..n).map(|i| h[i][i]).collect()
}

fn qr(a: &[Vec<C64>], n: usize) -> (Vec<Vec<C64>>, Vec<Vec<C64>>) {
    let mut q = vec![vec![C64::default(); n]; n];
    let mut r = vec![vec![C64::default(); n]; n];
    let mut v: Vec<Vec<C64>> = a.to_vec();
    for j in 0..n {
        for k in 0..j {
            let mut dot = C64::default();
            for i in 0..n {
                dot += q[i][k].conj() * v[i][j];
            }
            r[k][j] = dot;
            for i in 0..n {
                let d = dot * q[i][k];
                v[i][j] -= d;
            }
        }
        let norm = (0..n).map(|i| v[i][j].abs().powi(2)).sum::<f64>().sqrt();
        r[j][j] = c64(norm, 0.0);
        for i in 0..n {
            q[i][j] = if norm < 1e-300 { C64::default() } else { v[i][j] / norm };
        }
    }
    (q, r)
}

fn mat_mul_c(a: &[Vec<C64>], b: &[Vec<C64>], n: usize) -> Vec<Vec<C64>> {
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let mut s = C64::default();
                    for k in 0..n {
                        s += a[i][k] * b[k][j];
                    }
                    s
                })
                .collect()
        })
        .collect()
}