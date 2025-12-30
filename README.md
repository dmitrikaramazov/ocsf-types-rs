# OCSF Types for Rust
[![Crates.io](https://img.shields.io/crates/v/ocsf-types.svg)](https://crates.io/crates/ocsf-types)
[![Docs.rs](https://docs.rs/ocsf-types/badge.svg)](https://docs.rs/ocsf-types)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Downloads](https://img.shields.io/crates/d/ocsf-types.svg)](https://crates.io/crates/ocsf-types)
![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)

Strongly typed Rust structs for the [OCSF](https://schema.ocsf.io/)

This crate provides native Rust types for OCSF events, objects, and enums. It is generated programmatically from the official OCSF schema. It uses the official python OCSF compile tool to first create a flattened JSON schema then use `serde` to parse it. 

The goal of this project is to provide an easy, safe interface that fully matches the OCSF specifications. 

## Usage

Add this to your `Cargo.toml`:
```toml
[dependencies]
ocsf-types = "0.2.0"
```

## Example

Here is how you may use this package:
```rs
use ocsf_types::AccountChange;
let event = {
    let mut e = AccountChange::default();
    e.activity_id = Some(1);
    // You should ensure that all required fields are entered
    e
};
let serialized = serde.json::to_string(&event).ok();

let event_2 = serde_json::from_value(
    serde_json::json!({"activity_id":1})
).ok();

```

## Development

If you are interested in building this from scratch or contributing.

### Requirements
- git
- rust
- python ^3.13 with ocsf-lib

### Building
```sh
git submodule update --init --recursive
pip install ocsf-lib
python -m ocsf.compile ocsf-schema/ > src/resolved.json
cargo run --example generate
cargo build
```
Note - if generate.rs is failing, make sure there exists a src/ocsf_generated.rs file

## Notes
We currently ignore data if it is unknown and not mapped to any fields. 
This may result in data loss, but abides by the OCSF standards.
Any data not in a field which should be saved, should be tied to the `unmapped` field by the client.

We do not validate fields, yet. Someone may put in `severity_id:-1` which we would parse without error. 