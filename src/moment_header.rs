use crate::errors::{DecodeError, EncodeError};
use crate::model::{Accuracy, MomentHeader};

const I_BIT: u8 = 0b1000_0000;
const KIND_MOMENT_BIT: u8 = 0b0100_0000;
const CATEGORY_BIT: u8 = 0b0100_0000;

/*
For reference

Byte 1: I K D T T T Z A
Byte 2: I C A T Z D L L
Byte 3: I C T Z Z L R R

Extension header:
        I C U V V V R R

Where V is the LSL bit
*/

pub fn encode_moment_header(header: &MomentHeader) -> Result<Vec<u8>, EncodeError> {
    validate_header(header)?;

    let accuracy_bits = header.accuracy.to_binary();

    let has_extension_header = header.has_uncertainty || header.lsl_status != 0;

    let needs_second_byte = header.time_resolution_level > 7
        || header.date_range_level > 1
        || header.zone_level > 1
        || accuracy_bits > 1
        || header.leap_counter_length > 0;

    let needs_third_byte = header.time_resolution_level > 15
        || header.zone_level > 3
        || header.leap_counter_length > 3;

    let mut bytes = Vec::new();

    let one_byte_final = !needs_second_byte && !has_extension_header;
    let two_bytes_final = needs_second_byte && !needs_third_byte && !has_extension_header;
    let three_bytes_final = needs_third_byte && !has_extension_header;

    bytes.push(encode_first_byte(header, one_byte_final));

    if needs_second_byte {
        bytes.push(encode_second_byte(header, two_bytes_final));
    }

    if needs_third_byte {
        bytes.push(encode_third_byte(header, three_bytes_final));
    }

    if has_extension_header {
        bytes.push(encode_extension_header(header, true));
    }

    Ok(bytes)
}

fn validate_header(header: &MomentHeader) -> Result<(), EncodeError> {
    if header.time_resolution_level > 20 {
        return Err(EncodeError::TimeResolutionTooLarge);
    }

    if header.date_range_level > 3 {
        return Err(EncodeError::DateRangeLevelTooLarge);
    }

    if header.zone_level > 10 {
        return Err(EncodeError::ZoneLevelTooLarge);
    }

    // Maybe make distinction that bit length check is not same
    // as saying accuracy can be 0b11 ? Since only defined up to 0b10.
    if header.accuracy.to_binary() > 3 {
        return Err(EncodeError::AccuracyTooLarge);
    }

    if header.leap_counter_length > 7 {
        return Err(EncodeError::LeapCounterLengthTooLarge);
    }

    // Adding this to stop reserved LSL bits from being used by accident
    if header.lsl_status > 3 {
        return Err(EncodeError::InvalidLslStatus);
    }
    Ok(())
}

fn encode_first_byte(header: &MomentHeader, is_final: bool) -> u8 {
    let accuracy_bits = header.accuracy.to_binary();

    let d_0 = header.date_range_level & 0b1;
    let t_0_1_2 = header.time_resolution_level & 0b111;
    let z_0 = header.zone_level & 0b1;
    let a_0 = accuracy_bits & 0b1;

    let mut byte = 0u8;

    if is_final {
        byte |= I_BIT;
    }

    // Uses masks to set the kind and indicator bits
    // Then shifts necessary bits to their right position in the mask before bitwise OR
    // with the expected byte result
    byte |= KIND_MOMENT_BIT;
    byte |= d_0 << 5;
    byte |= t_0_1_2 << 2;
    byte |= z_0 << 1;
    byte |= a_0;

    byte
}

fn encode_second_byte(header: &MomentHeader, is_final: bool) -> u8 {
    let accuracy_bits = header.accuracy.to_binary();

    // Bitwise shift is done to discard bits already encoded in the 1st byte
    let a_1 = (accuracy_bits >> 1) & 0b1;
    let t_3 = (header.time_resolution_level >> 3) & 0b1;
    let z_1 = (header.zone_level >> 1) & 0b1;
    let d_1 = (header.date_range_level >> 1) & 0b1;
    let l_0_1 = header.leap_counter_length & 0b11;

    let mut byte = 0u8;

    if is_final {
        byte |= I_BIT;
    }

    // Category bit already set to 0 due to indicator bit mask op above

    byte |= a_1 << 5;
    byte |= t_3 << 4;
    byte |= z_1 << 3;
    byte |= d_1 << 2;
    byte |= l_0_1;

    byte
}

fn encode_third_byte(header: &MomentHeader, is_final: bool) -> u8 {
    let t_4 = (header.time_resolution_level >> 4) & 0b1; // fifth time header bit shifts right by 4 places
    let z_2 = (header.zone_level >> 2) & 0b1;
    let z_3 = (header.zone_level >> 3) & 0b1;
    let l_2 = (header.leap_counter_length >> 2) & 0b1;

    let mut byte = 0u8;

    if is_final {
        byte |= I_BIT;
    }

    byte |= t_4 << 5;
    byte |= z_3 << 4;
    byte |= z_2 << 3;
    byte |= l_2 << 2;

    byte
}

fn encode_extension_header(header: &MomentHeader, is_final: bool) -> u8 {
    let mut byte = 0u8;

    if is_final {
        byte |= I_BIT;
    }

    byte |= CATEGORY_BIT;

    if header.has_uncertainty {
        byte |= 1 << 5;
    }

    // binary AND and left shift
    byte |= (header.lsl_status & 0b111) << 2;

    byte
}

// Decoding Moment header

pub fn decode_moment_header(bytes: &[u8]) -> Result<(MomentHeader, usize), DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::EmptyInput);
    }

    let byte_one = bytes[0];

    let is_moment = (byte_one & KIND_MOMENT_BIT) != 0;
    if !is_moment {
        return Err(DecodeError::NotMomentHeader);
    }

    let byte_one_is_final = (byte_one & I_BIT) != 0;

    let d_0 = (byte_one >> 5) & 0b1;
    let t_0_1_2 = (byte_one >> 2) & 0b111;
    let z_0 = (byte_one >> 1) & 0b1;
    let a_0 = byte_one & 0b1;

    let mut d = d_0;
    let mut t = t_0_1_2;
    let mut z = z_0;
    let mut a = a_0;
    let mut l = 0u8;

    let accuracy = Accuracy::from_binary(a).ok_or(DecodeError::InvalidAccuracyBits)?;

    let mut header = MomentHeader {
        time_resolution_level: t,
        date_range_level: d,
        zone_level: z,
        accuracy,
        leap_counter_length: l,
        has_uncertainty: false,
        lsl_status: 0,
    };

    if byte_one_is_final {
        return Ok((header, 1));
    }

    if bytes.len() < 2 {
        return Err(DecodeError::UnexpectedEnd);
    }

    let byte_two_or_extension = bytes[1];
    let is_extension = (byte_two_or_extension & CATEGORY_BIT) != 0;

    if is_extension {
        decode_extension(byte_two_or_extension, &mut header)?;

        let is_final = (byte_two_or_extension & I_BIT) != 0;
        if !is_final {
            return Err(DecodeError::UnexpectedEnd);
        }

        return Ok((header, 2));
    }

    let byte_two = byte_two_or_extension;
    let byte_two_is_final = (byte_two & I_BIT) != 0;

    let a_1 = (byte_two >> 5) & 0b1;
    let t_3 = (byte_two >> 4) & 0b1;
    let z_1 = (byte_two >> 3) & 0b1;
    let d_1 = (byte_two >> 2) & 0b1;
    let l_0_1 = byte_two & 0b11;

    d = d_0 | (d_1 << 1);
    t = t_0_1_2 | (t_3 << 3);
    z = z_0 | (z_1 << 1);
    a = a_0 | (a_1 << 1);
    l = l_0_1;

    let accuracy = Accuracy::from_binary(a).ok_or(DecodeError::InvalidAccuracyBits)?;

    header = MomentHeader {
        time_resolution_level: t,
        date_range_level: d,
        zone_level: z,
        accuracy,
        leap_counter_length: l,
        has_uncertainty: false,
        lsl_status: 0,
    };

    if byte_two_is_final {
        return Ok((header, 2));
    }

    if bytes.len() < 3 {
        return Err(DecodeError::UnexpectedEnd);
    }

    let byte_three_or_extension = bytes[2];
    let is_extension = (byte_three_or_extension & CATEGORY_BIT) != 0;

    if is_extension {
        decode_extension(byte_three_or_extension, &mut header)?;

        let is_final = (byte_three_or_extension & I_BIT) != 0;
        if !is_final {
            return Err(DecodeError::UnexpectedEnd);
        }

        return Ok((header, 3));
    }

    let byte_three = byte_three_or_extension;
    let byte_three_is_final = (byte_three & I_BIT) != 0;

    let reserved = byte_three & 0b11;
    if reserved != 0 {
        return Err(DecodeError::NonZeroReservedBits);
    }

    let t_4 = (byte_three >> 5) & 0b1;
    let z_3 = (byte_three >> 4) & 0b1;
    let z_2 = (byte_three >> 3) & 0b1;
    let l_2 = (byte_three >> 2) & 0b1;

    t = t_0_1_2 | (t_3 << 3) | (t_4 << 4);
    d = d_0 | (d_1 << 1);
    z = z_0 | (z_1 << 1) | (z_2 << 2) | (z_3 << 3);
    a = a_0 | (a_1 << 1);
    l = l_0_1 | (l_2 << 2);

    let accuracy = Accuracy::from_binary(a).ok_or(DecodeError::InvalidAccuracyBits)?;

    header = MomentHeader {
        time_resolution_level: t,
        date_range_level: d,
        zone_level: z,
        accuracy,
        leap_counter_length: l,
        has_uncertainty: false,
        lsl_status: 0,
    };

    if byte_three_is_final {
        return Ok((header, 3));
    }

    if bytes.len() < 4 {
        return Err(DecodeError::UnexpectedEnd);
    }

    let ext = bytes[3];

    let is_extension = (ext & CATEGORY_BIT) != 0;
    if !is_extension {
        return Err(DecodeError::CoreHeaderAfterExtension);
    }

    decode_extension(ext, &mut header)?;

    let ext_is_final = (ext & I_BIT) != 0;
    if !ext_is_final {
        return Err(DecodeError::UnsupportedAdditionalExtensionHeaders);
        // This error is to stop further extension bytes from being added, but this is
        // essentially where future work in extensibility can be applied
    }

    Ok((header, 4))
}

fn decode_extension(byte: u8, header: &mut MomentHeader) -> Result<(), DecodeError> {
    let category_is_ext = (byte & CATEGORY_BIT) != 0;
    if !category_is_ext {
        return Err(DecodeError::InvalidHeaderCategory);
    }

    let reserved = byte & 0b11;
    if reserved != 0 {
        return Err(DecodeError::NonZeroReservedBits);
    }

    let has_uncertainty = ((byte >> 5) & 0b1) != 0;
    let lsl_status = (byte >> 2) & 0b111;

    if lsl_status > 3 {
        return Err(DecodeError::InvalidLslStatus);
    }

    header.has_uncertainty = has_uncertainty;
    header.lsl_status = lsl_status;

    Ok(())
}
