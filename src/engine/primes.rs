use super::real::{big, bit_len};
use super::*;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::collections::BTreeMap;
use std::sync::LazyLock;
pub static SMALL: LazyLock<Vec<u32>> = LazyLock::new(|| sieve(100000));

fn sieve(n: usize) -> Vec<u32> {
    let mut comp = vec![false; n + 1];
    let mut list = vec![];
    for i in 2..=n {
        if comp[i] {
            continue;
        }
        list.push(i as u32);
        let mut j = i * i;
        while j <= n {
            comp[j] = true;
            j += i;
        }
    }
    list
}

const MR_BASES: [u32; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

pub fn is_prime(n: &BigInt) -> bool {
    if *n < big(2) {
        return false;
    }
    let small = &*SMALL;
    if *n <= big(*small.last().unwrap() as i64) {
        return small.binary_search(&n.to_u32().unwrap()).is_ok();
    }
    for &p in small.iter().take(50) {
        if (n % p).is_zero() {
            return false;
        }
    }
    let mut d: BigInt = n - 1;
    let mut s = 0;
    while d.is_even() {
        d >>= 1;
        s += 1;
    }
    for &a in &MR_BASES {
        if !miller_rabin(n, &big(a as i64), &d, s) {
            return false;
        }
    }
    if bit_len(n) > 82 {
        let mut seed: u64 = 12345;
        let words = (bit_len(n) as usize).div_ceil(32);
        for _ in 0..16 {
            let mut r = BigInt::zero();
            for _ in 0..words {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                r = (r << 32usize) + (seed >> 32);
            }
            let a = r % (n - 4) + 2;
            if !miller_rabin(n, &a, &d, s) {
                return false;
            }
        }
    }
    true
}

fn miller_rabin(n: &BigInt, a: &BigInt, d: &BigInt, s: i32) -> bool {
    let nm1 = n - 1;
    let mut x = a.modpow(d, n);
    if x.is_one() || x == nm1 {
        return true;
    }
    for _ in 1..s {
        x = x.modpow(&big(2), n);
        if x == nm1 {
            return true;
        }
        if x.is_one() {
            return false;
        }
    }
    false
}

pub fn next_prime(n: &BigInt) -> BigInt {
    if *n < big(2) {
        return big(2);
    }
    let mut c: BigInt = n + 1;
    if c.is_even() && c != big(2) {
        c += 1;
    }
    while !is_prime(&c) {
        c += 2;
    }
    c
}

pub fn prev_prime(n: &BigInt) -> BigInt {
    if *n <= big(2) {
        throw_s(format!("No prime below {}", n));
    }
    let mut c = n - 1;
    if c == big(2) {
        return c;
    }
    if c.is_even() {
        c -= 1;
    }
    while c > big(2) && !is_prime(&c) {
        c -= 2;
    }
    c
}

pub fn nth_prime(n: i64) -> BigInt {
    if n < 1 {
        throw("nthprime() requires a positive integer");
    }
    if n as usize <= SMALL.len() {
        return big(SMALL[n as usize - 1] as i64);
    }
    if n > 5000000 {
        throw("nthprime() argument too large");
    }
    let ln = (n as f64).ln();
    let limit = (n as f64 * (ln + ln.ln()) + 10.0) as usize;
    big(sieve(limit)[n as usize - 1] as i64)
}

pub fn prime_count(x: &BigInt) -> i64 {
    if *x < big(2) {
        return 0;
    }
    if *x > big(100000000) {
        throw("primePi() argument too large");
    }
    let n = x.to_usize().unwrap();
    let small = &*SMALL;
    if n <= *small.last().unwrap() as usize {
        return match small.binary_search(&(n as u32)) {
            Ok(i) => i as i64 + 1,
            Err(i) => i as i64,
        };
    }
    let mut comp = vec![false; n + 1];
    let mut count = 0;
    for i in 2..=n {
        if comp[i] {
            continue;
        }
        count += 1;
        let mut j = i * i;
        while j <= n {
            comp[j] = true;
            j += i;
        }
    }
    count
}

pub fn primes_up_to(n: i64) -> Vec<BigInt> {
    if n > 10000000 {
        throw("Limit too large");
    }
    sieve(n.max(2) as usize).into_iter().filter(|&p| (p as i64) <= n).map(|p| big(p as i64)).collect()
}

pub fn factorize(n: &BigInt) -> BTreeMap<BigInt, i32> {
    let mut res = BTreeMap::new();
    let mut n = n.abs();
    if n < big(2) {
        return res;
    }
    for &p in SMALL.iter() {
        if big(p as i64) * big(p as i64) > n {
            break;
        }
        while (&n % p).is_zero() {
            *res.entry(big(p as i64)).or_insert(0) += 1;
            n /= p;
        }
    }
    if n > BigInt::one() {
        factor_rec(&n, &mut res);
    }
    res
}

fn factor_rec(n: &BigInt, res: &mut BTreeMap<BigInt, i32>) {
    if n.is_one() {
        return;
    }
    if is_prime(n) {
        *res.entry(n.clone()).or_insert(0) += 1;
        return;
    }
    let d = rho(n);
    if d.is_one() || d == *n {
        *res.entry(n.clone()).or_insert(0) += 1;
        return;
    }
    factor_rec(&d, res);
    factor_rec(&(n / &d), res);
}

fn rho(n: &BigInt) -> BigInt {
    if n.is_even() {
        return big(2);
    }
    let start = std::time::Instant::now();
    for c in 1..50 {
        let mut x = big(2);
        let mut y = big(2);
        let mut d = BigInt::one();
        let mut iter: u64 = 0;
        while d.is_one() {
            x = (&x * &x + c) % n;
            y = (&y * &y + c) % n;
            y = (&y * &y + c) % n;
            d = (&x - &y).abs().gcd(n);
            iter += 1;
            if iter % 1000 == 0 {
                if start.elapsed().as_secs_f64() > 20.0 {
                    throw("Factorization timed out");
                }
                cur().check_abort();
            }
        }
        if d != *n {
            return d;
        }
    }
    n.clone()
}

pub fn divisors(n: &BigInt) -> Vec<BigInt> {
    let f = factorize(n);
    let mut divs = vec![BigInt::one()];
    for (p, e) in f {
        let mut curv = vec![];
        let mut pk = BigInt::one();
        for _ in 0..=e {
            for d in &divs {
                curv.push(d * &pk);
            }
            pk *= &p;
        }
        divs = curv;
        if divs.len() > 100000 {
            throw("Too many divisors");
        }
    }
    divs.sort();
    divs
}

pub fn totient(n: &BigInt) -> BigInt {
    if *n < BigInt::one() {
        throw("totient() requires a positive integer");
    }
    let mut r = n.clone();
    for p in factorize(n).keys() {
        r = r / p * (p - 1);
    }
    r
}

pub fn factorial(n: &BigInt, step: i32) -> BigInt {
    if n.is_negative() {
        throw("Factorial of negative number");
    }
    if *n > big(100000) {
        throw("Factorial argument too large for exact calculation");
    }
    let k = n.to_i64().unwrap();
    if step == 1 {
        return product_range(1, k);
    }
    let mut r = BigInt::one();
    let mut i = k;
    while i > 1 {
        r *= i;
        i -= step as i64;
    }
    r
}

fn product_range(a: i64, b: i64) -> BigInt {
    if a > b {
        return BigInt::one();
    }
    if b - a < 16 {
        let mut r = BigInt::one();
        for i in a..=b {
            r *= i;
        }
        return r;
    }
    let m = (a + b) / 2;
    product_range(a, m) * product_range(m + 1, b)
}

pub fn binomial(n: &BigInt, k: &BigInt) -> BigInt {
    let mut k = k.clone();
    if k.is_negative() {
        return BigInt::zero();
    }
    if !n.is_negative() && k > *n {
        return BigInt::zero();
    }
    if !n.is_negative() && k > n - &k {
        k = n - &k;
    }
    if k > big(1000000) {
        throw("binomial() argument too large");
    }
    let mut r = BigInt::one();
    let mut i = BigInt::one();
    while i <= k {
        r = r * (n - &k + &i) / &i;
        i += 1;
    }
    r
}

pub fn fibonacci(n: &BigInt) -> BigInt {
    if n.is_negative() {
        let f = fibonacci(&-n);
        return if n.is_even() { -f } else { f };
    }
    if *n > big(10000000) {
        throw("fibonacci() argument too large");
    }
    let mut a = BigInt::zero();
    let mut b = BigInt::one();
    let bits = bit_len(n);
    for i in (0..bits).rev() {
        let d = &a * (&b * 2 - &a);
        let e = &a * &a + &b * &b;
        a = d;
        b = e;
        if ((n >> (i as usize)) & BigInt::one()).is_one() {
            let t = &a + &b;
            a = std::mem::replace(&mut b, t);
        }
    }
    a
}

pub fn bernoulli(n: i64) -> Real {
    if n < 0 {
        throw("bernoulli() requires a non-negative integer");
    }
    if n == 1 {
        return Real::frac(1, 2);
    }
    if n > 1 && n % 2 == 1 {
        return Real::zero();
    }
    if n > 2000 {
        throw("bernoulli() argument too large");
    }
    let n = n as usize;
    let mut a = vec![Real::zero(); n + 1];
    for m in 0..=n {
        a[m] = Real::frac(1, m as i64 + 1);
        for j in (1..=m).rev() {
            a[j - 1] = Real::int(j as i64) * (&a[j - 1] - &a[j]);
        }
    }
    a[0].clone()
}

pub fn subfactorial(n: &BigInt) -> BigInt {
    if n.is_negative() {
        throw("Negative argument");
    }
    if *n > big(50000) {
        throw("Argument too large");
    }
    if n.is_zero() {
        return BigInt::one();
    }
    let mut a = BigInt::one();
    let mut b = BigInt::zero();
    let nn = n.to_i64().unwrap();
    for i in 2..=nn {
        let t = (&a + &b) * (i - 1);
        a = std::mem::replace(&mut b, t);
    }
    b
}

pub fn partitions(n: i64) -> BigInt {
    if n < 0 {
        return BigInt::zero();
    }
    if n > 20000 {
        throw("Argument too large");
    }
    let n = n as usize;
    let mut p = vec![BigInt::zero(); n + 1];
    p[0] = BigInt::one();
    for i in 1..=n {
        let mut s = BigInt::zero();
        let mut k = 1usize;
        loop {
            let g1 = k * (3 * k - 1) / 2;
            let g2 = k * (3 * k + 1) / 2;
            if g1 > i {
                break;
            }
            let pos = k % 2 == 1;
            let mut add = p[i - g1].clone();
            if g2 <= i {
                add += &p[i - g2];
            }
            if pos {
                s += add;
            } else {
                s -= add;
            }
            k += 1;
        }
        p[i] = s;
    }
    p[n].clone()
}

pub fn stirling2(n: i64, k: i64) -> BigInt {
    if k > n {
        return BigInt::zero();
    }
    let mut sum = BigInt::zero();
    for j in 0..=k {
        let term = binomial(&big(k), &big(j)) * num_traits::pow(big(j), n as usize);
        if (k - j) % 2 == 0 {
            sum += term;
        } else {
            sum -= term;
        }
    }
    sum / factorial(&big(k), 1)
}

pub fn mod_inverse(a: &BigInt, m: &BigInt) -> BigInt {
    let a1 = ((a % m) + m) % m;
    let (mut old_r, mut r) = (a1, m.clone());
    let (mut old_s, mut s) = (BigInt::one(), BigInt::zero());
    while !r.is_zero() {
        let q = &old_r / &r;
        let nr = &old_r - &q * &r;
        old_r = std::mem::replace(&mut r, nr);
        let ns = &old_s - &q * &s;
        old_s = std::mem::replace(&mut s, ns);
    }
    if !old_r.is_one() {
        throw("No modular inverse exists");
    }
    ((old_s % m) + m) % m
}