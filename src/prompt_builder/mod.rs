//! 结构化提示词构造器
//!
//! 将几何分析结果动态生成为结构化提示词，注入大模型上下文
//!
//! # 核心功能
//!
//! - 将基元、约束、校验结果组织为结构化 prompt
//! - 支持多种提示词模板（分析、推理、验证等）
//! - 动态注入精准几何信息
//! - 生成可解释的推理引导
//!
//! # 提示词结构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  System Prompt                                           │
//! │  "你是一个 CAD 几何推理专家..."                           │
//! └─────────────────────────────────────────────────────────┘
//!  │
//!  ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Context: 几何基元                                       │
//! │  - 识别出 15 个基元：8 条线段、4 个圆、3 个多边形...           │
//! │  - 坐标范围：[0, 100] x [0, 100]...                     │
//! └─────────────────────────────────────────────────────────┘
//!  │
//!  ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Context: 几何约束                                       │
//! │  - 平行关系：5 对 (line_0 ∥ line_2, ...)                 │
//! │  - 垂直关系：3 对 (line_0 ⊥ line_1, ...)                 │
//! │  - 相切关系：2 对 ...                                    │
//! └─────────────────────────────────────────────────────────┘
//!  │
//!  ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Context: 校验结果                                       │
//! │  - 约束合法性：✓ 无冲突                                  │
//! │  - 几何完整性：✓ 所有多边形闭合                          │
//! └─────────────────────────────────────────────────────────┘
//!  │
//!  ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Task: 推理任务                                          │
//! │  "请分析这个户型图，识别所有房间并计算面积"               │
//! └─────────────────────────────────────────────────────────┘
//!  │
//!  ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │  Guidance: 推理引导                                      │
//! │  "请按以下步骤推理：1. 识别封闭区域 2. 排除外边界..."     │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::prompt_builder::{PromptBuilder, PromptConfig};
//! use cadagent::prelude::*;
//!
//! // 创建一些测试基元
//! let primitives = vec![
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 100.0])),
//! ];
//!
//! let config = PromptConfig::default();
//! let builder = PromptBuilder::new(config);
//!
//! let prompt = builder.build_analysis_prompt(&primitives, &[], None);
//!
//! println!("{}", prompt.full_prompt);
//! // 将 prompt 送入 VLM 模型
//! ```

use crate::cad_reasoning::GeometricRelation;
use crate::cad_verifier::VerificationResult;
use crate::geometry::primitives::Primitive;
use serde::{Deserialize, Serialize};
use tokitai::tool;

/// 提示词模板类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptTemplate {
    /// 几何分析
    Analysis,
    /// 几何推理
    Reasoning,
    /// 约束验证
    Verification,
    /// 房间检测
    RoomDetection,
    /// 尺寸测量
    DimensionMeasurement,
    /// 自定义
    Custom,
}

/// 提示词配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    /// 提示词模板类型
    pub template: PromptTemplate,
    /// 是否包含基元详情
    pub include_primitive_details: bool,
    /// 是否包含约束列表
    pub include_constraints: bool,
    /// 是否包含校验结果
    pub include_verification: bool,
    /// 是否包含推理引导
    pub include_reasoning_guidance: bool,
    /// 是否包含坐标信息
    pub include_coordinate_info: bool,
    /// 最大基元数量（超过则简化）
    pub max_primitives_display: usize,
    /// 语言
    pub language: String,
    /// 是否启用摘要模式（当基元数量过多时自动启用）
    pub enable_summary_mode: bool,
    /// 触发摘要模式的基元数量阈值
    pub summary_mode_threshold: usize,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            template: PromptTemplate::Analysis,
            include_primitive_details: true,
            include_constraints: true,
            include_verification: true,
            include_reasoning_guidance: true,
            include_coordinate_info: true,
            max_primitives_display: 50,
            language: "zh-CN".to_string(),
            enable_summary_mode: true,
            summary_mode_threshold: 100, // 超过 100 个基元时启用摘要模式
        }
    }
}

/// 结构化提示词
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredPrompt {
    /// 系统提示
    pub system_prompt: String,
    /// 用户提示
    pub user_prompt: String,
    /// 完整提示
    pub full_prompt: String,
    /// 提示词元数据
    pub metadata: PromptMetadata,
}

/// 提示词元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    /// 基元数量
    pub primitive_count: usize,
    /// 约束数量
    pub constraint_count: usize,
    /// 提示词长度
    pub prompt_length: usize,
    /// 使用的模板
    pub template: PromptTemplate,
    /// 注入的上下文类型
    pub injected_context: Vec<String>,
}

/// 提示词构造器
#[derive(Debug, Clone)]
pub struct PromptBuilder {
    config: PromptConfig,
}

impl PromptBuilder {
    /// 创建新的提示词构造器
    pub fn new(config: PromptConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建构造器
    pub fn with_defaults() -> Self {
        Self::new(PromptConfig::default())
    }

    /// 构建几何分析提示词
    pub fn build_analysis_prompt(
        &self,
        primitives: &[Primitive],
        relations: &[GeometricRelation],
        verification: Option<&VerificationResult>,
    ) -> StructuredPrompt {
        let mut context_parts = Vec::new();
        let mut injected_context = Vec::new();

        // 1. 系统提示
        let system_prompt = self.get_system_prompt(PromptTemplate::Analysis);

        // 2. 基元上下文
        let primitive_context = self.build_primitive_context(primitives);
        context_parts.push(primitive_context.clone());
        injected_context.push("primitive_geometry".to_string());

        // 3. 约束上下文
        if self.config.include_constraints {
            let constraint_context = self.build_constraint_context(relations);
            context_parts.push(constraint_context);
            injected_context.push("geometric_constraints".to_string());
        }

        // 4. 校验上下文
        if self.config.include_verification {
            if let Some(ver_result) = verification {
                let verification_context = self.build_verification_context(ver_result);
                context_parts.push(verification_context);
                injected_context.push("constraint_verification".to_string());
            }
        }

        // 5. 推理引导
        let reasoning_guidance = if self.config.include_reasoning_guidance {
            self.get_reasoning_guidance(PromptTemplate::Analysis, primitives, relations)
        } else {
            String::new()
        };

        // 6. 组装用户提示
        let mut user_prompt_parts = Vec::new();

        user_prompt_parts.push("### 任务\n请分析这个 CAD 图纸的几何结构。".to_string());
        user_prompt_parts.push("".to_string());

        user_prompt_parts.push("### 几何上下文".to_string());
        user_prompt_parts.extend(context_parts);

        if !reasoning_guidance.is_empty() {
            user_prompt_parts.push("".to_string());
            user_prompt_parts.push("### 推理引导".to_string());
            user_prompt_parts.push(reasoning_guidance);
        }

        let user_prompt = user_prompt_parts.join("\n");

        // 7. 完整提示
        let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);
        let prompt_length = full_prompt.len();

        StructuredPrompt {
            system_prompt,
            user_prompt,
            full_prompt,
            metadata: PromptMetadata {
                primitive_count: primitives.len(),
                constraint_count: relations.len(),
                prompt_length,
                template: PromptTemplate::Analysis,
                injected_context,
            },
        }
    }

    /// 构建几何推理提示词
    pub fn build_reasoning_prompt(
        &self,
        primitives: &[Primitive],
        relations: &[GeometricRelation],
        verification: Option<&VerificationResult>,
        task: &str,
    ) -> StructuredPrompt {
        let mut context_parts = Vec::new();
        let mut injected_context = Vec::new();

        let system_prompt = self.get_system_prompt(PromptTemplate::Reasoning);

        // 基元上下文
        let primitive_context = self.build_primitive_context(primitives);
        context_parts.push(primitive_context);
        injected_context.push("primitive_geometry".to_string());

        // 约束上下文
        if self.config.include_constraints {
            let constraint_context = self.build_constraint_context(relations);
            context_parts.push(constraint_context);
            injected_context.push("geometric_constraints".to_string());
        }

        // 校验上下文
        if self.config.include_verification {
            if let Some(ver_result) = verification {
                let verification_context = self.build_verification_context(ver_result);
                context_parts.push(verification_context);
                injected_context.push("constraint_verification".to_string());
            }
        }

        let reasoning_guidance = if self.config.include_reasoning_guidance {
            self.get_reasoning_guidance(PromptTemplate::Reasoning, primitives, relations)
        } else {
            String::new()
        };

        let mut user_prompt_parts = Vec::new();

        user_prompt_parts.push(format!("### 任务\n{}", task));
        user_prompt_parts.push("".to_string());

        user_prompt_parts.push("### 几何上下文".to_string());
        user_prompt_parts.extend(context_parts);

        if !reasoning_guidance.is_empty() {
            user_prompt_parts.push("".to_string());
            user_prompt_parts.push("### 推理引导".to_string());
            user_prompt_parts.push(reasoning_guidance);
        }

        let user_prompt = user_prompt_parts.join("\n");

        let full_prompt = format!("{}\n\n{}", system_prompt, user_prompt);
        let prompt_length = full_prompt.len();

        StructuredPrompt {
            system_prompt,
            user_prompt,
            full_prompt,
            metadata: PromptMetadata {
                primitive_count: primitives.len(),
                constraint_count: relations.len(),
                prompt_length,
                template: PromptTemplate::Reasoning,
                injected_context,
            },
        }
    }

    /// 构建基元上下文（支持摘要模式）
    fn build_primitive_context(&self, primitives: &[Primitive]) -> String {
        let mut context = String::new();
        let total_count = primitives.len();

        // 检查是否启用摘要模式
        let is_summary_mode =
            self.config.enable_summary_mode && total_count > self.config.summary_mode_threshold;

        // 统计信息
        let mut line_count = 0;
        let mut circle_count = 0;
        let mut arc_count = 0;
        let mut polygon_count = 0;
        let mut rect_count = 0;
        let mut polyline_count = 0;
        let mut text_count = 0;

        for prim in primitives {
            match prim {
                Primitive::Point(_) => {}
                Primitive::Line(_) => line_count += 1,
                Primitive::Circle(_) => circle_count += 1,
                Primitive::Arc { .. } => arc_count += 1,
                Primitive::Polygon(_) => polygon_count += 1,
                Primitive::Rect(_) => rect_count += 1,
                Primitive::Polyline { .. } => polyline_count += 1,
                Primitive::Text { .. } => text_count += 1,
            }
        }

        context.push_str(&format!("- **基元统计**: 共 {} 个基元\n", total_count));

        // 摘要模式：只输出统计，不输出样本
        if is_summary_mode {
            context.push_str(&format!("  - 线段：{} 条\n", line_count));
            context.push_str(&format!("  - 圆：{} 个\n", circle_count));
            if polygon_count > 0 {
                context.push_str(&format!(
                    "  - 多边形：{} 个（可能表示房间或封闭区域）\n",
                    polygon_count
                ));
            }
            if arc_count > 0 {
                context.push_str(&format!("  - 弧：{} 个\n", arc_count));
            }
            if rect_count > 0 {
                context.push_str(&format!("  - 矩形：{} 个\n", rect_count));
            }
            if polyline_count > 0 {
                context.push_str(&format!("  - 折线：{} 条\n", polyline_count));
            }
            if text_count > 0 {
                context.push_str(&format!(
                    "  - 文本标注：{} 个（可能包含房间名称、尺寸等信息）\n",
                    text_count
                ));
            }
            context.push_str("\n> 📊 注：基元数量较多，已启用摘要模式以节省 Token。仅显示统计信息，未展示样本详情。");
            return context;
        }

        // 正常模式：输出统计 + 样本
        context.push_str(&format!("  - 线段：{} 条", line_count));
        if line_count > 0 {
            context.push_str(self.format_line_samples(primitives).as_str());
        }
        context.push('\n');

        context.push_str(&format!("  - 圆：{} 个", circle_count));
        if circle_count > 0 {
            context.push_str(self.format_circle_samples(primitives).as_str());
        }
        context.push('\n');

        if polygon_count > 0 {
            context.push_str(&format!(
                "  - 多边形：{} 个（可能表示房间或封闭区域）",
                polygon_count
            ));
            context.push_str(self.format_polygon_samples(primitives).as_str());
            context.push('\n');
        }

        if arc_count > 0 {
            context.push_str(&format!("  - 弧：{} 个\n", arc_count));
        }

        if rect_count > 0 {
            context.push_str(&format!("  - 矩形：{} 个\n", rect_count));
        }

        if polyline_count > 0 {
            context.push_str(&format!("  - 折线：{} 条\n", polyline_count));
        }

        if text_count > 0 {
            context.push_str(&format!(
                "  - 文本标注：{} 个（可能包含房间名称、尺寸等信息）",
                text_count
            ));
            context.push_str(self.format_text_samples(primitives).as_str());
            context.push('\n');
        }

        // 坐标范围
        if self.config.include_coordinate_info {
            if let Some(range) = self.compute_coordinate_range(primitives) {
                context.push_str(&format!(
                    "\n- **坐标范围**: x=[{:.2}, {:.2}], y=[{:.2}, {:.2}]",
                    range.0, range.1, range.2, range.3
                ));
            }
        }

        // 详细基元列表（如果启用且数量不多）
        if self.config.include_primitive_details
            && primitives.len() <= self.config.max_primitives_display
        {
            context.push_str("\n\n- **详细基元列表**:\n");
            for (id, prim) in primitives.iter().take(20).enumerate() {
                context.push_str(&format!("  - [{}] {}\n", id, self.format_primitive(prim)));
            }
            if primitives.len() > 20 {
                context.push_str(&format!("  ... 还有 {} 个基元\n", primitives.len() - 20));
            }
        }

        context
    }

    /// 构建约束上下文
    fn build_constraint_context(&self, relations: &[GeometricRelation]) -> String {
        let mut context = String::new();

        // 统计
        let mut parallel_count = 0;
        let mut perpendicular_count = 0;
        let mut collinear_count = 0;
        let mut tangent_count = 0;
        let mut concentric_count = 0;
        let mut connected_count = 0;

        for rel in relations {
            match rel {
                GeometricRelation::Parallel { .. } => parallel_count += 1,
                GeometricRelation::Perpendicular { .. } => perpendicular_count += 1,
                GeometricRelation::Collinear { .. } => collinear_count += 1,
                GeometricRelation::TangentLineCircle { .. }
                | GeometricRelation::TangentCircleCircle { .. } => tangent_count += 1,
                GeometricRelation::Concentric { .. } => concentric_count += 1,
                GeometricRelation::Connected { .. } => connected_count += 1,
                _ => {}
            }
        }

        context.push_str(&format!(
            "- **约束统计**: 共 {} 个几何关系\n",
            relations.len()
        ));

        if parallel_count > 0 {
            context.push_str(&format!("  - 平行关系：{} 对\n", parallel_count));
            context.push_str(&self.format_parallel_samples(relations));
        }

        if perpendicular_count > 0 {
            context.push_str(&format!("  - 垂直关系：{} 对\n", perpendicular_count));
            context.push_str(&self.format_perpendicular_samples(relations));
        }

        if collinear_count > 0 {
            context.push_str(&format!("  - 共线关系：{} 对\n", collinear_count));
        }

        if tangent_count > 0 {
            context.push_str(&format!("  - 相切关系：{} 对\n", tangent_count));
        }

        if concentric_count > 0 {
            context.push_str(&format!("  - 同心关系：{} 对\n", concentric_count));
        }

        if connected_count > 0 {
            context.push_str(&format!(
                "  - 连接关系：{} 对（表示基元共享端点）\n",
                connected_count
            ));
        }

        context
    }

    /// 构建校验上下文
    fn build_verification_context(&self, verification: &VerificationResult) -> String {
        let mut context = String::new();

        context.push_str(&format!(
            "- **合法性校验**: {}\n",
            if verification.is_valid {
                "✓ 通过"
            } else {
                "✗ 未通过"
            }
        ));

        if !verification.conflicts.is_empty() {
            context.push_str(&format!(
                "  - ⚠ 发现 {} 个约束冲突\n",
                verification.conflicts.len()
            ));
        }

        if !verification.geometry_issues.is_empty() {
            context.push_str(&format!(
                "  - ⚠ 发现 {} 个几何问题\n",
                verification.geometry_issues.len()
            ));
        }

        if !verification.redundant_constraints.is_empty() {
            context.push_str(&format!(
                "  - 发现 {} 个冗余约束\n",
                verification.redundant_constraints.len()
            ));
        }

        if !verification.fix_suggestions.is_empty() {
            context.push_str(&format!(
                "  - 提供 {} 条修复建议\n",
                verification.fix_suggestions.len()
            ));
        }

        context.push_str(&format!(
            "  - 总体评分：{:.1}/1.0",
            verification.overall_score
        ));

        context
    }

    /// 获取系统提示
    fn get_system_prompt(&self, template: PromptTemplate) -> String {
        match template {
            PromptTemplate::Analysis => {
                "你是一个 CAD 几何推理专家。你将收到经过精确计算的几何基元和约束关系，\
                 请基于这些结构化信息进行推理分析。你的推理应该：\n\
                 1. 基于给定的几何事实，不臆测\n\
                 2. 逻辑清晰，分步骤推理\n\
                 3. 输出可解释、可验证的结论"
                    .to_string()
            }
            PromptTemplate::Reasoning => {
                "你是一个 CAD 几何推理专家。你将收到精确的几何数据和约束关系，\
                 请根据任务要求进行推理。注意：\n\
                 1. 所有几何数据已经过算法验证，是可靠的事实\n\
                 2. 请基于这些事实进行逻辑推理\n\
                 3. 推理过程应该分步骤、可解释\n\
                 4. 如有不确定性，请明确指出"
                    .to_string()
            }
            _ => "你是一个 CAD 几何推理助手，请基于给定的几何信息进行分析。".to_string(),
        }
    }

    /// 获取推理引导
    fn get_reasoning_guidance(
        &self,
        template: PromptTemplate,
        primitives: &[Primitive],
        relations: &[GeometricRelation],
    ) -> String {
        match template {
            PromptTemplate::Analysis => "建议按以下步骤分析：\n\
                 1. 观察基元类型和数量分布\n\
                 2. 分析几何约束关系（平行、垂直等）\n\
                 3. 识别可能的功能区域（如房间、墙体等）\n\
                 4. 总结几何结构特征"
                .to_string(),
            PromptTemplate::Reasoning => {
                // 根据基元和约束动态生成引导
                let has_polygons = primitives
                    .iter()
                    .any(|p| matches!(p, Primitive::Polygon(_)));
                let has_perpendicular = relations
                    .iter()
                    .any(|r| matches!(r, GeometricRelation::Perpendicular { .. }));

                let mut guidance = Vec::new();

                if has_polygons {
                    guidance.push("1. 识别所有封闭区域（多边形）");
                }
                if has_perpendicular {
                    guidance.push("2. 注意垂直关系，这可能表示直角墙角");
                }
                guidance.push("3. 基于约束关系推导几何特征");
                guidance.push("4. 输出结构化结论");

                guidance.join("\n")
            }
            _ => String::new(),
        }
    }

    /// 格式化基元
    fn format_primitive(&self, prim: &Primitive) -> String {
        match prim {
            Primitive::Point(p) => format!("点 ({:.2}, {:.2})", p.x, p.y),
            Primitive::Line(line) => format!(
                "线段 [{:.2}, {:.2}] → [{:.2}, {:.2}] (长度={:.2})",
                line.start.x,
                line.start.y,
                line.end.x,
                line.end.y,
                line.length()
            ),
            Primitive::Circle(circle) => format!(
                "圆 中心=({:.2}, {:.2}), 半径={:.2}",
                circle.center.x, circle.center.y, circle.radius
            ),
            Primitive::Polygon(poly) => format!(
                "多边形 ({}个顶点，面积={:.2})",
                poly.vertices.len(),
                poly.area()
            ),
            Primitive::Rect(rect) => format!(
                "矩形 [{:.2}, {:.2}] × [{:.2}, {:.2}]",
                rect.min.x, rect.min.y, rect.max.x, rect.max.y
            ),
            Primitive::Polyline { points, closed } => format!(
                "折线 ({}个点，{})",
                points.len(),
                if *closed { "闭合" } else { "开放" }
            ),
            Primitive::Arc {
                center,
                radius,
                start_angle,
                end_angle,
            } => format!(
                "弧 中心=({:.2}, {:.2}), 半径={:.2}, 角度=[{:.1}°, {:.1}°]",
                center.x,
                center.y,
                radius,
                start_angle.to_degrees(),
                end_angle.to_degrees()
            ),
            Primitive::Text {
                content, position, ..
            } => format!(
                "文本 \"{}\" @ ({:.2}, {:.2})",
                content, position.x, position.y
            ),
        }
    }

    /// 格式化线段样本
    fn format_line_samples(&self, primitives: &[Primitive]) -> String {
        let lines: Vec<_> = primitives
            .iter()
            .filter_map(|p| {
                if let Primitive::Line(line) = p {
                    Some(line)
                } else {
                    None
                }
            })
            .take(3)
            .collect();

        if lines.is_empty() {
            return String::new();
        }

        let samples = lines
            .iter()
            .map(|l| {
                format!(
                    "[{:.1},{:.1}]→[{:.1},{:.1}]",
                    l.start.x, l.start.y, l.end.x, l.end.y
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("（样本：{}）", samples)
    }

    /// 格式化圆样本
    fn format_circle_samples(&self, primitives: &[Primitive]) -> String {
        let circles: Vec<_> = primitives
            .iter()
            .filter_map(|p| {
                if let Primitive::Circle(circle) = p {
                    Some(circle)
                } else {
                    None
                }
            })
            .take(3)
            .collect();

        if circles.is_empty() {
            return String::new();
        }

        let samples = circles
            .iter()
            .map(|c| {
                format!(
                    "中心=({:.1},{:.1}),r={:.1}",
                    c.center.x, c.center.y, c.radius
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!("（样本：{}）", samples)
    }

    /// 格式化多边形样本
    fn format_polygon_samples(&self, primitives: &[Primitive]) -> String {
        let polygons: Vec<_> = primitives
            .iter()
            .filter_map(|p| {
                if let Primitive::Polygon(poly) = p {
                    Some(poly)
                } else {
                    None
                }
            })
            .take(3)
            .collect();

        if polygons.is_empty() {
            return String::new();
        }

        let samples = polygons
            .iter()
            .map(|p| format!("{}顶点，面积={:.1}", p.vertices.len(), p.area()))
            .collect::<Vec<_>>()
            .join(", ");

        format!("（样本：{}）", samples)
    }

    /// 格式化文本样本
    fn format_text_samples(&self, primitives: &[Primitive]) -> String {
        let texts: Vec<_> = primitives
            .iter()
            .filter_map(|p| {
                if let Primitive::Text { content, .. } = p {
                    Some(content.as_str())
                } else {
                    None
                }
            })
            .take(5)
            .collect();

        if texts.is_empty() {
            return String::new();
        }

        format!("（样本：{}）", texts.join(", "))
    }

    /// 格式化平行关系样本
    fn format_parallel_samples(&self, relations: &[GeometricRelation]) -> String {
        let samples: Vec<_> = relations
            .iter()
            .filter_map(|r| {
                if let GeometricRelation::Parallel {
                    line1_id, line2_id, ..
                } = r
                {
                    Some(format!("line_{} ∥ line_{}", line1_id, line2_id))
                } else {
                    None
                }
            })
            .take(5)
            .collect();

        if samples.is_empty() {
            return String::new();
        }

        format!("    例如：{}\n", samples.join(", "))
    }

    /// 格式化垂直关系样本
    fn format_perpendicular_samples(&self, relations: &[GeometricRelation]) -> String {
        let samples: Vec<_> = relations
            .iter()
            .filter_map(|r| {
                if let GeometricRelation::Perpendicular {
                    line1_id, line2_id, ..
                } = r
                {
                    Some(format!("line_{} ⊥ line_{}", line1_id, line2_id))
                } else {
                    None
                }
            })
            .take(5)
            .collect();

        if samples.is_empty() {
            return String::new();
        }

        format!("    例如：{}\n", samples.join(", "))
    }

    /// 计算坐标范围
    fn compute_coordinate_range(&self, primitives: &[Primitive]) -> Option<(f64, f64, f64, f64)> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for prim in primitives {
            if let Some(bbox) = prim.bounding_box() {
                min_x = min_x.min(bbox.min.x);
                min_y = min_y.min(bbox.min.y);
                max_x = max_x.max(bbox.max.x);
                max_y = max_y.max(bbox.max.y);
            }
        }

        if min_x.is_finite() {
            Some((min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }
}

/// 提示词构造工具（tokitai 工具封装）
#[derive(Default, Clone)]
pub struct PromptBuilderTools;

#[tool]
impl PromptBuilderTools {
    /// 构建几何分析提示词
    ///
    /// # 参数
    ///
    /// * `primitives_json` - 基元列表（JSON 格式）
    /// * `relations_json` - 约束关系列表（JSON 格式）
    /// * `verification_json` - 可选的校验结果（JSON 格式）
    /// * `config_json` - 可选的配置（JSON 格式）
    ///
    /// # 返回
    ///
    /// 结构化提示词（包含 system_prompt、user_prompt、full_prompt）
    #[tool(name = "cad_build_analysis_prompt")]
    pub fn build_analysis_prompt(
        &self,
        primitives_json: String,
        relations_json: String,
        verification_json: Option<String>,
        config_json: Option<String>,
    ) -> serde_json::Value {
        let primitives: Vec<Primitive> = match serde_json::from_str(&primitives_json) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析基元失败：{}", e)
                });
            }
        };

        let relations: Vec<GeometricRelation> = match serde_json::from_str(&relations_json) {
            Ok(r) => r,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析关系失败：{}", e)
                });
            }
        };

        let verification: Option<VerificationResult> =
            verification_json.and_then(|s| serde_json::from_str(&s).ok());

        let config: PromptConfig = config_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let builder = PromptBuilder::new(config);
        let prompt = builder.build_analysis_prompt(&primitives, &relations, verification.as_ref());

        serde_json::json!({
            "success": true,
            "prompt": prompt,
            "metadata": prompt.metadata
        })
    }

    /// 构建几何推理提示词
    #[tool(name = "cad_build_reasoning_prompt")]
    pub fn build_reasoning_prompt(
        &self,
        primitives_json: String,
        relations_json: String,
        task: String,
        verification_json: Option<String>,
        config_json: Option<String>,
    ) -> serde_json::Value {
        let primitives: Vec<Primitive> = match serde_json::from_str(&primitives_json) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析基元失败：{}", e)
                });
            }
        };

        let relations: Vec<GeometricRelation> = match serde_json::from_str(&relations_json) {
            Ok(r) => r,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析关系失败：{}", e)
                });
            }
        };

        let verification: Option<VerificationResult> =
            verification_json.and_then(|s| serde_json::from_str(&s).ok());

        let config: PromptConfig = config_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let builder = PromptBuilder::new(config);
        let prompt =
            builder.build_reasoning_prompt(&primitives, &relations, verification.as_ref(), &task);

        serde_json::json!({
            "success": true,
            "prompt": prompt,
            "metadata": prompt.metadata
        })
    }

    /// 获取提示词构造器信息
    #[tool(name = "cad_get_prompt_builder_info")]
    pub fn get_builder_info(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "prompt_builder",
            "description": "结构化提示词构造器：将几何分析结果动态生成为 LLM 提示词",
            "templates": ["analysis", "reasoning", "verification", "room_detection", "dimension_measurement"],
            "injected_context": [
                "primitive_geometry",
                "geometric_constraints",
                "constraint_verification"
            ],
            "output_format": {
                "system_prompt": "系统提示词",
                "user_prompt": "用户提示词",
                "full_prompt": "完整提示词",
                "metadata": "提示词元数据"
            },
            "usage": "将几何工具的输出作为输入，生成结构化 prompt 注入 VLM"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::primitives::{Circle, Line, Point, Polygon};

    #[test]
    fn test_build_analysis_prompt() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 1.0])),
            Primitive::Circle(Circle::from_coords([0.5, 0.5], 0.1)),
        ];

        let relations = vec![GeometricRelation::Perpendicular {
            line1_id: 0,
            line2_id: 1,
            angle_diff: 0.0,
            confidence: 1.0,
        }];

        let builder = PromptBuilder::with_defaults();
        let prompt = builder.build_analysis_prompt(&primitives, &relations, None);

        assert!(!prompt.system_prompt.is_empty());
        assert!(!prompt.user_prompt.is_empty());
        assert!(prompt.metadata.primitive_count == 3);
        assert!(prompt.metadata.constraint_count == 1);
    }

    #[test]
    fn test_build_reasoning_prompt() {
        let primitives = vec![Primitive::Polygon(Polygon::from_coords(vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ]))];

        let relations = vec![];

        let builder = PromptBuilder::with_defaults();
        let prompt =
            builder.build_reasoning_prompt(&primitives, &relations, None, "计算这个房间的面积");

        assert!(prompt.full_prompt.contains("计算这个房间的面积"));
        assert!(prompt.metadata.template == PromptTemplate::Reasoning);
    }

    #[test]
    fn test_prompt_builder_tool() {
        let primitives = vec![Primitive::Point(Point::origin())];
        let relations: Vec<GeometricRelation> = vec![];

        let tools = PromptBuilderTools;
        let result = tools.build_analysis_prompt(
            serde_json::to_string(&primitives).unwrap(),
            serde_json::to_string(&relations).unwrap(),
            None,
            None,
        );

        assert!(result["success"].as_bool().unwrap_or(false));
        assert!(result["prompt"]["full_prompt"].as_str().is_some());
    }
}
