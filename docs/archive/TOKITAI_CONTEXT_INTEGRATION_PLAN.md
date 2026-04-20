# Tokitai-Context 集成方案

**创建日期**: 2026-04-06 | **版本**: v1.0  
**作者**: Tokitai Team

---

## 📋 执行摘要

### 集成目标

将 `tokitai-context` v0.1.2 深度集成到 CadWithAgent 项目中，实现：

1. **对话状态管理** - 替换现有 LRU 缓存，支持多轮对话上下文追踪
2. **设计分支管理** - 支持多方案探索和合并
3. **错误案例库** - 持久化存储错误模式和解决方案
4. **知识持久化** - 长期记忆设计模式和用户偏好
5. **任务规划追踪** - DAG 记录任务依赖和执行历史

### 核心价值

| 当前架构 | 升级后架构 |
|---------|-----------|
| LRU 缓存 (会话级) | Git 风格分支管理 |
| 单轮对话 | 多轮对话 + 上下文回溯 |
| 无状态工具调用 | 有状态任务规划 |
| 错误日志 (临时) | 错误案例库 (持久化) |
| 线性执行流 | DAG 任务图 + 并行分支 |

---

## 🏗️ 架构设计

### 集成层次

```
┌─────────────────────────────────────────────────────────────────┐
│                    CadWithAgent 应用层                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │
│  │ 自主代理层        │  │ 知识/记忆层       │  │ 设计/优化层   │  │
│  │ Autonomous Agent │  │ Knowledge/Memory │  │ Design/Opt   │  │
│  └────────┬─────────┘  └────────┬─────────┘  └──────┬───────┘  │
│           │                     │                    │          │
│           └─────────────────────┼────────────────────┘          │
│                                 │                                │
│                    ┌────────────▼────────────┐                   │
│                    │  Tokitai-Context Core   │                   │
│                    │  ┌───────────────────┐  │                   │
│                    │  │ ParallelContext   │  │  Git 风格分支     │
│                    │  │ DialogStateManager│  │  对话状态管理     │
│                    │  │ ErrorCaseLibrary  │  │  错误案例库       │
│                    │  │ KnowledgeGraph    │  │  知识图谱         │
│                    │  │ TaskPlanner       │  │  任务规划器       │
│                    │  └───────────────────┘  │                   │
│                    └─────────────────────────┘                   │
│                                 │                                │
│           ┌─────────────────────┼────────────────────┐           │
│           │                     │                    │           │
│  ┌────────▼─────────┐  ┌────────▼─────────┐  ┌──────▼───────┐  │
│  │ 理解/推理层       │  │ 解析/输入层       │  │ 工具层        │  │
│  │ Understanding    │  │ Parsing/Input    │  │ Tools        │  │
│  └──────────────────┘  └──────────────────┘  └──────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 模块映射

| CadWithAgent 模块 | Tokitai-Context 集成点 | 说明 |
|------------------|----------------------|------|
| `src/memory/` | → `tokitai_context::facade::Context` | 替换 LRU 缓存 |
| `src/llm_reasoning/` | → `tokitai_context::parallel::ParallelContextManager` | 分支对话历史 |
| `src/analysis/` | → `tokitai_context::facade::Layer::LongTerm` | 持久化分析结果 |
| `src/feature/` | → `tokitai_context::parallel::branch` | 设计分支管理 |
| `src/cad_verifier/` | → `tokitai_context::ai::resolver` | 冲突解决 |
| **NEW** | → `src/context/` | 新增上下文管理模块 |

---

## 📦 依赖配置

### Cargo.toml 更新

```toml
[dependencies]
# 现有 tokitai 依赖
tokitai = "0.4.0"
tokitai-core = "0.4.0"

# 新增：tokitai-context (Git 风格上下文管理)
tokitai-context = { version = "0.1.2", features = ["core", "wal", "ai"] }

# 可选：分布式协调 (Phase 4)
# tokitai-context = { version = "0.1.2", features = ["full"] }
```

### Feature 选择策略

| Feature | 是否启用 | 说明 |
|---------|---------|------|
| `core` | ✅ | 核心存储功能 (必需) |
| `wal` | ✅ | 写前日志 + 崩溃恢复 (推荐) |
| `ai` | ✅ | AI 冲突解决 + 语义搜索 (推荐) |
| `distributed` | ⏸️ | Phase 4 分布式协调 |
| `fuse` | ❌ | FUSE 文件系统 (不需要) |
| `benchmarks` | ⏸️ | 开发阶段使用 |
| `metrics` | ⏸️ | 生产监控 (可选) |

---

## 🔧 核心实现

### 1. DialogStateManager (对话状态管理器)

**定位**: 替换现有 LRU 缓存，支持多轮对话上下文追踪

```rust
// src/context/dialog_state.rs
//! 对话状态管理器
//!
//! 基于 tokitai-context 实现多轮对话上下文追踪
//!
//! # 特性
//!
//! - Git 风格分支管理：每个对话线程作为一个分支
//! - 分层存储：Transient (临时)/ShortTerm (短期)/LongTerm (长期)
//! - 语义搜索：基于 SimHash 的上下文检索
//! - 崩溃恢复：WAL 保证对话数据不丢失

use tokitai_context::facade::{Context, ContextConfig, Layer, ContextItem, SearchHit};
use tokitai_context::parallel::{ParallelContextManager, ParallelContextManagerConfig};
use crate::llm_reasoning::types::ChainOfThought;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 对话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogState {
    /// 对话 ID
    pub dialog_id: String,
    /// 当前分支
    pub current_branch: String,
    /// 对话轮数
    pub turn_count: usize,
    /// 当前任务
    pub current_task: Option<String>,
    /// 上下文摘要
    pub context_summary: Option<String>,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogMessage {
    /// 消息 ID
    pub id: String,
    /// 角色 (user/assistant/system)
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: u64,
    /// 关联的 CAD 文件 (可选)
    pub cad_file: Option<String>,
    /// 工具调用链 (可选)
    pub tool_chain: Option<String>,
}

/// 对话状态管理器
pub struct DialogStateManager {
    /// 上下文存储
    ctx: Context,
    /// 并行上下文管理器 (分支管理)
    parallel_manager: ParallelContextManager,
    /// 当前会话 ID
    current_session: String,
    /// 当前分支
    current_branch: String,
    /// 配置
    config: DialogStateConfig,
}

/// 配置
#[derive(Debug, Clone)]
pub struct DialogStateConfig {
    /// 最大短期对话轮数
    pub max_short_term_turns: usize,
    /// 启用 FileKV 后端
    pub enable_filekv: bool,
    /// 启用语义搜索
    pub enable_semantic_search: bool,
    /// 上下文根目录
    pub context_root: String,
}

impl Default for DialogStateConfig {
    fn default() -> Self {
        Self {
            max_short_term_turns: 20,
            enable_filekv: true,
            enable_semantic_search: true,
            context_root: "./.cad_context".to_string(),
        }
    }
}

impl DialogStateManager {
    /// 创建新的对话状态管理器
    pub fn new(session_id: &str) -> crate::error::CadAgentResult<Self> {
        Self::with_config(session_id, DialogStateConfig::default())
    }

    /// 使用自定义配置创建
    pub fn with_config(
        session_id: &str,
        config: DialogStateConfig,
    ) -> crate::error::CadAgentResult<Self> {
        let context_root = &config.context_root;

        // 初始化 ContextConfig
        let ctx_config = ContextConfig {
            max_short_term_rounds: config.max_short_term_turns,
            enable_filekv_backend: config.enable_filekv,
            enable_semantic_search: config.enable_semantic_search,
            ..Default::default()
        };

        // 打开上下文存储
        let ctx = Context::open_with_config(context_root, ctx_config)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        // 初始化并行管理器
        let parallel_config = ParallelContextManagerConfig {
            context_root: std::path::PathBuf::from(context_root),
            ..Default::default()
        };

        let parallel_manager = ParallelContextManager::new(parallel_config)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        Ok(Self {
            ctx,
            parallel_manager,
            current_session: session_id.to_string(),
            current_branch: "main".to_string(),
            config,
        })
    }

    /// 添加用户消息
    pub fn add_user_message(&mut self, message: &str) -> crate::error::CadAgentResult<String> {
        let msg = DialogMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: "user".to_string(),
            content: message.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cad_file: None,
            tool_chain: None,
        };

        let content = serde_json::to_vec(&msg)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        let hash = self.ctx.store(
            &self.current_session,
            &content,
            Layer::ShortTerm,
        )
        .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        Ok(hash)
    }

    /// 添加助手响应
    pub fn add_assistant_response(
        &mut self,
        response: &str,
        tool_chain: Option<&str>,
    ) -> crate::error::CadAgentResult<String> {
        let msg = DialogMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: "assistant".to_string(),
            content: response.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            cad_file: None,
            tool_chain: tool_chain.map(|s| s.to_string()),
        };

        let content = serde_json::to_vec(&msg)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        let hash = self.ctx.store(
            &self.current_session,
            &content,
            Layer::ShortTerm,
        )
        .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        Ok(hash)
    }

    /// 获取最近 N 轮对话
    pub fn get_recent_turns(&self, n: usize) -> crate::error::CadAgentResult<Vec<DialogMessage>> {
        // TODO: 实现从 Context 中检索并反序列化
        todo!()
    }

    /// 语义搜索上下文
    pub fn search_context(&self, query: &str) -> crate::error::CadAgentResult<Vec<SearchHit>> {
        let hits = self.ctx.search(&self.current_session, query)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;
        Ok(hits)
    }

    /// 创建对话分支 (用于多方案探索)
    pub fn create_branch(&mut self, branch_name: &str) -> crate::error::CadAgentResult<()> {
        // TODO: 使用 parallel_manager 创建分支
        todo!()
    }

    /// 切换到分支
    pub fn checkout_branch(&mut self, branch_name: &str) -> crate::error::CadAgentResult<()> {
        // TODO: 切换分支
        todo!()
    }

    /// 获取当前对话状态
    pub fn get_state(&self) -> DialogState {
        DialogState {
            dialog_id: self.current_session.clone(),
            current_branch: self.current_branch.clone(),
            turn_count: 0, // TODO: 实现计数
            current_task: None,
            context_summary: None,
        }
    }

    /// 清理会话
    pub fn cleanup_session(&mut self) -> crate::error::CadAgentResult<()> {
        self.ctx.cleanup_session(&self.current_session)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;
        Ok(())
    }
}
```

---

### 2. ErrorCaseLibrary (错误案例库)

**定位**: 持久化存储错误模式和解决方案，支持自我反思

```rust
// src/context/error_library.rs
//! 错误案例库
//!
//! 基于 tokitai-context 的 LongTerm 层持久化存储错误模式

use tokitai_context::facade::{Context, Layer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 错误案例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCase {
    /// 错误 ID
    pub id: String,
    /// 错误类型
    pub error_type: String,
    /// 错误描述
    pub description: String,
    /// 触发场景
    pub trigger_scenario: String,
    /// 根本原因
    pub root_cause: String,
    /// 解决方案
    pub solution: String,
    /// 预防措施
    pub prevention: String,
    /// 相关工具
    pub related_tools: Vec<String>,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 发生次数
    pub occurrence_count: u32,
}

/// 错误案例库
pub struct ErrorCaseLibrary {
    ctx: Context,
    cache: HashMap<String, ErrorCase>,
}

impl ErrorCaseLibrary {
    /// 创建错误案例库
    pub fn new(context_root: &str) -> crate::error::CadAgentResult<Self> {
        let ctx = Context::open(format!("{}/errors", context_root))
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        Ok(Self {
            ctx,
            cache: HashMap::new(),
        })
    }

    /// 添加错误案例
    pub fn add_case(&mut self, case: ErrorCase) -> crate::error::CadAgentResult<String> {
        let content = serde_json::to_vec(&case)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        let hash = self.ctx.store("error_library", &content, Layer::LongTerm)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        self.cache.insert(case.id.clone(), case);

        Ok(hash)
    }

    /// 根据错误类型检索案例
    pub fn find_by_type(&self, error_type: &str) -> Vec<ErrorCase> {
        self.cache
            .values()
            .filter(|c| c.error_type == error_type)
            .cloned()
            .collect()
    }

    /// 语义搜索相似案例
    pub fn search_similar(&self, query: &str) -> crate::error::CadAgentResult<Vec<ErrorCase>> {
        // TODO: 使用语义搜索
        todo!()
    }

    /// 记录错误发生
    pub fn record_occurrence(&mut self, error_id: &str) {
        if let Some(case) = self.cache.get_mut(error_id) {
            case.occurrence_count += 1;
        }
    }
}
```

---

### 3. TaskPlanner (任务规划器)

**定位**: 使用 DAG 管理任务依赖和执行历史

```rust
// src/context/task_planner.rs
//! 任务规划器
//!
//! 基于 tokitai-context 的 DAG 上下文图管理任务依赖

use tokitai_context::parallel::{ParallelContextManager, ParallelContextManagerConfig};
use serde::{Deserialize, Serialize};

/// 任务节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
    pub result: Option<String>,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

/// 任务规划器
pub struct TaskPlanner {
    manager: ParallelContextManager,
    current_plan_id: String,
}

impl TaskPlanner {
    pub fn new(context_root: &str) -> crate::error::CadAgentResult<Self> {
        let config = ParallelContextManagerConfig {
            context_root: std::path::PathBuf::from(context_root),
            ..Default::default()
        };

        let manager = ParallelContextManager::new(config)
            .map_err(|e| crate::error::CadAgentError::Internal(e.to_string()))?;

        Ok(Self {
            manager,
            current_plan_id: "default".to_string(),
        })
    }

    /// 创建任务计划
    pub fn create_plan(&mut self, plan_id: &str) -> crate::error::CadAgentResult<()> {
        // TODO: 创建计划分支
        self.current_plan_id = plan_id.to_string();
        Ok(())
    }

    /// 添加任务
    pub fn add_task(&mut self, task: TaskNode) -> crate::error::CadAgentResult<()> {
        // TODO: 添加到上下文
        todo!()
    }

    /// 执行任务
    pub fn execute_task(&mut self, task_id: &str) -> crate::error::CadAgentResult<String> {
        // TODO: 执行任务并记录结果
        todo!()
    }

    /// 获取任务状态
    pub fn get_task_status(&self, task_id: &str) -> Option<TaskStatus> {
        // TODO: 查询任务状态
        None
    }
}
```

---

## 📁 目录结构

```
src/
├── context/                    # NEW: 上下文管理模块 (基于 tokitai-context)
│   ├── mod.rs
│   ├── dialog_state.rs         # 对话状态管理器
│   ├── error_library.rs        # 错误案例库
│   ├── task_planner.rs         # 任务规划器
│   └── knowledge_graph.rs      # 知识图谱 (Phase 2)
├── memory/                     # EXISTING: 内存优化 (保留)
│   ├── mod.rs
│   ├── arena.rs
│   └── pool.rs
├── llm_reasoning/              # EXISTING: LLM 推理 (集成)
│   ├── mod.rs
│   ├── engine.rs
│   └── types.rs
└── ...                         # 其他现有模块
```

---

## 🧪 测试策略

### 单元测试

```rust
// src/context/dialog_state.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_user_message() {
        let temp_dir = tempdir().unwrap();
        let config = DialogStateConfig {
            context_root: temp_dir.path().to_str().unwrap().to_string(),
            ..Default::default()
        };

        let mut manager = DialogStateManager::with_config("test-session", config).unwrap();
        let hash = manager.add_user_message("你好，帮我分析这个 CAD 图纸").unwrap();
        
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_multi_turn_dialog() {
        // 测试多轮对话上下文追踪
    }

    #[test]
    fn test_branch_creation() {
        // 测试设计分支创建
    }

    #[test]
    fn test_semantic_search() {
        // 测试语义搜索
    }

    #[test]
    fn test_crash_recovery() {
        // 测试 WAL 崩溃恢复
    }
}
```

### 集成测试

```rust
// tests/context_integration.rs

#[test]
fn test_dialog_state_with_llm_reasoning() {
    // 测试 DialogStateManager 与 LLM 推理的集成
}

#[test]
fn test_error_library_learning() {
    // 测试错误案例库从错误中学习
}

#[test]
fn test_task_planner_execution() {
    // 测试任务规划器执行复杂任务
}
```

---

## 📊 性能基准

### 对比测试

| 操作 | 当前 LRU 缓存 | tokitai-context | 提升 |
|------|-------------|-----------------|------|
| 添加消息 | ~1μs | ~6ms | - |
| 检索上下文 | ~10μs | ~2ms (语义搜索~50ms) | 功能增强 |
| 创建分支 | N/A | ~6ms (O(1)) | 新功能 |
| 合并分支 | N/A | ~45ms | 新功能 |
| 崩溃恢复 | ❌ 不支持 | ~100ms (WAL) | 新能力 |

**注**: tokitai-context 单次操作延迟略高，但提供了 LRU 无法实现的高级功能

---

## 🚀 实施计划

### Phase 1: 基础集成 (周 1-2)

- [ ] 更新 `Cargo.toml` 添加 `tokitai-context` 依赖
- [ ] 创建 `src/context/` 模块结构
- [ ] 实现 `DialogStateManager` 基础功能
- [ ] 编写单元测试
- [ ] 集成到 `llm_reasoning` 模块

### Phase 2: 错误案例库 (周 3)

- [ ] 实现 `ErrorCaseLibrary`
- [ ] 与 `cad_verifier` 模块集成
- [ ] 添加错误自动记录功能
- [ ] 实现语义搜索

### Phase 3: 任务规划器 (周 4-6)

- [ ] 实现 `TaskPlanner`
- [ ] 与 `analysis` 模块集成
- [ ] 支持任务分解和依赖管理
- [ ] 添加任务执行追踪

### Phase 4: 高级功能 (周 7-8)

- [ ] AI 冲突解决 (使用 `tokitai-context` 的 `ai` feature)
- [ ] 知识图谱初步实现
- [ ] 性能优化和基准测试
- [ ] 文档完善

---

## 🔗 与现有模块集成

### 1. 与 `llm_reasoning` 集成

```rust
// src/llm_reasoning/engine.rs
use crate::context::DialogStateManager;

pub struct LlmReasoningEngine {
    // ... existing fields ...
    dialog_state: DialogStateManager,  // NEW
}

impl LlmReasoningEngine {
    pub fn reason(&mut self, request: LlmReasoningRequest) -> CadAgentResult<LlmReasoningResponse> {
        // 1. 记录用户输入
        self.dialog_state.add_user_message(&request.task)?;

        // 2. 执行推理
        let response = self.execute_reasoning(request)?;

        // 3. 记录助手响应
        self.dialog_state.add_assistant_response(
            &response.chain_of_thought.answer,
            Some(&response.tools_used),
        )?;

        Ok(response)
    }
}
```

### 2. 与 `cad_verifier` 集成

```rust
// src/cad_verifier/verifier.rs
use crate::context::ErrorCaseLibrary;

pub struct ConstraintVerifier {
    // ... existing fields ...
    error_library: ErrorCaseLibrary,  // NEW
}

impl ConstraintVerifier {
    pub fn verify(&mut self, constraints: &[Constraint]) -> CadAgentResult<VerificationResult> {
        match self.execute_verification(constraints) {
            Ok(result) => Ok(result),
            Err(e) => {
                // 记录错误案例
                self.error_library.add_case(ErrorCase {
                    id: uuid::Uuid::new_v4().to_string(),
                    error_type: "constraint_conflict".to_string(),
                    description: e.to_string(),
                    // ...
                })?;
                Err(e)
            }
        }
    }
}
```

---

## ⚠️ 风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| 学习曲线 | 中 | 高 | 详细文档 + 示例代码 |
| 存储开销 | 低 | 中 | COW 开销 ~18%，可接受 |
| 性能影响 | 中 | 中 | 异步操作 + 缓存优化 |
| 依赖复杂度 | 低 | 低 | 纯 Rust，无 C/C++ 依赖 |

---

## 📚 参考资料

1. [tokitai-context API 文档](https://docs.rs/tokitai-context)
2. [tokitai-context GitHub](https://github.com/silverenternal/tokitai)
3. [AI_AUTONOMY_GAP_ANALYSIS.md](./AI_AUTONOMY_GAP_ANALYSIS.md)
4. [AUTONOMOUS_CAD_IMPLEMENTATION_PLAN.md](./AUTONOMOUS_CAD_IMPLEMENTATION_PLAN.md)

---

## ✅ 验收标准

- [ ] `DialogStateManager` 支持 20+ 轮对话上下文追踪
- [ ] `ErrorCaseLibrary` 存储 100+ 错误案例
- [ ] `TaskPlanner` 可分解 5+ 子任务的复杂任务
- [ ] 所有单元测试通过
- [ ] 集成测试通过
- [ ] 性能基准达标
- [ ] 文档完整

---

**下一步**: 开始 Phase 1 实施，创建 `src/context/` 模块结构
