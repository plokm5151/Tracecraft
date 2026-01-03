# Mr. Hedgehog 🦔

A static analysis tool for multi-crate workspaces, enabling comprehensive call graph, AST, and dependency tree analysis for large-scale projects.

## ✨ Features

- **Multi-language support**: Rust and Python via SCIP indexing
- **Call graph generation**: Visualize function dependencies
- **AST analysis**: Parse and analyze source code structure
- **Dependency tracing**: Forward and reverse path analysis
- **IPC Backend**: Long-running daemon mode with JSON-TCP protocol
- **Qt GUI**: Modern dark-themed desktop application for interactive analysis

## 📦 Installation

### Prerequisites

- Rust toolchain (1.70+)
- Qt 6 (`brew install qt@6` on macOS)
- rust-analyzer (for Rust SCIP analysis)
- scip-python (optional, for Python support)

```bash
# Install rust-analyzer
brew install rust-analyzer

# Install scip-python (optional)
npm install -g @sourcegraph/scip-python
```

### Quick Packaging (macOS)

Use the provided script to build and deploy the standalone app to your Desktop:

```bash
./scripts/deploy_to_desktop.sh
```

### Building from Source

```bash
# Build Rust backend
cargo build --release

# Build Qt frontend
mkdir -p frontend/build && cd frontend/build
cmake .. -DCMAKE_PREFIX_PATH=/usr/local/opt/qt@6
make -j4
```

## 🚀 Usage

### Command Line

```bash
# Analyze Rust workspace
mr_hedgehog --workspace ./Cargo.toml --output graph.dot

# Start in Daemon Mode (for GUI integration)
mr_hedgehog --daemon --port 4545

# Analyze Python project
mr_hedgehog --engine scip --lang python --workspace ./project --output graph.dot
```

### GUI Application

Simply double-click **MrHedgehog** on your Desktop after running the deployment script.

## 📋 CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--workspace` | Path to Cargo.toml or project folder | - |
| `--output` | Output file path | - |
| `--engine` | `syn` or `scip` | `syn` |
| `--lang` | `rust` or `python` | `rust` |
| `--daemon` | Start as persistent TCP server | `false` |
| `--port` | TCP port for daemon mode | `4545` |
| `--reverse` | Reverse trace target | - |
| `--expand-paths` | Expand all paths from main | `false` |
| `--debug` | Debug output | `false` |

## 🏗️ Architecture

```
src/
├── domain/           # Core domain logic
├── infrastructure/   # External tool runners (SCIP, Cargo)
├── api/              # IPC Server & DTOs (JSON-TCP)
└── main.rs           # CLI Entrypoint
frontend/             # C++ Qt GUI
├── src/              # MainWindow & GraphView logic
└── CMakeLists.txt
scripts/              # Deployment & utility scripts
```

## ⚡ Performance

- **Parallel processing**: Rayon-based concurrent SCIP ingestion
- **Incremental caching**: Skip re-indexing unchanged files
- **Memory-mapped I/O**: Efficient large file loading

## 📄 License

MIT OR Apache-2.0

## 👤 Author

Frank Chen <plokm85222131@gmail.com>
