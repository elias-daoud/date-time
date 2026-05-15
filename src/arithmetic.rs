use crate::model::{Moment, MomentHeader, Period, PeriodHeader, Sign};

// Limited set of arithmetic operations and rule constraints, deeper implementation is future work
pub fn period_add(lhs: &Period, rhs: &Period) -> Result<Period, String> {
    period_add_or_sub(lhs, rhs, false)
}

pub fn period_subtract(lhs: &Period, rhs: &Period) -> Result<Period, String> {
    if lhs == rhs {
        return Ok(zero_period_identical_period(lhs));
    }

    period_add_or_sub(lhs, rhs, true)
}

pub fn moment_subtract(lhs: &Moment, rhs: &Moment) -> Result<Period, String> {
    if lhs == rhs {
        return Ok(zero_period_identical_moment(lhs));
    }

    check_moment_compatibility(lhs, rhs)?;

    let lhs_total = moment_to_total_units(lhs)?;
    let rhs_total = moment_to_total_units(rhs)?;

    let difference = lhs_total
        .checked_sub(rhs_total)
        .ok_or("Moment subtraciton overflow")?;

    let units = units_per_day(lhs.header.time_resolution_level)?;
    let (sign, date_duration, time_duration) =
        signed_total_to_period_components(difference, units)?;

    let uncertainty_offset = add_uncertainty(lhs.uncertainty_offset, rhs.uncertainty_offset)?;

    let leap = effective_moment_leap(lhs)?
        .checked_sub(effective_moment_leap(rhs)?)
        .ok_or("Leap counters overflow on subtraction")?;

    let (leap_counter_length, positive_leap_seconds, negative_leap_seconds) =
        leap_fields_signed(leap)?;

    Ok(Period {
        header: PeriodHeader {
            sign,
            time_resolution_level: lhs.header.time_resolution_level,
            date_range_level: lhs.header.date_range_level,
            leap_counter_length,
            has_uncertainty: uncertainty_offset.is_some(),
            lsl_status: 0,
        },
        date_duration,
        time_duration: if lhs.header.time_resolution_level > 0 {
            Some(time_duration)
        } else {
            None
        },
        positive_leap_seconds,
        negative_leap_seconds,
        uncertainty_offset,
        lsl_jdn: None,
    })
}

pub fn moment_add_period(moment: &Moment, period: &Period) -> Result<Moment, String> {
    moment_add_or_subtract_period(moment, period, false)
}

pub fn moment_subtract_period(moment: &Moment, period: &Period) -> Result<Moment, String> {
    moment_add_or_subtract_period(moment, period, true)
}

fn moment_add_or_subtract_period(
    moment: &Moment,
    period: &Period,
    subtract_period: bool,
) -> Result<Moment, String> {
    check_moment_period_compatibility(moment, period)?;

    if period_has_nonzero_leap_seconds(period) {
        return Err(
            "Moment arithmetic with Period with nonzero leap seconds is future work".to_string(),
        );
    }

    let moment_total = moment_to_total_units(moment)?;
    let mut period_total = period_to_signed_total_units(period)?;

    if subtract_period {
        period_total = period_total
            .checked_neg()
            .ok_or("Period negation overflow")?;
    }

    let result_total = moment_total
        .checked_add(period_total)
        .ok_or("Moment plus minus period overflow")?;

    let (date, time_value) =
        total_units_to_moment_parts(result_total, moment.header.time_resolution_level)?;

    let uncertainty_offset = add_uncertainty(moment.uncertainty_offset, period.uncertainty_offset)?;

    let mut header: MomentHeader = moment.header;
    header.has_uncertainty = uncertainty_offset.is_some();

    Ok(Moment {
        header,
        date_value: date,
        time_value: time_value,
        zone_value: moment.zone_value,
        positive_leap_seconds: moment.positive_leap_seconds,
        negative_leap_seconds: moment.negative_leap_seconds,
        uncertainty_offset,
        lsl_jdn: moment.lsl_jdn,
    })
}

fn period_has_nonzero_leap_seconds(period: &Period) -> bool {
    period.positive_leap_seconds.unwrap_or(0) != 0 || period.negative_leap_seconds.unwrap_or(0) != 0
}

fn check_moment_period_compatibility(moment: &Moment, period: &Period) -> Result<(), String> {
    if moment.header.date_range_level != period.header.date_range_level {
        return Err("Moment and Period require same date range level".to_string());
    }

    if moment.header.time_resolution_level != period.header.time_resolution_level {
        return Err("Moment and Period require same time resolution level".to_string());
    }

    Ok(())
}

fn check_moment_compatibility(lhs: &Moment, rhs: &Moment) -> Result<(), String> {
    if lhs.header.date_range_level != rhs.header.date_range_level {
        return Err("Moment operands must have same date range level".to_string());
    }

    if lhs.header.time_resolution_level != rhs.header.time_resolution_level {
        return Err("Moment operands msut have same time level".to_string());
    }

    if lhs.header.accuracy != rhs.header.accuracy {
        return Err("Mixed accuracy kind artihmetic not supported".to_string());
    }

    if lhs.header.zone_level != rhs.header.zone_level || lhs.zone_value != rhs.zone_value {
        return Err("Moment operands require same time zone, otherwise operation unsupported in implementation".to_string());
    }

    Ok(())
}

fn zero_period_identical_period(period: &Period) -> Period {
    Period {
        header: PeriodHeader {
            sign: Sign::Positive,
            date_range_level: period.header.date_range_level,
            time_resolution_level: period.header.time_resolution_level,
            leap_counter_length: 0,
            has_uncertainty: false,
            lsl_status: 0,
        },
        date_duration: 0,
        time_duration: if period.header.time_resolution_level > 0 {
            Some(0)
        } else {
            None
        },
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    }
}

fn zero_period_identical_moment(moment: &Moment) -> Period {
    Period {
        header: PeriodHeader {
            sign: Sign::Positive,
            date_range_level: moment.header.date_range_level,
            time_resolution_level: moment.header.time_resolution_level,
            leap_counter_length: 0,
            has_uncertainty: false,
            lsl_status: 0,
        },
        date_duration: 0,
        time_duration: if moment.header.time_resolution_level > 0 {
            Some(0)
        } else {
            None
        },
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    }
}

fn period_add_or_sub(lhs: &Period, rhs: &Period, subtract_rhs: bool) -> Result<Period, String> {
    require_compatible_period(lhs, rhs)?;

    let lhs_total = period_to_signed_total_units(lhs)?;
    let mut rhs_total = period_to_signed_total_units(rhs)?;

    if subtract_rhs {
        rhs_total = rhs_total
            .checked_neg()
            .ok_or("Period negation invalid or overflow")?;
    }

    let total = lhs_total
        .checked_add(rhs_total)
        .ok_or("Period arithmetic overflow")?;

    let units = units_per_day(lhs.header.time_resolution_level)?;

    let (sign, date_duration, time_duration_value) =
        signed_total_to_period_components(total, units)?;

    let uncertainty_offset = add_uncertainty(lhs.uncertainty_offset, rhs.uncertainty_offset)?;

    let lhs_leap = effective_period_leap(lhs)?;
    let mut rhs_leap = effective_period_leap(rhs)?;

    if subtract_rhs {
        rhs_leap = rhs_leap.checked_neg().ok_or("Leap negation overflow")?;
    }

    let leap = lhs_leap
        .checked_add(rhs_leap)
        .ok_or("Leap counte raddition overflow")?;

    let (leap_counter_length, positive_leap_seconds, negative_leap_seconds) =
        leap_fields_signed(leap)?;

    Ok(Period {
        header: PeriodHeader {
            sign,
            time_resolution_level: lhs.header.time_resolution_level,
            date_range_level: lhs.header.date_range_level,
            leap_counter_length,
            has_uncertainty: uncertainty_offset.is_some(),
            lsl_status: 0,
        },
        date_duration,
        time_duration: if lhs.header.time_resolution_level > 0 {
            Some(time_duration_value)
        } else {
            None
        },
        positive_leap_seconds,
        negative_leap_seconds,
        uncertainty_offset,
        lsl_jdn: None,
    })
}

fn require_compatible_period(lhs: &Period, rhs: &Period) -> Result<(), String> {
    if lhs.header.date_range_level != rhs.header.date_range_level {
        return Err("Both operands must have same date range level".to_string());
        // introducing this and other limitations to keep the implementation simple, otherwise will balloon
        // very quickly and outgrow scope
    }

    if lhs.header.time_resolution_level != rhs.header.time_resolution_level {
        return Err("Both operands must have same time resolution".to_string());
    }

    Ok(())
}

fn unsigned_to_signed(value: u128, field_name: &str) -> Result<i128, String> {
    if value > i128::MAX as u128 {
        return Err(format!(
            "{field_name} is too large for limited arithmetic implementation"
        ));
    }

    Ok(value as i128)
}

fn unsigned_to_signed_optional(value: Option<u128>, field_name: &str) -> Result<i128, String> {
    match value {
        Some(value) => unsigned_to_signed(value, field_name),
        None => Ok(0),
    }
}

fn add_uncertainty(lhs: Option<u128>, rhs: Option<u128>) -> Result<Option<u128>, String> {
    let sum = lhs
        .unwrap_or(0)
        .checked_add(rhs.unwrap_or(0))
        .ok_or("Uncertainty addition error")?;

    if sum == 0 { Ok(None) } else { Ok(Some(sum)) }
}

fn effective_period_leap(period: &Period) -> Result<i128, String> {
    let positive = unsigned_to_signed(
        period.positive_leap_seconds.unwrap_or(0),
        "Positive leap seconds",
    )?;
    let negative = unsigned_to_signed(
        period.negative_leap_seconds.unwrap_or(0),
        "Negative leap seconds",
    )?;

    let net = positive
        .checked_sub(negative)
        .ok_or("Leap counter subtraction overflow")?;

    match period.header.sign {
        Sign::Positive => Ok(net),
        Sign::Negative => net
            .checked_neg()
            .ok_or("Leap counter negation overflowed".to_string()),
    }
}

fn effective_moment_leap(moment: &Moment) -> Result<i128, String> {
    let positive = unsigned_to_signed(
        moment.positive_leap_seconds.unwrap_or(0),
        "Positive leap seconds",
    )?;
    let negative = unsigned_to_signed(
        moment.negative_leap_seconds.unwrap_or(0),
        "Negative leap seconds",
    )?;

    positive
        .checked_sub(negative)
        .ok_or("Leap counters subtraction overflow".to_string())
}

fn leap_fields_signed(value: i128) -> Result<(u8, Option<u128>, Option<u128>), String> {
    if value == 0 {
        return Ok((0, None, None));
    }

    let magnitude = value.unsigned_abs();
    let length = needed_leap_counter_length(magnitude)?;

    if value > 0 {
        Ok((length, Some(magnitude), Some(0)))
    } else {
        Ok((length, Some(0), Some(magnitude)))
    }
}

fn needed_leap_counter_length(value: u128) -> Result<u8, String> {
    if value == 0 {
        return Ok(0);
    }

    if value <= 0xFF {
        Ok(1)
    } else if value <= 0xFFFF {
        Ok(2)
    } else if value <= 0xFF_FFFF {
        Ok(3)
    } else if value <= 0xFFFF_FFFF {
        Ok(4)
    } else if value <= 0xFF_FFFF_FFFF {
        Ok(5)
    } else if value <= 0xFFFF_FFFF_FFFF {
        Ok(6)
    } else if value <= 0xFF_FFFF_FFFF_FFFF {
        Ok(7)
    } else {
        Err("leap counter requires more than 7 bytes".to_string())
    }
}

fn units_per_day(time_level: u8) -> Result<i128, String> {
    match time_level {
        0 => Ok(1),    // date only
        1 => Ok(24),   // hours
        4 => Ok(1440), // minutes, preceding two levels would be 15 min & 5 min respectively but not supported here
        5 => Ok(86400),
        _ => Err(format!(
            "Unsupported time resolution level {} for curent arithmetic implementation",
            time_level
        )),
    }
}

fn moment_to_total_units(moment: &Moment) -> Result<i128, String> {
    let units = units_per_day(moment.header.time_resolution_level)?;
    let date = unsigned_to_signed(moment.date_value, "Moment date value")?;
    let time = unsigned_to_signed_optional(moment.time_value, "Moment time  value")?;

    if moment.header.time_resolution_level > 0 && time >= units {
        return Err("Time value outside valid range, invalid Moment properties".to_string());
    }

    date.checked_mul(units)
        .ok_or("Moment date arithmetc overflow")?
        .checked_add(time)
        .ok_or("Moment time artihmetic overflow".to_string())
}

fn period_to_signed_total_units(period: &Period) -> Result<i128, String> {
    let units = units_per_day(period.header.time_resolution_level)?;
    let date = unsigned_to_signed(period.date_duration, "Period date duration")?;
    let time = unsigned_to_signed_optional(period.time_duration, "Period time duration")?;

    let base = date
        .checked_mul(units)
        .ok_or("Period date overflow")?
        .checked_add(time)
        .ok_or("Period duration overflow")?;

    match period.header.sign {
        Sign::Positive => Ok(base),
        Sign::Negative => base
            .checked_neg()
            .ok_or("Period negation overflow".to_string()),
    }
}

fn signed_total_to_period_components(
    total: i128,
    units: i128,
) -> Result<(Sign, u128, u128), String> {
    if total == 0 {
        return Ok((Sign::Positive, 0, 0));
    }

    let sign = if total < 0 {
        Sign::Negative
    } else {
        Sign::Positive
    };

    let magnitude = if total < 0 {
        total.checked_neg().ok_or("Magnitude overflow")? as u128
    } else {
        total as u128
    };

    let units = units as u128;

    Ok((sign, magnitude / units, magnitude % units)) // reflecting JDN + time conversion as per design
}

fn total_units_to_moment_parts(
    total: i128,
    time_level: u8,
) -> Result<(u128, Option<u128>), String> {
    if total < 0 {
        return Err("Moment result before supported JDN range".to_string());
    }

    let units = units_per_day(time_level)?;
    let date = total.div_euclid(units) as u128;
    let time = total.rem_euclid(units) as u128; // conversion to granule, for seconds this is supported but deeper resolutions aren't at the moment, u128 not enough

    let time_value = if time_level > 0 { Some(time) } else { None };

    Ok((date, time_value))
}
