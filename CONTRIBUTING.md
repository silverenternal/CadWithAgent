# 贡献指南

欢迎为 CadAgent 项目做出贡献！

## 项目状态

- **版本**: v0.1.0
- **测试状态**: 770+ 测试全部通过
- **测试覆盖率**: 80%+
- **构建状态**: 稳定

## 开发环境设置

### 前置要求

- Rust 1.70+（推荐使用 `rustup` 管理）
- Cargo（随 Rust 安装）

### 安装步骤

```bash
# 克隆项目
git clone https://github.com/tokitai/cadagent.git
cd cadagent

# 构建项目
cargo build

# 运行测试
cargo test

# 运行 clippy 检查
cargo clippy -- -D warnings
```

### 环境变量配置

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，填入你的 API Key
# 注意：如果只使用纯几何模式，无需设置 API Key
```

### 配置验证

开发过程中如需修改配置文件，请先验证配置：

```bash
# 验证默认配置
cargo run --bin cadagent-cli -- validate-config

# 验证自定义配置
cargo run --bin cadagent-cli -- validate-config --config path/to/your/config.json
```

## 开发流程

### 1. 分支管理

- `main` - 主分支，始终保持稳定
- `feature/xxx` - 新功能分支
- `fix/xxx` - Bug 修复分支

### 2. 代码规范

#### 格式化

```bash
cargo fmt --all
```

#### Lint 检查

```bash
cargo clippy -- -D warnings
```

#### 测试要求

- 所有新功能必须添加单元测试
- 核心模块覆盖率需达到 80%+
- 所有测试必须通过（当前 770+ 测试）

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test --test geometry_tests
cargo test --test cad_reasoning_tests
cargo test --test integration_tests

# 生成覆盖率报告
cargo tarpaulin --output-dir coverage --out html
```

### 3. 提交规范

遵循 Conventional Commits 规范：

```
feat: 添加新功能
fix: 修复 Bug
docs: 文档更新
style: 代码格式调整
refactor: 重构代码
test: 添加测试
chore: 构建/工具配置
```

示例：
```bash
git commit -m "feat: 添加 R-tree 空间索引优化"
git commit -m "fix: 修复 SVG 解析器命名空间处理"
git commit -m "test: 添加几何关系检测单元测试"
```

## 模块架构

### 核心模块

```
src/
├── analysis/          # 统一分析管线（推荐使用）
├── bridge/            # VLM 桥接层
├── cad_extractor/     # CAD 基元提取
├── cad_reasoning/     # 几何关系推理
├── cad_verifier/      # 约束校验
├── config/            # 配置管理
├── cot/               # Geo-CoT 生成
├── export/            # 文件导出
├── feature/           # 参数化特征树
├── geometry/          # 几何图元与工具
├── gpu/               # GPU 加速计算与渲染
├── incremental/       # 增量更新系统
├── llm_reasoning/     # LLM 推理
├── lod/               # 多层次细节 (LOD)
├── memory/            # 内存优化（Arena、对象池）
├── metrics/           # 评估指标
├── parser/            # 文件解析（SVG/DXF/STEP/IGES）
├── prompt_builder/    # 提示词构造
├── tools/             # 工具注册表
└── topology/          # 拓扑分析
```

### 添加新工具

1. 在对应模块实现功能
2. 在 `tools/registry.rs` 注册
3. 添加单元测试
4. 更新文档

示例：
```rust
// src/geometry/measure.rs
pub fn measure_new_metric(&self, input: Input) -> f64 {
    // 实现
}

// 在 register_tools() 中注册
registry.register("measure_new", |args| {
    measurer.measure_new(args.input)
});
```

## 测试指南

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        assert_eq!(expected, actual);
    }
}
```

### 集成测试

测试文件位于 `tests/` 目录：

```rust
// tests/integration_tests.rs
use cadagent::prelude::*;

#[test]
fn test_end_to_end() {
    let pipeline = AnalysisPipeline::with_defaults().unwrap();
    let result = pipeline.inject_from_svg_string(svg, "分析").unwrap();
    assert!(result.primitive_count() > 0);
}
```

### 基准测试

```rust
// benches/geometry_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_measure(c: &mut Criterion) {
    c.bench_function("measure_area", |b| {
        b.iter(|| measurer.measure_area(black_box(&vertices)))
    });
}
```

## 文档规范

### 代码注释

- 公共 API 必须有文档注释
- 复杂算法需要说明实现思路
- 使用 Rust doc 测试

```rust
/// 测量多边形面积
///
/// # 参数
/// * `vertices` - 多边形顶点坐标
///
/// # 返回值
/// 多边形面积（平方单位）
///
/// # 示例
/// ```
/// let area = measurer.measure_area(vertices);
/// ```
pub fn measure_area(&self, vertices: &[[f64; 2]]) -> f64 {
    // 实现
}
```

### 更新 README

- 新功能需更新 README 对应章节
- 修改 API 需更新快速开始示例
- 添加新配置需更新配置说明

## 性能要求

### 基准指标

- `parse_svg`: < 10ms (1000 基元)
- `detect_relations`: < 100ms (1000 基元，R-tree 优化)
- `build_prompt`: < 50ms
- `validate_config`: < 100ms (27 项检查)

### 性能优化

1. 使用 R-tree 空间索引
2. 使用 `rayon` 并行处理
3. 避免不必要的内存分配
4. 使用基准测试验证优化效果

```bash
cargo bench
```

## 问题反馈

### 提交 Issue

请包含：
- 问题描述
- 复现步骤
- 环境信息（Rust 版本、OS）
- 错误日志

### 提交 PR

1. Fork 项目
2. 创建功能分支
3. 提交修改
4. 推送到分支
5. 创建 Pull Request

## 许可证

本项目采用 MIT 许可证。提交代码即表示你同意将代码授权在 MIT 许可证下使用。

---

**感谢你的贡献！** 🎉
