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
use capnp::{
    message::{self, ReaderOptions, ReaderSegments, TypedBuilder},
    raw::get_struct_data_section,
};

use crate::vault_capnp::{block, block_id, index, node, union_id, NodeKind};

// TODO: Create UnionId? LocalId tracking is getting out of hand

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
        let mut id = BlockId { data: *hash.as_bytes() };
        id.set_header(size, has_header);
        id
    }

    /// Create a new `BlockId` from raw `data`.
    pub fn from_data(data: [u8; 32]) -> BlockId {
        BlockId { data }
    }

    /// Create a new `BlockId` from a capnp reader.
    pub fn from_reader(block_id_r: block_id::Reader) -> BlockId {
        let mut data = [0; 32];
        // TODO: Check length before hand to not panic?
        data.copy_from_slice(get_struct_data_section(block_id_r));
        BlockId { data }
    }

    /// Copy raw `BlockId` data to the specified capnp builder.
    pub fn to_builder(&self, mut block_id_b: block_id::Builder) {
        block_id_b.set_d1(u64::from_le_bytes(self.data[0..8].try_into().unwrap()));
        block_id_b.set_d2(u64::from_le_bytes(self.data[8..16].try_into().unwrap()));
        block_id_b.set_d3(u64::from_le_bytes(self.data[16..24].try_into().unwrap()));
        block_id_b.set_d4(u64::from_le_bytes(self.data[24..32].try_into().unwrap()));
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
    /// Check out [`BlockSize::from_marker`] for more information.
    pub fn block_size(&self) -> BlockSize {
        // The third to sixth least significant bits (4 bits) determine the size.
        let size_marker = (self.data[0] & 0b0011_1100u8) >> 2;
        // Ranges from 4 KiB to 128 MiB.
        BlockSize::from_marker(size_marker)
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

/// `BlockIdIndex` is a `u32` that refers to a specific entry.
///
/// This means roughly a limit of (2^32 * 128 MiB) == 512 PiB.
/// With the exact maximum file size being 2^59 - 280 * 2^27.
#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct BlockIdIndex(u32);

impl std::ops::Deref for BlockIdIndex {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BlockIdIndex {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::AddAssign for BlockIdIndex {
    fn add_assign(&mut self, other: Self) {
        self.0.add_assign(other.0)
    }
}

impl std::ops::Add for BlockIdIndex {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.add(other.0))
    }
}

impl std::ops::Sub for BlockIdIndex {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.sub(other.0))
    }
}

impl From<u32> for BlockIdIndex {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

const MAX_SIZE_MARKER: u8 = 0b1111; // 4 bits
const MAX_BLOCK_SIZE: u32 = 2u32.pow(27); // 128 MiB

// TODO: `BlockSize` and `FileSize` structs should guarantee sanity.
//       That is, when they are mutated, validity is checked.
//       No other mutable access is even possible, or at least via some scary escape hatch.

#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct BlockSize(u32);

impl BlockSize {
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
    /// [Advanced Format]: https://en.wikipedia.org/wiki/Advanced_Format
    pub const fn from_marker(size_marker: u8) -> BlockSize {
        // 4 bits max. Ranges from 4 KiB to 128 MiB.
        assert!(size_marker <= 0x0F);
        BlockSize(2u32.pow(12 + size_marker as u32))
    }

    pub const fn new(size: u32) -> BlockSize {
        assert!(BlockSize::valid(size));
        BlockSize(size)
    }

    pub const fn valid(size: u32) -> bool {
        size.count_ones() == 1 && size << 4 > 0 && size >> 12 > 0
    }

    /// Can panic!
    pub const fn as_offset(&self) -> BlockOffset {
        BlockOffset::new(self.0)
    }
}

impl From<u32> for BlockSize {
    fn from(value: u32) -> Self {
        BlockSize::new(value)
    }
}

impl std::ops::Deref for BlockSize {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Sub for BlockSize {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.sub(other.0))
    }
}

/// `BlockOffset` is a `u32` that refers to an offset inside a block.
///
/// This means a limit of 4 GiB, which is great because max block size is 128 MiB.
#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct BlockOffset(u32);

impl BlockOffset {
    pub const fn new(value: u32) -> BlockOffset {
        assert!(value < MAX_BLOCK_SIZE);
        BlockOffset(value)
    }
}

impl From<u32> for BlockOffset {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl std::ops::Add for BlockOffset {
    type Output = Self;

    // TODO: Check for validity?
    fn add(self, other: Self) -> Self::Output {
        Self(self.0.add(other.0))
    }
}

impl std::ops::Sub for BlockOffset {
    type Output = Self;

    // TODO: Check for sanity?
    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.sub(other.0))
    }
}

pub const MAX_FILE_SIZE: u64 = 2u64.pow(59) - 280 * 2u64.pow(27);

#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct FileSize(u64);

impl FileSize {
    pub const fn new(value: u64) -> FileSize {
        assert!(value <= MAX_FILE_SIZE);
        FileSize(value)
    }

    /// Converts the `FileSize` into a `BlockOffset`.
    ///
    /// This conversion is only safe if the `FileSize` value fits into `BlockOffset`.
    pub fn as_block_offset(&self) -> BlockOffset {
        (self.0 as u32).into()
    }

    /// Can panic!
    pub fn as_offset(&self) -> FileOffset {
        FileOffset::new(self.0)
    }
}

impl std::ops::Deref for FileSize {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BlockSize> for FileSize {
    fn from(value: BlockSize) -> FileSize {
        // u32 never exceeds MAX_FILE_SIZE
        // TODO: Add some sort of compile time assertion for this
        Self(value.0 as u64)
    }
}

impl From<u64> for FileSize {
    fn from(value: u64) -> FileSize {
        FileSize::new(value)
    }
}

// TODO: Ensure sanity for Add/AddAssign/Sub?
impl std::ops::Add for FileSize {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.add(other.0))
    }
}

impl std::ops::AddAssign for FileSize {
    fn add_assign(&mut self, other: Self) {
        self.0.add_assign(other.0)
    }
}

impl std::ops::Sub for FileSize {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.sub(other.0))
    }
}

/// `FileOffset` is a `u64` that refers to an offset inside a file.
///
/// This means a limit of 16384 PiB, which is great becauxe max supported file size is ~512 PiB.
/// With the exact maximum file size being 2^59 - 280 * 2^27.
#[derive(Eq, PartialEq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct FileOffset(u64);

impl FileOffset {
    pub const fn new(value: u64) -> FileOffset {
        assert!(value < MAX_FILE_SIZE);
        FileOffset(value)
    }

    pub fn as_size(&self) -> FileSize {
        FileSize(self.0)
    }

    /// Converts the `FileOffset` into a `BlockOffset`.
    ///
    /// This conversion is only safe if the `FileOffset` value fits into `BlockOffset`.
    pub fn as_block_offset(&self) -> BlockOffset {
        (self.0 as u32).into()
    }
}

impl std::ops::Deref for FileOffset {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<BlockOffset> for FileOffset {
    fn from(value: BlockOffset) -> Self {
        // u32 can't be larger than MAX_FILE_SIZE
        // TODO: Add compile time check for this
        FileOffset(value.0 as u64)
    }
}

impl From<BlockSize> for FileOffset {
    fn from(value: BlockSize) -> Self {
        // u32 can't be larger than MAX_FILE_SIZE
        // TODO: Add compile time check for this
        FileOffset(value.0 as u64)
    }
}

impl From<u64> for FileOffset {
    fn from(value: u64) -> Self {
        Self::new(value)
    }
}

impl std::ops::AddAssign for FileOffset {
    fn add_assign(&mut self, other: Self) {
        self.0.add_assign(other.0)
    }
}

impl std::ops::Add for FileOffset {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self(self.0.add(other.0))
    }
}

impl std::ops::Sub for FileOffset {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.sub(other.0))
    }
}

impl std::ops::Mul for FileOffset {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self(self.0.mul(rhs.0))
    }
}

impl std::ops::Div for FileOffset {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0.div(rhs.0))
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

/// The [`FileOffset`] immediately after the deterministic sequence of variable sized blocks.
///
/// This value is 6.75 GiB.
const REPEATING_BLOCKS_START_OFFSET: FileOffset = FileOffset::new(7_247_757_312);

impl InfoBlock {
    /// Returns the location of the offset inside a block.
    ///
    /// Every file starts with a deterministic sequence of variable sized blocks.
    /// This sequence is [`REPEATING_BLOCKS_START_OFFSET`] bytes long (6.75 GiB).
    /// After the initial sequence every block is maximum sized at 128 MiB.
    /// With the exception of the very last block which can be of any size that fits the data.
    fn translate_file_offset(offset: FileOffset) -> (BlockIdIndex, BlockOffset) {
        if offset < REPEATING_BLOCKS_START_OFFSET {
            // OPTIMIZE: More can be pre-calculated, fewer loops and branches.
            let mut block_start_offset = FileOffset::new(0);
            // There are 16 different block sizes.
            for size_marker in 0..16u32 {
                let block_size = BlockSize::from_marker(size_marker as u8);
                // Every block size gets at least 16 repetitions.
                for j in 0..16 {
                    // Blocks starting from 64 KiB require even more repetitions to keep alignment.
                    if size_marker > 3 && j == 15 {
                        // A 64 KiB block requires one extra repetition.
                        // Every subsequently larger block size adds an extra repetition.
                        for n in 0..(size_marker - 3) {
                            let old_block_offset = block_start_offset;
                            block_start_offset += block_size.into();
                            let mut block_index = size_marker * 16 + j;
                            if size_marker > 3 {
                                for ii in 4..size_marker {
                                    block_index += ii - 3;
                                }
                                block_index += n;
                            }
                            if offset < block_start_offset {
                                return (block_index.into(), (offset - old_block_offset).as_block_offset());
                            }
                        }
                    }
                    let old_block_offset = block_start_offset;
                    block_start_offset += block_size.into();
                    let mut block_index = size_marker * 16 + j;
                    if size_marker > 3 {
                        for ii in 4..size_marker {
                            block_index += ii - 3;
                        }
                        if j == 15 {
                            block_index += size_marker - 3;
                        }
                    }
                    if offset < block_start_offset {
                        return (block_index.into(), (offset - old_block_offset).as_block_offset());
                    }
                }
            }
            unreachable!();
        }

        let repeating_block_size = *BlockSize::from_marker(15) as u64;
        let remaining_bytes = offset - REPEATING_BLOCKS_START_OFFSET;
        let remaining_blocks = *remaining_bytes / repeating_block_size;
        let remaining_blocks_size = FileSize::from(remaining_blocks * repeating_block_size);
        let last_block_start_offset = REPEATING_BLOCKS_START_OFFSET + remaining_blocks_size.as_offset();
        let block_index = (334 + remaining_blocks as u32).into();

        (block_index, (offset - last_block_start_offset).as_block_offset())
    }

    pub fn new_vault(root_id: BlockId, index_id: BlockId) -> Block {
        let mut message_b = TypedBuilder::<block::Owned>::new_default(); // TODO: Look into allocation strategies
        let block_b = message_b.init_root();
        let nodes_b = block_b.init_nodes(1);
        let node_b = nodes_b.get(0);
        let mut vault_b = node_b.init_vault();
        let root_b = vault_b.reborrow().init_root();
        root_id.to_builder(root_b.init_block_id());
        let index_b = vault_b.init_index();
        index_id.to_builder(index_b.init_block_id());

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
                BlockId::from_reader(block_id_r)
            }
            union_id::Which::ShardId(_) => todo!(),
        };

        let index_r = vault_r.get_root().unwrap();
        let index_id = match index_r.which().unwrap() {
            union_id::Which::LocalId(_) => todo!(),
            union_id::Which::BlockId(block_id_r) => {
                let block_id_r = block_id_r.unwrap();
                BlockId::from_reader(block_id_r)
            }
            union_id::Which::ShardId(_) => todo!(),
        };

        (root_id, index_id)
    }

    pub fn update_root_id(&self, block_id: BlockId) -> Block {
        let block_r = self.block_reader();

        let mut message_b = TypedBuilder::<block::Owned>::new_default();
        message_b.set_root(block_r).unwrap();
        let block_b = message_b.get_root().unwrap();
        let nodes_b = block_b.get_nodes().unwrap();
        let node_b = nodes_b.get(0);

        let node::Vault(vault_b) = node_b.which().unwrap() else {
            panic!("Unexpected node");
        };
        let vault_b = vault_b.unwrap();
        let root_b = vault_b.init_root();
        block_id.to_builder(root_b.init_block_id());

        let segment = match message_b.borrow_inner().get_segments_for_output() {
            capnp::OutputSegments::SingleSegment(ss) => Bytes::copy_from_slice(ss[0]),
            capnp::OutputSegments::MultiSegment(_) => {
                panic!("got multiple output segments, but our reader doesn't want that")
            }
        };

        Block::from_data(segment)
    }

    /// Creates a new node of `kind` with `name`.
    ///
    /// Returns the new [`Block`] that contains the newly created inlined node, as well as the local id of that node.
    pub fn directory_create_local_node(&self, directory_node_idx: u32, name: &str, kind: NodeKind) -> (Block, u32) {
        let block_r = self.block_reader();
        let nodes_r = block_r.get_nodes().unwrap();
        let old_nodes_len = nodes_r.len();
        let node_r = nodes_r.get(directory_node_idx);

        let node::Directory(directory_r) = node_r.which().unwrap() else {
            panic!("Unexpected node");
        };
        let directory_r = directory_r.unwrap();

        let entries_r = directory_r.get_entries().unwrap();
        let old_entries_len = entries_r.len();

        let mut message_b = TypedBuilder::<block::Owned>::new_default();
        message_b.set_root(block_r).unwrap();
        let block_b = message_b.get_root().unwrap();

        // TODO: Don't init more nodes if we're not gonna inline
        let mut nodes_b = block_b.init_nodes(old_nodes_len + 1);
        for i in 0..old_nodes_len {
            let old_node = nodes_r.reborrow().get(i);
            nodes_b.set_with_caveats(i, old_node).unwrap();
        }

        let node_b = nodes_b.reborrow().get(directory_node_idx);

        let directory_b = match node_b.which().unwrap() {
            node::Directory(directory_b) => directory_b,
            node::Vault(_) => panic!("Unexpected vault node in the builder."),
            node::File(_) => panic!("Unexpected file node in the builder."),
        };
        let directory_b = directory_b.unwrap();

        let mut entries_b = directory_b.init_entries(old_entries_len + 1);
        for i in 0..old_entries_len {
            let old_entry_r = entries_r.reborrow().get(i);
            entries_b.set_with_caveats(i, old_entry_r).unwrap();
        }

        let mut entry_b = entries_b.reborrow().get(old_entries_len);
        entry_b.set_name(name);

        // TODO: Add ability to create this new node in a brand new block instead, and then reference it with blockId
        let mut id_b = entry_b.init_id();
        let next_local_id = old_nodes_len;
        id_b.set_local_id(next_local_id as u16); // TODO: Make sure we're not truncating

        let inline_node_b = nodes_b.get(next_local_id);
        match kind {
            NodeKind::Directory => {
                let directory_b = inline_node_b.init_directory();
                directory_b.init_entries(0);
            }
            NodeKind::File => {
                let mut file_b = inline_node_b.init_file();
                file_b.set_size(1234);
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

        (Block::from_data(segment), next_local_id)
    }

    pub fn directory_get_entry_block_id_and_node_index(
        &self,
        directory_node_idx: u32,
        entry_name: &str,
    ) -> Option<(Option<BlockId>, u32)> {
        let block_r = self.block_reader();
        let nodes_r = block_r.get_nodes().unwrap();
        let node_r = nodes_r.get(directory_node_idx);

        let node::Directory(directory_r) = node_r.which().unwrap() else {
            panic!("Unexpected node");
        };
        let directory_r = directory_r.unwrap();

        let entries_r = directory_r.get_entries().unwrap();
        for entry_r in entries_r.iter() {
            let name = entry_r.get_name().unwrap();
            if name == entry_name {
                assert!(entry_r.has_id());
                let id_r = entry_r.get_id().expect("failed to get id");
                match id_r.which().expect("failed to get readable id") {
                    union_id::Which::LocalId(local_id) => {
                        return Some((None, local_id as u32));
                    }
                    union_id::Which::BlockId(block_id_r) => {
                        let block_id_r = block_id_r.unwrap();
                        let block_id = BlockId::from_reader(block_id_r);
                        return Some((Some(block_id), 0));
                    }
                    union_id::Which::ShardId(_) => unimplemented!(),
                }
            }
        }
        None
    }

    pub fn directory_set_entry_block_id_and_node_index(
        &self,
        directory_node_idx: u32,
        entry_name: &str,
        block_id: Option<&BlockId>,
        node_index: u16,
    ) -> Option<Block> {
        let block_r = self.block_reader();
        let nodes_r = block_r.get_nodes().unwrap();
        let node_r = nodes_r.get(directory_node_idx);

        let node::Directory(directory_r) = node_r.which().unwrap() else {
            panic!("Unexpected node");
        };
        let directory_r = directory_r.unwrap();

        let entries_r = directory_r.get_entries().unwrap();
        for (entry_idx, entry_r) in entries_r.iter().enumerate() {
            let name = entry_r.get_name().unwrap();
            if name == entry_name {
                assert!(entry_r.has_id());
                let id_r = entry_r.get_id().expect("failed to get id");
                let id_matches = match id_r.which().expect("failed to get readable id") {
                    union_id::Which::LocalId(local_id) => block_id.is_none() && local_id == node_index,
                    union_id::Which::BlockId(block_id_r) => {
                        let block_id_r = block_id_r.unwrap();
                        let current_block_id = BlockId::from_reader(block_id_r);
                        block_id.is_some() && *block_id.unwrap() == current_block_id
                    }
                    union_id::Which::ShardId(_) => unimplemented!(),
                };
                if !id_matches {
                    let mut message_b = TypedBuilder::<block::Owned>::new_default();
                    message_b.set_root(block_r).unwrap();
                    let block_b = message_b.get_root().unwrap();

                    let nodes_b = block_b.get_nodes().unwrap();
                    let node_b = nodes_b.get(directory_node_idx);

                    let node::Directory(directory_b) = node_b.which().unwrap() else {
                        panic!("Unexpected node");
                    };
                    let directory_b = directory_b.unwrap();

                    let entries_b = directory_b.get_entries().unwrap();
                    let entry_b = entries_b.get(entry_idx as u32);
                    let mut id_b = entry_b.init_id();

                    if let Some(block_id) = block_id {
                        block_id.to_builder(id_b.init_block_id());
                    } else {
                        id_b.set_local_id(node_index);
                    }

                    let segment = match message_b.borrow_inner().get_segments_for_output() {
                        capnp::OutputSegments::SingleSegment(ss) => Bytes::copy_from_slice(ss[0]),
                        capnp::OutputSegments::MultiSegment(_) => {
                            panic!("got multiple output segments, but our reader doesn't want that")
                        }
                    };

                    return Some(Block::from_data(segment));
                }
            }
        }
        None
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

    #[test]
    fn block_size() {
        for size_marker in 0..MAX_SIZE_MARKER {
            let size = *BlockSize::from_marker(size_marker);
            assert_eq!(size, 2u32.pow(12 + size_marker as u32));
            assert!(BlockSize::valid(size));
            assert!(!BlockSize::valid(size - 1));
            assert!(!BlockSize::valid(size + 1));
        }
        assert!(BlockSize::valid(MAX_BLOCK_SIZE));
        assert!(!BlockSize::valid(MAX_BLOCK_SIZE - 1));
        assert!(!BlockSize::valid(MAX_BLOCK_SIZE + 1));
        assert!(!BlockSize::valid(2u32.pow(28)));
        assert!(!BlockSize::valid(2u32.pow(29) + 45));
        assert!(!BlockSize::valid(2u32.pow(30) + 123456));
    }

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
        assert_eq!(block_id.block_size(), 4096.into());

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
                        assert_eq!(block_id.block_size(), 2u32.pow(12 + size_marker as u32).into());

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
        assert_eq!(bid_c.block_size(), 2u32.pow(18).into()); // 0b0001_1010
        assert_eq!(bid_a.block_size(), 2u32.pow(19).into()); // 0b0001_1100
        assert_eq!(bid_d.block_size(), 2u32.pow(26).into()); // 0b0011_1000
        assert_eq!(bid_b.block_size(), 2u32.pow(27).into()); // 0b0011_1100
    }

    #[test]
    fn file_offset_translation() {
        // A very simple single block case
        let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset(4000.into());
        assert_eq!(block_id_idx, 0.into());
        assert_eq!(offset_in_block, 4000.into());

        // Simple two block case
        let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset(7000.into());
        assert_eq!(block_id_idx, 1.into());
        assert_eq!(offset_in_block, 2904.into());

        // Test every prefix of the size strategy
        let mut total = FileSize::new(0);
        let mut idx = BlockIdIndex::from(0);
        for size_marker in 0..16 {
            let size = BlockSize::from_marker(size_marker);
            for n in 0..16 {
                if n == 15 && size_marker > 3 {
                    for _ in 0..(size_marker - 3) {
                        total += size.into();
                        *idx += 1;
                        let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset(total.as_offset());
                        assert_eq!(block_id_idx, idx);
                        assert_eq!(offset_in_block, 0.into());
                        let (block_id_idx, offset_in_block) =
                            InfoBlock::translate_file_offset((total - 1.into()).as_offset());
                        assert_eq!(block_id_idx, idx - 1.into());
                        assert_eq!(offset_in_block, (*size - 1).into());
                        let (block_id_idx, offset_in_block) =
                            InfoBlock::translate_file_offset((total + 1.into()).as_offset());
                        assert_eq!(block_id_idx, idx);
                        assert_eq!(offset_in_block, 1.into());
                    }
                }
                total += size.into();
                *idx += 1;
                let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset(total.as_offset());
                assert_eq!(block_id_idx, idx);
                assert_eq!(offset_in_block, 0.into());
                let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset((total - 1.into()).as_offset());
                assert_eq!(block_id_idx, idx - 1.into());
                assert_eq!(offset_in_block, (*size - 1).into());
                let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset((total + 1.into()).as_offset());
                assert_eq!(block_id_idx, idx);
                assert_eq!(offset_in_block, 1.into());
            }
        }

        // We try 8138 extra 128 MiB blocks on top for a total size of 1 TiB
        let size = BlockSize::from_marker(15);
        for _ in 0..8138 {
            total += size.into();
            *idx += 1;
            let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset(total.as_offset());
            assert_eq!(block_id_idx, idx);
            assert_eq!(offset_in_block, 0.into());
            let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset((total - 1.into()).as_offset());
            assert_eq!(block_id_idx, idx - 1.into());
            assert_eq!(offset_in_block, (*size - 1).into());
            let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset((total + 1.into()).as_offset());
            assert_eq!(block_id_idx, idx);
            assert_eq!(offset_in_block, 1.into());
        }

        // 32 TiB with some ~118 MiB of change
        let (block_id_idx, offset_in_block) =
            InfoBlock::translate_file_offset(FileOffset::from(2u64.pow(45) + 123456789));
        assert_eq!(block_id_idx, BlockIdIndex::from(262424));
        assert_eq!(offset_in_block, 123456789.into());

        let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset(FileOffset::from(2u64.pow(50))); // 1 PiB
        assert_eq!(block_id_idx, BlockIdIndex::from(8388888));
        assert_eq!(offset_in_block, 0.into());

        let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset(FileOffset::from(2u64.pow(58))); // 256 PiB
        assert_eq!(block_id_idx, BlockIdIndex::from(2u32.pow(31) + 280));
        assert_eq!(offset_in_block, 0.into());

        let (block_id_idx, offset_in_block) = InfoBlock::translate_file_offset((MAX_FILE_SIZE - 1).into());
        assert_eq!(block_id_idx, BlockIdIndex::from(u32::MAX));
        assert_eq!(offset_in_block, (2u32.pow(27) - 1).into());
    }

    #[test]
    #[should_panic = "assertion failed: value < MAX_FILE_SIZE"]
    fn file_offset_translation_too_large_offset() {
        InfoBlock::translate_file_offset(MAX_FILE_SIZE.into());
    }
}
