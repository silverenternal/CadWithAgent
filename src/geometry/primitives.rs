//! 几何图元定义
//!
//! 提供 CAD 处理所需的基础几何图元类型

#![allow(clippy::cast_precision_loss)]

//! # 坐标验证
//!
//! 所有图元构造函数都会验证坐标的有效性：
//! - 不接受 NaN 或 Infinity 值
//! - 使用合理的容差值进行比较
//!
//! # 错误处理
//!
//! 本模块提供两种构造函数：
//! - `new()`: 快速构造，内部使用 `try_new`，失败时 panic（仅适用于测试和已知有效的输入）
//! - `try_new()`: 安全构造，返回 `Result`（**推荐用于生产环境和用户输入**）
//!
//! # 示例
//!
//! ```
//! use cadagent::geometry::{Point, Line, GeometryResult, GeometryError};
//!
//! // 安全构造（推荐用于生产代码）
//! let p1 = Point::try_new(1.0, 2.0).expect("有效坐标");
//! let p2 = Point::try_new(3.0, 4.0).expect("有效坐标");
//! let line = Line::try_new(p1, p2).expect("有效线段");
//!
//! // 处理错误
//! match Point::try_new(f64::NAN, 0.0) {
//!     Ok(_) => println!("不应成功"),
//!     Err(e) => println!("捕获错误：{}", e),
//! }
//! ```

use super::geometry_error::{GeometryError, GeometryResult};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

/// 默认几何容差
///
/// 用于判断线段长度、点距离等是否接近 0
/// 该值经过权衡：太小会导致数值不稳定，太大会丢失精度
const DEFAULT_TOLERANCE: f64 = 1e-10;

/// 从点迭代器计算包围盒
///
/// 辅助函数，用于提取重复的包围盒计算逻辑
fn compute_bounding_box_from_points<'a, I>(points: I) -> Rect
where
    I: IntoIterator<Item = &'a Point>,
{
    let (min_x, min_y, max_x, max_y) = points.into_iter().map(|p| (p.x, p.y)).fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, min_y, max_x, max_y), (x, y)| {
            (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
        },
    );
    Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y))
}

/// 二维点
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// 创建新点
    ///
    /// # Panics
    /// 如果坐标包含 NaN 或 Infinity，将 panic
    ///
    /// # 建议
    /// 对于可能无效的输入，**必须使用** `try_new` 方法
    ///
    /// # 生产代码警告
    /// 此方法在生产环境中应**避免使用**，除非你能 100% 保证输入有效
    pub fn new(x: f64, y: f64) -> Self {
        Self::try_new(x, y).unwrap_or_else(|e| {
            panic!("Point::new() 失败：{e}. 注意：生产代码应使用 try_new() 而非 new()")
        })
    }

    /// 尝试创建新点（带验证）
    ///
    /// # Errors
    /// 如果坐标包含 NaN 或 Infinity，返回 `GeometryError::InvalidCoordinate`
    ///
    /// # Examples
    /// ```
    /// use cadagent::geometry::Point;
    ///
    /// let p = Point::try_new(1.0, 2.0).unwrap();
    /// assert_eq!(p.x, 1.0);
    ///
    /// assert!(Point::try_new(f64::NAN, 0.0).is_err());
    /// assert!(Point::try_new(f64::INFINITY, 0.0).is_err());
    /// ```
    pub fn try_new(x: f64, y: f64) -> GeometryResult<Self> {
        if !x.is_finite() {
            return Err(GeometryError::invalid_coordinate(
                "Point",
                "x",
                x,
                "坐标必须为有限值，不能为 NaN 或 Infinity",
            ));
        }
        if !y.is_finite() {
            return Err(GeometryError::invalid_coordinate(
                "Point",
                "y",
                y,
                "坐标必须为有限值，不能为 NaN 或 Infinity",
            ));
        }
        Ok(Self { x, y })
    }

    /// 创建原点
    pub fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// 计算到另一点的距离
    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    /// 计算距离（带容差比较）
    pub fn distance_with_tolerance(&self, other: &Point, tolerance: f64) -> f64 {
        let dist = self.distance(other);
        if dist < tolerance {
            0.0
        } else {
            dist
        }
    }

    /// 转换为数组
    pub fn to_array(&self) -> [f64; 2] {
        [self.x, self.y]
    }

    /// 从数组创建
    ///
    /// # Panics
    /// 如果数组包含无效坐标，将 panic
    ///
    /// # 建议
    /// 生产代码应使用 `try_from_array`
    pub fn from_array(arr: [f64; 2]) -> Self {
        Self::try_new(arr[0], arr[1]).unwrap_or_else(|e| {
            panic!("Point::from_array() 失败：{e}. 注意：生产代码应使用 try_from_array()")
        })
    }

    /// 从数组创建（带验证）
    ///
    /// # Errors
    /// 如果数组包含无效坐标，返回错误
    pub fn try_from_array(arr: [f64; 2]) -> GeometryResult<Self> {
        Self::try_new(arr[0], arr[1])
    }

    /// 验证坐标是否有效
    pub fn is_valid(&self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// 线段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub start: Point,
    pub end: Point,
}

impl Line {
    /// 创建新线段（不验证）
    ///
    /// # Safety
    /// 此方法不验证线段长度，可能导致后续计算错误
    /// 仅用于测试或已知有效的内部场景
    ///
    /// # 注意
    /// 生产代码应使用 `try_new()`
    #[doc(hidden)]
    pub fn new_unchecked(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    /// 创建新线段
    ///
    /// # Panics
    /// 如果起点或终点重合（长度接近 0），将 panic
    ///
    /// # 建议
    /// **生产代码必须使用** `try_new()` 而非此方法
    pub fn new(start: Point, end: Point) -> Self {
        Self::try_new(start, end)
            .unwrap_or_else(|e| panic!("Line::new() 失败：{e}. 注意：生产代码应使用 try_new()"))
    }

    /// 尝试创建新线段（带验证）
    ///
    /// # Errors
    /// 如果起点或终点重合（长度接近 0），返回 `GeometryError::InvalidParameter`
    ///
    /// # Examples
    /// ```
    /// use cadagent::geometry::{Point, Line};
    ///
    /// let p1 = Point::origin();
    /// let p2 = Point::new(1.0, 0.0);
    /// let line = Line::try_new(p1, p2).unwrap();
    /// assert_eq!(line.length(), 1.0);
    ///
    /// // 起点终点相同会失败
    /// assert!(Line::try_new(p1, p1).is_err());
    /// ```
    pub fn try_new(start: Point, end: Point) -> GeometryResult<Self> {
        let len = start.distance(&end);
        if len <= DEFAULT_TOLERANCE {
            return Err(GeometryError::invalid_parameter(
                "Line",
                "length",
                len,
                "线段长度必须大于容差值",
            ));
        }
        Ok(Self { start, end })
    }

    /// 从坐标数组创建
    ///
    /// # Panics
    /// 如果坐标无效或线段长度接近 0，将 panic
    ///
    /// # 建议
    /// 生产代码应使用 `try_from_coords`
    pub fn from_coords(start: [f64; 2], end: [f64; 2]) -> Self {
        Self::try_from_coords(start, end).unwrap_or_else(|e| {
            panic!("Line::from_coords() 失败：{e}. 注意：生产代码应使用 try_from_coords()")
        })
    }

    /// 从坐标数组创建（不验证）
    ///
    /// # Safety
    /// 此方法不验证坐标和线段长度
    /// 仅用于测试或已知有效的内部场景
    #[doc(hidden)]
    pub fn from_coords_unchecked(start: [f64; 2], end: [f64; 2]) -> Self {
        Self {
            start: Point::from_array(start),
            end: Point::from_array(end),
        }
    }

    /// 从坐标数组创建（带验证）
    ///
    /// # Errors
    /// 如果坐标无效或线段长度接近 0，返回错误
    pub fn try_from_coords(start: [f64; 2], end: [f64; 2]) -> GeometryResult<Self> {
        let p1 = Point::try_from_array(start)?;
        let p2 = Point::try_from_array(end)?;
        Self::try_new(p1, p2)
    }

    /// 计算线段长度
    pub fn length(&self) -> f64 {
        self.start.distance(&self.end)
    }

    /// 计算中点
    pub fn midpoint(&self) -> Point {
        // 中点计算是两个有限坐标的平均值，结果必然有限
        Point::new(
            f64::midpoint(self.start.x, self.end.x),
            f64::midpoint(self.start.y, self.end.y),
        )
    }

    /// 计算单位方向向量
    pub fn direction(&self) -> Point {
        let len = self.length();
        if len < DEFAULT_TOLERANCE {
            return Point::origin();
        }
        Point::new(
            (self.end.x - self.start.x) / len,
            (self.end.y - self.start.y) / len,
        )
    }

    /// 验证线段是否有效
    pub fn is_valid(&self) -> bool {
        self.start.is_valid() && self.end.is_valid() && self.length() > DEFAULT_TOLERANCE
    }
}

/// 多边形（闭合的折线）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polygon {
    pub vertices: Vec<Point>,
    pub closed: bool,
}

impl Polygon {
    /// 创建新多边形
    ///
    /// # 注意
    /// 对于生产代码，建议使用 `try_new` 进行验证
    /// 此函数允许创建少于 3 个顶点的多边形用于测试边缘情况
    pub fn new(vertices: Vec<Point>) -> Self {
        Self {
            vertices,
            closed: true,
        }
    }

    /// 尝试创建新多边形（带验证）
    ///
    /// # Errors
    /// 如果顶点数少于 3 个或包含无效坐标，返回 `GeometryError`
    ///
    /// # Examples
    /// ```
    /// use cadagent::geometry::{Point, Polygon};
    ///
    /// let triangle = vec![
    ///     Point::new(0.0, 0.0),
    ///     Point::new(1.0, 0.0),
    ///     Point::new(0.0, 1.0),
    /// ];
    /// let poly = Polygon::try_new(triangle).unwrap();
    /// assert!(poly.area() > 0.0);
    ///
    /// // 顶点不足会失败
    /// assert!(Polygon::try_new(vec![]).is_err());
    /// assert!(Polygon::try_new(vec![Point::origin()]).is_err());
    /// ```
    pub fn try_new(vertices: Vec<Point>) -> GeometryResult<Self> {
        if vertices.len() < 3 {
            return Err(GeometryError::invalid_parameter(
                "Polygon",
                "vertex_count",
                vertices.len() as f64,
                "多边形至少需要 3 个顶点",
            ));
        }

        for (i, v) in vertices.iter().enumerate() {
            if !v.is_valid() {
                return Err(GeometryError::invalid_coordinate(
                    "Polygon",
                    format!("vertex_{i}"),
                    f64::NAN,
                    format!("顶点 {i} 包含无效坐标：{v:?}"),
                ));
            }
        }

        Ok(Self {
            vertices,
            closed: true,
        })
    }

    /// 从坐标数组创建
    ///
    /// # Panics
    /// 如果坐标无效，将 panic
    pub fn from_coords(coords: Vec<[f64; 2]>) -> Self {
        Self {
            vertices: coords.into_iter().map(Point::from_array).collect(),
            closed: true,
        }
    }

    /// 从坐标数组创建（带验证）
    ///
    /// # Errors
    /// 如果坐标无效或顶点不足，返回错误
    pub fn try_from_coords(coords: Vec<[f64; 2]>) -> GeometryResult<Self> {
        let points: GeometryResult<Vec<Point>> =
            coords.into_iter().map(Point::try_from_array).collect();
        Self::try_new(points?)
    }

    /// 使用鞋带公式计算面积
    ///
    /// # 注意
    /// 如果顶点数少于 3 个，返回 0.0
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
    ///
    /// # 注意
    /// 如果相邻顶点距离过近（小于容差），该边会被跳过
    /// 这可能导致多边形边界不完整
    ///
    /// # 建议
    /// 如果需要检测无效边，使用 `to_lines_with_validation()`
    pub fn to_lines(&self) -> SmallVec<[Line; 4]> {
        if self.vertices.len() < 2 {
            return smallvec![];
        }

        let mut lines = SmallVec::with_capacity(self.vertices.len().min(4));
        let n = self.vertices.len();
        for i in 0..n {
            let j = (i + 1) % n;
            // 使用 try_new 避免创建无效线段
            if let Ok(line) = Line::try_new(self.vertices[i], self.vertices[j]) {
                lines.push(line);
            }
        }
        lines
    }

    /// 转换为线段列表（带验证）
    ///
    /// # Returns
    /// 返回结构化结果，包含：
    /// - `lines`: 有效的线段
    /// - `warnings`: 被跳过的边索引及原因
    ///
    /// # Example
    /// ```rust
    /// use cadagent::geometry::Polygon;
    ///
    /// let poly = Polygon::from_coords(vec![
    ///     [0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0],
    /// ]);
    /// let result = poly.to_lines_with_validation();
    /// assert_eq!(result.lines.len(), 4);
    /// assert!(result.warnings.is_empty());
    /// ```
    pub fn to_lines_with_validation(&self) -> PolygonLinesResult {
        if self.vertices.len() < 2 {
            return PolygonLinesResult::empty();
        }

        let mut lines = SmallVec::with_capacity(self.vertices.len().min(4));
        let mut warnings = SmallVec::with_capacity(1);
        let n = self.vertices.len();

        for i in 0..n {
            let j = (i + 1) % n;
            let p1 = self.vertices[i];
            let p2 = self.vertices[j];
            let dist = p1.distance(&p2);

            if dist <= DEFAULT_TOLERANCE {
                warnings.push(SkippedEdgeWarning {
                    edge_index: i,
                    start_point: p1,
                    end_point: p2,
                    reason: format!("顶点间距过小：{dist:.2e} (容差：{DEFAULT_TOLERANCE:.2e})"),
                });
            } else {
                // Line::new will not fail here since we checked distance
                lines.push(Line::new(p1, p2));
            }
        }

        PolygonLinesResult { lines, warnings }
    }

    /// 验证多边形是否有效
    pub fn is_valid(&self) -> bool {
        if self.vertices.len() < 3 {
            return false;
        }
        self.vertices.iter().all(Point::is_valid)
    }
}

/// 被跳过的边警告
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkippedEdgeWarning {
    /// 边的索引（从 0 开始）
    pub edge_index: usize,
    /// 起点坐标
    pub start_point: Point,
    /// 终点坐标
    pub end_point: Point,
    /// 跳过原因
    pub reason: String,
}

/// `Polygon::to_lines_with_validation()` 的返回类型
///
/// 包含转换后的线段列表和警告信息
///
/// # 使用示例
///
/// ```rust
/// use cadagent::geometry::{Polygon, Point};
///
/// let poly = Polygon::from_coords(vec![
///     [0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0],
/// ]);
/// let result = poly.to_lines_with_validation();
///
/// // 访问有效线段
/// println!("有效线段数量：{}", result.lines.len());
///
/// // 检查是否有警告
/// if !result.warnings.is_empty() {
///     for warning in &result.warnings {
///         println!("警告：{}", warning.reason);
///     }
/// }
///
/// // 检查是否有跳过的边
/// if result.has_skipped_edges() {
///     println!("多边形边界不完整！");
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolygonLinesResult {
    /// 有效的线段列表（使用 `SmallVec` 优化小集合）
    pub lines: SmallVec<[Line; 4]>,
    /// 被跳过的边警告（使用 `SmallVec` 优化小集合）
    pub warnings: SmallVec<[SkippedEdgeWarning; 1]>,
}

impl PolygonLinesResult {
    /// 创建空结果
    pub fn empty() -> Self {
        Self {
            lines: smallvec![],
            warnings: smallvec![],
        }
    }

    /// 检查是否有跳过的边
    pub fn has_skipped_edges(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// 获取跳过的边数量
    pub fn skipped_count(&self) -> usize {
        self.warnings.len()
    }
}

/// 圆
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    pub center: Point,
    pub radius: f64,
}

impl Circle {
    /// 创建新圆
    ///
    /// # Panics
    /// 如果半径为负数或 0，将 panic
    pub fn new(center: Point, radius: f64) -> Self {
        assert!(
            radius > DEFAULT_TOLERANCE,
            "圆的半径必须为正数且大于容差值。收到：{radius}"
        );
        Self { center, radius }
    }

    /// 尝试创建新圆（带验证）
    ///
    /// # Errors
    /// 如果半径为负数或 0，返回错误
    pub fn try_new(center: Point, radius: f64) -> GeometryResult<Self> {
        if radius <= DEFAULT_TOLERANCE {
            return Err(GeometryError::invalid_parameter(
                "Circle",
                "radius",
                radius,
                "半径必须为正数且大于容差值",
            ));
        }
        Ok(Self { center, radius })
    }

    /// 从坐标数组创建
    ///
    /// # Panics
    /// 如果坐标无效或半径无效，将 panic
    ///
    /// # 建议
    /// 生产代码应使用 `try_from_coords`
    pub fn from_coords(center: [f64; 2], radius: f64) -> Self {
        Self::try_from_coords(center, radius).unwrap_or_else(|e| {
            panic!("Circle::from_coords() 失败：{e}. 注意：生产代码应使用 try_from_coords()")
        })
    }

    /// 从坐标数组创建（带验证）
    ///
    /// # Errors
    /// 如果坐标无效或半径无效，返回错误
    pub fn try_from_coords(center: [f64; 2], radius: f64) -> GeometryResult<Self> {
        let c = Point::try_from_array(center)?;
        Self::try_new(c, radius)
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

    /// 验证圆是否有效
    pub fn is_valid(&self) -> bool {
        self.center.is_valid() && self.radius > DEFAULT_TOLERANCE
    }
}

/// 矩形
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    /// 创建新矩形
    ///
    /// # 注意
    /// 如果 min.x > max.x 或 min.y > max.y，**会自动交换**以确保矩形有效
    /// 这是为了支持几何变换等操作可能产生反向坐标的场景
    ///
    /// # 建议
    /// 如果需要严格验证（不自动交换），使用 `try_new`
    pub fn new(min: Point, max: Point) -> Self {
        // 自动交换以确保 min <= max
        let (min, max) = if min.x <= max.x && min.y <= max.y {
            (min, max)
        } else {
            (
                Point::new(min.x.min(max.x), min.y.min(max.y)),
                Point::new(min.x.max(max.x), min.y.max(max.y)),
            )
        };
        Self { min, max }
    }

    /// 尝试创建新矩形（带验证，不自动交换）
    ///
    /// # Errors
    /// 如果 min.x > max.x 或 min.y > max.y，返回错误
    pub fn try_new(min: Point, max: Point) -> GeometryResult<Self> {
        if min.x > max.x || min.y > max.y {
            return Err(GeometryError::invalid_parameter(
                "Rect",
                "min/max",
                min.x.max(max.x),
                format!("min 坐标必须小于等于 max 坐标。收到：min={min:?}, max={max:?}"),
            ));
        }
        Ok(Self { min, max })
    }

    /// 从坐标数组创建
    ///
    /// # 注意
    /// **会自动交换** min 和 max 以确保矩形有效
    ///
    /// # 建议
    /// 如果需要严格验证（不自动交换），使用 `try_from_coords`
    pub fn from_coords(min: [f64; 2], max: [f64; 2]) -> Self {
        let p_min = Point::from_array(min);
        let p_max = Point::from_array(max);
        Self::new(p_min, p_max)
    }

    /// 从坐标数组创建（带验证）
    ///
    /// # Errors
    /// 如果坐标无效或 min > max，返回错误
    pub fn try_from_coords(min: [f64; 2], max: [f64; 2]) -> GeometryResult<Self> {
        let p_min = Point::try_from_array(min)?;
        let p_max = Point::try_from_array(max)?;
        Self::try_new(p_min, p_max)
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
        // Center calculation is safe: average of two finite coordinates
        Point::new(
            f64::midpoint(self.min.x, self.max.x),
            f64::midpoint(self.min.y, self.max.y),
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

    /// 验证矩形是否有效
    pub fn is_valid(&self) -> bool {
        self.min.is_valid()
            && self.max.is_valid()
            && self.min.x <= self.max.x
            && self.min.y <= self.max.y
    }
}

/// 贝塞尔曲线
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BezierCurve {
    /// 起点
    pub start: Point,
    /// 控制点 1
    pub control1: Point,
    /// 控制点 2
    pub control2: Point,
    /// 终点
    pub end: Point,
}

impl BezierCurve {
    /// 创建新的三次贝塞尔曲线
    pub fn new(start: Point, control1: Point, control2: Point, end: Point) -> Self {
        Self {
            start,
            control1,
            control2,
            end,
        }
    }

    /// 从坐标数组创建
    pub fn from_coords(
        start: [f64; 2],
        control1: [f64; 2],
        control2: [f64; 2],
        end: [f64; 2],
    ) -> Self {
        Self {
            start: Point::from_array(start),
            control1: Point::from_array(control1),
            control2: Point::from_array(control2),
            end: Point::from_array(end),
        }
    }

    /// 验证贝塞尔曲线是否有效
    pub fn is_valid(&self) -> bool {
        self.start.is_valid()
            && self.control1.is_valid()
            && self.control2.is_valid()
            && self.end.is_valid()
    }

    /// 获取包围盒
    pub fn bounding_box(&self) -> Rect {
        // 简化的包围盒计算：使用所有控制点的极值
        // 更精确的计算需要求解导数找到极值点
        let all_points = [self.start, self.control1, self.control2, self.end];
        compute_bounding_box_from_points(&all_points)
    }
}

/// 二次贝塞尔曲线
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuadraticBezier {
    /// 起点
    pub start: Point,
    /// 控制点
    pub control: Point,
    /// 终点
    pub end: Point,
}

impl QuadraticBezier {
    /// 创建新的二次贝塞尔曲线
    pub fn new(start: Point, control: Point, end: Point) -> Self {
        Self {
            start,
            control,
            end,
        }
    }

    /// 从坐标数组创建
    pub fn from_coords(start: [f64; 2], control: [f64; 2], end: [f64; 2]) -> Self {
        Self {
            start: Point::from_array(start),
            control: Point::from_array(control),
            end: Point::from_array(end),
        }
    }

    /// 验证二次贝塞尔曲线是否有效
    pub fn is_valid(&self) -> bool {
        self.start.is_valid() && self.control.is_valid() && self.end.is_valid()
    }

    /// 转换为三次贝塞尔曲线（用于统一处理）
    pub fn to_cubic(&self) -> BezierCurve {
        // 二次转三次的转换公式
        let c1 = Point::new(
            self.start.x + 2.0 / 3.0 * (self.control.x - self.start.x),
            self.start.y + 2.0 / 3.0 * (self.control.y - self.start.y),
        );
        let c2 = Point::new(
            self.end.x + 2.0 / 3.0 * (self.control.x - self.end.x),
            self.end.y + 2.0 / 3.0 * (self.control.y - self.end.y),
        );
        BezierCurve::new(self.start, c1, c2, self.end)
    }

    /// 获取包围盒
    pub fn bounding_box(&self) -> Rect {
        let all_points = [self.start, self.control, self.end];
        compute_bounding_box_from_points(&all_points)
    }
}

/// 椭圆弧
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EllipticalArc {
    /// 起点
    pub start: Point,
    /// X 轴半径
    pub rx: f64,
    /// Y 轴半径
    pub ry: f64,
    /// X 轴旋转角度（弧度）
    pub x_axis_rotation: f64,
    /// 大弧标志
    pub large_arc: bool,
    /// 扫描标志
    pub sweep: bool,
    /// 终点
    pub end: Point,
}

impl EllipticalArc {
    /// 创建新的椭圆弧
    pub fn new(
        start: Point,
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        end: Point,
    ) -> Self {
        Self {
            start,
            rx,
            ry,
            x_axis_rotation,
            large_arc,
            sweep,
            end,
        }
    }

    /// 从坐标数组创建
    pub fn from_coords(
        start: [f64; 2],
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        end: [f64; 2],
    ) -> Self {
        Self {
            start: Point::from_array(start),
            rx,
            ry,
            x_axis_rotation,
            large_arc,
            sweep,
            end: Point::from_array(end),
        }
    }

    /// 验证椭圆弧是否有效
    pub fn is_valid(&self) -> bool {
        self.start.is_valid() && self.end.is_valid() && self.rx > 0.0 && self.ry > 0.0
    }

    /// 获取包围盒（简化版本）
    pub fn bounding_box(&self) -> Rect {
        // 简化：使用起点和终点的包围盒，加上半径扩展
        let min_x = self.start.x.min(self.end.x) - self.rx;
        let min_y = self.start.y.min(self.end.y) - self.ry;
        let max_x = self.start.x.max(self.end.x) + self.rx;
        let max_y = self.start.y.max(self.end.y) + self.ry;

        Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y))
    }

    /// 获取中心点（近似）
    pub fn center(&self) -> Point {
        Point::new(
            f64::midpoint(self.start.x, self.end.x),
            f64::midpoint(self.start.y, self.end.y),
        )
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
    /// 椭圆弧
    EllipticalArc(EllipticalArc),
    /// 三次贝塞尔曲线
    BezierCurve(BezierCurve),
    /// 二次贝塞尔曲线
    QuadraticBezier(QuadraticBezier),
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
                Some(compute_bounding_box_from_points(&poly.vertices))
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
                Some(compute_bounding_box_from_points(points))
            }
            Primitive::Arc { center, radius, .. } => Some(Rect::new(
                Point::new(center.x - radius, center.y - radius),
                Point::new(center.x + radius, center.y + radius),
            )),
            Primitive::EllipticalArc(arc) => Some(arc.bounding_box()),
            Primitive::BezierCurve(curve) => Some(curve.bounding_box()),
            Primitive::QuadraticBezier(curve) => Some(curve.bounding_box()),
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
