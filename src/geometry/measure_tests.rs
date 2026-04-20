//! 几何测量工具测试

#[cfg(test)]
mod tests {
    use crate::geometry::{
        CircleDimensions, GeometryMeasurer, ParallelResult, PerpendicularResult, RectDimensions,
    };
    use std::time::Duration;

    #[test]
    fn test_measure_length() {
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

        // 零长度向量
        let angle = measurer.measure_angle([1.0, 1.0], [1.0, 1.0], [2.0, 2.0]);
        assert!((angle - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_check_parallel() {
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();
        let result = measurer.measure_rect([0.0, 0.0], [4.0, 3.0]);

        assert!((result.width - 4.0).abs() < 1e-10);
        assert!((result.height - 3.0).abs() < 1e-10);
        assert!((result.area - 12.0).abs() < 1e-10);
        assert!((result.center[0] - 2.0).abs() < 1e-10);
        assert!((result.center[1] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_measure_circle() {
        let mut measurer = GeometryMeasurer::new();
        let result = measurer.measure_circle([0.0, 0.0], 5.0);

        assert!((result.radius - 5.0).abs() < 1e-10);
        assert!((result.diameter - 10.0).abs() < 1e-10);
        assert!((result.area - std::f64::consts::PI * 25.0).abs() < 1e-10);
        assert!((result.circumference - std::f64::consts::PI * 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_perimeter() {
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

        // 线段长度为零的情况
        let dist = measurer.point_to_line_distance([3.0, 4.0], [0.0, 0.0], [0.0, 0.0]);
        assert!((dist - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_midpoint() {
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

        // 45 度角：p1(1,0), p2(0,0), p3(1,1)
        // v1 = (1,0), v2 = (1,1), 夹角 45 度
        let angle = measurer.measure_angle([1.0, 0.0], [0.0, 0.0], [1.0, 1.0]);
        assert!((angle - 45.0).abs() < 1.0);
    }

    #[test]
    fn test_check_parallel_with_tolerance() {
        let mut measurer = GeometryMeasurer::new();

        // 接近平行（0.5 度偏差）
        let result = measurer.check_parallel([0.0, 0.0], [10.0, 0.0], [0.0, 0.1], [10.0, 0.1]);
        assert!(result.is_parallel);
    }

    #[test]
    fn test_check_perpendicular_with_tolerance() {
        let mut measurer = GeometryMeasurer::new();

        // 接近垂直
        let result = measurer.check_perpendicular([0.0, 0.0], [10.0, 0.0], [0.0, 0.0], [0.1, 10.0]);
        assert!(result.is_perpendicular);
    }

    #[test]
    fn test_measure_negative_coordinates() {
        let mut measurer = GeometryMeasurer::new();

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
        let mut measurer = GeometryMeasurer::new();

        // 3-4-5 三角形周长
        let perimeter = measurer.measure_perimeter(vec![[0.0, 0.0], [3.0, 0.0], [0.0, 4.0]]);
        assert!((perimeter - 12.0).abs() < 1e-10);
    }

    // ==================== 缓存功能测试 ====================

    #[test]
    fn test_cache_builder() {
        let measurer = GeometryMeasurer::builder()
            .angle_tolerance(0.01)
            .enable_cache(true)
            .cache_capacity(100)
            .cache_expiration_secs(60)
            .build();

        assert!(measurer.is_cache_enabled());
    }

    #[test]
    fn test_cache_disable_by_default() {
        let measurer = GeometryMeasurer::new();
        assert!(!measurer.is_cache_enabled());
    }

    #[test]
    fn test_cache_measure_length() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次测量（未命中）
        let length1 = measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
        assert!((length1 - 5.0).abs() < 1e-10);

        // 第二次测量（命中缓存）
        let length2 = measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
        assert!((length2 - 5.0).abs() < 1e-10);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_measure_area() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        let vertices = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]];

        // 第一次测量
        let area1 = measurer.measure_area(vertices.clone());
        assert!((area1 - 12.0).abs() < 1e-10);

        // 第二次测量（缓存命中）
        let area2 = measurer.measure_area(vertices.clone());
        assert!((area2 - 12.0).abs() < 1e-10);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_measure_angle() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次测量
        let angle1 = measurer.measure_angle([0.0, 0.0], [1.0, 0.0], [1.0, 1.0]);
        assert!((angle1 - 90.0).abs() < 0.1);

        // 第二次测量（缓存命中）
        let angle2 = measurer.measure_angle([0.0, 0.0], [1.0, 0.0], [1.0, 1.0]);
        assert!((angle2 - 90.0).abs() < 0.1);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_check_parallel() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次检查
        let result1 = measurer.check_parallel([0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        assert!(result1.is_parallel);

        // 第二次检查（缓存命中）
        let result2 = measurer.check_parallel([0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        assert!(result2.is_parallel);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_check_perpendicular() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次检查
        let result1 = measurer.check_perpendicular([0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]);
        assert!(result1.is_perpendicular);

        // 第二次检查（缓存命中）
        let result2 = measurer.check_perpendicular([0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [0.0, 1.0]);
        assert!(result2.is_perpendicular);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_measure_rect() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次测量
        let rect1 = measurer.measure_rect([0.0, 0.0], [4.0, 3.0]);
        assert!((rect1.width - 4.0).abs() < 1e-10);

        // 第二次测量（缓存命中）
        let rect2 = measurer.measure_rect([0.0, 0.0], [4.0, 3.0]);
        assert!((rect2.width - 4.0).abs() < 1e-10);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_measure_circle() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次测量
        let circle1 = measurer.measure_circle([0.0, 0.0], 5.0);
        assert!((circle1.radius - 5.0).abs() < 1e-10);

        // 第二次测量（缓存命中）
        let circle2 = measurer.measure_circle([0.0, 0.0], 5.0);
        assert!((circle2.radius - 5.0).abs() < 1e-10);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_measure_perimeter() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        let vertices = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

        // 第一次测量
        let perimeter1 = measurer.measure_perimeter(vertices.clone());
        assert!((perimeter1 - 4.0).abs() < 1e-10);

        // 第二次测量（缓存命中）
        let perimeter2 = measurer.measure_perimeter(vertices.clone());
        assert!((perimeter2 - 4.0).abs() < 1e-10);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_point_to_line_distance() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次测量
        let dist1 = measurer.point_to_line_distance([0.0, 5.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((dist1 - 5.0).abs() < 1e-10);

        // 第二次测量（缓存命中）
        let dist2 = measurer.point_to_line_distance([0.0, 5.0], [0.0, 0.0], [10.0, 0.0]);
        assert!((dist2 - 5.0).abs() < 1e-10);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_midpoint() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 第一次测量
        let mid1 = measurer.midpoint([0.0, 0.0], [4.0, 6.0]);
        assert!((mid1[0] - 2.0).abs() < 1e-10);
        assert!((mid1[1] - 3.0).abs() < 1e-10);

        // 第二次测量（缓存命中）
        let mid2 = measurer.midpoint([0.0, 0.0], [4.0, 6.0]);
        assert!((mid2[0] - 2.0).abs() < 1e-10);
        assert!((mid2[1] - 3.0).abs() < 1e-10);

        let stats = measurer.cache_stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_cache_clear() {
        let measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 填充缓存
        let mut measurer = measurer;
        measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
        measurer.measure_area(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);

        // 清空缓存
        measurer.clear_cache();

        let _stats = measurer.cache_stats();
        // 清空后统计应保持不变
    }

    #[test]
    fn test_cache_reset_stats() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 填充缓存
        measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
        measurer.measure_length([0.0, 0.0], [3.0, 4.0]); // 命中

        // 重置统计
        measurer.reset_cache_stats();

        let stats = measurer.cache_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_disable_enable() {
        let mut measurer = GeometryMeasurer::new();

        // 初始禁用
        assert!(!measurer.is_cache_enabled());

        // 启用缓存
        measurer.enable_cache(100, Some(Duration::from_secs(60)));
        assert!(measurer.is_cache_enabled());

        // 禁用缓存
        measurer.disable_cache();
        assert!(!measurer.is_cache_enabled());
    }

    #[test]
    fn test_cache_hit_rate() {
        let mut measurer = GeometryMeasurer::builder()
            .enable_cache(true)
            .cache_capacity(100)
            .build();

        // 多次测量同一长度
        for _ in 0..5 {
            measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
        }

        let stats = measurer.cache_stats();
        // 第一次是 miss，后面 4 次是 hits
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 4);
        assert!((stats.hit_rate() - 0.8).abs() < 0.01);
    }
}
