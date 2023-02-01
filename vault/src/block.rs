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

/// `BlockId` is a globally unique 256 bit identifier for [`Block`].
///
/// It also contains a header with some information about the block.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct BlockId {
    /// The raw bytes that make up this `BlockId`.
    ///
    /// The first byte is a header byte, the other 31 are a hash of the block.
    /// Currently only the 6 least significant bits of the header byte are actually used.
    data: [u8; 32],
}

impl BlockId {
    /// Create a new `BlockId` from raw `bytes`.
    pub fn new(bytes: [u8; 32]) -> BlockId {
        BlockId { data: bytes }
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
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0x{:02X}{:02X}{:02X}{:02X}",
            self.data[0], self.data[1], self.data[2], self.data[3]
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

/// Immutable unencrypted block.
pub struct Block {
    kind: BlockKind,
    data: Vec<u8>,
}

impl Block {
    /// Create a new `Block`.
    pub fn new(kind: BlockKind, size: usize) -> Block {
        Block {
            kind,
            data: vec![0; size],
        }
    }

    /// Returns a reference to the block's data.
    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Returns the block's [`BlockKind`].
    pub fn kind(&self) -> BlockKind {
        self.kind
    }

    /// Returns the block's size in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
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
        let block_id = BlockId::new(id_bytes);
        assert!(block_id.supported_version());
        assert!(!block_id.block_has_header());
        assert_eq!(block_id.block_size(), 4096);

        id_bytes[0] = 0b0000_0001;
        let block_id = BlockId::new(id_bytes);
        assert!(!block_id.supported_version());

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

                        let block_id = BlockId::new(id_bytes);
                        assert!(block_id.supported_version());
                        assert_eq!(block_id.block_has_header(), header == 1);
                        assert_eq!(block_id.block_size(), 2usize.pow(12 + size_marker as u32));
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
        let bid_a = BlockId::new(id_bytes);
        bids.push(bid_a);
        thread_rng().fill(&mut id_bytes[..]);
        id_bytes[0] = 0b0011_1100;
        let bid_b = BlockId::new(id_bytes);
        bids.push(bid_b);
        thread_rng().fill(&mut id_bytes[..]);
        id_bytes[0] = 0b0001_1010;
        let bid_c = BlockId::new(id_bytes);
        bids.push(bid_c);
        thread_rng().fill(&mut id_bytes[..]);
        id_bytes[0] = 0b0011_1000;
        let bid_d = BlockId::new(id_bytes);
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
