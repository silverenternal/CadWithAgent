//! 几何图元定义
//!
//! 提供 CAD 处理所需的基础几何图元类型

use serde::{Deserialize, Serialize};

/// 二维点
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn to_array(&self) -> [f64; 2] {
        [self.x, self.y]
    }

    pub fn from_array(arr: [f64; 2]) -> Self {
        Self {
            x: arr[0],
            y: arr[1],
        }
    }
}

/// 线段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub start: Point,
    pub end: Point,
}

impl Line {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    pub fn from_coords(start: [f64; 2], end: [f64; 2]) -> Self {
        Self {
            start: Point::from_array(start),
            end: Point::from_array(end),
        }
    }

    pub fn length(&self) -> f64 {
        self.start.distance(&self.end)
    }

    pub fn midpoint(&self) -> Point {
        Point::new(
            (self.start.x + self.end.x) / 2.0,
            (self.start.y + self.end.y) / 2.0,
        )
    }

    pub fn direction(&self) -> Point {
        let len = self.length();
        if len == 0.0 {
            return Point::origin();
        }
        Point::new(
            (self.end.x - self.start.x) / len,
            (self.end.y - self.start.y) / len,
        )
    }
}

/// 多边形（闭合的折线）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub vertices: Vec<Point>,
    pub closed: bool,
}

impl Polygon {
    pub fn new(vertices: Vec<Point>) -> Self {
        Self {
            vertices,
            closed: true,
        }
    }

    pub fn from_coords(coords: Vec<[f64; 2]>) -> Self {
        Self {
            vertices: coords.into_iter().map(Point::from_array).collect(),
            closed: true,
        }
    }

    /// 使用鞋带公式计算面积
    pub fn area(&self) -> f64 {
        if self.vertices.len() < 3 {
            return 0.0;
        }

        let mut sum = 0.0;
        let n = self.vertices.len();
        for i in 0..n {
            let j = (i + 1) % n;
            sum += self.vertices[i].x * self.vertices[j].y;
            sum -= self.vertices[j].x * self.vertices[i].y;
        }
        (sum / 2.0).abs()
    }

    /// 获取周长
    pub fn perimeter(&self) -> f64 {
        if self.vertices.len() < 2 {
            return 0.0;
        }

        let mut sum = 0.0;
        let n = self.vertices.len();
        for i in 0..n {
            let j = (i + 1) % n;
            sum += self.vertices[i].distance(&self.vertices[j]);
        }
        sum
    }

    /// 转换为线段列表
    pub fn to_lines(&self) -> Vec<Line> {
        if self.vertices.len() < 2 {
            return vec![];
        }

        let mut lines = Vec::new();
        let n = self.vertices.len();
        for i in 0..n {
            let j = (i + 1) % n;
            lines.push(Line::new(self.vertices[i], self.vertices[j]));
        }
        lines
    }
}

/// 圆
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

impl Circle {
    pub fn new(center: Point, radius: f64) -> Self {
        Self { center, radius }
    }

    pub fn from_coords(center: [f64; 2], radius: f64) -> Self {
        Self {
            center: Point::from_array(center),
            radius,
        }
    }

    pub fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    pub fn circumference(&self) -> f64 {
        2.0 * std::f64::consts::PI * self.radius
    }

    pub fn diameter(&self) -> f64 {
        self.radius * 2.0
    }
}

/// 矩形
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    pub fn from_coords(min: [f64; 2], max: [f64; 2]) -> Self {
        Self {
            min: Point::from_array(min),
            max: Point::from_array(max),
        }
    }

    pub fn from_origin_size(width: f64, height: f64) -> Self {
        Self {
            min: Point::origin(),
            max: Point::new(width, height),
        }
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }

    pub fn center(&self) -> Point {
        Point::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }

    pub fn contains(&self, point: &Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn to_polygon(&self) -> Polygon {
        Polygon::new(vec![
            self.min,
            Point::new(self.max.x, self.min.y),
            self.max,
            Point::new(self.min.x, self.max.y),
        ])
    }
}

/// 统一图元类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Primitive {
    Point(Point),
    Line(Line),
    Polygon(Polygon),
    Circle(Circle),
    Rect(Rect),
    Polyline {
        points: Vec<Point>,
        closed: bool,
    },
    Arc {
        center: Point,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    },
    Text {
        content: String,
        position: Point,
        height: f64,
    },
}

impl Primitive {
    /// 获取图元的包围盒
    pub fn bounding_box(&self) -> Option<Rect> {
        match self {
            Primitive::Point(p) => Some(Rect::new(*p, *p)),
            Primitive::Line(line) => Some(Rect::new(
                Point::new(line.start.x.min(line.end.x), line.start.y.min(line.end.y)),
                Point::new(line.start.x.max(line.end.x), line.start.y.max(line.end.y)),
            )),
            Primitive::Polygon(poly) => {
                if poly.vertices.is_empty() {
                    return None;
                }
                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;
                for v in &poly.vertices {
                    min_x = min_x.min(v.x);
                    min_y = min_y.min(v.y);
                    max_x = max_x.max(v.x);
                    max_y = max_y.max(v.y);
                }
                Some(Rect::new(
                    Point::new(min_x, min_y),
                    Point::new(max_x, max_y),
                ))
            }
            Primitive::Circle(circle) => Some(Rect::new(
                Point::new(
                    circle.center.x - circle.radius,
                    circle.center.y - circle.radius,
                ),
                Point::new(
                    circle.center.x + circle.radius,
                    circle.center.y + circle.radius,
                ),
            )),
            Primitive::Rect(rect) => Some(rect.clone()),
            Primitive::Polyline { points, .. } => {
                if points.is_empty() {
                    return None;
                }
                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;
                for p in points {
                    min_x = min_x.min(p.x);
                    min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x);
                    max_y = max_y.max(p.y);
                }
                Some(Rect::new(
                    Point::new(min_x, min_y),
                    Point::new(max_x, max_y),
                ))
            }
            Primitive::Arc { center, radius, .. } => Some(Rect::new(
                Point::new(center.x - radius, center.y - radius),
                Point::new(center.x + radius, center.y + radius),
            )),
            Primitive::Text { position, .. } => Some(Rect::new(*position, *position)),
        }
    }
}

/// 房间结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub name: String,
    pub boundary: Polygon,
    pub area: f64,
    pub doors: Vec<Door>,
    pub windows: Vec<Window>,
}

/// 门
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Door {
    pub position: Point,
    pub width: f64,
    pub direction: DoorDirection,
}

/// 门的方向
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DoorDirection {
    Inward,
    Outward,
    Sliding,
}

/// 窗户
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub position: Point,
    pub width: f64,
    pub height: f64,
}

/// 墙体
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    pub line: Line,
    pub thickness: f64,
    pub wall_type: WallType,
}

/// 墙体类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallType {
    Exterior,
    Interior,
    LoadBearing,
    Partition,
}
