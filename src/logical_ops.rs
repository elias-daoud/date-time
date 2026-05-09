use crate::model::{Accuracy, Moment, Period, Sign};
// rhs as in arithmetic is right hand side, lhs is left hand side
// used to make this simpler
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Interval {
    pub start: i128,
    pub end: i128,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AllenRelation {
    Before,
    After,
    Meets,
    Overlaps,
    Contains,
    Starts,
    Equal,
    StartedBy,
    FinishedBy,
    During,
    Finishes,
    OverlappedBy,
    MetBy,
}

pub fn compare_moments(lhs: &Moment, rhs: &Moment) -> Result<AllenRelation, String> {
    check_moment_compatible(lhs, rhs)?;

    let lhs_interval = moment_to_interval(lhs)?;
    let rhs_interval = moment_to_interval(rhs)?;

    allen_relations(lhs_interval, rhs_interval)
}

// Should be noted that period comparison is duration comparison, so after can be seen as 'longer'
// Moment logical semantics do not transfer to Period logical cases
pub fn compare_periods(lhs: &Period, rhs: &Period) -> Result<AllenRelation, String> {
    check_period_compatible(lhs, rhs)?;

    let lhs_interval = period_to_interval(lhs)?;
    let rhs_interval = period_to_interval(rhs)?;

    allen_relations(lhs_interval, rhs_interval)
}

pub fn allen_relations(a: Interval, b: Interval) -> Result<AllenRelation, String> {
    validate_interval(a)?;
    validate_interval(b)?;

    if a.end < b.start {
        return Ok(AllenRelation::Before);
    }
    if a.end == b.start {
        return Ok(AllenRelation::Meets);
    }

    if a.start < b.start && b.end < a.end {
        return Ok(AllenRelation::Contains);
    }

    if a.start < b.start && b.start < a.end && a.end < b.end {
        return Ok(AllenRelation::Overlaps);
    }

    if a.start < b.start && a.end == b.end {
        return Ok(AllenRelation::FinishedBy);
    }

    if a.start == b.start && a.end < b.end {
        return Ok(AllenRelation::Starts);
    }

    if a.start == b.start && a.end == b.end {
        return Ok(AllenRelation::Equal);
    }
    if b.start < a.start && a.end < b.end {
        return Ok(AllenRelation::During);
    }

    if a.start == b.start && a.end > b.end {
        return Ok(AllenRelation::StartedBy);
    }

    if b.start < a.start && a.end == b.end {
        return Ok(AllenRelation::Finishes);
    }

    if b.start < a.start && a.start < b.end && b.end < a.end {
        return Ok(AllenRelation::OverlappedBy);
    }

    if b.end == a.start {
        return Ok(AllenRelation::MetBy);
    }

    if b.end < a.start {
        return Ok(AllenRelation::After);
    }

    Err("Unable to classify interval relation".to_string())
}

fn validate_interval(interval: Interval) -> Result<(), String> {
    if interval.start >= interval.end {
        return Err(format!(
            "Interval is invalid, lower bound {} cannot be equal to or greater than upper bound {}",
            interval.start, interval.end
        ));
    }

    Ok(())
}

fn moment_to_interval(moment: &Moment) -> Result<Interval, String> {
    let bucket_width = bucket_width_for_time_resolution(moment.header.time_resolution_level)?;
    let day_width = day_width_for_time_resolution(moment.header.time_resolution_level)?;

    let date_portion = unsigned_to_signed(moment.date_value, "Moment date value")?
        .checked_mul(day_width)
        .ok_or("Moment date val overflow")?;

    let time_portion = match moment.time_value {
        Some(value) => unsigned_to_signed(value, "Moment time value")?
            .checked_mul(bucket_width)
            .ok_or("Moment time componetnt overflow")?,
        None => 0,
    };

    let bucket_start = date_portion
        .checked_add(time_portion)
        .ok_or("Moment interval overflow")?;

    let bucket_end = bucket_start
        .checked_add(bucket_width)
        .ok_or("Moment interval end overflow")?;

    let mut interval = match moment.header.accuracy {
        Accuracy::Start => Interval {
            start: bucket_start,
            end: bucket_start + 1,
        },
        Accuracy::Whole => Interval {
            start: bucket_start,
            end: bucket_end,
        },
        Accuracy::End => Interval {
            start: bucket_end - 1,
            end: bucket_end,
        },
    };

    if let Some(uncertainty) = moment.uncertainty_offset {
        let uncertainty_width = unsigned_to_signed(uncertainty, "Moment uncertainty offset")?
            .checked_mul(bucket_width)
            .ok_or("Moment uncertainty overflw")?;

        interval.start = interval
            .start
            .checked_sub(uncertainty_width)
            .ok_or("Moment lower uncertainty bound overflow")?;

        interval.end = interval
            .end
            .checked_add(uncertainty_width)
            .ok_or("Moment upper uncertainty offset bound overflow")?;
    }

    validate_interval(interval)?;

    Ok(interval)
}

fn period_to_interval(period: &Period) -> Result<Interval, String> {
    let bucket_width = bucket_width_for_time_resolution(period.header.time_resolution_level)?;
    let day_width = day_width_for_time_resolution(period.header.time_resolution_level)?;

    let date_portion = unsigned_to_signed(period.date_duration, "Period date value")?
        .checked_mul(day_width)
        .ok_or("Period date val overflow")?;

    let time_portion = match period.time_duration {
        Some(value) => unsigned_to_signed(value, "Moment time value")?
            .checked_mul(bucket_width)
            .ok_or("Moment time componetnt overflow")?,
        None => 0,
    };

    let base_duration = date_portion
        .checked_add(time_portion)
        .ok_or("Period dur overflow")?;

    let mut duration = base_duration;

    if period.header.sign == Sign::Negative {
        duration = duration
            .checked_neg()
            .ok_or("Period duration negation overflow")?;
    }

    let mut interval = Interval {
        start: duration,
        end: duration + 1,
    };

    if let Some(uncertainty) = period.uncertainty_offset {
        let uncertainty_width = unsigned_to_signed(uncertainty, "Period uncertainty offset")?
            .checked_mul(bucket_width)
            .ok_or("Period uncertainty overflow")?;

        interval.start = interval
            .start
            .checked_sub(uncertainty_width)
            .ok_or("Period lower uncertainty bound overflow")?;

        interval.end = interval
            .end
            .checked_add(uncertainty_width)
            .ok_or("Period upper uncertainty offset bound overflow")?;
    }

    validate_interval(interval)?;

    Ok(interval)
}

fn bucket_width_for_time_resolution(time_level: u8) -> Result<i128, String> {
    match time_level {
        0 => Ok(24), // no time, day represented in terms of hour granules
        4 => Ok(60), // minute level, second granules
        5 => Ok(1_000),

        _ => Err(format!(
            "Interval conversion for time level {} in logical operation is future work for this implementation scope",
            time_level
        )),
    }
}

fn day_width_for_time_resolution(time_level: u8) -> Result<i128, String> {
    match time_level {
        0 => Ok(24),         // no time, day represented in terms of hour granules
        4 => Ok(86_400),     // minute level, second granules for the day
        5 => Ok(86_400_000), // second level represented over milliseconds

        _ => Err(format!(
            "Day Interval conversion for time level {} in logical operation is future work for this implementation scope",
            time_level
        )),
    }
}

fn moment_has_nonzero_leap(moment: &Moment) -> bool {
    moment.positive_leap_seconds.unwrap_or(0) != 0 || moment.negative_leap_seconds.unwrap_or(0) != 0
}

fn period_has_nonzero_leap(period: &Period) -> bool {
    period.positive_leap_seconds.unwrap_or(0) != 0 || period.negative_leap_seconds.unwrap_or(0) != 0
}

fn check_moment_compatible(lhs: &Moment, rhs: &Moment) -> Result<(), String> {
    if moment_has_nonzero_leap(lhs) || moment_has_nonzero_leap(rhs) {
        return Err("Logical comparison with nonzero leap seconds is future work in implementation, although defined in model".to_string());
    }

    if lhs.header.accuracy != rhs.header.accuracy {
        return Err("Moment logical comparison only supports equal accuracy kinds as for arithmetic, future work".to_string());
    }

    if lhs.header.date_range_level != rhs.header.date_range_level {
        return Err(
      "Moment logical comparison needs equal date level range per limited implementation scope".to_string(),
    );
    }

    if lhs.header.time_resolution_level != rhs.header.time_resolution_level {
        return Err(
        "Moment logical comparison needs equal time resolution levels per model arithmetic and logical operation constraint".to_string(),
      );
    }

    if lhs.header.zone_level != rhs.header.zone_level || lhs.zone_value != rhs.zone_value {
        return Err("Moment logical comparison needs equal time zone values per limited implemtnatino scope".to_string());
    };

    Ok(())
}

fn check_period_compatible(lhs: &Period, rhs: &Period) -> Result<(), String> {
    if period_has_nonzero_leap(lhs) || period_has_nonzero_leap(rhs) {
        return Err("Logical comparison with nonzero leap seconds is future work in implementation, although defined in model".to_string());
    }
    if lhs.header.date_range_level != rhs.header.date_range_level {
        return Err(
      "Period logical comparison needs equal date level range per limited implementation scope".to_string(),
    );
    }
    if lhs.header.time_resolution_level != rhs.header.time_resolution_level {
        return Err(
        "Period logical comparison needs equal time resolution levels per model arithmetic and logical operation constraint".to_string(),
      );
    }

    Ok(())
}

fn unsigned_to_signed(value: u128, field_name: &str) -> Result<i128, String> {
    if value > i128::MAX as u128 {
        return Err(format!(
            "{} too large for logic implementation ",
            field_name
        ));
    }

    Ok(value as i128)
}
