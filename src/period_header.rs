use crate::errors::{DecodeError, EncodeError};
use crate::model::{PeriodHeader, Sign};

const I_BIT: u8 = 0b1000_0000;
const KIND_MOMENT_BIT: u8 = 0b0100_0000; // Keeping this for the mask byte to check Period K bit
const CATEGORY_BIT: u8 = 0b0100_0000;

/*
For reference

Byte 1: I K S D T T T L
Byte 2: I C D T T L L R

No need to change old header format.
Where S is the sign bit, S=0 means positive, S=1 means negative

Extension header, same format as for Moment:
        I C U V V V R R

Where V is the LSL bit
*/

pub fn encode_period_header(header: &PeriodHeader) -> Result<Vec<u8>, EncodeError> {
    validate_header(header)?;

    let has_extension = header.has_uncertainty || header.lsl_status != 0;

    let needs_byte_two = header.time_resolution_level > 7
        || header.date_range_level > 1
        || header.leap_counter_length > 1;

    let mut bytes = Vec::new();

    let byte_one_is_final = !needs_byte_two && !has_extension;
    let byte_two_is_final = needs_byte_two && !has_extension;

    bytes.push(encode_byte_one(header, byte_one_is_final));

    if needs_byte_two {
        bytes.push(encode_byte_two(header, byte_two_is_final));
    }

    if has_extension {
        bytes.push(encode_extension(header, true));
    }

    Ok(bytes)
}

fn encode_byte_one(header: &PeriodHeader, is_final: bool) -> u8 {
    let mut byte = 0u8;

    if is_final {
        byte |= I_BIT;
    }

    let sign_bit = header.sign.to_binary();
    let d_0 = header.date_range_level & 0b1;
    let t_0_1_2 = header.time_resolution_level & 0b111;
    let l_0 = header.leap_counter_length & 0b1;

    byte |= sign_bit << 5;
    byte |= d_0 << 4;
    byte |= t_0_1_2 << 1;
    byte |= l_0;

    byte
}

fn encode_byte_two(header: &PeriodHeader, is_final: bool) -> u8 {
    let mut byte = 0u8;

    if is_final {
        byte |= I_BIT;
    }

    let d_1 = (header.date_range_level >> 1) & 0b1;
    let t_3 = (header.time_resolution_level >> 3) & 0b1;
    let t_4 = (header.time_resolution_level >> 4) & 0b1;
    let l_1 = (header.leap_counter_length >> 1) & 0b1;
    let l_2 = (header.leap_counter_length >> 2) & 0b1;

    byte |= d_1 << 5;
    byte |= t_3 << 4;
    byte |= t_4 << 3;
    byte |= l_1 << 2;
    byte |= l_2 << 1;

    byte
}

fn encode_extension(header: &PeriodHeader, is_final: bool) -> u8 {
    let mut byte = 0u8;

    if is_final {
        byte |= I_BIT;
    }

    // C gets set now since extension header
    byte |= CATEGORY_BIT;

    if header.has_uncertainty {
        byte |= 1 << 5;
    }

    byte |= (header.lsl_status & 0b111) << 2;

    byte
}

fn validate_header(header: &PeriodHeader) -> Result<(), EncodeError> {
    if header.time_resolution_level > 20 {
        return Err(EncodeError::TimeResolutionTooLarge);
    }

    if header.date_range_level > 3 {
        return Err(EncodeError::DateRangeLevelTooLarge);
    }

    if header.leap_counter_length > 7 {
        return Err(EncodeError::LeapCounterLengthTooLarge);
    }

    if header.lsl_status > 3 {
        return Err(EncodeError::InvalidLslStatus);
    }
    Ok(())
}

pub fn decode_period_header(bytes: &[u8]) -> Result<(PeriodHeader, usize), DecodeError> {
    if bytes.is_empty() {
        return Err(DecodeError::EmptyInput);
    }

    let byte_one = bytes[0];

    let is_period = (byte_one & KIND_MOMENT_BIT) == 0;

    if !is_period {
        return Err(DecodeError::NotPeriodHeader);
    }

    let byte_one_is_final = (byte_one & I_BIT) != 0;

    let sign_bit = (byte_one >> 5) & 0b1;
    let sign = Sign::from_binary(sign_bit).ok_or(DecodeError::InvalidSignBit)?;

    let d_0 = (byte_one >> 4) & 0b1;
    let t_0_1_2 = (byte_one >> 1) & 0b111;
    let l_0 = byte_one & 0b1;

    let mut header = PeriodHeader {
        sign,
        date_range_level: d_0,
        time_resolution_level: t_0_1_2,
        leap_counter_length: l_0,
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

    let reserved = byte_two & 0b1;
    if reserved != 0 {
        return Err(DecodeError::NonZeroReservedBits);
    }

    let d_1 = (byte_two >> 5) & 0b1;
    let t_3 = (byte_two >> 4) & 0b1;
    let t_4 = (byte_two >> 3) & 0b1;
    let l_1 = (byte_two >> 2) & 0b1;
    let l_2 = (byte_two >> 1) & 0b1;

    let d = d_0 | (d_1 << 1);
    let t = t_0_1_2 | (t_3 << 3) | (t_4 << 4);
    let l = l_0 | (l_1 << 1) | (l_2 << 2);

    header = PeriodHeader {
        sign,
        time_resolution_level: t,
        date_range_level: d,
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

    let extension = bytes[2];

    let is_extension = (extension & CATEGORY_BIT) != 0;
    if !is_extension {
        return Err(DecodeError::CoreHeaderAfterExtension);
    }

    decode_extension(extension, &mut header)?;

    let extension_is_final = (extension & I_BIT) != 0;
    if !extension_is_final {
        return Err(DecodeError::UnsupportedAdditionalExtensionHeaders);
    }

    Ok((header, 3))
}

fn decode_extension(byte: u8, header: &mut PeriodHeader) -> Result<(), DecodeError> {
    let cat_bit_is_extension = (byte & CATEGORY_BIT) != 0;
    if !cat_bit_is_extension {
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
