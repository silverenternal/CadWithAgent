//! 几何测量工具测试

#[cfg(test)]
mod tests {
    use crate::geometry::{
        CircleDimensions, GeometryMeasurer, ParallelResult, PerpendicularResult, RectDimensions,
    };

    #[test]
    fn test_measure_length() {
        let measurer = GeometryMeasurer;

        // 水平线
        let length = measurer.measure_length([0.0, 0.0], [3.0, 0.0]);
        assert!((length - 3.0).abs() < 1e-10);

        // 垂直线
        let length = measurer.measure_length([0.0, 0.0], [0.0, 4.0]);
        assert!((length - 4.0).abs() < 1e-10);

        // 斜线 (3-4-5 三角形)
        let length = measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
        assert!((length - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_area() {
        let measurer = GeometryMeasurer;

        // 正方形
        let area = measurer.measure_area(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        assert!((area - 1.0).abs() < 1e-10);

        // 矩形
        let area = measurer.measure_area(vec![[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]]);
        assert!((area - 12.0).abs() < 1e-10);

        // 三角形
        let area = measurer.measure_area(vec![[0.0, 0.0], [4.0, 0.0], [0.0, 3.0]]);
        assert!((area - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_angle() {
        let measurer = GeometryMeasurer;

        // 直角
        let angle = measurer.measure_angle([0.0, 0.0], [1.0, 0.0], [1.0, 1.0]);
        assert!((angle - 90.0).abs() < 0.1);

        // 平角
        let angle = measurer.measure_angle([0.0, 0.0], [1.0, 0.0], [2.0, 0.0]);
        assert!((angle - 180.0).abs() < 0.1);

        // 45 度角
        let angle = measurer.measure_angle([0.0, 0.0], [1.0, 0.0], [1.0, 1.0]);
        assert!((angle - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_measure_angle_zero_length() {
        let measurer = GeometryMeasurer;

        // 零长度向量
        let angle = measurer.measure_angle([1.0, 1.0], [1.0, 1.0], [2.0, 2.0]);
        assert!((angle - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_check_parallel() {
        let measurer = GeometryMeasurer;

        // 平行线
        let result = measurer.check_parallel([0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        assert!(result.is_parallel);
        assert!(result.angle_diff < 0.1);

        // 垂直线（不平行）
        let result = measurer.check_parallel([0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]);
        assert!(!result.is_parallel);
        assert!((result.angle_diff - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_check_perpendicular() {
        let measurer = GeometryMeasurer;

        // 垂直线
        let result = measurer.check_perpendicular([0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]);
        assert!(result.is_perpendicular);
        assert!(result.angle_diff < 0.1);

        // 平行线（不垂直）
        let result = measurer.check_perpendicular([0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        assert!(!result.is_perpendicular);
        assert!((result.angle_diff - 90.0).abs() < 0.1);
    }

    #[test]
    fn test_measure_rect() {
        let measurer = GeometryMeasurer;
        let result = measurer.measure_rect([0.0, 0.0], [4.0, 3.0]);

        assert!((result.width - 4.0).abs() < 1e-10);
        assert!((result.height - 3.0).abs() < 1e-10);
        assert!((result.area - 12.0).abs() < 1e-10);
        assert!((result.center[0] - 2.0).abs() < 1e-10);
        assert!((result.center[1] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_measure_circle() {
        let measurer = GeometryMeasurer;
        let result = measurer.measure_circle([0.0, 0.0], 5.0);

        assert!((result.radius - 5.0).abs() < 1e-10);
        assert!((result.diameter - 10.0).abs() < 1e-10);
        assert!((result.area - std::f64::consts::PI * 25.0).abs() < 1e-10);
        assert!((result.circumference - std::f64::consts::PI * 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_perimeter() {
        let measurer = GeometryMeasurer;

        // 正方形周长
        let perimeter =
            measurer.measure_perimeter(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        assert!((perimeter - 4.0).abs() < 1e-10);

        // 矩形周长
        let perimeter =
            measurer.measure_perimeter(vec![[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]]);
        assert!((perimeter - 14.0).abs() < 1e-10);
    }

    #[test]
    fn test_point_to_line_distance() {
        let measurer = GeometryMeasurer;

        // 点到水平线的距离
        let dist = measurer.point_to_line_distance([0.0, 5.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((dist - 5.0).abs() < 1e-10);

        // 点到垂直线的距离
        let dist = measurer.point_to_line_distance([5.0, 0.0], [0.0, 0.0], [0.0, 10.0]);
        assert!((dist - 5.0).abs() < 1e-10);

        // 点在线上的情况
        let dist = measurer.point_to_line_distance([5.0, 0.0], [0.0, 0.0], [10.0, 0.0]);
        assert!(dist < 1e-10);
    }

    #[test]
    fn test_point_to_line_distance_zero_length() {
        let measurer = GeometryMeasurer;

        // 线段长度为零的情况
        let dist = measurer.point_to_line_distance([3.0, 4.0], [0.0, 0.0], [0.0, 0.0]);
        assert!((dist - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_midpoint() {
        let measurer = GeometryMeasurer;

        let mid = measurer.midpoint([0.0, 0.0], [4.0, 6.0]);
        assert!((mid[0] - 2.0).abs() < 1e-10);
        assert!((mid[1] - 3.0).abs() < 1e-10);

        let mid = measurer.midpoint([1.0, 1.0], [1.0, 1.0]);
        assert!((mid[0] - 1.0).abs() < 1e-10);
        assert!((mid[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_parallel_result_serialization() {
        let result = ParallelResult {
            is_parallel: true,
            angle_diff: 0.5,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("is_parallel"));
        assert!(json.contains("angle_diff"));

        let deserialized: ParallelResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_parallel);
        assert!((deserialized.angle_diff - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_perpendicular_result_serialization() {
        let result = PerpendicularResult {
            is_perpendicular: true,
            angle_diff: 0.3,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PerpendicularResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.is_perpendicular);
        assert!((deserialized.angle_diff - 0.3).abs() < 1e-10);
    }

    #[test]
    fn test_rect_dimensions_serialization() {
        let result = RectDimensions {
            width: 10.0,
            height: 20.0,
            area: 200.0,
            center: [5.0, 10.0],
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: RectDimensions = serde_json::from_str(&json).unwrap();
        assert!((deserialized.width - 10.0).abs() < 1e-10);
        assert!((deserialized.height - 20.0).abs() < 1e-10);
    }

    #[test]
    fn test_circle_dimensions_serialization() {
        let result = CircleDimensions {
            radius: 5.0,
            diameter: 10.0,
            area: 78.53981633974483,
            circumference: 31.41592653589793,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CircleDimensions = serde_json::from_str(&json).unwrap();
        assert!((deserialized.radius - 5.0).abs() < 1e-10);
        assert!((deserialized.diameter - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_angle_45_degrees() {
        let measurer = GeometryMeasurer;

        // 45 度角：p1(1,0), p2(0,0), p3(1,1)
        // v1 = (1,0), v2 = (1,1), 夹角 45 度
        let angle = measurer.measure_angle([1.0, 0.0], [0.0, 0.0], [1.0, 1.0]);
        assert!((angle - 45.0).abs() < 1.0);
    }

    #[test]
    fn test_check_parallel_with_tolerance() {
        let measurer = GeometryMeasurer;

        // 接近平行（0.5 度偏差）
        let result = measurer.check_parallel([0.0, 0.0], [10.0, 0.0], [0.0, 0.1], [10.0, 0.1]);
        assert!(result.is_parallel);
    }

    #[test]
    fn test_check_perpendicular_with_tolerance() {
        let measurer = GeometryMeasurer;

        // 接近垂直
        let result = measurer.check_perpendicular([0.0, 0.0], [10.0, 0.0], [0.0, 0.0], [0.1, 10.0]);
        assert!(result.is_perpendicular);
    }

    #[test]
    fn test_measure_negative_coordinates() {
        let measurer = GeometryMeasurer;

        // 负坐标的长度
        let length = measurer.measure_length([-3.0, -4.0], [0.0, 0.0]);
        assert!((length - 5.0).abs() < 1e-10);

        // 负坐标的矩形
        let result = measurer.measure_rect([-2.0, -3.0], [2.0, 3.0]);
        assert!((result.width - 4.0).abs() < 1e-10);
        assert!((result.height - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_triangle_perimeter() {
        let measurer = GeometryMeasurer;

        // 3-4-5 三角形周长
        let perimeter = measurer.measure_perimeter(vec![[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]]);
        assert!((perimeter - 12.0).abs() < 1e-10);
    }
}
