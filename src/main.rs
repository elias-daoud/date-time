// Will probably include console testing here
// also note to self, maybe allow user input using prescribed notation
use date_time::model::{Accuracy, MomentHeader};
use date_time::moment_header::{decode_moment_header, encode_moment_header};

fn main() {
    // for small manual testing as I go
    let header = MomentHeader {
        time_resolution_level: 20,
        date_range_level: 3,
        zone_level: 10,
        accuracy: Accuracy::End,
        leap_counter_length: 7,
        has_uncertainty: true,
        lsl_status: 2,
    };

    print_encoded_header("Moment header test", &header);
}

fn print_encoded_header(label: &str, header: &MomentHeader) {
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
