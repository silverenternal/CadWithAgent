# CadAgent 自主 CAD 代理实施计划

**创建日期**: 2026-04-06 | **版本**: v1.0

---

## 📋 执行摘要

本文档提供从当前"几何增强 VLM 助手"升级为"自主 CAD 设计代理"的详细实施计划。

### 核心升级

```
当前：用户输入 → LLM 推理 + 几何工具 → 单轮回答
目标：用户意图 → 任务规划 → 多轮执行 → 自我反思 → 知识沉淀
```

---

## 🎯 优先级排序 (MoSCoW 方法)

### Must Have (P0 - 必须实现)

| ID | 功能 | 工时 | 依赖 | 验收标准 |
|----|------|------|------|---------|
| M1 | STEP 格式解析 | 2 周 | - | 可解析标准 STEP 文件，提取几何信息 |
| M2 | 对话状态管理 | 2 周 | - | 支持 10+ 轮对话上下文追踪 |
| M3 | 任务规划器 | 3 周 | M2 | 可分解复杂任务为 5+ 子任务 |
| M4 | 自我反思模块 | 3 周 | M3 | 可检测 80%+ 执行错误 |
| M5 | 错误案例库 | 1 周 | - | 存储 100+ 错误案例，支持检索 |

### Should Have (P1 - 应该实现)

| ID | 功能 | 工时 | 依赖 | 验收标准 |
|----|------|------|------|---------|
| S1 | 3D 特征识别 | 3 周 | M1 | 识别孔、槽、凸台等 10+ 特征 |
| S2 | 参数化约束求解 | 4 周 | - | 求解 100+ 约束系统 |
| S3 | 工具自主选择 | 2 周 | M3, M5 | LLM 自主选择正确工具率>85% |
| S4 | 设计模式知识库 | 3 周 | M5 | 存储 50+ 设计模式 |
| S5 | 用户偏好学习 | 2 周 | S4 | 准确率>80% 预测用户偏好 |

### Could Have (P2 - 可以实现)

| ID | 功能 | 工时 | 依赖 | 验收标准 |
|----|------|------|------|---------|
| C1 | 拓扑优化框架 | 4 周 | S2 | 减重 30%+ 保持结构完整 |
| C2 | 生成式设计引擎 | 4 周 | S2, S3 | 生成 3+ 可行设计方案 |
| C3 | 多目标优化 | 3 周 | C1 | 帕累托前沿 10+ 解 |
| C4 | 实时交互编辑 | 3 周 | S2 | 响应延迟<100ms |

### Won't Have (P3 - 暂不实现)

| ID | 功能 | 原因 | 未来考虑 |
|----|------|------|---------|
| W1 | 点云输入支持 | 非核心需求 | Phase 4 |
| W2 | 材料/工艺推断 | 跨学科复杂 | Phase 4 |
| W3 | 公差分析 | 专业领域深 | Phase 4 |
| W4 | 版本对比/合并 | 工程量大 | Phase 4 |

---

## 📦 Phase 1: 基础增强 (周 1-8)

### 周 1-2: STEP 格式解析

#### 任务分解

```rust
// src/parser/step.rs
//! STEP 格式解析器
//!
//! 支持 ISO 10303-21 (STEP File Format)

use stepnc_rs::step_file::StepFile;
use crate::geometry::primitives::*;
use crate::error::{CadAgentError, CadAgentResult};

pub struct StepParser {
    config: StepConfig,
}

#[derive(Debug, Clone)]
pub struct StepConfig {
    /// 坐标归一化范围
    pub normalize_range: [f64; 2],
    /// 是否启用几何简化
    pub enable_simplification: bool,
    /// 公差设置
    pub tolerance: f64,
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            normalize_range: [0.0, 100.0],
            enable_simplification: false,
            tolerance: 1e-6,
        }
    }
}

impl StepParser {
    pub fn new(config: StepConfig) -> Self {
        Self { config }
    }
    
    pub fn parse(&self, path: &Path) -> CadAgentResult<StepModel> {
        // 1. 解析 STEP 文件
        let step_file = StepFile::read(path)
            .map_err(|e| CadAgentError::parse("STEP", e.to_string()))?;
        
        // 2. 提取几何实体
        let geometry = self.extract_geometry(&step_file)?;
        
        // 3. 构建拓扑关系
        let topology = self.build_topology(&geometry)?;
        
        // 4. 坐标归一化
        let normalized = self.normalize(geometry)?;
        
        Ok(StepModel {
            geometry: normalized,
            topology,
            metadata: self.extract_metadata(&step_file),
        })
    }
    
    fn extract_geometry(&self, step_file: &StepFile) -> CadAgentResult<GeometryData> {
        // 提取：
        // - Advanced_Brep 或 Manifold_Surface_Shape_Representation
        // - Edge_Curve, Face_Surface 等拓扑实体
        // - 参数化曲线曲面 (NURBS, Bezier 等)
        todo!()
    }
}

/// 统一 CAD 数据模型
pub struct UnifiedCadModel {
    pub geometry: GeometryData,
    pub topology: TopologyData,
    pub semantics: SemanticData,
    pub metadata: Metadata,
}
```

#### 测试用例

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_step_part() {
        let parser = StepParser::default();
        let model = parser.parse("data/step/bracket.stp").unwrap();
        
        assert!(!model.geometry.is_empty());
        assert!(model.topology.faces.len() > 0);
    }
    
    #[test]
    fn test_extract_features() {
        let parser = StepParser::default();
        let model = parser.parse("data/step/plate_with_holes.stp").unwrap();
        
        // 应识别出孔特征
        let holes = model.geometry.find_features("hole");
        assert!(holes.len() > 0);
    }
}
```

#### 验收标准

- [ ] 可解析 ISO 10303-21 标准 STEP 文件
- [ ] 提取 B-Rep 几何信息
- [ ] 支持 NURBS 曲线曲面
- [ ] 坐标归一化正确
- [ ] 测试覆盖率>80%

---

### 周 3-4: 对话状态管理

#### 任务分解

```rust
// src/autonomous/dialog_state.rs
//! 对话状态管理器

use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogSession {
    /// 会话 ID
    pub session_id: Uuid,
    /// 用户 ID
    pub user_id: Option<Uuid>,
    /// 当前任务
    pub current_task: Option<Task>,
    /// 对话历史
    pub history: Vec<DialogTurn>,
    /// 上下文变量
    pub context: ContextVariables,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 对话轮次
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogTurn {
    pub turn_id: usize,
    pub user_utterance: String,
    pub agent_response: String,
    pub intent: Option<Intent>,
    pub slots: Vec<Slot>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 意图识别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    DesignOptimization,
    ConflictResolution,
    FeatureQuery,
    DimensionQuery,
    MaterialQuery,
    Custom(String),
}

/// 槽位信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub name: String,
    pub value: serde_json::Value,
    pub confidence: f32,
}

/// 上下文变量
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextVariables {
    /// 当前设计对象
    pub current_design: Option<DesignReference>,
    /// 优化目标
    pub objectives: Vec<Objective>,
    /// 约束条件
    pub constraints: Vec<Constraint>,
    /// 用户反馈
    pub feedback_history: Vec<Feedback>,
    /// 临时变量
    pub temp_vars: HashMap<String, serde_json::Value>,
}

/// 对话状态管理器
pub struct DialogStateManager {
    /// 当前会话
    current_session: Option<DialogSession>,
    /// 会话存储
    session_store: Arc<dyn SessionStore>,
    /// 意图识别器
    intent_classifier: IntentClassifier,
    /// 槽位填充器
    slot_filler: SlotFiller,
}

impl DialogStateManager {
    pub fn new(session_store: Arc<dyn SessionStore>) -> Self {
        Self {
            current_session: None,
            session_store,
            intent_classifier: IntentClassifier::default(),
            slot_filler: SlotFiller::default(),
        }
    }
    
    /// 处理用户输入
    pub async fn process_input(&mut self, input: &str) -> DialogResult {
        // 1. 意图识别
        let intent = self.intent_classifier.classify(input)?;
        
        // 2. 槽位填充
        let slots = self.slot_filler.fill(input, &intent)?;
        
        // 3. 更新上下文
        self.update_context(&intent, &slots)?;
        
        // 4. 记录对话历史
        self.record_turn(input, &intent, &slots)?;
        
        Ok(DialogResult {
            intent,
            slots,
            context: self.get_context(),
        })
    }
    
    /// 获取当前任务
    pub fn get_current_task(&self) -> Option<&Task> {
        self.current_session.as_ref()?.current_task.as_ref()
    }
    
    /// 更新任务状态
    pub fn update_task_status(&mut self, task_id: Uuid, status: TaskStatus) {
        if let Some(session) = &mut self.current_session {
            if let Some(task) = &mut session.current_task {
                if task.id == task_id {
                    task.status = status;
                }
            }
        }
    }
    
    /// 获取对话摘要
    pub fn get_summary(&self) -> DialogSummary {
        // 提取关键信息供 LLM 使用
        todo!()
    }
}
```

#### 测试用例

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_intent_classification() {
        let mut manager = DialogStateManager::new(InMemoryStore::new());
        
        let result = manager.process_input("优化这个零件的重量").await.unwrap();
        assert!(matches!(result.intent, Intent::DesignOptimization));
    }
    
    #[tokio::test]
    async fn test_slot_filling() {
        let mut manager = DialogStateManager::new(InMemoryStore::new());
        
        manager.process_input("优化这个零件的重量").await.unwrap();
        let result = manager.process_input("目标减重 30%").await.unwrap();
        
        assert!(result.slots.iter().any(|s| s.name == "weight_reduction_target"));
    }
    
    #[tokio::test]
    async fn test_context_tracking() {
        let mut manager = DialogStateManager::new(InMemoryStore::new());
        
        // 多轮对话
        manager.process_input("打开 bracket.stp").await.unwrap();
        manager.process_input("检测冲突").await.unwrap();
        manager.process_input("修复它").await.unwrap();
        
        // 上下文应保持
        let context = manager.get_context();
        assert!(context.current_design.is_some());
    }
}
```

---

### 周 5-7: 任务规划器

#### 任务分解

```rust
// src/autonomous/task_planner.rs
//! 任务规划与分解模块

use crate::llm_reasoning::LlmReasoningEngine;
use uuid::Uuid;

/// 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub description: String,
    pub intent: Intent,
    pub slots: Vec<Slot>,
    pub status: TaskStatus,
    pub subtasks: Vec<SubTask>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 子任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: Uuid,
    pub description: String,
    pub tool_name: Option<String>,
    pub arguments: serde_json::Value,
    pub status: SubTaskStatus,
    pub dependencies: Vec<Uuid>,
    pub result: Option<SubTaskResult>,
}

/// 任务规划器
pub struct TaskPlanner {
    llm_engine: LlmReasoningEngine,
    template_library: TaskTemplateLibrary,
    skill_registry: SkillRegistry,
}

impl TaskPlanner {
    pub fn new(llm_engine: LlmReasoningEngine) -> Self {
        Self {
            llm_engine,
            template_library: TaskTemplateLibrary::new(),
            skill_registry: SkillRegistry::new(),
        }
    }
    
    /// 理解任务
    pub async fn understand(&self, input: &str, context: &Context) -> TaskUnderstanding {
        let system_prompt = "你是一个 CAD 设计任务理解专家。分析用户意图，识别任务类型和关键参数。";
        
        let user_prompt = format!(
            "上下文：{}\n用户输入：{}\n\n请分析任务意图和所需参数。",
            context.summary(),
            input
        );
        
        let response = self.llm_engine
            .chat_completions(&[(system_prompt, user_prompt.as_str())])
            .await?;
        
        // 解析 LLM 返回的结构化理解
        self.parse_understanding(&response)
    }
    
    /// 分解任务
    pub fn decompose(&self, understanding: &TaskUnderstanding) -> Vec<SubTask> {
        // 1. 查找匹配的任务模板
        let template = self.template_library
            .get_template(&understanding.task_type);
        
        // 2. 根据模板生成子任务
        if let Some(t) = template {
            return self.instantiate_template(t, understanding);
        }
        
        // 3. 无模板时，使用 LLM 动态生成
        self.generate_plan_with_llm(understanding)
    }
    
    /// 执行规划
    pub async fn execute(&self, task: &Task, state: &mut DialogState) -> TaskResult {
        let mut results = Vec::new();
        
        // 拓扑排序子任务
        let ordered = self.topological_sort(&task.subtasks);
        
        for subtask in ordered {
            // 等待依赖完成
            self.wait_for_dependencies(&subtask, &results).await?;
            
            // 执行子任务
            let result = self.execute_subtask(subtask, state).await?;
            results.push(result);
        }
        
        // 整合结果
        self.synthesize_results(&results, task)
    }
}

/// 任务模板库
pub struct TaskTemplateLibrary {
    templates: HashMap<String, TaskTemplate>,
}

impl TaskTemplateLibrary {
    pub fn new() -> Self {
        let mut lib = Self {
            templates: HashMap::new(),
        };
        
        // 注册内置模板
        lib.register_builtin_templates();
        lib
    }
    
    fn register_builtin_templates(&mut self) {
        // 设计优化模板
        self.templates.insert(
            "design_optimization".to_string(),
            TaskTemplate {
                name: "设计优化",
                steps: vec![
                    TemplateStep {
                        name: "分析当前设计",
                        tool: "cad_analyze_design",
                        arguments: json!({"design": "$current_design"}),
                    },
                    TemplateStep {
                        name: "识别优化空间",
                        tool: "cad_identify_optimization_areas",
                        arguments: json!({"objectives": "$objectives"}),
                    },
                    TemplateStep {
                        name: "生成候选方案",
                        tool: "cad_generate_candidates",
                        arguments: json!({"constraints": "$constraints"}),
                    },
                    TemplateStep {
                        name: "验证几何可行性",
                        tool: "cad_verify_geometry",
                        arguments: json!({"candidates": "$candidates"}),
                    },
                    TemplateStep {
                        name: "评估性能指标",
                        tool: "cad_evaluate_performance",
                        arguments: json!({"candidates": "$candidates"}),
                    },
                    TemplateStep {
                        name: "输出最优方案",
                        tool: "cad_select_best",
                        arguments: json!({"evaluations": "$evaluations"}),
                    },
                ],
            },
        );
        
        // 冲突解决模板
        self.templates.insert(
            "conflict_resolution".to_string(),
            TaskTemplate {
                name: "冲突解决",
                steps: vec![
                    TemplateStep {
                        name: "检测约束冲突",
                        tool: "cad_detect_conflicts",
                        arguments: json!({"constraints": "$constraints"}),
                    },
                    TemplateStep {
                        name: "分析冲突原因",
                        tool: "cad_analyze_conflict_causes",
                        arguments: json!({"conflicts": "$conflicts"}),
                    },
                    TemplateStep {
                        name: "生成修复建议",
                        tool: "cad_generate_fixes",
                        arguments: json!({"conflicts": "$conflicts"}),
                    },
                    TemplateStep {
                        name: "验证修复方案",
                        tool: "cad_verify_fixes",
                        arguments: json!({"fixes": "$fixes"}),
                    },
                ],
            },
        );
    }
}
```

---

### 周 8: 自我反思模块

#### 任务分解

```rust
// src/autonomous/self_reflector.rs
//! 自我反思与修正模块

use crate::cad_verifier::ConstraintVerifier;
use crate::error::CadAgentError;

/// 自我反思器
pub struct SelfReflector {
    /// 验证规则库
    validation_rules: Vec<ValidationRule>,
    /// 几何验证器
    geometry_verifier: ConstraintVerifier,
    /// 错误案例库
    error_library: Arc<ErrorCaseLibrary>,
}

impl SelfReflector {
    pub fn new(error_library: Arc<ErrorCaseLibrary>) -> Self {
        Self {
            validation_rules: Vec::new(),
            geometry_verifier: ConstraintVerifier::default(),
            error_library,
        }
    }
    
    /// 检查执行结果
    pub fn check(&self, result: &ExecutionResult) -> Option<Issue> {
        // 1. 几何验证
        if let Some(geometry) = &result.geometry {
            if let Some(issue) = self.check_geometry(geometry) {
                return Some(issue);
            }
        }
        
        // 2. 约束验证
        if let Some(constraints) = &result.constraints {
            if let Some(issue) = self.check_constraints(constraints) {
                return Some(issue);
            }
        }
        
        // 3. 规则验证
        for rule in &self.validation_rules {
            if !rule.check(result) {
                return Some(Issue {
                    rule_violated: rule.name.clone(),
                    severity: rule.severity,
                    description: rule.description.clone(),
                });
            }
        }
        
        // 4. 基于案例的验证
        if let Some(similar_error) = self.error_library.find_similar(result) {
            return Some(Issue {
                rule_violated: "case_based_warning".to_string(),
                severity: Severity::Warning,
                description: format!(
                    "与历史错误案例相似：{}",
                    similar_error.description
                ),
            });
        }
        
        None
    }
    
    /// 生成修正建议
    pub fn generate_fix(&self, issue: &Issue, result: &ExecutionResult) -> Option<Fix> {
        // 1. 查找历史修复方案
        if let Some(case) = self.error_library.find_similar_fix(issue) {
            return Some(Fix {
                description: case.fix_description.clone(),
                confidence: case.similarity,
                steps: case.fix_steps.clone(),
            });
        }
        
        // 2. 基于规则生成修复
        if let Some(rule) = self.validation_rules
            .iter()
            .find(|r| r.name == issue.rule_violated)
        {
            return rule.suggest_fix(issue, result);
        }
        
        // 3. 使用 LLM 生成修复
        self.generate_fix_with_llm(issue, result)
    }
}

/// 验证规则
pub struct ValidationRule {
    pub name: String,
    pub description: String,
    pub severity: Severity,
    pub check_fn: Box<dyn Fn(&ExecutionResult) -> bool + Send + Sync>,
    pub fix_suggestion: Option<Box<dyn Fn(&Issue, &ExecutionResult) -> Option<Fix>>>,
}

impl ValidationRule {
    pub fn check(&self, result: &ExecutionResult) -> bool {
        (self.check_fn)(result)
    }
}
```

---

## 📦 Phase 2: 自主决策 (周 9-20)

### 周 9-11: 3D 特征识别

```rust
// src/feature_recognition/mod.rs
//! 3D 特征识别模块

use crate::parser::step::BRepModel;

/// 加工特征识别器
pub struct MachiningFeatureRecognizer;

impl MachiningFeatureRecognizer {
    pub fn recognize(&self, brep: &BRepModel) -> Vec<MachiningFeature> {
        let mut features = Vec::new();
        
        // 识别孔特征
        features.extend(self.recognize_holes(brep));
        
        // 识别槽特征
        features.extend(self.recognize_slots(brep));
        
        // 识别凸台特征
        features.extend(self.recognize_bosses(brep));
        
        // 识别倒角/圆角
        features.extend(self.recognize_chamfers_fillet(brep));
        
        features
    }
    
    fn recognize_holes(&self, brep: &BRepModel) -> Vec<HoleFeature> {
        // 基于几何特征识别孔：
        // 1. 查找圆柱面
        // 2. 检查是否为通孔/盲孔
        // 3. 识别孔口倒角
        todo!()
    }
}

/// 设计特征识别
pub struct DesignFeatureRecognizer;

impl DesignFeatureRecognizer {
    pub fn recognize_symmetry(&self, brep: &BRepModel) -> SymmetryInfo {
        // 识别对称面
        let symmetry_planes = self.find_symmetry_planes(brep);
        
        // 识别旋转对称
        let rotational_symmetry = self.find_rotational_symmetry(brep);
        
        SymmetryInfo {
            planes: symmetry_planes,
            axes: rotational_symmetry,
        }
    }
    
    pub fn infer_design_intent(&self, features: &[Feature]) -> DesignIntent {
        // 推断设计意图：
        // - 同轴关系
        // - 均布阵列
        // - 镜像对称
        todo!()
    }
}
```

---

## 📦 Phase 3: 设计优化 (周 21-32)

### 生成式设计引擎

```rust
// src/generative_design/mod.rs
//! 生成式设计模块

use crate::autonomous::TaskPlanner;
use crate::constraint_solver::ParametricSolver;

pub struct GenerativeDesigner {
    llm_engine: LlmReasoningEngine,
    constraint_solver: ParametricSolver,
    evaluator: DesignEvaluator,
}

impl GenerativeDesigner {
    pub fn generate_from_requirements(
        &self,
        requirements: &[FunctionalRequirement],
        constraints: &[Constraint]
    ) -> Vec<DesignCandidate> {
        // 1. LLM 生成初始概念
        let concepts = self.generate_concepts(requirements);
        
        // 2. 参数化实例化
        let candidates: Vec<DesignCandidate> = concepts
            .iter()
            .filter_map(|c| self.instantiate(c, constraints))
            .collect();
        
        // 3. 几何验证
        let valid_candidates = candidates
            .into_iter()
            .filter(|c| self.verify_geometry(c))
            .collect();
        
        valid_candidates
    }
    
    pub fn iterate_design(
        &self,
        current: &DesignCandidate,
        feedback: &str
    ) -> DesignCandidate {
        // 1. 解析反馈
        let modifications = self.parse_feedback(feedback);
        
        // 2. 应用修改
        let modified = self.apply_modifications(current, &modifications);
        
        // 3. 重新验证
        self.verify_and_refine(modified)
    }
}
```

---

## 📦 Phase 4: 知识增强 (周 33-40)

### 知识图谱构建

```rust
// src/knowledge_graph/mod.rs
//! CAD 知识图谱模块

pub struct CadKnowledgeGraph {
    store: GraphDatabase,
    embeddings: EmbeddingModel,
}

impl CadKnowledgeGraph {
    pub fn add_design_knowledge(
        &mut self,
        design: &DesignCandidate,
        outcome: &str
    ) {
        // 提取知识三元组
        let triples = self.extract_knowledge(design, outcome);
        
        // 添加到图谱
        for (subject, predicate, object) in triples {
            self.store.add_triple(subject, predicate, object);
        }
    }
    
    pub fn query(&self, query: &KnowledgeQuery) -> Vec<KnowledgeEntity> {
        // 图谱查询
        todo!()
    }
    
    pub fn similar_designs(&self, query: &DesignCandidate) -> Vec<DesignReference> {
        // 基于图谱的相似设计检索
        todo!()
    }
}
```

---

## 🧪 测试策略

### 单元测试

```rust
#[cfg(test)]
mod autonomous_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_task_decomposition() {
        let planner = TaskPlanner::new(mock_llm());
        let understanding = TaskUnderstanding {
            task_type: "design_optimization".to_string(),
            parameters: json!({"target": "weight", "reduction": 0.3}),
        };
        
        let subtasks = planner.decompose(&understanding);
        
        assert!(subtasks.len() >= 4);
        assert!(subtasks.iter().any(|s| s.tool_name == Some("cad_analyze_design".to_string())));
    }
    
    #[tokio::test]
    async fn test_self_reflection() {
        let reflector = SelfReflector::new(error_lib());
        let result = ExecutionResult {
            geometry: Some(invalid_geometry()),
            ..Default::default()
        };
        
        let issue = reflector.check(&result);
        
        assert!(issue.is_some());
    }
}
```

### 集成测试

```rust
#[cfg(test)]
mod integration_tests {
    use crate::autonomous::CadAgent;
    
    #[tokio::test]
    async fn test_autonomous_optimization() {
        let mut agent = CadAgent::new().await.unwrap();
        
        let task = Task {
            description: "优化这个支架的重量，目标减重 30%".to_string(),
            ..Default::default()
        };
        
        let result = agent.execute_task(&task).await.unwrap();
        
        assert!(result.success);
        assert!(result.weight_reduction >= 0.25); // 至少 25% 减重
    }
}
```

---

## 📊 进度追踪

### 里程碑

| 里程碑 | 目标日期 | 状态 | 验收标准 |
|--------|---------|------|---------|
| M1: STEP 解析完成 | 周 2 结束 | ⏳ | 解析测试通过 |
| M2: 对话状态完成 | 周 4 结束 | ⏳ | 10+ 轮对话测试通过 |
| M3: 任务规划完成 | 周 7 结束 | ⏳ | 复杂任务分解测试通过 |
| M4: 自主代理 MVP | 周 8 结束 | ⏳ | 端到端任务执行成功 |
| M5: Phase 1 完成 | 周 8 结束 | ⏳ | 所有 P0 功能完成 |

### 燃尽图

```
周次     剩余任务
0        ████████████████████████████████ 100%
4        ████████████████████            60%
8        ██████████                      30%
12       ████                            10%
16       █                               3%
20                                       0%
```

---

## 📝 代码审查清单

### 代码质量

- [ ] 所有公共 API 有 rustdoc 文档
- [ ] 关键函数有使用示例
- [ ] 错误类型清晰且可操作
- [ ] 日志记录完整
- [ ] 测试覆盖率>80%

### 架构设计

- [ ] 模块职责清晰
- [ ] 依赖方向合理（高层→低层）
- [ ] 接口抽象合理
- [ ] 无循环依赖

### 性能考虑

- [ ] 大对象使用引用传递
- [ ] 热点路径有基准测试
- [ ] 内存使用合理
- [ ] 并发安全

---

## ⚠️ 风险缓解

### 技术风险

| 风险 | 概率 | 影响 | 缓解措施 | 负责人 |
|------|------|------|---------|--------|
| STEP 解析复杂 | 中 | 高 | 使用成熟库 + 早期验证 | @tokitai |
| LLM 延迟高 | 中 | 中 | 本地模型 + 缓存 | @tokitai |
| 约束求解收敛难 | 高 | 高 | 混合方法 + 超时保护 | @tokitai |

### 进度风险

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|---------|
| 范围蔓延 | 高 | 中 | 严格执行 MoSCoW 优先级 |
| 依赖阻塞 | 中 | 中 | 并行开发 + Mock 接口 |
| 测试不足 | 中 | 高 | CI 强制覆盖率检查 |

---

## 🎯 成功标准

### 功能标准

- [ ] 可解析 STEP/DXF/SVG 格式
- [ ] 可执行 10+ 轮自主对话
- [ ] 可分解并执行复杂任务
- [ ] 可检测并修复 80%+ 错误
- [ ] 测试覆盖率>80%

### 性能标准

- [ ] 单任务执行<30 秒
- [ ] 对话响应<2 秒
- [ ] 内存使用<500MB
- [ ] 并发支持 10+ 会话

### 用户体验标准

- [ ] 自然语言交互流畅
- [ ] 错误提示清晰可操作
- [ ] 进度可视化
- [ ] 结果可解释

---

*维护者：CadAgent Team | 许可证：MIT*
