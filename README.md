# Mr. Hedgehog 🦔

A static analysis tool for multi-crate workspaces, enabling comprehensive call graph, AST, and dependency tree analysis for large-scale projects, optimized for local and privacy-sensitive environments.

## ✨ Features

- **Multi-language support**: Rust and Python via SCIP indexing
- **Call graph generation**: Visualize function dependencies
- **AST analysis**: Parse and analyze source code structure
- **Dependency tracing**: Forward and reverse path analysis
- **Qt GUI**: Modern dark-themed desktop application

## 📦 Installation

### Prerequisites

- Rust toolchain (1.70+)
- Qt 6 (for GUI, install via `brew install qt@6` on macOS)
- rust-analyzer (for Rust SCIP analysis)
- scip-python (for Python SCIP analysis, optional)

```bash
# Install rust-analyzer
brew install rust-analyzer
# or via rustup
rustup component add rust-analyzer

# Install scip-python (optional, for Python support)
npm install -g @sourcegraph/scip-python
```

### Building from Source

```bash
# Clone the repository
git clone https://github.com/plokm5151/Mr-Hedgehog
cd Mr-Hedgehog

# Build Rust backend
cargo build --release

# Build Qt frontend
cd frontend
mkdir -p build && cd build
cmake .. -DCMAKE_PREFIX_PATH=/usr/local/opt/qt@6
make -j4

# The app bundle is at dist/MrHedgehog.app
```

## 🚀 Usage

### Command Line

```bash
# Analyze a Rust workspace (syn engine - default)
mr_hedgehog --workspace ./Cargo.toml --output callgraph.dot

# Analyze with SCIP engine (more precise)
mr_hedgehog --workspace ./Cargo.toml --output callgraph.dot --engine scip

# Analyze a Python project
mr_hedgehog --workspace ./project --output callgraph.dot --engine scip --lang python

# Reverse trace: find all callers of a function
mr_hedgehog --workspace ./Cargo.toml --output trace.txt --reverse "MyType::my_func"

# Expand all paths from main
mr_hedgehog --workspace ./Cargo.toml --output paths.txt --expand-paths
```

### GUI Application

```bash
# Launch the Qt GUI
open dist/MrHedgehog.app
# or
./dist/MrHedgehog.app/Contents/MacOS/MrHedgehogUI
```

1. Click **Browse** to select a project folder
2. Click **Run Analysis** to generate the call graph
3. Use mouse wheel to zoom, drag to pan

## 📋 CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--workspace` | Path to Cargo.toml or project folder | - |
| `--input` | Single source file(s) | - |
| `--folder` | Folder(s) to scan recursively | - |
| `--output` | Output file path | required |
| `--format` | Output format | `dot` |
| `--engine` | Analysis engine: `syn` or `scip` | `syn` |
| `--lang` | Language: `rust` or `python` | `rust` |
| `--reverse` | Reverse trace target function | - |
| `--expand-paths` | Expand all paths from main | `false` |
| `--branch-summary` | Summarize branch events | `false` |
| `--store` | Storage backend: `mem` or `disk` | `mem` |
| `--debug` | Enable debug output | `false` |

## 🏗️ Architecture

```
src/
├── domain/           # Core domain logic
│   ├── language.rs   # Language enum (Rust, Python)
│   ├── callgraph.rs  # Call graph data structures
│   ├── scip_ingest.rs # SCIP index parser (parallel)
│   └── trace.rs      # Path tracing algorithms
├── infrastructure/   # External integrations
│   ├── scip_runner.rs # Multi-language SCIP generation
│   ├── scip_cache.rs  # Incremental caching
│   └── project_loader.rs
└── ports/            # Interface adapters

frontend/             # C++ Qt GUI
├── src/
│   ├── mainwindow.cpp
│   └── graphview.cpp
└── CMakeLists.txt
```

## ⚡ Performance

- **Parallel processing**: rayon-based concurrent SCIP ingestion
- **Incremental caching**: Skip re-indexing unchanged files
- **Memory-mapped I/O**: Efficient large file loading via mmap

Run benchmarks:
```bash
cargo bench
```

## 📄 License

MIT OR Apache-2.0

## 👤 Author

Frank Chen <plokm85222131@gmail.com>
