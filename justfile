# CadAgent Justfile
# 统一构建、测试、评估和基准测试命令
# 使用前请安装 just: https://just.systems/

# ============ 构建命令 ============

## 默认构建（debug 模式）
build:
    cargo build

## Release 构建（优化）
build-release:
    cargo build --release

## 检查编译错误（快速）
check:
    cargo check

## 清理构建产物
clean:
    cargo clean

# ============ 测试命令 ============

## 运行所有单元测试
test:
    cargo test --lib

## 运行所有测试（含集成测试）
test-all:
    cargo test

## 运行特定测试
test-filter name:
    cargo test {{name}}

## 运行基准测试套件
test-benchmark-suite:
    cargo test --test benchmark_suite

## 生成测试覆盖率报告
test-coverage:
    cargo tarpaulin --out Html --output-dir coverage

# ============ 基准测试 ============

## 运行所有基准测试
bench:
    cargo bench

## 运行几何基准测试
bench-geometry:
    cargo bench --bench geometry_bench

## 运行推理基准测试
bench-reasoning:
    cargo bench --bench reasoning_bench

## 运行约束校验基准测试
bench-verifier:
    cargo bench --bench verifier_bench

## 运行评估指标基准测试
bench-metrics:
    cargo bench --bench metrics_bench

## 运行增量更新基准测试
bench-incremental:
    cargo bench --bench incremental_bench

## 运行大规模场景基准测试
bench-large-scale:
    cargo bench --bench large_scale_bench

## 运行 NURBS 基准测试
bench-nurbs:
    cargo bench --bench nurbs_bench

# ============ 评估命令 ============

## 运行综合评估（房间 + 尺寸 + 冲突）
evaluate:
    python scripts/evaluate.py --output results/

## 运行评估并生成对比报告
evaluate-report:
    python scripts/evaluate.py --output results/ --compare

## 使用自定义数据集评估
evaluate-data path:
    python scripts/evaluate.py --data {{path}} --output results/

## 仅评估房间检测
evaluate-room:
    python scripts/evaluate.py --output results/ --task room

## 仅评估尺寸提取
evaluate-dimensions:
    python scripts/evaluate.py --output results/ --task dimension

## 仅评估冲突检测
evaluate-conflicts:
    python scripts/evaluate.py --output results/ --task conflict

## 生成 JSON 格式报告
evaluate-json:
    python scripts/evaluate.py --output results/ --report json

## 生成 CSV 格式对比
evaluate-csv:
    python scripts/evaluate.py --output results/ --report csv

# ============ 文档命令 ============

## 生成 API 文档
doc:
    cargo doc --no-deps

## 生成并打开 API 文档
doc-open:
    cargo doc --no-deps --open

## 检查文档警告
doc-check:
    cargo doc --no-deps 2>&1 | grep -i warning || true

## 生成文档并运行 doctest
doc-test:
    cargo test --doc

# ============ 代码质量 ============

## 格式化代码
fmt:
    cargo fmt

## 检查代码格式
fmt-check:
    cargo fmt -- --check

## 运行 Clippy 检查
clippy:
    cargo clippy -- -D warnings

## 运行 Clippy（允许警告）
clippy-warn:
    cargo clippy -- -W warnings

## 完整代码质量检查
lint: fmt-check clippy

## 检查 WGSL 语法
check-wgsl:
    ./scripts/check_wgsl.sh

## 检查 WGSL 语法（指定目录）
check-wgsl-dir path:
    ./scripts/check_wgsl.sh {{path}}

# ============ 发布命令 ============

## 发布新版本
release version:
    @echo "Releasing version {{version}}..."
    cargo bump {{version}}
    git tag v{{version}}
    git push origin v{{version}}
    cargo publish

# ============ 开发工作流 ============

## 完整开发流程（检查 + 测试 + 文档）
dev: check test doc

## 发布前检查
prerelease: lint test-all bench doc-test

## 快速迭代（仅检查 + 测试）
quick: check test

# ============ 帮助命令 ============

## 显示所有可用命令
help:
    @just --list

## 显示命令详情
help-cmd cmd:
    @just --show {{cmd}}

# ============ 别名 ============

b := build
br := build-release
t := test
ta := test-all
b := bench
e := evaluate
d := doc
l := lint
