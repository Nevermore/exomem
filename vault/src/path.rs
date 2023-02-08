/*
    Copyright 2023 OÜ Nevermore <strom@nevermore.ee>

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

use std::path::{Components, PathBuf};

/// Immutable filesystem path to a node in the vault.
///
/// The existence of an instance comes with a validity guarantee.
#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Debug)]
pub struct VaultPath {
    path: PathBuf,
}

impl VaultPath {
    pub fn new(path: impl Into<PathBuf>) -> VaultPath {
        let path = VaultPath { path: path.into() };
        assert!(path.valid());
        path
    }

    fn new_unchecked(path: impl Into<PathBuf>) -> VaultPath {
        VaultPath { path: path.into() }
    }

    fn valid(&self) -> bool {
        // TODO: Check more stuff
        //			- No contents of ".." or "."
        //			- Valid UTF-8
        self.path.has_root()
    }

    pub fn parent(&self) -> Option<VaultPath> {
        self.path
            .parent()
            .map(|path| VaultPath::new_unchecked(path))
    }

    pub fn to_str(&self) -> Option<&str> {
        self.path.to_str()
    }

    pub fn components(&self) -> Components<'_> {
        self.path.components()
    }

    pub fn file_name(&self) -> Option<&str> {
        self.path.file_name().map(|str| str.to_str().unwrap())
    }
}
