use super::real::bit_len;
use super::*;
use num_traits::{One, Signed, Zero};

#[derive(Debug)]
pub struct FloatFormat {
    pub name: &'static str,
    pub exp_bits: u32,
    pub man_bits: u32,
    pub explicit_int: bool,
    pub microchip: bool,
}

impl FloatFormat {
    pub fn total(&self) -> u32 {
        1 + self.exp_bits + self.man_bits
    }
    pub fn bias(&self) -> i64 {
        (1i64 << (self.exp_bits - 1)) - 1
    }
}

const fn ff(name: &'static str, exp_bits: u32, man_bits: u32, explicit_int: bool, microchip: bool) -> FloatFormat {
    FloatFormat { name, exp_bits, man_bits, explicit_int, microchip }
}

pub static HALF: FloatFormat = ff("16-bit (half precision)", 5, 10, false, false);
pub static SINGLE: FloatFormat = ff("32-bit (single precision)", 8, 23, false, false);
pub static DOUBLE: FloatFormat = ff("64-bit (double precision)", 11, 52, false, false);
pub static EXTENDED: FloatFormat = ff("80-bit (x86 extended format)", 15, 64, true, false);
pub static QUAD: FloatFormat = ff("128-bit (quadruple precision)", 15, 112, false, false);
pub static MICROCHIP24: FloatFormat = ff("Microchip 24-bit", 8, 15, false, true);
pub static MICROCHIP32: FloatFormat = ff("Microchip 32-bit", 8, 23, false, true);
pub static ALL: [&FloatFormat; 7] = [&HALF, &SINGLE, &DOUBLE, &EXTENDED, &QUAD, &MICROCHIP24, &MICROCHIP32];

pub fn by_bits(bits: i32) -> &'static FloatFormat {
    match bits {
        16 => &HALF,
        32 => &SINGLE,
        64 => &DOUBLE,
        80 => &EXTENDED,
        128 => &QUAD,
        24 => &MICROCHIP24,
        _ => throw("Unsupported floating point size (16, 24, 32, 64, 80 or 128 bits)"),
    }
}

fn frac_bits(f: &FloatFormat) -> u32 {
    if f.explicit_int {
        f.man_bits - 1
    } else {
        f.man_bits
    }
}

fn pow2(e: i64) -> Real {
    if e >= 0 {
        Real::big_int(BigInt::one() << (e as usize))
    } else {
        Real::exact(BigInt::one(), BigInt::one() << ((-e) as usize))
    }
}

pub fn encode(x: &Real, f: &FloatFormat) -> BigInt {
    let fb = frac_bits(f);
    let bias = f.bias();
    let max_exp = (1i64 << f.exp_bits) - 1;
    let neg = x.sign() < 0;
    let one = BigInt::one();
    let (exp_field, frac): (BigInt, BigInt) = if x.is_infinite() {
        (BigInt::from(max_exp), if f.explicit_int { &one << fb } else { BigInt::zero() })
    } else if x.is_zero() {
        (BigInt::zero(), BigInt::zero())
    } else {
        let a = x.abs();
        let mut e = bit_len(&a.n) - bit_len(&a.d);
        if pow2(e).gt(&a) {
            e -= 1;
        }
        if pow2(e + 1).le(&a) {
            e += 1;
        }
        let min_exp = 1 - bias;
        let mut sub = e < min_exp;
        if sub {
            e = min_exp;
        }
        let scaled = &a / &pow2(e) * pow2(fb as i64);
        let mut m = scaled.round(RoundingMode::HalfEven).n;
        if !sub && m >= (&one << (fb + 1)) {
            m >>= 1usize;
            e += 1;
        }
        if sub && m >= (&one << fb) {
            sub = false;
        }
        if e + bias >= max_exp && !f.microchip {
            (BigInt::from(max_exp), if f.explicit_int { &one << fb } else { BigInt::zero() })
        } else if sub {
            (BigInt::zero(), m)
        } else {
            (BigInt::from(e + bias), if f.explicit_int { m } else { m - (&one << fb) })
        }
    };
    let s = if neg { BigInt::one() } else { BigInt::zero() };
    if f.microchip {
        return (exp_field << (f.man_bits + 1) as usize) | (s << f.man_bits as usize) | frac;
    }
    (s << (f.exp_bits + f.man_bits) as usize) | (exp_field << f.man_bits as usize) | frac
}

pub fn decode(bits_text: &str, f: &FloatFormat) -> Real {
    let t = bits_text.replace([' ', '_'], "");
    let v = if t.to_lowercase().starts_with("0x") {
        BigInt::parse_bytes(format!("0{}", &t[2..]).as_bytes(), 16).unwrap_or_else(|| throw("Hexadecimal digits expected"))
    } else {
        if t.chars().any(|ch| ch != '0' && ch != '1') {
            throw("Binary digits expected");
        }
        t.chars().fold(BigInt::zero(), |v, ch| v * 2 + (ch as i32 - '0' as i32))
    };
    decode_value(&v, f)
}

pub fn decode_value(v: &BigInt, f: &FloatFormat) -> Real {
    let fb = frac_bits(f);
    let bias = f.bias();
    let one = BigInt::one();
    let man_mask = (&one << f.man_bits as usize) - 1;
    let (sign, exp_field, frac) = if f.microchip {
        (((v >> f.man_bits as usize) & &one), v >> (f.man_bits + 1) as usize, v & &man_mask)
    } else {
        (((v >> (f.exp_bits + f.man_bits) as usize) & &one), (v >> f.man_bits as usize) & ((&one << f.exp_bits as usize) - 1), v & &man_mask)
    };
    let max_exp = BigInt::from((1i64 << f.exp_bits) - 1);
    if exp_field == max_exp && !f.microchip {
        let fr = if f.explicit_int { &frac & ((&one << fb as usize) - 1) } else { frac.clone() };
        if fr.is_zero() {
            return if sign.is_one() { Real::neg_inf() } else { Real::pos_inf() };
        }
        throw("The value is NaN (not a number)");
    }
    if f.microchip && exp_field.is_zero() {
        return Real::zero();
    }
    let val = if exp_field.is_zero() {
        Real::big_int(frac) * pow2(1 - bias - fb as i64)
    } else {
        let m = if f.explicit_int { frac } else { frac + (&one << fb as usize) };
        let ef: i64 = exp_field.try_into().unwrap_or(0);
        Real::big_int(m) * pow2(ef - bias - fb as i64)
    };
    if sign.is_one() {
        -val
    } else {
        val
    }
}

pub fn to_bits(v: &BigInt, f: &FloatFormat) -> String {
    let v = if v.is_negative() { BigInt::zero() } else { v.clone() };
    (0..f.total()).rev().map(|i| if ((&v >> i as usize) & BigInt::one()).is_zero() { '0' } else { '1' }).collect()
}

pub fn to_hex(v: &BigInt, f: &FloatFormat) -> String {
    let digits = f.total().div_ceil(4) as usize;
    format!("0x{:0>w$}", v.to_str_radix(16).to_uppercase(), w = digits)
}

pub fn binary_string(r: &Real, f: &FloatFormat) -> String {
    let bits = to_bits(&encode(r, f), f);
    let e = f.exp_bits as usize;
    if f.microchip {
        return format!("{} {} {}", &bits[..e], &bits[e..e + 1], &bits[e + 1..]);
    }
    format!("{} {} {}", &bits[..1], &bits[1..1 + e], &bits[1 + e..])
}