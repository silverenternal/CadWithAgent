//! SVG 几何提取与分析能力测试
//!
//! 使用 CubiCasa5k 数据集中的真实户型图测试：
//! 1. SVG 基元提取能力
//! 2. 几何关系推理能力
//! 3. 房间检测能力
//! 4. tokitai 工具调用能力

use cadagent::analysis::{AnalysisConfig, AnalysisPipeline};
use cadagent::cad_extractor::CadPrimitiveExtractor;
use cadagent::cad_reasoning::GeometricRelationReasoner;
use cadagent::geometry::primitives::Primitive;
use cadagent::topology::room_detect::{detect_rooms, RoomDetectionResult};
use std::collections::HashSet;
use std::fs;

const TEST_SVG_PATH: &str = "data/CubiCasa5k/data/test_samples/1245/model.svg";

#[test]
fn test_extract_primitives_from_cubicasa_svg() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let extractor = CadPrimitiveExtractor::new(Default::default());
    let result = extractor
        .extract_from_svg_string(&svg_content)
        .expect("基元提取失败");

    println!("\n=== 基元提取结果 ===");
    println!("总基元数：{}", result.statistics.total_count);
    println!("  - 点：{}", result.statistics.point_count);
    println!("  - 线段：{}", result.statistics.line_count);
    println!("  - 圆：{}", result.statistics.circle_count);
    println!("  - 弧：{}", result.statistics.arc_count);
    println!("  - 多边形：{}", result.statistics.polygon_count);
    println!("  - 矩形：{}", result.statistics.rect_count);
    println!("  - 多段线：{}", result.statistics.polyline_count);
    println!("  - 文本：{}", result.statistics.text_count);
    println!("  - 贝塞尔曲线：{}", result.statistics.bezier_count);

    assert!(result.statistics.total_count > 0, "应该提取到至少一个基元");
}

#[test]
fn test_analyze_floor_plan_geometry() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let pipeline = AnalysisPipeline::geometry_only(AnalysisConfig::default());

    let result = pipeline
        .inject_from_svg_string(&svg_content, "分析这个户型图的几何结构")
        .expect("分析失败");

    println!("\n=== 几何分析结果 ===");
    println!("提示词长度：{} 字符", result.prompt.full_prompt.len());

    if result.tool_call_chain.is_some() {
        println!("工具调用链：已生成");
    } else {
        println!("工具调用链：未生成");
    }

    assert!(
        result.prompt.full_prompt.len() > 100,
        "提示词应该包含足够的几何信息"
    );
}

// 注意：此测试运行较慢，默认跳过
// 运行方式：cargo test --test svg_geometry_extraction_test test_room_detection -- --include-ignored
#[test]
#[ignore]
fn test_room_detection() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let extractor = CadPrimitiveExtractor::new(Default::default());
    let extract_result = extractor
        .extract_from_svg_string(&svg_content)
        .expect("基元提取失败");

    let room_result: RoomDetectionResult = detect_rooms(&extract_result.primitives);

    println!("\n=== 房间检测结果 ===");
    println!("检测到 {} 个房间", room_result.rooms.len());

    for (i, room) in room_result.rooms.iter().enumerate().take(5) {
        println!(
            "  房间 {}: 面积 = {:?}, 顶点数 = {}",
            i + 1,
            room.area,
            room.boundary.vertices.len()
        );
    }
}

#[test]
fn test_geometric_relations() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let extractor = CadPrimitiveExtractor::new(Default::default());
    let extract_result = extractor
        .extract_from_svg_string(&svg_content)
        .expect("基元提取失败");

    let reasoner = GeometricRelationReasoner::with_defaults();
    let reasoning_result = reasoner.find_all_relations(&extract_result.primitives);

    println!("\n=== 几何关系推理结果 ===");
    println!("检测到 {} 个几何关系", reasoning_result.relations.len());

    let mut parallel_count = 0;
    let mut perpendicular_count = 0;
    let mut connected_count = 0;

    for relation in &reasoning_result.relations {
        match relation {
            cadagent::cad_reasoning::GeometricRelation::Parallel { .. } => parallel_count += 1,
            cadagent::cad_reasoning::GeometricRelation::Perpendicular { .. } => {
                perpendicular_count += 1
            }
            cadagent::cad_reasoning::GeometricRelation::Connected { .. } => connected_count += 1,
            _ => {}
        }
    }

    println!("  - 平行关系：{}", parallel_count);
    println!("  - 垂直关系：{}", perpendicular_count);
    println!("  - 连接关系：{}", connected_count);
}

#[test]
fn test_svg_contains_room_labels() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let extractor = CadPrimitiveExtractor::new(Default::default());
    let result = extractor
        .extract_from_svg_string(&svg_content)
        .expect("基元提取失败");

    let mut found_kitchen = false;
    let mut found_bedroom = false;
    let mut found_bathroom = false;
    let mut found_living_room = false;

    for primitive in &result.primitives {
        if let Primitive::Text { content, .. } = primitive {
            let text_upper = content.to_uppercase();
            if text_upper.contains("KITCHEN") || text_upper.contains("K") {
                found_kitchen = true;
            }
            if text_upper.contains("BEDROOM") || text_upper.contains("MH") {
                found_bedroom = true;
            }
            if text_upper.contains("BATH") || text_upper.contains("WC") || text_upper.contains("KH")
            {
                found_bathroom = true;
            }
            if text_upper.contains("LIVING") || text_upper.contains("OH") {
                found_living_room = true;
            }
        }
    }

    println!("\n=== 房间标签检测 ===");
    println!("厨房标签：{}", if found_kitchen { "✓" } else { "✗" });
    println!("卧室标签：{}", if found_bedroom { "✓" } else { "✗" });
    println!("卫生间标签：{}", if found_bathroom { "✓" } else { "✗" });
    println!("客厅标签：{}", if found_living_room { "✓" } else { "✗" });
}

#[test]
fn test_coordinate_range() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let extractor = CadPrimitiveExtractor::new(Default::default());
    let result = extractor
        .extract_from_svg_string(&svg_content)
        .expect("基元提取失败");

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for primitive in &result.primitives {
        match primitive {
            Primitive::Line(line) => {
                min_x = min_x.min(line.start.x).min(line.end.x);
                max_x = max_x.max(line.start.x).max(line.end.x);
                min_y = min_y.min(line.start.y).min(line.end.y);
                max_y = max_y.max(line.start.y).max(line.end.y);
            }
            Primitive::Polygon(poly) => {
                for v in &poly.vertices {
                    min_x = min_x.min(v.x);
                    max_x = max_x.max(v.x);
                    min_y = min_y.min(v.y);
                    max_y = max_y.max(v.y);
                }
            }
            _ => {}
        }
    }

    println!("\n=== 坐标范围 ===");
    println!("X 范围：{:.2} - {:.2}", min_x, max_x);
    println!("Y 范围：{:.2} - {:.2}", min_y, max_y);
    println!("宽度：{:.2}", max_x - min_x);
    println!("高度：{:.2}", max_y - min_y);
}

#[test]
fn test_wall_detection() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let extractor = CadPrimitiveExtractor::new(Default::default());
    let result = extractor
        .extract_from_svg_string(&svg_content)
        .expect("基元提取失败");

    // 墙体在 SVG 中以 polygon 形式存在，通过统计多边形数量来推断
    let polygon_count = result.statistics.polygon_count;
    let rect_count = result.statistics.rect_count;

    println!("\n=== 墙体检测结果 ===");
    println!("多边形：{} 个", polygon_count);
    println!("矩形：{} 个", rect_count);

    // 户型图应该包含多个多边形（墙体、房间等）
    assert!(polygon_count > 10, "应该检测到足够的多边形结构");
}

#[test]
fn test_furniture_detection() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let extractor = CadPrimitiveExtractor::new(Default::default());
    let result = extractor
        .extract_from_svg_string(&svg_content)
        .expect("基元提取失败");

    let mut furniture_count = 0;
    let mut furniture_types: HashSet<String> = HashSet::new();

    // 通过文本标签检测家具
    for primitive in &result.primitives {
        if let Primitive::Text { content, .. } = primitive {
            let text_upper = content.to_uppercase();

            if text_upper.contains("SINK") {
                furniture_count += 1;
                furniture_types.insert("Sink".to_string());
            }
            if text_upper.contains("TOILET") || text_upper.contains("WC") {
                furniture_count += 1;
                furniture_types.insert("Toilet".to_string());
            }
            if text_upper.contains("CL") || text_upper.contains("CLOSET") {
                furniture_count += 1;
                furniture_types.insert("Closet".to_string());
            }
            if text_upper.contains("CB") || text_upper.contains("CABINET") {
                furniture_count += 1;
                furniture_types.insert("Cabinet".to_string());
            }
            if text_upper.contains("STOVE") || text_upper.contains("INT") {
                furniture_count += 1;
                furniture_types.insert("Stove".to_string());
            }
            if text_upper.contains("REF") || text_upper.contains("REFRIGERATOR") {
                furniture_count += 1;
                furniture_types.insert("Refrigerator".to_string());
            }
            if text_upper.contains("SHOWER") {
                furniture_count += 1;
                furniture_types.insert("Shower".to_string());
            }
            if text_upper.contains("SB") || text_upper.contains("BENCH") {
                furniture_count += 1;
                furniture_types.insert("Bench".to_string());
            }
        }
    }

    println!("\n=== 家具检测 ===");
    println!("检测到的家具标签：{} 个", furniture_count);
    println!("家具类型：{:?}", furniture_types);
}

#[test]
fn test_analysis_pipeline_with_statistics() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    let pipeline = AnalysisPipeline::geometry_only(AnalysisConfig::default());

    let result = pipeline
        .inject_from_svg_string(&svg_content, "统计这个户型的所有几何信息")
        .expect("分析失败");

    println!("\n=== 完整分析管线测试 ===");

    let prompt = &result.prompt.full_prompt;

    let stats = [
        ("基元", prompt.contains("primitives")),
        (
            "几何关系",
            prompt.contains("relations") || prompt.contains("几何关系"),
        ),
        ("房间", prompt.contains("rooms") || prompt.contains("房间")),
        ("墙体", prompt.contains("walls") || prompt.contains("墙")),
        (
            "门窗",
            prompt.contains("doors") || prompt.contains("windows"),
        ),
    ];

    println!("提示词包含的信息：");
    for (name, contains) in &stats {
        println!("  - {}: {}", name, if *contains { "✓" } else { "✗" });
    }

    let enabled_count = stats.iter().filter(|(_, c)| *c).count();
    assert!(
        enabled_count >= 2,
        "提示词应该包含至少 2 种几何信息类型，但只找到 {} 种",
        enabled_count
    );
}

#[test]
fn test_door_window_detection_in_svg() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    // 直接解析 SVG 查找门窗信息
    let mut door_count = 0;
    let mut window_count = 0;

    for line in svg_content.lines() {
        if line.contains("class=\"Door") || line.contains("class='Door") {
            door_count += 1;
        }
        if line.contains("class=\"Window") || line.contains("class='Window") {
            window_count += 1;
        }
    }

    println!("\n=== SVG 门窗检测 ===");
    println!("门：{} 个", door_count);
    println!("窗：{} 个", window_count);

    assert!(door_count > 0, "应该检测到门");
    assert!(window_count > 0, "应该检测到窗");
}

#[test]
fn test_full_pipeline_summary() {
    let svg_content = fs::read_to_string(TEST_SVG_PATH).expect("无法读取 SVG 文件");

    println!("\n========== 完整测试流程总结 ==========");

    // 1. 基元提取
    let extractor = CadPrimitiveExtractor::new(Default::default());
    let extract_result = extractor.extract_from_svg_string(&svg_content).unwrap();
    println!(
        "\n1. 基元提取：{} 个基元",
        extract_result.statistics.total_count
    );

    // 2. 几何关系推理
    let reasoner = GeometricRelationReasoner::with_defaults();
    let reasoning_result = reasoner.find_all_relations(&extract_result.primitives);
    println!("2. 几何关系：{} 个关系", reasoning_result.relations.len());

    // 3. 房间检测
    let room_result: RoomDetectionResult = detect_rooms(&extract_result.primitives);
    println!("3. 房间检测：{} 个房间", room_result.rooms.len());

    // 4. 分析管线
    let pipeline = AnalysisPipeline::geometry_only(AnalysisConfig::default());
    let analysis_result = pipeline
        .inject_from_svg_string(&svg_content, "总结")
        .unwrap();
    println!(
        "4. 分析管线：提示词长度 {} 字符",
        analysis_result.prompt.full_prompt.len()
    );

    println!("\n========== 测试完成 ==========");
}
