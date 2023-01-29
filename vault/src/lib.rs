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
use std::io;
use std::path::{Path, PathBuf};

mod file;
pub use file::File;

pub struct Vault {
    path: PathBuf,
    files: Vec<File>,
}

impl Vault {
    pub fn open(path: impl Into<PathBuf>) -> Vault {
        let path = path.into();
        let mut vault = Vault {
            path,
            files: Vec::new(),
        };
        if let Ok(data) = fs::read_to_string(&vault.path) {
            vault.deserialize(data);
        }
        vault
    }

    pub fn close(&self) {
        fs::write(&self.path, self.serialize()).unwrap();
    }

    fn serialize(&self) -> String {
        self.files
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<&str>>()
            .join("\n")
    }

    fn deserialize(&mut self, data: String) {
        self.files = data
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(|s| File {
                name: String::from(s),
                data: Vec::new(),
            })
            .collect();
    }

    pub fn put(&mut self, name: &str) -> Result<&File, io::Error> {
        let p = Path::new(name);
        let f = File::from_os(p)?;
        // The eventual .last().unwrap() is critically depending on the .push()
        self.files.push(f);
        Ok(self.files.last().unwrap())
    }

    pub fn get(&self, name: &str) -> Option<&File> {
        self.files.iter().find(|&f| f.name == name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.files.iter().map(|f| f.name.as_str()).collect()
    }
}

impl Drop for Vault {
    fn drop(&mut self) {
        self.close();
    }
}
