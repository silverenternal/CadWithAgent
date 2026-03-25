# CAD Geometric Reasoning Research Directions

This document records the research directions and innovations of the CadAgent project, based on the latest 2025-2026 academic frontier.

## Research Directions

### 1. Symbolic Geometric Reasoning and Algorithmic Solving

**Core**: Shift from generative to symbolic/deterministic geometric reasoning to solve LLM unreliability problems

**Representative Papers**: CadVLM, ChainGeo, GeoDPO

**Research Opportunities**:
- Geometric symbolic representation
- Constraint satisfaction problems (CSP)
- Geometric theorem proving
- Deterministic solving algorithms

---

### 2. Tool-Augmented Paradigm

**Core**: LLMs handle intent understanding, specialized tools handle geometric computation/constraints/verification

**Representative Papers**: CAD-Assistant (ICCV 2025)

**Research Opportunities**:
- Encapsulate geometric algorithms as MCP tools
- Provide external tool interfaces for LLMs

**CadAgent Implementation**:
- ✅ `cad_extract_primitives` - Primitive extraction tool
- ✅ `cad_find_geometric_relations` - Geometric relation reasoning tool
- ✅ `cad_verify_constraints` - Constraint verification tool
- ✅ `cad_build_analysis_prompt` - Prompt construction tool
- ✅ `cad_context_inject` - Complete context injection workflow

---

### 3. Multi-View Orthographic Projection Reasoning

**Core**: 2D multi-view → 3D CAD, cross-view correspondence, dimension chain reasoning, view consistency

**Representative Papers**: CReFT-CAD (2025), TriView2CAD Benchmark

**Research Opportunities**:
- Multi-view reasoning tools
- Cross-view correspondence algorithms
- Dimension chain verification

---

### 4. Geometric Constraints and GD&T Reasoning

**Core**: Automatic constraint generation, conflict resolution, GD&T semantic parsing, tolerance accumulation analysis

**Representative Papers**: CadVLM, Context-Aware Mapping (2026)

**Research Opportunities**:
- Constraint generation/verification tools
- GD&T reasoning algorithms
- Tolerance analysis

---

### 5. Structured Representation and Hierarchical Reasoning

**Core**: Hierarchical geometric graph, primitive-level tokens, structured sequence representation

**Representative Papers**: CAD-Tokenizer (ICLR 2026), Hierarchical Graph (2025)

**Research Opportunities**:
- CAD structured representation
- Hierarchical constraint graph
- Primitive-level symbolic reasoning

---

### 6. Training-Free/Lightweight Adaptation

**Core**: No model training, use RAG/prompt engineering/tool calling for industrial adaptation

**Representative Papers**: Error Notebook-Guided (2026)

**Research Opportunities**:
- Training-free toolization
- Geometric knowledge RAG
- Prompt engineering + geometric rules

---

### 7. Interpretability and Reliability

**Core**: Geometric reasoning interpretable, verifiable, traceable, meeting industrial-grade requirements

**Representative Papers**: ChainGeo, GeoDPO, Context-Aware Mapping

**Research Opportunities**:
- Reasoning chain visualization
- Geometric verifier
- Deterministic result output

---

## CadAgent Innovations

### Implemented ✅

1. **Geometric Reasoning MCP Toolchain**
   - Benchmark: CAD-Assistant (ICCV 2025)
   - Value: No model training, pure algorithmic augmentation, solves LLM geometric reasoning shortcomings

2. **Automatic Constraint Generation + Conflict Resolution**
   - Benchmark: CadVLM, CReFT-CAD
   - Value: Precise constraint derivation, conflict detection, more reliable than LLMs

3. **CAD Symbolic Representation and Reasoning Engine**
   - Benchmark: CAD-Tokenizer, ChainGeo
   - Value: Unified structured interface, LLMs can directly parse and call

4. **Training-Free Geometric Knowledge RAG + Prompt Engineering**
   - Benchmark: Error Notebook-Guided
   - Value: No fine-tuning required, rapid industrial adaptation

5. **Industrial-Grade Geometric Verifier (GC Verifier)**
   - Benchmark: Error Notebook-Guided
   - Value: Ensures geometric output determinism, interpretability, and reliability

6. **Interpretable Geometric Reasoning Chain Tool**
   - Benchmark: ChainGeo, GeoDPO
   - Value: Reasoning chain traceable, auditable, meets industrial compliance

### Future Work

1. **Multi-View Orthographic Projection Reasoning**
   - Status: Planned
   - Priority: Medium
   - Estimated Effort: High (80 hours)

2. **GD&T Reasoning**
   - Status: Planned
   - Priority: Medium
   - Estimated Effort: High (60 hours)

3. **Geometry RAG (Retrieval-Augmented Generation)**
   - Status: Planned
   - Priority: Medium
   - Estimated Effort: High (50 hours)

---

## MCP Tool Function List

### Priority 1 - Core Must-Have ✅

- [x] `cad_extract_primitives` - Primitive extraction
- [x] `cad_find_geometric_relations` - Geometric relation reasoning
- [x] `cad_verify_constraints` - Constraint verification
- [x] `cad_build_analysis_prompt` - Prompt construction
- [x] `cad_context_inject` - Complete context injection workflow

### Priority 2 - Important Next Steps

- [ ] `cad_constraint_generate` - Automatic constraint generation
- [ ] `cad_constraint_check` - Constraint conflict checking
- [ ] `cad_multiview_reason` - Multi-view reasoning
- [ ] `cad_sketch_complete` - Sketch completion

### Priority 3 - Extended Future Work

- [ ] `cad_gd_t_reason` - GD&T reasoning
- [ ] `cad_geometry_rag` - Geometric knowledge retrieval
- [ ] `cad_reasoning_chain` - Reasoning chain visualization

---

## Chain-of-Thought Reasoning Flow

CadAgent implements a five-step geometric reasoning chain-of-thought:

```
1. Primitive Extraction → Identify geometric primitives from drawings (lines, circles, arcs, etc.)
2. Relation Reasoning → Derive geometric relations between primitives (parallel, perpendicular, connected, etc.)
3. Constraint Verification → Check constraint validity, detect conflicts and redundancies
4. Prompt Construction → Build structured geometric analysis prompts
5. VLM Inference → Send to LLM to generate interpretable reasoning chains
```

### Output Format

```json
{
  "reasoning_chain": [
    {
      "step": 1,
      "step_name": "Primitive Extraction",
      "action": "Extract all geometric primitives from SVG drawing",
      "result": {"primitives": [...]},
      "explanation": "Identified 24 primitives, including 18 lines, 4 circles..."
    },
    {
      "step": 2,
      "step_name": "Relation Reasoning",
      "action": "Reason about geometric relations between primitives",
      "result": {"relations": [...]},
      "explanation": "Found 32 geometric relations, including 8 parallel pairs, 6 perpendicular pairs..."
    }
  ],
  "final_geometry_info": {
    "primitives": [...],
    "constraints": [...],
    "topology_graph": {...}
  }
}
```

---

## References

1. **CReFT-CAD** (2025) - Multi-view CAD reasoning benchmark
2. **CAD-Tokenizer** (ICLR 2026) - CAD structured representation
3. **ChainGeo** - Geometric chain-of-thought reasoning
4. **GeoDPO** - Geometric reasoning optimization
5. **CadVLM** - CAD-VLM multimodal reasoning
6. **CAD-Assistant** (ICCV 2025) - Tool-augmented CAD reasoning
7. **Error Notebook-Guided** (2026) - Training-free adaptation methods

---

*Document last updated: 2026-03-25*
