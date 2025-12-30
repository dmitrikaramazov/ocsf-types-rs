# OCSF Types for Rust
Strongly typed Rust structs for the [OCSF](https://schema.ocsf.io/)

This crate provides native Rust types for OCSF events, objects, and enums. It is generated programmatically from the official OCSF schema. It uses the official python OCSF compile tool to first create a flattened JSON schema then use `serde` to parse it. 

The goal of this project is to provide an easy, safe interface that fully matches the OCSF specifications. 

## Usage

Add this to your `Cargo.toml`:
```toml
[dependencies]
ocsf-types = "0.1.0"
```

## Example

Here is how you may use this package:
```rs
use ocsf_types_rs::AccountChange;
let mut event = AccountChange::default();
event.activity_id = 1;
event.class_uid = 1001;
event.message = Some("User password changed".to_string());
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