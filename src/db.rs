use postgres::{Client, NoTls};
use std::env;
use std::fmt::format;

use crate::body::{
    self, decode_moment_body, decode_period_body, encode_moment_body, encode_period_body,
};
use crate::model::{Accuracy, Moment, Period, Sign};
use crate::moment_header::{decode_moment_header, encode_moment_header};
use crate::notation::{
    format_hours_minutes, format_zone_offset, jdn_to_gregorian, moment_to_notation,
    period_to_notation,
};
use crate::period_header::{decode_period_header, encode_period_header};

pub fn connect() -> Result<Client, String> {
    // checking env variable too if user wants to set their own
    let connection_string = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "host=localhost user=postgres password=1234 dbname=datetime-thesis".to_string()
    });

    Client::connect(&connection_string, NoTls)
        .map_err(|err| format!("PostgreSQL DB connection failed: {}", err))
}

pub fn ensure_schema(client: &mut Client) -> Result<(), String> {
    client
        .batch_execute(
            "
  CREATE TABLE IF NOT EXISTS time_values (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('moment', 'period')),
    encoded BYTEA NOT NULL,
    notation TEXT NOT NULL,
    
    native_date DATE NULL,
    native_timestamp TIMESTAMPTZ NULL,
    native_interval INTERVAL NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
",
        )
        .map_err(|err| format!("Failed to create table per schema: {}", err))?;

    Ok(())
}

pub fn insert_moment(client: &mut Client, moment: &Moment) -> Result<i64, String> {
    let encoded = encode_full_moment(moment)?;
    let notation = moment_to_notation(moment)?;
    let conversion = moment_db_conversion(moment)?;

    let row = client
        .query_one(
            "
  INSERT INTO time_values (
    kind,
    encoded,
    notation,
    native_date,
    native_timestamp,
    native_interval
)
    VALUES (
    $1,
    $2,
    $3,
    $4::text::date,
    $5::text::timestamptz,
    $6::text::interval
)
    RETURNING id
  ",
            &[
                &"moment",
                &encoded,
                &notation,
                &conversion.native_date,
                &conversion.native_timestamp,
                &conversion.native_interval,
            ],
        )
        .map_err(|err| format!("Failed to insert moment: {}", err))?;

    Ok(row.get(0))
}

pub fn insert_period(client: &mut Client, period: &Period) -> Result<i64, String> {
    let encoded = encode_full_period(period)?;
    let notation = period_to_notation(period)?;
    let conversion = period_db_conversion(period)?;

    let row = client
        .query_one(
            "
  INSERT INTO time_values (
    kind,
    encoded,
    notation,
    native_date,
    native_timestamp,
    native_interval
)
    VALUES (
    $1,
    $2,
    $3,
    $4::text::date,
    $5::text::timestamptz,
    $6::text::interval
)
    RETURNING id
  ",
            &[
                &"period",
                &encoded,
                &notation,
                &conversion.native_date,
                &conversion.native_timestamp,
                &conversion.native_interval,
            ],
        )
        .map_err(|err| format!("Failed to insert peroid: {}", err))?;

    Ok(row.get(0))
}

pub fn load_moment(client: &mut Client, id: i64) -> Result<Moment, String> {
    let row = client
        .query_one(
            "
    SELECT kind, encoded
    FROM time_values
    WHERE id = $1
    ",
            &[&id],
        )
        .map_err(|err| format!("failed to load moment id {}: {}", id, err))?;

    let kind: String = row.get(0);
    if kind != "moment" {
        return Err(format!("Expected moment, found {}", kind));
    }

    let encoded: Vec<u8> = row.get(1);

    decode_full_moment(&encoded)
}

pub fn load_period(client: &mut Client, id: i64) -> Result<Period, String> {
    let row = client
        .query_one(
            "
    SELECT kind, encoded
    FROM time_values
    WHERE id = $1
    ",
            &[&id],
        )
        .map_err(|err| format!("failed to load period id {}: {}", id, err))?;

    let kind: String = row.get(0);
    if kind != "period" {
        return Err(format!("Expected period, found {}", kind));
    }

    let encoded: Vec<u8> = row.get(1);

    decode_full_period(&encoded)
}

pub fn print_row_summary(client: &mut Client, id: i64) -> Result<(), String> {
    let row = client
        .query_one(
            "
  SELECT
    id,
    kind,
    notation,
    native_date::text,
    native_timestamp::text,
    native_interval::text
  FROM time_values
  WHERE id = $1
    ",
            &[&id],
        )
        .map_err(|err| format!("Failed to read row summary : {}", err))?;

    let id: i64 = row.get(0);
    let kind: String = row.get(1);
    let notation: String = row.get(2);
    let native_date: Option<String> = row.get(3);
    let native_timestamp: Option<String> = row.get(4);
    let native_interval: Option<String> = row.get(5);

    println!("\n------ Database row summary -------");
    println!("id: {id}");
    println!("kind: {kind}");
    println!("Notation: {notation}");
    println!("Native date: {:?}", native_date);
    println!("Native timestamp: {:?}", native_timestamp);
    println!("Native interval: {:?}", native_interval);

    Ok(())
}

fn encode_full_moment(moment: &Moment) -> Result<Vec<u8>, String> {
    let mut bytes = encode_moment_header(&moment.header)
        .map_err(|err| format!("Failed to encode moment header: {:?}", err))?;
    bytes.extend(encode_moment_body(moment)?);

    Ok(bytes)
}

fn decode_full_moment(bytes: &[u8]) -> Result<Moment, String> {
    let (header, header_length) = decode_moment_header(bytes)
        .map_err(|err| format!("Failed to decode moment header; {:?}", err))?;
    let (moment, body_length) = decode_moment_body(&header, &bytes[header_length..])?;

    if header_length + body_length != bytes.len() {
        return Err("Moment decoding has not c onsumed all stored bytes".to_string());
    }

    Ok(moment)
}

fn encode_full_period(period: &Period) -> Result<Vec<u8>, String> {
    let mut bytes = encode_period_header(&period.header)
        .map_err(|err| format!("Failed to encode period header: {:?}", err))?;
    bytes.extend(encode_period_body(period)?);

    Ok(bytes)
}

fn decode_full_period(bytes: &[u8]) -> Result<Period, String> {
    let (header, header_length) = decode_period_header(bytes)
        .map_err(|err| format!("FAiled to decode period header: {:?}", err))?;
    let (period, body_length) = decode_period_body(&header, &bytes[header_length..])?;

    if header_length + body_length != bytes.len() {
        return Err("Period decoding didn't consume all bytes, something's wrong".to_string());
    }

    Ok(period)
}

struct DatabaseConversion {
    native_date: Option<String>,
    native_timestamp: Option<String>,
    native_interval: Option<String>,
}

fn moment_db_conversion(moment: &Moment) -> Result<DatabaseConversion, String> {
    let (year, month, day) = jdn_to_gregorian(moment.date_value)?;
    let date = format!("{:04}-{:02}-{:02}", year, month, day);

    match moment.header.time_resolution_level {
        0 => Ok(DatabaseConversion {
            native_date: Some(date),
            native_timestamp: None,
            native_interval: None,
        }),
        4 => {
            let time = moment
                .time_value
                .ok_or("Moment has minute resolution but no time")?;

            if time >= 24 * 60 {
                return Err("Moment minute value must be below 1440".to_string());
            }

            let time_string = format_hours_minutes(time)?;

            let zone_string = match moment.header.zone_level {
                0 => "+00:00".to_string(),

                1 => {
                    let zone = moment
                        .zone_value
                        .ok_or("Moment has zone level assigned but no zone offset")?;

                    format_zone_offset(zone, moment.header.zone_level)?
                }

                _ => {
                    return Ok(DatabaseConversion {
                        native_date: None,
                        native_timestamp: None,
                        native_interval: None,
                    });
                }
            };

            Ok(DatabaseConversion {
                native_date: None,
                native_timestamp: Some(format!("{} {}:00{}", date, time_string, zone_string)),
                native_interval: None,
            })
        }

        _ => Ok(DatabaseConversion {
            native_date: None,
            native_timestamp: None,
            native_interval: None,
        }),
    }
}

fn period_db_conversion(period: &Period) -> Result<DatabaseConversion, String> {
    let date_mins = period
        .date_duration
        .checked_mul(24 * 60)
        .ok_or("period date duration overflow")?;

    let total_mins = match period.header.time_resolution_level {
        0 => date_mins,
        4 => {
            let time = period
                .time_duration
                .ok_or("period has minute resolution but no time")?;

            date_mins
                .checked_add(time)
                .ok_or("Period time duration overflow")?
        }

        _ => {
            return Ok(DatabaseConversion {
                native_date: None,
                native_timestamp: None,
                native_interval: None,
            });
        }
    };

    let mut signed_mins = u128_to_i128(total_mins, "Period total mins")?;

    if period.header.sign == Sign::Negative {
        signed_mins = signed_mins
            .checked_neg()
            .ok_or("Period Postgresql interval oveflow")?;
    }
    Ok(DatabaseConversion {
        native_date: None,
        native_timestamp: None,
        native_interval: Some(format!("{} minutes", signed_mins)),
    })
}

fn u128_to_i128(value: u128, field_name: &str) -> Result<i128, String> {
    if value > i128::MAX as u128 {
        return Err(format!(
            "{} too large for Postgresql native ttypes",
            field_name
        ));
    }
    Ok(value as i128)
}
