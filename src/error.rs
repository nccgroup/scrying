#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("error: {0}")]
    Error(String),

    #[error("IO error: {0}")]
    IoError(String),
}

impl From<wkhtmltopdf::error::Error> for Error {
    fn from(e: wkhtmltopdf::error::Error) -> Self {
        Self::Error(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}
