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

use std::io;

use vault::File;
use vault::Vault;

pub struct Controller {
    vault: Vault,
}

impl Controller {
    pub fn new() -> Controller {
        let v = Vault::open(String::from("vault.db"));
        Controller { vault: v }
    }

    pub fn put(&mut self, s: &str) -> Result<&File, io::Error> {
        self.vault.put(s)
    }

    pub fn get(&self, s: &str) -> Option<&File> {
        self.vault.get(s)
    }

    pub fn list_files(&self) -> Vec<&str> {
        self.vault.list()
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.vault.close();
    }
}
