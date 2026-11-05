use super::ErrorBag;
use std::error::Error;

/// Error type build over ErrorBag, containing source code location and optional message
/// Note that only creating via macro is possible to catch line and file
#[derive(Debug)]
pub struct PaymentError {
    pub inner: Box<ErrorBag>,
    pub msg: Option<String>,
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

impl Error for PaymentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.inner.as_ref())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boxed_inner_preserves_error_source_type() {
        let error = crate::err_custom_create!("test failure");
        let source = error.source().unwrap();
        assert!(matches!(
            source.downcast_ref::<ErrorBag>(),
            Some(ErrorBag::CustomError(_))
        ));
        assert_eq!(source.to_string(), "CustomError: test failure");
    }
}
