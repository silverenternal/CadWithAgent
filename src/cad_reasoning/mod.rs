//! CAD 几何关系推理工具
//!
//! 计算几何基元之间的约束关系：平行、垂直、相切、同心、共线等
//!
//! # 支持的几何关系
//!
//! - **平行 (Parallel)**: 两线段方向向量夹角接近 0°或 180°
//! - **垂直 (Perpendicular)**: 两线段方向向量点积为 0
//! - **共线 (Collinear)**: 两线段在同一直线上
//! - **相切 (Tangent)**: 线与圆、圆与圆相切
//! - **同心 (Concentric)**: 两圆共享同一圆心
//! - **连接 (Connected)**: 基元共享端点
//! - **包含 (Contains)**: 点在线/圆/多边形上或内部
//! - **等距 (EqualDistance)**: 两线段长度相等
//! - **对称 (Symmetric)**: 关于某轴对称
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::cad_reasoning::{GeometricRelationReasoner, ReasoningConfig};
//! use cadagent::prelude::*;
//!
//! // 创建一些测试基元
//! let primitives = vec![
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
//!     Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 100.0])),
//! ];
//!
//! let config = ReasoningConfig::default();
//! let reasoner = GeometricRelationReasoner::new(config);
//! let result = reasoner.find_all_relations(&primitives);
//!
//! // 访问结果中的 relations 字段
//! for rel in &result.relations {
//!     println!("{:?}", rel);
//! }
//! ```

use crate::error::{CadAgentError, CadAgentResult};
use crate::geometry::primitives::{Circle, Line, Point, Polygon, Primitive};
use rstar::{RTree, RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use tokitai::tool;

/// R-tree 可索引的几何包络
#[derive(Debug, Clone)]
struct PrimitiveEnvelope {
    primitive_id: usize,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl RTreeObject for PrimitiveEnvelope {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners([self.min_x, self.min_y], [self.max_x, self.max_y])
    }
}

impl PrimitiveEnvelope {
    fn new(id: usize, primitive: &Primitive) -> Option<Self> {
        let bbox = primitive.bounding_box()?;
        Some(Self {
            primitive_id: id,
            min_x: bbox.min.x,
            min_y: bbox.min.y,
            max_x: bbox.max.x,
            max_y: bbox.max.y,
        })
    }
}

/// 几何关系类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "relation_type", rename_all = "snake_case")]
pub enum GeometricRelation {
    /// 平行
    Parallel {
        line1_id: usize,
        line2_id: usize,
        angle_diff: f64,
        confidence: f64,
    },
    /// 垂直
    Perpendicular {
        line1_id: usize,
        line2_id: usize,
        angle_diff: f64,
        confidence: f64,
    },
    /// 共线
    Collinear {
        line1_id: usize,
        line2_id: usize,
        distance: f64,
        confidence: f64,
    },
    /// 相切（线与圆）
    TangentLineCircle {
        line_id: usize,
        circle_id: usize,
        distance: f64,
        confidence: f64,
    },
    /// 相切（圆与圆）
    TangentCircleCircle {
        circle1_id: usize,
        circle2_id: usize,
        distance: f64,
        confidence: f64,
    },
    /// 同心
    Concentric {
        circle1_id: usize,
        circle2_id: usize,
        center_distance: f64,
        confidence: f64,
    },
    /// 连接
    Connected {
        primitive1_id: usize,
        primitive2_id: usize,
        connection_point: Point,
        confidence: f64,
    },
    /// 包含
    Contains {
        container_id: usize,
        contained_id: usize,
        relation: ContainmentType,
        confidence: f64,
    },
    /// 等距
    EqualDistance {
        line1_id: usize,
        line2_id: usize,
        length_diff: f64,
        confidence: f64,
    },
    /// 对称
    Symmetric {
        primitive1_id: usize,
        primitive2_id: usize,
        axis_line_id: Option<usize>,
        symmetry_type: SymmetryType,
        confidence: f64,
    },
}

/// 包含关系类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainmentType {
    /// 点在直线上
    PointOnLine,
    /// 点在圆上
    PointOnCircle,
    /// 点在多边形内
    PointInPolygon,
    /// 点在矩形内
    PointInRect,
    /// 圆包含点
    CircleContainsPoint,
    /// 多边形包含点
    PolygonContainsPoint,
}

/// 对称类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymmetryType {
    /// 轴对称
    Axial,
    /// 中心对称
    Central,
}

/// 推理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// 角度容差（弧度）
    pub angle_tolerance: f64,
    /// 距离容差
    pub distance_tolerance: f64,
    /// 最小置信度
    pub min_confidence: f64,
    /// 是否检测平行
    pub detect_parallel: bool,
    /// 是否检测垂直
    pub detect_perpendicular: bool,
    /// 是否检测共线
    pub detect_collinear: bool,
    /// 是否检测相切
    pub detect_tangent: bool,
    /// 是否检测同心
    pub detect_concentric: bool,
    /// 是否检测连接
    pub detect_connected: bool,
    /// 是否检测对称
    pub detect_symmetric: bool,
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            angle_tolerance: 0.01, // ~0.57 度
            distance_tolerance: 0.01,
            min_confidence: 0.8,
            detect_parallel: true,
            detect_perpendicular: true,
            detect_collinear: true,
            detect_tangent: true,
            detect_concentric: true,
            detect_connected: true,
            detect_symmetric: false,
        }
    }
}

impl ReasoningConfig {
    /// 验证配置参数的合理性
    ///
    /// # Errors
    /// 如果配置参数无效，返回 `CadAgentError::Config`
    pub fn validate(&self) -> CadAgentResult<()> {
        // 验证角度容差：0.01 弧度 ≈ 0.57 度，最大值设为 90 度（π/2）
        if self.angle_tolerance <= 0.0 {
            return Err(CadAgentError::Config(format!(
                "角度容差必须为正数，当前值：{}。建议值：0.01（约 0.57 度）",
                self.angle_tolerance
            )));
        }
        if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            return Err(CadAgentError::Config(format!(
                "角度容差过大（{} 弧度 ≈ {:.2} 度），最大允许 90 度（π/2）。建议值：0.01",
                self.angle_tolerance,
                self.angle_tolerance.to_degrees()
            )));
        }

        // 验证距离容差
        if self.distance_tolerance < 0.0 {
            return Err(CadAgentError::Config(format!(
                "距离容差必须为非负数，当前值：{}",
                self.distance_tolerance
            )));
        }

        // 验证置信度阈值
        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            return Err(CadAgentError::Config(format!(
                "最小置信度必须在 0 到 1 之间，当前值：{}",
                self.min_confidence
            )));
        }

        Ok(())
    }

    /// 验证并自动修正不合理的配置
    ///
    /// 如果配置参数超出合理范围，会自动修正到默认值并返回警告信息
    pub fn validate_or_fix(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.angle_tolerance <= 0.0 {
            warnings.push(format!(
                "角度容差 {} 无效，已修正为默认值 {}",
                self.angle_tolerance,
                ReasoningConfig::default().angle_tolerance
            ));
            self.angle_tolerance = ReasoningConfig::default().angle_tolerance;
        } else if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            warnings.push(format!(
                "角度容差 {} 过大，已修正为默认值 {}",
                self.angle_tolerance,
                ReasoningConfig::default().angle_tolerance
            ));
            self.angle_tolerance = ReasoningConfig::default().angle_tolerance;
        }

        if self.distance_tolerance < 0.0 {
            warnings.push(format!(
                "距离容差 {} 无效，已修正为默认值 {}",
                self.distance_tolerance,
                ReasoningConfig::default().distance_tolerance
            ));
            self.distance_tolerance = ReasoningConfig::default().distance_tolerance;
        }

        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            warnings.push(format!(
                "最小置信度 {} 无效，已修正为默认值 {}",
                self.min_confidence,
                ReasoningConfig::default().min_confidence
            ));
            self.min_confidence = ReasoningConfig::default().min_confidence;
        }

        warnings
    }
}

/// 几何关系推理结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    /// 检测到的几何关系
    pub relations: Vec<GeometricRelation>,
    /// 关系统计
    pub statistics: RelationStatistics,
    /// 推理日志
    pub reasoning_log: Vec<String>,
}

/// 关系统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationStatistics {
    pub parallel_count: usize,
    pub perpendicular_count: usize,
    pub collinear_count: usize,
    pub tangent_count: usize,
    pub concentric_count: usize,
    pub connected_count: usize,
    pub contains_count: usize,
    pub equal_distance_count: usize,
    pub symmetric_count: usize,
    pub total_count: usize,
}

/// 几何关系推理器
#[derive(Debug, Clone)]
pub struct GeometricRelationReasoner {
    config: ReasoningConfig,
}

impl GeometricRelationReasoner {
    /// 创建新的推理器
    pub fn new(config: ReasoningConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建推理器
    pub fn with_defaults() -> Self {
        Self::new(ReasoningConfig::default())
    }

    /// 构建 R-tree 空间索引
    fn build_rtree(primitives: &[Primitive]) -> RTree<PrimitiveEnvelope> {
        let envelopes: Vec<PrimitiveEnvelope> = primitives
            .iter()
            .enumerate()
            .filter_map(|(id, p)| PrimitiveEnvelope::new(id, p))
            .collect();
        RTree::bulk_load(envelopes)
    }

    /// 查找所有几何关系（使用 R-tree 空间索引优化）
    ///
    /// # 性能说明
    ///
    /// 对于小量基元（< 50），直接使用 O(n²) 算法
    /// 对于大量基元（>= 50），使用 R-tree 空间索引优化到 O(n log n)
    pub fn find_all_relations(&self, primitives: &[Primitive]) -> ReasoningResult {
        let mut relations = Vec::new();
        let mut reasoning_log = Vec::new();

        // 提取线段和圆
        let lines: Vec<(usize, &Line)> = primitives
            .iter()
            .enumerate()
            .filter_map(|(id, p)| {
                if let Primitive::Line(line) = p {
                    Some((id, line))
                } else {
                    None
                }
            })
            .collect();

        let circles: Vec<(usize, &Circle)> = primitives
            .iter()
            .enumerate()
            .filter_map(|(id, p)| {
                if let Primitive::Circle(circle) = p {
                    Some((id, circle))
                } else {
                    None
                }
            })
            .collect();

        // 根据基元数量选择算法
        // 阈值设为 50：对于 50+ 基元的场景，R-tree 的空间索引优势开始显现
        let use_rtree = primitives.len() >= 50;

        if use_rtree {
            // 构建 R-tree 空间索引
            let rtree = Self::build_rtree(primitives);

            // 检测平行关系（使用空间索引优化）
            if self.config.detect_parallel {
                relations.extend(self.detect_parallel_rtree(&lines, &rtree));
            }

            // 检测垂直关系（使用空间索引优化）
            if self.config.detect_perpendicular {
                relations.extend(self.detect_perpendicular_rtree(&lines, &rtree));
            }

            // 检测共线关系（使用空间索引优化）
            if self.config.detect_collinear {
                relations.extend(self.detect_collinear_rtree(&lines, &rtree));
            }

            // 检测相切关系（使用空间索引优化）
            if self.config.detect_tangent {
                relations.extend(self.detect_tangent_line_circle_rtree(&lines, &circles, &rtree));
                relations.extend(self.detect_tangent_circle_circle_rtree(&circles, &rtree));
            }

            // 检测同心关系（使用空间索引优化）
            if self.config.detect_concentric {
                relations.extend(self.detect_concentric_rtree(&circles, &rtree));
            }

            // 检测连接关系
            relations.extend(self.detect_connected(primitives));

            // 检测等距关系
            relations.extend(self.detect_equal_distance(&lines));

            // 检测包含关系
            relations.extend(self.detect_contains(primitives));

            reasoning_log.push(format!(
                "检测了 {} 个基元之间的几何关系（R-tree 优化）",
                primitives.len()
            ));
        } else {
            // 使用传统 O(n²) 算法
            // 检测平行关系
            if self.config.detect_parallel {
                relations.extend(self.detect_parallel(&lines));
            }

            // 检测垂直关系
            if self.config.detect_perpendicular {
                relations.extend(self.detect_perpendicular(&lines));
            }

            // 检测共线关系
            if self.config.detect_collinear {
                relations.extend(self.detect_collinear(&lines));
            }

            // 检测相切关系
            if self.config.detect_tangent {
                relations.extend(self.detect_tangent_line_circle(&lines, &circles));
                relations.extend(self.detect_tangent_circle_circle(&circles));
            }

            // 检测同心关系
            if self.config.detect_concentric {
                relations.extend(self.detect_concentric(&circles));
            }

            // 检测连接关系
            if self.config.detect_connected {
                relations.extend(self.detect_connected(primitives));
            }

            // 检测等距关系
            relations.extend(self.detect_equal_distance(&lines));

            // 检测包含关系
            relations.extend(self.detect_contains(primitives));

            reasoning_log.push(format!("检测了 {} 个基元之间的几何关系", primitives.len()));
        }

        // 过滤低置信度关系
        relations.retain(|r| self.get_confidence(r) >= self.config.min_confidence);

        // 计算统计信息
        let statistics = self.compute_statistics(&relations);

        reasoning_log.push(format!("发现 {} 个几何关系", relations.len()));

        ReasoningResult {
            relations,
            statistics,
            reasoning_log,
        }
    }

    /// 检测同心关系（使用 R-tree 优化）
    fn detect_concentric_rtree(
        &self,
        circles: &[(usize, &Circle)],
        rtree: &RTree<PrimitiveEnvelope>,
    ) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (id1, c1) in circles.iter() {
            // 使用 R-tree 查找邻近的圆
            let query_bbox = self.create_circle_bbox(c1, c1.radius * 1.5);

            let candidates: Vec<_> = rtree
                .locate_in_envelope(&query_bbox)
                .filter(|env| env.primitive_id != *id1)
                .collect();

            for env in candidates {
                let id2 = env.primitive_id;
                // 确保只处理一次每对圆
                if id2 <= *id1 {
                    continue;
                }

                if let Some((_, c2)) = circles.iter().find(|(i, _)| *i == id2) {
                    let center_dist = c1.center.distance(&c2.center);

                    if center_dist < self.config.distance_tolerance {
                        let confidence = 1.0 - (center_dist / self.config.distance_tolerance);
                        relations.push(GeometricRelation::Concentric {
                            circle1_id: *id1,
                            circle2_id: id2,
                            center_distance: center_dist,
                            confidence,
                        });
                    }
                }
            }
        }

        relations
    }

    /// 检测平行关系（使用 R-tree 优化）
    fn detect_parallel_rtree(
        &self,
        lines: &[(usize, &Line)],
        rtree: &RTree<PrimitiveEnvelope>,
    ) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (id1, line1) in lines.iter() {
            // 使用 R-tree 查找邻近的线段
            let query_bbox = self.create_expanded_bbox(
                line1.start.x,
                line1.start.y,
                line1.end.x,
                line1.end.y,
                0.1,
            );

            let candidates: Vec<_> = rtree
                .locate_in_envelope(&query_bbox)
                .filter(|env| env.primitive_id != *id1)
                .collect();

            for env in candidates {
                let id2 = env.primitive_id;
                // 确保只处理一次每对线段
                if id2 <= *id1 {
                    continue;
                }

                // 从 lines 数组中查找 line2
                if let Some((_, line2)) = lines.iter().find(|(i, _)| *i == id2) {
                    let dir1 = line1.direction();
                    let dir2 = line2.direction();

                    // 计算方向向量夹角的余弦值
                    let dot = dir1.x * dir2.x + dir1.y * dir2.y;
                    let angle_diff = (1.0 - dot.abs()).abs();

                    if angle_diff < self.config.angle_tolerance {
                        let confidence = 1.0 - (angle_diff / self.config.angle_tolerance);
                        relations.push(GeometricRelation::Parallel {
                            line1_id: *id1,
                            line2_id: id2,
                            angle_diff,
                            confidence,
                        });
                    }
                }
            }
        }

        relations
    }

    /// 检测垂直关系（使用 R-tree 优化）
    fn detect_perpendicular_rtree(
        &self,
        lines: &[(usize, &Line)],
        rtree: &RTree<PrimitiveEnvelope>,
    ) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (id1, line1) in lines.iter() {
            // 使用 R-tree 查找邻近的线段
            let query_bbox = self.create_expanded_bbox(
                line1.start.x,
                line1.start.y,
                line1.end.x,
                line1.end.y,
                0.1,
            );

            let candidates: Vec<_> = rtree
                .locate_in_envelope(&query_bbox)
                .filter(|env| env.primitive_id != *id1)
                .collect();

            for env in candidates {
                let id2 = env.primitive_id;
                // 确保只处理一次每对线段
                if id2 <= *id1 {
                    continue;
                }

                if let Some((_, line2)) = lines.iter().find(|(i, _)| *i == id2) {
                    let dir1 = line1.direction();
                    let dir2 = line2.direction();

                    // 垂直时点积为 0
                    let dot = (dir1.x * dir2.x + dir1.y * dir2.y).abs();

                    if dot < self.config.angle_tolerance {
                        let confidence = 1.0 - (dot / self.config.angle_tolerance);
                        relations.push(GeometricRelation::Perpendicular {
                            line1_id: *id1,
                            line2_id: id2,
                            angle_diff: dot,
                            confidence,
                        });
                    }
                }
            }
        }

        relations
    }

    /// 检测共线关系（使用 R-tree 优化）
    fn detect_collinear_rtree(
        &self,
        lines: &[(usize, &Line)],
        rtree: &RTree<PrimitiveEnvelope>,
    ) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (id1, line1) in lines.iter() {
            // 使用 R-tree 查找邻近的线段
            let query_bbox = self.create_expanded_bbox(
                line1.start.x,
                line1.start.y,
                line1.end.x,
                line1.end.y,
                self.config.distance_tolerance,
            );

            let candidates: Vec<_> = rtree
                .locate_in_envelope(&query_bbox)
                .filter(|env| env.primitive_id != *id1)
                .collect();

            for env in candidates {
                let id2 = env.primitive_id;
                // 确保只处理一次每对线段
                if id2 <= *id1 {
                    continue;
                }

                if let Some((_, line2)) = lines.iter().find(|(i, _)| *i == id2) {
                    // 先检查是否平行
                    let dir1 = line1.direction();
                    let dir2 = line2.direction();
                    let dot = dir1.x * dir2.x + dir1.y * dir2.y;

                    if (1.0 - dot.abs()).abs() > self.config.angle_tolerance {
                        continue;
                    }

                    // 检查 line2 的起点到 line1 的距离
                    let dist = self.point_to_line_distance(line2.start, line1);

                    if dist < self.config.distance_tolerance {
                        let confidence = 1.0 - (dist / self.config.distance_tolerance);
                        relations.push(GeometricRelation::Collinear {
                            line1_id: *id1,
                            line2_id: id2,
                            distance: dist,
                            confidence,
                        });
                    }
                }
            }
        }

        relations
    }

    /// 检测线与圆的相切关系（使用 R-tree 优化）
    fn detect_tangent_line_circle_rtree(
        &self,
        lines: &[(usize, &Line)],
        circles: &[(usize, &Circle)],
        rtree: &RTree<PrimitiveEnvelope>,
    ) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (line_id, line) in lines {
            // 使用 R-tree 查找邻近的圆
            let query_bbox =
                self.create_expanded_bbox(line.start.x, line.start.y, line.end.x, line.end.y, 0.1);

            let candidates: Vec<_> = rtree.locate_in_envelope(&query_bbox).collect();

            for env in candidates {
                if let Some((circle_id, circle)) =
                    circles.iter().find(|(i, _)| *i == env.primitive_id)
                {
                    let dist = self.point_to_line_distance(circle.center, line);
                    let expected_dist = circle.radius;
                    let diff = (dist - expected_dist).abs();

                    if diff < self.config.distance_tolerance {
                        let confidence = 1.0 - (diff / self.config.distance_tolerance);
                        relations.push(GeometricRelation::TangentLineCircle {
                            line_id: *line_id,
                            circle_id: *circle_id,
                            distance: diff,
                            confidence,
                        });
                    }
                }
            }
        }

        relations
    }

    /// 检测圆与圆的相切关系（使用 R-tree 优化）
    fn detect_tangent_circle_circle_rtree(
        &self,
        circles: &[(usize, &Circle)],
        rtree: &RTree<PrimitiveEnvelope>,
    ) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (id1, c1) in circles.iter() {
            // 使用 R-tree 查找邻近的圆
            let query_bbox = self.create_circle_bbox(c1, c1.radius * 2.5);

            let candidates: Vec<_> = rtree
                .locate_in_envelope(&query_bbox)
                .filter(|env| env.primitive_id != *id1)
                .collect();

            for env in candidates {
                let id2 = env.primitive_id;
                // 确保只处理一次每对圆
                if id2 <= *id1 {
                    continue;
                }

                if let Some((_, c2)) = circles.iter().find(|(i, _)| *i == id2) {
                    let center_dist = c1.center.distance(&c2.center);
                    let sum_radii = c1.radius + c2.radius;
                    let diff_radii = (c1.radius - c2.radius).abs();

                    // 外切
                    let outer_diff = (center_dist - sum_radii).abs();
                    // 内切
                    let inner_diff = (center_dist - diff_radii).abs();

                    if outer_diff < self.config.distance_tolerance {
                        let confidence = 1.0 - (outer_diff / self.config.distance_tolerance);
                        relations.push(GeometricRelation::TangentCircleCircle {
                            circle1_id: *id1,
                            circle2_id: id2,
                            distance: outer_diff,
                            confidence,
                        });
                    } else if inner_diff < self.config.distance_tolerance {
                        let confidence = 1.0 - (inner_diff / self.config.distance_tolerance);
                        relations.push(GeometricRelation::TangentCircleCircle {
                            circle1_id: *id1,
                            circle2_id: id2,
                            distance: inner_diff,
                            confidence,
                        });
                    }
                }
            }
        }

        relations
    }

    /// 创建扩展的包围盒
    fn create_expanded_bbox(
        &self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        margin: f64,
    ) -> AABB<[f64; 2]> {
        let min_x = x1.min(x2) - margin;
        let min_y = y1.min(y2) - margin;
        let max_x = x1.max(x2) + margin;
        let max_y = y1.max(y2) + margin;

        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }

    /// 创建圆的包围盒
    fn create_circle_bbox(&self, circle: &Circle, margin: f64) -> AABB<[f64; 2]> {
        let min_x = circle.center.x - circle.radius - margin;
        let min_y = circle.center.y - circle.radius - margin;
        let max_x = circle.center.x + circle.radius + margin;
        let max_y = circle.center.y + circle.radius + margin;

        AABB::from_corners([min_x, min_y], [max_x, max_y])
    }

    /// 检测平行关系
    fn detect_parallel(&self, lines: &[(usize, &Line)]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for i in 0..lines.len() {
            for j in (i + 1)..lines.len() {
                let (id1, line1) = lines[i];
                let (id2, line2) = lines[j];

                let dir1 = line1.direction();
                let dir2 = line2.direction();

                // 计算方向向量夹角的余弦值
                let dot = dir1.x * dir2.x + dir1.y * dir2.y;
                let angle_diff = (1.0 - dot.abs()).abs();

                if angle_diff < self.config.angle_tolerance {
                    let confidence = 1.0 - (angle_diff / self.config.angle_tolerance);
                    relations.push(GeometricRelation::Parallel {
                        line1_id: id1,
                        line2_id: id2,
                        angle_diff,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测垂直关系
    fn detect_perpendicular(&self, lines: &[(usize, &Line)]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for i in 0..lines.len() {
            for j in (i + 1)..lines.len() {
                let (id1, line1) = lines[i];
                let (id2, line2) = lines[j];

                let dir1 = line1.direction();
                let dir2 = line2.direction();

                // 垂直时点积为 0
                let dot = (dir1.x * dir2.x + dir1.y * dir2.y).abs();

                if dot < self.config.angle_tolerance {
                    let confidence = 1.0 - (dot / self.config.angle_tolerance);
                    relations.push(GeometricRelation::Perpendicular {
                        line1_id: id1,
                        line2_id: id2,
                        angle_diff: dot,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测共线关系
    fn detect_collinear(&self, lines: &[(usize, &Line)]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for i in 0..lines.len() {
            for j in (i + 1)..lines.len() {
                let (id1, line1) = lines[i];
                let (id2, line2) = lines[j];

                // 先检查是否平行
                let dir1 = line1.direction();
                let dir2 = line2.direction();
                let dot = dir1.x * dir2.x + dir1.y * dir2.y;

                if (1.0 - dot.abs()).abs() > self.config.angle_tolerance {
                    continue;
                }

                // 检查 line2 的起点到 line1 的距离
                let dist = self.point_to_line_distance(line2.start, line1);

                if dist < self.config.distance_tolerance {
                    let confidence = 1.0 - (dist / self.config.distance_tolerance);
                    relations.push(GeometricRelation::Collinear {
                        line1_id: id1,
                        line2_id: id2,
                        distance: dist,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测线与圆的相切关系
    fn detect_tangent_line_circle(
        &self,
        lines: &[(usize, &Line)],
        circles: &[(usize, &Circle)],
    ) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (line_id, line) in lines {
            for (circle_id, circle) in circles {
                let dist = self.point_to_line_distance(circle.center, line);
                let expected_dist = circle.radius;
                let diff = (dist - expected_dist).abs();

                if diff < self.config.distance_tolerance {
                    let confidence = 1.0 - (diff / self.config.distance_tolerance);
                    relations.push(GeometricRelation::TangentLineCircle {
                        line_id: *line_id,
                        circle_id: *circle_id,
                        distance: diff,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测圆与圆的相切关系
    fn detect_tangent_circle_circle(&self, circles: &[(usize, &Circle)]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for i in 0..circles.len() {
            for j in (i + 1)..circles.len() {
                let (id1, c1) = circles[i];
                let (id2, c2) = circles[j];

                let center_dist = c1.center.distance(&c2.center);
                let sum_radii = c1.radius + c2.radius;
                let diff_radii = (c1.radius - c2.radius).abs();

                // 外切
                let outer_diff = (center_dist - sum_radii).abs();
                // 内切
                let inner_diff = (center_dist - diff_radii).abs();

                if outer_diff < self.config.distance_tolerance {
                    let confidence = 1.0 - (outer_diff / self.config.distance_tolerance);
                    relations.push(GeometricRelation::TangentCircleCircle {
                        circle1_id: id1,
                        circle2_id: id2,
                        distance: outer_diff,
                        confidence,
                    });
                } else if inner_diff < self.config.distance_tolerance {
                    let confidence = 1.0 - (inner_diff / self.config.distance_tolerance);
                    relations.push(GeometricRelation::TangentCircleCircle {
                        circle1_id: id1,
                        circle2_id: id2,
                        distance: inner_diff,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测同心关系
    fn detect_concentric(&self, circles: &[(usize, &Circle)]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for i in 0..circles.len() {
            for j in (i + 1)..circles.len() {
                let (id1, c1) = circles[i];
                let (id2, c2) = circles[j];

                let center_dist = c1.center.distance(&c2.center);

                if center_dist < self.config.distance_tolerance {
                    let confidence = 1.0 - (center_dist / self.config.distance_tolerance);
                    relations.push(GeometricRelation::Concentric {
                        circle1_id: id1,
                        circle2_id: id2,
                        center_distance: center_dist,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测连接关系
    fn detect_connected(&self, primitives: &[Primitive]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        // 提取所有端点
        let mut endpoints: Vec<(usize, Point, usize)> = Vec::new(); // (primitive_id, point, endpoint_index)

        for (id, prim) in primitives.iter().enumerate() {
            match prim {
                Primitive::Line(line) => {
                    endpoints.push((id, line.start, 0));
                    endpoints.push((id, line.end, 1));
                }
                Primitive::Polygon(poly) => {
                    for (i, pt) in poly.vertices.iter().enumerate() {
                        endpoints.push((id, *pt, i));
                    }
                }
                Primitive::Polyline { points, .. } => {
                    for (i, pt) in points.iter().enumerate() {
                        endpoints.push((id, *pt, i));
                    }
                }
                _ => {}
            }
        }

        // 查找重合的端点
        for i in 0..endpoints.len() {
            for j in (i + 1)..endpoints.len() {
                let (id1, pt1, _) = endpoints[i];
                let (id2, pt2, _) = endpoints[j];

                if id1 == id2 {
                    continue;
                }

                let dist = pt1.distance(&pt2);
                if dist < self.config.distance_tolerance {
                    let confidence = 1.0 - (dist / self.config.distance_tolerance);
                    relations.push(GeometricRelation::Connected {
                        primitive1_id: id1,
                        primitive2_id: id2,
                        connection_point: pt1,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测等距关系
    fn detect_equal_distance(&self, lines: &[(usize, &Line)]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for i in 0..lines.len() {
            for j in (i + 1)..lines.len() {
                let (id1, line1) = lines[i];
                let (id2, line2) = lines[j];

                let len1 = line1.length();
                let len2 = line2.length();
                let diff = (len1 - len2).abs();
                let avg_len = (len1 + len2) / 2.0;

                if avg_len > 0.0 && diff / avg_len < self.config.distance_tolerance {
                    let confidence = 1.0 - (diff / avg_len / self.config.distance_tolerance);
                    relations.push(GeometricRelation::EqualDistance {
                        line1_id: id1,
                        line2_id: id2,
                        length_diff: diff,
                        confidence,
                    });
                }
            }
        }

        relations
    }

    /// 检测包含关系
    fn detect_contains(&self, primitives: &[Primitive]) -> Vec<GeometricRelation> {
        let mut relations = Vec::new();

        for (id1, prim1) in primitives.iter().enumerate() {
            for (id2, prim2) in primitives.iter().enumerate() {
                if id1 == id2 {
                    continue;
                }

                match (prim1, prim2) {
                    // 点在线/圆/多边形上
                    (Primitive::Line(line), Primitive::Point(pt)) => {
                        let dist = self.point_to_line_distance(*pt, line);
                        if dist < self.config.distance_tolerance {
                            let confidence = 1.0 - (dist / self.config.distance_tolerance);
                            relations.push(GeometricRelation::Contains {
                                container_id: id1,
                                contained_id: id2,
                                relation: ContainmentType::PointOnLine,
                                confidence,
                            });
                        }
                    }
                    (Primitive::Circle(circle), Primitive::Point(pt)) => {
                        let dist = circle.center.distance(pt);
                        let diff = (dist - circle.radius).abs();
                        if diff < self.config.distance_tolerance {
                            let confidence = 1.0 - (diff / self.config.distance_tolerance);
                            relations.push(GeometricRelation::Contains {
                                container_id: id1,
                                contained_id: id2,
                                relation: ContainmentType::PointOnCircle,
                                confidence,
                            });
                        }
                    }
                    (Primitive::Polygon(poly), Primitive::Point(pt)) => {
                        if self.point_in_polygon(pt, poly) {
                            relations.push(GeometricRelation::Contains {
                                container_id: id1,
                                contained_id: id2,
                                relation: ContainmentType::PointInPolygon,
                                confidence: 1.0,
                            });
                        }
                    }
                    (Primitive::Rect(rect), Primitive::Point(pt)) => {
                        if rect.contains(pt) {
                            relations.push(GeometricRelation::Contains {
                                container_id: id1,
                                contained_id: id2,
                                relation: ContainmentType::PointInRect,
                                confidence: 1.0,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        relations
    }

    /// 点到直线的距离
    fn point_to_line_distance(&self, point: Point, line: &Line) -> f64 {
        let dx = line.end.x - line.start.x;
        let dy = line.end.y - line.start.y;

        if dx == 0.0 && dy == 0.0 {
            return point.distance(&line.start);
        }

        let t =
            ((point.x - line.start.x) * dx + (point.y - line.start.y) * dy) / (dx * dx + dy * dy);

        let t_clamped = t.clamp(0.0, 1.0);

        let closest = Point::new(line.start.x + t_clamped * dx, line.start.y + t_clamped * dy);

        point.distance(&closest)
    }

    /// 点是否在多边形内（射线法）
    fn point_in_polygon(&self, point: &Point, polygon: &Polygon) -> bool {
        if polygon.vertices.len() < 3 {
            return false;
        }

        let mut inside = false;
        let n = polygon.vertices.len();

        for i in 0..n {
            let j = (i + 1) % n;
            let vi = &polygon.vertices[i];
            let vj = &polygon.vertices[j];

            if ((vi.y > point.y) != (vj.y > point.y))
                && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }
        }

        inside
    }

    /// 计算统计信息
    fn compute_statistics(&self, relations: &[GeometricRelation]) -> RelationStatistics {
        let mut stats = RelationStatistics {
            parallel_count: 0,
            perpendicular_count: 0,
            collinear_count: 0,
            tangent_count: 0,
            concentric_count: 0,
            connected_count: 0,
            contains_count: 0,
            equal_distance_count: 0,
            symmetric_count: 0,
            total_count: relations.len(),
        };

        for rel in relations {
            match rel {
                GeometricRelation::Parallel { .. } => stats.parallel_count += 1,
                GeometricRelation::Perpendicular { .. } => stats.perpendicular_count += 1,
                GeometricRelation::Collinear { .. } => stats.collinear_count += 1,
                GeometricRelation::TangentLineCircle { .. }
                | GeometricRelation::TangentCircleCircle { .. } => stats.tangent_count += 1,
                GeometricRelation::Concentric { .. } => stats.concentric_count += 1,
                GeometricRelation::Connected { .. } => stats.connected_count += 1,
                GeometricRelation::Contains { .. } => stats.contains_count += 1,
                GeometricRelation::EqualDistance { .. } => stats.equal_distance_count += 1,
                GeometricRelation::Symmetric { .. } => stats.symmetric_count += 1,
            }
        }

        stats
    }

    /// 获取关系的置信度
    fn get_confidence(&self, relation: &GeometricRelation) -> f64 {
        match relation {
            GeometricRelation::Parallel { confidence, .. } => *confidence,
            GeometricRelation::Perpendicular { confidence, .. } => *confidence,
            GeometricRelation::Collinear { confidence, .. } => *confidence,
            GeometricRelation::TangentLineCircle { confidence, .. } => *confidence,
            GeometricRelation::TangentCircleCircle { confidence, .. } => *confidence,
            GeometricRelation::Concentric { confidence, .. } => *confidence,
            GeometricRelation::Connected { confidence, .. } => *confidence,
            GeometricRelation::Contains { confidence, .. } => *confidence,
            GeometricRelation::EqualDistance { confidence, .. } => *confidence,
            GeometricRelation::Symmetric { confidence, .. } => *confidence,
        }
    }
}

/// 几何关系推理工具（tokitai 工具封装）
#[derive(Default, Clone)]
pub struct GeometricReasoningTools;

#[tool]
impl GeometricReasoningTools {
    /// 查找几何基元之间的所有关系
    ///
    /// # 参数
    ///
    /// * `primitives_json` - 基元列表（JSON 格式）
    /// * `config_json` - 可选的配置（JSON 格式）
    ///
    /// # 返回
    ///
    /// 包含关系列表、统计信息和推理日志的结构化结果
    #[tool(name = "cad_find_geometric_relations")]
    pub fn find_relations(
        &self,
        primitives_json: String,
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

        let config: ReasoningConfig = config_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let reasoner = GeometricRelationReasoner::new(config);
        let result = reasoner.find_all_relations(&primitives);

        serde_json::json!({
            "success": true,
            "relations": result.relations,
            "statistics": result.statistics,
            "reasoning_log": result.reasoning_log
        })
    }

    /// 检测特定类型的关系
    #[tool(name = "cad_check_relation")]
    pub fn check_relation(
        &self,
        primitives_json: String,
        relation_type: String,
        primitive_ids: Vec<usize>,
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

        if primitive_ids.len() < 2 {
            return serde_json::json!({
                "success": false,
                "error": "至少需要两个基元 ID"
            });
        }

        let reasoner = GeometricRelationReasoner::with_defaults();
        let result = reasoner.find_all_relations(&primitives);

        // 过滤指定类型的关系
        let filtered: Vec<&GeometricRelation> = result
            .relations
            .iter()
            .filter(|r| {
                let matches_type = match relation_type.to_lowercase().as_str() {
                    "parallel" => matches!(r, GeometricRelation::Parallel { .. }),
                    "perpendicular" => matches!(r, GeometricRelation::Perpendicular { .. }),
                    "collinear" => matches!(r, GeometricRelation::Collinear { .. }),
                    "tangent" => matches!(
                        r,
                        GeometricRelation::TangentLineCircle { .. }
                            | GeometricRelation::TangentCircleCircle { .. }
                    ),
                    "concentric" => matches!(r, GeometricRelation::Concentric { .. }),
                    "connected" => matches!(r, GeometricRelation::Connected { .. }),
                    _ => true,
                };

                // 检查是否涉及指定的基元
                let matches_ids = self.relation_involves_primitives(r, &primitive_ids);

                matches_type && matches_ids
            })
            .collect();

        serde_json::json!({
            "success": true,
            "relations": filtered,
            "count": filtered.len()
        })
    }

    /// 获取推理器配置信息
    #[tool(name = "cad_get_reasoning_config_info")]
    pub fn get_config_info(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "geometric_relation_reasoner",
            "description": "CAD 几何关系推理：检测平行、垂直、相切、同心等约束关系",
            "supported_relations": [
                "parallel", "perpendicular", "collinear",
                "tangent", "concentric", "connected",
                "contains", "equal_distance", "symmetric"
            ],
            "config_params": {
                "angle_tolerance": "角度容差（弧度），默认 0.01",
                "distance_tolerance": "距离容差，默认 0.01",
                "min_confidence": "最小置信度，默认 0.8"
            }
        })
    }
}

impl GeometricReasoningTools {
    fn relation_involves_primitives(&self, relation: &GeometricRelation, ids: &[usize]) -> bool {
        match relation {
            GeometricRelation::Parallel {
                line1_id, line2_id, ..
            } => ids.contains(line1_id) || ids.contains(line2_id),
            GeometricRelation::Perpendicular {
                line1_id, line2_id, ..
            } => ids.contains(line1_id) || ids.contains(line2_id),
            GeometricRelation::Collinear {
                line1_id, line2_id, ..
            } => ids.contains(line1_id) || ids.contains(line2_id),
            GeometricRelation::TangentLineCircle {
                line_id, circle_id, ..
            } => ids.contains(line_id) || ids.contains(circle_id),
            GeometricRelation::TangentCircleCircle {
                circle1_id,
                circle2_id,
                ..
            } => ids.contains(circle1_id) || ids.contains(circle2_id),
            GeometricRelation::Concentric {
                circle1_id,
                circle2_id,
                ..
            } => ids.contains(circle1_id) || ids.contains(circle2_id),
            GeometricRelation::Connected {
                primitive1_id,
                primitive2_id,
                ..
            } => ids.contains(primitive1_id) || ids.contains(primitive2_id),
            GeometricRelation::Contains {
                container_id,
                contained_id,
                ..
            } => ids.contains(container_id) || ids.contains(contained_id),
            GeometricRelation::EqualDistance {
                line1_id, line2_id, ..
            } => ids.contains(line1_id) || ids.contains(line2_id),
            GeometricRelation::Symmetric {
                primitive1_id,
                primitive2_id,
                ..
            } => ids.contains(primitive1_id) || ids.contains(primitive2_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::primitives::Line;

    #[test]
    fn test_detect_parallel() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 1.0], [1.0, 1.0])),
        ];

        let reasoner = GeometricRelationReasoner::with_defaults();
        let result = reasoner.find_all_relations(&primitives);

        assert!(result.statistics.parallel_count > 0);
    }

    #[test]
    fn test_detect_perpendicular() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 1.0])),
        ];

        let reasoner = GeometricRelationReasoner::with_defaults();
        let result = reasoner.find_all_relations(&primitives);

        assert!(result.statistics.perpendicular_count > 0);
    }

    #[test]
    fn test_detect_concentric() {
        let primitives = vec![
            Primitive::Circle(Circle::from_coords([0.0, 0.0], 1.0)),
            Primitive::Circle(Circle::from_coords([0.0, 0.0], 2.0)),
        ];

        let reasoner = GeometricRelationReasoner::with_defaults();
        let result = reasoner.find_all_relations(&primitives);

        assert!(result.statistics.concentric_count > 0);
    }

    #[test]
    fn test_find_relations_tool() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 0.0])),
            Primitive::Line(Line::from_coords([0.0, 0.0], [0.0, 1.0])),
        ];

        let tools = GeometricReasoningTools;
        let result = tools.find_relations(serde_json::to_string(&primitives).unwrap(), None);

        assert!(result["success"].as_bool().unwrap_or(false));
        assert!(result["statistics"]["total_count"].as_u64().unwrap_or(0) > 0);
    }
}
