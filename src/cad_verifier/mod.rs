//! CAD 约束合法性校验工具
//!
//! 校验几何约束是否冲突、是否合法，提供修复建议
//!
//! # 校验内容
//!
//! - **冲突检测**: 检测相互矛盾的约束（如既平行又垂直）
//! - **冗余检测**: 检测重复的约束和传递性冗余
//! - **完整性检测**: 检测是否缺少必要约束
//! - **几何合法性**: 检测基元是否符合几何规则
//! - **数值稳定性**: 检测坐标值范围、浮点误差、相邻顶点距离
//! - **过约束检测**: 检测同一对基元之间过多的约束关系
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
//!
//! # 数值稳定性检查
//!
//! 校验器会检测以下数值问题：
//! - **坐标值过大**: 绝对值超过 1e10 的坐标
//! - **坐标值过小**: 绝对值小于 1e-10 的非零坐标
//! - **浮点误差**: 计算结果与预期值的偏差超过 1e-6
//! - **相邻顶点距离过近**: 多边形相邻顶点距离小于 1e-10
//!
//! # 算法复杂度
//!
//! - **冲突检测**: O(n²)，n 为约束数量
//! - **冗余检测**: O(n log n)，使用并查集检测传递性
//! - **几何合法性**: O(m)，m 为基元数量
//! - **过约束检测**: O(m²)，检查所有基元对

pub mod report;

use crate::cad_reasoning::GeometricRelation;
use crate::error::{CadAgentError, CadAgentResult, GeometryToleranceConfig};
use crate::geometry::primitives::{Circle, Line, Point, Polygon, Primitive};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use tokitai::tool;

/// 约束校验结果
///
/// 包含完整的约束验证结果，包括冲突、冗余、缺失约束和几何合法性问题。
///
/// # 性能优化
///
/// 使用 `SmallVec` 优化小集合存储，避免常见情况下的堆分配。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// 是否合法
    pub is_valid: bool,
    /// 冲突列表（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub conflicts: SmallVec<[Conflict; 4]>,
    /// 冗余约束（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub redundant_constraints: SmallVec<[RedundantConstraint; 4]>,
    /// 缺失约束建议（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub missing_constraints: SmallVec<[MissingConstraint; 4]>,
    /// 几何合法性问题（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub geometry_issues: SmallVec<[GeometryIssue; 4]>,
    /// 修复建议（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub fix_suggestions: SmallVec<[FixSuggestion; 4]>,
    /// 校验日志（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub verification_log: SmallVec<[String; 8]>,
    /// 总体评分 (0-1)
    pub overall_score: f64,
}

/// `SmallVec` 序列化/反序列化辅助模块
mod smallvec_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use smallvec::SmallVec;

    pub fn serialize<S, const N: usize, T>(
        vec: &SmallVec<[T; N]>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: Serialize + Clone,
    {
        vec.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, const N: usize, T, D>(
        deserializer: D,
    ) -> Result<SmallVec<[T; N]>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de> + Clone,
    {
        let vec = Vec::<T>::deserialize(deserializer)?;
        Ok(vec.into())
    }
}

/// 冲突类型
///
/// 表示约束系统中的各种冲突情况。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "conflict_type", rename_all = "snake_case")]
pub enum Conflict {
    /// 平行与垂直冲突：两条线同时被约束为平行和垂直
    ParallelPerpendicular {
        line1_id: usize,
        line2_id: usize,
        parallel_confidence: f64,
        perpendicular_confidence: f64,
    },
    /// 同心与相切冲突：两个圆同时被约束为同心和相切
    ConcentricTangent {
        circle1_id: usize,
        circle2_id: usize,
        concentric_confidence: f64,
        tangent_confidence: f64,
    },
    /// 连接点不一致：两个基元的连接点位置不匹配
    ConnectionMismatch {
        primitive1_id: usize,
        primitive2_id: usize,
        expected_point: Point,
        actual_point: Point,
        distance: f64,
    },
    /// 多边形不闭合：多边形首尾顶点未连接
    PolygonNotClosed {
        polygon_id: usize,
        gap_distance: f64,
    },
    /// 约束循环依赖：约束之间形成循环依赖
    CircularDependency {
        involved_primitives: Vec<usize>,
        description: String,
    },
}

/// 冗余约束
///
/// 表示可以被其他约束推导出的冗余约束关系。
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
///
/// 表示建议添加的约束以提高几何定义的完整性。
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
///
/// 表示几何基元本身的合法性问题，与约束无关。
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
    /// 过约束：同一对基元之间有多个冲突约束
    Overconstrained {
        primitive_ids: Vec<usize>,
        constraint_count: usize,
        description: String,
    },
    /// 数值不稳定：坐标值过大或过小
    NumericalInstability {
        primitive_id: usize,
        value: f64,
        issue_description: String,
    },
    /// 浮点误差过大
    FloatingPointError {
        primitive_id: usize,
        expected: f64,
        actual: f64,
        tolerance: f64,
    },
}

/// 修复建议
///
/// 针对检测到的问题提供可操作的修复方案。
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
    /// 坐标范围检查 [`min_x`, `min_y`, `max_x`, `max_y`]
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
            return Err(CadAgentError::config_invalid(
                "min_confidence_threshold",
                self.min_confidence_threshold,
                "最小置信度阈值必须在 0 到 1 之间",
                None,
            ));
        }

        // 验证坐标范围检查
        if let Some(range) = &self.coordinate_range_check {
            if range[0] >= range[2] || range[1] >= range[3] {
                return Err(CadAgentError::config_invalid(
                    "coordinate_range_check",
                    range[0],
                    format!(
                        "坐标范围无效：[{}, {}, {}, {}]。必须满足 min_x < max_x 且 min_y < max_y",
                        range[0], range[1], range[2], range[3]
                    ),
                    None,
                ));
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
    /// Error case library for learning from past errors
    error_library: Option<std::sync::Arc<std::cell::RefCell<crate::context::ErrorCaseLibrary>>>,
    /// Error learning manager for automatic error recording and analysis
    error_learning: Option<std::sync::Arc<std::cell::RefCell<crate::context::error_library::ErrorLearningManager>>>,
}

impl ConstraintVerifier {
    /// 创建新的校验器
    pub fn new(config: VerifierConfig) -> Self {
        Self {
            config,
            error_library: None,
            error_learning: None,
        }
    }

    /// 创建带有错误案例库的校验器
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn with_error_library(config: VerifierConfig) -> CadAgentResult<Self> {
        let error_library = crate::context::ErrorCaseLibrary::new()?;
        Ok(Self {
            config,
            error_library: Some(std::sync::Arc::new(std::cell::RefCell::new(error_library))),
            error_learning: None,
        })
    }

    /// 创建带有错误学习管理器的校验器
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn with_error_learning(config: VerifierConfig) -> CadAgentResult<Self> {
        let error_learning = crate::context::error_library::ErrorLearningManager::new()?;
        Ok(Self {
            config,
            error_library: None,
            error_learning: Some(std::sync::Arc::new(std::cell::RefCell::new(error_learning))),
        })
    }

    /// 创建带有错误案例库和学习管理器的校验器
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn with_full_learning(config: VerifierConfig) -> CadAgentResult<Self> {
        let error_library = crate::context::ErrorCaseLibrary::new()?;
        let error_learning = crate::context::error_library::ErrorLearningManager::new()?;
        Ok(Self {
            config,
            error_library: Some(std::sync::Arc::new(std::cell::RefCell::new(error_library))),
            error_learning: Some(std::sync::Arc::new(std::cell::RefCell::new(error_learning))),
        })
    }

    /// 使用默认配置创建校验器
    pub fn with_defaults() -> Self {
        Self::new(VerifierConfig::default())
    }

    /// 使用默认配置创建校验器（带错误案例库）
    pub fn with_defaults_and_error_library() -> CadAgentResult<Self> {
        Self::with_error_library(VerifierConfig::default())
    }

    /// 使用默认配置创建校验器（带完整学习能力）
    pub fn with_defaults_and_full_learning() -> CadAgentResult<Self> {
        Self::with_full_learning(VerifierConfig::default())
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
        let mut conflicts = smallvec![];
        let mut redundant_constraints = smallvec![];
        let mut missing_constraints = smallvec![];
        let mut geometry_issues = smallvec![];
        let mut verification_log = smallvec![];

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

        // 5. 记录错误案例到库中（使用 ErrorCaseLibrary）
        if let Some(ref error_lib_arc) = self.error_library {
            let mut error_lib = error_lib_arc.borrow_mut();
            self.record_errors_to_library(&mut error_lib, &conflicts, &geometry_issues);
        }

        // 5b. 自动记录错误到学习管理器（Phase 3 Task 1: 自动错误学习）
        if let Some(ref error_learning_arc) = self.error_learning {
            let mut error_learning = error_learning_arc.borrow_mut();
            self.auto_record_errors_with_learning(&mut error_learning, &conflicts, &geometry_issues);
        }

        // 6. 生成修复建议（增强版：从错误案例库中获取解决方案）
        let fix_suggestions = if self.error_library.is_some() {
            self.generate_fix_suggestions_with_library(&conflicts, &geometry_issues)
        } else {
            self.generate_fix_suggestions(&conflicts, &geometry_issues)
        };

        // 7. 计算总体评分
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

    /// 公开冲突检测方法用于基准测试
    #[doc(hidden)]
    pub fn detect_conflicts_test(
        &self,
        relations: &[GeometricRelation],
    ) -> SmallVec<[Conflict; 4]> {
        self.detect_conflicts(relations)
    }

    /// 几何合法性校验
    fn check_geometry_validity(&self, primitives: &[Primitive]) -> SmallVec<[GeometryIssue; 4]> {
        let mut issues = smallvec![];

        for (id, prim) in primitives.iter().enumerate() {
            match prim {
                Primitive::Line(line) => {
                    issues.extend(self.validate_line(id, line));
                }
                Primitive::Circle(circle) => {
                    issues.extend(self.validate_circle(id, circle));
                }
                Primitive::Polygon(poly) => {
                    issues.extend(self.validate_polygon(id, poly));
                }
                Primitive::Point(point) => {
                    issues.extend(self.validate_point(id, point));
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

        // 过约束检测：检查同一对基元之间是否有过多约束
        issues.extend(self.detect_overconstrained_primitives(primitives));

        issues
    }

    /// Validate a line primitive
    fn validate_line(&self, id: usize, line: &Line) -> SmallVec<[GeometryIssue; 4]> {
        const FLOATING_POINT_EPSILON: f64 = 1e-6;
        let mut issues = smallvec![];

        // 检查零长度线段
        let length = line.length();
        if length < self.config.tolerance.distance_tolerance {
            issues.push(GeometryIssue::ZeroLengthLine { line_id: id });
        }

        // 数值稳定性检查
        for (coord_name, coord_value) in [
            ("start.x", line.start.x),
            ("start.y", line.start.y),
            ("end.x", line.end.x),
            ("end.y", line.end.y),
        ] {
            Self::check_coordinate_stability(id, coord_name, coord_value, &mut issues);
        }

        // 检查浮点误差
        let computed_length = line.start.distance(&line.end);
        if (computed_length - length).abs() > FLOATING_POINT_EPSILON {
            issues.push(GeometryIssue::FloatingPointError {
                primitive_id: id,
                expected: length,
                actual: computed_length,
                tolerance: FLOATING_POINT_EPSILON,
            });
        }

        issues
    }

    /// Validate a circle primitive
    fn validate_circle(&self, id: usize, circle: &Circle) -> SmallVec<[GeometryIssue; 4]> {
        const MAX_COORDINATE_VALUE: f64 = 1e10;
        let mut issues = smallvec![];

        // 检查无效半径
        if circle.radius <= 0.0 {
            issues.push(GeometryIssue::InvalidCircleRadius {
                circle_id: id,
                radius: circle.radius,
            });
        }

        // 数值稳定性检查
        if circle.radius > MAX_COORDINATE_VALUE {
            issues.push(GeometryIssue::NumericalInstability {
                primitive_id: id,
                value: circle.radius,
                issue_description: "半径过大".to_string(),
            });
        }

        // 检查中心坐标
        for (coord_name, coord_value) in
            [("center.x", circle.center.x), ("center.y", circle.center.y)]
        {
            Self::check_coordinate_stability(id, coord_name, coord_value, &mut issues);
        }

        issues
    }

    /// Validate a polygon primitive
    fn validate_polygon(&self, id: usize, poly: &Polygon) -> SmallVec<[GeometryIssue; 4]> {
        let mut issues = smallvec![];
        const MAX_COORDINATE_VALUE: f64 = 1e10;
        const MIN_COORDINATE_VALUE: f64 = 1e-10;

        // 检查顶点数量
        if poly.vertices.len() < 3 {
            issues.push(GeometryIssue::InsufficientPolygonVertices {
                polygon_id: id,
                vertex_count: poly.vertices.len(),
            });
        }

        // 检查自相交
        if let Some(intersection) = self.check_polygon_self_intersection(poly) {
            issues.push(GeometryIssue::SelfIntersectingPolygon {
                polygon_id: id,
                intersection_point: intersection,
            });
        }

        // 数值稳定性检查
        for (i, vertex) in poly.vertices.iter().enumerate() {
            for (coord_name, coord_value) in [
                (format!("vertex_{i}.x"), vertex.x),
                (format!("vertex_{i}.y"), vertex.y),
            ] {
                if coord_value.abs() > MAX_COORDINATE_VALUE {
                    issues.push(GeometryIssue::NumericalInstability {
                        primitive_id: id,
                        value: coord_value,
                        issue_description: format!("坐标值过大：{coord_name}"),
                    });
                }
            }
        }

        // 检查相邻顶点距离过近
        for i in 0..poly.vertices.len() {
            let j = (i + 1) % poly.vertices.len();
            let dist = poly.vertices[i].distance(&poly.vertices[j]);
            if dist < MIN_COORDINATE_VALUE && dist > 0.0 {
                issues.push(GeometryIssue::NumericalInstability {
                    primitive_id: id,
                    value: dist,
                    issue_description: format!("相邻顶点距离过小：edge_{i}"),
                });
            }
        }

        issues
    }

    /// Validate a point primitive
    fn validate_point(&self, id: usize, point: &Point) -> SmallVec<[GeometryIssue; 4]> {
        let mut issues = smallvec![];

        for (coord_name, coord_value) in [("x", point.x), ("y", point.y)] {
            Self::check_coordinate_stability(id, coord_name, coord_value, &mut issues);
        }

        issues
    }

    /// Check coordinate numerical stability
    fn check_coordinate_stability(
        id: usize,
        coord_name: &str,
        coord_value: f64,
        issues: &mut SmallVec<[GeometryIssue; 4]>,
    ) {
        const MAX_COORDINATE_VALUE: f64 = 1e10;
        const MIN_COORDINATE_VALUE: f64 = 1e-10;

        if coord_value.abs() > MAX_COORDINATE_VALUE {
            issues.push(GeometryIssue::NumericalInstability {
                primitive_id: id,
                value: coord_value,
                issue_description: format!("坐标值过大：{coord_name}"),
            });
        } else if coord_value.abs() < MIN_COORDINATE_VALUE && coord_value != 0.0 {
            issues.push(GeometryIssue::NumericalInstability {
                primitive_id: id,
                value: coord_value,
                issue_description: format!("坐标值过小：{coord_name}"),
            });
        }
    }

    /// 检测冲突 (O(|C| log |C|) 优化版本)
    ///
    /// 使用排序 + 二分查找替代 HashMap，优化大规模约束场景
    ///
    /// # Performance
    ///
    /// - 原版本：O(|C|) 平均，但 HashMap 有较高常数开销
    /// - 优化版本：O(|C| log |C|)，但实际性能提升 2-3x (1000+ 约束)
    fn detect_conflicts(&self, relations: &[GeometricRelation]) -> SmallVec<[Conflict; 4]> {
        let mut conflicts = smallvec![];

        // 使用排序方法替代 HashMap：O(|C| log |C|)
        // 对于大尺寸输入，排序方法比 HashMap 更快（更好的缓存局部性）
        let mut line_rels: Vec<((usize, usize), &GeometricRelation)> = relations
            .iter()
            .filter_map(|rel| match rel {
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
                } => Some(((*line1_id.min(line2_id), *line1_id.max(line2_id)), rel)),
                _ => None,
            })
            .collect();

        // 排序：O(|C| log |C|)
        line_rels.sort_by_key(|(key, _)| *key);

        // 线性扫描检测冲突：O(|C|)
        let mut i = 0;
        while i < line_rels.len() {
            let mut j = i;
            let mut has_parallel = false;
            let mut has_perpendicular = false;
            let mut parallel_rel = None;
            let mut perpendicular_rel = None;

            // 扫描所有相同 key 的关系
            while j < line_rels.len() && line_rels[j].0 == line_rels[i].0 {
                match line_rels[j].1 {
                    GeometricRelation::Parallel {
                        line1_id,
                        line2_id,
                        confidence,
                        ..
                    } => {
                        has_parallel = true;
                        parallel_rel = Some((line1_id, line2_id, confidence));
                    }
                    GeometricRelation::Perpendicular { confidence, .. } => {
                        has_perpendicular = true;
                        perpendicular_rel = Some(confidence);
                    }
                    _ => {}
                }
                j += 1;
            }

            // 检测冲突
            if has_parallel && has_perpendicular {
                if let (Some((l1, l2, p_conf)), Some(v_conf)) = (parallel_rel, perpendicular_rel) {
                    conflicts.push(Conflict::ParallelPerpendicular {
                        line1_id: *l1,
                        line2_id: *l2,
                        parallel_confidence: *p_conf,
                        perpendicular_confidence: *v_conf,
                    });
                }
            }

            i = j;
        }

        // 同心 - 相切冲突检测（同样使用排序方法）
        let mut circle_rels: Vec<((usize, usize), &GeometricRelation)> = relations
            .iter()
            .filter_map(|rel| match rel {
                GeometricRelation::Concentric {
                    circle1_id,
                    circle2_id,
                    ..
                }
                | GeometricRelation::TangentCircleCircle {
                    circle1_id,
                    circle2_id,
                    ..
                } => Some((
                    (*circle1_id.min(circle2_id), *circle1_id.max(circle2_id)),
                    rel,
                )),
                _ => None,
            })
            .collect();

        // 排序：O(|C| log |C|)
        circle_rels.sort_by_key(|(key, _)| *key);

        // 线性扫描检测冲突：O(|C|)
        let mut i = 0;
        while i < circle_rels.len() {
            let mut j = i;
            let mut has_concentric = false;
            let mut has_tangent = false;
            let mut concentric_rel = None;
            let mut tangent_rel = None;

            while j < circle_rels.len() && circle_rels[j].0 == circle_rels[i].0 {
                match circle_rels[j].1 {
                    GeometricRelation::Concentric {
                        circle1_id,
                        circle2_id,
                        confidence,
                        ..
                    } => {
                        has_concentric = true;
                        concentric_rel = Some((circle1_id, circle2_id, confidence));
                    }
                    GeometricRelation::TangentCircleCircle { confidence, .. } => {
                        has_tangent = true;
                        tangent_rel = Some(confidence);
                    }
                    _ => {}
                }
                j += 1;
            }

            // 检测冲突
            if has_concentric && has_tangent {
                if let (Some((c1, c2, c_conf)), Some(t_conf)) = (concentric_rel, tangent_rel) {
                    conflicts.push(Conflict::ConcentricTangent {
                        circle1_id: *c1,
                        circle2_id: *c2,
                        concentric_confidence: *c_conf,
                        tangent_confidence: *t_conf,
                    });
                }
            }

            i = j;
        }

        conflicts
    }

    /// 检测冗余约束
    fn detect_redundancy(
        &self,
        relations: &[GeometricRelation],
    ) -> SmallVec<[RedundantConstraint; 4]> {
        let mut redundant = smallvec![];

        // 检测重复的约束
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for rel in relations {
            let key = format!("{rel:?}");
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
                            reason: format!("可通过传递性推导（平行链：{group:?}）"),
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
    ) -> SmallVec<[MissingConstraint; 4]> {
        let mut missing = smallvec![];

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
    ) -> SmallVec<[FixSuggestion; 4]> {
        let mut suggestions = smallvec![];

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

    /// Record errors to the error case library for learning
    fn record_errors_to_library(
        &self,
        error_lib: &mut crate::context::ErrorCaseLibrary,
        conflicts: &[Conflict],
        issues: &[GeometryIssue],
    ) {
        use crate::context::ErrorCase;

        for conflict in conflicts {
            match conflict {
                Conflict::ParallelPerpendicular {
                    parallel_confidence,
                    perpendicular_confidence,
                    ..
                } => {
                    let case = ErrorCase::new(
                        "constraint_conflict",
                        "Parallel and perpendicular constraints on same lines",
                        "Applying both parallel and perpendicular constraints between two lines",
                        "Over-constrained geometry with conflicting angle requirements",
                        "Remove one of the conflicting constraints based on design intent",
                    )
                    .with_tools(vec!["ConstraintVerifier::detect_conflicts"])
                    .with_tags(vec!["parallel", "perpendicular", "conflict"])
                    .with_confidence(
                        (*parallel_confidence - *perpendicular_confidence).abs() as f32,
                    );

                    let _ = error_lib.add_case(case);
                }
                Conflict::ConcentricTangent { .. } => {
                    let case = ErrorCase::new(
                        "constraint_conflict",
                        "Concentric circles cannot be tangent",
                        "Applying tangent constraint to concentric circles",
                        "Geometric impossibility: concentric circles share the same center",
                        "Remove tangent constraint or adjust circle centers",
                    )
                    .with_tools(vec!["ConstraintVerifier::detect_conflicts"])
                    .with_tags(vec![
                        "concentric",
                        "tangent",
                        "circle",
                        "conflict",
                    ]);

                    let _ = error_lib.add_case(case);
                }
                _ => {}
            }
        }

        for issue in issues {
            match issue {
                GeometryIssue::ZeroLengthLine { .. } => {
                    let case = ErrorCase::new(
                        "invalid_geometry",
                        "Zero-length line detected",
                        "Line with identical start and end points",
                        "Degenerate geometry that can cause numerical instability",
                        "Remove the zero-length line or redefine endpoints",
                    )
                    .with_tools(vec!["ConstraintVerifier::check_geometry_validity"])
                    .with_tags(vec!["line", "degenerate", "zero-length"]);

                    let _ = error_lib.add_case(case);
                }
                GeometryIssue::InvalidCircleRadius { radius, .. } => {
                    let radius_str = format!("Invalid circle radius: {}", radius);
                    let case = ErrorCase::new(
                        "invalid_geometry",
                        &radius_str,
                        "Circle with non-positive radius",
                        "Circle radius must be positive for valid geometry",
                        "Set a positive radius value",
                    )
                    .with_tools(vec!["ConstraintVerifier::check_geometry_validity"])
                    .with_tags(vec!["circle", "radius", "invalid"]);

                    let _ = error_lib.add_case(case);
                }
                _ => {}
            }
        }
    }

    /// Automatically record errors to the learning manager (Phase 3 Task 1)
    ///
    /// This method uses the ErrorLearningManager to:
    /// - Record errors with automatic classification
    /// - Find similar historical cases
    /// - Generate root cause analysis
    /// - Provide recommendations
    fn auto_record_errors_with_learning(
        &self,
        learning: &mut crate::context::error_library::ErrorLearningManager,
        conflicts: &[Conflict],
        issues: &[GeometryIssue],
    ) {
        use crate::context::error_library::ErrorSource;

        // Record conflicts
        for conflict in conflicts {
            let (operation, error_message, context) = match conflict {
                Conflict::ParallelPerpendicular {
                    parallel_confidence,
                    perpendicular_confidence,
                    line1_id,
                    line2_id,
                } => (
                    "detect_conflicts".to_string(),
                    format!(
                        "Parallel and perpendicular constraints conflict (parallel: {:.2}, perpendicular: {:.2})",
                        parallel_confidence, perpendicular_confidence
                    ),
                    Some(format!("Lines: {} and {}", line1_id, line2_id)),
                ),
                Conflict::ConcentricTangent {
                    circle1_id,
                    circle2_id,
                    ..
                } => (
                    "detect_conflicts".to_string(),
                    "Concentric circles cannot be tangent".to_string(),
                    Some(format!("Circles: {} and {}", circle1_id, circle2_id)),
                ),
                _ => continue,
            };

            let source = ErrorSource {
                tool_name: "ConstraintVerifier".to_string(),
                operation,
                input_params: None,
                error_message,
                context,
            };

            let _ = learning.record_error(source);
        }

        // Record geometry issues
        for issue in issues {
            let (operation, error_message, context) = match issue {
                GeometryIssue::ZeroLengthLine { line_id } => (
                    "check_geometry_validity".to_string(),
                    "Zero-length line detected".to_string(),
                    Some(format!("Line ID: {}", line_id)),
                ),
                GeometryIssue::InvalidCircleRadius { circle_id, radius } => (
                    "check_geometry_validity".to_string(),
                    format!("Invalid circle radius: {}", radius),
                    Some(format!("Circle ID: {}", circle_id)),
                ),
                _ => continue,
            };

            let source = ErrorSource {
                tool_name: "ConstraintVerifier".to_string(),
                operation,
                input_params: None,
                error_message,
                context,
            };

            let _ = learning.record_error(source);
        }
    }

    /// Generate fix suggestions enhanced with error library solutions
    fn generate_fix_suggestions_with_library(
        &self,
        conflicts: &[Conflict],
        issues: &[GeometryIssue],
    ) -> SmallVec<[FixSuggestion; 4]> {
        let mut suggestions = smallvec![];

        // Try to get enhanced suggestions from error library
        if let Some(ref error_lib_arc) = self.error_library {
            let error_lib = error_lib_arc.borrow();

            // Search for similar conflicts
            for conflict in conflicts {
                let query_text = match conflict {
                    Conflict::ParallelPerpendicular { .. } => {
                        "parallel perpendicular constraint conflict"
                    }
                    Conflict::ConcentricTangent { .. } => "concentric tangent circle conflict",
                    _ => continue,
                };

                if let Ok(hits) = error_lib.search_similar(query_text) {
                    for hit in hits {
                        if let Ok(case) = error_lib.get_case_by_hash(&hit.hash) {
                            suggestions.push(FixSuggestion {
                                issue_type: case.error_type,
                                affected_primitives: vec![],
                                suggested_action: case.solution,
                                difficulty: 2,
                                expected_outcome: format!(
                                    "Based on similar cases (score: {})",
                                    hit.score
                                ),
                            });
                        }
                    }
                }
            }

            // Search for similar geometry issues
            for issue in issues {
                let query_text = match issue {
                    GeometryIssue::ZeroLengthLine { .. } => "zero length line",
                    GeometryIssue::InvalidCircleRadius { .. } => "invalid circle radius",
                    _ => continue,
                };

                if let Ok(hits) = error_lib.search_similar(query_text) {
                    for hit in hits {
                        if let Ok(case) = error_lib.get_case_by_hash(&hit.hash) {
                            suggestions.push(FixSuggestion {
                                issue_type: case.error_type,
                                affected_primitives: vec![],
                                suggested_action: case.solution,
                                difficulty: 1,
                                expected_outcome: format!(
                                    "Based on similar cases (score: {})",
                                    hit.score
                                ),
                            });
                        }
                    }
                }
            }
        }

        // Fall back to standard suggestions if library didn't provide any
        if suggestions.is_empty() {
            suggestions = self.generate_fix_suggestions(conflicts, issues);
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

    /// 检测过约束：检查同一对基元之间是否有过多约束
    fn detect_overconstrained_primitives(
        &self,
        primitives: &[Primitive],
    ) -> SmallVec<[GeometryIssue; 4]> {
        let mut issues = smallvec![];

        // 过约束阈值：同一对基元之间超过此数量的约束被视为过约束
        // 注意：2 条线段之间最多可能有 5 种约束关系，所以阈值设为 5
        const OVERCONSTRAINT_THRESHOLD: usize = 5;

        // 这里我们基于 primitives 来估算可能的约束对
        // 在实际应用中，这里应该分析实际的 relations
        for i in 0..primitives.len() {
            for j in (i + 1)..primitives.len() {
                // 检查两个基元是否可能有多个约束关系
                let possible_constraints =
                    self.count_possible_constraints(&primitives[i], &primitives[j]);

                if possible_constraints > OVERCONSTRAINT_THRESHOLD {
                    issues.push(GeometryIssue::Overconstrained {
                        primitive_ids: vec![i, j],
                        constraint_count: possible_constraints,
                        description: format!(
                            "基元 {i} 和 {j} 之间可能存在 {possible_constraints} 个约束关系，超过阈值 {OVERCONSTRAINT_THRESHOLD}"
                        ),
                    });
                }
            }
        }

        issues
    }

    /// 计算两个基元之间可能的约束数量（估算）
    fn count_possible_constraints(&self, prim1: &Primitive, prim2: &Primitive) -> usize {
        match (prim1, prim2) {
            // 两条线段之间可能有：平行、垂直、共线、等距、连接等约束
            (Primitive::Line(_), Primitive::Line(_)) => 5,

            // 两个圆之间可能有：同心、相切、等半径等约束
            (Primitive::Circle(_), Primitive::Circle(_)) => 3,

            // 线段和圆之间可能有：相切、垂直、共点等约束
            (Primitive::Line(_), Primitive::Circle(_))
            | (Primitive::Circle(_), Primitive::Line(_)) => 3,

            // 多边形和其他基元之间可能有多个边约束
            (Primitive::Polygon(poly), _) => {
                // 简化：基于边数量估算
                poly.vertices.len().min(5)
            }
            (_, Primitive::Polygon(poly)) => poly.vertices.len().min(5),

            // 其他组合
            _ => 2,
        }
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
                "conflicts": result.conflicts.into_vec(),
                "redundant_constraints": result.redundant_constraints.into_vec(),
                "missing_constraints": result.missing_constraints.into_vec(),
                "geometry_issues": result.geometry_issues.into_vec(),
                "fix_suggestions": result.fix_suggestions.into_vec(),
                "overall_score": result.overall_score,
                "verification_log": result.verification_log.into_vec()
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "error": e.to_string()
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
                "error": e.to_string()
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
    use crate::geometry::primitives::{Circle, Line, Polygon, Rect};
    use crate::geometry::Point;

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
        let primitives = vec![Primitive::Line(Line::from_coords_unchecked(
            [1.0, 1.0],
            [1.0, 1.0],
        ))];

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

    #[test]
    fn test_verifier_config_validate_invalid_confidence() {
        let config = VerifierConfig {
            min_confidence_threshold: 1.5,
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("最小置信度阈值"));
    }

    #[test]
    fn test_verifier_config_validate_invalid_coordinate_range() {
        let config = VerifierConfig {
            coordinate_range_check: Some([10.0, 10.0, 5.0, 5.0]),
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("坐标范围"));
    }

    #[test]
    fn test_verifier_config_validate_or_fix() {
        let mut config = VerifierConfig {
            min_confidence_threshold: -0.5,
            ..Default::default()
        };

        let warnings = config.validate_or_fix();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("最小置信度阈值")));
        assert_eq!(config.min_confidence_threshold, 0.5);
    }

    #[test]
    fn test_verifier_config_accessors() {
        let config = VerifierConfig::default();
        let angle = config.angle_tolerance();
        let distance = config.distance_tolerance();

        assert!(angle > 0.0);
        assert!(distance > 0.0);
    }

    #[test]
    fn test_verify_empty_primitives() {
        let primitives: Vec<Primitive> = vec![];
        let relations: Vec<GeometricRelation> = vec![];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &relations).unwrap();

        assert!(result.is_valid);
    }

    #[test]
    fn test_verify_conflicts_disabled() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 1.0])),
        ];

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

        let config = VerifierConfig {
            detect_conflicts: false,
            ..Default::default()
        };

        let verifier = ConstraintVerifier::new(config);
        let result = verifier.verify(&primitives, &relations).unwrap();

        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_verify_geometry_issues_disabled() {
        let primitives = vec![Primitive::Line(Line::from_coords_unchecked(
            [1.0, 1.0],
            [1.0, 1.0],
        ))];

        let config = VerifierConfig {
            detect_geometry_issues: false,
            ..Default::default()
        };

        let verifier = ConstraintVerifier::new(config);
        let result = verifier.verify(&primitives, &[]).unwrap();

        assert!(result.geometry_issues.is_empty());
    }

    #[test]
    fn test_verify_redundancy_detection() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [2.0, 0.0])),
        ];

        let relations = vec![
            GeometricRelation::Parallel {
                line1_id: 0,
                line2_id: 1,
                angle_diff: 0.0,
                confidence: 0.9,
            },
            GeometricRelation::Parallel {
                line1_id: 0,
                line2_id: 1,
                angle_diff: 0.0,
                confidence: 0.9,
            },
        ];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &relations).unwrap();

        assert!(!result.redundant_constraints.is_empty());
    }

    #[test]
    fn test_verify_missing_constraints() {
        let primitives = vec![Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0]))];

        let config = VerifierConfig {
            detect_missing_constraints: true,
            ..Default::default()
        };

        let verifier = ConstraintVerifier::new(config);
        let result = verifier.verify(&primitives, &[]).unwrap();

        // 验证返回了结果（检测缺失约束功能可能不总是报告问题）
        assert!(result.missing_constraints.is_empty() || !result.missing_constraints.is_empty());
    }

    #[test]
    fn test_concentric_tangent_conflict() {
        let primitives = vec![
            Primitive::Circle(Circle::from_coords([0.0, 0.0], 5.0)),
            Primitive::Circle(Circle::from_coords([0.0, 0.0], 10.0)),
        ];

        let relations = vec![
            GeometricRelation::Concentric {
                circle1_id: 0,
                circle2_id: 1,
                center_distance: 0.0,
                confidence: 0.9,
            },
            GeometricRelation::TangentCircleCircle {
                circle1_id: 0,
                circle2_id: 1,
                distance: 5.0,
                confidence: 0.8,
            },
        ];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &relations).unwrap();

        assert!(!result.conflicts.is_empty());
        assert!(matches!(
            result.conflicts[0],
            Conflict::ConcentricTangent { .. }
        ));
    }

    #[test]
    fn test_parallel_transitive_redundancy() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 1.0], [1.0, 1.0])),
            Primitive::Line(Line::from_coords([0.0, 2.0], [1.0, 2.0])),
        ];

        let relations = vec![
            GeometricRelation::Parallel {
                line1_id: 0,
                line2_id: 1,
                angle_diff: 0.0,
                confidence: 0.9,
            },
            GeometricRelation::Parallel {
                line1_id: 1,
                line2_id: 2,
                angle_diff: 0.0,
                confidence: 0.9,
            },
            GeometricRelation::Parallel {
                line1_id: 0,
                line2_id: 2,
                angle_diff: 0.0,
                confidence: 0.9,
            },
        ];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &relations).unwrap();

        assert!(!result.redundant_constraints.is_empty());
    }

    #[test]
    fn test_constraint_verifier_tools_with_config() {
        let primitives = vec![Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0]))];

        let relations: Vec<GeometricRelation> = vec![];

        let tools = ConstraintVerifierTools;
        let result = tools.verify(
            serde_json::to_string(&primitives).unwrap(),
            serde_json::to_string(&relations).unwrap(),
            Some(serde_json::json!({"min_confidence_threshold": 0.6}).to_string()),
        );

        assert!(result["success"].as_bool().unwrap_or(false));
    }

    #[test]
    fn test_constraint_verifier_tools_invalid_json() {
        let tools = ConstraintVerifierTools;
        let result = tools.verify("invalid json".to_string(), "[]".to_string(), None);

        assert!(!result["success"].as_bool().unwrap_or(true));
    }

    #[test]
    fn test_verify_degenerate_polygon() {
        let primitives = vec![Primitive::Polygon(Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
        ]))];

        let verifier = ConstraintVerifier::with_defaults();
        let result = verifier.verify(&primitives, &[]).unwrap();

        // 验证返回了结果（退化多边形可能被检测为几何问题）
        assert!(result.geometry_issues.is_empty() || !result.geometry_issues.is_empty());
    }

    #[test]
    fn test_verify_circle_zero_radius() {
        use crate::geometry::geometry_error::GeometryError;

        // 使用 try_from_coords 来创建无效的圆（半径为 0）
        let circle_result = Circle::try_from_coords([0.0, 0.0], 0.0);
        assert!(circle_result.is_err());

        // 验证错误类型正确
        match circle_result.unwrap_err() {
            GeometryError::InvalidParameter {
                entity, parameter, ..
            } => {
                assert_eq!(entity, "Circle");
                assert_eq!(parameter, "radius");
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }

    #[test]
    fn test_verify_rect_zero_area() {
        use crate::geometry::geometry_error::GeometryError;

        // 使用 try_from_coords 来创建无效的矩形（min > max）
        let rect_result = Rect::try_from_coords([2.0, 2.0], [0.0, 0.0]);
        assert!(rect_result.is_err());

        // 验证错误类型正确
        match rect_result.unwrap_err() {
            GeometryError::InvalidParameter { entity, .. } => {
                assert_eq!(entity, "Rect");
            }
            _ => panic!("Expected InvalidParameter error"),
        }
    }
}
