pub mod client;
mod json;
pub mod model;
include!(concat!(env!("OUT_DIR"), "/generated.rs"));
