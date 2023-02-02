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

/// `ShardId` is a globally unique 64 bit [`Shard`] identifier.
///
/// The 64 bits were chosen to match processor word size and provide a good enough supply of ids.
///
/// 64 bits provides 8 billion people each 2 billion ids.
/// With proper id recycling in place this should be enough.
pub struct ShardId {
    /// Globally unique 64 bit identifier.
    id: u64,
}

impl ShardId {
    /// Create a `ShardId` from its inner `u64`.
    pub fn new(id: u64) -> ShardId {
        ShardId { id }
    }

    /// Returns the inner `u64` of the `ShardId`.
    pub fn id(&self) -> u64 {
        self.id
    }
}
