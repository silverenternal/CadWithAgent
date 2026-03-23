//! 布尔运算模块
//!
//! 提供几何图元的布尔运算（并集、交集、差集）
//!
//! 注意：完整的布尔运算需要复杂的几何算法，
//! 这里提供基础框架，后续可集成 geo-bool 等库

use crate::geometry::{Polygon, Point};

/// 布尔运算结果
#[derive(Debug, Clone)]
pub struct BooleanResult {
    pub polygons: Vec<Polygon>,
    pub success: bool,
    pub error: Option<String>,
}

impl BooleanResult {
    pub fn success(polygons: Vec<Polygon>) -> Self {
        Self {
            polygons,
            success: true,
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            polygons: vec![],
            success: false,
            error: Some(msg.into()),
        }
    }
}

/// 布尔运算类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Difference,
}

/// 计算两个多边形的并集
///
/// 注意：这是简化实现，适用于不相交或简单相交的多边形
/// 完整的布尔运算建议使用 Clipper2 或 geo-bool 库
pub fn union(poly1: &Polygon, poly2: &Polygon) -> BooleanResult {
    // 如果两个多边形不相交，直接返回两个多边形
    if !polygons_intersect(poly1, poly2) {
        return BooleanResult::success(vec![poly1.clone(), poly2.clone()]);
    }

    // 简化实现：返回包含两个多边形顶点凸包
    // 注意：这不是精确的并集，仅用于演示
    let mut all_vertices = poly1.vertices.clone();
    all_vertices.extend(poly2.vertices.iter().cloned());

    // 简单的凸包算法（Graham 扫描）
    let hull = convex_hull(&all_vertices);
    BooleanResult::success(vec![Polygon::new(hull)])
}

/// 计算两个多边形的交集
///
/// 注意：这是简化实现
pub fn intersection(poly1: &Polygon, poly2: &Polygon) -> BooleanResult {
    // 如果不相交，返回空
    if !polygons_intersect(poly1, poly2) {
        return BooleanResult::success(vec![]);
    }

    // 简化实现：收集在多边形 2 内部的 poly1 顶点
    let mut intersect_vertices = Vec::new();

    for vertex in &poly1.vertices {
        if point_in_polygon(vertex, poly2) {
            intersect_vertices.push(*vertex);
        }
    }

    for vertex in &poly2.vertices {
        if point_in_polygon(vertex, poly1) && !intersect_vertices.contains(vertex) {
            intersect_vertices.push(*vertex);
        }
    }

    // 添加边的交点
    for l1 in &poly1.to_lines() {
        for l2 in &poly2.to_lines() {
            if let Some(pt) = line_intersection(l1, l2) {
                if !intersect_vertices.contains(&pt) {
                    intersect_vertices.push(pt);
                }
            }
        }
    }

    if intersect_vertices.is_empty() {
        return BooleanResult::success(vec![]);
    }

    // 计算凸包作为交集结果
    let hull = convex_hull(&intersect_vertices);
    if hull.len() >= 3 {
        BooleanResult::success(vec![Polygon::new(hull)])
    } else {
        BooleanResult::error("Intersection result has fewer than 3 vertices")
    }
}

/// 计算两个多边形的差集 (poly1 - poly2)
///
/// 注意：这是简化实现
pub fn difference(poly1: &Polygon, poly2: &Polygon) -> BooleanResult {
    // 如果不相交，直接返回 poly1
    if !polygons_intersect(poly1, poly2) {
        return BooleanResult::success(vec![poly1.clone()]);
    }

    // 简化实现：收集在 poly2 外部的 poly1 顶点
    let mut diff_vertices = Vec::new();

    for vertex in &poly1.vertices {
        if !point_in_polygon(vertex, poly2) {
            diff_vertices.push(*vertex);
        }
    }

    // 添加边的交点
    for l1 in &poly1.to_lines() {
        for l2 in &poly2.to_lines() {
            if let Some(pt) = line_intersection(l1, l2) {
                if !diff_vertices.contains(&pt) {
                    diff_vertices.push(pt);
                }
            }
        }
    }

    if diff_vertices.is_empty() {
        return BooleanResult::success(vec![]);
    }

    // 计算凸包
    let hull = convex_hull(&diff_vertices);
    if hull.len() >= 3 {
        BooleanResult::success(vec![Polygon::new(hull)])
    } else {
        BooleanResult::error("Difference result has fewer than 3 vertices")
    }
}

/// 计算点集的凸包（Graham 扫描算法）
fn convex_hull(points: &[Point]) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut points: Vec<Point> = points.to_vec();

    // 找到最左下角的点
    let mut min_idx = 0;
    for i in 1..points.len() {
        if points[i].y < points[min_idx].y
            || (points[i].y == points[min_idx].y && points[i].x < points[min_idx].x)
        {
            min_idx = i;
        }
    }
    points.swap(0, min_idx);
    let pivot = points[0];

    // 按极角排序
    points[1..].sort_by(|a, b| {
        let cross = cross_product(pivot, *a, *b);
        if cross == 0.0 {
            // 极角相同，按距离排序
            let dist_a = (a.x - pivot.x).powi(2) + (a.y - pivot.y).powi(2);
            let dist_b = (b.x - pivot.x).powi(2) + (b.y - pivot.y).powi(2);
            dist_a.partial_cmp(&dist_b).unwrap()
        } else if cross > 0.0 {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    // Graham 扫描
    let mut hull = vec![points[0], points[1], points[2]];

    for point in points.iter().skip(3) {
        while hull.len() > 1 {
            let top = hull[hull.len() - 1];
            let next_top = hull[hull.len() - 2];
            if cross_product(next_top, top, *point) > 0.0 {
                break;
            }
            hull.pop();
        }
        hull.push(*point);
    }

    hull
}

/// 计算叉积 (B - A) × (C - A)
fn cross_product(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

/// 检查点是否在多边形内（射线法）
pub fn point_in_polygon(point: &Point, polygon: &Polygon) -> bool {
    let mut inside = false;
    let n = polygon.vertices.len();

    if n < 3 {
        return false;
    }

    let mut j = n - 1;
    for i in 0..n {
        let vi = &polygon.vertices[i];
        let vj = &polygon.vertices[j];

        if ((vi.y > point.y) != (vj.y > point.y))
            && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
        {
            inside = !inside;
        }
        j = i;
    }

    inside
}

/// 检查两个多边形是否相交
pub fn polygons_intersect(poly1: &Polygon, poly2: &Polygon) -> bool {
    // 检查 poly1 的顶点是否在 poly2 内
    for vertex in &poly1.vertices {
        if point_in_polygon(vertex, poly2) {
            return true;
        }
    }

    // 检查 poly2 的顶点是否在 poly1 内
    for vertex in &poly2.vertices {
        if point_in_polygon(vertex, poly1) {
            return true;
        }
    }

    // 检查边是否相交
    let lines1 = poly1.to_lines();
    let lines2 = poly2.to_lines();

    for l1 in &lines1 {
        for l2 in &lines2 {
            if lines_intersect(l1, l2) {
                return true;
            }
        }
    }

    false
}

/// 检查两条线段是否相交
pub fn lines_intersect(line1: &crate::geometry::Line, line2: &crate::geometry::Line) -> bool {
    let p1 = line1.start;
    let q1 = line1.end;
    let p2 = line2.start;
    let q2 = line2.end;

    fn ccw(a: Point, b: Point, c: Point) -> bool {
        (c.y - a.y) * (b.x - a.x) > (b.y - a.y) * (c.x - a.x)
    }

    ccw(p1, q1, p2) != ccw(p1, q1, q2) || ccw(p2, q2, p1) != ccw(p2, q2, q1)
}

/// 计算线段交点
pub fn line_intersection(line1: &crate::geometry::Line, line2: &crate::geometry::Line) -> Option<Point> {
    let p1 = line1.start;
    let p2 = line1.end;
    let p3 = line2.start;
    let p4 = line2.end;

    let d = (p1.x - p2.x) * (p3.y - p4.y) - (p1.y - p2.y) * (p3.x - p4.x);

    if d == 0.0 {
        return None; // 平行或共线
    }

    let t = ((p1.x - p3.x) * (p3.y - p4.y) - (p1.y - p3.y) * (p3.x - p4.x)) / d;

    Some(Point::new(
        p1.x + t * (p2.x - p1.x),
        p1.y + t * (p2.y - p1.y),
    ))
}
