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

use std::io;

use vault::File;
use vault::Vault;

pub struct TaskManager<'a> {
    vault: &'a mut Vault,
}

impl<'a> TaskManager<'a> {
    pub fn new(vault: &mut Vault) -> TaskManager {
        TaskManager { vault }
    }

    pub fn put(&mut self, s: &str) -> Result<&File, io::Error> {
        self.vault.put(s)
    }

    pub fn get(&self, s: &str) -> Option<&File> {
        self.vault.get(s)
    }

    pub fn list(&self) -> Vec<&str> {
        self.vault.list()
    }
}
