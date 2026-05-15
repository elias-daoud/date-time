use crate::model::{Moment, MomentHeader, Period, PeriodHeader};

pub fn write_u128_big_endian(
    value: u128,
    byte_length: usize,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    if byte_length > 16 {
        return Err("Not possible to write more than 16 bytes into a u128 value".to_string());
    }

    let bytes = value.to_be_bytes();
    out.extend_from_slice(&bytes[16 - byte_length..]); // use rangefrom to only capture needed bytes from big endian format
    Ok(())
}

pub fn read_u128_big_endian(bytes: &[u8]) -> u128 {
    let mut full = [0u8; 16]; // create byte array of 0u8, needed for big endian padding
    let start = 16 - bytes.len();
    full[start..].copy_from_slice(bytes); // copies the relevant bytes into the empty big endian array at right position
    u128::from_be_bytes(full) // returns u128
}

pub fn time_byte_length(time_resolution_level: u8) -> usize {
    match time_resolution_level {
        0 => 0, // no time value so date only, refer to thesis for full specification
        1 => 1,
        2 => 1,
        3 => 2,
        4 => 2,
        5 => 3, // seconds
        6 => 4, // milliseconds
        7 => 5,
        8 => 6,
        9 => 8,
        10 => 9, // femto seconds
        11 => 10,
        12 => 11, // zepto seconds
        13 => 13,
        14 => 14,
        15 => 15, // yocto X 10 ^ -6, or 10^-30 seconds
        16 => 16,
        // 16 byte limit, uncommenting below levels since they are future work
        //17 => 17,
        //18 => 19,
        //19 => 20,
        //20 => 21, // planck time
        _ => panic!("Invalid time resolution level"),
    }
}

pub fn date_byte_length(date_range_level: u8) -> usize {
    match date_range_level {
        0 => 2,
        1 => 3,
        2 => 4,
        3 => 6,
        _ => panic!("Invalid date range level specified"),
    }
}

pub fn zone_bit_length(zone_level: u8) -> usize {
    match zone_level {
        // must account for sign bit per old design, so cannot mirror time level bit scheme
        0 => 0,
        1 => 7, // 15 min offset, 31:30 hour coverage enough for modern day uses. 7 bits including sign
        2 => 12, // 11 bits + sign
        3 => 28,
        4 => 48,
        5 => 68,
        6 => 88,
        7 => 108,
        8 => 128,
        // uncommenting below, anything above 16 bytes not supported
        // 9 => 147,
        // 10 => 162,
        _ => panic!("Invalid time zone level"),
    }
}

pub fn zone_byte_length(zone_level: u8) -> usize {
    zone_bit_length(zone_level).div_ceil(8)
}

pub fn leap_counter_byte_length(leap_counter_length: u8) -> usize {
    leap_counter_length as usize
}

pub fn encode_moment_body(moment: &Moment) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();

    let time_length = time_byte_length(moment.header.time_resolution_level);
    let date_length = date_byte_length(moment.header.date_range_level);
    let zone_length = zone_byte_length(moment.header.zone_level);
    let leap_length = leap_counter_byte_length(moment.header.leap_counter_length);

    write_u128_big_endian(moment.date_value, date_length, &mut out)?;

    if time_length > 0 {
        let time = moment.time_value.unwrap_or(0);
        write_u128_big_endian(time, time_length, &mut out)?;
    }

    if zone_length > 0 {
        let zone = moment.zone_value.unwrap_or(0);
        write_u128_big_endian(zone, zone_length, &mut out)?;
    }

    if leap_length > 0 {
        let positive = moment.positive_leap_seconds.unwrap_or(0);
        let negative = moment.negative_leap_seconds.unwrap_or(0);

        write_u128_big_endian(positive, leap_length, &mut out)?;
        write_u128_big_endian(negative, leap_length, &mut out)?;
    }

    if moment.header.has_uncertainty {
        let uncertainty = moment.uncertainty_offset.unwrap_or(0);

        let uncertainty_length = if time_length > 0 {
            time_length
        } else {
            date_length
        };
        // intentional design decision, because a Moment can still just be on day level so following
        // the semantic rule on uncertainty handling, the offset would be for the same resolution

        write_u128_big_endian(uncertainty, uncertainty_length, &mut out)?;
    }

    if moment.header.lsl_status != 0 {
        let jdn = moment.lsl_jdn.unwrap_or(0);

        match moment.header.lsl_status {
            1 => write_u128_big_endian(jdn as u128, 3, &mut out)?, // Range level 2, 22 bit 11k year JDN coverage from -4713-11-24 BC
            2 => write_u128_big_endian(jdn as u128, 4, &mut out)?, // Range lvl 3, 32 bits
            3 => write_u128_big_endian(jdn as u128, 6, &mut out)?, // Range lvl 4, 48 bits
            _ => return Err("Unsupported leap second list code".to_string()),
        }
    }

    Ok(out)
}

pub fn decode_moment_body(header: &MomentHeader, bytes: &[u8]) -> Result<(Moment, usize), String> {
    let mut pos = 0usize;

    let date_length = date_byte_length(header.date_range_level);
    let time_length = time_byte_length(header.time_resolution_level);
    let zone_length = zone_byte_length(header.zone_level);
    let leap_length = leap_counter_byte_length(header.leap_counter_length);

    if bytes.len() < pos + date_length {
        return Err("Insufficient number of bytes for date".to_string());
    }

    let date_value = read_u128_big_endian(&bytes[pos..pos + date_length]);
    pos += date_length;

    let time_value = if time_length > 0 {
        if bytes.len() < pos + time_length {
            return Err("Insufficient bytes for itme".to_string());
        }
        let value = read_u128_big_endian(&bytes[pos..pos + time_length]);
        pos += time_length;
        Some(value)
    } else {
        None
    };

    let zone_value = if zone_length > 0 {
        if bytes.len() < pos + zone_length {
            return Err("Insufficient bytes for zone level".to_string());
        }
        let value = read_u128_big_endian(&bytes[pos..pos + zone_length]);
        pos += zone_length;
        Some(value)
    } else {
        None
    };

    let (positive_leap_seconds, negative_leap_seconds) = if leap_length > 0 {
        if bytes.len() < pos + leap_length * 2 {
            return Err("Insufficient bytes for leap second counters ".to_string());
        }

        let positive = read_u128_big_endian(&bytes[pos..pos + leap_length]);
        pos += leap_length;

        let negative = read_u128_big_endian(&bytes[pos..pos + leap_length]);
        pos += leap_length;

        (Some(positive), Some(negative))
    } else {
        (None, None)
    };

    let uncertainty_offset = if header.has_uncertainty {
        let uncertainty_length = if time_length > 0 {
            time_length
        } else {
            date_length
        };

        if bytes.len() < pos + uncertainty_length {
            return Err("Insufficinet bytes for uncertainty offset".to_string());
        }

        let value = read_u128_big_endian(&bytes[pos..pos + uncertainty_length]);
        pos += uncertainty_length;

        Some(value)
    } else {
        None
    };

    let lsl_jdn = if header.lsl_status != 0 {
        let length = match header.lsl_status {
            1 => 3,
            2 => 4,
            3 => 6,
            _ => return Err("Unsupported Lsl state".to_string()),
        };

        if bytes.len() < pos + length {
            return Err("Not enough bytes for LSL's JDN payload".to_string());
        }

        let value = read_u128_big_endian(&bytes[pos..pos + length]);
        pos += length;

        Some(value as u64)
    } else {
        None
    };

    Ok((
        Moment {
            header: header.clone(),
            time_value,
            date_value,
            zone_value,
            positive_leap_seconds,
            negative_leap_seconds,
            uncertainty_offset,
            lsl_jdn,
        },
        pos,
    ))
}

pub fn encode_period_body(period: &Period) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();

    let date_length = date_byte_length(period.header.date_range_level);
    let time_length = time_byte_length(period.header.time_resolution_level);
    let leap_length = leap_counter_byte_length(period.header.leap_counter_length);

    write_u128_big_endian(period.date_duration, date_length, &mut out)?;

    if time_length > 0 {
        let time = period.time_duration.unwrap_or(0);
        write_u128_big_endian(time, time_length, &mut out)?;
    }

    if leap_length > 0 {
        let positive = period.positive_leap_seconds.unwrap_or(0);
        let negative = period.negative_leap_seconds.unwrap_or(0);

        write_u128_big_endian(positive, leap_length, &mut out)?;
        write_u128_big_endian(negative, leap_length, &mut out)?;
    }

    if period.header.has_uncertainty {
        let uncertainty = period.uncertainty_offset.unwrap_or(0);
        let uncertainty_length = if time_length > 0 {
            time_length
        } else {
            date_length
        };
        write_u128_big_endian(uncertainty, uncertainty_length, &mut out)?;
    }

    if period.header.lsl_status != 0 {
        let jdn = period.lsl_jdn.unwrap_or(0);

        match period.header.lsl_status {
            1 => write_u128_big_endian(jdn as u128, 3, &mut out)?,
            2 => write_u128_big_endian(jdn as u128, 4, &mut out)?,
            3 => write_u128_big_endian(jdn as u128, 6, &mut out)?,
            _ => return Err("Unsupported Lsl state".to_string()),
        }
    }

    Ok(out)
}

pub fn decode_period_body(header: &PeriodHeader, bytes: &[u8]) -> Result<(Period, usize), String> {
    let mut pos = 0usize;

    let date_length = date_byte_length(header.date_range_level);
    let time_length = time_byte_length(header.time_resolution_level);
    let leap_length = leap_counter_byte_length(header.leap_counter_length);

    if bytes.len() < pos + date_length {
        return Err("Insufficient number of bytes for date".to_string());
    }

    let date_duration = read_u128_big_endian(&bytes[pos..pos + date_length]);
    pos += date_length;

    let time_duration = if time_length > 0 {
        if bytes.len() < pos + time_length {
            return Err("Insufficient bytes for itme".to_string());
        }
        let value = read_u128_big_endian(&bytes[pos..pos + time_length]);
        pos += time_length;
        Some(value)
    } else {
        None
    };

    let (positive_leap_seconds, negative_leap_seconds) = if leap_length > 0 {
        if bytes.len() < pos + leap_length * 2 {
            return Err("Insufficient bytes for leap second counters ".to_string());
        }

        let positive = read_u128_big_endian(&bytes[pos..pos + leap_length]);
        pos += leap_length;

        let negative = read_u128_big_endian(&bytes[pos..pos + leap_length]);
        pos += leap_length;

        (Some(positive), Some(negative))
    } else {
        (None, None)
    };

    let uncertainty_offset = if header.has_uncertainty {
        let uncertainty_length = if time_length > 0 {
            time_length
        } else {
            date_length
        };

        if bytes.len() < pos + uncertainty_length {
            return Err("Insufficinet bytes for uncertainty offset".to_string());
        }

        let value = read_u128_big_endian(&bytes[pos..pos + uncertainty_length]);
        pos += uncertainty_length;

        Some(value)
    } else {
        None
    };

    let lsl_jdn = if header.lsl_status != 0 {
        let length = match header.lsl_status {
            1 => 3,
            2 => 4,
            3 => 6,
            _ => return Err("Unsupported Lsl state".to_string()),
        };

        if bytes.len() < pos + length {
            return Err("Not enough bytes for LSL's JDN payload".to_string());
        }

        let value = read_u128_big_endian(&bytes[pos..pos + length]);
        pos += length;

        Some(value as u64)
    } else {
        None
    };

    Ok((
        Period {
            header: header.clone(),
            time_duration,
            date_duration,
            positive_leap_seconds,
            negative_leap_seconds,
            uncertainty_offset,
            lsl_jdn,
        },
        pos,
    ))
}
