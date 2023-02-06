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
use std::path::PathBuf;

use crate::BlockKind;
use crate::EncryptedBlock;
use crate::File;
use crate::InfoBlock;
use crate::NodeKind;
use crate::Provider;

pub struct Vault<'a> {
    path: PathBuf,
    provider: &'a Provider,
    vault: InfoBlock,
    root: InfoBlock,
    index: InfoBlock,
}

impl<'a> Vault<'a> {
    pub fn open(provider: &'a Provider, path: impl Into<PathBuf>) -> Vault<'a> {
        let path = path.into();
        let vault_id = Provider::load_block_id_from_file(path.clone());

        println!("Opening vault starting at block {}", vault_id.base64());

        let vault_block = provider.load_block_from_file(vault_id, 0).info();

        let (root_id, index_id) = vault_block.get_root_id_and_index_id();

        let root_block = provider.load_block_from_file(root_id, 0).info();
        let index_block = provider.load_block_from_file(index_id, 0).info();

        Vault {
            path,
            provider,
            vault: vault_block,
            root: root_block,
            index: index_block,
        }
    }

    pub fn initialize(provider: &'a Provider, path: impl Into<PathBuf>) -> Vault<'a> {
        let path = path.into();

        // Initialize the root block
        let root_block = InfoBlock::new_directory();
        let root_block = root_block.info().create("welcome", NodeKind::Directory);
        let encrypted_root_block = EncryptedBlock::encrypt(&root_block, 0);
        let root_id = encrypted_root_block.id(BlockKind::Info);
        let root_block = provider
            .add_block(root_id, encrypted_root_block, root_block)
            .info();

        println!("Initialized root  block {}", root_id.base64());

        // Initialize the index block
        let index_block = InfoBlock::new_index();
        let encrypted_index_block = EncryptedBlock::encrypt(&index_block, 0);
        let index_id = encrypted_index_block.id(BlockKind::Info);
        let index_block = provider
            .add_block(index_id, encrypted_index_block, index_block)
            .info();

        println!("Initialized index block {}", index_id.base64());

        // Initialize the vault block
        let vault_block = InfoBlock::new_vault(root_id, index_id);
        let encrypted_vault_block = EncryptedBlock::encrypt(&vault_block, 0);
        let vault_id = encrypted_vault_block.id(BlockKind::Info);
        let vault_block = provider
            .add_block(vault_id, encrypted_vault_block, vault_block)
            .info();

        println!("Initialized vault block {}", vault_id.base64());

        Provider::save_block_id_to_file(vault_id, path.clone());

        Vault {
            path,
            provider,
            vault: vault_block,
            root: root_block,
            index: index_block,
        }
    }

    pub fn put(&mut self, name: &str) -> Result<&File, io::Error> {
        /*
        let p = Path::new(name);
        let f = File::from_os(p)?;
        // The eventual .last().unwrap() is critically depending on the .push()
        self.files.push(f);
        Ok(self.files.last().unwrap())
        */
        Err(io::Error::new(io::ErrorKind::Other, "foobar"))
    }

    pub fn create_directory(&mut self, name: &str) {
        println!("Creating directory ..");

        let root_block = self.root.create(name, NodeKind::Directory);
        let encrypted_block = EncryptedBlock::encrypt(&root_block, 0);
        let root_block_id = encrypted_block.id(BlockKind::Info);
        let root_block = self
            .provider
            .add_block(root_block_id, encrypted_block, root_block)
            .info();

        println!("Created a new root  block {}", root_block_id.base64());

        let vault_block = self.vault.update_root_id(root_block_id);
        let encrypted_block = EncryptedBlock::encrypt(&vault_block, 0);
        let vault_block_id = encrypted_block.id(BlockKind::Info);
        let vault_block = self
            .provider
            .add_block(vault_block_id, encrypted_block, vault_block)
            .info();

        println!("Created a new vault block {}", vault_block_id.base64());

        Provider::save_block_id_to_file(vault_block_id, self.path.clone());

        self.root = root_block;
        self.vault = vault_block;
    }

    pub fn get(&self, name: &str) -> Option<&File> {
        None
    }

    pub fn list(&mut self) -> Vec<(NodeKind, &str)> {
        self.root.directory_list(0)
    }
}
