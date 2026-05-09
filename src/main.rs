// Will probably include console testing here
// also note to self, maybe allow user input using prescribed notation
use date_time::arithmetic::{
    moment_add_period, moment_subtract, moment_subtract_period, period_add, period_subtract,
};
use date_time::body::{
    decode_moment_body, decode_period_body, encode_moment_body, encode_period_body,
};
use date_time::db::{
    connect, ensure_schema, insert_moment, insert_period, load_moment, load_period,
    print_row_summary,
};
use date_time::logical_ops::{compare_moments, compare_periods};
use date_time::model::{Accuracy, Moment, MomentHeader, Period, PeriodHeader, Sign};
use date_time::moment_header::{decode_moment_header, encode_moment_header};
use date_time::notation::{
    moment_to_notation, parse_moment_notation, parse_period_notation, period_to_notation,
};
use date_time::period_header::{decode_period_header, encode_period_header};
use std::io::{self, Write, stdout};
use std::path::PrefixComponent;

fn main() {
    // for small manual testing as I go
    let moment_header = MomentHeader {
        time_resolution_level: 20,
        date_range_level: 3,
        zone_level: 10,
        accuracy: Accuracy::End,
        leap_counter_length: 7,
        has_uncertainty: true,
        lsl_status: 2,
    };

    print_encoded_moment_header("Moment header test", &moment_header);

    let period_header = PeriodHeader {
        sign: Sign::Negative,
        time_resolution_level: 20,
        date_range_level: 3,
        leap_counter_length: 7,
        has_uncertainty: true,
        lsl_status: 2,
    };

    print_encoded_period_header("Period header test", &period_header);

    // Body tests

    let moment_body_header = MomentHeader {
        time_resolution_level: 5, // seconds level
        date_range_level: 1,
        zone_level: 1,
        accuracy: Accuracy::Start,
        leap_counter_length: 1,
        has_uncertainty: true,
        lsl_status: 2,
    };

    let moment = Moment {
        header: moment_body_header,
        date_value: 0x0025_8C2C,
        time_value: Some(40_000),
        zone_value: Some(0b000_0001),
        positive_leap_seconds: Some(37),
        negative_leap_seconds: Some(0),
        uncertainty_offset: Some(10),
        lsl_jdn: Some(2_461_100),
    };

    print_encoded_moment_body("Moment body test", &moment);

    let period_body_header = PeriodHeader {
        sign: Sign::Negative,
        time_resolution_level: 5,
        date_range_level: 2,
        leap_counter_length: 1,
        has_uncertainty: true,
        lsl_status: 2,
    };

    let period = Period {
        header: period_body_header,
        date_duration: 50,
        time_duration: Some(20_000),
        positive_leap_seconds: Some(37),
        negative_leap_seconds: Some(0),
        uncertainty_offset: Some(10),
        lsl_jdn: Some(2_461_100),
    };

    print_encoded_period_body("Period body test ", &period);

    // Arithmetic testing. Limited scope on the operations

    let arithmetic_moment_header = MomentHeader {
        time_resolution_level: 5,
        date_range_level: 1,
        zone_level: 1,
        accuracy: Accuracy::Start,
        leap_counter_length: 0,
        has_uncertainty: false,
        lsl_status: 0,
    };

    let moment_a = Moment {
        header: arithmetic_moment_header,
        date_value: 2_461_000,
        time_value: Some(40_000),
        zone_value: Some(0),
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    };

    let moment_b = Moment {
        header: arithmetic_moment_header,
        date_value: 2_461_000,
        time_value: Some(50_000),
        zone_value: Some(0),
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    };

    let arithmetic_period_header = PeriodHeader {
        sign: Sign::Positive,
        time_resolution_level: 5,
        date_range_level: 1,
        leap_counter_length: 0,
        has_uncertainty: false,
        lsl_status: 0,
    };

    let period_a = Period {
        header: arithmetic_period_header,
        date_duration: 2,
        time_duration: Some(1_000),
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    };

    let period_b = Period {
        header: arithmetic_period_header,
        date_duration: 2,
        time_duration: Some(500),
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    };

    print_period_arithmetic_result("Period plus Period", period_add(&period_a, &period_b));
    print_period_arithmetic_result("Period minus Period", period_subtract(&period_a, &period_b));
    print_period_arithmetic_result("Moment minus Moment", moment_subtract(&moment_b, &moment_a));

    print_moment_arithmetic_result(
        "Moment plus Period",
        moment_add_period(&moment_a, &period_a),
    );
    print_moment_arithmetic_result(
        "Moment minus Period",
        moment_subtract_period(&moment_b, &period_b),
    );

    // Allen relation testing for logical operations. Limited

    print_moment_logical_operation(
        "Moment A compared to Moment B",
        compare_moments(&moment_a, &moment_b),
    );
    print_moment_logical_operation(
        "Moment B compared to Moment A",
        compare_moments(&moment_b, &moment_a),
    );

    print_period_logical_operation(
        "Period A compared to Period B",
        compare_periods(&period_a, &period_b),
    );
    print_period_logical_operation(
        "Period B compared to Period A",
        compare_periods(&period_b, &period_a),
    );

    postgresql_storage_test();

    notation_user_input();
}

fn notation_user_input() {
    println!("\n----- Notation user input test ----");
    println!("Use M for Moment or P for Period");
    print!("Moment example: @G2026-01-01T12:00+01:00 {{d:1 t:4 z:1 a:s l:0-0 u:10 lsv:2461100}}@");
    println!("Period example: @P2DT12:00 {{d:1 t:4 l:0-0 u:10 lsv:2461100}}@");

    let kind = read_input("Type M or P: ");
    let notation = read_input("Notation: ");

    match kind.trim().to_lowercase().as_str() {
        "m" | "moment" => match parse_moment_notation(&notation) {
            Ok(moment) => {
                println!("\nParsed Moment : ");
                println!("{moment:#?}");

                match moment_to_notation(&moment) {
                    Ok(rendered) => println!("\nConverted back to notation: \n{rendered}"),
                    Err(err) => println!("Failed to render Moment notation - {err}"),
                }
            }

            Err(err) => println!("Failed to parse Moment notation - {err}"),
        },

        "p" | "period" => match parse_period_notation(&notation) {
            Ok(period) => {
                println!("\nParsed Period: ");
                print!("{period:#?}");
                match period_to_notation(&period) {
                    Ok(rendered) => println!("\n Converted back to Period notation:\n{rendered}"),
                    Err(err) => print!("Failed to render period notation - {err}"),
                }
            }
            Err(err) => println!("Failed to parse Period notation - {err}"),
        },
        _ => println!("Unknown type"),
    }
}

fn read_input(prompt: &str) -> String {
    print!("{prompt}");
    io::stdout().flush().expect("Failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed eading input");

    input.trim().to_string()
}

fn print_moment_logical_operation(
    label: &str,
    result: Result<date_time::logical_ops::AllenRelation, String>,
) {
    println!("\n---- {label} ----");

    match result {
        Ok(relation) => {
            println!("Moment relation: {relation:#?}");
        }
        Err(err) => {
            print!("Logical comparison has failed: {err}");
        }
    }
}

fn print_period_logical_operation(
    label: &str,
    result: Result<date_time::logical_ops::AllenRelation, String>,
) {
    println!("\n---- {label} ----");

    match result {
        Ok(relation) => {
            println!("Period relation: {relation:#?}");
        }
        Err(err) => {
            print!("Logical comparison has failed: {err}");
        }
    }
}

fn print_encoded_moment_header(label: &str, header: &MomentHeader) {
    println!("\n----- {label} -----");
    println!("Input: {header:#?}"); // pretty print the header so it's not just one line

    let bytes = match encode_moment_header(header) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("Failed to encode header: {:?}", err);
            return;
        }
    };

    println!("Encoded bytes: ");
    for (i, byte) in bytes.iter().enumerate() {
        println!("Header byte {}: {:08b} 0x{:02X}", i + 1, byte, byte);
    }

    // checks whether encode-decode works fully
    match decode_moment_header(&bytes) {
        Ok((decoded, consumed)) => {
            println!("\nDecoded header: ");
            print!("{decoded:#?}\n");
            print!("\nNumber of consumed header bytes: {consumed}\n");
        }
        Err(err) => {
            println!("Failed to decode header: {:?}", err);
        }
    }
}

fn print_encoded_period_header(label: &str, header: &PeriodHeader) {
    println!("\n----- {label} -----");
    println!("Input: {header:#?}"); // pretty print the header so it's not just one line

    let bytes = match encode_period_header(header) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("Failed to encode header: {:?}", err);
            return;
        }
    };

    println!("Encoded bytes: ");
    for (i, byte) in bytes.iter().enumerate() {
        println!("Header byte {}: {:08b} 0x{:02X}", i + 1, byte, byte);
    }

    // checks whether encode-decode works fully
    match decode_period_header(&bytes) {
        Ok((decoded, consumed)) => {
            println!("\nDecoded header: ");
            print!("{decoded:#?}\n");
            print!("\nNumber of consumed header bytes: {consumed}\n");
        }
        Err(err) => {
            println!("Failed to decode header: {:?}", err);
        }
    }
}

fn print_encoded_moment_body(label: &str, moment: &Moment) {
    println!("\n----- {label} -----");
    println!("Input moment: {moment:#?}"); // pretty print the header so it's not just one line

    let bytes = match encode_moment_body(moment) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("Failed to encode moment body: {:?}", err);
            return;
        }
    };

    println!("Encoded body bytes: ");
    for (i, byte) in bytes.iter().enumerate() {
        println!("Body byte {}: {:08b} 0x{:02X}", i + 1, byte, byte);
    }

    // checks whether encode-decode works fully
    match decode_moment_body(&moment.header, &bytes) {
        Ok((decoded, consumed)) => {
            println!("\nDecoded Moment: ");
            print!("{decoded:#?}\n");
            print!("\nNumber of consumed body bytes: {consumed}\n");
        }
        Err(err) => {
            println!("Failed to decode moment body: {:?}", err);
        }
    }
}

fn print_encoded_period_body(label: &str, period: &Period) {
    println!("\n----- {label} -----");
    println!("Input period: {period:#?}"); // pretty print the header so it's not just one line

    let bytes = match encode_period_body(period) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("Failed to encode peroid body: {:?}", err);
            return;
        }
    };

    println!("Encoded body bytes: ");
    for (i, byte) in bytes.iter().enumerate() {
        println!("Body byte {}: {:08b} 0x{:02X}", i + 1, byte, byte);
    }

    // checks whether encode-decode works fully
    match decode_period_body(&period.header, &bytes) {
        Ok((decoded, consumed)) => {
            println!("\nDecoded period: ");
            print!("{decoded:#?}\n");
            print!("\nNumber of consumed body bytes: {consumed}\n");
        }
        Err(err) => {
            println!("Failed to decode period body: {:?}", err);
        }
    }
}

fn print_period_arithmetic_result(label: &str, result: Result<Period, String>) {
    println!("\n---- {label} ----");

    match result {
        Ok(period) => {
            println!("Result Period: ");
            println!("{period:#?}");
        }
        Err(err) => {
            print!("Arithmetic operation failed: {err}");
        }
    }
}

fn print_moment_arithmetic_result(label: &str, result: Result<Moment, String>) {
    println!("\n---- {label} ----");

    match result {
        Ok(moment) => {
            println!("Result Moment: ");
            println!("{moment:#?}");
        }
        Err(err) => {
            print!("Arithmetic operation failed: {err}");
        }
    }
}

// This is deliberately not open to user input, limited in scope for proof of concept because
// anything further will add more layers of complexity and be closer to actual library
fn postgresql_storage_test() {
    println!(
        "\n------------ PostgreSQL Database Conversion, Storage and Retrieval Test ------------"
    );

    let mut client = match connect() {
        Ok(client) => client,
        Err(err) => {
            println!("Error on testing database: {err}");
            return;
        }
    };

    if let Err(err) = ensure_schema(&mut client) {
        println!("Failed to validate schema, error on calling function: {err}");
        return;
    }

    let db_moment_header = MomentHeader {
        time_resolution_level: 4,
        date_range_level: 1,
        zone_level: 1,
        accuracy: Accuracy::Start,
        leap_counter_length: 0,
        has_uncertainty: false,
        lsl_status: 0,
    };

    let db_moment = Moment {
        header: db_moment_header,
        date_value: 2_461_050,
        time_value: Some(720),
        zone_value: Some(4),
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    };

    let db_period_header = PeriodHeader {
        sign: Sign::Positive,
        time_resolution_level: 4,
        date_range_level: 1,
        leap_counter_length: 0,
        has_uncertainty: false,
        lsl_status: 0,
    };

    let db_period = Period {
        header: db_period_header,
        date_duration: 2,
        time_duration: Some(720),
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    };

    let moment_id = match insert_moment(&mut client, &db_moment) {
        Ok(id) => id,
        Err(err) => {
            println!("Failed to insert moment: {err}");
            return;
        }
    };

    let loaded_moment = match load_moment(&mut client, moment_id) {
        Ok(moment) => moment,
        Err(err) => {
            println!("Failed to load moment: {err}");
            return;
        }
    };

    println!("Inserted Moment ID: {moment_id}");
    println!(
        "Loaded Moment is the same as the original: {}",
        loaded_moment == db_moment
    );

    if let Err(err) = print_row_summary(&mut client, moment_id) {
        println!("Failed to print Moment row usmmary: {err}");
    }

    let period_id = match insert_period(&mut client, &db_period) {
        Ok(id) => id,
        Err(err) => {
            println!("Failed to insert Period: {err}");
            return;
        }
    };

    let loaded_period = match load_period(&mut client, period_id) {
        Ok(period) => period,
        Err(err) => {
            println!("Failed to load period: {err}");
            return;
        }
    };

    println!("Inserted period ID: {period_id}");
    println!(
        "Loaded Period is the same as original: {}",
        loaded_period == db_period
    );

    if let Err(err) = print_row_summary(&mut client, period_id) {
        println!("Failed to print period row summay: {err}");
    }
}
