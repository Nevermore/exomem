@0x9b975e10356300d0;

struct BlockId {
	d1 @0: UInt64;
	d2 @1: UInt64;
	d3 @2: UInt64;
	d4 @3: UInt64;
}

using ShardId = UInt64;

struct UnionId {
	union {
		localId @0: UInt16; # Max 65536 local entries
		blockId @1: BlockId; # TODO: Add key
		shardId @2: ShardId; # TODO: Add key, and counter?
	}
}

struct Index {
	# TODO: Make this a union or do we want to support them at the same time?
	links @0: List(Link); # 256 links, index representing another byte of the hash
	data @1: List(Data); # Inline definitions

	struct Link {
		id @0: BlockId;
		# TODO: block key
	}

	struct Data {
		hashSuffix @0: Data;
		id @1: BlockId;
		# TODO: block key
		count @2: UInt32; # Number of references
	}
}

struct Block {
	nodes @0: List(Node);
	# The list can be e.g. one Directory followed by a bunch of inlined File/Directory nodes referenced via localId.
	transactions @1: List(Transaction);
	data @2: List(Data);
} # Super cool `Block`.

struct Node {
	union {
		vault @0: Vault;
		directory @1: Directory;
		file @2: File;
	}
	# TODO: POSIX user id, group id, mode, timestamps

	struct Vault {
		root @0: UnionId;
		index @1: UnionId;
	}

	struct Directory {
		entries @0: List(Entry);

		struct Entry {
			name @0: Text;
			id @1: UnionId;
		}
	}

	struct File {
		size @0: UInt64;
		id @1: List(UnionId); # List, because File data could be 7 blocks, and their block ids would be listed here.
	}
}

struct Transaction {
	actions @0: List(Action);
}

struct Action {
	union {
		create @0: Create;
		rename @1: Rename;
		delete @2: Delete;
		write @3: Write;
	}
}

enum NodeKind {
	vault @0;
	directory @1;
	file @2;
	# TODO: Symlink?
}

struct Create {
	path @0: Text; # The context where this node is.
	name @1: Text;
	kind @2: NodeKind;
}

struct Rename {
	path @0: Text; # The context where this node is.
	name @1: Text;
	newName @2: Text;
}

struct Delete {
	path @0: Text; # The context where this node is.
	name @1: Text;
}

struct Write {
	path @0: Text; # The context where this node is.
	name @1: Text;
	# TODO: Add actual payload
}