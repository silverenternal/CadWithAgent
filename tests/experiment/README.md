# CadAgent Experiment Framework

## Overview

This experiment framework provides comprehensive validation for the CadAgent system, designed to meet the rigorous standards of top-tier computer science conferences (SIGGRAPH, CHI, UIST, CVPR, ICCV).

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Experiment Design](#experiment-design)
- [Running Experiments](#running-experiments)
- [Results & Visualization](#results--visualization)
- [Reproducibility](#reproducibility)
- [Paper Integration](#paper-integration)

---

## Quick Start

```bash
# Run all experiments
cargo test --test experiment_test -- --nocapture

# Run specific experiment
cargo test --test experiment_test exp1_accuracy_validation -- --nocapture

# Generate visualizations
python tests/experiment/scripts/visualize_results.py

# Generate LaTeX tables for paper
cargo run --bin generate_tables
```

---

## Experiment Design

### Research Questions

Our experiments are designed to answer the following research questions:

| RQ | Question | Experiment |
|----|----------|------------|
| RQ1 | Does CadAgent achieve 100% accuracy in deterministic geometric computations? | Exp-1 |
| RQ2 | What is the performance improvement from R-tree spatial indexing? | Exp-2 |
| RQ3 | How much does tool-augmented context injection improve VLM reasoning? | Exp-3 |
| RQ4 | What is the contribution of each module to overall performance? | Exp-4 |
| RQ5 | Is CadAgent effective in real-world application scenarios? | Exp-5 |
| RQ6 | How does CadAgent compare to existing methods? | Exp-6 |

### Experimental Structure

```
tests/experiment/
├── exp1_accuracy.rs      # Geometric computation accuracy
├── exp2_performance.rs   # Performance benchmarks
├── exp3_vlm_reasoning.rs # VLM reasoning quality
├── exp4_ablation.rs      # Module contribution analysis
├── exp5_case_studies.rs  # Real-world case studies
├── exp6_comparison.rs    # Comparative evaluation
├── metrics.rs            # Evaluation metrics
├── runner.rs             # Experiment runner
├── utils.rs              # Utilities & statistics
├── fixtures/             # Test fixtures
├── data/                 # Raw experimental data
├── results/              # Generated results
├── scripts/              # Visualization scripts
└── cubicasa5k.rs         # CubiCasa5k dataset integration
```

### Data Integration

#### CubiCasa5k Dataset

The [CubiCasa5k](https://github.com/CubiCasa5k) dataset is integrated for architectural floor plan analysis:

```bash
# Data location
data/CubiCasa5k/data/cubicasa5k/

# Test house SVG
data/CubiCasa5k/data/cubicasa5k/test_house/model.svg
```

#### SVG to PNG Conversion

Before using the dataset, convert SVG files to PNG format:

```bash
# Convert single directory
cargo run --release --manifest-path data/svg_to_png_converter/Cargo.toml \
    data/CubiCasa5k/data/cubicasa5k/test_house \
    data/CubiCasa5k/data/cubicasa5k_output/test_house

# Or use the batch conversion script
cd data/CubiCasa5k
bash convert_all.sh
```

#### Usage in tests

```bash
# Run CubiCasa5k tests
cargo test --test experiment_test cubicasa5k -- --nocapture
```

The `cubicasa5k` module provides:
- SVG parsing and element extraction
- PNG image loading and validation
- Room layout analysis
- Wall, door, window detection
- Topology validation

```rust
use cadagent::experiment::cubicasa5k::*;

// Load test house
let house = HouseData::load_test_house().unwrap();

// Read SVG content
let svg_content = house.read_svg_content().unwrap();

// Read PNG data (after conversion)
if house.has_png() {
    let png_data = house.read_png_data().unwrap();
    println!("PNG size: {} bytes", png_data.len());
}

// Parse SVG elements
let elements = parse_svg_elements(&svg_content);

// Analyze room layout
let analysis = analyze_room_layout(&elements);
println!("Rooms: {}", analysis.rooms_count);
```

#### Using Your Own Data

Place your datasets in the `data/` directory:

```
data/
├── CubiCasa5k/           # Architectural floor plans
├── your_dataset/         # Your custom dataset
│   ├── train/
│   ├── val/
│   └── test/
```


## Running Experiments

### Individual Experiments

#### Experiment 1: Geometric Accuracy Validation

```bash
cargo test --test experiment_test exp1_accuracy_validation -- --nocapture
```

**Purpose**: Validate 100% accuracy of deterministic geometric algorithms.

**Metrics**:
- Absolute error: |measured - expected|
- Relative error: |measured - expected| / expected
- Accuracy: correct / total × 100%

**Expected Results**:
- Measurement accuracy: relative error < 1e-10
- Relation detection: 100% accuracy
- Transformation accuracy: relative error < 1e-10

#### Experiment 2: Performance Benchmarks

```bash
cargo test --test experiment_test exp2_performance_benchmark -- --nocapture
```

**Purpose**: Validate R-tree spatial indexing performance advantages.

**Metrics**:
- Throughput (ops/sec)
- Latency: p50, p95, p99
- Speedup vs baseline
- Memory usage (MB)

**Scalability Testing**: Data sizes [100, 500, 1000, 5000, 10000]

#### Experiment 3: VLM Reasoning Quality

```bash
cargo test --test experiment_test exp3_vlm_reasoning_quality -- --nocapture
```

**Purpose**: Validate tool-augmented context injection effectiveness.

**Metrics**:
- Answer accuracy
- Reasoning step F1 score
- Hallucination rate
- Response time

**Comparison**:
- Baseline: Pure VLM reasoning
- Enhanced: Tool-augmented + context injection

#### Experiment 4: Ablation Study

```bash
cargo test --test experiment_test exp4_ablation_study -- --nocapture
```

**Purpose**: Analyze module contribution to overall performance.

**Configurations**:
- Full system (all modules enabled)
- Without R-tree spatial index
- Without tool augmentation
- Without context injection
- Without geometry verification

#### Experiment 5: Case Studies

```bash
cargo test --test experiment_test exp5_case_studies -- --nocapture
```

**Purpose**: Validate effectiveness in real-world applications.

**Case Types**:
- Mechanical part analysis (DXF)
- Architectural plan analysis (SVG)
- Circuit diagram analysis
- User interaction cases

#### Experiment 6: Comparative Evaluation

```bash
cargo test --test experiment_test exp6_comparison_study -- --nocapture
```

**Purpose**: Comprehensive comparison with existing methods.

**Comparison Targets**:
- Commercial: AutoCAD, SolidWorks
- Open Source: LibreCAD, FreeCAD
- Traditional: Rule-based methods
- AI Tools: Other AI-assisted CAD systems

---

## Results & Visualization

### Output Structure

```
tests/experiment/results/
├── exp1_accuracy_result.json
├── exp2_performance_result.json
├── exp3_vlm_reasoning_result.json
├── exp4_ablation_result.json
├── exp5_case_studies_result.json
├── exp6_comparison_result.json
├── accuracy_chart.png
├── performance_chart.png
├── ablation_chart.png
├── comparison_chart.png
├── summary_report.md
└── paper_tables.tex
```

### Generating Visualizations

```bash
# Python visualization (requires matplotlib)
python tests/experiment/scripts/visualize_results.py

# LaTeX table generation
cargo run --bin generate_tables --output paper/tables/
```

### Chart Types

| Chart | Purpose | Experiment |
|-------|---------|------------|
| Bar chart | Accuracy comparison | Exp-1, Exp-4 |
| Line chart (log-log) | Scalability analysis | Exp-2 |
| Radar chart | Multi-dimension comparison | Exp-6 |
| Box plot | Distribution analysis | Exp-2, Exp-3 |

---

## Reproducibility

### Environment Capture

```bash
# Capture environment
cargo run --bin capture_environment > environment.json

# Reproduce experiment run
cargo run --bin reproduce_experiment --config environment.json
```

### Random Seed Management

All experiments use deterministic seeds for reproducibility:

```rust
// In experiment configuration
let config = ExperimentConfig::default()
    .with_seed(42)  // Fixed seed for reproducibility
    .with_deterministic(true);
```

### Reporting Checklist

For paper submission, ensure:

- [ ] All random seeds documented
- [ ] Hardware specifications reported
- [ ] Software versions captured
- [ ] Raw data archived
- [ ] Analysis scripts included

---

## Paper Integration

### LaTeX Tables

Generated tables follow standard conference formats:

```latex
% Accuracy Results (Exp-1)
\begin{table}[t]
\caption{Geometric Computation Accuracy}
\begin{tabular}{lcc}
\toprule
\textbf{Operation} & \textbf{Accuracy} & \textbf{Max Error} \\
\midrule
Length Measurement & 100\% & $1.2 \times 10^{-10}$ \\
Area Measurement   & 100\% & $2.1 \times 10^{-10}$ \\
Angle Measurement  & 100\% & $8.5 \times 10^{-11}$ \\
\bottomrule
\end{tabular}
\end{table}
```

### Figure Export

```bash
# High-resolution figures (300 DPI)
python scripts/visualize_results.py --dpi 300 --format pdf

# Vector graphics for papers
python scripts/visualize_results.py --format svg
```

### Statistical Reporting

```rust
// Statistical significance reporting
let result = statistical_test(&baseline, &enhanced);
println!("t({}) = {:.3}, p = {:.4}, d = {:.3}",
    result.df, result.t_value, result.p_value, result.effect_size);
```

---

## Statistical Analysis

### Significance Testing

The framework includes built-in statistical tests:

```rust
use experiment::statistics::{t_test, anova, effect_size};

// Independent t-test
let t_result = t_test::independent(&group_a, &group_b);
assert!(t_result.p_value < 0.05); // Significant at α=0.05

// Effect size (Cohen's d)
let d = effect_size::cohens_d(&group_a, &group_b);
println!("Effect size: {:.2}", d); // >0.8 = large effect
```

### Power Analysis

```rust
// Sample size justification
let required_n = power_analysis::sample_size(
    effect_size = 0.5,  // Expected medium effect
    alpha = 0.05,       // Significance level
    power = 0.80        // Desired statistical power
);
println!("Required samples: {}", required_n);
```

---

## Validity Threats

### Internal Validity

- **Selection Bias**: Random sampling with fixed seeds
- **Testing Effects**: Counterbalanced test order
- **Instrumentation**: Calibrated measurement tools

### External Validity

- **Generalizability**: Diverse test cases across domains
- **Ecological Validity**: Real-world CAD datasets

### Construct Validity

- **Metric Validity**: Standard metrics from prior work
- **Monomethod Bias**: Multiple evaluation methods

---

## Citation

When using this experiment framework in your research:

```bibtex
@software{cadagent2024,
  title = {CadAgent: Tool-Augmented Context Injection for CAD Geometric Reasoning},
  author = {Tokitai Team},
  year = {2024},
  url = {https://github.com/tokitai/cadagent}
}
```

---

## Support

For questions about the experiment framework:

1. Check this README
2. Review individual experiment documentation
3. Open an issue on GitHub

---

## License

MIT License - See LICENSE file for details
