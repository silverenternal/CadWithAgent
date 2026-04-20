# AI 自主全流程 CAD 理解 - 能力差距分析

**最后更新**: 2026-04-06 | **版本**: v1.0

---

## 📋 执行摘要

### 当前能力评估

| 能力维度 | 当前状态 | 目标状态 | 差距 |
|---------|---------|---------|------|
| **图纸解析** | ✅ SVG/DXF 解析完成 | ✅ 支持 STEP/IGES | 🔶 中等 |
| **几何理解** | ✅ 基元提取 + 关系推理 | ✅ 3D 特征识别 | 🔴 高 |
| **设计优化** | ⚠️ 冲突检测 + 修复建议 | ✅ 自动参数化优化 | 🔴 高 |
| **自主决策** | ⚠️ 单轮 LLM 推理 | ✅ 多轮迭代优化 | 🔴 高 |
| **记忆持久化** | ⚠️ LRU 缓存 (会话级) | ✅ 长期知识图谱 | 🔴 高 |

### 核心问题

**当前 CadAgent 是"几何增强的 VLM 助手"，而非"自主 CAD 设计代理"**

---

## 🎯 目标：AI 自主全流程能力

### 理想工作流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI 自主全流程 CAD 处理                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. 解析 ──→ 2. 理解 ──→ 3. 设计 ──→ 4. 优化 ──→ 5. 验证       │
│      │           │           │           │           │          │
│      ▼           ▼           ▼           ▼           ▼          │
│  多格式      语义理解    方案生成    迭代优化    自动验证       │
│  自适应      意图推断    约束求解    性能评估    合规检查       │
│                                                                  │
│  ◀──────────────────── 反馈循环 ────────────────────────────▶   │
│                                                                  │
│  🧠 长期记忆：设计模式库 | 错误知识库 | 用户偏好                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🔍 差距分析

### 1️⃣ 图纸解析层 (Parsing Layer)

#### 当前能力 ✅
- SVG 解析 (完整)
- DXF 解析 (完整)
- 坐标归一化
- 图层过滤

#### 欠缺能力 ❌

| 功能 | 重要性 | 实现难度 | 优先级 |
|------|--------|---------|--------|
| **STEP 格式解析** | 🔥 高 | 中 | P0 |
| **IGES 格式解析** | 🔥 高 | 中 | P0 |
| **B-Rep 边界表示** | 🔥 高 | 高 | P0 |
| **点云输入支持** | 中 | 中 | P1 |
| **OCR 文本识别** | 中 | 低 | P1 |
| **图层语义标注** | 中 | 低 | P2 |

#### 建议实现

```rust
// 新增：统一解析器 trait
pub trait CadFormatParser: Send + Sync {
    fn parse_file(&self, path: &Path) -> CadAgentResult<UnifiedCadModel>;
    fn supported_formats(&self) -> &[&str];
}

// 新增：统一 CAD 数据模型
pub struct UnifiedCadModel {
    pub geometry: GeometryData,      // 几何数据
    pub topology: TopologyData,      // 拓扑关系
    pub semantics: SemanticData,     // 语义信息
    pub metadata: Metadata,          // 元数据
}

// 新增：STEP 解析器
pub struct StepParser {
    config: StepConfig,
}

impl StepParser {
    pub fn parse(&self, path: &Path) -> CadAgentResult<StepModel> {
        // 使用 stepnc 或 opencascade Rust 绑定
        todo!()
    }
}
```

**推荐依赖**:
```toml
[dependencies]
# STEP/IGES 解析
stepnc = "0.1"  # 或
opencascade-sys = "0.1"  # OpenCASCADE 绑定 (功能最强)

# 点云处理
ply-rs = "0.1"  # PLY 格式
pcap = "0.1"    # 点云处理
```

---

### 2️⃣ 几何理解层 (Understanding Layer)

#### 当前能力 ✅
- 基元提取 (线、圆、弧、多边形)
- 几何关系推理 (平行、垂直、连接)
- 房间检测 (2D 封闭区域)
- 约束冲突检测

#### 欠缺能力 ❌

| 功能 | 重要性 | 实现难度 | 优先级 |
|------|--------|---------|--------|
| **3D 特征识别** | 🔥 高 | 高 | P0 |
| **设计意图推断** | 🔥 高 | 高 | P0 |
| **参数化约束求解** | 🔥 高 | 高 | P0 |
| **装配关系理解** | 高 | 中 | P1 |
| **公差分析** | 中 | 中 | P2 |
| **材料/工艺推断** | 低 | 高 | P3 |

#### 建议实现

```rust
// 新增：3D 特征识别
pub mod feature_recognition {
    /// 加工特征识别
    pub struct MachiningFeatureRecognizer;
    
    impl MachiningFeatureRecognizer {
        pub fn recognize(&self, brep: &BRepModel) -> Vec<MachiningFeature> {
            // 识别孔、槽、凸台等加工特征
            todo!()
        }
    }
    
    /// 设计特征识别
    pub struct DesignFeatureRecognizer;
    
    impl DesignFeatureRecognizer {
        pub fn recognize_symmetry(&self, brep: &BRepModel) -> SymmetryInfo {
            // 识别对称面、旋转轴等
            todo!()
        }
        
        pub fn infer_design_intent(&self, features: &[Feature]) -> DesignIntent {
            // 推断设计意图（如：同轴、均布、镜像）
            todo!()
        }
    }
}

// 新增：参数化约束求解器
pub mod constraint_solver {
    pub struct ParametricSolver {
        equation_system: SparseSystem,
    }
    
    impl ParametricSolver {
        pub fn solve(&mut self, constraints: &[Constraint]) -> SolverResult {
            // 使用牛顿 - 拉弗森法求解非线性约束
            todo!()
        }
        
        pub fn solve_with_optimization(
            &mut self,
            constraints: &[Constraint],
            objective: ObjectiveFunction
        ) -> OptimizationResult {
            // 带优化目标的约束求解（如：最小材料、最小应力）
            todo!()
        }
    }
}

// 新增：设计意图推断
pub mod intent_inference {
    use crate::llm_reasoning::LlmReasoningEngine;
    
    pub struct IntentInferenceEngine {
        geometric_reasoner: GeometricReasoner,
        llm_engine: LlmReasoningEngine,
    }
    
    impl IntentInferenceEngine {
        pub fn infer_pattern(&self, features: &[Feature]) -> PatternIntent {
            // 推断阵列、镜像、缩放等设计模式
            todo!()
        }
        
        pub fn infer_functional_requirement(
            &self,
            geometry: &GeometryData,
            context: &str
        ) -> Vec<FunctionalRequirement> {
            // 基于几何和上下文推断功能需求
            todo!()
        }
    }
}
```

---

### 3️⃣ 设计优化层 (Design & Optimization Layer)

#### 当前能力 ⚠️
- 冲突检测
- 修复建议生成 (文本)
- 几何变换 (平移、旋转、缩放)

#### 欠缺能力 ❌

| 功能 | 重要性 | 实现难度 | 优先级 |
|------|--------|---------|--------|
| **自动拓扑优化** | 🔥 高 | 高 | P0 |
| **生成式设计** | 🔥 高 | 高 | P0 |
| **多目标优化** | 高 | 高 | P1 |
| **实时交互编辑** | 高 | 中 | P1 |
| **版本对比/合并** | 中 | 中 | P2 |

#### 建议实现

```rust
// 新增：拓扑优化引擎
pub mod topology_optimization {
    pub struct TopologyOptimizer {
        mesh: MeshModel,
        material: MaterialModel,
    }
    
    impl TopologyOptimizer {
        pub fn optimize_simp(&mut self, volume_fraction: f64) -> MeshModel {
            // SIMP 法拓扑优化
            todo!()
        }
        
        pub fn optimize_level_set(&mut self) -> MeshModel {
            // 水平集方法
            todo!()
        }
    }
}

// 新增：生成式设计引擎
pub mod generative_design {
    use crate::llm_reasoning::LlmReasoningEngine;
    
    pub struct GenerativeDesigner {
        llm_engine: LlmReasoningEngine,
        constraint_solver: ParametricSolver,
    }
    
    impl GenerativeDesigner {
        pub fn generate_from_requirements(
            &self,
            requirements: &[FunctionalRequirement],
            constraints: &[Constraint]
        ) -> Vec<DesignCandidate> {
            // LLM 生成设计方案，几何引擎验证可行性
            todo!()
        }
        
        pub fn iterate_design(
            &self,
            current: &DesignCandidate,
            feedback: &str
        ) -> DesignCandidate {
            // 基于反馈迭代设计
            todo!()
        }
    }
}

// 新增：多目标优化
pub mod multi_objective {
    pub struct MultiObjectiveOptimizer {
        objectives: Vec<ObjectiveFunction>,
        constraints: Vec<Constraint>,
    }
    
    impl MultiObjectiveOptimizer {
        pub fn optimize_nsga2(&self) -> Vec<ParetoSolution> {
            // NSGA-II 多目标优化
            todo!()
        }
        
        pub fn optimize_mopso(&self) -> Vec<ParetoSolution> {
            // 多目标粒子群优化
            todo!()
        }
    }
}
```

---

### 4️⃣ 自主决策层 (Autonomous Decision Layer)

#### 当前能力 ⚠️
- 单轮 LLM 推理
- 工具调用链 (2-4 步)
- 回退机制 (Mock 模式)

#### 欠缺能力 ❌

| 功能 | 重要性 | 实现难度 | 优先级 |
|------|--------|---------|--------|
| **多轮对话状态管理** | 🔥 高 | 中 | P0 |
| **任务规划与分解** | 🔥 高 | 高 | P0 |
| **自我反思/修正** | 🔥 高 | 高 | P0 |
| **工具自主选择** | 高 | 中 | P1 |
| **不确定性量化** | 中 | 中 | P2 |

#### 建议实现

```rust
// 新增：自主 CAD 代理
pub mod autonomous_agent {
    use crate::llm_reasoning::LlmReasoningEngine;
    use crate::tools::ToolRegistry;
    
    /// 自主 CAD 设计代理
    pub struct CadAgent {
        /// 任务规划器
        planner: TaskPlanner,
        /// 工具执行器
        executor: ToolExecutor,
        /// 反思模块
        reflector: SelfReflector,
        /// 状态管理器
        state_manager: DialogStateManager,
    }
    
    impl CadAgent {
        pub async fn execute_task(&mut self, task: &Task) -> AgentResult {
            // 1. 任务理解
            let understanding = self.planner.understand(task).await?;
            
            // 2. 任务分解
            let subtasks = self.planner.decompose(&understanding)?;
            
            // 3. 迭代执行
            let mut results = Vec::new();
            for subtask in &subtasks {
                let result = self.executor.execute(subtask).await?;
                
                // 4. 自我反思
                if let Some(issue) = self.reflector.check(&result) {
                    // 5. 修正执行
                    let corrected = self.executor.execute_corrective(subtask, &issue).await?;
                    results.push(corrected);
                } else {
                    results.push(result);
                }
            }
            
            // 6. 整合结论
            self.synthesize(&results)
        }
    }
    
    /// 任务规划器
    pub struct TaskPlanner {
        llm: LlmReasoningEngine,
        template_library: TaskTemplateLibrary,
    }
    
    impl TaskPlanner {
        pub async fn understand(&self, task: &Task) -> TaskUnderstanding {
            // LLM 理解任务意图
            todo!()
        }
        
        pub fn decompose(&self, understanding: &TaskUnderstanding) -> Vec<SubTask> {
            // 分解为可执行的子任务
            // 例如："优化这个零件" → ["分析约束", "识别优化空间", "生成候选方案", "验证可行性"]
            todo!()
        }
    }
    
    /// 自我反思器
    pub struct SelfReflector {
        validation_rules: Vec<ValidationRule>,
    }
    
    impl SelfReflector {
        pub fn check(&self, result: &ExecutionResult) -> Option<Issue> {
            // 检查结果是否满足约束
            // 发现潜在问题（如：几何冲突、性能不达标）
            todo!()
        }
    }
    
    /// 对话状态管理器
    pub struct DialogStateManager {
        context_window: ContextWindow,
        memory: LongTermMemory,
    }
}

// 新增：任务模板库
pub mod task_templates {
    pub struct TaskTemplateLibrary;
    
    impl TaskTemplateLibrary {
        pub fn get_template(&self, task_type: &str) -> Option<TaskTemplate> {
            match task_type {
                "design_optimization" => Some(TaskTemplate {
                    name: "设计优化",
                    steps: vec![
                        "分析当前设计约束",
                        "识别优化目标",
                        "生成候选方案",
                        "验证几何可行性",
                        "评估性能指标",
                        "输出最优方案",
                    ],
                }),
                "conflict_resolution" => Some(TaskTemplate {
                    name: "冲突解决",
                    steps: vec![
                        "检测约束冲突",
                        "分析冲突原因",
                        "生成修复建议",
                        "验证修复方案",
                    ],
                }),
                _ => None,
            }
        }
    }
}
```

---

### 5️⃣ 记忆与知识层 (Memory & Knowledge Layer)

#### 当前能力 ⚠️
- LRU 缓存 (VLM 响应)
- 会话级状态

#### 欠缺能力 ❌

| 功能 | 重要性 | 实现难度 | 优先级 |
|------|--------|---------|--------|
| **设计模式知识库** | 🔥 高 | 中 | P0 |
| **错误案例库** | 🔥 高 | 低 | P0 |
| **用户偏好学习** | 高 | 中 | P1 |
| **跨项目知识迁移** | 高 | 高 | P1 |
| **版本历史追踪** | 中 | 低 | P2 |

#### 建议实现

```rust
// 新增：长期记忆系统
pub mod memory {
    use serde::{Serialize, Deserialize};
    
    /// 设计模式知识库
    pub struct DesignPatternDatabase {
        storage: SqliteStorage,
        embeddings: EmbeddingModel,
    }
    
    impl DesignPatternDatabase {
        pub fn store_pattern(&self, pattern: &DesignPattern) {
            // 存储设计模式（如：对称布局、标准件选型）
            todo!()
        }
        
        pub fn retrieve_similar(&self, query: &str, k: usize) -> Vec<DesignPattern> {
            // 基于语义检索相似设计模式
            todo!()
        }
    }
    
    /// 错误案例库
    pub struct ErrorCaseLibrary {
        cases: Vec<ErrorCase>,
    }
    
    impl ErrorCaseLibrary {
        pub fn record_error(&mut self, error: &GeometryError, context: &str, fix: &str) {
            // 记录错误案例和修复方案
            todo!()
        }
        
        pub fn find_similar_case(&self, error: &GeometryError) -> Option<&ErrorCase> {
            // 查找相似错误案例
            todo!()
        }
    }
    
    /// 用户偏好
    pub struct UserPreferenceModel {
        preferences: HashMap<String, PreferenceValue>,
    }
    
    impl UserPreferenceModel {
        pub fn learn_from_feedback(&mut self, action: &str, feedback: f32) {
            // 从用户反馈中学习偏好
            todo!()
        }
        
        pub fn get_preference(&self, context: &str) -> Option<&PreferenceValue> {
            todo!()
        }
    }
}

// 新增：知识图谱
pub mod knowledge_graph {
    pub struct CadKnowledgeGraph {
        entities: Vec<KnowledgeEntity>,
        relations: Vec<KnowledgeRelation>,
    }
    
    impl CadKnowledgeGraph {
        pub fn add_design_knowledge(&mut self, design: &DesignCandidate, outcome: &str) {
            // 从设计结果中提取知识
            todo!()
        }
        
        pub fn query(&self, query: &KnowledgeQuery) -> Vec<KnowledgeEntity> {
            // 图谱查询
            todo!()
        }
    }
}
```

---

## 🏗️ 架构升级建议

### 当前架构

```
┌─────────────────────────────────────────┐
│           LLM Reasoning Engine          │
├─────────────────────────────────────────┤
│  Analysis Pipeline (Geometry Tools)     │
├─────────────────────────────────────────┤
│  CAD Extractor → Parser → Geometry      │
└─────────────────────────────────────────┘
```

### 目标架构

```
┌─────────────────────────────────────────────────────────┐
│              Autonomous CAD Agent Layer                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Task       │  │  Self-      │  │  Dialog     │     │
│  │  Planner    │  │  Reflector  │  │  State Mgr  │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Knowledge & Memory Layer                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Design     │  │  Error      │  │  User       │     │
│  │  Patterns   │  │  Library    │  │  Preference │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Design & Optimization Layer                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Generative │  │  Topology   │  │  Multi-     │     │
│  │  Designer   │  │  Optimizer  │  │  Objective  │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Understanding & Reasoning Layer             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Feature    │  │  Intent     │  │  Constraint │     │
│  │  Recognizer │  │  Inference  │  │  Solver     │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              Parsing & Input Layer                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  STEP/IGES  │  │  SVG/DXF    │  │  Point      │     │
│  │  Parser     │  │  Parser     │  │  Cloud      │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
```

---

## 📅 实施路线图

### Phase 1: 基础增强 (2026 Q2) - 2 个月

| 任务 | 预计工时 | 依赖 |
|------|---------|------|
| STEP/IGES 解析器 | 2 周 | - |
| 3D 特征识别框架 | 3 周 | STEP 解析 |
| 参数化约束求解器 | 4 周 | - |
| 对话状态管理 | 2 周 | - |
| 错误案例库 | 1 周 | - |

**里程碑**: 支持 3D CAD 格式输入 + 基础参数化编辑

### Phase 2: 自主决策 (2026 Q3) - 3 个月

| 任务 | 预计工时 | 依赖 |
|------|---------|------|
| 任务规划器 | 3 周 | 对话状态管理 |
| 自我反思模块 | 3 周 | 错误案例库 |
| 工具自主选择 | 2 周 | 任务规划器 |
| 设计模式知识库 | 3 周 | - |
| 用户偏好学习 | 2 周 | - |

**里程碑**: 多轮自主任务执行能力

### Phase 3: 设计优化 (2026 Q4) - 3 个月

| 任务 | 预计工时 | 依赖 |
|------|---------|------|
| 生成式设计引擎 | 4 周 | 约束求解器 |
| 拓扑优化框架 | 4 周 | - |
| 多目标优化 | 3 周 | 生成式设计 |
| 实时交互编辑 | 3 周 | - |

**里程碑**: 完整生成式设计能力

### Phase 4: 知识增强 (2027 Q1) - 2 个月

| 任务 | 预计工时 | 依赖 |
|------|---------|------|
| 知识图谱构建 | 3 周 | 设计模式库 |
| 跨项目迁移学习 | 3 周 | 知识图谱 |
| 版本历史系统 | 2 周 | - |

**里程碑**: 知识驱动的自主设计

---

## 📊 预期效果

### 能力对比

| 能力 | 当前 | Phase 1 | Phase 2 | Phase 3 | Phase 4 |
|------|------|---------|---------|---------|---------|
| 格式支持 | 2D | 3D | 3D | 3D | 3D |
| 自主决策 | 单轮 | 单轮 | 多轮 | 多轮 | 多轮 + 学习 |
| 设计优化 | 建议 | 参数化 | 自动 | 生成式 | 知识驱动 |
| 准确率 | 89% | 85%* | 90% | 92% | 95% |

*Phase 1 因 3D 复杂度增加，准确率可能暂时下降

### 性能指标

| 指标 | 当前 | 目标 |
|------|------|------|
| 1000+ 基元推理 | 263 µs | 100 µs |
| 约束求解规模 | 100 | 1000+ |
| 任务完成率 | N/A | 85% |
| 用户满意度 | N/A | 4.5/5 |

---

## ⚠️ 风险与挑战

### 技术风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| STEP 解析复杂度超预期 | 中 | 高 | 使用成熟库 (opencascade-sys) |
| 约束求解器收敛困难 | 高 | 高 | 采用混合方法 (符号 + 数值) |
| LLM 推理延迟过高 | 中 | 中 | 本地模型 + 缓存优化 |
| 知识图谱规模爆炸 | 中 | 中 | 分层存储 + 增量更新 |

### 工程风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 代码复杂度激增 | 高 | 中 | 模块化设计 + 严格代码审查 |
| 测试覆盖率下降 | 中 | 中 | 强制 80%+ 覆盖率 |
| 编译时间过长 | 中 | 低 | 增量编译 + 特性标志 |

---

## 🎯 关键决策点

### 决策 1: 是否集成 OpenCASCADE？

**选项 A**: 集成 OpenCASCADE (功能强大，体积大)
- ✅ 完整的 B-Rep 支持
- ✅ 成熟的约束求解器
- ❌ 依赖 C++ 绑定
- ❌ 二进制体积大 (~100MB)

**选项 B**: 纯 Rust 实现 (轻量，功能有限)
- ✅ 无外部依赖
- ✅ 编译快
- ❌ 开发周期长
- ❌ 功能可能不完整

**推荐**: Phase 1 用 OpenCASCADE 快速验证，Phase 3 逐步替换核心算法为 Rust 实现

### 决策 2: LLM 模型选择？

**选项 A**: 云端 API (Qwen/GPT-4)
- ✅ 能力强
- ✅ 无需维护
- ❌ 延迟高
- ❌ 数据隐私

**选项 B**: 本地模型 (Qwen-7B/14B)
- ✅ 低延迟
- ✅ 数据隐私
- ❌ 能力较弱
- ❌ 需 GPU

**推荐**: 混合模式 - 复杂任务用云端，简单任务用本地

---

## 📝 下一步行动

### 立即行动 (本周)

1. [ ] 评估 STEP 解析库 (stepnc vs opencascade-sys)
2. [ ] 设计自主代理接口原型
3. [ ] 创建错误案例库 schema

### 短期行动 (本月)

1. [ ] 实现 STEP 解析器 MVP
2. [ ] 设计对话状态管理数据结构
3. [ ] 编写任务规划器接口定义

### 中期行动 (本季度)

1. [ ] 完成 Phase 1 所有功能
2. [ ] 收集用户反馈调整优先级
3. [ ] 准备论文实验数据

---

## 📚 参考资料

### 开源项目参考

1. **FreeCAD** (Python/C++): 参数化 CAD，开源约束求解器
2. **OpenSCAD**: 脚本驱动 CAD，适合参考 DSL 设计
3. **Compas** (Python): 建筑几何计算框架
4. **CADQuery** (Python): 参数化 CAD 库

### 学术论文

1. **"Deep Learning for CAD Feature Recognition"** (CAD Journal 2025)
2. **"Autonomous Design Agents"** (NeurIPS AI for Science 2025)
3. **"Knowledge Graphs for Engineering Design"** (ASME JMD 2026)

### 商业产品参考

1. **Autodesk Fusion 360**: 生成式设计功能
2. **nTopology**: 隐式建模 + 拓扑优化
3. **Ansys Discovery**: 实时仿真驱动设计

---

*维护者：CadAgent Team | 许可证：MIT*
