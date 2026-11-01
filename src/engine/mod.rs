pub use num_bigint::BigInt;
pub mod algebra;
pub mod calculator;
pub mod calculus;
pub mod constants;
pub mod c64;
pub mod cx;
pub mod data;
mod datatables;
pub mod dates;
pub mod defs;
pub mod evaluator;
pub mod expr;
pub mod floatformats;
pub mod formatter;
pub mod funcs;
pub mod funcs2;
pub mod intervals;
pub mod matrices;
pub mod options;
pub mod parser;
pub mod polynomials;
pub mod primes;
pub mod real;
pub mod rmath;
pub mod solver;
pub mod special;
pub mod units;
pub mod useritems;
pub use calculator::{cur, CalcResult, Calculator};
pub use cx::Cx;
pub use defs::*;
pub use expr::*;
pub use options::*;
pub use real::{Real, RoundingMode};

#[derive(Debug, Clone)]
pub struct CalcError(pub String);

pub fn throw(msg: &str) -> ! {
    std::panic::panic_any(CalcError(msg.to_string()))
}

pub fn throw_s(msg: String) -> ! {
    std::panic::panic_any(CalcError(msg))
}

fn panic_message(p: Box<dyn std::any::Any + Send>) -> Result<String, Box<dyn std::any::Any + Send>> {
    if let Some(e) = p.downcast_ref::<CalcError>() {
        return Ok(e.0.clone());
    }
    Err(p)
}

// runs f, turning a CalcError into Err
pub fn attempt<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => Ok(v),
        Err(p) => match panic_message(p) {
            Ok(m) => Err(m),
            Err(p) => std::panic::resume_unwind(p),
        },
    }
}

// like attempt, but any panic becomes an error message
pub fn attempt_all<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => Ok(v),
        Err(p) => match panic_message(p) {
            Ok(m) => Err(m),
            Err(p) => {
                if let Some(s) = p.downcast_ref::<&str>() {
                    Err(format!("Internal error: {}", s))
                } else if let Some(s) = p.downcast_ref::<String>() {
                    Err(format!("Internal error: {}", s))
                } else {
                    Err("Internal error".to_string())
                }
            }
        },
    }
}

// runs f and always runs restore afterwards, re-raising any panic (try/finally)
pub fn finally<T>(f: impl FnOnce() -> T, restore: impl FnOnce()) -> T {
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    restore();
    match r {
        Ok(v) => v,
        Err(p) => std::panic::resume_unwind(p),
    }
}

// keeps CalcError panics quiet; real bugs still print
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if info.payload().downcast_ref::<CalcError>().is_some() {
            return;
        }
        prev(info);
    }));
}