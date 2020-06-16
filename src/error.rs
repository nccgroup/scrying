#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[cfg(feature = "wkhtmltoimage")]
    #[error("error: {0}")]
    WkhtmltoimageError(String),

    #[cfg(feature = "headlesschrome")]
    #[error("Chrome error: {0}")]
    ChromeError(String),

    #[error("IO error: {0}")]
    IoError(String),
}

#[cfg(feature = "headlesschrome")]
impl From<failure::Error> for Error {
    fn from(e: failure::Error) -> Self {
        Self::ChromeError(e.to_string())
    }
}

#[cfg(feature = "wkhtmltoimage")]
impl From<wkhtmltopdf::error::Error> for Error {
    fn from(e: wkhtmltopdf::error::Error) -> Self {
        Self::WkhtmltoimageError(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}
