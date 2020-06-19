/*
 *   This file is part of NCC Group Scrying https://github.com/nccgroup/scrying
 *   Copyright 2020 David Young <david(dot)young(at)nccgroup(dot)com>
 *   Released as open source by NCC Group Plc - https://www.nccgroup.com
 *
 *   Scrying is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU General Public License as published by
 *   the Free Software Foundation, either version 3 of the License, or
 *   (at your option) any later version.
 *
 *   Scrying is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *   GNU General Public License for more details.
 *
 *   You should have received a copy of the GNU General Public License
 *   along with Scrying.  If not, see <https://www.gnu.org/licenses/>.
*/

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Chrome error: {0}")]
    ChromeError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("RDP error: {0}")]
    RdpError(String),

    #[error("MPSC error: {0}")]
    MpscError(String),

    #[error("Template error: {0}")]
    TemplateError(String),
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

impl From<image::error::ImageError> for Error {
    fn from(e: image::error::ImageError) -> Self {
        Self::RdpError(format!("Image error: {}", e.to_string()))
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for Error {
    fn from(e: std::sync::mpsc::SendError<T>) -> Self {
        Self::MpscError(e.to_string())
    }
}

impl From<askama::shared::Error> for Error {
    fn from(e: askama::shared::Error) -> Self {
        Self::TemplateError(e.to_string())
    }
}
