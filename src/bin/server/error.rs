use std::error;
use std::fmt;

#[derive(Debug)]
pub struct SyntaxError;

#[derive(Debug)]
pub struct InternalError;

#[derive(Debug)]
pub struct InvalidCommand;

#[derive(Debug)]
pub struct CustomMessageError{
    pub detail : String
}

#[derive(Debug)]
pub struct DatabaseError{
    pub detail : String
}


impl fmt::Display for SyntaxError {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", "ERR syntax error")
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
        write!(f, "{}", "ERR internal error")
    }
}

// This is important for other errors to wrap this one.
impl error::Error for InternalError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}


impl fmt::Display for InvalidCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", "ERR invalid command")
    }
}

// This is important for other errors to wrap this one.
impl error::Error for InvalidCommand {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl fmt::Display for CustomMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.detail)
    }
}

// This is important for other errors to wrap this one.
impl error::Error for CustomMessageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl CustomMessageError {
    pub fn new(detail : &str) -> CustomMessageError{
        CustomMessageError {
            detail : detail.to_owned()
        }
    }
}


// -Mark: Database Error
impl error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.detail)
    }
}

impl DatabaseError {
    pub fn new(detail : &str) -> DatabaseError{
        DatabaseError {
            detail : detail.to_owned()
        }
    }
}
