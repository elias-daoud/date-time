# Date-Time

This repository is for the Implementation part of the bachelor's thesis "Approaching Uncertainty and Robustness in Date-Time Representation" on improving date-time handling given uncertainty and other real-world considerations such as leap second correction or precision. 

The implementation is written in Rust and uses PostgreSQL for simple storage and retrieval testing. It is not intended to be a complete date-time library. It demonstrates that the proposed model can be operationally evaluated and stored in a database, while being reliably encoded and decoded.

## Functionality

- Moment and Period data structures
- Binary header/body encoding and decoding
- Notation parsing and rendering
- Limited arithmetic operations
- Logical comparison operations
- PostgreSQL storage and retrieval
- Limited projection into PostgreSQL-native date-time types


## Requirements

- Rust
- PostgreSQL
- A PostgreSQL database named:

```text
datetime-thesis
```

The default connection string used by the program is:

```text
host=localhost user=postgres password=1234 dbname=datetime-thesis
```

These can be changed within the code if desired or the `DATABSE_URL` environment variable set while running the program.


## Database setup

Create a PostgreSQL database manually, for example through pgAdmin 4.

The program creates the required table automatically when the database test is run, so no separate SQL setup file is required.

The table stores the encoded binary value, the textual notation and optional PostgreSQL-native date-time conversion fields.

## Running the project

Clone the repository and run:

```bash
cargo run
```

The program runs manual tests for the model, including notation, encoding/decoding and database storage/retrieval.
The user will also be able to input Moment/Period data using the defined notation, with the program showing the encode-decode round trips work.

## Scope

This implementation is intentionally limited. Its purpose is to demonstrate the feasibility of the proposed model, otherwise the work can quickly grow in complexity towards a production-ready date-time library. Several edge cases and broader operations are left as future work in the thesis.

Furthermore, each property in a Moment/Period is limited to 16 bytes due to Rust's u128 data size, although a Moment/Period is represented as a vector of bytes `Vec<u8>` and may in total represent far larger values than 16 bytes.

Alternatively, this could be used to extend the implementation to maximally cover the model by having the properties be vectors of bytes, but due to the increase in complexity and this being a proof of concept, it is left as future work.
