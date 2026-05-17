use crate::model::{Accuracy, Moment, MomentHeader, Period, PeriodHeader, Sign};
use std::collections::HashMap;

pub fn moment_to_notation(moment: &Moment) -> Result<String, String> {
    let (year, month, day) = jdn_to_gregorian(moment.date_value)?;
    let mut base = format!("{:04}-{:02}-{:02}", year, month, day);

    match moment.header.time_resolution_level {
        0 => {}
        4 => {
            let time = moment
                .time_value
                .ok_or("Moment has minute time level but no value")?;

            if time >= 24 * 60 {
                return Err("Moment minute value must be below 1440".to_string());
            }

            base.push('T');
            base.push_str(&format_hours_minutes(time)?);
        }
        _ => {
            return Err("Notation has limited scope at the moment, only for time levels 0 (no time, date only) and level 4 (minute)".to_string());
            // This constraint is necessary otherwise balloons to library level implementation
        }
    }

    if moment.header.zone_level > 0 {
        let zone = moment
            .zone_value
            .ok_or("Moment has zone level but no value")?;

        base.push_str(&format_zone_offset(zone, moment.header.zone_level)?);
    }

    let positive_leap_seconds = moment.positive_leap_seconds.unwrap_or(0);
    let negative_leap_seconds = moment.negative_leap_seconds.unwrap_or(0);

    let mut output = Vec::new();
    output.push(format!("d:{}", moment.header.date_range_level));
    output.push(format!("t:{}", moment.header.time_resolution_level));
    output.push(format!("z:{}", moment.header.zone_level));
    output.push(format!("a:{}", accuracy_to_string(moment.header.accuracy)));
    output.push(format!(
        "l:{}-{}",
        positive_leap_seconds, negative_leap_seconds
    ));

    if moment.header.has_uncertainty {
        output.push(format!("u:{}", moment.uncertainty_offset.unwrap_or(0)));
    }

    if moment.header.lsl_status != 0 {
        let lsl_jdn = moment.lsl_jdn.ok_or("Moment has LSL status but no value")?;

        output.push(format!("lsv:{}", lsl_jdn));
    }

    Ok(format!("@G{} {{{}}}@", base, output.join(" ")))
}

pub fn period_to_notation(period: &Period) -> Result<String, String> {
    let sign = match period.header.sign {
        Sign::Positive => "+",
        Sign::Negative => "-",
    };

    let mut base = format!("P{}{}D", sign, period.date_duration);

    match period.header.time_resolution_level {
        0 => {}
        4 => {
            let time = period
                .time_duration
                .ok_or("Moment has minute time level but no value")?;

            base.push('T');
            base.push_str(&format_hours_minutes(time)?);
        }
        _ => {
            return Err("Notation has limited scope at the moment, only for time levels 0 (no time, date only) and level 4 (minute)".to_string());
            // This constraint is necessary otherwise balloons to library level implementation
        }
    }

    let positive_leap_seconds = period.positive_leap_seconds.unwrap_or(0);
    let negative_leap_seconds = period.negative_leap_seconds.unwrap_or(0);

    let mut output = Vec::new();
    output.push(format!("d:{}", period.header.date_range_level));
    output.push(format!("t:{}", period.header.time_resolution_level));
    output.push(format!(
        "l:{}-{}",
        positive_leap_seconds, negative_leap_seconds
    ));

    if period.header.has_uncertainty {
        output.push(format!("u:{}", period.uncertainty_offset.unwrap_or(0)));
    }

    if period.header.lsl_status != 0 {
        let lsl_jdn = period
            .lsl_jdn
            .ok_or("Period has lsl status but no jdn val")?;

        output.push(format!("lsv:{}", lsl_jdn));
    }

    Ok(format!("@{} {{{}}}@", base, output.join(" ")))
}

pub fn parse_moment_notation(input: &str) -> Result<Moment, String> {
    let (base, metadata) = split_notation(input)?;

    let base = base
        .strip_prefix('G')
        .ok_or("Moment notation must start with G".to_string())?;

    let date_range_level = required_unsigned_byte(&metadata, "d")?;
    let time_resolution_level = required_unsigned_byte(&metadata, "t")?;
    let zone_level = required_unsigned_byte(&metadata, "z")?;
    let accuracy = get_accuracy(&metadata)?;
    let (positive_leap, negative_leap) = get_leap_pair(&metadata)?;

    if time_resolution_level != 0 && time_resolution_level != 4 {
        return Err("Notation parsing currently only supports time levels 0 and 4".to_string());
    }

    let (base_without_zone, zone_value) = parse_zone_suffix(base, zone_level)?;

    let (date_portion, time_portion) = match base_without_zone.split_once('T') {
        Some((date, time)) => (date.trim(), Some(time.trim())),
        None => (base_without_zone.trim(), None),
    };

    let date_value = gregorian_to_jdn_handler(date_portion)?;

    let time_value = match time_resolution_level {
        0 => {
            if time_portion.is_some() {
                return Err("Moment with t:0 cant contain time part".to_string());
            }

            None
        }

        4 => {
            let time = time_portion.ok_or("Moment with t:4 must have HH:MM")?;
            Some(parse_hours_minutes(time, false)?)
        }
        _ => {
            unreachable!()
        }
    };

    let uncertainty_offset = optional_u128(&metadata, "u")?;
    let uncertainty_offset = match uncertainty_offset {
        Some(0) | None => None,
        Some(value) => Some(value),
    };
    let has_uncertainty = uncertainty_offset.is_some();

    let lsl_jdn = optional_u64(&metadata, "lsv")?;
    let lsl_jdn = match lsl_jdn {
        Some(0) | None => None,
        Some(value) => Some(value),
    };

    let lsl_status = match lsl_jdn {
        Some(jdn) => needed_lsl_status(jdn)?,
        None => 0,
    };

    let leap_counter_length = needed_leap_counter_length(positive_leap.max(negative_leap))?;
    let (positive_leap_seconds, negative_leap_seconds) = leap_options(positive_leap, negative_leap);

    Ok(Moment {
        header: MomentHeader {
            time_resolution_level,
            date_range_level,
            zone_level,
            accuracy,
            leap_counter_length,
            has_uncertainty,
            lsl_status,
        },
        date_value,
        time_value,
        zone_value,
        positive_leap_seconds,
        negative_leap_seconds,
        uncertainty_offset,
        lsl_jdn,
    })
}

pub fn parse_period_notation(input: &str) -> Result<Period, String> {
    let (base, metadata) = split_notation(input)?;

    let date_range_level = required_unsigned_byte(&metadata, "d")?;
    let time_resolution_level = required_unsigned_byte(&metadata, "t")?;
    let (positive_leap, negative_leap) = get_leap_pair(&metadata)?;

    if time_resolution_level != 0 && time_resolution_level != 4 {
        return Err("Notation parsing currently only supports time levels 0 and 4".to_string());
    }

    let body = base
        .strip_prefix('P')
        .ok_or("Invalid Period notation, must start with P".to_string())?;

    let (sign, rest) = if let Some(rest) = body.strip_prefix('-') {
        (Sign::Negative, rest)
    } else if let Some(rest) = body.strip_prefix('+') {
        (Sign::Positive, rest)
    } else {
        (Sign::Positive, body)
    };

    let (date_portion, time_portion) = match rest.split_once('T') {
        Some((date, time)) => (date.trim(), Some(time.trim())),
        None => (rest.trim(), None),
    };

    let date_duration = date_portion
        .strip_suffix('D')
        .ok_or("Period date portion must end with D".to_string())?
        .parse::<u128>()
        .map_err(|_| "Invalid Period date duration value".to_string())?;

    let time_duration = match time_resolution_level {
        0 => {
            if time_portion.is_some() {
                return Err("Peroid with t:0 cant contain time part".to_string());
            }

            None
        }

        4 => {
            let time = time_portion.ok_or("Period with t:4 must have HH:MM")?;
            Some(parse_hours_minutes(time, true)?)
        }
        _ => {
            unreachable!()
        }
    };

    let uncertainty_offset = optional_u128(&metadata, "u")?;
    let uncertainty_offset = match uncertainty_offset {
        Some(0) | None => None,
        Some(value) => Some(value),
    };
    let has_uncertainty = uncertainty_offset.is_some();

    let lsl_jdn = optional_u64(&metadata, "lsv")?;
    let lsl_jdn = match lsl_jdn {
        Some(0) | None => None,
        Some(value) => Some(value),
    };

    let lsl_status = match lsl_jdn {
        Some(jdn) => needed_lsl_status(jdn)?,
        None => 0,
    };
    // The reason this is kept for Period is LSL can also be set per model design but with several
    // constraints

    let leap_counter_length = needed_leap_counter_length(positive_leap.max(negative_leap))?;
    let (positive_leap_seconds, negative_leap_seconds) = leap_options(positive_leap, negative_leap);

    Ok(Period {
        header: PeriodHeader {
            sign,
            date_range_level,
            time_resolution_level,
            leap_counter_length,
            has_uncertainty,
            lsl_status,
        },
        date_duration,
        time_duration,
        positive_leap_seconds,
        negative_leap_seconds,
        uncertainty_offset,
        lsl_jdn,
    })
}

fn needed_lsl_status(jdn: u64) -> Result<u8, String> {
    if jdn <= 0xFF_FFFF {
        Ok(1)
    } else if jdn <= 0xFFFF_FFFF {
        Ok(2)
    } else if jdn <= 0xFFFF_FFFF_FFFF {
        Ok(3)
    } else {
        Err("JDN leap second list version needs more than the supported 6 bytes or 770B years coverage".to_string())
    }
}

fn parse_zone_suffix(base: &str, zone_level: u8) -> Result<(&str, Option<u128>), String> {
    if zone_level == 0 {
        if has_zone_offset(base) {
            return Err("Moment with z:0 cant contain zone suffix".to_string());
        }

        return Ok((base.trim(), None));
    }

    if zone_level != 1 {
        return Err("Notation parsing only supports zone level 1 for the time being".to_string());
    }

    let Some(index) = find_zone_offset_index(base) else {
        return Err("Moment with zone level 1 must end with +HH:MM or -HH:MM".to_string());
    };

    let date_time = base[..index].trim();
    let offset = base[index..].trim();

    let zone_value = parse_zone_offset_level_one(offset)?;

    Ok((date_time, Some(zone_value)))
}

fn required_unsigned_byte(metadata: &HashMap<String, String>, key: &str) -> Result<u8, String> {
    metadata
        .get(key)
        .ok_or(format!("Missing required field '{}'", key))?
        .parse::<u8>()
        .map_err(|_| format!("Invalid byte property '{}'", key))
}

fn optional_u128(metadata: &HashMap<String, String>, key: &str) -> Result<Option<u128>, String> {
    match metadata.get(key) {
        Some(value) => {
            Ok(Some(value.parse::<u128>().map_err(|_| {
                format!("Invalid unsigned 16 byte property '{}'", key)
            })?))
        }
        None => Ok(None),
    }
}

fn optional_u64(metadata: &HashMap<String, String>, key: &str) -> Result<Option<u64>, String> {
    match metadata.get(key) {
        Some(value) => {
            Ok(Some(value.parse::<u64>().map_err(|_| {
                format!("Invalid unsigned 8 byte property '{}'", key)
            })?))
        }
        None => Ok(None),
    }
}

fn gregorian_to_jdn_handler(input: &str) -> Result<u128, String> {
    let parts: Vec<&str> = input.split('-').collect();

    if parts.len() != 3 {
        return Err("Date must be YYYY-MM-DD".to_string());
    }

    let year = parts[0]
        .parse::<i64>()
        .map_err(|_| "Invalid year in date".to_string())?;
    let month = parts[1]
        .parse::<i64>()
        .map_err(|_| "Invalid month in date".to_string())?;
    let day = parts[2]
        .parse::<i64>()
        .map_err(|_| "Invalid day in date".to_string())?;

    if !(1..=12).contains(&month) {
        return Err("Month must be within 1 to 12 range".to_string());
    }

    if !(1..=31).contains(&day) {
        return Err("Day must be within 1 to 31 range".to_string());
    }

    gregorian_to_jdn(year, month, day)
}

fn parse_hours_minutes(input: &str, is_duration: bool) -> Result<u128, String> {
    let parts: Vec<&str> = input.split(':').collect();

    if parts.len() != 2 {
        return Err("Minute level time must be HH:MM".to_string());
    }

    let hours = parts[0]
        .parse::<u128>()
        .map_err(|_| "Invalid hour value".to_string())?;

    let minutes = parts[1]
        .parse::<u128>()
        .map_err(|_| "Invalid minute value".to_string())?;

    if !is_duration && hours >= 24 {
        return Err("Moment hour cannot be equal to or more than 24".to_string());
    }

    if minutes >= 60 {
        return Err("Minute must be below 60".to_string());
    }

    Ok(hours * 60 + minutes)
}

fn get_leap_pair(metadata: &HashMap<String, String>) -> Result<(u128, u128), String> {
    let value = metadata
        .get("l")
        .ok_or("Missing leap property 'l'".to_string())?;
    let (positive, negative) = value
        .split_once('-')
        .ok_or("Leap property must be l:positive-negative format".to_string())?;

    let positive = positive
        .parse::<u128>()
        .map_err(|_| "Invalid positive leap seconds".to_string())?;

    let negative = negative
        .parse::<u128>()
        .map_err(|_| "Invalid negative leap seconds".to_string())?;

    Ok((positive, negative))
}

fn leap_options(positive: u128, negative: u128) -> (Option<u128>, Option<u128>) {
    if positive == 0 && negative == 0 {
        (None, None)
    } else {
        (Some(positive), Some(negative))
    }
}

fn needed_leap_counter_length(value: u128) -> Result<u8, String> {
    if value == 0 {
        Ok(0)
    } else if value <= 0xFF {
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
        Err("Leap counters need more than the suppoted 7 bytes".to_string())
    }
}

fn split_notation(input: &str) -> Result<(&str, HashMap<String, String>), String> {
    let trimmed = input.trim();

    if !trimmed.starts_with('@') || !trimmed.ends_with('@') {
        return Err("Invalid syntax, notation must start and end with @".to_string());
    }

    let inner = trimmed[1..trimmed.len() - 1].trim();

    let open = inner
        .find('{')
        .ok_or("notation must have metadata block.".to_string())?;

    let close = inner
        .rfind('}')
        .ok_or("notation metadata must end with }".to_string())?;

    if close < open {
        return Err("Metadata block is invalid".to_string());
    }

    let base = inner[..open].trim();
    let metadata = inner[open + 1..close].trim(); // strip { and }

    if !inner[close + 1..].trim().is_empty() {
        return Err("Invalid data after expected end of metadata blocj".to_string());
    }

    Ok((base, parse_properties(metadata)?))
}

fn has_zone_offset(base: &str) -> bool {
    find_zone_offset_index(base).is_some()
}

fn find_zone_offset_index(base: &str) -> Option<usize> {
    let t_index = base.find('T')?;
    let bytes = base.as_bytes();

    for i in t_index + 1..bytes.len() {
        if bytes[i] == b'+' || bytes[i] == b'-' {
            return Some(i);
        }
    }

    None
}

fn parse_zone_offset_level_one(offset: &str) -> Result<u128, String> {
    let negative = offset.starts_with('-');
    let positive = offset.starts_with('+');

    if !negative && !positive {
        return Err("Time zone offset must start with + or -".to_string());
    }

    let body = &offset[1..];

    let (hours, minutes) = body
        .split_once(':')
        .ok_or("Time zone offset must be HH:MM".to_string())?;

    let hours = hours
        .parse::<u128>()
        .map_err(|_| "Invalid time zone hour offset".to_string())?;

    let minutes = minutes
        .parse::<u128>()
        .map_err(|_| "Invalid tiem zone minute offset".to_string())?;

    if minutes >= 60 {
        return Err("Time zone minutes cannot be 60 or more".to_string());
    }

    let total_mins = hours * 60 + minutes;

    if total_mins % 15 != 0 {
        return Err("Time zone lvl 1 needs 15 minute offset units ".to_string());
    }

    let magnitude = total_mins / 15;

    if magnitude > 63 {
        return Err("Time zone offset too large for lvl 1".to_string());
    }

    if negative {
        Ok(64 + magnitude)
    } else {
        Ok(magnitude)
    }
}

pub fn format_zone_offset(zone_value: u128, zone_level: u8) -> Result<String, String> {
    if zone_level != 1 {
        return Err("Notation formatting supports only time zone level 1 at present".to_string());
    }

    if zone_value > 127 {
        return Err("Zone level 1 must fit in 7 bits".to_string());
    }

    let negative = zone_value >= 64;
    let magnitude = if negative {
        zone_value - 64
    } else {
        zone_value
    };

    let total_mins = magnitude * 15;
    let hours = total_mins / 60;
    let minutes = total_mins % 60;

    let sign = if negative { "-" } else { "+" };

    Ok(format!("{}{:02}:{:02}", sign, hours, minutes))
}

fn parse_properties(raw: &str) -> Result<HashMap<String, String>, String> {
    let mut output = HashMap::new();
    let normalized = raw.replace(',', " ");

    for token in normalized.split_whitespace() {
        let (key, value) = token
            .split_once(':')
            .ok_or(format!("Invalid property: '{}'", token))?;

        output.insert(key.trim().to_lowercase(), value.trim().to_lowercase());
    }
    Ok(output)
}

pub fn format_hours_minutes(value: u128) -> Result<String, String> {
    let hours = value / 60;
    let minutes = value % 60;

    Ok(format!("{:02}:{:02}", hours, minutes))
}

// Formula also based on Fliegel and van Flandern's algorithms referenced below
fn gregorian_to_jdn(year: i64, month: i64, day: i64) -> Result<u128, String> {
    let jdn = (day - 32075
        + 1461 * (year + 4800 + (month - 14) / 12) / 4
        + 367 * (month - 2 - (month - 14) / 12 * 12) / 12
        - 3 * ((year + 4900 + (month - 14) / 12) / 100) / 4);

    if jdn < 0 {
        return Err("Negative JDN not allowed in limited notation implementaiton".to_string());
    }

    Ok(jdn as u128)
}

fn get_accuracy(metadata: &HashMap<String, String>) -> Result<Accuracy, String> {
    let value = metadata
        .get("a")
        .ok_or("Missing required accuracy property 'a'".to_string())?;

    match value.as_str() {
        "s" | "start" => Ok(Accuracy::Start),
        "w" | "whole" => Ok(Accuracy::Whole),
        "e" | "end" => Ok(Accuracy::End),
        _ => Err(format!("Invalid accuracy kind specified: '{}' ", value)),
    }
}

fn accuracy_to_string(accuracy: Accuracy) -> &'static str {
    match accuracy {
        Accuracy::Start => "s",
        Accuracy::Whole => "w",
        Accuracy::End => "e",
    }
}

pub fn jdn_to_gregorian(jdn: u128) -> Result<(i64, i64, i64), String> {
    if jdn > i64::MAX as u128 {
        return Err("JDN value is too large for notation conversion".to_string());
        // JDN is defined up to 61 bits, this just guarding against unsafe input using type sizes
        // Moment and Period internal functions will deal with JDN over undefined values
    }

    // Fliegel and van Flandern's algorithm for JDN-Gregorian calendar conversion
    // From: https://aa.usno.navy.mil/faq/JD_formula
    let mut l = jdn as i64 + 68569;
    let n = (4 * l) / 146097;
    l -= (146097 * n + 3) / 4;
    let mut i = (4000 * (l + 1)) / 1461001;
    l = l - (1461 * i) / 4 + 31;
    let mut j = (80 * l) / 2447;
    let k = l - (2447 * j) / 80; // day
    l = j / 11;
    j = j + 2 - 12 * l; // month
    i = 100 * (n - 49) + i + l; // year

    Ok((i, j, k))
}
