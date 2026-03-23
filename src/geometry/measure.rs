//! 几何测量工具
//!
//! 提供长度、面积、角度等测量功能

use crate::geometry::{Circle, Line, Point, Polygon, Rect};

/// 几何测量工具集
#[derive(Default, Clone)]
pub struct GeometryMeasurer;

impl GeometryMeasurer {
    /// 测量两点之间的线段长度
    pub fn measure_length(&self, start: [f64; 2], end: [f64; 2]) -> f64 {
        let p1 = Point::from_array(start);
        let p2 = Point::from_array(end);
        p1.distance(&p2)
    }

    /// 计算多边形面积（使用鞋带公式）
    pub fn measure_area(&self, vertices: Vec<[f64; 2]>) -> f64 {
        let polygon = Polygon::from_coords(vertices);
        polygon.area()
    }

    /// 测量三点形成的角度（以度为单位）
    pub fn measure_angle(&self, p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]) -> f64 {
        let v1 = (p1[0] - p2[0], p1[1] - p2[1]);
        let v2 = (p3[0] - p2[0], p3[1] - p2[1]);

        let dot = v1.0 * v2.0 + v1.1 * v2.1;
        let mag1 = (v1.0.powi(2) + v1.1.powi(2)).sqrt();
        let mag2 = (v2.0.powi(2) + v2.1.powi(2)).sqrt();

        if mag1 == 0.0 || mag2 == 0.0 {
            return 0.0;
        }

        let cos_angle = dot / (mag1 * mag2);
        let cos_angle = cos_angle.clamp(-1.0, 1.0);
        let angle_rad = cos_angle.acos();
        angle_rad.to_degrees()
    }

    /// 检查两条线段是否平行
    pub fn check_parallel(
        &self,
        line1_start: [f64; 2],
        line1_end: [f64; 2],
        line2_start: [f64; 2],
        line2_end: [f64; 2],
    ) -> ParallelResult {
        let line1 = Line::from_coords(line1_start, line1_end);
        let line2 = Line::from_coords(line2_start, line2_end);

        let dir1 = line1.direction();
        let dir2 = line2.direction();

        // 计算方向向量的叉积
        let cross = dir1.x * dir2.y - dir1.y * dir2.x;
        let angle_diff = (cross).abs().asin().to_degrees();

        ParallelResult {
            is_parallel: angle_diff < 1.0, // 角度差小于 1 度视为平行
            angle_diff,
        }
    }

    /// 检查两条线段是否垂直
    pub fn check_perpendicular(
        &self,
        line1_start: [f64; 2],
        line1_end: [f64; 2],
        line2_start: [f64; 2],
        line2_end: [f64; 2],
    ) -> PerpendicularResult {
        let line1 = Line::from_coords(line1_start, line1_end);
        let line2 = Line::from_coords(line2_start, line2_end);

        let dir1 = line1.direction();
        let dir2 = line2.direction();

        // 计算方向向量的点积
        let dot = dir1.x * dir2.x + dir1.y * dir2.y;
        let angle_diff = (90.0 - (dot).abs().acos().to_degrees()).abs();

        PerpendicularResult {
            is_perpendicular: angle_diff < 1.0, // 角度差小于 1 度视为垂直
            angle_diff,
        }
    }

    /// 计算矩形的宽度和高度
    pub fn measure_rect(&self, min: [f64; 2], max: [f64; 2]) -> RectDimensions {
        let rect = Rect::from_coords(min, max);
        RectDimensions {
            width: rect.width(),
            height: rect.height(),
            area: rect.area(),
            center: rect.center().to_array(),
        }
    }

    /// 计算圆的面积和周长
    pub fn measure_circle(&self, center: [f64; 2], radius: f64) -> CircleDimensions {
        let circle = Circle::from_coords(center, radius);
        CircleDimensions {
            radius: circle.radius,
            diameter: circle.diameter(),
            area: circle.area(),
            circumference: circle.circumference(),
        }
    }

    /// 计算多边形的周长
    pub fn measure_perimeter(&self, vertices: Vec<[f64; 2]>) -> f64 {
        let polygon = Polygon::from_coords(vertices);
        polygon.perimeter()
    }

    /// 计算点到直线的距离
    pub fn point_to_line_distance(
        &self,
        point: [f64; 2],
        line_start: [f64; 2],
        line_end: [f64; 2],
    ) -> f64 {
        let p = Point::from_array(point);
        let line = Line::from_coords(line_start, line_end);

        let a = (p.x - line.start.x) * (line.end.y - line.start.y)
            - (p.y - line.start.y) * (line.end.x - line.start.x);
        let b = line.length();

        if b == 0.0 {
            return p.distance(&line.start);
        }

        a.abs() / b
    }

    /// 计算两个点之间的中点
    pub fn midpoint(&self, p1: [f64; 2], p2: [f64; 2]) -> [f64; 2] {
        let line = Line::from_coords(p1, p2);
        line.midpoint().to_array()
    }
}

/// 平行检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParallelResult {
    pub is_parallel: bool,
    pub angle_diff: f64,
}

/// 垂直检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerpendicularResult {
    pub is_perpendicular: bool,
    pub angle_diff: f64,
}

/// 矩形尺寸
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RectDimensions {
    pub width: f64,
    pub height: f64,
    pub area: f64,
    pub center: [f64; 2],
}

/// 圆尺寸
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CircleDimensions {
    pub radius: f64,
    pub diameter: f64,
    pub area: f64,
    pub circumference: f64,
}

// 重新导出便捷类型
pub use ParallelResult as MeasureParallelResult;
pub use PerpendicularResult as MeasurePerpendicularResult;
pub use RectDimensions as MeasureRectResult;
pub use CircleDimensions as MeasureCircleResult;
