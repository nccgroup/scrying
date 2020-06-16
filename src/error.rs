#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Chrome error: {0}")]
    ChromeError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("RDP error: {0}")]
    RdpError(String),
}

impl From<failure::Error> for Error {
    fn from(e: failure::Error) -> Self {
        Self::ChromeError(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

impl From<rdp::model::error::Error> for Error {
    fn from(e: rdp::model::error::Error) -> Self {
        Self::RdpError(format!("{:?}", e))
    }
}