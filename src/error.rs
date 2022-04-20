/*
 *   This file is part of NCC Group Scrying https://github.com/nccgroup/scrying
 *   Copyright 2020-2021 David Young <david(dot)young(at)nccgroup(dot)com>
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
    #[error("IO error: {0}")]
    Io(String),

    #[error("RDP error: {0}")]
    Rdp(String),

    #[error("MPSC error: {0}")]
    Mpsc(String),

    #[error("Template error: {0}")]
    Template(String),

    #[error("VNC error: {0}")]
    Vnc(String),

    #[error("Conversion error: {0}")]
    Conversion(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<rdp::model::error::Error> for Error {
    fn from(e: rdp::model::error::Error) -> Self {
        Self::Rdp(format!("{:?}", e))
    }
}

impl From<image::error::ImageError> for Error {
    fn from(e: image::error::ImageError) -> Self {
        Self::Rdp(format!("Image error: {e}"))
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for Error {
    fn from(e: std::sync::mpsc::SendError<T>) -> Self {
        Self::Mpsc(e.to_string())
    }
}

impl From<askama::shared::Error> for Error {
    fn from(e: askama::shared::Error) -> Self {
        Self::Template(e.to_string())
    }
}

impl From<vnc::Error> for Error {
    fn from(e: vnc::Error) -> Self {
        Self::Vnc(e.to_string())
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Self::Conversion(e.to_string())
    }
}

impl From<std::array::TryFromSliceError> for Error {
    fn from(e: std::array::TryFromSliceError) -> Self {
        Self::Conversion(e.to_string())
    }
}
