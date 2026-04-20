//! 几何变换工具
#![allow(clippy::cast_precision_loss)]
//!
//! 提供平移、旋转、缩放、镜像等变换功能

use crate::geometry::{
    BezierCurve, Circle, EllipticalArc, Line, Point, Polygon, Primitive, QuadraticBezier, Rect,
};
use serde::{Deserialize, Serialize};
use tokitai::tool;

/// 镜像轴
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorAxis {
    X,
    Y,
}

/// 几何变换工具集
#[derive(Default, Clone)]
pub struct GeometryTransform;

impl GeometryTransform {
    /// 平移图元
    pub fn translate(&self, primitives: Vec<Primitive>, dx: f64, dy: f64) -> Vec<Primitive> {
        primitives
            .into_iter()
            .map(|p| self.translate_primitive(p, dx, dy))
            .collect()
    }

    /// 旋转图元（绕指定中心点，角度为度数）
    pub fn rotate(
        &self,
        primitives: Vec<Primitive>,
        angle: f64,
        center: [f64; 2],
    ) -> Vec<Primitive> {
        let center = Point::from_array(center);
        primitives
            .into_iter()
            .map(|p| self.rotate_primitive(p, angle, center))
            .collect()
    }

    /// 缩放图元（相对于指定中心点）
    pub fn scale(
        &self,
        primitives: Vec<Primitive>,
        factor: f64,
        center: [f64; 2],
    ) -> Vec<Primitive> {
        let center = Point::from_array(center);
        primitives
            .into_iter()
            .map(|p| self.scale_primitive(p, factor, center))
            .collect()
    }

    /// 镜像图元（关于 X 轴或 Y 轴）
    pub fn mirror(&self, primitives: Vec<Primitive>, axis: MirrorAxis) -> Vec<Primitive> {
        primitives
            .into_iter()
            .map(|p| self.mirror_primitive(p, axis))
            .collect()
    }

    /// 镜像图元（关于任意直线）
    pub fn mirror_about_line(
        &self,
        primitives: Vec<Primitive>,
        line_start: [f64; 2],
        line_end: [f64; 2],
    ) -> Vec<Primitive> {
        let line = Line::from_coords(line_start, line_end);
        primitives
            .into_iter()
            .map(|p| self.mirror_primitive_about_line(p, &line))
            .collect()
    }
}

impl GeometryTransform {
    /// Generic helper function to transform a primitive by applying a point transformation function
    fn transform_primitive<F>(primitive: Primitive, transform_point: F) -> Primitive
    where
        F: Fn(Point) -> Point,
    {
        match primitive {
            Primitive::Point(p) => Primitive::Point(transform_point(p)),
            Primitive::Line(line) => Primitive::Line(Line::new(
                transform_point(line.start),
                transform_point(line.end),
            )),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.iter().map(|p| transform_point(*p)).collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => {
                Primitive::Circle(Circle::new(transform_point(circle.center), circle.radius))
            }
            Primitive::Rect(rect) => Primitive::Rect(Rect::new(
                transform_point(rect.min),
                transform_point(rect.max),
            )),
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.iter().map(|p| transform_point(*p)).collect(),
                closed,
            },
            Primitive::Arc {
                center,
                radius,
                start_angle,
                end_angle,
            } => Primitive::Arc {
                center: transform_point(center),
                radius,
                start_angle,
                end_angle,
            },
            Primitive::EllipticalArc(arc) => Primitive::EllipticalArc(EllipticalArc::new(
                transform_point(arc.start),
                arc.rx,
                arc.ry,
                arc.x_axis_rotation,
                arc.large_arc,
                arc.sweep,
                transform_point(arc.end),
            )),
            Primitive::BezierCurve(curve) => Primitive::BezierCurve(BezierCurve::new(
                transform_point(curve.start),
                transform_point(curve.control1),
                transform_point(curve.control2),
                transform_point(curve.end),
            )),
            Primitive::QuadraticBezier(curve) => Primitive::QuadraticBezier(QuadraticBezier::new(
                transform_point(curve.start),
                transform_point(curve.control),
                transform_point(curve.end),
            )),
            Primitive::Text {
                content,
                position,
                height,
            } => Primitive::Text {
                content,
                position: transform_point(position),
                height,
            },
        }
    }

    fn translate_primitive(&self, primitive: Primitive, dx: f64, dy: f64) -> Primitive {
        Self::transform_primitive(primitive, |p| Point::new(p.x + dx, p.y + dy))
    }

    fn rotate_primitive(&self, primitive: Primitive, angle: f64, center: Point) -> Primitive {
        let rad = angle.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();

        let rotate_point = |p: Point| -> Point {
            let dx = p.x - center.x;
            let dy = p.y - center.y;
            Point::new(
                center.x + dx * cos_a - dy * sin_a,
                center.y + dx * sin_a + dy * cos_a,
            )
        };

        // Special handling for Rect, Arc, and Text which need additional transformations
        match primitive {
            Primitive::Rect(rect) => {
                let corners = vec![
                    rotate_point(rect.min),
                    rotate_point(Point::new(rect.max.x, rect.min.y)),
                    rotate_point(rect.max),
                    rotate_point(Point::new(rect.min.x, rect.max.y)),
                ];
                Primitive::Polygon(Polygon::new(corners))
            }
            Primitive::Arc {
                center: c,
                radius,
                start_angle,
                end_angle,
            } => Primitive::Arc {
                center: rotate_point(c),
                radius,
                start_angle: start_angle + angle,
                end_angle: end_angle + angle,
            },
            Primitive::Text {
                content,
                position,
                height,
            } => Primitive::Text {
                content,
                position: rotate_point(position),
                height,
            },
            _ => Self::transform_primitive(primitive, rotate_point),
        }
    }

    fn scale_primitive(&self, primitive: Primitive, factor: f64, center: Point) -> Primitive {
        let scale_point = |p: Point| -> Point {
            Point::new(
                center.x + (p.x - center.x) * factor,
                center.y + (p.y - center.y) * factor,
            )
        };

        // Special handling for Circle, Arc, and Text which have scale-dependent properties
        match primitive {
            Primitive::Circle(circle) => Primitive::Circle(Circle::new(
                scale_point(circle.center),
                circle.radius * factor,
            )),
            Primitive::Arc {
                center,
                radius,
                start_angle,
                end_angle,
            } => Primitive::Arc {
                center: scale_point(center),
                radius: radius * factor,
                start_angle,
                end_angle,
            },
            Primitive::Text {
                content,
                position,
                height,
            } => Primitive::Text {
                content,
                position: scale_point(position),
                height: height * factor,
            },
            _ => Self::transform_primitive(primitive, scale_point),
        }
    }

    fn mirror_primitive(&self, primitive: Primitive, axis: MirrorAxis) -> Primitive {
        match axis {
            MirrorAxis::X => self.mirror_primitive_x(primitive),
            MirrorAxis::Y => self.mirror_primitive_y(primitive),
        }
    }

    fn mirror_primitive_x(&self, primitive: Primitive) -> Primitive {
        let mirror_point = |p: Point| -> Point { Point::new(p.x, -p.y) };

        // Special handling for Arc which needs angle transformation
        match primitive {
            Primitive::Arc {
                center,
                radius,
                start_angle,
                end_angle,
            } => Primitive::Arc {
                center: mirror_point(center),
                radius,
                start_angle: -start_angle,
                end_angle: -end_angle,
            },
            _ => Self::transform_primitive(primitive, mirror_point),
        }
    }

    fn mirror_primitive_y(&self, primitive: Primitive) -> Primitive {
        let mirror_point = |p: Point| -> Point { Point::new(-p.x, p.y) };

        // Special handling for Arc which needs angle transformation
        match primitive {
            Primitive::Arc {
                center,
                radius,
                start_angle,
                end_angle,
            } => Primitive::Arc {
                center: mirror_point(center),
                radius,
                start_angle: 180.0 - start_angle,
                end_angle: 180.0 - end_angle,
            },
            _ => Self::transform_primitive(primitive, mirror_point),
        }
    }

    fn mirror_primitive_about_line(&self, primitive: Primitive, line: &Line) -> Primitive {
        // 计算点关于直线的镜像点
        let mirror_point = |p: Point| -> Point {
            let dx = line.end.x - line.start.x;
            let dy = line.end.y - line.start.y;
            let len_sq = dx * dx + dy * dy;

            if len_sq == 0.0 {
                return p;
            }

            // 计算投影
            let t = ((p.x - line.start.x) * dx + (p.y - line.start.y) * dy) / len_sq;
            let proj = Point::new(line.start.x + t * dx, line.start.y + t * dy);

            // 镜像点 = 2 * 投影点 - 原点
            Point::new(2.0 * proj.x - p.x, 2.0 * proj.y - p.y)
        };

        Self::transform_primitive(primitive, mirror_point)
    }
}

// ==================== tokitai 工具封装 ====================

/// 几何变换工具（tokitai 工具封装）
#[derive(Default, Clone)]
pub struct GeometryTransformTools;

#[tool]
impl GeometryTransformTools {
    /// 平移图元
    ///
    /// # 参数
    ///
    /// * `primitives_json` - 图元列表（JSON 格式）
    /// * `dx` - X 方向平移量
    /// * `dy` - Y 方向平移量
    ///
    /// # 返回
    ///
    /// 平移后的图元列表
    #[tool(name = "translate")]
    pub fn translate(&self, primitives_json: String, dx: f64, dy: f64) -> serde_json::Value {
        let primitives: Vec<Primitive> = match serde_json::from_str(&primitives_json) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析失败：{}", e)
                });
            }
        };

        let transform = GeometryTransform;
        let result = transform.translate(primitives, dx, dy);

        serde_json::json!({
            "success": true,
            "primitives": result,
            "count": result.len()
        })
    }

    /// 旋转图元
    ///
    /// # 参数
    ///
    /// * `primitives_json` - 图元列表（JSON 格式）
    /// * `angle` - 旋转角度（度数）
    /// * `center_x` - 旋转中心 X 坐标
    /// * `center_y` - 旋转中心 Y 坐标
    ///
    /// # 返回
    ///
    /// 旋转后的图元列表
    #[tool(name = "rotate")]
    pub fn rotate(
        &self,
        primitives_json: String,
        angle: f64,
        center_x: f64,
        center_y: f64,
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

        let transform = GeometryTransform;
        let result = transform.rotate(primitives, angle, [center_x, center_y]);

        serde_json::json!({
            "success": true,
            "primitives": result,
            "count": result.len()
        })
    }

    /// 缩放图元
    ///
    /// # 参数
    ///
    /// * `primitives_json` - 图元列表（JSON 格式）
    /// * `factor` - 缩放因子
    /// * `center_x` - 缩放中心 X 坐标
    /// * `center_y` - 缩放中心 Y 坐标
    ///
    /// # 返回
    ///
    /// 缩放后的图元列表
    #[tool(name = "scale")]
    pub fn scale(
        &self,
        primitives_json: String,
        factor: f64,
        center_x: f64,
        center_y: f64,
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

        let transform = GeometryTransform;
        let result = transform.scale(primitives, factor, [center_x, center_y]);

        serde_json::json!({
            "success": true,
            "primitives": result,
            "count": result.len()
        })
    }

    /// 镜像图元（关于 X 轴或 Y 轴）
    ///
    /// # 参数
    ///
    /// * `primitives_json` - 图元列表（JSON 格式）
    /// * `axis` - 镜像轴："x" 或 "y"
    ///
    /// # 返回
    ///
    /// 镜像后的图元列表
    #[tool(name = "mirror")]
    pub fn mirror(&self, primitives_json: String, axis: String) -> serde_json::Value {
        let primitives: Vec<Primitive> = match serde_json::from_str(&primitives_json) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析失败：{}", e)
                });
            }
        };

        let mirror_axis = match axis.to_lowercase().as_str() {
            "x" => MirrorAxis::X,
            "y" => MirrorAxis::Y,
            _ => {
                return serde_json::json!({
                    "success": false,
                    "error": "无效的镜像轴，必须是 'x' 或 'y'"
                });
            }
        };

        let transform = GeometryTransform;
        let result = transform.mirror(primitives, mirror_axis);

        serde_json::json!({
            "success": true,
            "primitives": result,
            "count": result.len()
        })
    }

    /// 获取变换工具信息
    #[tool(name = "transform_get_info")]
    pub fn get_info(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "geometry_transform",
            "description": "几何变换工具：平移、旋转、缩放、镜像",
            "tools": [
                {"name": "translate", "description": "平移图元"},
                {"name": "rotate", "description": "旋转图元"},
                {"name": "scale", "description": "缩放图元"},
                {"name": "mirror", "description": "镜像图元"}
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate() {
        let primitives = vec![
            Primitive::Point(Point::new(0.0, 0.0)),
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 1.0])),
        ];

        let transform = GeometryTransform;
        let result = transform.translate(primitives, 10.0, 20.0);

        assert_eq!(result.len(), 2);
        let Primitive::Point(p) = &result[0] else {
            panic!("Expected Point variant");
        };
        assert!((p.x - 10.0).abs() < 1e-10);
        assert!((p.y - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_rotate() {
        let primitives = vec![Primitive::Point(Point::new(1.0, 0.0))];

        let transform = GeometryTransform;
        let result = transform.rotate(primitives, 90.0, [0.0, 0.0]);

        assert_eq!(result.len(), 1);
        let Primitive::Point(p) = &result[0] else {
            panic!("Expected Point variant");
        };
        assert!((p.x - 0.0).abs() < 1e-10);
        assert!((p.y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_scale() {
        let primitives = vec![Primitive::Point(Point::new(2.0, 4.0))];

        let transform = GeometryTransform;
        let result = transform.scale(primitives, 0.5, [0.0, 0.0]);

        assert_eq!(result.len(), 1);
        let Primitive::Point(p) = &result[0] else {
            panic!("Expected Point variant");
        };
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!((p.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_mirror_x() {
        let primitives = vec![Primitive::Point(Point::new(3.0, 4.0))];

        let transform = GeometryTransform;
        let result = transform.mirror(primitives, MirrorAxis::X);

        assert_eq!(result.len(), 1);
        let Primitive::Point(p) = &result[0] else {
            panic!("Expected Point variant");
        };
        assert!((p.x - 3.0).abs() < 1e-10);
        assert!((p.y - (-4.0)).abs() < 1e-10);
    }

    #[test]
    fn test_transform_tools() {
        let tools = GeometryTransformTools;

        let primitives = r#"[{"type": "point", "x": 0.0, "y": 0.0}]"#;
        let result = tools.translate(primitives.to_string(), 10.0, 20.0);

        assert!(result["success"].as_bool().unwrap_or(false));
        assert_eq!(result["count"].as_u64().unwrap_or(0), 1);
    }
}
