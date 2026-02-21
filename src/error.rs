use std::fmt;

#[derive(Debug)]
pub enum BuclError {
    ParseError(String),
    RuntimeError(String),
    UnknownFunction(String),
    IoError(std::io::Error),
}

impl fmt::Display for BuclError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
            Self::UnknownFunction(name) => write!(f, "Unknown function: '{}'", name),
            Self::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for BuclError {}

impl From<std::io::Error> for BuclError {
    fn from(e: std::io::Error) -> Self {
        BuclError::IoError(e)
    }
}

pub type Result<T> = std::result::Result<T, BuclError>;
