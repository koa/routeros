use core::convert::{From, Infallible};
use core::fmt::{Display, Formatter};
use std::net::AddrParseError;
use std::num::ParseIntError;
use std::str::ParseBoolError;

use crate::RosError::FieldMissingError;
use mac_address::MacParseError;

pub mod client;
pub mod model;
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[derive(Debug)]
pub enum RosError {
    TokioError(tokio::io::Error),
    SimpleMessage(String),
    ParseIntError(ParseIntError),
    ParseBoolError(ParseBoolError),
    AddrParseError(AddrParseError),
    MacParseError(MacParseError),
    Umbrella(Vec<RosError>),
    FieldWriteError {
        structure: &'static str,
        field_name: String,
        field_value: String,
        error: Box<RosError>,
    },
    FieldMissingError {
        structure: &'static str,
        field_name: String,
        field_value: String,
    },
}

impl RosError {
    pub fn field_missing_error(
        structure: &'static str,
        field_name: &str,
        field_value: &str,
    ) -> RosError {
        FieldMissingError {
            structure,
            field_name: field_name.to_string(),
            field_value: field_value.to_string(),
        }
    }
}

impl Display for RosError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RosError::TokioError(e) => Display::fmt(&e, f),
            RosError::SimpleMessage(msg) => f.write_str(msg),
            RosError::ParseIntError(e) => std::fmt::Display::fmt(&e, f),
            RosError::ParseBoolError(e) => std::fmt::Display::fmt(&e, f),
            RosError::AddrParseError(e) => std::fmt::Display::fmt(&e, f),
            RosError::MacParseError(e) => std::fmt::Display::fmt(&e, f),
            RosError::Umbrella(errors) => {
                for error in errors {
                    std::fmt::Display::fmt(&error, f)?;
                }
                Ok(())
            }
            RosError::FieldWriteError {
                structure,
                field_name,
                field_value,
                error,
            } => {
                f.write_str("Error on field on ")?;
                f.write_str(structure)?;
                f.write_str(": ")?;
                f.write_str(&field_name)?;
                f.write_str(" value ")?;
                f.write_str(&field_value)?;
                f.write_str(": ")?;
                std::fmt::Display::fmt(&error, f)?;
                Ok(())
            }
            RosError::FieldMissingError {
                structure,
                field_name,
                field_value,
            } => {
                f.write_str("Missing ")?;
                f.write_str(structure)?;
                f.write_str(": ")?;
                f.write_str(&field_name)?;
                f.write_str(" value from api ")?;
                f.write_str(&field_value)
            }
        }
    }
}

impl From<ParseIntError> for RosError {
    fn from(e: ParseIntError) -> Self {
        RosError::ParseIntError(e)
    }
}

impl From<ParseBoolError> for RosError {
    fn from(e: ParseBoolError) -> Self {
        RosError::ParseBoolError(e)
    }
}

impl From<AddrParseError> for RosError {
    fn from(e: AddrParseError) -> Self {
        RosError::AddrParseError(e)
    }
}

impl From<MacParseError> for RosError {
    fn from(e: MacParseError) -> Self {
        RosError::MacParseError(e)
    }
}

impl From<tokio::io::Error> for RosError {
    fn from(e: tokio::io::Error) -> Self {
        RosError::TokioError(e)
    }
}

impl From<String> for RosError {
    fn from(e: String) -> Self {
        RosError::SimpleMessage(e)
    }
}

impl From<&str> for RosError {
    fn from(e: &str) -> Self {
        RosError::SimpleMessage(String::from(e))
    }
}

impl From<Infallible> for RosError {
    fn from(_: Infallible) -> Self {
        panic!("Infallible means it cannot happen");
    }
}

impl std::error::Error for RosError {}
