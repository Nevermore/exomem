/*
    Copyright 2019-2023 OÜ Nevermore <strom@nevermore.ee>

    This file is part of exomem.

    Exomem is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as
    published by the Free Software Foundation, either version 3 of the
    License, or (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

pub struct File {
    pub name: String,
    pub data: Vec<u8>,
}

impl File {
    pub fn from_os(path: &Path) -> Result<File, Error> {
        if !path.is_file() {
            return Err(Error::new(ErrorKind::InvalidInput, "Not a file."));
        }
        let name = path
            .file_name()
            .ok_or(Error::new(
                ErrorKind::InvalidInput,
                "Can't determine file name.",
            ))?
            .to_str()
            .ok_or(Error::new(
                ErrorKind::InvalidInput,
                "Can't determine file name because of invalid Unicode.",
            ))?;
        let data = fs::read(path)?;
        Ok(File {
            name: String::from(name),
            data,
        })
    }
}
