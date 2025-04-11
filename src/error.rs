use data_encoding::DecodeError;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ValueError(String),
    DecodeError(DecodeError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::ValueError(message) => write!(f, "ValueError: {}", message),
            Error::IoError(err) => write!(f, "IoError: {}", err),
            Error::DecodeError(err) => write!(f, "DecodeError: {}", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<DecodeError> for Error {
    fn from(err: DecodeError) -> Self {
        Error::DecodeError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
