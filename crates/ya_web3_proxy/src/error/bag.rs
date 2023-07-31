use super::{CustomError, TransactionFailedError};
use rustc_hex::FromHexError;
use std::fmt::Display;
use std::num::ParseIntError;

/// Enum containing all possible errors used in the library
/// Probably you can use thiserror crate to simplify this process
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum ErrorBag {
    ParseError(ParseIntError),
    IoError(std::io::Error),
    CustomError(CustomError),
    TransactionFailedError(TransactionFailedError),
    FromHexError(FromHexError),
}

impl Display for ErrorBag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorBag::ParseError(parse_int_error) => write!(f, "{parse_int_error}"),
            ErrorBag::IoError(io_error) => write!(f, "{io_error}"),
            ErrorBag::CustomError(custom_error) => write!(f, "{custom_error}"),
            ErrorBag::TransactionFailedError(transaction_failed_error) => {
                write!(f, "{transaction_failed_error}")
            }
            ErrorBag::FromHexError(from_hex_error) => write!(f, "{from_hex_error:?}"),
        }
    }
}

impl std::error::Error for ErrorBag {}

impl From<ParseIntError> for ErrorBag {
    fn from(err: ParseIntError) -> Self {
        ErrorBag::ParseError(err)
    }
}

impl From<std::io::Error> for ErrorBag {
    fn from(err: std::io::Error) -> Self {
        ErrorBag::IoError(err)
    }
}

impl From<CustomError> for ErrorBag {
    fn from(err: CustomError) -> Self {
        ErrorBag::CustomError(err)
    }
}

impl From<TransactionFailedError> for ErrorBag {
    fn from(err: TransactionFailedError) -> Self {
        ErrorBag::TransactionFailedError(err)
    }
}

impl From<FromHexError> for ErrorBag {
    fn from(err: FromHexError) -> Self {
        ErrorBag::FromHexError(err)
    }
}
