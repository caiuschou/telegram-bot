# Lance Vector Store Integration

This document describes the LanceDB integration for the memory crate.

## Prerequisites

### Installing Protocol Buffers Compiler (protoc)

LanceDB requires `protoc` to compile its native dependencies. Install it based on your platform:

#### Linux (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler
```

#### Linux (CentOS/RHEL/Fedora)
```bash
# CentOS/RHEL
sudo yum install protobuf-compiler

# Fedora
sudo dnf install protobuf-compiler
```

#### macOS
```bash
brew install protobuf
```

#### Windows (WSL)
```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler
```

#### Windows (Native)
1. Download from [protobuf releases](https://github.com/protocolbuffers/protobuf/releases)
2. Extract and add `bin` folder to PATH

#### Verify Installation
```bash
protoc --version
# Expected output: libprotoc 28.x or higher
```

#### Troubleshooting

If you see `error: failed to run custom build command for lance-encoding`:
- Ensure `protoc` is in your PATH
- Try setting the `PROTOC` environment variable:
  ```bash
  export PROTOC=/usr/bin/protoc
  ```

## Usage

### Enabling the Feature

Add to your `Cargo.toml`:

```toml
[dependencies]
memory = { version = "0.1", features = ["lance"] }
```

### Basic Example

```rust
use memory::{LanceVectorStore, MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create store
    let store = LanceVectorStore::new("./data/lancedb").await?;

    // Create entry
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: Some("conv456".to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(0.8),
    };
    let entry = MemoryEntry::new("Hello, world!".to_string(), metadata);

    // Add to store
    store.add(entry).await?;

    // Semantic search
    let query_embedding = vec![0.1; 1536]; // Your query embedding
    let results = store.semantic_search(&query_embedding, 10).await?;

    for result in results {
        println!("Found: {}", result.content);
    }

    Ok(())
}
```

### Custom Configuration

```rust
use memory::{LanceVectorStore, LanceConfig, DistanceType};

let config = LanceConfig {
    db_path: "./custom/path".to_string(),
    table_name: "my_memories".to_string(),
    embedding_dim: 1536,
    distance_type: DistanceType::Cosine,
};

let store = LanceVectorStore::with_config(config).await?;
```

### Creating Indexes

```rust
use memory::lance_store::LanceIndexType;

// Create index for faster vector search
store.create_index(LanceIndexType::Auto).await?;
```

## Migration from SQLite

```rust
use memory::migration::sqlite_to_lance;

// Migrate all data from SQLite to LanceDB
let count = sqlite_to_lance("./data/memory.db", "./data/lancedb").await?;
println!("Migrated {} entries", count);
```

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `db_path` | `String` | `"./data/lancedb"` | Path to LanceDB database |
| `table_name` | `String` | `"memories"` | Table name |
| `embedding_dim` | `usize` | `1536` | Vector dimension |
| `distance_type` | `DistanceType` | `Cosine` | Distance metric |

## Distance Types

- `Cosine` - Cosine similarity (recommended for normalized embeddings)
- `L2` - Euclidean distance
- `Dot` - Dot product similarity

## Index Types

- `Auto` - Automatically choose best index
- `IvfPq` - IVF with Product Quantization (balanced)
- `Hnsw` - Hierarchical Navigable Small World (fastest)
