use super::*;
use chrono::{Datelike, Duration, Local, LocalResult, Months, NaiveDate, NaiveDateTime, TimeZone, Utc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarKind {
    Gregorian,
    Hebrew,
    Islamic,
    Persian,
    Indian,
    Chinese,
    Julian,
    RevisedJulian,
    Coptic,
    Ethiopian,
}

pub const CALENDARS: [CalendarKind; 10] = [
    CalendarKind::Gregorian,
    CalendarKind::Hebrew,
    CalendarKind::Islamic,
    CalendarKind::Persian,
    CalendarKind::Indian,
    CalendarKind::Chinese,
    CalendarKind::Julian,
    CalendarKind::RevisedJulian,
    CalendarKind::Coptic,
    CalendarKind::Ethiopian,
];

pub const CALENDAR_NAMES: [&str; 10] = ["Gregorian", "Hebrew", "Islamic (Hijri)", "Persian (Solar Hijri)", "Indian (National)", "Chinese", "Julian", "Revised Julian (Milanković)", "Coptic", "Ethiopian"];

pub fn get(e: &Expr) -> NaiveDateTime {
    match &as_date(e).e {
        E::Date(t, ..) => *t,
        _ => unreachable!(),
    }
}

pub fn as_date(e: &Expr) -> Expr {
    match &e.e {
        E::Date(..) => e.clone(),
        E::Str(s) => {
            let (d, ht, u) = funcs::parse_date(s);
            date_e(d, ht, u)
        }
        _ => throw("A date is required"),
    }
}

pub fn local_to_utc(t: NaiveDateTime) -> NaiveDateTime {
    match Local.from_local_datetime(&t) {
        LocalResult::Single(d) | LocalResult::Ambiguous(d, _) => d.naive_utc(),
        LocalResult::None => t,
    }
}

pub fn utc_to_local(t: NaiveDateTime) -> NaiveDateTime {
    Utc.from_utc_datetime(&t).with_timezone(&Local).naive_local()
}

pub fn add_months(t: NaiveDateTime, n: i64) -> NaiveDateTime {
    let r = if n >= 0 { t.checked_add_months(Months::new(n as u32)) } else { t.checked_sub_months(Months::new((-n) as u32)) };
    r.unwrap_or_else(|| throw("Date out of range"))
}

pub fn add_seconds(d: &Expr, seconds: &Real) -> Expr {
    let E::Date(t, has_time, utc) = &d.e else { throw("A date is required") };
    let s = seconds.to_f64();
    let t = t.checked_add_signed(Duration::microseconds((s * 1e6).round() as i64)).unwrap_or_else(|| throw("Date out of range"));
    let has_time = *has_time || (s % 86400.0).abs() > 1e-6;
    date_e(t, has_time, *utc)
}

const SYNODIC: f64 = 29.530588853;

fn known_new_moon() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2000, 1, 6).unwrap().and_hms_opt(18, 14, 0).unwrap()
}

pub fn lunar_phase(d: NaiveDateTime) -> f64 {
    let days = (local_to_utc(d) - known_new_moon()).num_milliseconds() as f64 / 86400000.0;
    let mut ph = (days / SYNODIC) % 1.0;
    if ph < 0.0 {
        ph += 1.0;
    }
    (ph * 10000.0).round() / 10000.0
}

pub fn next_lunar_phase(from: NaiveDateTime, phase: f64) -> NaiveDateTime {
    let mut delta = phase - lunar_phase(from);
    if delta <= 0.0 {
        delta += 1.0;
    }
    from + Duration::milliseconds((delta * SYNODIC * 86400000.0) as i64)
}

pub fn easter(y: i32) -> NaiveDateTime {
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e, f) = (b / 4, b % 4, (b + 8) / 25);
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    ymd(y, month, day)
}

fn ymd(y: i32, m: i32, d: i32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(y, m as u32, d as u32).and_then(|x| x.and_hms_opt(0, 0, 0)).unwrap_or_else(|| throw("Date out of range"))
}

// offset in minutes; zero gives a UTC date, anything else a text with the offset
pub fn to_utc(e: &Expr, offset_min: i64) -> Expr {
    let E::Date(t, _, u) = &e.e else { throw("UTC conversion requires a date") };
    let utc = if *u { *t } else { local_to_utc(*t) };
    if offset_min == 0 {
        return date_e(utc, true, true);
    }
    let tt = utc + Duration::minutes(offset_min);
    let ab = offset_min.abs();
    str_e(&format!("{}{}{:02}:{:02}", tt.format("%Y-%m-%dT%H:%M:%S"), if offset_min < 0 { "-" } else { "+" }, ab / 60, ab % 60))
}

pub fn to_jdn(g: NaiveDate) -> i64 {
    let (gy, gm, gd) = (g.year() as i64, g.month() as i64, g.day() as i64);
    let a = (14 - gm) / 12;
    let y = gy + 4800 - a;
    let m = gm + 12 * a - 3;
    gd + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045
}

pub fn from_jdn(jdn: i64) -> NaiveDate {
    let a = jdn + 32044;
    let b = (4 * a + 3) / 146097;
    let c = a - 146097 * b / 4;
    let d = (4 * c + 3) / 1461;
    let e = c - 1461 * d / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = 100 * b + d - 4800 + m / 10;
    NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32).unwrap_or_else(|| throw("Date out of range"))
}

fn julian_to_jdn(y: i64, m: i64, d: i64) -> i64 {
    let a = (14 - m) / 12;
    let yy = y + 4800 - a;
    let mm = m + 12 * a - 3;
    d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - 32083
}

fn jdn_to_julian(jdn: i64) -> (i32, i32, i32) {
    let c = jdn + 32082;
    let dd = (4 * c + 3) / 1461;
    let e = c - 1461 * dd / 4;
    let mm = (5 * e + 2) / 153;
    ((dd - 4800 + mm / 10) as i32, (mm + 3 - 12 * (mm / 10)) as i32, (e - (153 * mm + 2) / 5 + 1) as i32)
}

fn floor_div(a: i64, b: i64) -> i64 {
    a.div_euclid(b)
}

fn rj_leap(y: i64) -> bool {
    let r = y.rem_euclid(900);
    (y % 4 == 0 && y % 100 != 0) || r == 200 || r == 600
}

fn rj_leaps_before(y: i64) -> i64 {
    let n = y - 1;
    floor_div(n, 4) - floor_div(n, 100) + floor_div(n - 200, 900) + 1 + floor_div(n - 600, 900) + 1
}

const MONTH_DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

fn rj_raw(y: i64, m: i64, d: i64) -> i64 {
    let mut days = 365 * (y - 1) + rj_leaps_before(y);
    for i in 1..m {
        days += MONTH_DAYS[(i - 1) as usize] + if i == 2 && rj_leap(y) { 1 } else { 0 };
    }
    days + d
}

fn revised_julian_to_jdn(y: i64, m: i64, d: i64) -> i64 {
    let off = to_jdn(NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()) - rj_raw(2000, 1, 1);
    rj_raw(y, m, d) + off
}

fn jdn_to_revised_julian(jdn: i64) -> (i32, i32, i32) {
    let g = from_jdn(jdn);
    let mut y = g.year() as i64 + 1;
    while revised_julian_to_jdn(y, 1, 1) > jdn {
        y -= 1;
    }
    let mut m = 12;
    while revised_julian_to_jdn(y, m, 1) > jdn {
        m -= 1;
    }
    (y as i32, m as i32, (jdn - revised_julian_to_jdn(y, m, 1) + 1) as i32)
}

fn coptic_like(epoch: i64, y: i64, m: i64, d: i64) -> i64 {
    epoch - 1 + 365 * (y - 1) + y / 4 + 30 * (m - 1) + d
}

fn from_coptic_like(epoch: i64, jdn: i64) -> (i32, i32, i32) {
    let y = (4 * (jdn - epoch) + 1463) / 1461;
    let m = (jdn - coptic_like(epoch, y, 1, 1)) / 30 + 1;
    let d = jdn + 1 - coptic_like(epoch, y, m, 1);
    (y as i32, m as i32, d as i32)
}

const COPTIC_EPOCH: i64 = 1825030;
const ETHIOPIAN_EPOCH: i64 = 1724221;

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn indian_lens(leap: bool) -> [i64; 12] {
    [if leap { 31 } else { 30 }, 31, 31, 31, 31, 31, 30, 30, 30, 30, 30, 30]
}

fn to_indian(g: NaiveDate) -> (i32, i32, i32) {
    let gy = g.year();
    let mut leap = is_leap(gy);
    let mut start = NaiveDate::from_ymd_opt(gy, 3, if leap { 21 } else { 22 }).unwrap();
    let mut sy = gy - 78;
    if g < start {
        sy -= 1;
        leap = is_leap(gy - 1);
        start = NaiveDate::from_ymd_opt(gy - 1, 3, if leap { 21 } else { 22 }).unwrap();
    }
    let mut doy = (g - start).num_days();
    let len = indian_lens(leap);
    let mut m = 0;
    while doy >= len[m] {
        doy -= len[m];
        m += 1;
    }
    (sy, m as i32 + 1, doy as i32 + 1)
}

fn from_indian(y: i32, m: i32, d: i32) -> NaiveDate {
    let gy = y + 78;
    let leap = is_leap(gy);
    let start = NaiveDate::from_ymd_opt(gy, 3, if leap { 21 } else { 22 }).unwrap_or_else(|| throw("Date out of range"));
    let len = indian_lens(leap);
    let days: i64 = len.iter().take((m - 1).clamp(0, 12) as usize).sum();
    start + Duration::days(days + d as i64 - 1)
}

// ---- Hebrew (arithmetic calendar, months numbered from Tishrei like .NET) ----
const HEBREW_EPOCH_JDN: i64 = 347998;

fn hebrew_leap(y: i64) -> bool {
    (7 * y + 1).rem_euclid(19) < 7
}

fn hebrew_elapsed(y: i64) -> i64 {
    let months = (235 * y - 234).div_euclid(19);
    let parts = 12084 + 13753 * months;
    let mut day = months * 29 + parts.div_euclid(25920);
    if (3 * (day + 1)).rem_euclid(7) < 3 {
        day += 1;
    }
    day
}

fn hebrew_delay(y: i64) -> i64 {
    let ny0 = hebrew_elapsed(y - 1);
    let ny1 = hebrew_elapsed(y);
    let ny2 = hebrew_elapsed(y + 1);
    if ny2 - ny1 == 356 {
        2
    } else if ny1 - ny0 == 382 {
        1
    } else {
        0
    }
}

fn hebrew_new_year(y: i64) -> i64 {
    HEBREW_EPOCH_JDN + hebrew_elapsed(y) + hebrew_delay(y)
}

fn hebrew_year_days(y: i64) -> i64 {
    hebrew_new_year(y + 1) - hebrew_new_year(y)
}

fn hebrew_month_days(y: i64, m: i64) -> i64 {
    let leap = hebrew_leap(y);
    let len = hebrew_year_days(y);
    // m counted from Tishrei = 1
    let civil = if leap { m } else if m >= 6 { m + 1 } else { m };
    match civil {
        1 => 30,
        2 => {
            if len % 10 == 5 {
                30
            } else {
                29
            }
        }
        3 => {
            if len % 10 == 3 {
                29
            } else {
                30
            }
        }
        4 => 29,
        5 => 30,
        6 => {
            if leap {
                30
            } else {
                29
            }
        }
        7 => 29,
        8 => 30,
        9 => 29,
        10 => 30,
        11 => 29,
        12 => 30,
        _ => 29,
    }
}

fn hebrew_to_jdn(y: i64, m: i64, d: i64) -> i64 {
    let mut j = hebrew_new_year(y);
    for i in 1..m {
        j += hebrew_month_days(y, i);
    }
    j + d - 1
}

fn jdn_to_hebrew(jdn: i64) -> (i32, i32, i32) {
    let mut y = from_jdn(jdn).year() as i64 + 3760;
    while hebrew_new_year(y + 1) <= jdn {
        y += 1;
    }
    while hebrew_new_year(y) > jdn {
        y -= 1;
    }
    let months = if hebrew_leap(y) { 13 } else { 12 };
    let mut m = 1;
    let mut start = hebrew_new_year(y);
    while m < months && start + hebrew_month_days(y, m) <= jdn {
        start += hebrew_month_days(y, m);
        m += 1;
    }
    (y as i32, m as i32, (jdn - start + 1) as i32)
}

// ---- Islamic (tabular, civil epoch) ----
const ISLAMIC_EPOCH: i64 = 1948440;

fn islamic_to_jdn(y: i64, m: i64, d: i64) -> i64 {
    d + (29.5 * (m - 1) as f64).ceil() as i64 + (y - 1) * 354 + (3 + 11 * y).div_euclid(30) + ISLAMIC_EPOCH - 1
}

fn jdn_to_islamic(jdn: i64) -> (i32, i32, i32) {
    let y = (30 * (jdn - ISLAMIC_EPOCH) + 10646).div_euclid(10631);
    let m = (((jdn - 29 - islamic_to_jdn(y, 1, 1)) as f64 / 29.5).ceil() as i64 + 1).clamp(1, 12);
    let d = jdn - islamic_to_jdn(y, m, 1) + 1;
    (y as i32, m as i32, d as i32)
}

// ---- Persian (jalaali, Borkowski's algorithm) ----
const JALAALI_BREAKS: [i64; 20] = [-61, 9, 38, 199, 426, 686, 756, 818, 1111, 1181, 1210, 1635, 2060, 2097, 2192, 2262, 2324, 2394, 2456, 3178];

fn jal_cal(jy: i64) -> (i64, i64) {
    let gy = jy + 621;
    let mut leap_j = -14;
    let mut jp = JALAALI_BREAKS[0];
    let mut jump = 0;
    if jy < jp || jy >= JALAALI_BREAKS[19] {
        throw("Date out of range");
    }
    for &jm in &JALAALI_BREAKS[1..] {
        jump = jm - jp;
        if jy < jm {
            break;
        }
        leap_j += jump / 33 * 8 + (jump % 33) / 4;
        jp = jm;
    }
    let mut n = jy - jp;
    leap_j += n / 33 * 8 + ((n % 33) + 3) / 4;
    if jump % 33 == 4 && jump - n == 4 {
        leap_j += 1;
    }
    let leap_g = gy / 4 - (gy / 100 + 1) * 3 / 4 - 150;
    let march = 20 + leap_j - leap_g;
    if jump - n < 6 {
        n = n - jump + (jump + 4) / 33 * 33;
    }
    let _leap = ((n + 1) % 33 - 1) % 4;
    (gy, march)
}

fn persian_to_jdn(jy: i64, jm: i64, jd: i64) -> i64 {
    let (gy, march) = jal_cal(jy);
    to_jdn(NaiveDate::from_ymd_opt(gy as i32, 3, march as u32).unwrap()) + (jm - 1) * 31 - (jm / 7) * (jm - 7) + jd - 1
}

fn jdn_to_persian(jdn: i64) -> (i32, i32, i32) {
    let gy = from_jdn(jdn).year() as i64;
    let mut jy = gy - 621;
    let (_, march) = jal_cal(jy);
    let jdn1f = to_jdn(NaiveDate::from_ymd_opt(gy as i32, 3, march as u32).unwrap());
    let mut k = jdn - jdn1f;
    if k >= 0 {
        if k <= 185 {
            return (jy as i32, (1 + k / 31) as i32, (k % 31 + 1) as i32);
        }
        k -= 186;
    } else {
        jy -= 1;
        k += 179;
        let (_, m2) = jal_cal(jy);
        let leap = jdn1f - to_jdn(NaiveDate::from_ymd_opt(gy as i32 - 1, 3, m2 as u32).unwrap()) == 366;
        if leap {
            k += 1;
        }
    }
    (jy as i32, (7 + k / 30) as i32, (k % 30 + 1) as i32)
}

// ---- Chinese lunisolar (astronomical new moons and solar terms, UTC+8) ----
fn new_moon_day(k: i64) -> i64 {
    let t = k as f64 / 1236.85;
    let t2 = t * t;
    let t3 = t2 * t;
    let dr = std::f64::consts::PI / 180.0;
    let mut jd1 = 2415020.75933 + 29.53058868 * k as f64 + 0.0001178 * t2 - 0.000000155 * t3;
    jd1 += 0.00033 * ((166.56 + 132.87 * t - 0.009173 * t2) * dr).sin();
    let m = 359.2242 + 29.10535608 * k as f64 - 0.0000333 * t2 - 0.00000347 * t3;
    let mpr = 306.0253 + 385.81691806 * k as f64 + 0.0107306 * t2 + 0.00001236 * t3;
    let f = 21.2964 + 390.67050646 * k as f64 - 0.0016528 * t2 - 0.00000239 * t3;
    let mut c1 = (0.1734 - 0.000393 * t) * (m * dr).sin() + 0.0021 * (2.0 * dr * m).sin();
    c1 = c1 - 0.4068 * (mpr * dr).sin() + 0.0161 * (dr * 2.0 * mpr).sin();
    c1 -= 0.0004 * (dr * 3.0 * mpr).sin();
    c1 = c1 + 0.0104 * (dr * 2.0 * f).sin() - 0.0051 * (dr * (m + mpr)).sin();
    c1 = c1 - 0.0074 * (dr * (m - mpr)).sin() + 0.0004 * (dr * (2.0 * f + m)).sin();
    c1 = c1 - 0.0004 * (dr * (2.0 * f - m)).sin() - 0.0006 * (dr * (2.0 * f + mpr)).sin();
    c1 = c1 + 0.0010 * (dr * (2.0 * f - mpr)).sin() + 0.0005 * (dr * (2.0 * mpr + m)).sin();
    let deltat = if t < -11.0 { 0.001 + 0.000839 * t + 0.0002261 * t2 - 0.00000845 * t3 - 0.000000081 * t * t3 } else { -0.000278 + 0.000265 * t + 0.000262 * t2 };
    let jd = jd1 + c1 - deltat;
    (jd + 0.5 + 8.0 / 24.0).floor() as i64
}

fn sun_longitude(jdn: i64) -> i64 {
    let t = (jdn as f64 - 2451545.5 - 8.0 / 24.0) / 36525.0;
    let t2 = t * t;
    let dr = std::f64::consts::PI / 180.0;
    let m = 357.52910 + 35999.05030 * t - 0.0001559 * t2 - 0.00000048 * t * t2;
    let l0 = 280.46645 + 36000.76983 * t + 0.0003032 * t2;
    let mut dl = (1.914600 - 0.004817 * t - 0.000014 * t2) * (dr * m).sin();
    dl = dl + (0.019993 - 0.000101 * t) * (dr * 2.0 * m).sin() + 0.000290 * (dr * 3.0 * m).sin();
    let mut l = (l0 + dl) * dr;
    l -= std::f64::consts::PI * 2.0 * (l / (std::f64::consts::PI * 2.0)).floor();
    (l / std::f64::consts::PI * 6.0).floor() as i64
}

fn lunar_month11(yy: i64) -> i64 {
    let off = to_jdn(NaiveDate::from_ymd_opt(yy as i32, 12, 31).unwrap()) - 2415021;
    let k = (off as f64 / 29.530588853).floor() as i64;
    let mut nm = new_moon_day(k);
    if sun_longitude(nm) >= 9 {
        nm = new_moon_day(k - 1);
    }
    nm
}

fn leap_month_offset(a11: i64) -> i64 {
    let k = ((a11 as f64 - 2415021.076998695) / 29.530588853 + 0.5).floor() as i64;
    let mut last;
    let mut i = 1;
    let mut arc = sun_longitude(new_moon_day(k + i));
    loop {
        last = arc;
        i += 1;
        arc = sun_longitude(new_moon_day(k + i));
        if arc == last || i >= 14 {
            break;
        }
    }
    i - 1
}

// (year, month 1..12, day, leap)
fn solar_to_chinese(jdn: i64, year: i64) -> (i64, i64, i64, bool) {
    let k = ((jdn as f64 - 2415021.076998695) / 29.530588853).floor() as i64;
    let mut month_start = new_moon_day(k + 1);
    if month_start > jdn {
        month_start = new_moon_day(k);
    }
    let mut a11 = lunar_month11(year);
    let mut b11 = a11;
    let mut lyear;
    if a11 >= month_start {
        lyear = year;
        a11 = lunar_month11(year - 1);
    } else {
        lyear = year + 1;
        b11 = lunar_month11(year + 1);
    }
    let day = jdn - month_start + 1;
    let diff = (month_start - a11) / 29;
    let mut leap = false;
    let mut month = diff + 11;
    if b11 - a11 > 365 {
        let leap_diff = leap_month_offset(a11);
        if diff >= leap_diff {
            month = diff + 10;
            if diff == leap_diff {
                leap = true;
            }
        }
    }
    if month > 12 {
        month -= 12;
    }
    if month >= 11 && diff < 4 {
        lyear -= 1;
    }
    (lyear, month, day, leap)
}

fn chinese_to_solar(lyear: i64, lmonth: i64, lday: i64, leap: bool) -> i64 {
    let (a11, b11) = if lmonth < 11 { (lunar_month11(lyear - 1), lunar_month11(lyear)) } else { (lunar_month11(lyear), lunar_month11(lyear + 1)) };
    let k = (0.5 + (a11 as f64 - 2415021.076998695) / 29.530588853).floor() as i64;
    let mut off = lmonth - 11;
    if off < 0 {
        off += 12;
    }
    if b11 - a11 > 365 {
        let leap_off = leap_month_offset(a11);
        let mut leap_month = leap_off - 2;
        if leap_month < 0 {
            leap_month += 12;
        }
        if leap && lmonth != leap_month {
            throw("Invalid leap month");
        } else if leap || off >= leap_off {
            off += 1;
        }
    }
    new_moon_day(k + off) + lday - 1
}

// the leap month of a Chinese year as a month number after which it falls (0 = none)
fn chinese_leap_month(lyear: i64) -> i64 {
    let a11 = lunar_month11(lyear - 1);
    let b11 = lunar_month11(lyear);
    if b11 - a11 <= 365 {
        return 0;
    }
    let mut lm = leap_month_offset(a11) - 2;
    if lm <= 0 {
        lm += 12;
    }
    lm
}

pub fn month_names(k: CalendarKind) -> Vec<String> {
    let v: &[&str] = match k {
        CalendarKind::Hebrew => &["Tishrei", "Heshvan", "Kislev", "Tevet", "Shevat", "Adar I", "Adar II", "Nisan", "Iyar", "Sivan", "Tammuz", "Av", "Elul"],
        CalendarKind::Islamic => &["Muharram", "Safar", "Rabi' al-awwal", "Rabi' al-thani", "Jumada al-awwal", "Jumada al-thani", "Rajab", "Sha'ban", "Ramadan", "Shawwal", "Dhu al-Qi'dah", "Dhu al-Hijjah"],
        CalendarKind::Persian => &["Farvardin", "Ordibehesht", "Khordad", "Tir", "Mordad", "Shahrivar", "Mehr", "Aban", "Azar", "Dey", "Bahman", "Esfand"],
        CalendarKind::Indian => &["Chaitra", "Vaishakha", "Jyeshtha", "Ashadha", "Shravana", "Bhadra", "Ashvin", "Kartika", "Agrahayana", "Pausha", "Magha", "Phalguna"],
        CalendarKind::Coptic => &["Thout", "Paopi", "Hathor", "Koiak", "Tobi", "Meshir", "Paremhat", "Parmouti", "Pashons", "Paoni", "Epip", "Mesori", "Pi Kogi Enavot"],
        CalendarKind::Ethiopian => &["Meskerem", "Tikimt", "Hidar", "Tahsas", "Tir", "Yekatit", "Megabit", "Miazia", "Genbot", "Sene", "Hamle", "Nehase", "Pagume"],
        CalendarKind::Chinese => return (1..=13).map(|i| format!("Month {}", i)).collect(),
        _ => &["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"],
    };
    v.iter().map(|s| s.to_string()).collect()
}

pub fn to_gregorian(k: CalendarKind, y: i32, m: i32, d: i32) -> NaiveDate {
    let (y, m, d) = (y as i64, m as i64, d as i64);
    let jdn = match k {
        CalendarKind::Gregorian => return NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32).unwrap_or_else(|| throw("Invalid date")),
        CalendarKind::Hebrew => {
            let months = if hebrew_leap(y) { 13 } else { 12 };
            if m < 1 || m > months || d < 1 || d > hebrew_month_days(y, m) {
                throw("Invalid date");
            }
            hebrew_to_jdn(y, m, d)
        }
        CalendarKind::Islamic => islamic_to_jdn(y, m, d),
        CalendarKind::Persian => persian_to_jdn(y, m, d),
        CalendarKind::Chinese => {
            // months numbered 1..13 with the leap month inserted, like .NET
            let lm = chinese_leap_month(y);
            if lm > 0 && m > lm {
                chinese_to_solar(y, if m == lm + 1 { lm } else { m - 1 }, d, m == lm + 1)
            } else {
                chinese_to_solar(y, m, d, false)
            }
        }
        CalendarKind::Indian => return from_indian(y as i32, m as i32, d as i32),
        CalendarKind::Julian => julian_to_jdn(y, m, d),
        CalendarKind::RevisedJulian => revised_julian_to_jdn(y, m, d),
        CalendarKind::Coptic => coptic_like(COPTIC_EPOCH, y, m, d),
        CalendarKind::Ethiopian => coptic_like(ETHIOPIAN_EPOCH, y, m, d),
    };
    from_jdn(jdn)
}

pub fn from_gregorian(k: CalendarKind, g: NaiveDate) -> (i32, i32, i32) {
    let jdn = to_jdn(g);
    match k {
        CalendarKind::Gregorian => (g.year(), g.month() as i32, g.day() as i32),
        CalendarKind::Hebrew => jdn_to_hebrew(jdn),
        CalendarKind::Islamic => jdn_to_islamic(jdn),
        CalendarKind::Persian => jdn_to_persian(jdn),
        CalendarKind::Chinese => {
            let (ly, lm, ld, leap) = solar_to_chinese(jdn, g.year() as i64);
            let lmonth = chinese_leap_month(ly);
            let m = if lmonth > 0 && (lm > lmonth || (lm == lmonth && leap)) { lm + 1 } else { lm };
            (ly as i32, m as i32, ld as i32)
        }
        CalendarKind::Indian => to_indian(g),
        CalendarKind::Julian => jdn_to_julian(jdn),
        CalendarKind::RevisedJulian => jdn_to_revised_julian(jdn),
        CalendarKind::Coptic => from_coptic_like(COPTIC_EPOCH, jdn),
        CalendarKind::Ethiopian => from_coptic_like(ETHIOPIAN_EPOCH, jdn),
    }
}

const STEMS: [&str; 10] = ["Jiă", "Yĭ", "Bĭng", "Dīng", "Wù", "Jĭ", "Gēng", "Xīn", "Rén", "Guĭ"];
const BRANCHES: [&str; 12] = ["Zĭ (Rat)", "Chŏu (Ox)", "Yín (Tiger)", "Măo (Rabbit)", "Chén (Dragon)", "Sì (Snake)", "Wŭ (Horse)", "Wèi (Goat)", "Shēn (Monkey)", "Yŏu (Rooster)", "Xū (Dog)", "Hài (Pig)"];

pub fn describe(k: CalendarKind, g: NaiveDate) -> String {
    let r = attempt(|| {
        let (y, m, d) = from_gregorian(k, g);
        if k == CalendarKind::Chinese {
            let stem = (y as i64 - 4).rem_euclid(10) as usize;
            let branch = (y as i64 - 4).rem_euclid(12) as usize;
            let leap = chinese_leap_month(y as i64) as i32;
            let month = if leap > 0 && m > leap {
                if m == leap + 1 {
                    format!("Leap month {}", m - 1)
                } else {
                    format!("Month {}", m - 1)
                }
            } else {
                format!("Month {}", m)
            };
            return format!("{} {}, {}, day {}", STEMS[stem], BRANCHES[branch], month, d);
        }
        let names = month_names(k);
        let mut mn = names.get((m - 1) as usize).cloned().unwrap_or_else(|| m.to_string());
        if k == CalendarKind::Hebrew && !hebrew_leap(y as i64) {
            const NL: [&str; 12] = ["Tishrei", "Heshvan", "Kislev", "Tevet", "Shevat", "Adar", "Nisan", "Iyar", "Sivan", "Tammuz", "Av", "Elul"];
            mn = NL[((m - 1) as usize).min(11)].to_string();
        }
        format!("{} {} {}", d, mn, y)
    });
    r.unwrap_or_else(|_| "out of range".to_string())
}

pub fn calendars_text(e: &Expr) -> Expr {
    let d = get(e).date();
    let lines: Vec<String> = CALENDARS.iter().enumerate().map(|(i, k)| format!("{}: {}", CALENDAR_NAMES[i], describe(*k, d))).collect();
    str_e(&lines.join("\n"))
}