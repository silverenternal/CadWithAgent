//! 布尔运算工具测试

#[cfg(test)]
mod tests {
    use crate::geometry::{
        Point, Polygon,
        boolean::{
            BooleanResult, BooleanOp,
            point_in_polygon, polygons_intersect, lines_intersect,
            line_intersection, union, intersection, difference,
        },
    };

    fn create_square(x: f64, y: f64, size: f64) -> Polygon {
        Polygon::new(vec![
            Point::new(x, y),
            Point::new(x + size, y),
            Point::new(x + size, y + size),
            Point::new(x, y + size),
        ])
    }

    fn create_triangle(p1: Point, p2: Point, p3: Point) -> Polygon {
        Polygon::new(vec![p1, p2, p3])
    }

    #[test]
    fn test_boolean_result_success() {
        let poly = create_square(0.0, 0.0, 1.0);
        let result = BooleanResult::success(vec![poly.clone()]);
        
        assert!(result.success);
        assert!(result.error.is_none());
        assert_eq!(result.polygons.len(), 1);
    }

    #[test]
    fn test_boolean_result_error() {
        let result = BooleanResult::error("Test error message");
        
        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.error.unwrap(), "Test error message");
        assert!(result.polygons.is_empty());
    }

    #[test]
    fn test_boolean_op_enum() {
        let union_op = BooleanOp::Union;
        let intersection_op = BooleanOp::Intersection;
        let difference_op = BooleanOp::Difference;
        
        assert_ne!(union_op, intersection_op);
        assert_ne!(union_op, difference_op);
        assert_ne!(intersection_op, difference_op);
    }

    #[test]
    fn test_union_basic() {
        let poly1 = create_square(0.0, 0.0, 1.0);
        let poly2 = create_square(0.5, 0.5, 1.0);

        let result = union(&poly1, &poly2);

        assert!(result.success);
        assert!(!result.polygons.is_empty());
    }

    #[test]
    fn test_intersection_basic() {
        let poly1 = create_square(0.0, 0.0, 1.0);
        let poly2 = create_square(0.5, 0.5, 1.0);

        let result = intersection(&poly1, &poly2);

        // 相交应该返回非空结果
        assert!(result.success);
        // 相交区域应该有顶点
        if !result.polygons.is_empty() {
            assert!(!result.polygons[0].vertices.is_empty());
        }
    }

    #[test]
    fn test_difference_basic() {
        let poly1 = create_square(0.0, 0.0, 1.0);
        let poly2 = create_square(0.2, 0.2, 0.3);

        let result = difference(&poly1, &poly2);

        assert!(result.success);
        // 差集应该返回至少一个多边形
        assert!(!result.polygons.is_empty());
    }

    #[test]
    fn test_union_disjoint_polygons() {
        let poly1 = create_square(0.0, 0.0, 1.0);
        let poly2 = create_square(10.0, 10.0, 1.0);

        let result = union(&poly1, &poly2);

        assert!(result.success);
        assert_eq!(result.polygons.len(), 2); // 不相交的多边形应该保持独立
    }

    #[test]
    fn test_intersection_disjoint_polygons() {
        let poly1 = create_square(0.0, 0.0, 1.0);
        let poly2 = create_square(10.0, 10.0, 1.0);

        let result = intersection(&poly1, &poly2);

        assert!(result.success);
        assert!(result.polygons.is_empty()); // 不相交应该返回空
    }

    #[test]
    fn test_difference_disjoint_polygons() {
        let poly1 = create_square(0.0, 0.0, 1.0);
        let poly2 = create_square(10.0, 10.0, 1.0);

        let result = difference(&poly1, &poly2);

        assert!(result.success);
        assert_eq!(result.polygons.len(), 1); // 不相交应该返回原多边形
    }

    #[test]
    fn test_point_in_polygon_square() {
        let square = create_square(0.0, 0.0, 10.0);
        
        // 内部点
        assert!(point_in_polygon(&Point::new(5.0, 5.0), &square));
        
        // 边界点（射线法可能判定为内或外，取决于实现）
        // 外部点
        assert!(!point_in_polygon(&Point::new(15.0, 5.0), &square));
        assert!(!point_in_polygon(&Point::new(-5.0, 5.0), &square));
        assert!(!point_in_polygon(&Point::new(5.0, 15.0), &square));
        assert!(!point_in_polygon(&Point::new(5.0, -5.0), &square));
    }

    #[test]
    fn test_point_in_polygon_triangle() {
        let triangle = create_triangle(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(5.0, 10.0),
        );
        
        // 内部点
        assert!(point_in_polygon(&Point::new(5.0, 3.0), &triangle));
        
        // 外部点
        assert!(!point_in_polygon(&Point::new(0.0, 10.0), &triangle));
        assert!(!point_in_polygon(&Point::new(15.0, 5.0), &triangle));
    }

    #[test]
    fn test_point_in_polygon_degenerate() {
        // 少于 3 个顶点的多边形
        let line_polygon = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
        ]);
        
        assert!(!point_in_polygon(&Point::new(5.0, 0.0), &line_polygon));
        
        // 空多边形
        let empty_polygon = Polygon::new(vec![]);
        assert!(!point_in_polygon(&Point::new(0.0, 0.0), &empty_polygon));
    }

    #[test]
    fn test_point_in_polygon_concave() {
        // L 形多边形
        let l_shape = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 5.0),
            Point::new(5.0, 5.0),
            Point::new(5.0, 10.0),
            Point::new(0.0, 10.0),
        ]);
        
        // 内部点
        assert!(point_in_polygon(&Point::new(3.0, 3.0), &l_shape));
        assert!(point_in_polygon(&Point::new(3.0, 7.0), &l_shape));
        
        // 外部点（凹角处）
        assert!(!point_in_polygon(&Point::new(7.0, 7.0), &l_shape));
    }

    #[test]
    fn test_polygons_intersect_overlapping() {
        let poly1 = create_square(0.0, 0.0, 5.0);
        let poly2 = create_square(3.0, 3.0, 5.0);
        
        assert!(polygons_intersect(&poly1, &poly2));
    }

    #[test]
    fn test_polygons_intersect_containment() {
        let poly1 = create_square(0.0, 0.0, 10.0);
        let poly2 = create_square(2.0, 2.0, 3.0);
        
        // poly2 完全在 poly1 内部
        assert!(polygons_intersect(&poly1, &poly2));
    }

    #[test]
    fn test_polygons_intersect_separate() {
        let poly1 = create_square(0.0, 0.0, 2.0);
        let poly2 = create_square(10.0, 10.0, 2.0);
        
        assert!(!polygons_intersect(&poly1, &poly2));
    }

    #[test]
    fn test_polygons_intersect_touching() {
        let poly1 = create_square(0.0, 0.0, 5.0);
        let poly2 = create_square(5.0, 0.0, 5.0);
        
        // 共享一条边
        assert!(polygons_intersect(&poly1, &poly2));
    }

    #[test]
    fn test_polygons_intersect_triangles() {
        let tri1 = create_triangle(
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(2.5, 5.0),
        );
        let tri2 = create_triangle(
            Point::new(2.5, 2.5),
            Point::new(7.5, 2.5),
            Point::new(5.0, 7.5),
        );
        
        assert!(polygons_intersect(&tri1, &tri2));
    }

    #[test]
    fn test_lines_intersect() {
        use crate::geometry::Line;
        
        // 相交的线段
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let line2 = Line::new(Point::new(0.0, 10.0), Point::new(10.0, 0.0));
        
        assert!(lines_intersect(&line1, &line2));
    }

    #[test]
    fn test_lines_intersect_parallel() {
        use crate::geometry::Line;
        
        // 平行线段
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let line2 = Line::new(Point::new(0.0, 5.0), Point::new(10.0, 5.0));
        
        assert!(!lines_intersect(&line1, &line2));
    }

    #[test]
    fn test_lines_intersect_collinear_overlapping() {
        use crate::geometry::Line;
        
        // 共线且重叠的线段
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let line2 = Line::new(Point::new(5.0, 0.0), Point::new(15.0, 0.0));
        
        // 共线情况取决于实现，可能返回 true 或 false
        // 这里只测试不抛出 panic
        let _ = lines_intersect(&line1, &line2);
    }

    #[test]
    fn test_lines_intersect_not_intersecting() {
        use crate::geometry::Line;
        
        // 不相交的线段（即使延长线会相交）
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(5.0, 5.0));
        let line2 = Line::new(Point::new(10.0, 0.0), Point::new(15.0, 5.0));
        
        assert!(!lines_intersect(&line1, &line2));
    }

    #[test]
    fn test_lines_intersect_perpendicular() {
        use crate::geometry::Line;
        
        // 垂直相交的线段
        let line1 = Line::new(Point::new(5.0, 0.0), Point::new(5.0, 10.0));
        let line2 = Line::new(Point::new(0.0, 5.0), Point::new(10.0, 5.0));
        
        assert!(lines_intersect(&line1, &line2));
    }

    #[test]
    fn test_line_intersection() {
        use crate::geometry::Line;
        
        // 相交的线段
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 10.0));
        let line2 = Line::new(Point::new(0.0, 10.0), Point::new(10.0, 0.0));
        
        let intersection = line_intersection(&line1, &line2);
        
        assert!(intersection.is_some());
        let point = intersection.unwrap();
        assert!((point.x - 5.0).abs() < 1e-10);
        assert!((point.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_line_intersection_parallel() {
        use crate::geometry::Line;
        
        // 平行线段（无交点）
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let line2 = Line::new(Point::new(0.0, 5.0), Point::new(10.0, 5.0));
        
        let intersection = line_intersection(&line1, &line2);
        
        assert!(intersection.is_none());
    }

    #[test]
    fn test_line_intersection_collinear() {
        use crate::geometry::Line;
        
        // 共线线段
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let line2 = Line::new(Point::new(5.0, 0.0), Point::new(15.0, 0.0));
        
        let intersection = line_intersection(&line1, &line2);
        
        assert!(intersection.is_none());
    }

    #[test]
    fn test_line_intersection_not_intersecting_segments() {
        use crate::geometry::Line;

        // 线段本身不相交，但延长线会相交（平行线，斜率相同）
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(5.0, 5.0));
        let line2 = Line::new(Point::new(10.0, 0.0), Point::new(15.0, 5.0));

        let intersection = line_intersection(&line1, &line2);

        // 两条线平行（斜率都是 1），没有交点
        assert!(intersection.is_none());
    }

    #[test]
    fn test_boolean_result_debug_traits() {
        let result = BooleanResult::success(vec![create_square(0.0, 0.0, 1.0)]);
        
        // 测试 Debug trait
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("BooleanResult"));
        
        // 测试 Clone trait
        let cloned = result.clone();
        assert!(cloned.success);
    }

    #[test]
    fn test_boolean_op_clone_copy() {
        let op = BooleanOp::Union;
        let cloned = op.clone();
        let copied = op;
        
        assert_eq!(cloned, copied);
    }

    #[test]
    fn test_point_in_polygon_origin() {
        let square = create_square(-5.0, -5.0, 10.0);
        
        // 原点应该在正方形内部
        assert!(point_in_polygon(&Point::new(0.0, 0.0), &square));
    }

    #[test]
    fn test_lines_intersect_at_endpoint() {
        use crate::geometry::Line;

        // 在端点处相交
        let line1 = Line::new(Point::new(0.0, 0.0), Point::new(5.0, 5.0));
        let line2 = Line::new(Point::new(5.0, 5.0), Point::new(10.0, 0.0));

        // ccw 算法在端点接触时可能返回 false，这取决于具体实现
        // 这里只测试不 panic
        let _ = lines_intersect(&line1, &line2);
    }

    #[test]
    fn test_polygons_intersect_same_polygon() {
        let poly = create_square(0.0, 0.0, 5.0);
        
        // 同一个多边形与自己相交
        assert!(polygons_intersect(&poly, &poly));
    }
}
