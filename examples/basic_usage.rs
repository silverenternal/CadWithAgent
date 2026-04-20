//! 基础使用示例
//!
//! 演示如何使用 CadAgent 的基础功能

use cadagent::cot::generator::GeoCotGenerator;
use cadagent::export::dxf::DxfExporter;
use cadagent::geometry::{GeometryMeasurer, GeometryTransform};
use cadagent::prelude::*;
use cadagent::topology::room_detect::RoomDetector;

fn main() -> anyhow::Result<()> {
    println!("=== CadAgent 基础使用示例 ===\n");

    // 1. 创建几何图元
    println!("1. 创建几何图元");
    let room_boundary =
        Polygon::from_coords(vec![[0.0, 0.0], [500.0, 0.0], [500.0, 400.0], [0.0, 400.0]]);
    println!("   房间边界：4 个顶点，面积 = {:.2}", room_boundary.area());

    // 2. 使用测量工具
    println!("\n2. 使用测量工具");
    let mut measurer = GeometryMeasurer::new();

    let length = measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
    println!("   线段长度 (0,0) 到 (3,4): {}", length);

    let area = measurer.measure_area(vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]);
    println!("   正方形面积 (100x100): {}", area);

    let angle = measurer.measure_angle([0.0, 0.0], [1.0, 0.0], [1.0, 1.0]);
    println!("   直角角度：{:.1}°", angle);

    // 3. 使用变换工具
    println!("\n3. 使用变换工具");
    let transform = GeometryTransform;

    let primitives = vec![Primitive::Polygon(room_boundary.clone())];

    // 平移
    let translated = transform.translate(primitives.clone(), 50.0, 50.0);
    println!(
        "   平移后第一个顶点：{:?}",
        if let Primitive::Polygon(p) = &translated[0] {
            p.vertices[0].to_array()
        } else {
            [0.0, 0.0]
        }
    );

    // 旋转
    let rotated = transform.rotate(primitives.clone(), 45.0, [250.0, 200.0]);
    println!("   旋转 45° 后的图元数量：{}", rotated.len());

    // 缩放
    let scaled = transform.scale(primitives.clone(), 0.5, [250.0, 200.0]);
    if let Primitive::Polygon(p) = &scaled[0] {
        println!("   缩放后的面积：{:.2}", p.area());
    }

    // 4. 房间检测
    println!("\n4. 房间检测");
    let detector = RoomDetector;

    // 创建一个简单的户型图（两个房间）
    let floor_plan = vec![
        // 房间 1
        Primitive::Polygon(Polygon::from_coords(vec![
            [0.0, 0.0],
            [400.0, 0.0],
            [400.0, 300.0],
            [0.0, 300.0],
        ])),
        // 房间 2
        Primitive::Polygon(Polygon::from_coords(vec![
            [400.0, 0.0],
            [700.0, 0.0],
            [700.0, 300.0],
            [400.0, 300.0],
        ])),
    ];

    let room_count = detector.count_rooms(floor_plan.clone());
    println!("   检测到的房间数量：{}", room_count);

    let max_area = detector.max_room_area(floor_plan.clone());
    println!("   最大房间面积：{:.2}", max_area);

    // 5. Geo-CoT 生成
    println!("\n5. Geo-CoT 生成");
    let generator = GeoCotGenerator::new();
    let cot_data = generator.generate(&floor_plan, "计算所有房间的面积");

    println!(
        "   感知：{}",
        cot_data.perception.chars().take(100).collect::<String>()
    );
    println!(
        "   推理：{}",
        cot_data.reasoning.chars().take(100).collect::<String>()
    );
    println!("   答案：{}", cot_data.answer);

    // 6. DXF 导出
    println!("\n6. DXF 导出");
    let output_path = std::env::temp_dir().join("cadagent_example.dxf");
    let result = DxfExporter::export(&floor_plan, &output_path)?;
    println!(
        "   导出成功：{} ({} 个图元)",
        result.path, result.entity_count
    );

    // 7. 使用工具注册表
    println!("\n7. 使用工具注册表");
    let registry = ToolRegistry::new();
    let tools = registry.list_tools();
    println!("   可用工具数量：{}", tools.len());
    println!("   部分工具:");
    for tool in tools.iter().take(5) {
        println!("     - {}: {}", tool.name, tool.description);
    }

    // 调用工具
    let result = registry.call(
        "measure_length",
        json!({
            "start": [0.0, 0.0],
            "end": [100.0, 0.0]
        }),
    )?;
    println!("   工具调用结果：{}", result);

    println!("\n=== 示例完成 ===");

    Ok(())
}
