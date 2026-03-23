# CadAgent

基于 Rust 的 CAD 几何处理工具链，通过工具增强上下文注入范式驱动 VLM 进行几何推理。

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Test Status](https://img.shields.io/badge/tests-248%20passed-brightgreen)]()
[![Coverage Status](https://img.shields.io/badge/coverage-80%2B%25-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

## 特性

- **🔧 工具化几何算法**: 测量、变换、拓扑分析等封装为 tokitai 工具
- **🧠 工具增强上下文注入**: 无需修改 VLM，通过外部几何算法构造精准提示词
- **📐 完整几何工具链**: 基元提取 → 关系推理 → 约束校验 → 提示词生成
- **🤖 VLM 集成**: 支持 ZazaZ、OpenAI 等兼容 API，自动思维链生成
- **⚡ 高性能**: R-tree 空间索引优化，1000+ 基元场景性能提升 10 倍 +
- **📊 SVG/DXF 处理**: 完整的文件解析与导出能力

## 项目思路

### 为什么需要 CadAgent？

直接使用 VLM 处理 CAD 图纸存在以下问题：

1. **几何计算不可靠**: VLM 不擅长精确的长度、面积、角度计算
2. **约束关系易遗漏**: 平行、垂直、连接等几何关系容易判断错误
3. **结果不可解释**: 无法追溯推理过程，难以满足工业级可信要求

### CadAgent 的解决方案

**工具增强上下文注入范式** —— 让专业的做专业的事：

```
┌─────────────────────────────────────────────────────────────┐
│  输入：CAD 图纸 (SVG/DXF)                                     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  确定性几何算法层 (CadAgent)                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  基元提取     │→ │  关系推理     │→ │  约束校验     │      │
│  │ • 线段/圆/弧  │  │ • 平行/垂直  │  │ • 冲突检测    │      │
│  │ • 坐标解析    │  │ • 连接/相切  │  │ • 冗余检查    │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                            │                                 │
│                            ▼                                 │
│                   ┌──────────────┐                          │
│                   │ 结构化提示词  │ ← 注入精准的几何上下文    │
│                   └──────────────┘                          │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  VLM 推理层 (Qwen/GPT 等)                                      │
│  • 理解任务意图                                              │
│  • 基于精准几何数据推理                                       │
│  • 生成可解释的思维链                                         │
└─────────────────────────────────────────────────────────────┘
```

### 核心设计理念

1. **确定性算法处理几何计算** — 100% 准确，可验证
2. **VLM 专注高层推理** — 意图理解、任务规划、自然语言生成
3. **不修改模型，即插即用** — 通过提示词工程实现能力增强
4. **推理链可追溯** — 每一步几何关系都有算法依据

## 快速开始

### 安装

```bash
# 克隆项目
git clone https://github.com/tokitai/cadagent.git
cd cadagent

# 构建
cargo build --release

# 运行测试
cargo test
```

### 配置环境变量

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 设置 API Key
export PROVIDER_ZAZAZ_API_KEY="your-api-key"
```

### 基础示例

```rust
use cadagent::prelude::*;

fn main() -> anyhow::Result<()> {
    // 创建分析管线
    let pipeline = AnalysisPipeline::with_defaults()?;

    // 从 SVG 字符串注入上下文
    let svg = r#"<svg width="100" height="100">
        <line x1="0" y1="0" x2="100" y2="0" />
        <line x1="100" y1="0" x2="100" y2="100" />
        <line x1="100" y1="100" x2="0" y2="100" />
        <line x1="0" y1="100" x2="0" y2="0" />
    </svg>"#;

    let result = pipeline.inject_from_svg_string(svg, "分析这个户型图")?;

    println!("基元数量：{}", result.primitives.len());
    println!("几何关系：{} 个", result.relations.len());
    println!("提示词长度：{} 字符", result.prompt.full_prompt.len());

    Ok(())
}
```

### 完整用法（含 VLM 推理）

```rust
use cadagent::prelude::*;

fn main() -> anyhow::Result<()> {
    let pipeline = AnalysisPipeline::with_defaults()?;

    // 执行完整的几何分析 + VLM 推理
    let result = pipeline.inject_from_svg_string_with_vlm(
        svg_content,
        "请分析这个户型图，识别所有房间并计算面积"
    )?;

    // 访问 VLM 回答
    if let Some(vlm) = &result.vlm_response {
        println!("模型：{}", vlm.model);
        println!("回答：{}", vlm.content);
        println!("Token 使用：{:?}", vlm.usage);
    }

    Ok(())
}
```

## 架构设计

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

## 核心模块

### 1. 几何图元与工具

```rust
use cadagent::prelude::*;

// 创建图元
let room = Polygon::from_coords(vec![
    [0.0, 0.0], [500.0, 0.0], [500.0, 400.0], [0.0, 400.0],
]);

// 测量工具
let measurer = GeometryMeasurer;
let area = measurer.measure_area(vec![
    [0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0],
]);

// 变换工具
let transform = GeometryTransform;
let translated = transform.translate(vec![Primitive::Polygon(room)], 50.0, 50.0);
```

### 2. 拓扑分析

```rust
use cadagent::topology::room_detect::RoomDetector;

let detector = RoomDetector;
let room_count = detector.count_rooms(primitives);
let doors = detector.detect_doors(&primitives);
let windows = detector.detect_windows(&primitives);
```

### 3. 分析管线（推荐）

```rust
use cadagent::prelude::*;

// 创建管线
let pipeline = AnalysisPipeline::with_defaults()?;

// 执行完整分析
let result = pipeline.inject_from_svg_string(svg, "分析这个图形")?;

// 访问结果
println!("基元：{} 个", result.primitives.len());
println!("关系：{} 个", result.relations.len());
println!("提示词：{} 字符", result.prompt.full_prompt.len());
```

### 4. 自定义 VLM 供应商

```rust
use cadagent::bridge::vlm_client::VlmConfig;
use cadagent::prelude::*;

// 配置 ZazaZ API
let vlm_config = VlmConfig::new(
    "https://zazaz.top/v1",
    "sk-your-api-key",
    "./Qwen3.5-27B-FP8",
);

// 或使用 OpenAI
// let vlm_config = VlmConfig::default_openai()?;

let pipeline = AnalysisPipeline::with_vlm_config(vlm_config)?;
let result = pipeline.inject_from_svg_string_with_vlm(svg, "分析")?;
```

## 工具列表

### 测量工具

| 工具名 | 描述 |
|--------|------|
| `measure_length` | 测量线段长度 |
| `measure_area` | 计算多边形面积 |
| `measure_angle` | 测量角度 |
| `measure_perimeter` | 计算周长 |
| `check_parallel` | 检查平行 |
| `check_perpendicular` | 检查垂直 |

### 变换工具

| 工具名 | 描述 |
|--------|------|
| `translate` | 平移 |
| `rotate` | 旋转 |
| `scale` | 缩放 |
| `mirror` | 镜像 |

### 拓扑分析工具

| 工具名 | 描述 |
|--------|------|
| `detect_rooms` | 检测房间 |
| `count_rooms` | 统计房间数量 |
| `detect_doors` | 检测门 |
| `detect_windows` | 检测窗户 |
| `find_closed_loop` | 查找闭合回路 |

### Geo-CoT 工具

| 工具名 | 描述 |
|--------|------|
| `generate_geo_cot` | 生成几何思维链 |
| `generate_qa` | 生成问答对 |

### 分析工具（工具增强上下文注入）

| 工具名 | 描述 |
|--------|------|
| `cad_extract_primitives` | 从 SVG 提取几何基元 |
| `cad_find_geometric_relations` | 查找几何关系 |
| `cad_verify_constraints` | 校验约束合法性 |
| `cad_build_analysis_prompt` | 构建分析提示词 |
| `cad_context_inject` | 执行完整上下文注入流程 |

## 命令行使用

```bash
# 解析 SVG 文件
cargo run --bin cadagent-cli -- parse-svg --input floor_plan.svg --output primitives.json

# 测量
cargo run --bin cadagent-cli -- measure --kind area --data '{"vertices": [[0,0],[100,0],[100,100],[0,100]]}'

# 检测房间
cargo run --bin cadagent-cli -- detect-rooms --input primitives.json

# 导出 DXF
cargo run --bin cadagent-cli -- export-dxf --input primitives.json --output output.dxf

# 生成 Geo-CoT 数据
cargo run --bin cadagent-cli -- generate-cot --input primitives.json --task "计算所有房间的面积"

# 一致性检查
cargo run --bin cadagent-cli -- check-consistency --input primitives.json

# 列出所有工具
cargo run --bin cadagent-cli -- list-tools
```

## 示例

```bash
# 基础使用示例
cargo run --example basic_usage

# 完整管线示例
cargo run --example pipeline

# Geo-CoT 生成示例
cargo run --example cot_generation

# 上下文注入示例（不含 VLM 推理）
cargo run --example context_injection

# 真实 VLM 推理示例（调用 API）
cargo run --example vlm_inference
```

## 测试

```bash
# 运行所有测试
cargo test

# 运行几何模块测试
cargo test --test geometry_tests

# 运行几何推理测试
cargo test --test cad_reasoning_tests

# 生成覆盖率报告
cargo tarpaulin --output-dir coverage --out html
```

### 测试覆盖

- **几何模块**: 31 个单元测试
- **几何推理**: 17 个单元测试
- **总计**: 248 个测试全部通过
- **核心模块覆盖率**: 80%+

## 性能指标

| 操作 | 性能（1000 基元） |
|------|------------------|
| `parse_svg` | < 10ms |
| `detect_relations` (R-tree) | < 100ms |
| `build_prompt` | < 50ms |

## 依赖

- **tokitai**: AI 工具集成协议
- **reqwest**: HTTP 客户端（VLM API 调用）
- **tokio**: 异步运行时
- **serde/serde_json**: 序列化
- **roxmltree**: 可靠的 XML 解析
- **rstar**: R-tree 空间索引
- **geo/nalgebra**: 几何计算
- **clap**: CLI 解析
- **tracing**: 日志

## 项目结构

```
cadagent/
├── src/
│   ├── analysis/          # 统一分析管线（推荐使用）
│   ├── cad_extractor/     # CAD 基元提取
│   ├── cad_reasoning/     # 几何关系推理
│   ├── cad_verifier/      # 约束校验
│   ├── prompt_builder/    # 提示词构造
│   ├── geometry/          # 几何图元与工具
│   ├── topology/          # 拓扑分析
│   ├── cot/               # Geo-CoT 生成
│   ├── parser/            # 文件解析（SVG/DXF）
│   ├── export/            # 文件导出（JSON/DXF）
│   ├── bridge/            # VLM 桥接
│   ├── tools/             # 工具注册表
│   ├── llm_reasoning/     # LLM 推理
│   └── metrics/           # 评估指标
├── examples/              # 使用示例
├── tests/                 # 集成测试
├── benches/               # 性能基准测试
└── config/                # 配置文件
```

## 配置

配置文件位于 `config/` 目录：

- `config/default.json`: 默认配置
- `config/templates.json`: CoT 模板配置

环境变量配置见 `.env.example`：

```bash
# ZazaZ API 配置
export PROVIDER_ZAZAZ_API_KEY="your-api-key"
export PROVIDER_ZAZAZ_API_URL="https://zazaz.top/v1"
export PROVIDER_ZAZAZ_MODEL="./Qwen3.5-27B-FP8"

# OpenAI API 配置（可选）
export OPENAI_API_KEY="sk-..."
export OPENAI_MODEL="gpt-4o"
```

## 贡献

详见 [CONTRIBUTING.md](CONTRIBUTING.md)

### 快速开始

```bash
# 克隆项目
git clone https://github.com/tokitai/cadagent.git

# 构建
cargo build

# 运行测试
cargo test

# 运行 clippy
cargo clippy -- -D warnings
```

## 许可证

MIT License

## 相关链接

- [tokitai 文档](https://docs.rs/tokitai)
- [贡献指南](CONTRIBUTING.md)
