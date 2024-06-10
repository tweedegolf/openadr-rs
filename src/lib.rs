#![doc=include_str!("../README.md")]

mod client;
mod error;
pub mod generated;
pub mod wire;

pub use client::Client;
pub use error::*;
