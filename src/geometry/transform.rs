//! 几何变换工具
//!
//! 提供平移、旋转、缩放、镜像等变换功能

use crate::geometry::{Point, Primitive, Line, Polygon, Circle, Rect};
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
        primitives.into_iter().map(|p| self.translate_primitive(p, dx, dy)).collect()
    }

    /// 旋转图元（绕指定中心点，角度为度数）
    pub fn rotate(
        &self,
        primitives: Vec<Primitive>,
        angle: f64,
        center: [f64; 2],
    ) -> Vec<Primitive> {
        let center = Point::from_array(center);
        primitives.into_iter().map(|p| self.rotate_primitive(p, angle, center)).collect()
    }

    /// 缩放图元（相对于指定中心点）
    pub fn scale(
        &self,
        primitives: Vec<Primitive>,
        factor: f64,
        center: [f64; 2],
    ) -> Vec<Primitive> {
        let center = Point::from_array(center);
        primitives.into_iter().map(|p| self.scale_primitive(p, factor, center)).collect()
    }

    /// 镜像图元（关于 X 轴或 Y 轴）
    pub fn mirror(&self, primitives: Vec<Primitive>, axis: MirrorAxis) -> Vec<Primitive> {
        primitives.into_iter().map(|p| self.mirror_primitive(p, axis)).collect()
    }

    /// 镜像图元（关于任意直线）
    pub fn mirror_about_line(
        &self,
        primitives: Vec<Primitive>,
        line_start: [f64; 2],
        line_end: [f64; 2],
    ) -> Vec<Primitive> {
        let line = Line::from_coords(line_start, line_end);
        primitives.into_iter().map(|p| self.mirror_primitive_about_line(p, &line)).collect()
    }
}

impl GeometryTransform {
    fn translate_primitive(&self, primitive: Primitive, dx: f64, dy: f64) -> Primitive {
        match primitive {
            Primitive::Point(p) => Primitive::Point(Point::new(p.x + dx, p.y + dy)),
            Primitive::Line(line) => Primitive::Line(Line::new(
                Point::new(line.start.x + dx, line.start.y + dy),
                Point::new(line.end.x + dx, line.end.y + dy),
            )),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.iter().map(|p| Point::new(p.x + dx, p.y + dy)).collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => Primitive::Circle(Circle::new(
                Point::new(circle.center.x + dx, circle.center.y + dy),
                circle.radius,
            )),
            Primitive::Rect(rect) => Primitive::Rect(Rect::new(
                Point::new(rect.min.x + dx, rect.min.y + dy),
                Point::new(rect.max.x + dx, rect.max.y + dy),
            )),
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.iter().map(|p| Point::new(p.x + dx, p.y + dy)).collect(),
                closed,
            },
            Primitive::Arc { center, radius, start_angle, end_angle } => Primitive::Arc {
                center: Point::new(center.x + dx, center.y + dy),
                radius,
                start_angle,
                end_angle,
            },
            Primitive::Text { content, position, height } => Primitive::Text {
                content,
                position: Point::new(position.x + dx, position.y + dy),
                height,
            },
        }
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

        match primitive {
            Primitive::Point(p) => Primitive::Point(rotate_point(p)),
            Primitive::Line(line) => Primitive::Line(Line::new(
                rotate_point(line.start),
                rotate_point(line.end),
            )),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.iter().map(|p| rotate_point(*p)).collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => Primitive::Circle(Circle::new(
                rotate_point(circle.center),
                circle.radius,
            )),
            Primitive::Rect(rect) => {
                // 矩形旋转后可能需要重新计算包围盒
                let corners = vec![
                    rotate_point(rect.min),
                    rotate_point(Point::new(rect.max.x, rect.min.y)),
                    rotate_point(rect.max),
                    rotate_point(Point::new(rect.min.x, rect.max.y)),
                ];
                // 简化处理：返回旋转后的多边形
                Primitive::Polygon(Polygon::new(corners))
            }
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.iter().map(|p| rotate_point(*p)).collect(),
                closed,
            },
            Primitive::Arc { center: c, radius, start_angle, end_angle } => Primitive::Arc {
                center: rotate_point(c),
                radius,
                start_angle: start_angle + angle,
                end_angle: end_angle + angle,
            },
            Primitive::Text { content, position, height } => Primitive::Text {
                content,
                position: rotate_point(position),
                height,
            },
        }
    }

    fn scale_primitive(&self, primitive: Primitive, factor: f64, center: Point) -> Primitive {
        let scale_point = |p: Point| -> Point {
            Point::new(
                center.x + (p.x - center.x) * factor,
                center.y + (p.y - center.y) * factor,
            )
        };

        match primitive {
            Primitive::Point(p) => Primitive::Point(scale_point(p)),
            Primitive::Line(line) => Primitive::Line(Line::new(
                scale_point(line.start),
                scale_point(line.end),
            )),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.iter().map(|p| scale_point(*p)).collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => Primitive::Circle(Circle::new(
                scale_point(circle.center),
                circle.radius * factor,
            )),
            Primitive::Rect(rect) => Primitive::Rect(Rect::new(
                scale_point(rect.min),
                scale_point(rect.max),
            )),
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.iter().map(|p| scale_point(*p)).collect(),
                closed,
            },
            Primitive::Arc { center, radius, start_angle, end_angle } => Primitive::Arc {
                center: scale_point(center),
                radius: radius * factor,
                start_angle,
                end_angle,
            },
            Primitive::Text { content, position, height } => Primitive::Text {
                content,
                position: scale_point(position),
                height: height * factor,
            },
        }
    }

    fn mirror_primitive(&self, primitive: Primitive, axis: MirrorAxis) -> Primitive {
        match axis {
            MirrorAxis::X => self.mirror_primitive_x(primitive),
            MirrorAxis::Y => self.mirror_primitive_y(primitive),
        }
    }

    fn mirror_primitive_x(&self, primitive: Primitive) -> Primitive {
        let mirror_point = |p: Point| -> Point {
            Point::new(p.x, -p.y)
        };

        match primitive {
            Primitive::Point(p) => Primitive::Point(mirror_point(p)),
            Primitive::Line(line) => Primitive::Line(Line::new(
                mirror_point(line.start),
                mirror_point(line.end),
            )),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.iter().map(|p| mirror_point(*p)).collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => Primitive::Circle(Circle::new(
                mirror_point(circle.center),
                circle.radius,
            )),
            Primitive::Rect(rect) => Primitive::Rect(Rect::new(
                mirror_point(rect.min),
                mirror_point(rect.max),
            )),
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.iter().map(|p| mirror_point(*p)).collect(),
                closed,
            },
            Primitive::Arc { center, radius, start_angle, end_angle } => Primitive::Arc {
                center: mirror_point(center),
                radius,
                start_angle: -start_angle,
                end_angle: -end_angle,
            },
            Primitive::Text { content, position, height } => Primitive::Text {
                content,
                position: mirror_point(position),
                height,
            },
        }
    }

    fn mirror_primitive_y(&self, primitive: Primitive) -> Primitive {
        let mirror_point = |p: Point| -> Point {
            Point::new(-p.x, p.y)
        };

        match primitive {
            Primitive::Point(p) => Primitive::Point(mirror_point(p)),
            Primitive::Line(line) => Primitive::Line(Line::new(
                mirror_point(line.start),
                mirror_point(line.end),
            )),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.iter().map(|p| mirror_point(*p)).collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => Primitive::Circle(Circle::new(
                mirror_point(circle.center),
                circle.radius,
            )),
            Primitive::Rect(rect) => Primitive::Rect(Rect::new(
                mirror_point(rect.min),
                mirror_point(rect.max),
            )),
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.iter().map(|p| mirror_point(*p)).collect(),
                closed,
            },
            Primitive::Arc { center, radius, start_angle, end_angle } => Primitive::Arc {
                center: mirror_point(center),
                radius,
                start_angle: 180.0 - start_angle,
                end_angle: 180.0 - end_angle,
            },
            Primitive::Text { content, position, height } => Primitive::Text {
                content,
                position: mirror_point(position),
                height,
            },
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
            let proj = Point::new(
                line.start.x + t * dx,
                line.start.y + t * dy,
            );

            // 镜像点 = 2 * 投影点 - 原点
            Point::new(2.0 * proj.x - p.x, 2.0 * proj.y - p.y)
        };

        match primitive {
            Primitive::Point(p) => Primitive::Point(mirror_point(p)),
            Primitive::Line(line) => Primitive::Line(Line::new(
                mirror_point(line.start),
                mirror_point(line.end),
            )),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.iter().map(|p| mirror_point(*p)).collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => Primitive::Circle(Circle::new(
                mirror_point(circle.center),
                circle.radius,
            )),
            Primitive::Rect(rect) => Primitive::Rect(Rect::new(
                mirror_point(rect.min),
                mirror_point(rect.max),
            )),
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.iter().map(|p| mirror_point(*p)).collect(),
                closed,
            },
            Primitive::Arc { center, radius, start_angle, end_angle } => Primitive::Arc {
                center: mirror_point(center),
                radius,
                start_angle,
                end_angle,
            },
            Primitive::Text { content, position, height } => Primitive::Text {
                content,
                position: mirror_point(position),
                height,
            },
        }
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
    pub fn translate(
        &self,
        primitives_json: String,
        dx: f64,
        dy: f64,
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
    pub fn mirror(
        &self,
        primitives_json: String,
        axis: String,
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

        let transform = GeometryTransform::default();
        let result = transform.translate(primitives, 10.0, 20.0);

        assert_eq!(result.len(), 2);
        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 10.0).abs() < 1e-10);
            assert!((p.y - 20.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_rotate() {
        let primitives = vec![Primitive::Point(Point::new(1.0, 0.0))];

        let transform = GeometryTransform::default();
        let result = transform.rotate(primitives, 90.0, [0.0, 0.0]);

        assert_eq!(result.len(), 1);
        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 0.0).abs() < 1e-10);
            assert!((p.y - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_scale() {
        let primitives = vec![Primitive::Point(Point::new(2.0, 4.0))];

        let transform = GeometryTransform::default();
        let result = transform.scale(primitives, 0.5, [0.0, 0.0]);

        assert_eq!(result.len(), 1);
        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 1.0).abs() < 1e-10);
            assert!((p.y - 2.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_mirror_x() {
        let primitives = vec![Primitive::Point(Point::new(3.0, 4.0))];

        let transform = GeometryTransform::default();
        let result = transform.mirror(primitives, MirrorAxis::X);

        assert_eq!(result.len(), 1);
        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 3.0).abs() < 1e-10);
            assert!((p.y - (-4.0)).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_transform_tools() {
        let tools = GeometryTransformTools::default();

        let primitives = r#"[{"type": "point", "x": 0.0, "y": 0.0}]"#;
        let result = tools.translate(primitives.to_string(), 10.0, 20.0);

        assert!(result["success"].as_bool().unwrap_or(false));
        assert_eq!(result["count"].as_u64().unwrap_or(0), 1);
    }
}
