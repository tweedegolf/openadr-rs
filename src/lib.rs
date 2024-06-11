#![doc=include_str!("../README.md")]

mod client;
mod error;
mod generated;
pub mod wire;

pub use client::*;
pub use error::*;
