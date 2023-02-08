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
use std::path::Component;
use std::path::PathBuf;

use crate::file;
use crate::vault_capnp::node::directory::entry;
use crate::Block;
use crate::BlockId;
use crate::BlockKind;
use crate::EncryptedBlock;
use crate::File;
use crate::InfoBlock;
use crate::NodeKind;
use crate::Provider;
use crate::VaultPath;

pub struct Vault<'a> {
    path: PathBuf,
    provider: &'a Provider,
    vault: InfoBlock,
    root: InfoBlock,
    root_id: BlockId,
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
            root_id,
            index: index_block,
        }
    }

    pub fn initialize(provider: &'a Provider, path: impl Into<PathBuf>) -> Vault<'a> {
        let path = path.into();

        // Initialize the root block
        let root_block = InfoBlock::new_directory();
        let (root_block, _) =
            root_block
                .info()
                .directory_create_local_node(0, "welcome", NodeKind::Directory);
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
            root_id,
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

    pub fn create_directory(&mut self, path: VaultPath) {
        println!("Creating directory ..");

        // Make sure that all the directories exist from left to right

        let mut blocks = vec![Some(self.root.block())]; // None means use parent
        let mut entry_names = vec![""];
        let mut node_indexes = vec![0];
        let mut created_anything = false;
        for component in path.components() {
            match component {
                Component::Prefix(_) => (),               // Ignore
                Component::RootDir => (),                 // Ignore
                Component::CurDir => (),                  // Ignore
                Component::ParentDir => unimplemented!(), // Should probably just forbid for now in VaultPath
                Component::Normal(name) => {
                    // Does it exist?
                    let entry_name = name.to_str().unwrap();
                    let block = blocks
                        .iter()
                        .rev()
                        .find(|block| block.is_some())
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .info();
                    let node_index = *node_indexes.last().unwrap();
                    if let Some((block_id, node_index)) =
                        block.directory_get_entry_block_id_and_node_index(node_index, entry_name)
                    {
                        if let Some(block_id) = block_id {
                            blocks.push(Some(self.provider.get_block(block_id)));
                        } else {
                            blocks.push(None);
                        }
                        node_indexes.push(node_index);
                    } else {
                        // It doesn't exist, so create the directory and continue the loop
                        let (new_block, entry_node_index) = block.directory_create_local_node(
                            node_index,
                            entry_name,
                            NodeKind::Directory,
                        );

                        // Update the parent block
                        *blocks
                            .iter_mut()
                            .rev()
                            .find(|block| block.is_some())
                            .unwrap() = Some(new_block);
                        blocks.push(None); // We use the parent's block
                        node_indexes.push(entry_node_index);
                        created_anything = true;
                    }
                    entry_names.push(entry_name);
                }
            }
        }

        // Tricky task of backtracking and updating all the blockid references

        if created_anything {
            let mut entry_block = None;
            let mut entry_block_id = None;
            let mut entry_node_index = None;
            let mut entry_name = None;

            for i in (0..blocks.len()).rev() {
                let block = &mut blocks[i];
                let node_index = node_indexes[i];
                let name = entry_names[i];

                if let Some(block) = block {
                    if let (Some(entry_node_index), Some(entry_name)) =
                        (entry_node_index, entry_name)
                    {
                        // Make sure the entry is pointing to this
                        if let Some(new_block) =
                            block.info().directory_set_entry_block_id_and_node_index(
                                node_index,
                                entry_name,
                                entry_block_id.as_ref(),
                                entry_node_index,
                            )
                        {
                            *block = new_block;
                        }
                    }

                    let encrypted_block = EncryptedBlock::encrypt(block, 0);
                    let block_id = encrypted_block.id(BlockKind::Info);
                    let block = self
                        .provider
                        .add_block(block_id, encrypted_block, block.clone())
                        .info();
                    println!("Created a new dir   block {}", block_id.base64());

                    entry_block = Some(block);
                    entry_block_id = Some(block_id);
                } else {
                    entry_block = None;
                    entry_block_id = None;
                }
                entry_node_index = Some(node_index as u16);
                entry_name = Some(name);
            }

            let vault_block = self.vault.update_root_id(entry_block_id.unwrap());
            let encrypted_block = EncryptedBlock::encrypt(&vault_block, 0);
            let vault_block_id = encrypted_block.id(BlockKind::Info);
            let vault_block = self
                .provider
                .add_block(vault_block_id, encrypted_block, vault_block)
                .info();

            println!("Created a new vault block {}", vault_block_id.base64());

            Provider::save_block_id_to_file(vault_block_id, self.path.clone());

            self.root = entry_block.unwrap();
            self.vault = vault_block;
        }
    }

    pub fn get(&self, name: &str) -> Option<&File> {
        None
    }

    fn get_path_block_id_and_node_index(&self, path: VaultPath) -> (BlockId, u32) {
        // TODO: Check in-memory cache

        // If we have a parent directory
        if let Some(parent_path) = path.parent() {
            // Get that directory's block id and node index
            // TODO: Perhaps better performance to check here if parent is root, and then immediately use self.root
            let (parent_block_id, parent_node_index) =
                self.get_path_block_id_and_node_index(parent_path);

            let parent_block = self.provider.get_block(parent_block_id).info();

            let file_name = path.file_name().unwrap();
            if let Some((block_id, node_index)) = parent_block
                .directory_get_entry_block_id_and_node_index(parent_node_index, file_name)
            {
                let block_id = block_id.unwrap_or(parent_block_id);
                return (block_id, node_index);
            } else {
                panic!(
                    "No such entry: {:?} in {:?}",
                    file_name,
                    path.parent().unwrap()
                );
            }
        }
        // Root node
        (self.root_id, 0)
    }

    pub fn list(&self, path: VaultPath) -> Vec<(NodeKind, String)> {
        let path = path.into();
        let (block_id, node_index) = self.get_path_block_id_and_node_index(path);
        let list_block = self.provider.get_block(block_id).info();
        list_block
            .directory_list(node_index)
            .iter()
            .map(|(kind, name)| (*kind, String::from(*name)))
            .collect()
    }
}
