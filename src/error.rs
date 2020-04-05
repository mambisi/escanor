use std::error;
use std::fmt;
use std::option;

#[derive(Debug)]
pub struct SyntaxError;
#[derive(Debug)]
pub struct InternalError;

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(error) syntax error")
    }
}

// This is important for other errors to wrap this one.
impl error::Error for SyntaxError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(error) internal error")
    }
}

// This is important for other errors to wrap this one.
impl error::Error for InternalError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}
