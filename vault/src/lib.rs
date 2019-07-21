/*
    Copyright 2019 OÜ Nevermore <strom@nevermore.ee>

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

pub struct Vault {
    location: String,
	files: Vec<String>,
}

impl Vault {
    pub fn open(location: String) -> Vault {
        let mut v = Vault{ location: location, files: Vec::new() };

        if let Ok(data) = fs::read_to_string(&v.location) {
            v.deserialize(data);
        }

        v
    }

    pub fn close(&self) {
        fs::write(&self.location, self.serialize()).unwrap();
    }

    fn serialize(&self) -> String {
        self.files.join("\n")
    }

    fn deserialize(&mut self, data: String) {
        self.files = data.split("\n").filter(|s| s.len() > 0).map(|s| String::from(s)).collect();
    }

	pub fn put(&mut self, name: String) {
		self.files.push(name);
	}

    pub fn get(&self, name: &str) -> bool {
        self.files.iter().any(|s| s == name)
    }

	pub fn list(&self) -> &Vec<String> {
		&self.files
	}
}