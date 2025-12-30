//! # OCSF Types for Rust
//! Provids strongly typed Rust structs for the [OCSF](https://schema.ocsf.io/)
//! The types are generated programmatically from the official OCSF schema.
//! ## Usage
//! ```rust
//! use ocsf_types::AccountChange;
//! let mut event = AccountChange::default();
//! event.activity_id = Some(1);
//! event.class_uid = Some(1001);
//! event.message = Some("User password changed".to_string());
//! ```
//! ## Features
//! - **Strongly Typed**
//! - **Serde Integration**
//! - **Built from Official OCSF Schema**
#![recursion_limit = "512"]
pub mod ocsf_generated;
pub use ocsf_generated::*;
