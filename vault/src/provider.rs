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

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::{Block, BlockId, EncryptedBlock};

pub struct Provider {
    blocks: HashMap<BlockId, Block>,
}

impl Provider {
    pub fn new() -> Provider {
        Provider {
            blocks: HashMap::new(),
        }
    }

    pub fn get_block(&self, id: BlockId) -> &Block {
        // TODO: Check if it already exists in-memory
        // TODO: Check if the disk has a copy
        // TODO: Check if any LAN devices have a copy
        // TODO: Get it from the service

        self.blocks.get(&id).unwrap()
    }

    // TODO: Single-file on-disk cache support ... dynamically sized capnp header and then aligned blocks follow

    pub fn load_block_from_file(&mut self, id: BlockId, key: u128) -> &Block {
        let path = Self::id_to_path(id);
        let block = if let Ok(data) = fs::read(&path) {
            EncryptedBlock::from_data(data).decrypt(key)
        } else {
            panic!("Failed to read from file {path:?}");
        };
        self.blocks.insert(id, block);
        self.blocks.get(&id).unwrap()
    }

    // TODO: Might need to remove mut to ensure easier usage
    pub fn add_block(&mut self, id: BlockId, encrypted_block: EncryptedBlock, block: Block) {
        // If we already have it, then no need to add it again.
        if self.blocks.contains_key(&id) {
            return;
        }
        self.blocks.insert(id, block);

        // Save it to disk
        // TODO: Check if the disk already has it
        fs::write(Self::id_to_path(id), encrypted_block.data()).unwrap();
    }

    fn id_to_path(id: BlockId) -> PathBuf {
        format!("temp/{}.bin", id.base64()).into()
    }

    pub fn load_block_id_from_file(path: impl Into<PathBuf>) -> BlockId {
        let path = path.into();
        let block_id = if let Ok(data) = fs::read(&path) {
            BlockId::from_data(data.try_into().unwrap())
        } else {
            panic!("Failed to read from file {path:?}");
        };
        block_id
    }

    pub fn save_block_id_to_file(id: BlockId, path: impl Into<PathBuf>) {
        let path = path.into();
        fs::write(path, id.data()).unwrap();
    }
}
