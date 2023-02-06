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

use std::fmt;

use bytes::Bytes;
use capnp::message::{self, ReaderOptions, ReaderSegments, TypedBuilder};

use crate::vault_capnp::{block, index, node, union_id, NodeKind};

/// `BlockId` is a globally unique 256 bit identifier for [`Block`].
///
/// It also contains a header with some information about the block.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct BlockId {
    /// The raw bytes that make up this `BlockId`.
    ///
    /// The first byte is a header byte, the other 31 are a hash of the block.
    /// Currently only the 6 least significant bits of the header byte are actually used.
    data: [u8; 32],
}

impl BlockId {
    /// Create a new `BlockId` from the provided `hash` and options.
    pub fn new(hash: blake3::Hash, size: usize, has_header: bool) -> BlockId {
        let mut id = BlockId {
            data: *hash.as_bytes(),
        };
        id.set_header(size, has_header);
        id
    }

    /// Create a new `BlockId` from raw `data`.
    pub fn from_data(data: [u8; 32]) -> BlockId {
        BlockId { data }
    }

    // TODO: Write tests for this at every size
    fn set_header(&mut self, size: usize, has_header: bool) {
        let size_marker = 12 - size.ilog2() as u8;
        if size_marker > 15 {
            panic!("Unexpected size marker");
        }
        let mut header = 0;
        if has_header {
            header |= 0b0000_0010u8;
        }
        header |= size_marker << 2;
        self.data[0] = header
    }

    /// Returns the raw bytes that make up this `BlockId`.
    pub fn data(&self) -> &[u8; 32] {
        &self.data
    }

    /// Returns the first of the raw bytes, which is the header byte.
    pub fn header(&self) -> u8 {
        self.data[0]
    }

    /// Returns `true` if the current implementation can properly handle this `BlockId`.
    pub fn supported_version(&self) -> bool {
        // The least significant bit determines the version.
        // We currently support only one version where the bit is zero.
        (self.data[0] & 0b0000_0001u8) == 0
    }

    /// Returns `true` if the block is of a [`supported_version`] and unused bits are zero.
    pub fn valid(&self) -> bool {
        self.supported_version() && (self.data[0] & 0b1100_0000u8 == 0)
    }

    /// Returns `true` if the block has a header.
    pub fn block_has_header(&self) -> bool {
        // The second least significant bit determines the kind of block.
        // 0 == data block, no header
        // 1 == info block, has a header
        (self.data[0] & 0b0000_0010u8) >> 1 != 0
    }

    /// Returns the block size in number of bytes, in powers of two in the range of 4 KiB - 128 MiB.
    ///
    /// The minimum block size of 4 KiB was chosen because it is a very common device sector size.
    /// This choice helps with great compatability with [Advanced Format]. It is likely to offer
    /// better performance than smaller block sizes and less likely to get stuck in a device buffer.
    ///
    /// The choice of calculating powers of two is because it offers good granularity
    /// when choosing for future optimizations while ensuring all the sizes are multiples of 4 KiB.
    ///
    /// The choice of using 4 bits (16 values) comes because 3 bits would get us only to 512 KiB
    /// which is not nearly enough for 100 GB files. On the other hand 5 bits would get us all the
    /// way up to 8 TiB. That block size is prohibitively large for embedded and mobile use.
    /// Luckily 4 bits gets us to 128 MiB which is quite good for 100 GB files while still being
    /// managable by embedded and mobile devices.
    ///
    /// In the long term future a new `BlockId` version could introduce a different scale.
    ///
    /// [Advanced Format]: https://en.wikipedia.org/wiki/Advanced_Format
    pub fn block_size(&self) -> usize {
        // The third to sixth least significant bits (4 bits) determine the size.
        let size_marker = (self.data[0] & 0b0011_1100u8) >> 2;
        // Ranges from 4 KiB to 128 MiB.
        2usize.pow(12 + size_marker as u32)
    }

    /// Returns the Base64 representation of the `BlockId`.
    pub fn base64(&self) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        URL_SAFE_NO_PAD.encode(self.data)
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Figure out if there is a more efficient printing code
        write!(
            f,
            "{:032x}{:032x}",
            u128::from_le_bytes(self.data.as_slice()[0..16].try_into().unwrap()),
            u128::from_le_bytes(self.data.as_slice()[16..32].try_into().unwrap()),
        )
    }
}

impl fmt::Debug for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08b} {:08b}", self.data[0], self.data[1])
    }
}

/// Determines the kind of [`Block`].
///
/// There are two kinds:
/// * Data blocks which are 100% data without any metadata.
/// * Info blocks which start with a header describing the block.
#[derive(Copy, Clone)]
pub enum BlockKind {
    /// 100% of the block is data, there is no metadata.
    Data,
    /// The block starts with a header describing the remaining contents.
    Info,
}

impl BlockKind {
    /// Returns `true` if this kind of block has a header.
    pub fn has_header(&self) -> bool {
        match self {
            BlockKind::Data => false,
            BlockKind::Info => true,
        }
    }
}

/// Immutable encrypted block.
#[derive(Clone)]
pub struct EncryptedBlock {
    /// The raw bytes of this encrypted block.
    data: Bytes,
}

impl EncryptedBlock {
    /// Returns an empty [`EncryptedBlock`].
    pub const fn empty() -> EncryptedBlock {
        EncryptedBlock { data: Bytes::new() }
    }

    /// Returns a new [`EncryptedBlock`] with the provided raw `data`.
    pub fn from_data(data: Bytes) -> EncryptedBlock {
        EncryptedBlock { data }
    }

    /// Returns a new [`EncryptedBlock`] based on `block`.
    pub fn encrypt(block: &Block, _key: u128) -> EncryptedBlock {
        // TODO: Actually encrypt
        EncryptedBlock { data: block.data() }
    }

    /// Returns the decrypted [`Block`].
    pub fn decrypt(&self, _key: u128) -> Block {
        // TODO: Actually decrypt
        Block::from_data(self.data.clone())
    }

    /// Returns a reference to the block's data.
    pub fn data(&self) -> Bytes {
        self.data.clone()
    }

    /// Returns the [`BlockId`] of this [`EncryptedBlock`].
    pub fn id(&self, kind: BlockKind) -> BlockId {
        let hash = blake3::hash(self.data.as_ref());
        BlockId::new(hash, self.data.len(), kind.has_header())
    }
}

/// Immutable unencrypted block.
#[derive(Clone)]
pub struct Block {
    /// The raw bytes of this unencrypted block.
    data: Bytes,
}

impl Block {
    /// Returns an empty [`Block`].
    pub const fn empty() -> Block {
        Block { data: Bytes::new() }
    }

    /// Returns a new [`Block`] from the provided raw `data`.
    pub fn from_data(data: Bytes) -> Block {
        Block { data }
    }

    /// Returns a reference to the block's raw data.
    pub fn data(&self) -> Bytes {
        self.data.clone()
    }

    /// Returns the block's size in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Returns an [`InfoBlock`] if you know this is an info block.
    pub fn info(&self) -> InfoBlock {
        InfoBlock::from(self.clone())
    }
}

impl ReaderSegments for Block {
    fn get_segment(&self, idx: u32) -> Option<&[u8]> {
        match idx {
            0 => Some(self.data.as_ref()),
            _ => None,
        }
    }

    fn len(&self) -> usize {
        1
    }

    fn is_empty(&self) -> bool {
        false
    }
}

/// Immutable unencrypted info block.
pub struct InfoBlock {
    /// The underlying unencrypted [`Block`].
    block: Block,
    /// A capnp message reader pointed to the underlying block.
    message_reader: message::Reader<Block>,
}

impl From<Block> for InfoBlock {
    fn from(block: Block) -> Self {
        InfoBlock {
            block: block.clone(),
            // We construct a capnp message reader directly without doing any segment analysis.
            // Our messages are always expected to be a single segment.
            message_reader: message::Reader::new(block, ReaderOptions::new()),
        }
    }
}

impl InfoBlock {
    pub fn new_vault(root: BlockId, index: BlockId) -> Block {
        let mut message_b = TypedBuilder::<block::Owned>::new_default(); // TODO: Look into allocation strategies
        let block_b = message_b.init_root();
        let nodes_b = block_b.init_nodes(1);
        let node_b = nodes_b.get(0);
        let mut vault_b = node_b.init_vault();
        let root_b = vault_b.reborrow().init_root();
        let mut root_block_id_b = root_b.init_block_id();
        root_block_id_b.set_d1(u64::from_le_bytes(root.data[0..8].try_into().unwrap()));
        root_block_id_b.set_d2(u64::from_le_bytes(root.data[8..16].try_into().unwrap()));
        root_block_id_b.set_d3(u64::from_le_bytes(root.data[16..24].try_into().unwrap()));
        root_block_id_b.set_d4(u64::from_le_bytes(root.data[24..32].try_into().unwrap()));
        let index_b = vault_b.init_index();
        let mut index_block_id_b = index_b.init_block_id();
        index_block_id_b.set_d1(u64::from_le_bytes(index.data[0..8].try_into().unwrap()));
        index_block_id_b.set_d2(u64::from_le_bytes(index.data[8..16].try_into().unwrap()));
        index_block_id_b.set_d3(u64::from_le_bytes(index.data[16..24].try_into().unwrap()));
        index_block_id_b.set_d4(u64::from_le_bytes(index.data[24..32].try_into().unwrap()));

        let segment = match message_b.borrow_inner().get_segments_for_output() {
            capnp::OutputSegments::SingleSegment(ss) => Bytes::copy_from_slice(ss[0]),
            capnp::OutputSegments::MultiSegment(_) => {
                panic!("got multiple output segments, but our reader doesn't want that")
            }
        };

        Block::from_data(segment)
    }

    pub fn new_index() -> Block {
        let mut message_b = TypedBuilder::<index::Owned>::new_default(); // TODO: Look into allocation strategies
        let _block_b = message_b.init_root();

        let segment = match message_b.borrow_inner().get_segments_for_output() {
            capnp::OutputSegments::SingleSegment(ss) => Bytes::copy_from_slice(ss[0]),
            capnp::OutputSegments::MultiSegment(_) => {
                panic!("got multiple output segments, but our reader doesn't want that")
            }
        };

        Block::from_data(segment)
    }

    pub fn new_directory() -> Block {
        let mut message_b = TypedBuilder::<block::Owned>::new_default(); // TODO: Look into allocation strategies
        let block_b = message_b.init_root();
        let nodes_b = block_b.init_nodes(1);
        let node_b = nodes_b.get(0);
        let directory_b = node_b.init_directory();
        directory_b.init_entries(0);

        let segment = match message_b.borrow_inner().get_segments_for_output() {
            capnp::OutputSegments::SingleSegment(ss) => Bytes::copy_from_slice(ss[0]),
            capnp::OutputSegments::MultiSegment(_) => {
                panic!("got multiple output segments, but our reader doesn't want that")
            }
        };

        Block::from_data(segment)
    }

    /// Returns the underlying `Block`.
    pub fn block(&self) -> Block {
        self.block.clone()
    }

    /// Returns a new instance of `block::Reader`.
    fn block_reader(&self) -> block::Reader {
        // Unfortunately Rust lifetimes make it difficult to cache the resulting struct.
        // Luckily the amount of work being done here is minimal.
        self.message_reader
            .get_root::<block::Reader>()
            .expect("failed to get block reader")
    }

    pub fn get_root_id_and_index_id(&self) -> (BlockId, BlockId) {
        let block_r = self.block_reader();
        let nodes_r = block_r.get_nodes().unwrap();
        let node_r = nodes_r.get(0);

        let node::Vault(vault_r) = node_r.which().unwrap() else {
            panic!("Unexpected node");
        };
        let vault_r = vault_r.unwrap();

        let root_r = vault_r.get_root().unwrap();
        let root_id = match root_r.which().unwrap() {
            union_id::Which::LocalId(_) => todo!(),
            union_id::Which::BlockId(block_id_r) => {
                let block_id_r = block_id_r.unwrap();
                let d1 = block_id_r.get_d1().to_le_bytes();
                let d2 = block_id_r.get_d2().to_le_bytes();
                let d3 = block_id_r.get_d3().to_le_bytes();
                let d4 = block_id_r.get_d4().to_le_bytes();

                // TODO: capnp::raw::get_struct_data_section better?

                let mut result = [0; 32];
                result[0..8].copy_from_slice(&d1);
                result[8..16].copy_from_slice(&d2);
                result[16..24].copy_from_slice(&d3);
                result[24..32].copy_from_slice(&d4);

                BlockId::from_data(result)
            }
            union_id::Which::ShardId(_) => todo!(),
        };

        let index_r = vault_r.get_root().unwrap();
        let index_id = match index_r.which().unwrap() {
            union_id::Which::LocalId(_) => todo!(),
            union_id::Which::BlockId(block_id_r) => {
                let block_id_r = block_id_r.unwrap();
                let d1 = block_id_r.get_d1().to_le_bytes();
                let d2 = block_id_r.get_d2().to_le_bytes();
                let d3 = block_id_r.get_d3().to_le_bytes();
                let d4 = block_id_r.get_d4().to_le_bytes();

                let mut result = [0; 32];
                result[0..8].copy_from_slice(&d1);
                result[8..16].copy_from_slice(&d2);
                result[16..24].copy_from_slice(&d3);
                result[24..32].copy_from_slice(&d4);

                BlockId::from_data(result)
            }
            union_id::Which::ShardId(_) => todo!(),
        };

        (root_id, index_id)
    }

    /// Creates a new node of `kind` with `name`.
    ///
    /// Returns the new [`Block`].
    pub fn create(&self, name: &str, kind: NodeKind) -> Block {
        let block_r = self.block_reader();
        let nodes_r = block_r.get_nodes().unwrap();
        let old_nodes_len = nodes_r.len();
        let node_r = nodes_r.get(0);

        let node::Directory(directory_r) = node_r.which().unwrap() else {
            panic!("Unexpected node");
        };
        let directory_r = directory_r.unwrap();

        let entries_r = directory_r.get_entries().unwrap();
        let old_len = entries_r.len();

        let mut message_b = TypedBuilder::<block::Owned>::new_default();
        message_b.set_root(block_r).unwrap();
        let block = message_b.get_root().unwrap();

        let mut nodes = block.init_nodes(old_nodes_len + 1);
        // TODO: Don't init more nodes if we're not gonna inline
        //let nodes = block_r.reborrow().get_nodes().unwrap();
        let node = nodes.reborrow().get(0);

        let node::Directory(dir) = node.which().unwrap() else {
            panic!("Unexpected node");
        };
        let dir = dir.unwrap();

        let mut entries = dir.init_entries(old_len + 1);
        for i in 0..old_len {
            let old_entry = entries_r.reborrow().get(i);
            entries.set_with_caveats(i, old_entry).unwrap();
        }

        let mut entry = entries.reborrow().get(old_len);
        entry.set_name(name);

        // TODO: Add ability to create this new node in a brand new block instead, and then reference it with blockId
        let mut id = entry.init_id();
        let next_local_id = old_nodes_len;
        id.set_local_id(next_local_id as u16); // TODO: Make sure we're not truncating

        let inline_node = nodes.get(next_local_id);
        match kind {
            NodeKind::Directory => {
                let directory = inline_node.init_directory();
                directory.init_entries(0);
            }
            NodeKind::File => {
                let mut file = inline_node.init_file();
                file.set_size(1234);
                // TODO: Set id
            }
            NodeKind::Vault => {
                // TODO
            }
        }

        let segment = match message_b.borrow_inner().get_segments_for_output() {
            capnp::OutputSegments::SingleSegment(ss) => Bytes::copy_from_slice(ss[0]),
            capnp::OutputSegments::MultiSegment(_) => {
                panic!("got multiple output segments, but our reader doesn't want that")
            }
        };

        Block::from_data(segment)
    }

    pub fn directory_list(&self, node_idx: u32) -> Vec<(NodeKind, &str)> {
        let block_r = self.block_reader();
        let nodes_r = block_r.get_nodes().unwrap();
        let node_r = nodes_r.get(node_idx);

        let node::Directory(directory_r) = node_r.which().unwrap() else {
            panic!("Unexpected node");
        };
        let directory_r = directory_r.unwrap();

        let entries_r = directory_r.get_entries().unwrap();

        let mut result = Vec::<(NodeKind, &str)>::with_capacity(entries_r.len() as usize);
        for entry_r in entries_r.iter() {
            assert!(entry_r.has_id());
            let id_r = entry_r.get_id().expect("failed to get id");
            let kind = match id_r.which().expect("failed to get readable id") {
                union_id::Which::LocalId(local_id) => {
                    let entry_node_r = nodes_r.get(local_id as u32);
                    match entry_node_r.which().expect("not a readable node") {
                        node::Which::Directory(_) => NodeKind::Directory,
                        node::Which::File(_) => NodeKind::File,
                        node::Which::Vault(_) => NodeKind::Vault,
                    }
                }
                union_id::Which::BlockId(_) => unimplemented!(),
                union_id::Which::ShardId(_) => unimplemented!(),
            };

            let name = entry_r.get_name().unwrap();
            result.push((kind, name));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use super::*;

    /// Make sure that all `BlockId` variants are properly detected.
    #[test]
    fn block_id_header() {
        let mut id_bytes = [0; 32];
        thread_rng().fill(&mut id_bytes[..]);

        id_bytes[0] = 0b0000_0000;
        let block_id = BlockId::from_data(id_bytes);
        assert!(block_id.supported_version());
        assert!(block_id.valid());
        assert!(!block_id.block_has_header());
        assert_eq!(block_id.block_size(), 4096);

        id_bytes[0] = 0b0000_0001;
        let block_id = BlockId::from_data(id_bytes);
        assert!(!block_id.supported_version());
        assert!(!block_id.valid());

        for unused_bit_a in 0..=1 {
            for unused_bit_b in 0..=1 {
                for header in 0..=1 {
                    for size_marker in 0..=0x0F {
                        id_bytes[0] = 0b0000_0000;
                        if unused_bit_a == 1 {
                            id_bytes[0] |= 0b1000_0000;
                        }
                        if unused_bit_b == 1 {
                            id_bytes[0] |= 0b0100_0000;
                        }
                        if header == 1 {
                            id_bytes[0] |= 0b0000_0010;
                        }
                        id_bytes[0] |= size_marker << 2;

                        let block_id = BlockId::from_data(id_bytes);
                        assert!(block_id.supported_version());
                        assert_eq!(block_id.block_has_header(), header == 1);
                        assert_eq!(block_id.block_size(), 2usize.pow(12 + size_marker as u32));

                        if unused_bit_a == 1 || unused_bit_b == 1 {
                            assert!(!block_id.valid());
                        } else {
                            assert!(block_id.valid());
                        }
                    }
                }
            }
        }
    }

    /// Make sure that `BlockId` is sorted by size.
    #[test]
    fn block_id_sorting() {
        let mut bids = Vec::new();
        let mut id_bytes = [0; 32];

        thread_rng().fill(&mut id_bytes[..]);
        id_bytes[0] = 0b0001_1100;
        let bid_a = BlockId::from_data(id_bytes);
        bids.push(bid_a);
        thread_rng().fill(&mut id_bytes[..]);
        id_bytes[0] = 0b0011_1100;
        let bid_b = BlockId::from_data(id_bytes);
        bids.push(bid_b);
        thread_rng().fill(&mut id_bytes[..]);
        id_bytes[0] = 0b0001_1010;
        let bid_c = BlockId::from_data(id_bytes);
        bids.push(bid_c);
        thread_rng().fill(&mut id_bytes[..]);
        id_bytes[0] = 0b0011_1000;
        let bid_d = BlockId::from_data(id_bytes);
        bids.push(bid_d);

        // Before sort
        assert_eq!(bids[0], bid_a); // 0b0001_1100
        assert_eq!(bids[1], bid_b); // 0b0011_1100
        assert_eq!(bids[2], bid_c); // 0b0001_1010
        assert_eq!(bids[3], bid_d); // 0b0011_1000

        bids.sort_unstable();

        // After sort
        assert_eq!(bids[0], bid_c); // 0b0001_1010
        assert_eq!(bids[1], bid_a); // 0b0001_1100
        assert_eq!(bids[2], bid_d); // 0b0011_1000
        assert_eq!(bids[3], bid_b); // 0b0011_1100

        // Sizes
        assert_eq!(bid_c.block_size(), 262144); // 0b0001_1010
        assert_eq!(bid_a.block_size(), 524288); // 0b0001_1100
        assert_eq!(bid_d.block_size(), 67108864); // 0b0011_1000
        assert_eq!(bid_b.block_size(), 134217728); // 0b0011_1100
    }
}
