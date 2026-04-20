# CadAgent 快速入门指南

**5 分钟上手 CadAgent - 几何引导的多模态 CAD 理解框架**

---

## 🎯 你能用 CadAgent 做什么？

- ✅ **自动分析 CAD 图纸** - 从 SVG/DXF 提取几何约束
- ✅ **检测设计冲突** - 发现平行/垂直等约束矛盾
- ✅ **AI 辅助推理** - 使用 zazaz API 生成修复建议
- ✅ **可追溯推理链** - 每一步结论都有算法证据

---

## ⚡ 5 分钟快速开始

### 1. 安装 (1 分钟)

```bash
# 克隆项目
git clone https://github.com/tokitai/cadagent.git
cd cadagent

# 编译 (首次约 13 秒)
cargo build --release
```

### 2. 配置 (可选，如需 AI 功能)

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env，填入你的 ZazaZ API Key
# 获取：https://zazaz.top
```

**仅需几何功能？** 跳过此步，无需 API Key。

### 3. 运行第一个分析 (2 分钟)

创建 `analyze.rs`:

```rust
use cadagent::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建分析管线
    let pipeline = AnalysisPipeline::with_defaults()?;

    // SVG 输入 (一个简单的矩形)
    let svg = r#"<svg width="500" height="400">
        <line x1="0" y1="0" x2="500" y2="0" />
        <line x1="500" y1="0" x2="500" y2="400" />
        <line x1="500" y1="400" x2="0" y2="400" />
        <line x1="0" y1="400" x2="0" y2="0" />
    </svg>"#;

    // 执行分析
    let result = pipeline.inject_from_svg_string(svg, "分析这个图形")?;

    // 查看结果
    println!("几何基元：{} 个", result.primitive_count());
    println!("工具调用链:\n{}", result.tool_chain_json());

    Ok(())
}
```

运行:

```bash
cargo run --example basic_usage
```

### 4. 查看结果 (2 分钟)

输出示例:

```
几何基元：4 个
工具调用链:
{
  "steps": [
    {
      "step_id": 1,
      "tool_name": "extract_primitives",
      "explanation": "从 SVG 提取 4 条线段"
    },
    {
      "step_id": 2,
      "tool_name": "detect_relations",
      "explanation": "检测相邻关系和角度"
    },
    {
      "step_id": 3,
      "tool_name": "verify_constraints",
      "explanation": "验证闭合回路约束"
    }
  ]
}
```

---

## 📚 下一步学习

### 按你的需求选择路径

**🔬 研究人员:**
- 阅读 [RESEARCH_GUIDE.md](RESEARCH_GUIDE.md)
- 了解 GMR 框架和实验设计
- 复现论文结果

**🛠️ 开发者:**
- 阅读 [ARCHITECTURE.md](ARCHITECTURE.md)
- 了解模块设计和 API
- 查看 [CONTRIBUTING.md](CONTRIBUTING.md)

**⚡ 性能优化:**
- 阅读 [PERFORMANCE.md](PERFORMANCE.md)
- 了解 R-tree 索引和 SIMD 优化
- 运行基准测试

---

## 🎓 核心概念速查

### Geo-Guided Prompt (几何引导提示)

```
传统 VLM: "这个图形是什么？"
         ↓ (可能产生几何幻觉)

CadAgent: "这个图形包含 4 条线段，
           约束：相邻垂直、闭合回路"
         ↓ (确定性几何约束注入)
         → 更准确的推理
```

### Traceable Tool-Chain (可追溯工具链)

每一步推理都有记录:

```
1. extract_primitives → 4 条线段
2. detect_relations   → 相邻 + 垂直
3. verify_constraints → 闭合回路 ✓
4. infer_semantics    → 矩形房间
```

### Conflict Detection (冲突检测)

```rust
// 检测冲突：既平行又垂直
let conflict = verifier.detect_conflict(&constraints)?;
// → "wall_0 ⟂ wall_1 且 wall_0 ∥ wall_1，矛盾"
```

---

## 📖 完整文档索引

| 文档 | 用途 |
|------|------|
| [README.md](README.md) | 项目概览和研究框架 |
| [RESEARCH_GUIDE.md](RESEARCH_GUIDE.md) | 研究使用指南 |
| [ARCHITECTURE.md](ARCHITECTURE.md) | 架构设计详解 |
| [API_REFERENCE.md](API_REFERENCE.md) | API 完整参考 |
| [PERFORMANCE.md](PERFORMANCE.md) | 性能优化指南 |
| [CONTRIBUTING.md](CONTRIBUTING.md) | 贡献指南 |

---

## 🆘 常见问题

### Q: 需要 Rust 经验吗？
A: 基础 Rust 知识有帮助，但示例都很简单，可直接运行。

### Q: 必须设置 API Key 吗？
A: 不。纯几何功能 (基元提取、约束求解) 无需 API。

### Q: 支持哪些 CAD 格式？
A: 当前支持 SVG、DXF。STEP/IGES 在开发中。

### Q: 如何引用这个项目？
A: 使用 BibTeX:
```bibtex
@software{cadagent2026,
  title = {CadAgent: Geometry-Guided Multimodal Reasoning for CAD},
  author = {Tokitai Team},
  year = {2026},
  url = {https://github.com/tokitai/cadagent}
}
```

---

*最后更新：2026-04-06 | 版本：v0.1.0*
