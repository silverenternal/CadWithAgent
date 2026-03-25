# CadAgent

CAD geometry processing toolchain in Rust, powered by Tool-Augmented Context Injection paradigm for VLM-driven geometric reasoning.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Test Status](https://img.shields.io/badge/tests-248%20passed-brightgreen)]()
[![Coverage Status](https://img.shields.io/badge/coverage-80%2B%25-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

## Features

- **🔧 Tool-based Geometry Algorithms**: Measurement, transformation, topology analysis wrapped as tokitai tools
- **🧠 Tool-Augmented Context Injection**: No VLM modification needed, external geometry algorithms construct precise prompts
- **📐 Complete Geometry Toolchain**: Primitive extraction → Relation reasoning → Constraint verification → Prompt generation
- **🤖 VLM Integration**: Support for ZazaZ, OpenAI and compatible APIs, automatic chain-of-thought generation
- **⚡ High Performance**: R-tree spatial indexing, 10x+ performance improvement for 1000+ primitive scenarios
- **📊 SVG/DXF Processing**: Complete file parsing and export capabilities
- **🔒 Configuration Validation**: Complete config file schema validation with `validate-config` command
- **🎯 Geometry-Only Mode**: Pure geometry processing without VLM API key requirement

## Project Philosophy

### Why CadAgent?

Using VLMs directly for CAD drawings has these problems:

1. **Unreliable Geometric Computation**: VLMs are poor at precise length, area, and angle calculations
2. **Missing Constraint Relations**: Parallel, perpendicular, and connection relationships are often misjudged
3. **Uninterpretable Results**: Cannot trace reasoning process, failing to meet industrial credibility requirements

### CadAgent's Solution

**Tool-Augmented Context Injection Paradigm** — Let professionals do professional work:

```
┌─────────────────────────────────────────────────────────────┐
│  Input: CAD Drawing (SVG/DXF)                               │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Deterministic Geometry Algorithm Layer (CadAgent)          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Primitive   │→ │  Relation    │→ │  Constraint   │      │
│  │  Extraction  │  │  Reasoning   │  │  Verification │      │
│  │ • Line/Circle│  │ • Parallel/  │  │ • Conflict    │      │
│  │ • Coordinate │  │   Perpendic. │  │ • Redundancy  │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                            │                                 │
│                            ▼                                 │
│                   ┌──────────────┐                          │
│                   │Structured Prompt│ ← Inject precise geo.  │
│                   └──────────────┘                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  VLM Reasoning Layer (Qwen/GPT/etc.)                        │
│  • Understand task intent                                    │
│  • Reason based on precise geometric data                    │
│  • Generate interpretable chain-of-thought                   │
└─────────────────────────────────────────────────────────────┘
```

### Core Design Principles

1. **Deterministic algorithms for geometric computation** — 100% accurate, verifiable
2. **VLM focuses on high-level reasoning** — Intent understanding, task planning, natural language generation
3. **No model modification, plug-and-play** — Achieved through prompt engineering
4. **Traceable reasoning chain** — Every geometric step has algorithmic basis

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/tokitai/cadagent.git
cd cadagent

# Build
cargo build --release

# Run tests
cargo test
```

### Configure Environment Variables

```bash
# Copy environment variable template
cp .env.example .env

# Edit .env to set API Key
export PROVIDER_ZAZAZ_API_KEY="your-api-key"
```

### Basic Example

```rust
use cadagent::prelude::*;

fn main() -> anyhow::Result<()> {
    // Create analysis pipeline
    let pipeline = AnalysisPipeline::with_defaults()?;

    // Inject context from SVG string
    let svg = r#"<svg width="100" height="100">
        <line x1="0" y1="0" x2="100" y2="0" />
        <line x1="100" y1="0" x2="100" y2="100" />
        <line x1="100" y1="100" x2="0" y2="100" />
        <line x1="0" y1="100" x2="0" y2="0" />
    </svg>"#;

    let result = pipeline.inject_from_svg_string(svg, "Analyze this floor plan")?;

    println!("Primitives: {}", result.primitives.len());
    println!("Relations: {}", result.relations.len());
    println!("Prompt length: {} chars", result.prompt.full_prompt.len());

    Ok(())
}
```

### Complete Usage (with VLM Inference)

```rust
use cadagent::prelude::*;

fn main() -> anyhow::Result<()> {
    let pipeline = AnalysisPipeline::with_defaults()?;

    // Execute complete geometric analysis + VLM inference
    let result = pipeline.inject_from_svg_string_with_vlm(
        svg_content,
        "Please analyze this floor plan, identify all rooms and calculate areas"
    )?;

    // Access VLM response
    if let Some(vlm) = &result.vlm_response {
        println!("Model: {}", vlm.model);
        println!("Response: {}", vlm.content);
        println!("Token usage: {:?}", vlm.usage);
    }

    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      AI Model (VLM)                          │
│              Qwen2.5-VL / InternVL2 / etc.                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ tool_calls
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Tokitai Protocol                          │
│              Compile-time Tool Definitions                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ dispatch
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  Analysis Pipeline                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Extractor   │→ │  Reasoner    │→ │  Verifier    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│         │                  │                  │              │
│         └──────────────────┴──────────────────┘              │
│                            │                                 │
│                            ▼                                 │
│                   ┌──────────────┐                          │
│                   │ Prompt Builder│                         │
│                   └──────────────┘                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ execute
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Geometry Engine                          │
│  Primitives | Boolean Ops | R-tree Index | Room Detection   │
└─────────────────────────────────────────────────────────────┘
```

## Core Modules

### 1. Geometry Primitives and Tools

```rust
use cadagent::prelude::*;

// Create primitives
let room = Polygon::from_coords(vec![
    [0.0, 0.0], [500.0, 0.0], [500.0, 400.0], [0.0, 400.0],
]);

// Measurement tools
let measurer = GeometryMeasurer;
let area = measurer.measure_area(vec![
    [0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0],
]);

// Transformation tools
let transform = GeometryTransform;
let translated = transform.translate(vec![Primitive::Polygon(room)], 50.0, 50.0);
```

### 2. Topology Analysis

```rust
use cadagent::topology::room_detect::RoomDetector;

let detector = RoomDetector;
let room_count = detector.count_rooms(primitives);
let doors = detector.detect_doors(&primitives);
let windows = detector.detect_windows(&primitives);
```

### 3. Analysis Pipeline (Recommended)

```rust
use cadagent::prelude::*;

// Create pipeline
let pipeline = AnalysisPipeline::with_defaults()?;

// Execute complete analysis
let result = pipeline.inject_from_svg_string(svg, "Analyze this shape")?;

// Access results
println!("Primitives: {} items", result.primitives.len());
println!("Relations: {} items", result.relations.len());
println!("Prompt: {} chars", result.prompt.full_prompt.len());
```

### 4. Custom VLM Provider

```rust
use cadagent::bridge::vlm_client::VlmConfig;
use cadagent::prelude::*;

// Configure ZazaZ API
let vlm_config = VlmConfig::new(
    "https://zazaz.top/v1",
    "sk-your-api-key",
    "./Qwen3.5-27B-FP8",
);

// Or use OpenAI
// let vlm_config = VlmConfig::default_openai()?;

let pipeline = AnalysisPipeline::with_vlm_config(vlm_config)?;
let result = pipeline.inject_from_svg_string_with_vlm(svg, "Analyze")?;
```

### 5. Geometry-Only Mode (No VLM API Key Required)

```rust
use cadagent::prelude::*;

// Create geometry-only pipeline (no VLM API Key needed)
let pipeline = AnalysisPipeline::geometry_only()?;

// Execute geometry analysis without VLM
let result = pipeline.inject_from_svg_string(svg, "Analyze this shape")?;

// Access geometry results (no VLM response)
println!("Primitives: {}", result.primitives.len());
println!("Relations: {}", result.relations.len());
assert!(result.vlm_response.is_none());
```

## Tool List

### Measurement Tools

| Tool Name | Description |
|-----------|-------------|
| `measure_length` | Measure line segment length |
| `measure_area` | Calculate polygon area |
| `measure_angle` | Measure angle |
| `measure_perimeter` | Calculate perimeter |
| `check_parallel` | Check parallelism |
| `check_perpendicular` | Check perpendicularity |

### Transformation Tools

| Tool Name | Description |
|-----------|-------------|
| `translate` | Translation |
| `rotate` | Rotation |
| `scale` | Scaling |
| `mirror` | Mirroring |

### Topology Analysis Tools

| Tool Name | Description |
|-----------|-------------|
| `detect_rooms` | Detect rooms |
| `count_rooms` | Count rooms |
| `detect_doors` | Detect doors |
| `detect_windows` | Detect windows |
| `find_closed_loop` | Find closed loops |

### Geo-CoT Tools

| Tool Name | Description |
|-----------|-------------|
| `generate_geo_cot` | Generate geometric chain-of-thought |
| `generate_qa` | Generate question-answer pairs |

### Analysis Tools (Tool-Augmented Context Injection)

| Tool Name | Description |
|-----------|-------------|
| `cad_extract_primitives` | Extract geometric primitives from SVG |
| `cad_find_geometric_relations` | Find geometric relations |
| `cad_verify_constraints` | Verify constraint validity |
| `cad_build_analysis_prompt` | Build analysis prompt |
| `cad_context_inject` | Execute complete context injection flow |

## Command Line Usage

```bash
# Parse SVG file
cargo run --bin cadagent-cli -- parse-svg --input floor_plan.svg --output primitives.json

# Measure
cargo run --bin cadagent-cli -- measure --kind area --data '{"vertices": [[0,0],[100,0],[100,100],[0,100]]}'

# Detect rooms
cargo run --bin cadagent-cli -- detect-rooms --input primitives.json

# Export DXF
cargo run --bin cadagent-cli -- export-dxf --input primitives.json --output output.dxf

# Generate Geo-CoT data
cargo run --bin cadagent-cli -- generate-cot --input primitives.json --task "Calculate areas of all rooms"

# Consistency check
cargo run --bin cadagent-cli -- check-consistency --input primitives.json

# Validate configuration
cargo run --bin cadagent-cli -- validate-config --config config/default.json

# List all tools
cargo run --bin cadagent-cli -- list-tools
```

## Examples

```bash
# Basic usage example
cargo run --example basic_usage

# Complete pipeline example
cargo run --example pipeline

# Geo-CoT generation example
cargo run --example cot_generation

# Context injection example (without VLM inference)
cargo run --example context_injection

# Real VLM inference example (calls API)
cargo run --example vlm_inference
```

## Testing

```bash
# Run all tests
cargo test

# Run geometry module tests
cargo test --test geometry_tests

# Run geometric reasoning tests
cargo test --test cad_reasoning_tests

# Generate coverage report
cargo tarpaulin --output-dir coverage --out html
```

### Test Coverage

- **Geometry Module**: 31 unit tests
- **Geometric Reasoning**: 17 unit tests
- **Total**: All 248 tests passing
- **Core Module Coverage**: 80%+

## Performance Metrics

| Operation | Performance (1000 primitives) |
|-----------|-------------------------------|
| `parse_svg` | < 10ms |
| `detect_relations` (R-tree) | < 100ms |
| `build_prompt` | < 50ms |

## Dependencies

- **tokitai**: AI tool integration protocol
- **reqwest**: HTTP client (VLM API calls)
- **tokio**: Async runtime
- **serde/serde_json**: Serialization
- **roxmltree**: Reliable XML parsing
- **rstar**: R-tree spatial indexing
- **geo/nalgebra**: Geometric computation
- **clap**: CLI parsing
- **tracing**: Logging

## Project Structure

```
cadagent/
├── src/
│   ├── analysis/          # Unified analysis pipeline (recommended)
│   ├── cad_extractor/     # CAD primitive extraction
│   ├── cad_reasoning/     # Geometric relation reasoning
│   ├── cad_verifier/      # Constraint verification
│   ├── prompt_builder/    # Prompt construction
│   ├── geometry/          # Geometry primitives and tools
│   ├── topology/          # Topology analysis
│   ├── cot/               # Geo-CoT generation
│   ├── parser/            # File parsing (SVG/DXF)
│   ├── export/            # File export (JSON/DXF)
│   ├── bridge/            # VLM bridge
│   ├── tools/             # Tool registry
│   ├── llm_reasoning/     # LLM reasoning
│   └── metrics/           # Evaluation metrics
├── examples/              # Usage examples
├── tests/                 # Integration tests
├── benches/               # Performance benchmarks
└── config/                # Configuration files
```

## Configuration

Configuration files are located in the `config/` directory:

- `config/default.json`: Default configuration
- `config/templates.json`: CoT template configuration

### Configuration Validation

Validate configuration files using the `validate-config` command:

```bash
cargo run --bin cadagent-cli -- validate-config
```

Example output:
```
✓ Configuration file loaded successfully
✓ Schema validation passed
✓ Model name validation passed
✓ API endpoint validation passed
✓ Template syntax validation passed
...
All 27 checks passed! Configuration is valid.
```

### Environment Variables

Environment variable configuration see `.env.example`:

```bash
# ZazaZ API configuration
export PROVIDER_ZAZAZ_API_KEY="your-api-key"
export PROVIDER_ZAZAZ_API_URL="https://zazaz.top/v1"
export PROVIDER_ZAZAZ_MODEL="./Qwen3.5-27B-FP8"

# OpenAI API configuration (optional)
export OPENAI_API_KEY="sk-..."
export OPENAI_MODEL="gpt-4o"
```

### Geometry-Only Mode

If you only need geometry processing (without VLM inference), no API Key is required. Simply use:

```rust
let pipeline = AnalysisPipeline::geometry_only()?;
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md)

### Quick Start

```bash
# Clone the project
git clone https://github.com/tokitai/cadagent.git

# Build
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy -- -D warnings
```

## License

MIT License

## Related Links

- [tokitai Documentation](https://docs.rs/tokitai)
- [Contributing Guide](CONTRIBUTING.md)
