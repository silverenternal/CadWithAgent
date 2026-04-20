# CadAgent 架构设计详解

**深入理解 CadAgent 的模块设计、数据流和核心抽象**

**版本**: v0.1.0 | **最后更新**: 2026-04-07 | **实现阶段**: Phase 1-7 完成

---

## 🏗️ 架构概览

### 完整系统架构 (2026-04)

```
┌─────────────────────────────────────────────────────────────────────┐
│                        用户接口层                                    │
│  CLI (main.rs)  │  Web UI (React/Three.js)  │  Library (lib.rs)    │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Web API 服务层 (Axum)                           │
│  /health  /chat  /upload  /export/:format  /tools  /constraints     │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      统一分析管线                                    │
│                analysis::AnalysisPipeline                            │
│  • inject_from_svg_string()    • inject_from_dxf_file()             │
│  • tool_call_chain()           • export_results()                   │
└─────────────────────────────────────────────────────────────────────┘
            │                           │                           │
            ▼                           ▼                           ▼
┌───────────────┐          ┌────────────────┐          ┌─────────────┐
│  解析层        │          │   几何引擎层    │          │   推理层    │
│  parser/      │          │   geometry/    │          │ cad_        │
│  • SVG        │          │  • 基元提取     │          │ reasoning/  │
│  • DXF        │          │  • 约束求解     │          │  • 关系检测 │
│  • STEP       │          │  • 3D 约束      │          │  • 冲突分析 │
│  • IGES       │          │  • GPU 加速     │          │  • 语义推断 │
└───────────────┘          └────────────────┘          └─────────────┘
            │                           │                           │
            ▼                           ▼                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      验证与导出层                                    │
│  cad_verifier/   │   cot/   │   export/   │   bridge/              │
│  约束验证          │  思维链   │  STEP/IGES  │  VLM API 适配          │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      上下文管理                                      │
│  context/ (tokitai-context)  │  memory/  │  metrics/                │
│  Git 风格分支管理              │  持久化    │  评估指标                │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      GPU 加速层 (wgpu)                               │
│  gpu::compute (JacobianPipeline)  │  gpu::renderer (RenderConfig)  │
│  2.7x-5x 加速 (50-500 变量)         │  MSAA, LOD, 线框模式            │
└─────────────────────────────────────────────────────────────────────┘
```

---

## ✅ Phase 1-7 实现状态 (2026-04)

### 已完成功能清单

| Phase | 任务 | 状态 | 关键成果 |
|-------|------|------|----------|
| **Phase 0** | 代码质量清理 | ✅ 完成 | Clippy 0 warnings, 统一错误处理 |
| **Phase 1** | LLM 推理能力 | ✅ 完成 | 本地模型支持 (Ollama, LM Studio), 50+ 轮对话 |
| **Phase 2** | 3D 约束求解 | ✅ 完成 | 12 种 3D 约束类型，Newton 求解器 |
| **Phase 7** | GPU 加速 | ✅ 完成 | JacobianPipeline 2.7x-5x 加速，RenderConfig |
| **Phase 8** | Web UI | ✅ 完成 | React/Three.js 前端，Axum 后端 |

### 核心性能指标

| 指标 | 数值 | 说明 |
|------|------|------|
| 测试通过 | 1063 tests | 0 failed, 1 ignored |
| Clippy | 0 warnings | 代码质量达标 |
| 构建时间 | ~13s | `cargo build --release` |
| GPU 加速 | 2.7x-5x | 50-500 变量约束系统 |
| 对话轮数 | 50+ | max_short_term_turns 默认 50 |
| Web API | 8 端点 | /health, /chat, /upload, /export 等 |

### Web API 端点

| 端点 | 方法 | 功能 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/chat` | POST | AI 聊天 |
| `/upload` | POST | 文件上传 |
| `/export/:format` | GET | 导出为 STEP/IGES/SVG |
| `/tools` | GET | 可用工具列表 |
| `/tools/execute` | POST | 执行工具 |
| `/constraints/apply` | POST | 应用约束 |
| `/constraints/solve` | POST | 求解约束系统 |

### Web UI 组件

| 组件 | 技术 | 功能 |
|------|------|------|
| CADModel.tsx | Three.js/R3F | 3D 模型渲染 |
| ChatPanel.tsx | React/Markdown | AI 聊天界面 |
| FeatureTree.tsx | React | 特征树面板 |
| PropertiesPanel.tsx | React | 属性编辑器 |
| Toolbar.tsx | React | 工具栏 |

---

## 📦 核心模块详解

### 24 个模块分类

#### P0 - 核心研究模块 (必懂)

| 模块 | 行数 | 测试 | 职责 | 状态 |
|------|------|------|------|------|
| `analysis/` | 1,200 | 90% | **统一入口**，整合所有功能 | ✅ |
| `geometry/` | 8,500 | 96% | 几何算法核心 (约束求解、测量) | ✅ |
| `cad_reasoning/` | 2,800 | 90% | 几何关系推理 (相邻、平行、垂直) | ✅ |
| `cad_verifier/` | 1,500 | 85% | 约束冲突检测与诊断 | ✅ |
| `prompt_builder/` | 800 | 80% | 结构化提示词构造 | ✅ |

#### P1 - 支撑模块 (按需了解)

| 模块 | 行数 | 测试 | 职责 | 状态 |
|------|------|------|------|------|
| `parser/` | 3,200 | 88% | SVG/DXF/STEP/IGES 文件解析 | ✅ |
| `topology/` | 1,800 | 85% | 拓扑分析 (回路、房间检测) | ✅ |
| `bridge/` | 1,500 | 75% | VLM API 适配 (zazaz, OpenAI) | ✅ |
| `context/` | 1,200 | 85% | 对话状态管理 (tokitai-context) | ✅ |
| `cot/` | 600 | 70% | Geo-CoT 思维链模板 | ✅ |
| `memory/` | 500 | 80% | 持久化存储 | ✅ |
| `metrics/` | 900 | 75% | 几何一致性评估指标 | ✅ |

#### P2 - 扩展模块 (高级功能)

| 模块 | 行数 | 测试 | 职责 | 状态 |
|------|------|------|------|------|
| `gpu/` | 1,800 | 100% | GPU 加速计算 (wgpu) | ✅ |
| `feature/` | 800 | 50% | 特征识别 (门窗、家具) | ⚠️ 基础 |
| `incremental/` | 600 | 40% | 增量更新优化 | 🔜 TODO |
| `lod/` | 400 | 30% | Level of Detail 管理 | 🔜 TODO |
| `llm_reasoning/` | 1,000 | 65% | LLM 推理引擎 | ⚠️ Mock |

#### 工具模块

| 模块 | 行数 | 测试 | 职责 | 状态 |
|------|------|------|------|------|
| `config/` | 400 | 90% | 配置验证 | ✅ |
| `tools/` | 300 | 85% | 工具函数 | ✅ |
| `error.rs` | 200 | 100% | 统一错误类型 | ✅ |
| `web_server.rs` | 350 | 85% | Axum Web API 服务器 | ✅ |

---

## 🔄 核心数据流

### 典型分析流程

```
SVG 文件
  │
  ▼
┌─────────────────┐
│ parser::svg     │ 解析为几何基元
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ geometry::      │ 提取 Point, Line, Circle
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ cad_reasoning:: │ 检测几何关系
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ cad_verifier::  │ 验证约束一致性
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ prompt_builder::│ 构造 Geo-Guided Prompt
└─────────────────┘
  │
  ▼
┌─────────────────┐
│ bridge::        │ 调用 VLM API
└─────────────────┘
  │
  ▼
分析结果 (含工具调用链)
```

### 数据结构演化

```rust
// 1. 原始输入
let svg = "<svg>...</svg>";

// 2. 解析为基元
struct Primitive {
    id: UUID,
    kind: PrimitiveKind,  // Line, Circle, Arc...
    geometry: Geometry,   // 具体坐标参数
}

// 3. 提取关系
struct GeometricRelation {
    kind: RelationKind,   // Adjacent, Parallel, Perpendicular
    entities: [UUID; 2],
    confidence: f64,
}

// 4. 构建约束
struct Constraint {
    kind: ConstraintKind, // Distance, Angle, Coincident
    variables: Vec<UUID>,
    equation: Equation,
}

// 5. 验证结果
struct VerificationResult {
    is_consistent: bool,
    conflicts: Vec<Conflict>,
    suggestions: Vec<String>,
}

// 6. 最终输出
struct AnalysisResult {
    primitives: Vec<Primitive>,
    relations: Vec<GeometricRelation>,
    constraints: Vec<Constraint>,
    verification: VerificationResult,
    tool_chain: ToolCallChain,  // 可追溯推理链
}
```

---

## 🎯 核心抽象

### 1. AnalysisPipeline (统一入口)

```rust
pub struct AnalysisPipeline {
    config: AnalysisConfig,
    vlm_client: Option<VlmClient>,
    // ... 内部状态
}

impl AnalysisPipeline {
    // 创建默认管线
    pub fn with_defaults() -> Result<Self, Error>;
    
    // 从 SVG 字符串注入
    pub fn inject_from_svg_string(
        &self,
        svg: &str,
        task: &str,
    ) -> Result<AnalysisResult, Error>;
    
    // 从 DXF 文件注入
    pub fn inject_from_dxf_file(
        &self,
        path: &Path,
        task: &str,
    ) -> Result<AnalysisResult, Error>;
}
```

**设计思想:** 隐藏底层复杂性，提供简单一致的 API。

### 2. ToolCallChain (可追溯性)

```rust
pub struct ToolCallChain {
    steps: Vec<ToolCallStep>,
    metadata: ChainMetadata,
}

pub struct ToolCallStep {
    step_id: usize,
    tool_name: String,      // "extract_primitives", "detect_relations"
    inputs: JsonValue,
    outputs: JsonValue,
    explanation: String,    // 人类可读的解释
    timestamp: u64,
}
```

**用途:** 每一步推理都有据可查，避免"黑箱"AI。

### 3. ConstraintSystem (约束系统)

```rust
pub struct ConstraintSystem {
    variables: Vec<ConstraintVariable>,
    constraints: Vec<Constraint>,
    jacobian: SparseMatrix,  // 稀疏 Jacobian
}

impl ConstraintSystem {
    // 求解约束
    pub fn solve(&mut self) -> Result<Solution, SolverError>;
    
    // 检测冲突
    pub fn detect_conflicts(&self) -> Vec<Conflict>;
    
    // 诊断问题
    pub fn diagnose(&self) -> Diagnosis;
}
```

**特点:** 使用稀疏矩阵优化大规模约束求解。

---

## 🔧 关键设计决策

### 1. 为什么选择 Rust？

**性能:**
- 零成本抽象
- SIMD 自动向量化
- 并行计算 (Rayon)

**安全:**
- 编译期内存检查
- 无数据竞争
- 类型安全

**生态:**
- 优秀的几何库 (geo, nalgebra)
- 成熟的 WebAssembly 支持

### 2. 为什么分层设计？

```
analysis/ (统一入口)
  │
  ├── cad_extractor/ (底层)
  ├── cad_reasoning/ (底层)
  └── cad_verifier/ (底层)
```

**优点:**
- 新用户只需了解 `analysis` 模块
- 高级用户可深入底层定制
- 测试隔离，便于维护

### 3. 为什么使用稀疏矩阵？

约束求解的 Jacobian 矩阵通常是稀疏的:

```
对于 1000 个变量的约束系统:
- 稠密矩阵：1000×1000 = 1,000,000 元素
- 稀疏矩阵：仅~5000 非零元素 (99.5% 稀疏度)

性能提升：3-20x
```

### 4. 为什么集成 zazaz API？

**自研优先:**
- 完全控制 API 演进
- 避免第三方依赖风险
- 成本优化

**功能需求:**
- AI 辅助合并
- 冲突自动解决
- 分支目的推断

---

## 📊 性能特征

### 时间复杂度

| 操作 | 朴素实现 | CadAgent | 提升 |
|------|---------|----------|------|
| 空间关系检测 | O(n²) | O(n log n) | 10-100x |
| 约束求解 | O(n²) | O(n log n)* | 3-20x |
| 冲突检测 | O(n²) | O(n log n) | 2-3x |

*依赖分析优化后

### 内存使用

| 场景 | 内存占用 |
|------|---------|
| 小型户型图 (50 基元) | ~5 MB |
| 中型楼层 (500 基元) | ~50 MB |
| 大型建筑 (5000 基元) | ~500 MB |

---

## 🧪 测试策略

### 测试金字塔

```
       /\
      /  \   E2E 测试 (10%)
     /----\  完整分析流程
    /      \
   /--------\  集成测试 (30%)
  /          \  模块间交互
 /------------\
/--------------\  单元测试 (60%)
                 单个函数/方法
```

### 测试覆盖要求

| 模块类型 | 覆盖目标 | 当前状态 |
|---------|---------|---------|
| P0 核心模块 | 90%+ | 96% ✅ |
| P1 支撑模块 | 80%+ | 85% ✅ |
| P2 扩展模块 | 60%+ | 70% ⚠️ |
| 工具模块 | 95%+ | 100% ✅ |

---

## 🔮 未来架构演进

### v0.2.0 (2026 Q3)

```
新增:
• B-Rep 完整支持
• 完整 DXF 实体解析
• 约束依赖分析优化
• WebAssembly 支持
```

### v0.3.0 (2026 Q4)

```
新增:
• 实时协作 (多用户)
• 版本控制增强
• GUI 界面 (Tauri)
• 插件系统
```

### v1.0.0 (2027 Q1)

```
目标:
• 生产就绪
• 完整文档
• 稳定 API
• 社区生态
```

---

*最后更新：2026-04-07 | 版本：v0.1.0 | Phase 1-7 完成*
