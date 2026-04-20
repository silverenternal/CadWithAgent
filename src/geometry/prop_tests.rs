//! 几何不变量属性测试
//!
//! 使用 proptest 测试几何操作的不变量属性

use crate::geometry::{Circle, Line, Point, Polygon, Rect};
use proptest::prelude::*;

/// 生成有效的 Point（避免 NaN 和 Infinity）
fn valid_point() -> impl Strategy<Value = Point> {
    (-1000.0..1000.0f64).prop_flat_map(|x| (-1000.0..1000.0f64).prop_map(move |y| Point::new(x, y)))
}

/// 生成两个不同的点
fn distinct_points() -> impl Strategy<Value = (Point, Point)> {
    (valid_point(), valid_point())
        .prop_filter("Points must be distinct", |(p1, p2)| p1.distance(p2) > 1e-6)
}

/// 生成有效的多边形顶点（至少 3 个点）
fn polygon_vertices() -> impl Strategy<Value = Vec<Point>> {
    prop::collection::vec(valid_point(), 3..10)
}

/// 生成有效的圆（正半径）
fn valid_circle() -> impl Strategy<Value = (Point, f64)> {
    (valid_point(), 0.1..1000.0f64)
}

/// 生成有效的矩形
fn valid_rect() -> impl Strategy<Value = Rect> {
    (valid_point(), valid_point()).prop_map(|(p1, p2)| {
        Rect::new(
            Point::new(p1.x.min(p2.x), p1.y.min(p2.y)),
            Point::new(p1.x.max(p2.x), p1.y.max(p2.y)),
        )
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// 测试：点到自身的距离为 0
    #[test]
    fn test_point_distance_to_self(p in valid_point()) {
        prop_assert!(p.distance(&p) < 1e-10);
    }

    /// 测试：距离的对称性
    #[test]
    fn test_point_distance_symmetry(p1 in valid_point(), p2 in valid_point()) {
        let d1 = p1.distance(&p2);
        let d2 = p2.distance(&p1);
        prop_assert!((d1 - d2).abs() < 1e-10);
    }

    /// 测试：距离的非负性
    #[test]
    fn test_point_distance_non_negative(p1 in valid_point(), p2 in valid_point()) {
        let d = p1.distance(&p2);
        prop_assert!(d >= 0.0);
    }

    /// 测试：线段长度的非负性
    #[test]
    fn test_line_length_non_negative((p1, p2) in distinct_points()) {
        let line = Line::new(p1, p2);
        prop_assert!(line.length() >= 0.0);
    }

    /// 测试：线段长度等于端点距离
    #[test]
    fn test_line_length_equals_endpoint_distance((p1, p2) in distinct_points()) {
        let line = Line::new(p1, p2);
        let expected = p1.distance(&p2);
        prop_assert!((line.length() - expected).abs() < 1e-10);
    }

    /// 测试：线段中点到两端点距离相等
    #[test]
    fn test_line_midpoint_equidistant((p1, p2) in distinct_points()) {
        let line = Line::new(p1, p2);
        let mid = line.midpoint();
        let d1 = mid.distance(&line.start);
        let d2 = mid.distance(&line.end);
        prop_assert!((d1 - d2).abs() < 1e-10);
    }

    /// 测试：中点在线段上（到两端点距离之和等于线段长度）
    #[test]
    fn test_midpoint_on_line((p1, p2) in distinct_points()) {
        let line = Line::new(p1, p2);
        let mid = line.midpoint();
        let sum = mid.distance(&line.start) + mid.distance(&line.end);
        prop_assert!((sum - line.length()).abs() < 1e-10);
    }

    /// 测试：多边形面积的非负性
    #[test]
    fn test_polygon_area_non_negative(vertices in polygon_vertices()) {
        let poly = Polygon::new(vertices);
        let area = poly.area();
        prop_assert!(area >= 0.0);
    }

    /// 测试：矩形面积的非负性
    #[test]
    fn test_rect_area_non_negative(rect in valid_rect()) {
        let area = rect.area();
        prop_assert!(area >= 0.0);
    }

    /// 测试：矩形宽度非负
    #[test]
    fn test_rect_width_non_negative(rect in valid_rect()) {
        prop_assert!(rect.width() >= 0.0);
    }

    /// 测试：矩形高度非负
    #[test]
    fn test_rect_height_non_negative(rect in valid_rect()) {
        prop_assert!(rect.height() >= 0.0);
    }

    /// 测试：矩形包含其中心点
    #[test]
    fn test_rect_contains_center(rect in valid_rect()) {
        let center = rect.center();
        prop_assert!(rect.contains(&center));
    }

    /// 测试：圆面积的非负性
    #[test]
    fn test_circle_area_non_negative((center, radius) in valid_circle()) {
        let circle = Circle::new(center, radius);
        let area = circle.area();
        prop_assert!(area >= 0.0);
    }

    /// 测试：圆周长非负
    #[test]
    fn test_circle_circumference_non_negative((center, radius) in valid_circle()) {
        let circle = Circle::new(center, radius);
        let circumference = circle.circumference();
        prop_assert!(circumference >= 0.0);
    }

    /// 测试：圆直径非负
    #[test]
    fn test_circle_diameter_non_negative((center, radius) in valid_circle()) {
        let circle = Circle::new(center, radius);
        let diameter = circle.diameter();
        prop_assert!(diameter >= 0.0);
    }

    /// 测试：圆面积公式正确性 (A = πr²)
    #[test]
    fn test_circle_area_formula((center, radius) in valid_circle()) {
        let circle = Circle::new(center, radius);
        let expected = std::f64::consts::PI * radius * radius;
        let actual = circle.area();
        prop_assert!((actual - expected).abs() < 1e-6);
    }

    /// 测试：圆周长公式正确性 (C = 2πr)
    #[test]
    fn test_circle_circumference_formula((center, radius) in valid_circle()) {
        let circle = Circle::new(center, radius);
        let expected = 2.0 * std::f64::consts::PI * radius;
        let actual = circle.circumference();
        prop_assert!((actual - expected).abs() < 1e-6);
    }

    /// 测试：圆直径与半径关系 (d = 2r)
    #[test]
    fn test_circle_diameter_radius_relationship((center, radius) in valid_circle()) {
        let circle = Circle::new(center, radius);
        let expected = 2.0 * radius;
        let actual = circle.diameter();
        prop_assert!((actual - expected).abs() < 1e-10);
    }

    /// 测试：点到线段端点的距离小于等于线段长度加上点到线段的距离
    #[test]
    fn test_triangle_inequality_for_line((p1, p2) in distinct_points(), p3 in valid_point()) {
        let line = Line::new(p1, p2);
        let d1 = p3.distance(&line.start);
        let d2 = p3.distance(&line.end);
        let len = line.length();
        // 三角不等式：任意两边之和大于第三边
        prop_assert!(d1 + d2 >= len);
    }

    /// 测试：多边形周长非负
    #[test]
    fn test_polygon_perimeter_non_negative(vertices in polygon_vertices()) {
        let poly = Polygon::new(vertices);
        let perimeter = poly.perimeter();
        prop_assert!(perimeter >= 0.0);
    }

    /// 测试：矩形转多边形的面积一致性
    #[test]
    fn test_rect_to_polygon_area(rect in valid_rect()) {
        let rect_area = rect.area();
        let poly = rect.to_polygon();
        let poly_area = poly.area();
        prop_assert!((rect_area - poly_area).abs() < 1e-6);
    }

    /// 测试：坐标变换的平移不变性（距离保持不变）
    #[test]
    fn test_translation_distance_invariance(
        (p1, p2) in distinct_points(),
        dx in -100.0..100.0f64,
        dy in -100.0..100.0f64
    ) {
        let original_dist = p1.distance(&p2);
        let translated_p1 = Point::new(p1.x + dx, p1.y + dy);
        let translated_p2 = Point::new(p2.x + dx, p2.y + dy);
        let translated_dist = translated_p1.distance(&translated_p2);
        prop_assert!((original_dist - translated_dist).abs() < 1e-10);
    }

    /// 测试：坐标变换的缩放一致性
    #[test]
    fn test_scaling_distance_consistency(
        (p1, p2) in distinct_points(),
        scale in 0.1..10.0f64
    ) {
        let original_dist = p1.distance(&p2);
        let scaled_p1 = Point::new(p1.x * scale, p1.y * scale);
        let scaled_p2 = Point::new(p2.x * scale, p2.y * scale);
        let scaled_dist = scaled_p1.distance(&scaled_p2);
        let expected = original_dist * scale;
        prop_assert!((scaled_dist - expected).abs() < 1e-6 * original_dist);
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;

    #[test]
    fn test_valid_point_generation() {
        // 简单测试确保策略生成有效的点
        let point = Point::new(1.0, 2.0);
        assert!(point.is_valid());
        // 使用 try_new 测试无效点
        assert!(Point::try_new(f64::NAN, 0.0).is_err());
        assert!(Point::try_new(f64::INFINITY, 0.0).is_err());
    }

    #[test]
    fn test_line_direction() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(1.0, 0.0);
        let line = Line::new(p1, p2);
        let dir = line.direction();

        // 单位向量的长度应为 1
        let dir_len = (dir.x.powi(2) + dir.y.powi(2)).sqrt();
        assert!((dir_len - 1.0).abs() < 1e-10);
    }
}
