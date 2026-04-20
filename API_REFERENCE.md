# CadAgent API 参考

**完整的模块 API 文档和使用示例**

---

## 📦 模块概览

### 核心模块 (P0)

| 模块 | 用途 | 关键 API |
|------|------|---------|
| [`analysis`](#analysis) | 统一分析管线 | `AnalysisPipeline::with_defaults()` |
| [`geometry`](#geometry) | 几何算法 | `ConstraintSystem`, `measure_*()` |
| [`cad_reasoning`](#cad_reasoning) | 关系推理 | `RelationReasoner` |
| [`cad_verifier`](#cad_verifier) | 约束验证 | `ConstraintVerifier` |
| [`prompt_builder`](#prompt_builder) | 提示词构造 | `PromptBuilder` |

### 支撑模块 (P1)

| 模块 | 用途 | 关键 API |
|------|------|---------|
| [`parser`](#parser) | 文件解析 | `parse_svg()`, `parse_dxf()` |
| [`topology`](#topology) | 拓扑分析 | `detect_rooms()`, `detect_loops()` |
| [`bridge`](#bridge) | API 适配 | `VlmClient`, `ZazaClient` |
| [`context`](#context) | 上下文管理 | `DialogState`, `Branch` |
| [`cot`](#cot) | 思维链 | `GeoCoTTemplate` |

---

## analysis

**统一分析管线 - 推荐入口**

### 快速开始

```rust
use cadagent::prelude::*;

// 创建默认管线
let pipeline = AnalysisPipeline::with_defaults()?;

// 从 SVG 分析
let svg = r#"<svg width="100" height="100">
    <line x1="0" y1="0" x2="100" y2="100" />
</svg>"#;

let result = pipeline.inject_from_svg_string(svg, "分析这个图形")?;

println!("基元：{} 个", result.primitive_count());
println!("工具链：{}", result.tool_chain_json());
```

### 核心 API

```rust
// 创建管线
pub struct AnalysisPipeline {
    // ...
}

impl AnalysisPipeline {
    // 使用默认配置
    pub fn with_defaults() -> Result<Self, Error>;
    
    // 使用自定义配置
    pub fn with_config(config: AnalysisConfig) -> Result<Self, Error>;
    
    // 从 SVG 字符串注入
    pub fn inject_from_svg_string(
        &self,
        svg: &str,
        task: &str,
    ) -> Result<AnalysisResult, Error>;
    
    // 从 SVG 文件注入
    pub fn inject_from_svg_file(
        &self,
        path: &Path,
        task: &str,
    ) -> Result<AnalysisResult, Error>;
    
    // 从 DXF 文件注入
    pub fn inject_from_dxf_file(
        &self,
        path: &Path,
        task: &str,
    ) -> Result<AnalysisResult, Error>;
}

// 分析结果
pub struct AnalysisResult {
    pub primitives: Vec<Primitive>,
    pub relations: Vec<GeometricRelation>,
    pub constraints: Vec<Constraint>,
    pub verification: VerificationResult,
    pub tool_chain: ToolCallChain,
}

impl AnalysisResult {
    // 基元数量
    pub fn primitive_count(&self) -> usize;
    
    // 工具链 JSON
    pub fn tool_chain_json(&self) -> String;
    
    // 导出为 DXF
    pub fn export_dxf(&self, path: &Path) -> Result<(), Error>;
    
    // 导出为 JSON
    pub fn export_json(&self) -> Result<String, Error>;
}
```

### 配置选项

```rust
pub struct AnalysisConfig {
    // 启用 R-tree 索引 (默认：true)
    pub use_rtree: bool,
    
    // 启用 SIMD 优化 (默认：true)
    pub use_simd: bool,
    
    // 启用稀疏求解器 (默认：true)
    pub use_sparse_solver: bool,
    
    // VLM API 配置 (可选)
    pub vlm_config: Option<VlmConfig>,
    
    // 超时 (毫秒)
    pub timeout_ms: u64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            use_rtree: true,
            use_simd: true,
            use_sparse_solver: true,
            vlm_config: None,
            timeout_ms: 60000,
        }
    }
}
```

---

## geometry

**几何算法核心**

### 几何基元

```rust
use cadagent::geometry::*;

// 点
let p1 = Point2D::new(0.0, 0.0);
let p2 = Point2D::new(10.0, 10.0);

// 线段
let line = Line::new(p1, p2);

// 圆
let circle = Circle::new(Point2D::origin(), 5.0);

// 圆弧
let arc = Arc::new(center, start, end, clockwise);

// 多段线
let poly = Polyline::new(vec![p1, p2, p3]);
```

### 几何测量

```rust
use cadagent::geometry::measure::*;

// 距离
let d = distance(p1, p2);

// 角度
let angle = angle_between(line1, line2);

// 面积
let area = polygon_area(&polygon);

// 点到线距离
let dist = point_to_line_distance(point, line);

// 最近点
let closest = closest_point_on_line(point, line);
```

### 约束求解

```rust
use cadagent::geometry::constraint::*;

// 创建约束系统
let mut system = ConstraintSystem::new();

// 添加变量
system.add_variable("x0", 0.0);
system.add_variable("y0", 0.0);

// 添加约束
system.add_constraint(Constraint::distance(p1, p2, 10.0));
system.add_constraint(Constraint::perpendicular(l1, l2));
system.add_constraint(Constraint::parallel(l1, l2));
system.add_constraint(Constraint::coincident(p1, p2));

// 求解
let solution = system.solve()?;

// 诊断
let diagnosis = system.diagnose();
if diagnosis.has_conflicts() {
    for conflict in diagnosis.conflicts() {
        println!("冲突：{}", conflict);
    }
}
```

### 稀疏约束求解 (大规模)

```rust
use cadagent::geometry::constraint_sparse::*;

// 适用于 100+ 变量的场景
let mut system = SparseConstraintSystem::new();

// 自动分析依赖，构建稀疏 Jacobian
system.add_constraint(/* ... */);

// 求解 (3-20x 快于稠密实现)
let solution = system.solve_sparse()?;
```

### 几何变换

```rust
use cadagent::geometry::transform::*;

// 平移
let translated = translate(&shape, dx, dy);

// 旋转
let rotated = rotate(&shape, angle_degrees, center);

// 缩放
let scaled = scale(&shape, sx, sy, center);

// 仿射变换
let matrix = AffineMatrix::rotation(angle_degrees)
    .then_translation(dx, dy)
    .then_scale(sx, sy);
    
let transformed = transform(&shape, &matrix);
```

### 布尔运算

```rust
use cadagent::geometry::boolean::*;

// 交集
let intersection = intersect(&poly1, &poly2)?;

// 并集
let union = union(&poly1, &poly2)?;

// 差集
let difference = subtract(&poly1, &poly2)?;

// 裁剪
let clipped = clip(&polygon, &clip_bounds)?;
```

---

## cad_reasoning

**几何关系推理**

### 关系检测

```rust
use cadagent::cad_reasoning::*;

let reasoner = RelationReasoner::new();

// 检测相邻关系
let adjacent = reasoner.detect_adjacent(&primitives)?;

// 检测平行关系
let parallel = reasoner.detect_parallel(&primitives)?;

// 检测垂直关系
let perpendicular = reasoner.detect_perpendicular(&primitives)?;

// 检测共线关系
let collinear = reasoner.detect_collinear(&primitives)?;

// 检测相切关系
let tangent = reasoner.detect_tangent(&primitives)?;
```

### 使用 R-tree 加速

```rust
use cadagent::cad_reasoning::*;

// 50+ 基元场景自动启用 R-tree
let reasoner = RelationReasoner::with_rtree();

// 复杂度：O(n²) → O(n log n)
let relations = reasoner.detect_all_relations(&primitives)?;
```

### 关系图构建

```rust
use cadagent::cad_reasoning::*;

// 构建关系图
let graph = RelationGraph::from_relations(&relations);

// 查询连通性
let is_connected = graph.is_connected(p1_id, p2_id);

// 查询所有关系
let rels = graph.get_relations(primitive_id);
```

---

## cad_verifier

**约束验证与冲突检测**

### 基本验证

```rust
use cadagent::cad_verifier::*;

let verifier = ConstraintVerifier::new();

// 验证一致性
let result = verifier.verify_consistency(&constraints)?;

if result.is_consistent {
    println!("约束系统一致 ✓");
} else {
    println!("发现 {} 个冲突", result.conflicts.len());
}
```

### 冲突检测

```rust
use cadagent::cad_verifier::*;

let verifier = ConstraintVerifier::new();

// 检测所有冲突
let conflicts = verifier.detect_conflicts(&constraints)?;

for conflict in conflicts {
    println!("冲突类型：{}", conflict.kind);
    println!("涉及实体：{:?}", conflict.entities);
    println!("描述：{}", conflict.description);
    println!("修复建议：{}", conflict.suggestion);
}
```

### 冲突类型

```rust
pub enum ConflictKind {
    // 既平行又垂直
    ParallelPerpendicular,
    
    // 距离矛盾
    DistanceContradiction,
    
    // 角度矛盾
    AngleContradiction,
    
    // 共线矛盾
    CoincidentContradiction,
    
    // 过约束
    OverConstrained,
    
    // 欠约束
    UnderConstrained,
}
```

---

## prompt_builder

**结构化提示词构造**

### 基本用法

```rust
use cadagent::prompt_builder::*;

let builder = PromptBuilder::new();

let prompt = builder
    .with_task("分析这个户型图")
    .with_geometric_constraints(&constraints)
    .with_semantic_hints(&room_detection)
    .build();

println!("{}", prompt);
```

### 输出示例

```
你是一个 CAD 几何推理专家。

任务：分析这个户型图

几何约束:
- 12 条线段 (坐标：...)
- 相邻关系：wall_0 相邻 wall_1
- 垂直关系：wall_0 ⟂ wall_1
- 平行关系：wall_0 ∥ wall_2
- 闭合回路：room_0 (面积：50.5 m²)

语义提示:
- 检测到的房间：3 个
- 房间类型：客厅、卧室、厨房

请回答：这个户型图有几个房间？每个房间的面积是多少？
```

### CoT 模板

```rust
use cadagent::prompt_builder::cot_templates::*;

// 5 阶段推理模板
let cot = GeoCoTTemplate::five_stage(
    "perception",
    "relation",
    "verification",
    "semantics",
    "conclusion",
);

let prompt = builder
    .with_task(task)
    .with_cot_template(&cot)
    .build();
```

---

## parser

**文件解析**

### SVG 解析

```rust
use cadagent::parser::*;

// 从字符串解析
let primitives = parse_svg_string(svg_content)?;

// 从文件解析
let primitives = parse_svg_file("path/to/file.svg")?;

// 带选项解析
let options = SvgParseOptions {
    tolerance: 1e-6,
    merge_collinear: true,
};
let primitives = parse_svg_with_options(svg_content, options)?;
```

### DXF 解析

```rust
use cadagent::parser::*;

// 解析 DXF 文件
let dxf_result = parse_dxf_file("path/to/file.dxf")?;

// 访问解析结果
for entity in dxf_result.entities {
    match entity.kind {
        EntityKind::Line(line) => { /* ... */ }
        EntityKind::Circle(circle) => { /* ... */ }
        EntityKind::Arc(arc) => { /* ... */ }
        EntityKind::Polyline(poly) => { /* ... */ }
        // 已支持：Line, Circle, Arc, LwPolyline, Polyline, Text, MText
        // 待支持：Spline, Ellipse, Hatch, Dimension, Leader
    }
}
```

### STEP 解析 (WIP)

```rust
use cadagent::parser::*;

// 解析 STEP 文件 (仅 ManifoldSolidBrep)
let step_result = parse_step_file("path/to/file.step")?;

// 当前支持：
// - ManifoldSolidBrep with tessellation
// 待支持：
// - AdvancedBrep (需要几何内核)
```

---

## bridge

**VLM API 适配**

### VlmClient (通用)

```rust
use cadagent::bridge::*;

// 使用 zazaz 配置
let config = VlmConfig::default_zazaz()?;
let client = VlmClient::new(config);

// 聊天补全
let response = client.chat_completions(&[
    ("system", "你是 CAD 专家"),
    ("user", "分析这个图形"),
])?;

println!("回答：{}", response.choices[0].message.content);
```

### ZazaClient (zazaz 专用)

```rust
use cadagent::bridge::zaza_client::*;

// 从环境变量创建
let client = ZazaClient::from_env()?;

// 生成响应
let response = client.generate("解释 CAD 约束").await?;

// AI 辅助合并
let advice = client.assist_merge(
    "scheme-A",
    "scheme-B",
    "冲突描述"
).await?;

// 冲突解决
let resolution = client.resolve_conflict(
    "perpendicular_parallel_conflict",
    &["wall_0", "wall_1"],
    &["wall_0 ⟂ wall_1", "wall_0 ∥ wall_1"]
).await?;

// 分支目的推断
let purpose = client.infer_branch_purpose(
    "scheme-modern",
    &["移动厨房到北侧"],
    "用户想要更开放的空间"
).await?;
```

---

## context

**上下文管理 (基于 tokitai-context)**

### 对话状态

```rust
use cadagent::context::*;

// 创建对话状态
let mut state = DialogState::new();

// 添加消息
state.add_user_message("分析这个户型图");
state.add_assistant_message("检测到 3 个房间...");

// 获取最近对话
let recent = state.get_recent_turns(5)?;
```

### 分支管理

```rust
use cadagent::context::branch::*;

// 创建分支
let mut branch = Branch::new("main");

// 创建子分支 (O(1))
let feature = branch.create_child("feature-room-detection")?;

// 合并分支
let merge_result = branch.merge(&feature, MergeStrategy::SelectiveMerge)?;

// 检测冲突
if merge_result.has_conflicts() {
    // 使用 AI 解决冲突
    let client = ZazaClient::from_env()?;
    let resolution = client.resolve_conflict(/* ... */).await?;
}
```

---

## topology

**拓扑分析**

### 房间检测

```rust
use cadagent::topology::*;

// 检测房间
let rooms = detect_rooms(&primitives)?;

for room in rooms {
    println!("房间面积：{} m²", room.area);
    println!("房间周长：{} m", room.perimeter);
    println!("房间类型：{}", room.infer_type());
}
```

### 回路检测

```rust
use cadagent::topology::*;

// 检测闭合回路
let loops = detect_loops(&primitives)?;

// 分析回路属性
for loop_ in loops {
    let is_clockwise = loop_.is_clockwise();
    let area = loop_.area();
    let is_valid_room = loop_.is_simple() && area > 1.0;
}
```

### 门窗检测

```rust
use cadagent::topology::*;

// 检测门窗
let doors = detect_doors(&primitives, &rooms)?;
let windows = detect_windows(&primitives, &rooms)?;

// 分析连通性
let connectivity = analyze_connectivity(&rooms, &doors)?;
```

---

## cot

**Geo-CoT 思维链**

### 使用模板

```rust
use cadagent::cot::*;

// 5 阶段模板
let template = GeoCoTTemplate::five_stage(
    "Perception: 我观察到 4 条线段",
    "Relation: 它们形成相邻和垂直关系",
    "Verification: 约束一致，形成闭合回路",
    "Semantics: 这是一个矩形房间",
    "Conclusion: 户型图包含 1 个房间",
);

// 生成 CoT
let cot = template.generate()?;
```

### 自定义模板

```rust
use cadagent::cot::*;

let template = GeoCoTTemplate::custom(vec![
    CoTStage::new("观察", "描述看到的几何元素"),
    CoTStage::new("分析", "分析几何关系"),
    CoTStage::new("验证", "验证约束一致性"),
    CoTStage::new("推理", "推断语义信息"),
    CoTStage::new("结论", "给出最终答案"),
]);
```

---

## metrics

**评估指标**

### 计算指标

```rust
use cadagent::metrics::*;

// 计算 F1 分数
let f1 = f1_score(precision, recall);

// 计算 IoU
let iou = intersection_over_union(pred_boxes, gt_boxes);

// 综合评估
let metrics = EvaluationMetrics::compute(&predictions, &ground_truth)?;

println!("F1: {:.2}", metrics.f1);
println!("Precision: {:.2}", metrics.precision);
println!("Recall: {:.2}", metrics.recall);
```

---

## web_server

**Web API 服务器 (Axum)**

### 启动服务器

```rust
use cadagent::web_server::WebServer;

// 创建服务器 (默认端口 8080)
let server = WebServer::new(8080);

// 启动
server.run().await?;
```

### CLI 命令

```bash
# 启动 Web API 服务器
cargo run -- serve

# 指定端口和主机
cargo run -- serve --port 9000 --host 0.0.0.0
```

### REST API 端点

#### GET /health

健康检查端点

```bash
curl http://localhost:8080/health
```

响应:
```json
{
  "status": "ok",
  "timestamp": "2026-04-07T10:30:00Z"
}
```

#### POST /chat

AI 聊天端点

```bash
curl -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "分析这个 CAD 图纸",
    "session_id": "session-123"
  }'
```

请求体:
```json
{
  "message": "用户消息",
  "session_id": "会话 ID",
  "branch_id": "可选分支 ID"
}
```

响应:
```json
{
  "response": "AI 响应内容",
  "tool_chain": {...},
  "session_id": "session-123"
}
```

#### POST /upload

文件上传端点 (multipart/form-data)

```bash
curl -X POST http://localhost:8080/upload \
  -F "file=@drawing.svg" \
  -F "session_id=session-123"
```

响应:
```json
{
  "file_id": "uuid-123",
  "file_name": "drawing.svg",
  "file_size": 1024
}
```

#### GET /export/:format

导出为指定格式

```bash
# 导出为 STEP
curl http://localhost:8080/export/step?session_id=session-123 \
  -o output.step

# 导出为 IGES
curl http://localhost:8080/export/iges?session_id=session-123 \
  -o output.iges

# 导出为 SVG
curl http://localhost:8080/export/svg?session_id=session-123 \
  -o output.svg
```

支持的格式：`step`, `iges`, `svg`, `dxf`

#### GET /tools

获取可用工具列表

```bash
curl http://localhost:8080/tools
```

响应:
```json
{
  "tools": [
    {
      "name": "create_line",
      "description": "创建线段",
      "parameters": ["p1", "p2"]
    },
    {
      "name": "create_circle",
      "description": "创建圆",
      "parameters": ["center", "radius"]
    }
  ]
}
```

#### POST /tools/execute

执行工具

```bash
curl -X POST http://localhost:8080/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "create_line",
    "parameters": {
      "p1": {"x": 0, "y": 0},
      "p2": {"x": 10, "y": 10}
    }
  }'
```

响应:
```json
{
  "success": true,
  "result": {
    "id": "line-123",
    "type": "line",
    "geometry": {...}
  }
}
```

#### POST /constraints/apply

应用约束

```bash
curl -X POST http://localhost:8080/constraints/apply \
  -H "Content-Type: application/json" \
  -d '{
    "constraint": {
      "type": "parallel",
      "entities": ["line-1", "line-2"]
    }
  }'
```

响应:
```json
{
  "success": true,
  "constraint_id": "constraint-123"
}
```

#### POST /constraints/solve

求解约束系统

```bash
curl -X POST http://localhost:8080/constraints/solve \
  -H "Content-Type: application/json" \
  -d '{
    "session_id": "session-123"
  }'
```

响应:
```json
{
  "success": true,
  "solution": {...},
  "iterations": 5,
  "residual": 1e-6
}
```

### CORS 配置

服务器默认启用 CORS，允许来自 `http://localhost:3000` 的请求：

```rust
// web_server.rs 中的 CORS 配置
let cors = CorsLayer::new()
    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([CONTENT_TYPE, AUTHORIZATION]);
```

### 文件上传

支持 multipart/form-data 文件上传：

```rust
use axum::extract::Multipart;

async fn upload_handler(mut multipart: Multipart) -> Result<Json<UploadResponse>> {
    while let Some(field) = multipart.next_field().await? {
        let data = field.bytes().await?;
        // 处理文件数据
    }
    Ok(Json(UploadResponse { success: true }))
}
```

---

## web_ui

**Web UI 组件 (React/Three.js)**

### 状态管理 (Zustand)

```typescript
import { create } from 'zustand'

interface AppState {
  primitives: Primitive[]
  selectedIds: string[]
  darkMode: boolean
  chatMessages: ChatMessage[]
  isChatLoading: boolean
}

export const useStore = create<AppState>((set) => ({
  primitives: [],
  selectedIds: [],
  darkMode: false,
  chatMessages: [],
  isChatLoading: false,
  // actions...
}))
```

### 3D 渲染 (React Three Fiber)

```tsx
import { Canvas } from '@react-three/fiber'
import { OrbitControls } from '@react-three/drei'

function App() {
  return (
    <Canvas camera={{ position: [5, 5, 5], fov: 50 }}>
      <ambientLight intensity={0.5} />
      <directionalLight position={[10, 10, 5]} />
      <CADModel primitives={primitives} />
      <OrbitControls />
    </Canvas>
  )
}
```

### API 客户端

```typescript
// src/utils/api.ts
import axios from 'axios'

const api = axios.create({
  baseURL: 'http://localhost:8080',
})

export async function chat(message: string, sessionId: string) {
  const response = await api.post('/chat', { message, session_id: sessionId })
  return response.data
}

export async function uploadFile(file: File, sessionId: string) {
  const formData = new FormData()
  formData.append('file', file)
  formData.append('session_id', sessionId)
  const response = await api.post('/upload', formData)
  return response.data
}
```

### 可用组件

| 组件 | 路径 | 功能 |
|------|------|------|
| App | `src/App.tsx` | 主应用组件 |
| CADModel | `src/components/CADModel.tsx` | 3D 模型渲染 |
| ChatPanel | `src/components/ChatPanel.tsx` | AI 聊天界面 |
| FeatureTree | `src/components/FeatureTree.tsx` | 特征树面板 |
| PropertiesPanel | `src/components/PropertiesPanel.tsx` | 属性编辑器 |
| Toolbar | `src/components/Toolbar.tsx` | 工具栏 |

详见 [WEB_UI_GUIDE.md](WEB_UI_GUIDE.md)

---

*最后更新：2026-04-07 | 版本：v0.1.0 | Phase 1-7 完成*
