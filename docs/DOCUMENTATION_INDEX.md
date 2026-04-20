# CadAgent 文档索引

**最后更新**: 2026-04-06

---

## 📚 核心文档（5 个）

| 文档 | 用途 | 读者 | 位置 |
|------|------|------|------|
| [GETTING_STARTED.md](../GETTING_STARTED.md) | **5 分钟快速上手** | 所有用户 | 根目录 |
| [README.md](../README.md) | 项目概览、研究框架 | 所有用户 | 根目录 |
| [ARCHITECTURE.md](../ARCHITECTURE.md) | **架构设计详解** | 开发者 | 根目录 |
| [RESEARCH_GUIDE.md](../RESEARCH_GUIDE.md) | **研究使用指南** | 研究人员 | 根目录 |
| [CONTRIBUTING.md](../CONTRIBUTING.md) | 贡献指南、开发流程 | 贡献者 | 根目录 |

---

## 🔧 技术文档（2 个）

| 文档 | 用途 | 位置 |
|------|------|------|
| [PERFORMANCE.md](../PERFORMANCE.md) | 性能优化技术与基准测试 | 根目录 |
| [API_REFERENCE.md](../API_REFERENCE.md) | 完整 API 参考 | 根目录 |

---

## 🗄️ 归档文档

以下文档已移至 `docs/archive/` 目录，供历史参考：

### 技术报告
- `OPTIMIZATION_SUMMARY.md` - 优化总结
- `OPTIMIZATION_SUMMARY_2026_04_06.md` - 优化总结 (2026-04-06)
- `OPTIMIZATION_REPORT_2026_04_06.md` - 优化报告
- `CODE_QUALITY_IMPROVEMENT_2026_04_06.md` - 代码质量改进
- `GPU_OPTIMIZATION_REPORT_2026_04_06.md` - GPU 优化报告
- `GPU_TEST_FIX_REPORT_2026_04_06.md` - GPU 测试修复报告
- `IGES_ENHANCEMENT_2026_04_06.md` - IGES 增强报告

### 设计与规划
- `PRODUCT_POSITIONING.md` - 产品定位
- `PERFORMANCE_OPTIMIZATION.md` - 性能优化 (旧版)
- `IMPLEMENTATION_STATUS.md` - 实现状态
- `AUTONOMOUS_CAD_IMPLEMENTATION_PLAN.md` - 实现计划
- `AI_OPTIMIZATION_RECOMMENDATIONS.md` - AI 优化建议
- `AI_AUTONOMY_GAP_ANALYSIS.md` - AI 自主性差距分析

### tokitai-context 集成文档
- `TOKITAI_CONTEXT_ANALYSIS.md` - 分析
- `TOKITAI_CONTEXT_EXAMPLES.md` - 示例
- `TOKITAI_CONTEXT_INTEGRATION_PLAN.md` - 集成计划
- `TOKITAI_CONTEXT_INTEGRATION_SUMMARY.md` - 集成总结

---

## 🧪 常用命令

```bash
# 构建与测试
cargo build --release    # ~13s
cargo test --lib         # 1010 tests ✅

# 基准测试
cargo bench

# API 文档
cargo doc --open

# 代码质量
cargo clippy --lib
cargo clippy --all-targets
```

---

## 📊 核心指标

| 指标 | 目标 | 当前 | 状态 |
|------|------|------|------|
| 测试通过 | 1000+ | 1010 | ✅ |
| 测试覆盖率 | 80%+ | 80%+ | ✅ |
| 编译时间 | <15s | ~13s | ✅ |
| Clippy 警告 | 0 | 0 | ✅ |
| TODO/FIXME | 0 | 0 | ✅ |

---

## 🎯 快速选择阅读路径

**我是新用户，想快速上手:**
→ [GETTING_STARTED.md](../GETTING_STARTED.md)

**我是研究人员，想了解 GMR 框架:**
→ [RESEARCH_GUIDE.md](../RESEARCH_GUIDE.md)

**我是开发者，想了解架构:**
→ [ARCHITECTURE.md](../ARCHITECTURE.md)

**我想贡献代码:**
→ [CONTRIBUTING.md](../CONTRIBUTING.md)

**我想优化性能:**
→ [PERFORMANCE.md](../PERFORMANCE.md)

---

*维护者：CadAgent Team | 许可证：MIT*
