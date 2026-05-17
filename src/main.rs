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
        date_value: 2_461_162,
        time_value: Some(40_000),
        zone_value: Some(0),
        positive_leap_seconds: None,
        negative_leap_seconds: None,
        uncertainty_offset: None,
        lsl_jdn: None,
    };

    let moment_b = Moment {
        header: arithmetic_moment_header,
        date_value: 2_461_162,
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

    print_period_arithmetic_result(
        "Period + Period: @P2DT00:16:40 {d:1 t:5 l:0-0 u:0 lsv:0}@ + @P2DT00:08:20 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        period_add(&period_a, &period_b),
    );
    print_period_arithmetic_result(
        "Period minus Period: @P2DT00:16:40 {d:1 t:5 l:0-0 u:0 lsv:0}@ - @P2DT00:08:20 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        period_subtract(&period_a, &period_b),
    );
    print_period_arithmetic_result(
        "Moment minus Moment: @G2026-05-01T13:53:20 {d:1 t:5 l:0-0 u:0 lsv:0}@ - @G2026-05-01T11:06:40 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        moment_subtract(&moment_b, &moment_a),
    );

    print_moment_arithmetic_result(
        "Moment plus Period: @G2026-05-01T11:06:40 {d:1 t:5 l:0-0 u:0 lsv:0}@ + @P2DT00:16:40 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        moment_add_period(&moment_a, &period_a),
    );
    print_moment_arithmetic_result(
        "Moment minus Period: @G2026-05-01T11:06:40 {d:1 t:5 l:0-0 u:0 lsv:0}@ + @P2DT00:08:20 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        moment_subtract_period(&moment_b, &period_b),
    );

    // Allen relation testing for logical operations. Limited

    print_moment_logical_operation(
        "Moment A @G2026-05-01T11:06:40 {d:1 t:5 l:0-0 u:0 lsv:0}@ compared to Moment B @G2026-05-01T13:53:20 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        compare_moments(&moment_a, &moment_b),
    );
    print_moment_logical_operation(
        "Moment B @G2026-05-01T13:53:20 {d:1 t:5 l:0-0 u:0 lsv:0}@ compared to Moment A @G2026-05-01T11:06:40 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        compare_moments(&moment_b, &moment_a),
    );

    print_period_logical_operation(
        "Period A @P2DT00:16:40 {d:1 t:5 l:0-0 u:0 lsv:0}@ compared to Period B @P2DT00:08:20 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        compare_periods(&period_a, &period_b),
    );
    print_period_logical_operation(
        "Period B @P2DT00:08:20 {d:1 t:5 l:0-0 u:0 lsv:0}@ compared to Period A @P2DT00:16:40 {d:1 t:5 l:0-0 u:0 lsv:0}@",
        compare_periods(&period_b, &period_a),
    );

    notation_user_input();
}

fn notation_user_input() {
    loop {
        println!("\n----- Notation input and database insertion test ----");
        println!("Use M for Moment or P for Period, or Q to quit program");
        println!(
            "Moment example: @G2026-01-01T12:00+01:00 {{d:1 t:4 z:1 a:s l:37-0 u:10 lsv:2457755}}@"
        );
        println!("Period example: @P2DT12:00 {{d:1 t:4 l:0-0 u:10 lsv:0}}@");

        let kind = read_input("Type M or P, or Q: ");
        let kind = kind.trim().to_lowercase();

        if kind == "q" {
            println!("Terminating program");
            break; // just breaks loop to end program
        }

        if kind != "m" && kind != "p" {
            println!("Invalid input. Only M, P or Q allowed.");
            continue;
        }

        let notation = read_input("Notation: ");

        match kind.as_str() {
            "m" => {
                let moment = match parse_moment_notation(&notation) {
                    Ok(moment) => moment,
                    Err(err) => {
                        println!("Moment parsing failure: {err}");
                        continue;
                    }
                };

                println!("Parsed Moment: ");
                println!("{moment:#?}");

                let save_to_db = read_input("Insert into PostgreSQL? Y/N: ");

                if save_to_db.trim().eq_ignore_ascii_case("y") {
                    insert_moment_from_input(&moment);
                }
            }

            "p" => {
                let period = match parse_period_notation(&notation) {
                    Ok(period) => period,
                    Err(err) => {
                        println!("Period parsing failure: {err}");
                        continue;
                    }
                };

                println!("Parsed Period: ");
                println!("{period:#?}");

                let save_to_db = read_input("Insert into PostgreSQL? Y/N: ");

                if save_to_db.trim().eq_ignore_ascii_case("y") {
                    insert_period_from_input(&period);
                }
            }

            _ => unreachable!(),
        }
    }
}

fn insert_moment_from_input(moment: &Moment) {
    let mut client = match connect() {
        Ok(client) => client,
        Err(err) => {
            println!("DB connection failed : {err}");
            return;
        }
    };

    if let Err(err) = ensure_schema(&mut client) {
        println!("Schema setup failed: {err}");
        return;
    }

    match insert_moment(&mut client, moment) {
        Ok(id) => {
            println!("Inserted Moment with id: {id}");
            if let Err(err) = print_row_summary(&mut client, id) {
                println!("Failed to print Moment row summary: {err}");
            }
        }
        Err(err) => println!("Failed to insert Moment: {err}"),
    }
}

fn insert_period_from_input(period: &Period) {
    let mut client = match connect() {
        Ok(client) => client,
        Err(err) => {
            println!("DB connection failed : {err}");
            return;
        }
    };

    if let Err(err) = ensure_schema(&mut client) {
        println!("Schema setup failed: {err}");
        return;
    }

    match insert_period(&mut client, period) {
        Ok(id) => {
            println!("Inserted period with id: {id}");
            if let Err(err) = print_row_summary(&mut client, id) {
                println!("Failed to print Moment row summary: {err}");
            }
        }
        Err(err) => println!("Failed to insert Period: {err}"),
    }
}

fn read_input(prompt: &str) -> String {
    println!("{prompt}");
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
            println!("Logical comparison has failed: {err}");
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
            println!("Logical comparison has failed: {err}");
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
            println!("{decoded:#?}\n");
            println!("\nNumber of consumed header bytes: {consumed}\n");
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
            println!("{decoded:#?}\n");
            println!("\nNumber of consumed header bytes: {consumed}\n");
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
            println!("{decoded:#?}\n");
            println!("\nNumber of consumed body bytes: {consumed}\n");
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
            println!("{decoded:#?}\n");
            println!("\nNumber of consumed body bytes: {consumed}\n");
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
            println!("Arithmetic operation failed: {err}");
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
            println!("Arithmetic operation failed: {err}");
        }
    }
}
