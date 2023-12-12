use super::ErrorBag;
use std::error::Error;

/// Error type build over ErrorBag, containing source code location and optional message
/// Note that only creating via macro is possible to catch line and file
#[derive(Debug)]
pub struct PaymentError {
    pub inner: ErrorBag,
    pub msg: Option<String>,
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

impl Error for PaymentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}

impl std::fmt::Display for PaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let file_loc = {
            #[cfg(debug_assertions)]
            {
                use std::fs;
                let p = std::path::Path::new(self.file);
                if p.exists() {
                    let path = fs::canonicalize(p).unwrap_or_else(|_| self.file.into());
                    path.display().to_string().replace(r"\\?\", "")
                } else {
                    self.file.replace('\\', "/")
                }
            }
            #[cfg(not(debug_assertions))]
            self.file.replace('\\', "/")
        };

        if let Some(msg) = &self.msg {
            write!(
                f,
                "{}, {}, {}:{}:{}",
                msg, self.inner, file_loc, self.line, self.column
            )
        } else {
            write!(
                f,
                "{}, {}:{}:{}",
                self.inner, file_loc, self.line, self.column
            )
        }
    }
}
