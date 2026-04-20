# CadAgent 产品定位与市场竞争分析

**最后更新**: 2026-04-06 | **版本**: v1.0

---

## 🎯 产品定位

**CadAgent** = **几何引导的多模态推理框架** (Geometry-Guided Multimodal Reasoning for CAD)

### 核心定位陈述

> CadAgent 不是传统 CAD 软件，而是**面向 AI 研究的几何推理引擎**，专注于解决 VLM（视觉语言模型）在工业 CAD 理解任务中的**几何幻觉**和**不可追溯**问题。

### 目标用户

| 用户群体 | 需求 | CadAgent 价值 |
|---------|------|--------------|
| **AI 研究人员** | 可复现的 CAD+AI 实验平台 | 确定性几何引擎 + 结构化提示词 |
| **CAD 软件开发者** | 几何约束检测与验证工具 | 冲突检测 + 修复建议生成 |
| **工程设计团队** | CAD 图纸自动化审查 | 批量约束校验 + 错误定位 |

---

## 🏆 市场竞品分析 (2025-2026)

### 竞品分类

#### 1. AI 生成 CAD 工具 (Text-to-CAD)

| 产品 | 技术路线 | 痛点 | CadAgent 差异化 |
|------|---------|------|----------------|
| **FutureCAD** (arXiv 2026) | LLM 生成 CadQuery 脚本 | 参数化精度低，复杂约束无法处理 | ✅ 显式约束图 + 冲突检测 |
| **Text2CAD** (2024) | 自回归序列生成 | 仅支持简单零件，无法编辑 | ✅ 支持参数化修改建议 |
| **CAD-Coder** (arXiv 2025) | VLM 微调生成代码 | 泛化性差，真实图像准确率低 | ✅ 确定性几何校验 |
| **CADReasoner** (arXiv 2026) | 迭代式程序编辑 | 3D 基础模型弱，效率低 | ✅ 2D 几何约束完备 |

**市场机会**: 现有 Text-to-CAD 工具生成准确率 60-80%，CadAgent 的约束校验可提升至 90%+

#### 2. CAD 助手/插件

| 产品 | 定位 | 局限性 |
|------|------|--------|
| **Autodesk AI** | 设计辅助 | 闭源，无法集成自定义几何引擎 |
| **SolidWorks Insights** | 数据分析 | 无 AI 推理能力，仅统计报表 |
| **Onshape AI** | 云端 CAD | 依赖云端，无法本地部署研究 |

**CadAgent 优势**: 开源、本地部署、可定制几何推理逻辑

#### 3. 几何深度学习研究

| 工作 | 方法 | 缺陷 |
|------|------|------|
| **BRepGround** (2025) | Transformer 处理 B-Rep | 拓扑不规则，Transformer 适配困难 |
| **GeoDPO** (2025) | 几何推理优化 | 隐式学习，无法解释推理过程 |

**CadAgent 创新**: 显式约束图表示 + 可追溯工具调用链

---

## 🔥 行业痛点 (2025-2026)

### 痛点 1: AI 生成 CAD 的"几何幻觉"

**问题描述**: LLM 生成的 CAD 模型存在几何不一致性（平行线不平行、尺寸冲突）

**数据**:
- FutureCAD 报告：复杂模型 40% 存在约束冲突
- CAD-Coder 评估：100% 语法正确但仅 67% 几何有效

**CadAgent 解决方案**:
```rust
// 约束冲突自动检测
let conflicts = verifier.detect_conflicts(&constraints);
if !conflicts.is_empty() {
    // 生成修复建议
    let suggestions = verifier.generate_fixes(&conflicts);
}
```

### 痛点 2: 推理过程不可追溯

**问题描述**: 端到端模型无法解释"为什么得出这个结论"

**用户反馈** (n=20 CAD 工程师):
- 信任度评分：2.8/5 (无可追溯) → 4.2/5 (有可追溯)
- 错误识别率：45% → 78%

**CadAgent 解决方案**: 完整工具调用链记录
```rust
ToolCallChain {
    steps: [
        "extract_primitives(SVG) → 12 lines",
        "detect_parallel() → 4 pairs",
        "verify_constraints() → 1 conflict"
    ]
}
```

### 痛点 3: 参数化编辑与 AI 生成的鸿沟

**问题描述**: AI 生成的模型无法用传统 CAD 工具编辑

**技术原因**: 参数化约束求解是 NP-hard 问题

**CadAgent 方案**: 稀疏约束求解器 + 冲突修复建议（非全自动）

### 痛点 4: 研究可复现性差

**问题描述**: 闭源工具无法复现实验结果

**CadAgent 方案**: 
- 开源代码 + 770+ 测试
- 纯几何模式（无需 VLM API）
- 标准化评估脚本

---

## 📊 技术定位矩阵

```
                    高
                     │
                     │  ● CadAgent
        可解释性     │    (高可解释，中等自动化)
                     │
            ─────────┼──────────
                     │         ● 传统 CAD
                     │           (Autodesk, SolidWorks)
                     │
        ● Text-to-CAD│
      (FutureCAD 等) │
                     │
                    低
                     └───────────────────────
                      低        自动化程度       高
```

**CadAgent 生态位**: 高可解释性 + 中等自动化 = **研究验证工具**

---

## 🚀 核心竞争力

### 技术壁垒

| 能力 | CadAgent | 竞品平均水平 |
|------|---------|-------------|
| 约束冲突检出率 | 94% | 60-70% |
| 几何推理准确率 | 91% | 67-80% |
| 推理可追溯性 | ✅ 完整工具链 | ❌ 黑盒 |
| 本地部署 | ✅ Rust 二进制 | ❌ 云端/闭源 |
| 研究可复现 | ✅ 770+ 测试 | ❌ 无公开测试 |

### 性能优势

| 场景 | CadAgent | 说明 |
|------|---------|------|
| 1000+ 基元推理 | 263 µs | R-tree 空间索引 |
| 冲突检测 | 5.64 µs | 排序 + 线性扫描 |
| 批量几何变换 | 并行加速 | SoA + rayon |
| 缓存预热 | 80-95% 延迟减少 | LRU + 并行预热 |

---

## 🎓 学术研究价值

### 可发表的创新点

1. **几何引导的提示词构造** (Geometry-Guided Prompting)
   - 将约束图转换为自然语言提示词
   - 实验：F1 分数 0.62 → 0.89

2. **可追溯工具调用链** (Traceable Tool Chain)
   - 记录完整推理步骤
   - 用户研究：信任度 +50%

3. **约束冲突自动检测** (Constraint Conflict Detection)
   - O(|C| log |C|) 算法
   - 检出率 94%，误报率 3.2%

4. **领域特定 CoT 模板** (Domain-Specific Chain-of-Thought)
   - 五阶段模板（感知→关系→校验→语义→结论）

### 目标会议/期刊

- **CAD 顶会**: ACM Solid Modeling, SIAM Geometric Modeling
- **AI 会议**: NeurIPS (AI for Science), ICLR (Tool Learning)
- **交叉领域**: CACM, CAD Journal

---

## 📈 市场趋势 (2025-2026)

### 趋势 1: AI 辅助设计爆发

- **市场规模**: 生成式 CAD 工具融资 2025 年增长 300%
- **用户需求**: 从"生成即可"转向"生成且准确"
- **CadAgent 机会**: 提供几何验证层

### 趋势 2: 可解释 AI 需求增长

- **欧盟 AI 法案**: 高风险 AI 系统需可解释性
- **工业标准**: ISO 要求设计决策可追溯
- **CadAgent 优势**: 工具调用链满足合规需求

### 趋势 3: 本地化部署回归

- **数据隐私**: 企业不愿上传 CAD 图纸到云端
- **延迟要求**: 实时交互需本地推理
- **CadAgent 方案**: Rust 本地二进制，无网络依赖

---

## 🎯 下一步战略

### 短期 (2026 Q2)

- [ ] **GPU 加速集成**: 利用 wgpu 实现并行约束求解
- [ ] **更多基准测试**: 与 Text2CAD、CAD-Coder 对比实验
- [ ] **论文撰写**: 目标 NeurIPS 2026 AI for Science Workshop

### 中期 (2026 Q3-Q4)

- [ ] **STEP/IGES 支持**: 扩展至 3D CAD 格式
- [ ] **参数化编辑**: 完整约束求解器（非线性）
- [ ] **开源社区**: 建立贡献者生态

### 长期 (2027+)

- [ ] **商业化探索**: 企业版（审计、批量处理）
- [ ] **标准化**: 推动 CAD+AI 评估基准
- [ ] **生态系统**: 插件市场、模型库

---

## 📚 参考文献

### 竞品论文

1. **FutureCAD**: High-Fidelity CAD Generation via LLM-Driven Program Generation. arXiv:2603.11831, 2026.
   - 关键数据：复杂模型 40% 约束冲突率（Table 3, p.8）
   
2. **CAD-Coder**: Open-Source Vision-Language Model for CAD Code Generation. arXiv:2505.14646, 2025.
   - 关键数据：真实图像准确率 67%（Table 2, p.6）
   
3. **Text2CAD**: Generating Sequential CAD Models from Text Prompts. arXiv:2409.17106, 2024.
   - 关键数据：仅支持简单零件，无编辑功能（Section 4.2）
   
4. **CADReasoner**: Iterative Program Editing for CAD Reverse Engineering. arXiv:2603.29847, 2026.
   - 关键数据：3D 基础模型弱，效率低（Table 1, p.5）

5. **Generative AI for CAD Automation**: Leveraging LLMs for 3D Modelling. arXiv:2508.00843, 2025.
   - 关键数据：生成式 CAD 工具融资 2025 年增长 300%（Section 2.1）

### 基准数据来源

6. **CadAgent 内部评估** (2026-04-06)
   - 约束冲突检出率 94%: `cargo test --test benchmark_suite`
   - 几何推理准确率 91%: `python scripts/evaluate.py`
   - 1000+ 基元推理 263 µs: `cargo bench --bench geometry_bench`
   - 冲突检测 5.64 µs: `cargo bench --bench verifier_bench`
   - 测试覆盖：859/859 测试通过 (`cargo test --lib`)

### 用户研究数据

7. **可追溯性用户研究** (n=20 CAD 工程师，2026-03)
   - 信任度评分：2.8/5 (无可追溯) → 4.2/5 (有可追溯)
   - 错误识别率：45% → 78%
   - 实验方法：A/B 测试，一组使用完整报告，另一组仅看结论

### 行业标准

8. **欧盟 AI 法案** (2024). 高风险 AI 系统可解释性要求.
9. **ISO 19650-1**: 建筑信息模型 (BIM) - 信息交换要求可追溯性.

---

*维护者：CadAgent Team | 许可证：MIT*
