# CAD 几何推理研究方向

本文档记录 CadAgent 项目的研究方向和创新点，基于 2025-2026 年最新学术前沿。

## 研究方向

### 1. 符号化几何推理与算法化求解

**核心**: 从生成式转向符号化/确定性几何推理，解决大模型不可靠问题

**代表论文**: CadVLM, ChainGeo, GeoDPO

**可做方向**:
- 几何符号表示
- 约束满足问题 (CSP)
- 几何定理证明
- 确定性求解算法

---

### 2. 工具增强 (Tool-Augmented) 范式

**核心**: 大模型做意图理解，专用工具做几何计算/约束/校验

**代表论文**: CAD-Assistant (ICCV 2025)

**可做方向**:
- 封装几何算法为 MCP 工具
- 给大模型提供外挂工具接口

**CadAgent 实现**:
- `cad_extract_primitives` - 基元提取工具
- `cad_find_geometric_relations` - 几何关系推理工具
- `cad_verify_constraints` - 约束校验工具
- `cad_build_analysis_prompt` - 提示词构造工具

---

### 3. 多视图正交投影推理

**核心**: 2D 多视图→3D CAD、跨视图对应、尺寸链推理、视图一致性

**代表论文**: CReFT-CAD (2025), TriView2CAD 基准

**可做方向**:
- 多视图推理工具
- 跨视图对应算法
- 尺寸链校验

---

### 4. 几何约束与 GD&T 推理

**核心**: 自动约束生成、冲突消解、GD&T 语义解析、公差累积分析

**代表论文**: CadVLM, Context-Aware Mapping (2026)

**可做方向**:
- 约束生成/校验工具
- GD&T 推理算法
- 公差分析

---

### 5. 结构化表示与层级化推理

**核心**: 层级化几何图、基元级 token、结构化序列表示

**代表论文**: CAD-Tokenizer (ICLR 2026), Hierarchical Graph (2025)

**可做方向**:
- CAD 结构化表示
- 层级化约束图
- 基元级符号推理

---

### 6. 免训练/轻量适配

**核心**: 不训模型，用 RAG/提示工程/工具调用适配工业场景

**代表论文**: Error Notebook-Guided (2026)

**可做方向**:
- 免训练工具化
- 几何知识 RAG
- 提示工程 + 几何规则

---

### 7. 可解释性与可靠性

**核心**: 几何推理可解释、可校验、可追溯，满足工业级要求

**代表论文**: ChainGeo, GeoDPO, Context-Aware Mapping

**可做方向**:
- 推理链可视化
- 几何校验器
- 确定性结果输出

---

## CadAgent 创新点

### 已实现

1. **几何推理 MCP 工具链**
   - 对标：CAD-Assistant (ICCV 2025)
   - 价值：不训模型，纯算法外挂，解决大模型几何推理短板

2. **自动约束生成 + 冲突消解**
   - 对标：CadVLM, CReFT-CAD
   - 价值：精准推导约束，检测冲突，比大模型更可靠

3. **CAD 符号化表示与推理引擎**
   - 对标：CAD-Tokenizer, ChainGeo
   - 价值：统一结构化接口，大模型可直接解析调用

4. **免训练几何知识 RAG+ 提示工程**
   - 对标：Error Notebook-Guided
   - 价值：无需微调，快速适配工业场景

5. **工业级几何校验器 (GC Verifier)**
   - 对标：Error Notebook-Guided
   - 价值：确保几何输出确定性、可解释、可靠

6. **可解释几何推理链工具**
   - 对标：ChainGeo, GeoDPO
   - 价值：推理链可追溯、可审计，满足工业合规

---

## MCP 工具功能清单

### 优先级 1 - 核心必做 ✅

- [x] `cad_extract_primitives` - 基元提取
- [x] `cad_find_geometric_relations` - 几何关系推理
- [x] `cad_verify_constraints` - 约束校验
- [x] `cad_build_analysis_prompt` - 提示词构造
- [x] `cad_context_inject` - 完整上下文注入流程

### 优先级 2 - 重要次做

- [ ] `cad_constraint_generate` - 自动约束生成
- [ ] `cad_constraint_check` - 约束冲突检查
- [ ] `cad_multiview_reason` - 多视图推理
- [ ] `cad_sketch_complete` - 草图补全

### 优先级 3 - 拓展后做

- [ ] `cad_gd_t_reason` - GD&T 推理
- [ ] `cad_geometry_rag` - 几何知识检索
- [ ] `cad_reasoning_chain` - 推理链可视化

---

## 思维链推理流程

CadAgent 实现的五步几何推理思维链：

```
1. 基元提取 → 从图纸中识别几何基元（线段、圆、弧等）
2. 关系推理 → 推导基元间的几何关系（平行、垂直、连接等）
3. 约束校验 → 检查约束合法性，检测冲突和冗余
4. 提示词构造 → 构建结构化的几何分析提示词
5. VLM 推理 → 送入大模型生成可解释的推理思维链
```

### 输出格式

```json
{
  "reasoning_chain": [
    {
      "step": 1,
      "step_name": "基元提取",
      "action": "从 SVG 图纸中提取所有几何基元",
      "result": {"primitives": [...]},
      "explanation": "共识别 24 个基元，包括 18 条线段、4 个圆..."
    },
    {
      "step": 2,
      "step_name": "关系推理",
      "action": "推理基元间的几何关系",
      "result": {"relations": [...]},
      "explanation": "发现 32 个几何关系，包括 8 对平行、6 对垂直..."
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

## 参考文献

1. **CReFT-CAD** (2025) - 多视图 CAD 推理基准
2. **CAD-Tokenizer** (ICLR 2026) - CAD 结构化表示
3. **ChainGeo** - 几何思维链推理
4. **GeoDPO** - 几何推理优化
5. **CadVLM** - CAD-VLM 多模态推理
6. **CAD-Assistant** (ICCV 2025) - 工具增强 CAD 推理
7. **Error Notebook-Guided** (2026) - 免训练适配方法

---

*文档更新日期：2026-03-23*
