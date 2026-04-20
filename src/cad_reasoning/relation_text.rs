//! 几何关系 → 自然语言转换规则库
//!
//! 提供统一的几何关系文本转换规则，用于生成可读性强的自然语言描述。
//! 支持多种输出风格：简洁模式、详细模式、技术模式。

#![allow(clippy::similar_names)]

use crate::cad_reasoning::GeometricRelation;
use crate::geometry::primitives::Primitive;

/// 文本输出风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextStyle {
    /// 简洁模式：使用符号表示（如 ∥, ⊥）
    Concise,
    /// 详细模式：使用完整自然语言描述
    #[default]
    Verbose,
    /// 技术模式：包含置信度和详细参数
    Technical,
}

/// 关系文本转换器
///
/// 提供几何关系到自然语言的转换规则
pub struct RelationTextConverter {
    /// 输出风格
    pub style: TextStyle,
    /// 是否显示置信度
    pub show_confidence: bool,
    /// 是否显示 IDs（而非类型 + 索引）
    pub show_ids: bool,
}

impl Default for RelationTextConverter {
    fn default() -> Self {
        Self {
            style: TextStyle::Verbose,
            show_confidence: false,
            show_ids: true,
        }
    }
}

impl RelationTextConverter {
    /// 创建新的转换器
    pub fn new(style: TextStyle) -> Self {
        Self {
            style,
            ..Default::default()
        }
    }

    /// 将单个关系转换为文本
    pub fn convert_relation(
        &self,
        relation: &GeometricRelation,
        primitives: &[Primitive],
    ) -> String {
        match self.style {
            TextStyle::Concise => self.convert_concise(relation),
            TextStyle::Verbose => self.convert_verbose(relation, primitives),
            TextStyle::Technical => self.convert_technical(relation, primitives),
        }
    }

    /// 将多个关系批量转换为文本
    pub fn convert_relations(
        &self,
        relations: &[GeometricRelation],
        primitives: &[Primitive],
    ) -> Vec<String> {
        relations
            .iter()
            .map(|r| self.convert_relation(r, primitives))
            .collect()
    }

    // ==================== 简洁模式 ====================

    fn convert_concise(&self, relation: &GeometricRelation) -> String {
        match relation {
            GeometricRelation::Parallel {
                line1_id, line2_id, ..
            } => {
                format!("line_{line1_id} ∥ line_{line2_id}")
            }
            GeometricRelation::Perpendicular {
                line1_id, line2_id, ..
            } => {
                format!("line_{line1_id} ⊥ line_{line2_id}")
            }
            GeometricRelation::Collinear {
                line1_id, line2_id, ..
            } => {
                format!("line_{line1_id} ≡ line_{line2_id}")
            }
            GeometricRelation::TangentLineCircle {
                line_id, circle_id, ..
            } => {
                format!("line_{line_id} tangent circle_{circle_id}")
            }
            GeometricRelation::TangentCircleCircle {
                circle1_id,
                circle2_id,
                ..
            } => {
                format!("circle_{circle1_id} tangent circle_{circle2_id}")
            }
            GeometricRelation::Concentric {
                circle1_id,
                circle2_id,
                ..
            } => {
                format!("circle_{circle1_id} concentric circle_{circle2_id}")
            }
            GeometricRelation::Connected {
                primitive1_id,
                primitive2_id,
                ..
            } => {
                format!("prim_{primitive1_id} — prim_{primitive2_id}")
            }
            GeometricRelation::Contains {
                container_id,
                contained_id,
                relation,
                ..
            } => {
                let type_str = match relation {
                    crate::cad_reasoning::ContainmentType::PointOnLine => "on line",
                    crate::cad_reasoning::ContainmentType::PointOnCircle => "on circle",
                    crate::cad_reasoning::ContainmentType::PointInPolygon => "in polygon",
                    crate::cad_reasoning::ContainmentType::PointInRect => "in rect",
                    crate::cad_reasoning::ContainmentType::CircleContainsPoint => {
                        "circle has point"
                    }
                    crate::cad_reasoning::ContainmentType::PolygonContainsPoint => {
                        "polygon has point"
                    }
                };
                format!("prim_{container_id} {type_str} prim_{contained_id}")
            }
            GeometricRelation::EqualDistance {
                line1_id, line2_id, ..
            } => {
                format!("line_{line1_id} = line_{line2_id}")
            }
            GeometricRelation::Symmetric {
                primitive1_id,
                primitive2_id,
                axis_line_id,
                symmetry_type,
                ..
            } => {
                let axis_str = if let Some(id) = axis_line_id {
                    format!(" about line_{id}")
                } else {
                    String::new()
                };
                let sym_type = match symmetry_type {
                    crate::cad_reasoning::SymmetryType::Axial => "axial",
                    crate::cad_reasoning::SymmetryType::Central => "central",
                };
                format!("prim_{primitive1_id} symmetric{axis_str} {sym_type} prim_{primitive2_id}")
            }
        }
    }

    // ==================== 详细模式 ====================

    fn convert_verbose(&self, relation: &GeometricRelation, primitives: &[Primitive]) -> String {
        match relation {
            GeometricRelation::Parallel {
                line1_id, line2_id, ..
            } => {
                let line1_desc = self.get_primitive_desc(*line1_id, primitives);
                let line2_desc = self.get_primitive_desc(*line2_id, primitives);
                format!("线段 {line1_desc} 与线段 {line2_desc} 平行")
            }
            GeometricRelation::Perpendicular {
                line1_id, line2_id, ..
            } => {
                let line1_desc = self.get_primitive_desc(*line1_id, primitives);
                let line2_desc = self.get_primitive_desc(*line2_id, primitives);
                format!("线段 {line1_desc} 与线段 {line2_desc} 垂直")
            }
            GeometricRelation::Collinear {
                line1_id, line2_id, ..
            } => {
                let line1_desc = self.get_primitive_desc(*line1_id, primitives);
                let line2_desc = self.get_primitive_desc(*line2_id, primitives);
                format!("线段 {line1_desc} 与线段 {line2_desc} 共线")
            }
            GeometricRelation::TangentLineCircle {
                line_id, circle_id, ..
            } => {
                let line_desc = self.get_primitive_desc(*line_id, primitives);
                let circle_desc = self.get_primitive_desc(*circle_id, primitives);
                format!("线段 {line_desc} 与圆 {circle_desc} 相切")
            }
            GeometricRelation::TangentCircleCircle {
                circle1_id,
                circle2_id,
                ..
            } => {
                let circle1_desc = self.get_primitive_desc(*circle1_id, primitives);
                let circle2_desc = self.get_primitive_desc(*circle2_id, primitives);
                format!("圆 {circle1_desc} 与圆 {circle2_desc} 相切")
            }
            GeometricRelation::Concentric {
                circle1_id,
                circle2_id,
                ..
            } => {
                let circle1_desc = self.get_primitive_desc(*circle1_id, primitives);
                let circle2_desc = self.get_primitive_desc(*circle2_id, primitives);
                format!("圆 {circle1_desc} 与圆 {circle2_desc} 同心")
            }
            GeometricRelation::Connected {
                primitive1_id,
                primitive2_id,
                connection_point,
                ..
            } => {
                let prim1_desc = self.get_primitive_desc(*primitive1_id, primitives);
                let prim2_desc = self.get_primitive_desc(*primitive2_id, primitives);
                format!(
                    "{} 与 {} 在点 ({:.2}, {:.2}) 处连接",
                    prim1_desc, prim2_desc, connection_point.x, connection_point.y
                )
            }
            GeometricRelation::Contains {
                container_id,
                contained_id,
                relation,
                ..
            } => {
                let container_desc = self.get_primitive_desc(*container_id, primitives);
                let contained_desc = self.get_primitive_desc(*contained_id, primitives);
                #[allow(clippy::match_same_arms)]
                let relation_desc = match relation {
                    crate::cad_reasoning::ContainmentType::PointOnLine => "点在",
                    crate::cad_reasoning::ContainmentType::PointOnCircle => "点在",
                    crate::cad_reasoning::ContainmentType::PointInPolygon => "点在",
                    crate::cad_reasoning::ContainmentType::PointInRect => "点在",
                    crate::cad_reasoning::ContainmentType::CircleContainsPoint => "圆包含点",
                    crate::cad_reasoning::ContainmentType::PolygonContainsPoint => "多边形包含点",
                };
                format!("{container_desc} {relation_desc} {contained_desc} 内部或上面")
            }
            GeometricRelation::EqualDistance {
                line1_id, line2_id, ..
            } => {
                let line1_desc = self.get_primitive_desc(*line1_id, primitives);
                let line2_desc = self.get_primitive_desc(*line2_id, primitives);
                format!("线段 {line1_desc} 与线段 {line2_desc} 等长")
            }
            GeometricRelation::Symmetric {
                primitive1_id,
                primitive2_id,
                axis_line_id,
                symmetry_type,
                ..
            } => {
                let prim1_desc = self.get_primitive_desc(*primitive1_id, primitives);
                let prim2_desc = self.get_primitive_desc(*primitive2_id, primitives);
                let axis_desc = if let Some(id) = axis_line_id {
                    let axis_desc = self.get_primitive_desc(*id, primitives);
                    format!(" 关于 {axis_desc} 轴对称")
                } else {
                    String::new()
                };
                let sym_type = match symmetry_type {
                    crate::cad_reasoning::SymmetryType::Axial => "",
                    crate::cad_reasoning::SymmetryType::Central => " 中心",
                };
                format!("{prim1_desc} 与 {prim2_desc}{axis_desc}{sym_type} 对称")
            }
        }
    }

    // ==================== 技术模式 ====================

    fn convert_technical(&self, relation: &GeometricRelation, primitives: &[Primitive]) -> String {
        let base_text = self.convert_verbose(relation, primitives);

        // 添加置信度信息
        let confidence = self.get_confidence(relation);
        let confidence_str = if self.show_confidence {
            format!(" (置信度：{:.0}%)", (confidence * 100.0))
        } else {
            String::new()
        };

        // 添加技术参数
        let params_str = self.get_technical_params(relation);

        format!("{base_text}{confidence_str}{params_str}")
    }

    // ==================== 辅助方法 ====================

    fn get_confidence(&self, relation: &GeometricRelation) -> f64 {
        #[allow(clippy::match_same_arms)]
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

    fn get_technical_params(&self, relation: &GeometricRelation) -> String {
        match relation {
            GeometricRelation::Parallel { angle_diff, .. } => {
                format!(" (角度差：{:.4}°)", angle_diff.to_degrees())
            }
            GeometricRelation::Perpendicular { angle_diff, .. } => {
                format!(
                    " (角度差：{:.4}°)",
                    (std::f64::consts::FRAC_PI_2 - angle_diff).to_degrees()
                )
            }
            GeometricRelation::Collinear { distance, .. } => {
                format!(" (距离：{distance:.4})")
            }
            GeometricRelation::TangentLineCircle { distance, .. }
            | GeometricRelation::TangentCircleCircle { distance, .. } => {
                format!(" (切线距离：{distance:.4})")
            }
            GeometricRelation::Concentric {
                center_distance, ..
            } => {
                format!(" (圆心距：{center_distance:.4})")
            }
            GeometricRelation::EqualDistance { length_diff, .. } => {
                format!(" (长度差：{length_diff:.4})")
            }
            _ => String::new(),
        }
    }

    fn get_primitive_desc(&self, id: usize, primitives: &[Primitive]) -> String {
        if !self.show_ids {
            return format!("#{id}");
        }

        if let Some(prim) = primitives.get(id) {
            match prim {
                Primitive::Point(_) => format!("点 #{id}"),
                Primitive::Line(_) => format!("线段 #{id}"),
                Primitive::Polygon(_) => format!("多边形 #{id}"),
                Primitive::Circle(_) => format!("圆 #{id}"),
                Primitive::Rect(_) => format!("矩形 #{id}"),
                Primitive::Arc { .. } => format!("弧 #{id}"),
                Primitive::Polyline { .. } => format!("多段线 #{id}"),
                Primitive::Text { .. } => format!("文本 #{id}"),
                Primitive::EllipticalArc { .. } => format!("椭圆弧 #{id}"),
                Primitive::BezierCurve { .. } => format!("贝塞尔曲线 #{id}"),
                Primitive::QuadraticBezier(_) => format!("二次贝塞尔曲线 #{id}"),
            }
        } else {
            format!("#{id}")
        }
    }
}

/// 便捷函数：使用默认转换器将关系转换为文本
pub fn relation_to_text(
    relation: &GeometricRelation,
    primitives: &[Primitive],
    style: TextStyle,
) -> String {
    let converter = RelationTextConverter::new(style);
    converter.convert_relation(relation, primitives)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::primitives::{Circle, Line};

    fn test_line() -> Primitive {
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0]))
    }

    #[allow(dead_code)]
    fn test_circle() -> Primitive {
        Primitive::Circle(Circle::new(crate::geometry::Point::new(50.0, 50.0), 10.0))
    }

    #[test]
    fn test_concise_parallel() {
        let converter = RelationTextConverter::new(TextStyle::Concise);
        let relation = GeometricRelation::Parallel {
            line1_id: 0,
            line2_id: 1,
            angle_diff: 0.001,
            confidence: 0.95,
        };
        let result = converter.convert_relation(&relation, &[test_line(), test_line()]);
        assert!(result.contains("∥"));
    }

    #[test]
    fn test_verbose_parallel() {
        let converter = RelationTextConverter::new(TextStyle::Verbose);
        let relation = GeometricRelation::Parallel {
            line1_id: 0,
            line2_id: 1,
            angle_diff: 0.001,
            confidence: 0.95,
        };
        let result = converter.convert_relation(&relation, &[test_line(), test_line()]);
        assert!(result.contains("平行"));
        assert!(result.contains("线段"));
    }

    #[test]
    fn test_technical_mode() {
        let converter = RelationTextConverter {
            style: TextStyle::Technical,
            show_confidence: true,
            show_ids: true,
        };
        let relation = GeometricRelation::Parallel {
            line1_id: 0,
            line2_id: 1,
            angle_diff: 0.001,
            confidence: 0.95,
        };
        let result = converter.convert_relation(&relation, &[test_line(), test_line()]);
        assert!(result.contains("平行"));
        assert!(result.contains("置信度"));
    }

    #[test]
    fn test_perpendicular_conversion() {
        let converter = RelationTextConverter::default();
        let relation = GeometricRelation::Perpendicular {
            line1_id: 0,
            line2_id: 1,
            angle_diff: 1.57,
            confidence: 0.98,
        };
        let result = converter.convert_relation(&relation, &[test_line(), test_line()]);
        assert!(result.contains("垂直"));
    }

    #[test]
    fn test_connected_conversion() {
        let converter = RelationTextConverter::default();
        let relation = GeometricRelation::Connected {
            primitive1_id: 0,
            primitive2_id: 1,
            connection_point: crate::geometry::Point::new(50.0, 50.0),
            confidence: 0.99,
        };
        let result = converter.convert_relation(&relation, &[test_line(), test_line()]);
        assert!(result.contains("连接"));
    }
}
