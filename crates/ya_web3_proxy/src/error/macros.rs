///This macro creates a new error object with line info
#[macro_export]
macro_rules! err_create {
    ($t:expr) => {
        Web3ProxyError {
            inner: ErrorBag::from($t),
            msg: None,
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

///This macro creates a new error object with line info
#[macro_export]
macro_rules! err_custom_create {
    ($($t:tt)*) => {
        Web3ProxyError {
            inner: ErrorBag::from(CustomError::from_owned_string(format!($($t)*))),
            msg: None,
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

///This macro is wrapping error with line + file info without optional message
#[macro_export]
macro_rules! err_from {
    () => {
        |e| Web3ProxyError {
            inner: ErrorBag::from(e),
            msg: None,
            file: file!(),
            line: line!(),
            column: column!(),
        }
    };
}

///This macro is wrapping error with line + file info with optional message
///Message is formatted with arguments
#[macro_export]
macro_rules! err_from_msg {
    ($($t:tt)*) => {{
        |e| Web3ProxyError {
            inner: ErrorBag::from(e),
            msg: Some(format!($($t)*)),
            file: file!(),
            line: line!(),
            column: column!(),
        }
    }};
}
