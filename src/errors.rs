use std::fmt;

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum TranspilerError {
    ParseError {
        message: String,
        location: SourceLocation,
    },
    ValidationError {
        message: String,
        hints: Vec<String>,
    },
    IoError {
        message: String,
        path: String,
    },
    InternalError {
        message: String,
    },
}

impl fmt::Display for TranspilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TranspilerError::ParseError { message, location } => {
                write!(
                    f,
                    "{}:{}:{} — Parse error: {}",
                    location.file, location.line, location.column, message
                )
            }
            TranspilerError::ValidationError { message, hints } => {
                if hints.is_empty() {
                    write!(f, "Validation error: {}", message)
                } else {
                    write!(f, "Validation error: {}\nHints:\n  {}", message, hints.join("\n  "))
                }
            }
            TranspilerError::IoError { message, path } => {
                write!(f, "IO error in '{}': {}", path, message)
            }
            TranspilerError::InternalError { message } => {
                write!(f, "Internal error: {}", message)
            }
        }
    }
}
