use crate::magnet::MagnetLinkError;
use crate::request::RequestError;
use data_encoding::DecodeError;

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    DecodeError(DecodeError),
    MagnetLinkError(MagnetLinkError),
    RequestError(RequestError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::IoError(err) => write!(f, "IoError: {}", err),
            Error::DecodeError(err) => write!(f, "DecodeError: {}", err),
            Error::MagnetLinkError(err) => write!(f, "MagnetLinkError: {}", err),
            Error::RequestError(err) => write!(f, "RequestError: {}", err),
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

impl From<RequestError> for Error {
    fn from(err: RequestError) -> Self {
        Error::RequestError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
