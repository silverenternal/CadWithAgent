//! 几何验证报告生成器
//!
//! 生成可追溯的几何验证报告，包含完整的推理链和置信度评分
//!
//! # 功能
//!
//! - **可追溯报告**: 记录每个几何结论的推理依据
//! - **置信度传播**: 基于约束置信度计算整体可信度
//! - **可视化输出**: 支持 JSON、Markdown 格式导出
//! - **多格式导出**: JSON / Markdown / PlainText
//!
//! # 快速开始
//!
//! ## 基本使用
//!
//! ```rust,no_run
//! use cadagent::cad_verifier::report::{VerificationReport, ReportFormat};
//!
//! // 创建空报告
//! let report = VerificationReport::new("户型图分析");
//!
//! // 导出为 Markdown
//! let markdown = report.export(ReportFormat::Markdown);
//! println!("{}", markdown);
//! ```
//!
//! ## 从验证结果生成报告
//!
//! ```rust,no_run
//! use std::time::Instant;
//! use cadagent::cad_verifier::{
//!     ConstraintVerifier, VerifierConfig,
//!     report::{VerificationReport, ReportFormat},
//! };
//! use cadagent::geometry::primitives::{Line, Primitive};
//! use cadagent::cad_reasoning::GeometricRelation;
//!
//! // 准备几何数据
//! let primitives = vec![
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 100.0])),
//! ];
//!
//! let relations = vec![
//!     GeometricRelation::Perpendicular {
//!         line1_id: 0,
//!         line2_id: 1,
//!         angle_diff: 0.0,
//!         confidence: 0.95,
//!     },
//! ];
//!
//! // 执行验证
//! let verifier = ConstraintVerifier::new(VerifierConfig::default());
//! let start_time = Instant::now();
//! let result = verifier.verify(&primitives, &relations).unwrap();
//!
//! // 生成报告
//! let report = VerificationReport::from_verification_result(
//!     &result,
//!     &primitives,
//!     &relations,
//!     start_time,
//! );
//!
//! // 导出为不同格式
//! let json = report.export(ReportFormat::Json);
//! let markdown = report.export(ReportFormat::Markdown);
//! let text = report.export(ReportFormat::PlainText);
//! ```
//!
//! ## 获取可追溯推理链
//!
//! ```rust
//! use cadagent::cad_verifier::report::VerificationReport;
//!
//! let report = VerificationReport::new("测试报告");
//! let chain = report.get_traceable_chain();
//!
//! for step in chain {
//!     println!("推理步骤：{}", step);
//! }
//! ```
//!
//! # 报告结构
//!
//! 报告包含以下条目类型：
//!
//! 1. **基元提取** (`PrimitiveExtraction`): 记录从原始数据提取的几何基元
//! 2. **关系推理** (`RelationInference`): 记录推断出的几何关系（平行、垂直等）
//! 3. **约束校验** (`ConstraintValidation`): 记录约束条件的验证结果
//! 4. **冲突检测** (`ConflictDetection`): 记录检测到的几何冲突
//! 5. **几何问题** (`GeometryIssue`): 记录几何异常（零长度线段、无效半径等）
//! 6. **修复建议** (`FixSuggestion`): 提供修复问题的建议
//! 7. **结论** (`Conclusion`): 总体评分和验证结论

use crate::cad_reasoning::GeometricRelation;
use crate::cad_verifier::{Conflict, GeometryIssue, VerificationResult};
use crate::geometry::primitives::Primitive;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// 报告格式
///
/// 用于指定报告导出的格式
///
/// # 示例
///
/// ```rust
/// use cadagent::cad_verifier::report::{VerificationReport, ReportFormat};
///
/// let report = VerificationReport::new("测试报告");
///
/// // 导出为 JSON
/// let json = report.export(ReportFormat::Json);
/// assert!(json.contains("测试报告"));
///
/// // 导出为 Markdown
/// let md = report.export(ReportFormat::Markdown);
/// assert!(md.contains("# 测试报告"));
///
/// // 导出为纯文本
/// let text = report.export(ReportFormat::PlainText);
/// assert!(text.contains("测试报告"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// JSON 格式，适合机器解析
    Json,
    /// Markdown 格式，适合阅读和展示
    Markdown,
    /// 纯文本格式，适合日志记录
    PlainText,
}

/// 验证报告条目
///
/// 报告中的单个记录项，包含完整的上下文信息
///
/// # 字段说明
///
/// - `entry_type`: 条目类型（基元提取、关系推理、冲突检测等）
/// - `title`: 条目标题
/// - `description`: 详细描述
/// - `confidence`: 置信度 (0-1)，1.0 表示完全确定
/// - `primitive_ids`: 涉及的几何基元 ID 列表
/// - `timestamp_ms`: 时间戳（从验证开始计算的毫秒数）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEntry {
    /// 条目类型
    pub entry_type: EntryType,
    /// 标题
    pub title: String,
    /// 描述
    pub description: String,
    /// 置信度 (0-1)
    pub confidence: f64,
    /// 涉及的基元 ID
    pub primitive_ids: Vec<usize>,
    /// 时间戳 (ms)
    pub timestamp_ms: u64,
}

/// 条目类型
///
/// 标识报告中条目的类别
///
/// # 变体说明
///
/// - `PrimitiveExtraction`: 基元提取，记录从原始数据提取的几何基元
/// - `RelationInference`: 关系推理，记录推断出的几何关系
/// - `ConstraintValidation`: 约束校验，记录约束条件的验证结果
/// - `ConflictDetection`: 冲突检测，记录检测到的几何冲突
/// - `GeometryIssue`: 几何问题，记录几何异常
/// - `FixSuggestion`: 修复建议，提供修复问题的建议
/// - `Conclusion`: 结论，总体评分和验证结论
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    /// 基元提取
    PrimitiveExtraction,
    /// 关系推理
    RelationInference,
    /// 约束校验
    ConstraintValidation,
    /// 冲突检测
    ConflictDetection,
    /// 几何问题
    GeometryIssue,
    /// 修复建议
    FixSuggestion,
    /// 结论
    Conclusion,
}

/// 几何验证报告
///
/// 完整的验证过程记录，支持可追溯性查询
///
/// # 字段说明
///
/// - `title`: 报告标题
/// - `generated_at`: 报告生成时间（Unix 时间戳）
/// - `total_time_ms`: 验证总耗时（毫秒）
/// - `entries`: 报告条目列表，按时间顺序记录推理过程
/// - `overall_score`: 总体评分 (0-1)
/// - `primitive_count`: 基元数量
/// - `relation_count`: 关系数量
/// - `conflict_count`: 冲突数量
/// - `issue_count`: 几何问题数量
/// - `suggestion_count`: 修复建议数量
///
/// # 示例
///
/// ```rust
/// use cadagent::cad_verifier::report::VerificationReport;
///
/// let mut report = VerificationReport::new("户型图验证");
/// assert_eq!(report.title, "户型图验证");
/// assert!(report.entries.is_empty());
/// assert_eq!(report.overall_score, 0.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    /// 报告标题
    pub title: String,
    /// 报告生成时间
    pub generated_at: String,
    /// 总耗时 (ms)
    pub total_time_ms: u64,
    /// 报告条目
    pub entries: Vec<ReportEntry>,
    /// 总体评分
    pub overall_score: f64,
    /// 基元数量
    pub primitive_count: usize,
    /// 关系数量
    pub relation_count: usize,
    /// 冲突数量
    pub conflict_count: usize,
    /// 几何问题数量
    pub issue_count: usize,
    /// 修复建议数量
    pub suggestion_count: usize,
}

impl VerificationReport {
    /// 创建新的验证报告
    pub fn new(title: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            title: title.to_string(),
            generated_at: timestamp.to_string(),
            total_time_ms: 0,
            entries: Vec::new(),
            overall_score: 0.0,
            primitive_count: 0,
            relation_count: 0,
            conflict_count: 0,
            issue_count: 0,
            suggestion_count: 0,
        }
    }

    /// 从验证结果生成报告
    pub fn from_verification_result(
        result: &VerificationResult,
        primitives: &[Primitive],
        relations: &[GeometricRelation],
        elapsed_time: Instant,
    ) -> Self {
        let mut report = Self::new("几何验证报告");
        let start_time = elapsed_time;

        // 1. 基元提取条目
        let primitive_entry = ReportEntry {
            entry_type: EntryType::PrimitiveExtraction,
            title: "基元提取".to_string(),
            description: format!("成功提取 {} 个几何基元", primitives.len()),
            confidence: 1.0,
            primitive_ids: (0..primitives.len()).collect(),
            timestamp_ms: start_time.elapsed().as_millis() as u64,
        };
        report.entries.push(primitive_entry);

        // 2. 关系推理条目
        let relation_entry = ReportEntry {
            entry_type: EntryType::RelationInference,
            title: "几何关系推理".to_string(),
            description: format!("推断出 {} 个几何关系", relations.len()),
            confidence: relations
                .iter()
                .map(|r| match r {
                    GeometricRelation::Parallel { confidence, .. }
                    | GeometricRelation::Perpendicular { confidence, .. }
                    | GeometricRelation::Collinear { confidence, .. }
                    | GeometricRelation::Concentric { confidence, .. }
                    | GeometricRelation::TangentCircleCircle { confidence, .. }
                    | GeometricRelation::TangentLineCircle { confidence, .. }
                    | GeometricRelation::Connected { confidence, .. }
                    | GeometricRelation::Contains { confidence, .. }
                    | GeometricRelation::EqualDistance { confidence, .. }
                    | GeometricRelation::Symmetric { confidence, .. } => *confidence,
                })
                .fold(0.0, |a, b| a + b)
                / relations.len().max(1) as f64,
            primitive_ids: Vec::new(),
            timestamp_ms: start_time.elapsed().as_millis() as u64,
        };
        report.entries.push(relation_entry);

        // 3. 冲突检测条目
        for conflict in &result.conflicts {
            let (title, desc, ids, confidence) = match conflict {
                Conflict::ParallelPerpendicular {
                    line1_id,
                    line2_id,
                    parallel_confidence,
                    perpendicular_confidence,
                } => (
                    "冲突：平行与垂直矛盾".to_string(),
                    format!(
                        "线段 {} 和 {} 同时被约束为平行 (置信度 {:.2}) 和垂直 (置信度 {:.2})",
                        line1_id, line2_id, parallel_confidence, perpendicular_confidence
                    ),
                    vec![*line1_id, *line2_id],
                    (*parallel_confidence + *perpendicular_confidence) / 2.0,
                ),
                Conflict::ConcentricTangent {
                    circle1_id,
                    circle2_id,
                    concentric_confidence,
                    tangent_confidence,
                } => (
                    "冲突：同心与相切矛盾".to_string(),
                    format!(
                        "圆 {} 和 {} 同时被约束为同心 (置信度 {:.2}) 和相切 (置信度 {:.2})",
                        circle1_id, circle2_id, concentric_confidence, tangent_confidence
                    ),
                    vec![*circle1_id, *circle2_id],
                    (*concentric_confidence + *tangent_confidence) / 2.0,
                ),
                _ => (
                    "几何冲突".to_string(),
                    format!("{conflict:?}"),
                    Vec::new(),
                    0.5,
                ),
            };

            report.entries.push(ReportEntry {
                entry_type: EntryType::ConflictDetection,
                title,
                description: desc,
                confidence,
                primitive_ids: ids,
                timestamp_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // 4. 几何问题条目
        for issue in &result.geometry_issues {
            let (title, desc, ids, confidence) = match issue {
                GeometryIssue::ZeroLengthLine { line_id } => (
                    "几何问题：零长度线段".to_string(),
                    format!("线段 {} 的长度为零", line_id),
                    vec![*line_id],
                    1.0,
                ),
                GeometryIssue::InvalidCircleRadius { circle_id, radius } => (
                    "几何问题：无效的圆半径".to_string(),
                    format!("圆 {} 的半径为 {:.4}", circle_id, radius),
                    vec![*circle_id],
                    1.0,
                ),
                _ => (
                    "几何问题".to_string(),
                    format!("{issue:?}"),
                    Vec::new(),
                    1.0,
                ),
            };

            report.entries.push(ReportEntry {
                entry_type: EntryType::GeometryIssue,
                title,
                description: desc,
                confidence,
                primitive_ids: ids,
                timestamp_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // 5. 修复建议条目
        for suggestion in &result.fix_suggestions {
            report.entries.push(ReportEntry {
                entry_type: EntryType::FixSuggestion,
                title: "修复建议".to_string(),
                description: suggestion.suggested_action.clone(),
                confidence: 1.0 - (suggestion.difficulty as f64 / 5.0),
                primitive_ids: suggestion.affected_primitives.clone(),
                timestamp_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // 6. 结论条目
        report.entries.push(ReportEntry {
            entry_type: EntryType::Conclusion,
            title: if result.is_valid {
                "验证通过".to_string()
            } else {
                "验证失败".to_string()
            },
            description: format!(
                "总体评分：{:.2}/1.0, 发现 {} 冲突，{} 几何问题",
                result.overall_score,
                result.conflicts.len(),
                result.geometry_issues.len()
            ),
            confidence: result.overall_score,
            primitive_ids: Vec::new(),
            timestamp_ms: start_time.elapsed().as_millis() as u64,
        });

        // 填充统计信息
        report.total_time_ms = start_time.elapsed().as_millis() as u64;
        report.overall_score = result.overall_score;
        report.primitive_count = primitives.len();
        report.relation_count = relations.len();
        report.conflict_count = result.conflicts.len();
        report.issue_count = result.geometry_issues.len();
        report.suggestion_count = result.fix_suggestions.len();

        report
    }

    /// 导出为指定格式
    pub fn export(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::Json => {
                serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
            }
            ReportFormat::Markdown => self.export_markdown(),
            ReportFormat::PlainText => self.export_plain_text(),
        }
    }

    /// 导出为 Markdown 格式
    fn export_markdown(&self) -> String {
        let mut md = String::new();

        writeln!(&mut md, "# {}\n", self.title).unwrap();
        writeln!(&mut md, "**生成时间**: {}", self.generated_at).unwrap();
        writeln!(&mut md, "**总耗时**: {} ms\n", self.total_time_ms).unwrap();

        // 摘要
        writeln!(&mut md, "## 摘要\n").unwrap();
        writeln!(&mut md, "| 指标 | 数值 |").unwrap();
        writeln!(&mut md, "|------|------|").unwrap();
        writeln!(&mut md, "| 基元数量 | {} |", self.primitive_count).unwrap();
        writeln!(&mut md, "| 关系数量 | {} |", self.relation_count).unwrap();
        writeln!(&mut md, "| 冲突数量 | {} |", self.conflict_count).unwrap();
        writeln!(&mut md, "| 几何问题 | {} |", self.issue_count).unwrap();
        writeln!(&mut md, "| 修复建议 | {} |", self.suggestion_count).unwrap();
        writeln!(
            &mut md,
            "| **总体评分** | **{:.2}/1.0** |\n",
            self.overall_score
        )
        .unwrap();

        // 详细条目
        writeln!(&mut md, "## 详细推理链\n").unwrap();
        for (i, entry) in self.entries.iter().enumerate() {
            let icon = match entry.entry_type {
                EntryType::PrimitiveExtraction => "📐",
                EntryType::RelationInference => "🔗",
                EntryType::ConflictDetection => "❌",
                EntryType::GeometryIssue => "⚠️",
                EntryType::FixSuggestion => "💡",
                EntryType::Conclusion => "✅",
                EntryType::ConstraintValidation => "✓",
            };

            writeln!(&mut md, "### {}. {} {}\n", i + 1, icon, entry.title).unwrap();
            writeln!(&mut md, "{}\n", entry.description).unwrap();
            writeln!(&mut md, "- **置信度**: {:.2}", entry.confidence).unwrap();
            if !entry.primitive_ids.is_empty() {
                writeln!(&mut md, "- **涉及基元**: {:?}", entry.primitive_ids).unwrap();
            }
            writeln!(&mut md, "- **时间**: {} ms\n", entry.timestamp_ms).unwrap();
        }

        // 可追溯性说明
        writeln!(&mut md, "## 可追溯性说明\n").unwrap();
        writeln!(&mut md, "本报告记录了从基元提取到最终结论的完整推理过程。").unwrap();
        writeln!(&mut md, "每个结论都有明确的几何依据和置信度评分。").unwrap();
        writeln!(&mut md, "可通过基元 ID 追溯到具体的几何实体。\n").unwrap();

        md
    }

    /// 导出为纯文本格式
    fn export_plain_text(&self) -> String {
        let mut text = String::new();

        writeln!(&mut text, "{}", self.title).unwrap();
        writeln!(&mut text, "================\n").unwrap();
        writeln!(&mut text, "生成时间：{}", self.generated_at).unwrap();
        writeln!(&mut text, "总耗时：{} ms\n", self.total_time_ms).unwrap();

        writeln!(&mut text, "摘要:").unwrap();
        writeln!(&mut text, "  基元数量：{}", self.primitive_count).unwrap();
        writeln!(&mut text, "  关系数量：{}", self.relation_count).unwrap();
        writeln!(&mut text, "  冲突数量：{}", self.conflict_count).unwrap();
        writeln!(&mut text, "  几何问题：{}", self.issue_count).unwrap();
        writeln!(&mut text, "  修复建议：{}", self.suggestion_count).unwrap();
        writeln!(&mut text, "  总体评分：{:.2}/1.0\n", self.overall_score).unwrap();

        writeln!(&mut text, "推理链:").unwrap();
        for (i, entry) in self.entries.iter().enumerate() {
            writeln!(
                &mut text,
                "  {}. [{}] {} (置信度：{:.2})",
                i + 1,
                match entry.entry_type {
                    EntryType::PrimitiveExtraction => "基元",
                    EntryType::RelationInference => "关系",
                    EntryType::ConflictDetection => "冲突",
                    EntryType::GeometryIssue => "问题",
                    EntryType::FixSuggestion => "建议",
                    EntryType::Conclusion => "结论",
                    EntryType::ConstraintValidation => "校验",
                },
                entry.title,
                entry.confidence
            )
            .unwrap();
        }

        text
    }

    /// 获取可追溯推理链
    pub fn get_traceable_chain(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|e| {
                format!(
                    "{}: {} (置信度：{:.2})",
                    match e.entry_type {
                        EntryType::PrimitiveExtraction => "基元提取",
                        EntryType::RelationInference => "关系推理",
                        EntryType::ConflictDetection => "冲突检测",
                        EntryType::GeometryIssue => "几何问题",
                        EntryType::FixSuggestion => "修复建议",
                        EntryType::Conclusion => "结论",
                        EntryType::ConstraintValidation => "约束校验",
                    },
                    e.title,
                    e.confidence
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cad_verifier::{ConstraintVerifier, VerifierConfig};
    use crate::geometry::primitives::Line;

    #[test]
    fn test_report_generation() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 100.0])),
        ];

        let relations = vec![
            GeometricRelation::Parallel {
                line1_id: 0,
                line2_id: 1,
                angle_diff: 90.0,
                confidence: 0.9,
            },
            GeometricRelation::Perpendicular {
                line1_id: 0,
                line2_id: 1,
                angle_diff: 0.0,
                confidence: 0.9,
            },
        ];

        let verifier = ConstraintVerifier::new(VerifierConfig::default());
        let start_time = Instant::now();
        let result = verifier.verify(&primitives, &relations).unwrap();

        let report = VerificationReport::from_verification_result(
            &result,
            &primitives,
            &relations,
            start_time,
        );

        assert_eq!(report.primitive_count, 2);
        assert_eq!(report.relation_count, 2);
        assert!(report.conflict_count > 0);
        assert!(report.overall_score < 1.0);
    }

    #[test]
    fn test_report_export() {
        let report = VerificationReport::new("TestReport");

        let json = report.export(ReportFormat::Json);
        assert!(json.contains("TestReport"));

        let md = report.export(ReportFormat::Markdown);
        assert!(md.contains("# TestReport"));

        let text = report.export(ReportFormat::PlainText);
        assert!(text.contains("TestReport"));
    }

    #[test]
    fn test_traceable_chain() {
        let report = VerificationReport::new("测试报告");
        let chain = report.get_traceable_chain();
        assert!(chain.is_empty()); // 空报告
    }
}
