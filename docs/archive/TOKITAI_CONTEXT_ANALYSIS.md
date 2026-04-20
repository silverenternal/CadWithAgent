# Tokitai-Context v0.1.2 Analysis

## 概述

**Tokitai-Context** 是一个专为 AI Agent 设计的 Git 风格并行上下文管理系统。它提供了高效的分支、合并和冲突解决能力，适用于需要维护多个并行对话/任务上下文的场景。

**仓库**: https://github.com/silverenternal/tokitai  
**Crate**: https://crates.io/crates/tokitai-context/0.1.2  
**文档**: https://docs.rs/tokitai-context/0.1.2

---

## 核心特性

### 1. Git 风格分支管理
- **O(1) 分支创建**: 使用写时复制 (COW) 和符号链接实现瞬时分支
- **6 种合并策略**: FastForward, SelectiveMerge, AIAssisted, ThreeWayMerge 等
- **冲突检测与解决**: AI 驱动的冲突检测和自动解决
- **DAG 上下文图**: 完整的分支历史和血缘追踪

### 2. 分层存储架构
```rust
pub enum Layer {
    Transient,   // 临时内容，会话清理时删除
    ShortTerm,   // 最近内容，自动修剪到 N 轮
    LongTerm,    // 永久内容 (规则、配置、模式)
}
```

### 3. 高性能存储引擎
- **FileKV 后端**: LSM-Tree 架构 (MemTable + Segment + BlockCache)
- **写前日志 (WAL)**: 崩溃恢复支持
- **多级缓存**: BlockCache (64MB 默认), MemTable (4MB 刷新阈值)
- **增量哈希链**: 快照和回滚支持

### 4. AI 增强功能 (需 `ai` feature)
- **AI 冲突解决**: 使用 LLM 自动解决分支合并冲突
- **分支目的推断**: 基于内容自动推断分支用途
- **智能合并推荐**: AI 评估合并风险并给出建议
- **分支摘要生成**: 自动生成变更摘要

### 5. 高级功能
- **MVCC**: 多版本并发控制
- **列族支持**: 类似 RocksDB 的列族管理
- **PITR**: 时间点恢复
- **一致性检查**: 数据完整性验证
- **Prometheus 指标**: 完整的监控指标导出
- **审计日志**: 合规性审计追踪

---

## 架构概览

```
.context/
├── branches/        # 分支元数据和内容
├── graph.json       # 上下文图 (DAG)
├── merge_logs/      # 合并历史
├── checkpoints/     # 保存的检查点
├── cow_store/       # 写时复制存储
├── sessions/        # 会话数据
│   └── {session_id}/
│       ├── transient/
│       ├── short-term/
│       └── long-term/
├── hashes/          # 哈希索引
├── logs/            # 操作日志
└── filekv/          # FileKV 后端
    ├── segments/    # LSM 段文件
    ├── wal/         # 写前日志
    ├── index/       # 稀疏索引
    └── checkpoints/ # 检查点
```

---

## 核心 API

### 基础用法 (Facade API)

```rust
use tokitai_context::facade::{Context, ContextConfig, Layer};

// 打开上下文存储
let mut ctx = Context::open("./.context")?;

// 存储内容
let hash = ctx.store("session-1", b"Hello, World!", Layer::ShortTerm)?;

// 检索内容
let item = ctx.retrieve("session-1", &hash)?;
println!("Content: {:?}", String::from_utf8_lossy(&item.content));

// 批量存储
let entries = vec![
    (b"data 1".as_slice(), Layer::ShortTerm),
    (b"data 2".as_slice(), Layer::LongTerm),
];
let hashes = ctx.store_batch("session-1", &entries)?;

// 语义搜索
let hits = ctx.search("session-1", "hello world")?;
for hit in hits {
    println!("Found: {} (score: {})", hit.hash, hit.score);
}

// 删除内容
ctx.delete("session-1", &hash)?;

// 清理整个会话
ctx.cleanup_session("session-1")?;

// 完整性检查和恢复
let report = ctx.recover()?;
println!("Health: {}", report.is_healthy);
```

### 高级用法 (并行上下文管理)

```rust
use tokitai_context::{
    ParallelContextManager, 
    ParallelContextManagerConfig,
    parallel::branch::MergeStrategy,
};

let config = ParallelContextManagerConfig {
    context_root: std::path::PathBuf::from(".context"),
    default_merge_strategy: MergeStrategy::AIAssisted,
    ..Default::default()
};

let mut manager = ParallelContextManager::new(config)?;

// 创建新分支 (从 main 分支创建 feature 分支)
let branch = manager.create_branch("feature", "main")?;

// 切换到分支
manager.checkout(&branch.branch_id)?;

// 在分支中添加内容
manager.add_content("data on feature branch", Layer::ShortTerm)?;

// 切换回 main
manager.checkout("main")?;

// 合并分支 (使用 AI 辅助解决冲突)
let result = manager.merge("feature", "main", Some(MergeStrategy::AIAssisted))?;
println!("Merge completed: {}", result.success);
```

### AI 增强用法 (需 `ai` feature)

```rust
use tokitai_context::facade::{Context, AIContext};
use tokitai_context::ai::client::LLMClient;
use std::sync::Arc;

// 创建 LLM 客户端
let llm_client: Arc<dyn LLMClient> = Arc::new(/* your LLM client */);

// 打开上下文并包装为 AIContext
let mut ctx = Context::open("./.context")?;
let mut ai_ctx = AIContext::new(&mut ctx, Arc::clone(&llm_client));

// AI 辅助合并
let merge_result = ai_ctx.merge_with_ai("feature", "main").await?;

// 推断分支目的
let purpose = ai_ctx.infer_branch_purpose("feature").await?;
println!("Branch purpose: {}", purpose.summary);

// 获取合并建议
let recommendation = ai_ctx.get_merge_recommendation("feature", "main").await?;
println!("Merge risk: {:?}", recommendation.risk_level);

// 生成分支摘要
let summary = ai_ctx.summarize_branch("feature").await?;
println!("Summary: {}", summary.text);

// AI 解决具体冲突
let resolution = ai_ctx.resolve_conflict(
    "conflict-123",
    "feature",
    "main",
    source_content,
    target_content,
).await?;
```

---

## 配置选项

### ContextConfig

```rust
pub struct ContextConfig {
    /// 最大保留短期轮数
    pub max_short_term_rounds: usize,  // 默认：10
    /// 启用内存映射文件 I/O
    pub enable_mmap: bool,              // 默认：true
    /// 启用操作日志
    pub enable_logging: bool,           // 默认：true
    /// 启用语义搜索
    pub enable_semantic_search: bool,   // 默认：true
    /// 启用 FileKV 后端 (高性能)
    pub enable_filekv_backend: bool,    // 默认：false
    /// MemTable 刷新阈值 (字节)
    pub memtable_flush_threshold_bytes: usize,  // 默认：4MB
    /// BlockCache 大小 (字节)
    pub block_cache_size_bytes: usize,          // 默认：64MB
}
```

### FileKVConfig (底层配置)

```rust
pub struct FileKVConfig {
    // 存储路径
    pub segment_dir: PathBuf,
    pub wal_dir: PathBuf,
    pub index_dir: PathBuf,
    
    // WAL 配置
    pub enable_wal: bool,
    pub wal_max_size_bytes: usize,      // 默认：100MB
    pub wal_max_files: usize,           // 默认：5
    
    // Bloom 过滤器
    pub enable_bloom: bool,
    
    // 后台刷新
    pub enable_background_flush: bool,
    pub background_flush_interval_ms: u64,  // 默认：100ms
    
    // 段预分配
    pub segment_preallocate_size: usize,    // 默认：16MB
    
    // MemTable 配置
    pub memtable: MemTableConfig {
        flush_threshold_bytes: usize,       // 默认：4MB
        max_entries: usize,                 // 默认：100,000
        max_memory_bytes: usize,            // 默认：64MB
    },
    
    // BlockCache 配置
    pub cache: BlockCacheConfig {
        max_memory_bytes: usize,            // 默认：64MB
        max_items: usize,                   // 默认：10,000
        min_block_size: usize,              // 默认：64
        max_block_size: usize,              // 默认：1MB
    },
    
    // 压缩配置
    pub compression: DictionaryCompressionConfig::default(),
    
    // 写合并
    pub write_coalescing_enabled: bool,     // 默认：true
    
    // 缓存预热
    pub cache_warming_enabled: bool,        // 默认：true
    
    // 异步 I/O (生产环境可选)
    pub async_io_enabled: bool,             // 默认：false
    
    // 检查点
    pub checkpoint_dir: PathBuf,
    
    // 审计日志 (合规性)
    pub audit_log: AuditLogConfig::default(),
}
```

---

## Feature Flags

```toml
[dependencies]
tokitai-context = { version = "0.1.2", features = ["core", "wal", "ai"] }
```

| Feature | 说明 | 依赖 |
|---------|------|------|
| `default` | 默认启用 WAL | `wal` |
| `core` | 核心存储功能 | 无 |
| `wal` | 写前日志和崩溃恢复 | 无 |
| `ai` | AI 增强功能 (冲突解决、语义搜索) | `reqwest`, `jsonschema`, `dotenvy` |
| `benchmarks` | 性能基准测试套件 | `criterion` |
| `distributed` | 分布式协调 (etcd) | `etcd-client`, `tokio-stream` |
| `fuse` | FUSE 文件系统挂载 | `fuser`, `libc` |
| `metrics` | Prometheus 指标导出 | `prometheus`, `metrics` |
| `full` | 启用所有功能 | 以上全部 |

---

## 性能指标

根据官方数据:

| 操作 | 延迟 | 说明 |
|------|------|------|
| Fork (分支创建) | ~6ms | O(1) 通过符号链接 |
| Merge (合并) | ~45ms | 平均值 |
| Checkout (切换) | ~2ms | 快速切换 |
| 存储开销 | ~18% | COW 语义额外开销 |

---

## 类型映射

### 核心类型

```rust
// 上下文存储门面
pub struct Context {
    service: InternalService,
    root: PathBuf,
    filekv: Option<Arc<FileKV>>,
    use_filekv: bool,
}

// 上下文项
pub struct ContextItem {
    pub hash: String,
    pub content: Vec<u8>,
    pub summary: Option<String>,
}

// 搜索结果
pub struct SearchHit {
    pub hash: String,
    pub score: f32,        // 0.0 - 1.0
    pub summary: Option<String>,
}

// 统计信息
pub struct ContextStats {
    pub sessions_count: usize,
    pub items_count: usize,
    pub total_size_bytes: u64,
    pub cache_hit_rate: f32,
}

// 恢复报告
pub struct RecoveryReport {
    pub is_healthy: bool,
    pub files_scanned: usize,
    pub hash_index_exists: bool,
    pub symlinks_count: usize,
    pub path_files_count: usize,
    pub log_exists: bool,
    pub log_entries: u64,
}
```

### 错误类型

```rust
pub enum ErrorCategory {
    NotFound,
    Corruption,
    Conflict,
    ResourceExhausted,
    InvalidArgument,
    Internal,
}

pub enum RecoveryAction {
    None,
    RebuildIndex,
    ReplayWal,
    RestoreFromCheckpoint,
    ManualIntervention,
}

pub type Result<T> = std::result::Result<T, ContextError>;
```

---

## 模块结构

```
tokitai-context/
├── core/              # 核心存储
│   ├── file_service   # 文件服务
│   ├── layers         # 存储层管理
│   ├── hash_index     # 哈希索引
│   ├── logger         # 操作日志
│   ├── hash_chain     # 增量哈希链
│   ├── semantic_index # SimHash 语义索引
│   └── knowledge_*    # 知识索引和监控
├── parallel/          # 并行上下文
│   ├── branch         # 分支管理
│   ├── graph          # DAG 上下文图
│   ├── merge          # 合并策略
│   ├── manager        # 并行管理器
│   └── cow            # 写时复制存储
├── optimization/      # 性能优化
│   ├── cache          # 多级缓存
│   ├── compression    # 压缩算法
│   ├── dedup          # 去重
│   └── algorithms     # 优化算法
├── ai/                # AI 功能 (feature-gated)
│   ├── resolver       # 冲突解决
│   ├── purpose        # 目的推断
│   ├── smart_merge    # 智能合并推荐
│   ├── summarizer     # 摘要生成
│   └── client         # LLM 客户端 trait
├── wal/               # 写前日志
├── file_kv/           # FileKV 存储引擎
│   ├── async_io       # 异步 I/O
│   ├── memtable       # 内存表
│   ├── segment        # 段文件
│   └── block_cache    # 块缓存
├── mvcc/              # 多版本并发控制
├── pitr/              # 时间点恢复
├── consistency_check/ # 一致性检查
├── crash_recovery/    # 崩溃恢复
├── query_optimizer/   # 查询优化器
├── auto_tuner/        # 自动调优
├── distributed_coordination/ # 分布式协调 (feature-gated)
├── column_family/     # 列族管理
├── fuse_fs/           # FUSE 文件系统 (feature-gated)
└── facade/            # 简化门面 API
```

---

## 与 CadWithAgent 项目的适用性分析

### ✅ 优势

1. **纯 Rust 实现**: 符合项目技术栈选择，无外部 C/C++ 依赖
2. **成熟的存储架构**: LSM-Tree + WAL + MVCC，生产级可靠性
3. **Git 风格分支**: 非常适合 CAD 设计的多方案探索场景
4. **AI 集成友好**: 内置 LLM 客户端 trait，易于集成现有 LLM 模块
5. **高性能**: O(1) 分支、多级缓存、写合并等优化
6. **崩溃恢复**: WAL 和检查点机制保证数据安全
7. **可观测性**: Prometheus 指标、审计日志、追踪配置

### 🎯 适用场景

1. **多轮对话状态管理**: 使用 `Layer::ShortTerm` 存储对话历史
2. **设计分支管理**: 每个设计方案作为一个分支，支持合并和比较
3. **错误案例库**: 使用 `Layer::LongTerm` 持久化存储错误模式
4. **任务规划追踪**: 使用 DAG 记录任务依赖和执行历史
5. **知识持久化**: 使用语义搜索检索历史设计模式

### ⚠️ 潜在限制

1. **学习曲线**: 完整功能需要理解 LSM-Tree、MVCC 等概念
2. **存储开销**: COW 语义带来 ~18% 额外开销
3. **AI 功能依赖**: `ai` feature 需要额外依赖 (`reqwest`, `jsonschema`)
4. **文档完整性**: 部分高级功能文档不够详细

### 🔧 推荐集成方案

```toml
# Cargo.toml
[dependencies]
tokitai-context = { 
    version = "0.1.2", 
    features = ["core", "wal", "ai"]  # 基础 + 崩溃恢复 + AI 增强
}
```

```rust
// 在 CadWithAgent 中的使用示例
use tokitai_context::facade::{Context, ContextConfig, Layer};

pub struct DialogStateManager {
    ctx: Context,
    current_session: String,
}

impl DialogStateManager {
    pub fn new(session_id: &str) -> Result<Self> {
        let config = ContextConfig {
            max_short_term_rounds: 20,  // 保留最近 20 轮对话
            enable_filekv_backend: true, // 启用高性能后端
            ..Default::default()
        };
        
        let ctx = Context::open_with_config("./.cad_context", config)?;
        
        Ok(Self {
            ctx,
            current_session: session_id.to_string(),
        })
    }
    
    pub fn add_user_message(&mut self, message: &str) -> Result<String> {
        let hash = self.ctx.store(
            &self.current_session,
            message.as_bytes(),
            Layer::ShortTerm,
        )?;
        Ok(hash)
    }
    
    pub fn add_design_result(&mut self, result: &[u8]) -> Result<String> {
        let hash = self.ctx.store(
            &self.current_session,
            result,
            Layer::LongTerm,  // 设计结果永久保存
        )?;
        Ok(hash)
    }
    
    pub fn create_design_branch(&mut self, branch_name: &str) -> Result<()> {
        // 使用并行管理器创建设计分支
        // ...
        Ok(())
    }
}
```

---

## 下一步建议

1. **快速验证**: 创建测试项目验证基础 API
2. **POC 实现**: 实现 `DialogStateManager` 原型
3. **性能基准**: 对比当前方案与 tokitai-context 的性能
4. **集成评估**: 评估与现有 LLM 模块的集成复杂度
5. **生产配置**: 调整 FileKV、WAL、Cache 参数以适应 CAD 场景

---

## 参考资料

- **GitHub**: https://github.com/silverenternal/tokitai
- **Crate**: https://crates.io/crates/tokitai-context
- **文档**: https://docs.rs/tokitai-context
- **相关 Crate**: 
  - `tokitai` v0.4.0 (AI 工具宏)
  - `tokitai-mcp-server` v0.4 (MCP 服务器)
