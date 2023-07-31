mod bag;
mod custom;
mod wrapped;

pub use bag::ErrorBag;
pub use custom::{CustomError, TransactionFailedError};
pub use wrapped::Web3ProxyError;

/// Export macros for creating errors
mod macros;
