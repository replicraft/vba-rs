//! `VBA.Financial` (MS-VBAL §6.1.2.5 and the Microsoft language reference).
//!
//! Cash-flow sign convention: money paid out is negative, money received is
//! positive. `due` is `vbEndOfPeriod` (0) or `vbBegOfPeriod` (non-zero).
//! `IRR` and `Rate` use Newton iteration with at most 20 tries and an
//! accuracy of 0.00001 percent (1e-7), as documented; failure to converge is
//! error 5. Microsoft does not publish the exact iteration, so results agree
//! with Office only to about that accuracy.

use crate::call::{Args, Ctx};
use crate::error::StdResult;
use vba_value::{Variant, VbaError, VbaResult, convert};

const E5: VbaError = VbaError::INVALID_PROCEDURE_CALL;

fn finite(x: f64) -> VbaResult<f64> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(VbaError::OVERFLOW)
    }
}

fn t(due: bool) -> f64 {
    if due { 1.0 } else { 0.0 }
}

/// `FV`.
pub fn fv(rate: f64, nper: f64, pmt: f64, pv: f64, due: bool) -> VbaResult<f64> {
    if rate == 0.0 {
        return finite(-(pv + pmt * nper));
    }
    let temp = (1.0 + rate).powf(nper);
    finite(-(pv * temp + pmt * (1.0 + rate * t(due)) * (temp - 1.0) / rate))
}

/// `PV`.
pub fn pv(rate: f64, nper: f64, pmt: f64, fv: f64, due: bool) -> VbaResult<f64> {
    if rate == 0.0 {
        return finite(-(fv + pmt * nper));
    }
    let temp = (1.0 + rate).powf(nper);
    finite(-(fv + pmt * (1.0 + rate * t(due)) * (temp - 1.0) / rate) / temp)
}

/// `Pmt`; `nper = 0` is error 5.
pub fn pmt(rate: f64, nper: f64, pv: f64, fv: f64, due: bool) -> VbaResult<f64> {
    if nper == 0.0 {
        return Err(E5);
    }
    if rate == 0.0 {
        return finite(-(fv + pv) / nper);
    }
    let temp = (1.0 + rate).powf(nper);
    let denom = (1.0 + rate * t(due)) * (temp - 1.0);
    if denom == 0.0 {
        return Err(E5);
    }
    finite(-(fv + pv * temp) * rate / denom)
}

/// `NPer`.
pub fn nper(rate: f64, pmt: f64, pv: f64, fv: f64, due: bool) -> VbaResult<f64> {
    if rate == 0.0 {
        if pmt == 0.0 {
            return Err(E5);
        }
        return finite(-(pv + fv) / pmt);
    }
    if rate <= -1.0 {
        return Err(E5);
    }
    let p1 = pmt * (1.0 + rate * t(due));
    let (num, den) = (p1 - fv * rate, pv * rate + p1);
    if den == 0.0 || num / den <= 0.0 {
        return Err(E5);
    }
    finite((num / den).ln() / (1.0 + rate).ln())
}

/// `IPmt`; `per` must be within `1..=nper`.
pub fn ipmt(rate: f64, per: f64, nper: f64, pv: f64, fv: f64, due: bool) -> VbaResult<f64> {
    if per < 1.0 || per > nper {
        return Err(E5);
    }
    let p = pmt(rate, nper, pv, fv, due)?;
    if due && per == 1.0 {
        return Ok(0.0);
    }
    let (pv2, k) = if due {
        (pv + p, per - 2.0)
    } else {
        (pv, per - 1.0)
    };
    finite(self::fv(rate, k, p, pv2, false)? * rate)
}

/// `PPmt`.
pub fn ppmt(rate: f64, per: f64, nper: f64, pv: f64, fv: f64, due: bool) -> VbaResult<f64> {
    Ok(pmt(rate, nper, pv, fv, due)? - ipmt(rate, per, nper, pv, fv, due)?)
}

/// `NPV`: values are discounted from the end of the first period.
pub fn npv(rate: f64, values: &[f64]) -> VbaResult<f64> {
    if rate <= -1.0 {
        return Err(E5);
    }
    let mut sum = 0.0;
    for (i, v) in values.iter().enumerate() {
        sum += v / (1.0 + rate).powi(i as i32 + 1);
    }
    finite(sum)
}

fn newton(mut r: f64, f: impl Fn(f64) -> f64) -> VbaResult<f64> {
    for _ in 0..20 {
        let fr = f(r);
        if !fr.is_finite() {
            return Err(E5);
        }
        let h = 1e-7 * (1.0 + r.abs());
        let d = (f(r + h) - f(r - h)) / (2.0 * h);
        if d == 0.0 || !d.is_finite() {
            return Err(E5);
        }
        let mut step = fr / d;
        // Damp steps that leave the domain or do not reduce the residual.
        let mut next = r - step;
        for _ in 0..30 {
            if next > -1.0 && f(next).abs() <= fr.abs() * 1.0000001 {
                break;
            }
            step /= 2.0;
            next = r - step;
        }
        if next <= -1.0 {
            return Err(E5);
        }
        if (next - r).abs() < 1e-7 {
            return Ok(next);
        }
        r = next;
    }
    Err(E5)
}

/// `IRR`.
pub fn irr(values: &[f64], guess: f64) -> VbaResult<f64> {
    if !(values.iter().any(|&v| v > 0.0) && values.iter().any(|&v| v < 0.0)) {
        return Err(E5);
    }
    newton(guess, |r| {
        values
            .iter()
            .enumerate()
            .map(|(i, v)| v / (1.0 + r).powi(i as i32))
            .sum()
    })
}

/// `MIRR`.
pub fn mirr(values: &[f64], finance_rate: f64, reinvest_rate: f64) -> VbaResult<f64> {
    let n = values.len();
    let pos: Vec<f64> = values.iter().map(|&v| v.max(0.0)).collect();
    let neg: Vec<f64> = values.iter().map(|&v| v.min(0.0)).collect();
    if n < 2 || !values.iter().any(|&v| v > 0.0) || !values.iter().any(|&v| v < 0.0) {
        return Err(E5);
    }
    let num = -npv(reinvest_rate, &pos)? * (1.0 + reinvest_rate).powi(n as i32);
    let den = npv(finance_rate, &neg)? * (1.0 + finance_rate);
    finite((num / den).powf(1.0 / (n as f64 - 1.0)) - 1.0)
}

/// `Rate`.
pub fn rate(nper: f64, pmt: f64, pv: f64, fv: f64, due: bool, guess: f64) -> VbaResult<f64> {
    if nper <= 0.0 {
        return Err(E5);
    }
    newton(guess, |r| {
        if r == 0.0 {
            pv + pmt * nper + fv
        } else {
            let tmp = (1.0 + r).powf(nper);
            pv * tmp + pmt * (1.0 + r * t(due)) * (tmp - 1.0) / r + fv
        }
    })
}

/// `SLN`; `life <= 0` is error 5.
pub fn sln(cost: f64, salvage: f64, life: f64) -> VbaResult<f64> {
    if life <= 0.0 {
        return Err(E5);
    }
    finite((cost - salvage) / life)
}

/// `SYD`.
pub fn syd(cost: f64, salvage: f64, life: f64, period: f64) -> VbaResult<f64> {
    if salvage < 0.0 || life <= 0.0 || period <= 0.0 || period > life {
        return Err(E5);
    }
    finite((cost - salvage) * (life - period + 1.0) * 2.0 / (life * (life + 1.0)))
}

/// `DDB`. A whole period iterates the documented declining balance; a
/// fractional period uses the book value `max(salvage, cost * (1 - factor /
/// life) ^ t)` at the two ends of the period (**unverified** against Office).
pub fn ddb(cost: f64, salvage: f64, life: f64, period: f64, factor: f64) -> StdResult<f64> {
    if cost < 0.0 || salvage < 0.0 || life <= 0.0 || period <= 0.0 || period > life || factor <= 0.0
    {
        return Err(E5.into());
    }
    if period.fract() != 0.0 {
        let keep = (1.0 - factor / life).max(0.0);
        let book = |t: f64| (cost * keep.powf(t)).max(salvage.min(cost));
        let dep = book((period - 1.0).max(0.0)) - book(period);
        return Ok(finite(dep)?);
    }
    let mut book = cost;
    let mut dep = 0.0;
    for _ in 0..period as u64 {
        dep = (book * factor / life).min((book - salvage).max(0.0));
        book -= dep;
    }
    Ok(finite(dep)?)
}

// ---------------------------------------------------------------------------
// Registry adapters.

fn values_arg(a: &Args<'_>, i: usize) -> StdResult<Vec<f64>> {
    let Variant::Array(arr) = a.req(i)? else {
        return Err(E5.into());
    };
    Ok(arr
        .iter()
        .map(convert::to_double)
        .collect::<VbaResult<Vec<_>>>()?)
}

fn due_arg(a: &Args<'_>, i: usize) -> StdResult<bool> {
    Ok(a.long_or(i, 0)? != 0)
}

macro_rules! dbl {
    ($e:expr) => {
        Ok(Variant::Double($e?))
    };
}

pub(crate) fn f_fv(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(fv(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double_or(3, 0.0)?,
        due_arg(a, 4)?
    ))
}
pub(crate) fn f_pv(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(pv(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double_or(3, 0.0)?,
        due_arg(a, 4)?
    ))
}
pub(crate) fn f_pmt(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(pmt(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double_or(3, 0.0)?,
        due_arg(a, 4)?
    ))
}
pub(crate) fn f_nper(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(nper(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double_or(3, 0.0)?,
        due_arg(a, 4)?
    ))
}
pub(crate) fn f_ipmt(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(ipmt(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double(3)?,
        a.double_or(4, 0.0)?,
        due_arg(a, 5)?
    ))
}
pub(crate) fn f_ppmt(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(ppmt(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double(3)?,
        a.double_or(4, 0.0)?,
        due_arg(a, 5)?
    ))
}
pub(crate) fn f_npv(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(npv(a.double(0)?, &values_arg(a, 1)?))
}
pub(crate) fn f_irr(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(irr(&values_arg(a, 0)?, a.double_or(1, 0.1)?))
}
pub(crate) fn f_mirr(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(mirr(&values_arg(a, 0)?, a.double(1)?, a.double(2)?))
}
pub(crate) fn f_rate(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(rate(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double_or(3, 0.0)?,
        due_arg(a, 4)?,
        a.double_or(5, 0.1)?
    ))
}
pub(crate) fn f_sln(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(sln(a.double(0)?, a.double(1)?, a.double(2)?))
}
pub(crate) fn f_syd(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    dbl!(syd(a.double(0)?, a.double(1)?, a.double(2)?, a.double(3)?))
}
pub(crate) fn f_ddb(_: &mut Ctx<'_>, a: &Args<'_>) -> StdResult<Variant> {
    Ok(Variant::Double(ddb(
        a.double(0)?,
        a.double(1)?,
        a.double(2)?,
        a.double(3)?,
        a.double_or(4, 2.0)?,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "{a} vs {b}");
    }

    // Reference values are the worked examples of the Excel financial
    // function documentation, whose definitions VBA shares.
    #[test]
    fn worked_examples() {
        close(
            pmt(0.08 / 12.0, 10.0, 10000.0, 0.0, false).unwrap(),
            -1037.03,
            0.005,
        );
        close(
            fv(0.06 / 12.0, 10.0, -200.0, -500.0, true).unwrap(),
            2581.40,
            0.005,
        );
        close(
            pv(0.08 / 12.0, 240.0, 500.0, 0.0, false).unwrap(),
            -59777.15,
            0.005,
        );
        close(
            nper(0.12 / 12.0, -100.0, -1000.0, 10000.0, true).unwrap(),
            59.67,
            0.005,
        );
        close(
            rate(48.0, -200.0, 8000.0, 0.0, false, 0.1).unwrap(),
            0.007701472,
            1e-7,
        );
        close(
            ipmt(0.1 / 12.0, 1.0, 36.0, 8000.0, 0.0, false).unwrap(),
            -66.67,
            0.005,
        );
        close(
            ppmt(0.1 / 12.0, 1.0, 24.0, 2000.0, 0.0, false).unwrap(),
            -75.62,
            0.005,
        );
        close(
            npv(0.1, &[-10000.0, 3000.0, 4200.0, 6800.0]).unwrap(),
            1188.44,
            0.005,
        );
        close(
            irr(&[-70000.0, 12000.0, 15000.0, 18000.0, 21000.0], 0.1).unwrap(),
            -0.0212448,
            1e-6,
        );
        close(
            irr(
                &[-70000.0, 12000.0, 15000.0, 18000.0, 21000.0, 26000.0],
                0.1,
            )
            .unwrap(),
            0.0866309,
            1e-6,
        );
        close(
            mirr(
                &[-120000.0, 39000.0, 30000.0, 21000.0, 37000.0, 46000.0],
                0.10,
                0.12,
            )
            .unwrap(),
            0.126094,
            1e-6,
        );
        close(sln(30000.0, 7500.0, 10.0).unwrap(), 2250.0, 1e-9);
        close(syd(30000.0, 7500.0, 10.0, 1.0).unwrap(), 4090.91, 0.005);
        close(syd(30000.0, 7500.0, 10.0, 10.0).unwrap(), 409.09, 0.005);
        close(ddb(2400.0, 300.0, 10.0, 1.0, 2.0).unwrap(), 480.0, 1e-9);
        close(ddb(2400.0, 300.0, 10.0, 2.0, 2.0).unwrap(), 384.0, 1e-9);
        close(ddb(2400.0, 300.0, 10.0, 10.0, 2.0).unwrap(), 22.12, 0.005);
    }

    #[test]
    fn identities_and_errors() {
        // Interest plus principal is the payment; principal repays the loan.
        let (r, n, p) = (0.05 / 12.0, 36.0, 10000.0);
        let pm = pmt(r, n, p, 0.0, false).unwrap();
        let mut total = 0.0;
        for k in 1..=36 {
            let (i, pp) = (
                ipmt(r, k as f64, n, p, 0.0, false).unwrap(),
                ppmt(r, k as f64, n, p, 0.0, false).unwrap(),
            );
            close(i + pp, pm, 1e-9);
            total += pp;
        }
        close(total, -p, 1e-6);
        assert_eq!(irr(&[1.0, 2.0], 0.1), Err(E5));
        assert_eq!(ipmt(0.01, 0.0, 12.0, 100.0, 0.0, false), Err(E5));
        assert_eq!(sln(1.0, 0.0, 0.0), Err(E5));
        assert_eq!(syd(1.0, 0.0, 5.0, 6.0), Err(E5));
        close(fv(0.0, 10.0, -100.0, 0.0, false).unwrap(), 1000.0, 1e-9);
    }
}
