//! 高级几何算法示例
//!
//! 演示新添加的高级几何算法功能：
//! - 线段交点计算
//! - 点到线段最近点
//! - 多边形质心
//! - 凸包计算
//! - 点在多边形内判断
//! - 向量夹角计算

use cadagent::geometry::{
    closest_point_on_segment, convex_hull, line_intersection, point_in_polygon,
    point_to_segment_distance, polygon_centroid, vector_angle, LineIntersection,
};

fn main() -> anyhow::Result<()> {
    println!("=== 高级几何算法示例 ===\n");

    // 1. 线段交点计算
    println!("1. 线段交点计算");

    // 相交的线段
    let result = line_intersection([0.0, 0.0], [4.0, 4.0], [0.0, 4.0], [4.0, 0.0]);
    match result {
        LineIntersection::Single(point) => {
            println!("   相交线段交点：({:.2}, {:.2})", point[0], point[1]);
        }
        _ => println!("   无交点"),
    }

    // 平行的线段
    let result = line_intersection([0.0, 0.0], [5.0, 0.0], [0.0, 3.0], [5.0, 3.0]);
    match result {
        LineIntersection::None => println!("   平行线段：无交点 ✓"),
        _ => println!("   意外结果"),
    }

    // 2. 点到线段最近点
    println!("\n2. 点到线段最近点");

    let point = [5.0, 5.0];
    let line_start = [0.0, 0.0];
    let line_end = [10.0, 0.0];

    let closest = closest_point_on_segment(point, line_start, line_end);
    println!(
        "   点 (5, 5) 到线段 [(0,0)-(10,0)] 的最近点：({:.2}, {:.2})",
        closest[0], closest[1]
    );

    let dist = point_to_segment_distance(point, line_start, line_end);
    println!("   点到线段的距离：{:.2}", dist);

    // 3. 多边形质心
    println!("\n3. 多边形质心");

    // 正方形
    let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    if let Some(centroid) = polygon_centroid(&square) {
        println!("   正方形质心：({:.2}, {:.2})", centroid[0], centroid[1]);
    }

    // 三角形
    let triangle = [[0.0, 0.0], [6.0, 0.0], [3.0, 4.0]];
    if let Some(centroid) = polygon_centroid(&triangle) {
        println!("   三角形质心：({:.2}, {:.2})", centroid[0], centroid[1]);
    }

    // L 形多边形
    let l_shape = [
        [0.0, 0.0],
        [6.0, 0.0],
        [6.0, 2.0],
        [2.0, 2.0],
        [2.0, 6.0],
        [0.0, 6.0],
    ];
    if let Some(centroid) = polygon_centroid(&l_shape) {
        println!("   L 形质心：({:.2}, {:.2})", centroid[0], centroid[1]);
    }

    // 4. 凸包计算
    println!("\n4. 凸包计算");

    let points = vec![
        [0.0, 0.0],
        [1.0, 1.0],
        [2.0, 0.0],
        [1.0, 2.0],
        [0.5, 0.5],
        [1.0, 0.5],
        [1.5, 1.0],
        [0.5, 1.5],
    ];

    let hull = convex_hull(&points);
    println!("   输入点数：{}", points.len());
    println!("   凸包顶点数：{}", hull.len());
    println!("   凸包顶点:");
    for (i, point) in hull.iter().enumerate() {
        println!("     [{}] ({:.2}, {:.2})", i, point[0], point[1]);
    }

    // 5. 点在多边形内判断
    println!("\n5. 点在多边形内判断");

    let polygon = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];

    let test_points = vec![
        ([5.0, 5.0], "中心"),
        ([0.0, 0.0], "顶点"),
        ([10.0, 5.0], "边上"),
        ([15.0, 5.0], "外部"),
        ([-5.0, 5.0], "外部左侧"),
    ];

    for (point, label) in test_points {
        let inside = point_in_polygon(point, &polygon);
        println!(
            "   点 {} {:?}: {}",
            label,
            point,
            if inside { "内部" } else { "外部" }
        );
    }

    // 6. 向量夹角
    println!("\n6. 向量夹角计算");

    use std::f64::consts::PI;

    // 垂直向量
    let angle = vector_angle([1.0, 0.0], [0.0, 1.0]);
    println!(
        "   垂直向量夹角：{:.2}° ({:.4} rad)",
        angle * 180.0 / PI,
        angle
    );

    // 平行向量
    let angle = vector_angle([1.0, 0.0], [1.0, 0.0]);
    println!(
        "   平行向量夹角：{:.2}° ({:.4} rad)",
        angle * 180.0 / PI,
        angle
    );

    // 反向向量
    let angle = vector_angle([1.0, 0.0], [-1.0, 0.0]);
    println!(
        "   反向向量夹角：{:.2}° ({:.4} rad)",
        angle * 180.0 / PI,
        angle
    );

    // 45 度角
    let angle = vector_angle([1.0, 0.0], [1.0, 1.0]);
    println!(
        "   45 度向量夹角：{:.2}° ({:.4} rad)",
        angle * 180.0 / PI,
        angle
    );

    // 7. 实际应用场景
    println!("\n7. 实际应用场景：碰撞检测");

    // 检测点是否在房间内
    let room = [[0.0, 0.0], [500.0, 0.0], [500.0, 400.0], [0.0, 400.0]];
    let person_position = [250.0, 200.0];

    if point_in_polygon(person_position, &room) {
        println!("   ✓ 人员在房间内");
    } else {
        println!("   ✗ 人员在房间外");
    }

    // 计算人员到墙壁的最短距离
    let walls = vec![
        ([0.0, 0.0], [500.0, 0.0]),     // 南墙
        ([500.0, 0.0], [500.0, 400.0]), // 东墙
        ([500.0, 400.0], [0.0, 400.0]), // 北墙
        ([0.0, 400.0], [0.0, 0.0]),     // 西墙
    ];

    let mut min_dist = f64::MAX;
    for (start, end) in &walls {
        let dist = point_to_segment_distance(person_position, *start, *end);
        min_dist = min_dist.min(dist);
    }
    println!("   人员到最近墙壁的距离：{:.2}", min_dist);

    println!("\n=== 示例完成 ===");

    Ok(())
}
