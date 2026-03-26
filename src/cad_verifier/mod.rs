//! CAD 约束合法性校验工具
//!
//! 校验几何约束是否冲突、是否合法，提供修复建议
//!
//! # 校验内容
//!
//! - **冲突检测**: 检测相互矛盾的约束（如既平行又垂直）
//! - **冗余检测**: 检测重复的约束
//! - **完整性检测**: 检测是否缺少必要约束
//! - **几何合法性**: 检测基元是否符合几何规则
//! - **数值稳定性**: 检测数值是否合理
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::cad_verifier::{ConstraintVerifier, VerifierConfig};
//! use cadagent::error::GeometryToleranceConfig;
//! use cadagent::prelude::*;
//!
//! // 创建一些测试基元
//! let primitives = vec![
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 100.0])),
//! ];
//!
//! let tolerance = GeometryToleranceConfig::default();
//! let config = VerifierConfig {
//!     tolerance,
//!     min_confidence_threshold: 0.5,
//!     detect_conflicts: true,
//!     detect_redundancy: true,
//!     detect_missing_constraints: false,
//!     detect_geometry_issues: true,
//!     coordinate_range_check: None,
//! };
//! let verifier = ConstraintVerifier::new(config);
//! let result = verifier.verify(&primitives, &[]).unwrap();
//!
//! if !result.is_valid {
//!     println!("发现 {} 个冲突", result.conflicts.len());
//!     for conflict in &result.conflicts {
//!         println!("冲突：{:?}", conflict);
//!     }
//! }
//! ```

use crate::cad_reasoning::GeometricRelation;
use crate::error::{CadAgentError, CadAgentResult, GeometryToleranceConfig};
use crate::geometry::primitives::{Line, Point, Polygon, Primitive};
use serde::{Deserialize, Serialize};
use tokitai::tool;

/// 约束校验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// 是否合法
    pub is_valid: bool,
    /// 冲突列表
    pub conflicts: Vec<Conflict>,
    /// 冗余约束
    pub redundant_constraints: Vec<RedundantConstraint>,
    /// 缺失约束建议
    pub missing_constraints: Vec<MissingConstraint>,
    /// 几何合法性问题
    pub geometry_issues: Vec<GeometryIssue>,
    /// 修复建议
    pub fix_suggestions: Vec<FixSuggestion>,
    /// 校验日志
    pub verification_log: Vec<String>,
    /// 总体评分 (0-1)
    pub overall_score: f64,
}

/// 冲突类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "conflict_type", rename_all = "snake_case")]
pub enum Conflict {
    /// 平行与垂直冲突
    ParallelPerpendicular {
        line1_id: usize,
        line2_id: usize,
        parallel_confidence: f64,
        perpendicular_confidence: f64,
    },
    /// 同心与相切冲突
    ConcentricTangent {
        circle1_id: usize,
        circle2_id: usize,
        concentric_confidence: f64,
        tangent_confidence: f64,
    },
    /// 连接点不一致
    ConnectionMismatch {
        primitive1_id: usize,
        primitive2_id: usize,
        expected_point: Point,
        actual_point: Point,
        distance: f64,
    },
    /// 多边形不闭合
    PolygonNotClosed {
        polygon_id: usize,
        gap_distance: f64,
    },
    /// 约束循环依赖
    CircularDependency {
        involved_primitives: Vec<usize>,
        description: String,
    },
}

/// 冗余约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedundantConstraint {
    /// 冗余的关系
    pub relation: GeometricRelation,
    /// 冗余原因
    pub reason: String,
    /// 是否可以安全移除
    pub safely_removable: bool,
}

/// 缺失约束
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingConstraint {
    /// 涉及的基元
    pub primitive_ids: Vec<usize>,
    /// 建议的约束类型
    pub suggested_type: String,
    /// 原因
    pub reason: String,
    /// 优先级 (0-1)
    pub priority: f64,
}

/// 几何合法性问题
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "issue_type", rename_all = "snake_case")]
pub enum GeometryIssue {
    /// 线段长度为 0
    ZeroLengthLine { line_id: usize },
    /// 圆半径为 0 或负数
    InvalidCircleRadius { circle_id: usize, radius: f64 },
    /// 多边形顶点不足
    InsufficientPolygonVertices {
        polygon_id: usize,
        vertex_count: usize,
    },
    /// 多边形自相交
    SelfIntersectingPolygon {
        polygon_id: usize,
        intersection_point: Point,
    },
    /// 坐标超出合理范围
    CoordinateOutOfRange {
        primitive_id: usize,
        coordinate: Point,
        expected_range: [f64; 4],
    },
    /// 数值精度问题
    NumericalPrecisionIssue {
        primitive_id: usize,
        description: String,
    },
}

/// 修复建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixSuggestion {
    /// 针对的问题
    pub issue_type: String,
    /// 涉及的基元 ID
    pub affected_primitives: Vec<usize>,
    /// 建议操作
    pub suggested_action: String,
    /// 修复难度 (1-5)
    pub difficulty: u8,
    /// 预期效果
    pub expected_outcome: String,
}

/// 校验器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierConfig {
    /// 几何容差配置（共享配置）
    pub tolerance: GeometryToleranceConfig,
    /// 最小置信度阈值
    pub min_confidence_threshold: f64,
    /// 是否检测冲突
    pub detect_conflicts: bool,
    /// 是否检测冗余
    pub detect_redundancy: bool,
    /// 是否检测缺失约束
    pub detect_missing_constraints: bool,
    /// 是否检测几何合法性
    pub detect_geometry_issues: bool,
    /// 坐标范围检查 [min_x, min_y, max_x, max_y]
    pub coordinate_range_check: Option<[f64; 4]>,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            tolerance: GeometryToleranceConfig::default(),
            min_confidence_threshold: 0.5,
            detect_conflicts: true,
            detect_redundancy: true,
            detect_missing_constraints: false,
            detect_geometry_issues: true,
            coordinate_range_check: None,
        }
    }
}

impl VerifierConfig {
    /// 验证配置参数的合理性
    ///
    /// # Errors
    /// 如果配置参数无效，返回 `CadAgentError::Config`
    pub fn validate(&self) -> CadAgentResult<()> {
        // 验证几何容差
        self.tolerance.validate()?;

        // 验证置信度阈值
        if self.min_confidence_threshold < 0.0 || self.min_confidence_threshold > 1.0 {
            return Err(CadAgentError::Config(format!(
                "最小置信度阈值必须在 0 到 1 之间，当前值：{}",
                self.min_confidence_threshold
            )));
        }

        // 验证坐标范围检查
        if let Some(range) = &self.coordinate_range_check {
            if range[0] >= range[2] || range[1] >= range[3] {
                return Err(CadAgentError::Config(format!(
                    "坐标范围无效：[{}, {}, {}, {}]。必须满足 min_x < max_x 且 min_y < max_y",
                    range[0], range[1], range[2], range[3]
                )));
            }
        }

        Ok(())
    }

    /// 验证并自动修正不合理的配置
    pub fn validate_or_fix(&mut self) -> Vec<String> {
        let mut warnings = self.tolerance.validate_or_fix();

        if self.min_confidence_threshold < 0.0 || self.min_confidence_threshold > 1.0 {
            warnings.push(format!(
                "最小置信度阈值 {} 无效，已修正为默认值 0.5",
                self.min_confidence_threshold
            ));
            self.min_confidence_threshold = 0.5;
        }

        warnings
    }

    /// 便捷访问角度容差
    #[inline]
    pub fn angle_tolerance(&self) -> f64 {
        self.tolerance.angle_tolerance
    }

    /// 便捷访问距离容差
    #[inline]
    pub fn distance_tolerance(&self) -> f64 {
        self.tolerance.distance_tolerance
    }
}

/// 约束校验器
#[derive(Debug, Clone)]
pub struct ConstraintVerifier {
    config: VerifierConfig,
}

impl ConstraintVerifier {
    /// 创建新的校验器
    pub fn new(config: VerifierConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建校验器
    pub fn with_defaults() -> Self {
        Self::new(VerifierConfig::default())
    }

    /// 执行约束校验
    ///
    /// # Errors
    /// 如果校验过程失败，返回 `CadAgentError::Validation`
    pub fn verify(
        &self,
        primitives: &[Primitive],
        relations: &[GeometricRelation],
    ) -> CadAgentResult<VerificationResult> {
        let mut conflicts = Vec::new();
        let mut redundant_constraints = Vec::new();
        let mut missing_constraints = Vec::new();
        let mut geometry_issues = Vec::new();
        let mut verification_log = Vec::new();

        verification_log.push(format!(
            "开始校验 {} 个基元和 {} 个约束关系",
            primitives.len(),
            relations.len()
        ));

        // 1. 几何合法性校验
        if self.config.detect_geometry_issues {
            geometry_issues = self.check_geometry_validity(primitives);
            verification_log.push(format!("发现 {} 个几何合法性问题", geometry_issues.len()));
        }

        // 2. 冲突检测
        if self.config.detect_conflicts {
            conflicts = self.detect_conflicts(relations);
            verification_log.push(format!("发现 {} 个冲突", conflicts.len()));
        }

        // 3. 冗余检测
        if self.config.detect_redundancy {
            redundant_constraints = self.detect_redundancy(relations);
            verification_log.push(format!("发现 {} 个冗余约束", redundant_constraints.len()));
        }

        // 4. 缺失约束检测
        if self.config.detect_missing_constraints {
            missing_constraints = self.detect_missing_constraints(primitives, relations);
            verification_log.push(format!("建议添加 {} 个约束", missing_constraints.len()));
        }

        // 5. 生成修复建议
        let fix_suggestions = self.generate_fix_suggestions(&conflicts, &geometry_issues);

        // 6. 计算总体评分
        let overall_score = self.compute_overall_score(
            primitives.len(),
            relations.len(),
            &conflicts,
            &geometry_issues,
        );

        let is_valid = conflicts.is_empty() && geometry_issues.is_empty();

        verification_log.push(format!(
            "校验完成：{} (评分：{:.2})",
            if is_valid { "合法" } else { "不合法" },
            overall_score
        ));

        Ok(VerificationResult {
            is_valid,
            conflicts,
            redundant_constraints,
            missing_constraints,
            geometry_issues,
            fix_suggestions,
            verification_log,
            overall_score,
        })
    }

    /// 几何合法性校验
    fn check_geometry_validity(&self, primitives: &[Primitive]) -> Vec<GeometryIssue> {
        let mut issues = Vec::new();

        for (id, prim) in primitives.iter().enumerate() {
            match prim {
                Primitive::Line(line) => {
                    // 检查零长度线段
                    if line.length() < self.config.tolerance.distance_tolerance {
                        issues.push(GeometryIssue::ZeroLengthLine { line_id: id });
                    }
                }
                Primitive::Circle(circle) => {
                    // 检查无效半径
                    if circle.radius <= 0.0 {
                        issues.push(GeometryIssue::InvalidCircleRadius {
                            circle_id: id,
                            radius: circle.radius,
                        });
                    }
                }
                Primitive::Polygon(poly) => {
                    // 检查顶点数量
                    if poly.vertices.len() < 3 {
                        issues.push(GeometryIssue::InsufficientPolygonVertices {
                            polygon_id: id,
                            vertex_count: poly.vertices.len(),
                        });
                    }

                    // 检查自相交（简化实现）
                    if let Some(intersection) = self.check_polygon_self_intersection(poly) {
                        issues.push(GeometryIssue::SelfIntersectingPolygon {
                            polygon_id: id,
                            intersection_point: intersection,
                        });
                    }
                }
                _ => {}
            }

            // 坐标范围检查
            if let Some(range) = &self.config.coordinate_range_check {
                if let Some(bbox) = prim.bounding_box() {
                    if bbox.min.x < range[0]
                        || bbox.min.y < range[1]
                        || bbox.max.x > range[2]
                        || bbox.max.y > range[3]
                    {
                        issues.push(GeometryIssue::CoordinateOutOfRange {
                            primitive_id: id,
                            coordinate: bbox.min,
                            expected_range: *range,
                        });
                    }
                }
            }
        }

        issues
    }

    /// 检测冲突
    fn detect_conflicts(&self, relations: &[GeometricRelation]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // 构建关系索引
        let mut line_relations: std::collections::HashMap<(usize, usize), Vec<&GeometricRelation>> =
            std::collections::HashMap::new();

        for rel in relations {
            let key = match rel {
                GeometricRelation::Parallel {
                    line1_id, line2_id, ..
                }
                | GeometricRelation::Perpendicular {
                    line1_id, line2_id, ..
                }
                | GeometricRelation::Collinear {
                    line1_id, line2_id, ..
                }
                | GeometricRelation::EqualDistance {
                    line1_id, line2_id, ..
                } => Some((*line1_id.min(line2_id), *line1_id.max(line2_id))),
                _ => None,
            };

            if let Some(k) = key {
                line_relations.entry(k).or_default().push(rel);
            }
        }

        // 检测平行 - 垂直冲突
        for rels in line_relations.values() {
            let has_parallel = rels
                .iter()
                .any(|r| matches!(r, GeometricRelation::Parallel { .. }));
            let has_perpendicular = rels
                .iter()
                .any(|r| matches!(r, GeometricRelation::Perpendicular { .. }));

            if has_parallel && has_perpendicular {
                if let (Some(parallel), Some(perp)) = (
                    rels.iter()
                        .find(|r| matches!(r, GeometricRelation::Parallel { .. })),
                    rels.iter()
                        .find(|r| matches!(r, GeometricRelation::Perpendicular { .. })),
                ) {
                    if let (
                        GeometricRelation::Parallel {
                            line1_id,
                            line2_id,
                            confidence: p_conf,
                            ..
                        },
                        GeometricRelation::Perpendicular {
                            confidence: v_conf, ..
                        },
                    ) = (parallel, perp)
                    {
                        conflicts.push(Conflict::ParallelPerpendicular {
                            line1_id: *line1_id,
                            line2_id: *line2_id,
                            parallel_confidence: *p_conf,
                            perpendicular_confidence: *v_conf,
                        });
                    }
                }
            }
        }

        // 检测同心 - 相切冲突
        let mut circle_relations: std::collections::HashMap<
            (usize, usize),
            Vec<&GeometricRelation>,
        > = std::collections::HashMap::new();

        for rel in relations {
            let key = match rel {
                GeometricRelation::Concentric {
                    circle1_id,
                    circle2_id,
                    ..
                }
                | GeometricRelation::TangentCircleCircle {
                    circle1_id,
                    circle2_id,
                    ..
                } => Some((*circle1_id.min(circle2_id), *circle1_id.max(circle2_id))),
                _ => None,
            };

            if let Some(k) = key {
                circle_relations.entry(k).or_default().push(rel);
            }
        }

        for rels in circle_relations.values() {
            let has_concentric = rels
                .iter()
                .any(|r| matches!(r, GeometricRelation::Concentric { .. }));
            let has_tangent = rels
                .iter()
                .any(|r| matches!(r, GeometricRelation::TangentCircleCircle { .. }));

            if has_concentric && has_tangent {
                if let (Some(conc), Some(tan)) = (
                    rels.iter()
                        .find(|r| matches!(r, GeometricRelation::Concentric { .. })),
                    rels.iter()
                        .find(|r| matches!(r, GeometricRelation::TangentCircleCircle { .. })),
                ) {
                    if let (
                        GeometricRelation::Concentric {
                            circle1_id,
                            circle2_id,
                            confidence: c_conf,
                            ..
                        },
                        GeometricRelation::TangentCircleCircle {
                            confidence: t_conf, ..
                        },
                    ) = (conc, tan)
                    {
                        conflicts.push(Conflict::ConcentricTangent {
                            circle1_id: *circle1_id,
                            circle2_id: *circle2_id,
                            concentric_confidence: *c_conf,
                            tangent_confidence: *t_conf,
                        });
                    }
                }
            }
        }

        conflicts
    }

    /// 检测冗余约束
    fn detect_redundancy(&self, relations: &[GeometricRelation]) -> Vec<RedundantConstraint> {
        let mut redundant = Vec::new();

        // 检测重复的约束
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for rel in relations {
            let key = format!("{:?}", rel);
            if seen.contains(&key) {
                redundant.push(RedundantConstraint {
                    relation: rel.clone(),
                    reason: "重复的约束".to_string(),
                    safely_removable: true,
                });
            }
            seen.insert(key);
        }

        // 检测传递性冗余（如 A∥B, B∥C ⇒ A∥C 是冗余的）
        let parallel_groups = self.find_parallel_groups(relations);
        for group in parallel_groups {
            if group.len() > 2 {
                // 超过 2 个的平行组，额外的平行关系可能是冗余的
                for i in 2..group.len() {
                    if let Some(rel) = relations.iter().find(|r| {
                        matches!(r, GeometricRelation::Parallel { line1_id, line2_id, .. }
                            if (*line1_id == group[0] && *line2_id == group[i])
                                || (*line1_id == group[i] && *line2_id == group[0]))
                    }) {
                        redundant.push(RedundantConstraint {
                            relation: rel.clone(),
                            reason: format!("可通过传递性推导（平行链：{:?}）", group),
                            safely_removable: true,
                        });
                    }
                }
            }
        }

        redundant
    }

    /// 查找平行组
    fn find_parallel_groups(&self, relations: &[GeometricRelation]) -> Vec<Vec<usize>> {
        use std::collections::HashMap;

        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();

        for rel in relations {
            if let GeometricRelation::Parallel {
                line1_id, line2_id, ..
            } = rel
            {
                adj.entry(*line1_id).or_default().push(*line2_id);
                adj.entry(*line2_id).or_default().push(*line1_id);
            }
        }

        let mut visited: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut groups = Vec::new();

        for &start in adj.keys() {
            if visited.contains(&start) {
                continue;
            }

            let mut group = Vec::new();
            let mut stack = vec![start];

            while let Some(node) = stack.pop() {
                if visited.contains(&node) {
                    continue;
                }
                visited.insert(node);
                group.push(node);

                if let Some(neighbors) = adj.get(&node) {
                    for &neighbor in neighbors {
                        if !visited.contains(&neighbor) {
                            stack.push(neighbor);
                        }
                    }
                }
            }

            if group.len() > 1 {
                groups.push(group);
            }
        }

        groups
    }

    /// 检测缺失约束
    fn detect_missing_constraints(
        &self,
        primitives: &[Primitive],
        relations: &[GeometricRelation],
    ) -> Vec<MissingConstraint> {
        let mut missing = Vec::new();

        // 检查多边形是否完全约束
        for (id, prim) in primitives.iter().enumerate() {
            if let Primitive::Polygon(poly) = prim {
                // 检查每条边是否有约束
                let edges = poly.to_lines();
                for (i, _edge) in edges.iter().enumerate() {
                    // 简化：建议添加垂直约束
                    let _next_i = (i + 1) % edges.len();

                    // 检查是否已有垂直约束
                    let has_perp = relations
                        .iter()
                        .any(|r| matches!(r, GeometricRelation::Perpendicular { .. }));

                    if !has_perp && poly.vertices.len() == 4 {
                        // 四边形建议检查直角
                        missing.push(MissingConstraint {
                            primitive_ids: vec![id],
                            suggested_type: "perpendicular".to_string(),
                            reason: "四边形通常有直角约束，建议检查相邻边是否垂直".to_string(),
                            priority: 0.7,
                        });
                        break;
                    }
                }
            }
        }

        missing
    }

    /// 生成修复建议
    fn generate_fix_suggestions(
        &self,
        conflicts: &[Conflict],
        issues: &[GeometryIssue],
    ) -> Vec<FixSuggestion> {
        let mut suggestions = Vec::new();

        for conflict in conflicts {
            match conflict {
                Conflict::ParallelPerpendicular {
                    line1_id,
                    line2_id,
                    parallel_confidence,
                    perpendicular_confidence,
                } => {
                    let keep_parallel = parallel_confidence > perpendicular_confidence;
                    suggestions.push(FixSuggestion {
                        issue_type: "constraint_conflict".to_string(),
                        affected_primitives: vec![*line1_id, *line2_id],
                        suggested_action: if keep_parallel {
                            "移除垂直约束，保持平行关系".to_string()
                        } else {
                            "移除平行约束，保持垂直关系".to_string()
                        },
                        difficulty: 2,
                        expected_outcome: "消除几何矛盾".to_string(),
                    });
                }
                Conflict::ConcentricTangent {
                    circle1_id,
                    circle2_id,
                    ..
                } => {
                    suggestions.push(FixSuggestion {
                        issue_type: "constraint_conflict".to_string(),
                        affected_primitives: vec![*circle1_id, *circle2_id],
                        suggested_action: "同心圆无法相切，请检查圆的半径或位置".to_string(),
                        difficulty: 3,
                        expected_outcome: "消除几何矛盾".to_string(),
                    });
                }
                _ => {}
            }
        }

        for issue in issues {
            match issue {
                GeometryIssue::ZeroLengthLine { line_id } => {
                    suggestions.push(FixSuggestion {
                        issue_type: "invalid_geometry".to_string(),
                        affected_primitives: vec![*line_id],
                        suggested_action: "移除零长度线段或重新定义端点".to_string(),
                        difficulty: 1,
                        expected_outcome: "消除无效几何体".to_string(),
                    });
                }
                GeometryIssue::InvalidCircleRadius { circle_id, radius } => {
                    suggestions.push(FixSuggestion {
                        issue_type: "invalid_geometry".to_string(),
                        affected_primitives: vec![*circle_id],
                        suggested_action: if *radius <= 0.0 {
                            "设置有效的正半径值".to_string()
                        } else {
                            "检查半径数值".to_string()
                        },
                        difficulty: 1,
                        expected_outcome: "消除无效几何体".to_string(),
                    });
                }
                _ => {}
            }
        }

        suggestions
    }

    /// 计算总体评分
    fn compute_overall_score(
        &self,
        primitive_count: usize,
        _relation_count: usize,
        conflicts: &[Conflict],
        issues: &[GeometryIssue],
    ) -> f64 {
        if primitive_count == 0 {
            return 0.0;
        }

        let base_score = 1.0;
        let conflict_penalty = (conflicts.len() as f64) * 0.2;
        let issue_penalty = (issues.len() as f64) * 0.1;

        let score = base_score - conflict_penalty - issue_penalty;
        score.clamp(0.0, 1.0)
    }

    /// 检查多边形自相交（简化实现）
    fn check_polygon_self_intersection(&self, polygon: &Polygon) -> Option<Point> {
        // 简化：仅检查相邻边是否重合
        if polygon.vertices.len() < 4 {
            return None;
        }

        let lines = polygon.to_lines();
        for i in 0..lines.len() {
            for j in (i + 2)..lines.len() {
                if i == 0 && j == lines.len() - 1 {
                    continue; // 跳过相邻边
                }

                if let Some(intersection) = self.line_intersection(&lines[i], &lines[j]) {
                    return Some(intersection);
                }
            }
        }

        None
    }

    /// 线段交点
    fn line_intersection(&self, line1: &Line, line2: &Line) -> Option<Point> {
        let dx1 = line1.end.x - line1.start.x;
        let dy1 = line1.end.y - line1.start.y;
        let dx2 = line2.end.x - line2.start.x;
        let dy2 = line2.end.y - line2.start.y;

        let det = dx1 * dy2 - dy1 * dx2;
        if det.abs() < 1e-10 {
            return None; // 平行
        }

        let t =
            ((line2.start.x - line1.start.x) * dy2 - (line2.start.y - line1.start.y) * dx2) / det;
        let u =
            ((line2.start.x - line1.start.x) * dy1 - (line2.start.y - line1.start.y) * dx1) / det;

        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            Some(Point::new(line1.start.x + t * dx1, line1.start.y + t * dy1))
        } else {
            None
        }
    }
}

/// 约束校验工具（tokitai 工具封装）
#[derive(Default, Clone)]
pub struct ConstraintVerifierTools;

#[tool]
impl ConstraintVerifierTools {
    /// 执行约束合法性校验
    ///
    /// # 参数
    ///
    /// * `primitives_json` - 基元列表（JSON 格式）
    /// * `relations_json` - 约束关系列表（JSON 格式）
    /// * `config_json` - 可选的配置（JSON 格式）
    ///
    /// # 返回
    ///
    /// 包含校验结果、冲突列表、修复建议的结构化结果
    #[tool(name = "cad_verify_constraints")]
    pub fn verify(
        &self,
        primitives_json: String,
        relations_json: String,
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

        let config: VerifierConfig = config_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let verifier = ConstraintVerifier::new(config);

        match verifier.verify(&primitives, &relations) {
            Ok(result) => serde_json::json!({
                "success": true,
                "is_valid": result.is_valid,
                "conflicts": result.conflicts,
                "redundant_constraints": result.redundant_constraints,
                "missing_constraints": result.missing_constraints,
                "geometry_issues": result.geometry_issues,
                "fix_suggestions": result.fix_suggestions,
                "overall_score": result.overall_score,
                "verification_log": result.verification_log
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "error": e
            }),
        }
    }

    /// 快速获取校验摘要
    #[tool(name = "cad_quick_verify")]
    pub fn quick_verify(
        &self,
        primitives_json: String,
        relations_json: String,
    ) -> serde_json::Value {
        let primitives: Vec<Primitive> = match serde_json::from_str(&primitives_json) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析失败：{}", e)
                });
            }
        };

        let relations: Vec<GeometricRelation> = match serde_json::from_str(&relations_json) {
            Ok(r) => r,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析失败：{}", e)
                });
            }
        };

        let verifier = ConstraintVerifier::with_defaults();

        match verifier.verify(&primitives, &relations) {
            Ok(result) => serde_json::json!({
                "success": true,
                "is_valid": result.is_valid,
                "conflict_count": result.conflicts.len(),
                "issue_count": result.geometry_issues.len(),
                "overall_score": result.overall_score,
                "has_critical_issues": !result.conflicts.is_empty()
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "error": e
            }),
        }
    }

    /// 获取校验器配置信息
    #[tool(name = "cad_get_verifier_info")]
    pub fn get_verifier_info(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "constraint_verifier",
            "description": "CAD 约束合法性校验：检测冲突、冗余、缺失约束",
            "checks": [
                "conflict_detection",
                "redundancy_detection",
                "missing_constraint_detection",
                "geometry_validity_check"
            ],
            "output": {
                "is_valid": "是否合法",
                "conflicts": "冲突列表",
                "fix_suggestions": "修复建议",
                "overall_score": "总体评分 (0-1)"
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::primitives::Line;

    #[test]
    fn test_verify_no_conflicts() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 1.0])),
        ];

        let relations = vec![GeometricRelation::Perpendicular {
            line1_id: 0,
            line2_id: 1,
            angle_diff: 0.0,
            confidence: 1.0,
        }];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &relations).unwrap();

        assert!(result.is_valid);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_verify_with_conflict() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
        ];

        // 同时声明平行和垂直（冲突）
        let relations = vec![
            GeometricRelation::Parallel {
                line1_id: 0,
                line2_id: 1,
                angle_diff: 0.0,
                confidence: 0.9,
            },
            GeometricRelation::Perpendicular {
                line1_id: 0,
                line2_id: 1,
                angle_diff: 0.0,
                confidence: 0.9,
            },
        ];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &relations).unwrap();

        assert!(!result.is_valid);
        assert!(!result.conflicts.is_empty());
    }

    #[test]
    fn test_verify_zero_length_line() {
        let primitives = vec![Primitive::Line(Line::from_coords([1.0, 1.0], [1.0, 1.0]))];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &[]).unwrap();

        assert!(!result.is_valid);
        assert!(!result.geometry_issues.is_empty());
    }

    #[test]
    fn test_verify_tool() {
        let primitives = vec![Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0]))];

        let relations: Vec<GeometricRelation> = vec![];

        let tools = ConstraintVerifierTools;
        let result = tools.verify(
            serde_json::to_string(&primitives).unwrap(),
            serde_json::to_string(&relations).unwrap(),
            None,
        );

        assert!(result["success"].as_bool().unwrap_or(false));
        assert!(result["is_valid"].as_bool().unwrap_or(false));
    }
}
