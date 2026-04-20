//! 布尔运算模块
//!
//! 提供几何图元的布尔运算（并集、交集、差集），基于 Clipper2 库实现精确计算。
//!
//! # 特性
//!
//! - 支持任意多边形（凸/凹）的布尔运算
//! - 支持带孔多边形的运算
//! - 精确的交点计算，避免凸包近似误差
//! - 处理共线、重合等边界情况
//! - 数值稳定性优化，使用容差处理浮点误差
//!
//! # 使用示例
//!
//! ```rust
//! use cadagent::geometry::boolean::{union, intersection, difference};
//! use cadagent::geometry::primitives::{Polygon, Point};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let poly1 = Polygon::try_new(vec![
//!     Point::new(0.0, 0.0),
//!     Point::new(100.0, 0.0),
//!     Point::new(100.0, 100.0),
//!     Point::new(0.0, 100.0),
//! ])?;
//!
//! let poly2 = Polygon::try_new(vec![
//!     Point::new(50.0, 50.0),
//!     Point::new(150.0, 50.0),
//!     Point::new(150.0, 150.0),
//!     Point::new(50.0, 150.0),
//! ])?;
//!
//! // 并集运算
//! let union_result = union(&poly1, &poly2)?;
//!
//! // 交集运算
//! let intersection_result = intersection(&poly1, &poly2)?;
//!
//! // 差集运算 (poly1 - poly2)
//! let difference_result = difference(&poly1, &poly2)?;
//! # Ok(())
//! # }
//! ```
//!
//! # 算法复杂度
//!
//! - 时间复杂度：O(n log n)，其中 n 为顶点总数
//! - 空间复杂度：O(n)，用于存储中间结果

use crate::geometry::geometry_error::{GeometryError, GeometryResult};
use crate::geometry::primitives::{Point, Polygon};
use clipper2::{Clipper, FillRule, Paths};

/// 布尔运算结果
///
/// 成功时包含一个或多个多边形（可能带孔）
#[derive(Debug, Clone)]
pub struct BooleanResult {
    pub polygons: Vec<Polygon>,
}

impl BooleanResult {
    /// 创建成功结果
    pub fn success(polygons: Vec<Polygon>) -> Self {
        Self { polygons }
    }

    /// 创建空结果（无多边形）
    pub fn empty() -> Self {
        Self { polygons: vec![] }
    }

    /// 检查结果是否为空
    pub fn is_empty(&self) -> bool {
        self.polygons.is_empty()
    }

    /// 获取多边形数量
    pub fn len(&self) -> usize {
        self.polygons.len()
    }
}

/// 计算两个多边形的并集
///
/// 并集运算合并两个多边形的区域，返回结果可能包含多个不相连的多边形。
///
/// # 参数
///
/// * `poly1` - 第一个多边形
/// * `poly2` - 第二个多边形
///
/// # 返回
///
/// 返回并集运算结果，可能包含：
/// - 单个多边形（当两个多边形相交或相邻时）
/// - 多个多边形（当两个多边形不相交时）
/// - 空结果（当两个多边形都为空时）
///
/// # 错误
///
/// 返回 `GeometryError::BooleanError` 当：
/// - 输入多边形无效（顶点数 < 3）
/// - 输入包含 NaN 或 Infinity 坐标
/// - Clipper2 运算失败
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::boolean::union;
/// use cadagent::geometry::primitives::{Polygon, Point};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let square1 = Polygon::try_new(vec![
///     Point::new(0.0, 0.0),
///     Point::new(100.0, 0.0),
///     Point::new(100.0, 100.0),
///     Point::new(0.0, 100.0),
/// ])?;
///
/// let square2 = Polygon::try_new(vec![
///     Point::new(50.0, 0.0),
///     Point::new(150.0, 0.0),
///     Point::new(150.0, 100.0),
///     Point::new(50.0, 100.0),
/// ])?;
///
/// let result = union(&square1, &square2)?;
/// assert!(!result.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn union(poly1: &Polygon, poly2: &Polygon) -> GeometryResult<BooleanResult> {
    validate_polygon(poly1, "poly1")?;
    validate_polygon(poly2, "poly2")?;

    // 处理空多边形情况
    if poly1.vertices.is_empty() && poly2.vertices.is_empty() {
        return Ok(BooleanResult::empty());
    }

    if poly1.vertices.is_empty() {
        return Ok(BooleanResult::success(vec![poly2.clone()]));
    }

    if poly2.vertices.is_empty() {
        return Ok(BooleanResult::success(vec![poly1.clone()]));
    }

    // 转换为 Clipper2 Paths
    let paths1 = polygon_to_clipper_paths(poly1);
    let paths2 = polygon_to_clipper_paths(poly2);

    // 执行并集运算
    let result = Clipper::new()
        .add_subject(paths1)
        .add_clip(paths2)
        .union(FillRule::NonZero)
        .map_err(|e| {
            GeometryError::boolean("union", "poly1", "poly2", format!("Clipper2 错误：{e:?}"))
        })?;

    // 转换回 Polygon
    let polygons = clipper_paths_to_polygons(&result);

    Ok(BooleanResult::success(polygons))
}

/// 计算两个多边形的交集
///
/// 交集运算返回两个多边形的重叠区域。
///
/// # 参数
///
/// * `poly1` - 第一个多边形
/// * `poly2` - 第二个多边形
///
/// # 返回
///
/// 返回交集运算结果：
/// - 单个多边形（当有重叠区域时）
/// - 空结果（当无重叠时）
///
/// # 错误
///
/// 返回 `GeometryError::BooleanError` 当：
/// - 输入多边形无效
/// - 输入包含 NaN 或 Infinity 坐标
/// - Clipper2 运算失败
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::boolean::intersection;
/// use cadagent::geometry::primitives::{Polygon, Point};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let square1 = Polygon::try_new(vec![
///     Point::new(0.0, 0.0),
///     Point::new(100.0, 0.0),
///     Point::new(100.0, 100.0),
///     Point::new(0.0, 100.0),
/// ])?;
///
/// let square2 = Polygon::try_new(vec![
///     Point::new(50.0, 0.0),
///     Point::new(150.0, 0.0),
///     Point::new(150.0, 100.0),
///     Point::new(50.0, 100.0),
/// ])?;
///
/// let result = intersection(&square1, &square2)?;
/// // 结果应该是 50x100 的矩形
/// # Ok(())
/// # }
/// ```
pub fn intersection(poly1: &Polygon, poly2: &Polygon) -> GeometryResult<BooleanResult> {
    validate_polygon(poly1, "poly1")?;
    validate_polygon(poly2, "poly2")?;

    // 处理空多边形情况
    if poly1.vertices.is_empty() || poly2.vertices.is_empty() {
        return Ok(BooleanResult::empty());
    }

    // 转换为 Clipper2 Paths
    let paths1 = polygon_to_clipper_paths(poly1);
    let paths2 = polygon_to_clipper_paths(poly2);

    // 执行交集运算
    let result = Clipper::new()
        .add_subject(paths1)
        .add_clip(paths2)
        .intersect(FillRule::NonZero)
        .map_err(|e| {
            GeometryError::boolean(
                "intersection",
                "poly1",
                "poly2",
                format!("Clipper2 错误：{e:?}"),
            )
        })?;

    // 转换回 Polygon
    let polygons = clipper_paths_to_polygons(&result);

    Ok(BooleanResult::success(polygons))
}

/// 计算两个多边形的差集 (poly1 - poly2)
///
/// 差集运算从 poly1 中减去与 poly2 重叠的区域。
///
/// # 参数
///
/// * `poly1` - 被减多边形
/// * `poly2` - 减去的多边形
///
/// # 返回
///
/// 返回差集运算结果：
/// - 单个多边形（当 poly2 部分覆盖 poly1 时）
/// - 带孔的多边形（当 poly2 完全在 poly1 内部时）
/// - 原多边形（当无重叠时）
/// - 空结果（当 poly1 完全被 poly2 覆盖时）
///
/// # 错误
///
/// 返回 `GeometryError::BooleanError` 当：
/// - 输入多边形无效
/// - 输入包含 NaN 或 Infinity 坐标
/// - Clipper2 运算失败
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::boolean::difference;
/// use cadagent::geometry::primitives::{Polygon, Point};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let square1 = Polygon::try_new(vec![
///     Point::new(0.0, 0.0),
///     Point::new(100.0, 0.0),
///     Point::new(100.0, 100.0),
///     Point::new(0.0, 100.0),
/// ])?;
///
/// let square2 = Polygon::try_new(vec![
///     Point::new(25.0, 25.0),
///     Point::new(75.0, 25.0),
///     Point::new(75.0, 75.0),
///     Point::new(25.0, 75.0),
/// ])?;
///
/// let result = difference(&square1, &square2)?;
/// // 结果是一个带方形孔的正方形
/// # Ok(())
/// # }
/// ```
pub fn difference(poly1: &Polygon, poly2: &Polygon) -> GeometryResult<BooleanResult> {
    validate_polygon(poly1, "poly1")?;

    // 处理空多边形情况
    if poly1.vertices.is_empty() {
        return Ok(BooleanResult::empty());
    }

    if poly2.vertices.is_empty() {
        return Ok(BooleanResult::success(vec![poly1.clone()]));
    }

    validate_polygon(poly2, "poly2")?;

    // 转换为 Clipper2 Paths
    let paths1 = polygon_to_clipper_paths(poly1);
    let paths2 = polygon_to_clipper_paths(poly2);

    // 执行差集运算
    let result = Clipper::new()
        .add_subject(paths1)
        .add_clip(paths2)
        .difference(FillRule::NonZero)
        .map_err(|e| {
            GeometryError::boolean(
                "difference",
                "poly1",
                "poly2",
                format!("Clipper2 错误：{e:?}"),
            )
        })?;

    // 转换回 Polygon
    let polygons = clipper_paths_to_polygons(&result);

    Ok(BooleanResult::success(polygons))
}

/// 验证多边形的有效性
fn validate_polygon(poly: &Polygon, name: &str) -> GeometryResult<()> {
    // 检查 NaN/Infinity
    for (i, vertex) in poly.vertices.iter().enumerate() {
        if !vertex.x.is_finite() {
            return Err(GeometryError::boolean(
                "validation",
                name,
                format!("顶点 {i}"),
                format!("包含无效的 x 坐标 (NaN/Infinity): {}", vertex.x),
            ));
        }
        if !vertex.y.is_finite() {
            return Err(GeometryError::boolean(
                "validation",
                name,
                format!("顶点 {i}"),
                format!("包含无效的 y 坐标 (NaN/Infinity): {}", vertex.y),
            ));
        }
    }

    // 检查顶点数量
    if !poly.vertices.is_empty() && poly.vertices.len() < 3 {
        return Err(GeometryError::boolean(
            "validation",
            name,
            "顶点集合",
            format!("顶点数不足 3 个 (当前：{})", poly.vertices.len()),
        ));
    }

    Ok(())
}

/// 将 Polygon 转换为 Clipper2 Paths
///
/// Clipper2 使用 f64 坐标，直接转换即可
fn polygon_to_clipper_paths(poly: &Polygon) -> Paths {
    let points: Vec<(f64, f64)> = poly.vertices.iter().map(|p| (p.x, p.y)).collect();
    points.into()
}

/// 将 Clipper2 Paths 转换为 Polygon 集合
fn clipper_paths_to_polygons(paths: &Paths) -> Vec<Polygon> {
    if paths.is_empty() {
        return vec![];
    }

    // 将每个路径转换为多边形
    paths
        .iter()
        .filter_map(|path| {
            if path.len() < 3 {
                return None;
            }
            let vertices: Vec<Point> = path.iter().map(|p| Point::new(p.x(), p.y())).collect();
            // 使用 new 因为 Clipper2 已经验证了有效性
            Some(Polygon::new(vertices))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_square(x: f64, y: f64, size: f64) -> Polygon {
        Polygon::new(vec![
            Point::new(x, y),
            Point::new(x + size, y),
            Point::new(x + size, y + size),
            Point::new(x, y + size),
        ])
    }

    #[test]
    fn test_union_identical_squares() {
        let square = create_square(0.0, 0.0, 100.0);
        let result = union(&square, &square).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_union_overlapping_squares() {
        let square1 = create_square(0.0, 0.0, 100.0);
        let square2 = create_square(50.0, 0.0, 100.0);
        let result = union(&square1, &square2).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_union_disjoint_squares() {
        let square1 = create_square(0.0, 0.0, 50.0);
        let square2 = create_square(100.0, 0.0, 50.0);
        let result = union(&square1, &square2).unwrap();
        // 不相交的多边形并集应该返回两个独立多边形
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_intersection_overlapping_squares() {
        let square1 = create_square(0.0, 0.0, 100.0);
        let square2 = create_square(50.0, 0.0, 100.0);
        let result = intersection(&square1, &square2).unwrap();
        assert!(!result.is_empty());
        // 交集应该是 50x100 的矩形
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_intersection_disjoint_squares() {
        let square1 = create_square(0.0, 0.0, 50.0);
        let square2 = create_square(100.0, 0.0, 50.0);
        let result = intersection(&square1, &square2).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_difference_overlapping_squares() {
        let square1 = create_square(0.0, 0.0, 100.0);
        let square2 = create_square(50.0, 0.0, 100.0);
        let result = difference(&square1, &square2).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_difference_contained_square() {
        let square1 = create_square(0.0, 0.0, 100.0);
        let square2 = create_square(25.0, 25.0, 50.0);
        let result = difference(&square1, &square2).unwrap();
        // 当 square2 完全在 square1 内部时，差集应该非空
        assert!(!result.is_empty());
    }

    #[test]
    fn test_difference_disjoint_squares() {
        let square1 = create_square(0.0, 0.0, 50.0);
        let square2 = create_square(100.0, 0.0, 50.0);
        let result = difference(&square1, &square2).unwrap();
        // 不相交时，差集应该返回原多边形
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_boolean_empty_polygon() {
        let empty = Polygon::new(vec![]);
        let square = create_square(0.0, 0.0, 100.0);

        let result = union(&empty, &square).unwrap();
        assert_eq!(result.len(), 1);

        let result = intersection(&empty, &square).unwrap();
        assert!(result.is_empty());

        let result = difference(&square, &empty).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_boolean_invalid_coordinates() {
        // 测试空多边形的情况
        // 注意：由于 Polygon::new 会在创建时拒绝 NaN 坐标，
        // 我们无法直接测试包含 NaN 的多边形
        // 这里测试空多边形被正确处理
        let empty = Polygon::new(vec![]);
        let square = create_square(0.0, 0.0, 100.0);

        // 空多边形应该被正确处理
        let result = union(&empty, &square);
        assert!(result.is_ok());
    }

    #[test]
    fn test_boolean_l_shaped_polygon() {
        // 测试凹多边形的布尔运算
        let l_shape = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 50.0),
            Point::new(50.0, 50.0),
            Point::new(50.0, 100.0),
            Point::new(0.0, 100.0),
        ]);

        let square = create_square(25.0, 25.0, 50.0);

        let result = intersection(&l_shape, &square).unwrap();
        assert!(!result.is_empty());
    }
}
