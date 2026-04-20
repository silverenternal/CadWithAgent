//! STEP 文件解析示例
//!
//! 本示例演示如何使用 CadAgent 解析 STEP (ISO 10303) 文件
//! STEP 是产品模型数据交换的国际标准格式，支持 AP203/214/242 协议

use cadagent::parser::step::{StepEntityData, StepParser};

/// 示例 1: 解析简单的 STEP 文件
fn example_basic_step_parsing() {
    println!("\n=== 示例 1: 基础 STEP 解析 ===\n");

    let step_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Example Part'),'2;1');
FILE_NAME('bracket.step','2024-01-01',('Designer'),('Company'),'CADSystem','1.0','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((0.0, 0.0, 0.0));
#2 = CARTESIAN_POINT((100.0, 0.0, 0.0));
#3 = CARTESIAN_POINT((100.0, 50.0, 0.0));
#4 = CARTESIAN_POINT((0.0, 50.0, 0.0));
#5 = CIRCLE((50.0, 25.0), 10.0);
ENDSEC;
END-ISO-10303-21;"#;

    let parser = StepParser::new();
    let model = parser
        .parse_string(step_content)
        .expect("Failed to parse STEP");

    println!("模型名称：{:?}", model.name);
    println!("源软件：{:?}", model.metadata.source_software);
    println!("实体数量：{}", model.entities.len());

    // 转换为图元
    let primitives = model.to_primitives();
    println!("图元数量：{}", primitives.len());

    for (i, prim) in primitives.iter().enumerate() {
        println!("  图元 {}: {:?}", i, prim);
    }
}

/// 示例 2: 解析 3D B-Rep 模型
fn example_3d_brep_parsing() {
    println!("\n=== 示例 2: 3D B-Rep 模型解析 ===\n");

    let step_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('3D Solid'),'2;1');
FILE_NAME('solid.step','','','','','','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((0.0, 0.0, 0.0));
#2 = CARTESIAN_POINT((10.0, 0.0, 0.0));
#3 = CARTESIAN_POINT((10.0, 10.0, 0.0));
#4 = CARTESIAN_POINT((0.0, 10.0, 0.0));
#5 = CARTESIAN_POINT((0.0, 0.0, 10.0));
#6 = CARTESIAN_POINT((10.0, 0.0, 10.0));
#7 = CARTESIAN_POINT((10.0, 10.0, 10.0));
#8 = CARTESIAN_POINT((0.0, 10.0, 10.0));
#9 = MANIFOLD_SOLID_BREP(#10);
#10 = ADVANCED_FACE(#11, (0.0, 0.0, 1.0), .T.);
ENDSEC;
END-ISO-10303-21;"#;

    let parser = StepParser::new();
    let model = parser
        .parse_string(step_content)
        .expect("Failed to parse STEP");

    println!("3D 实体数量：{}", model.entities.len());

    // 统计不同类型的实体
    let mut brep_count = 0;
    let mut face_count = 0;
    let mut point_count = 0;

    for entity in &model.entities {
        match &entity.data {
            StepEntityData::ManifoldSolidBrep { .. } => brep_count += 1,
            StepEntityData::AdvancedFace { .. } => face_count += 1,
            StepEntityData::CartesianPoint3D { .. } => point_count += 1,
            _ => {}
        }
    }

    println!("B-Rep 实体：{}", brep_count);
    println!("高级面：{}", face_count);
    println!("3D 点：{}", point_count);

    // 投影到 2D 进行可视化
    let primitives = model.to_primitives();
    println!("\n2D 投影图元：");
    for (i, prim) in primitives.iter().enumerate() {
        println!("  {}: {:?}", i, prim);
    }
}

/// 示例 3: 解析 NURBS 曲线
fn example_nurbs_parsing() {
    println!("\n=== 示例 3: NURBS 曲线解析 ===\n");

    let step_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('NURBS Curve'),'2;1');
FILE_NAME('nurbs.step','','','','','','');
ENDSEC;
DATA;
#1 = B_SPLINE_CURVE_WITH_KNOTS(3, ((0.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 0.0, 0.0), (3.0, 1.0, 0.0)), .UNSPECIFIED., .F., .F., (1, 1), (0.0, 1.0), .UNSPECIFIED.);
ENDSEC;
END-ISO-10303-21;"#;

    let parser = StepParser::new();
    let model = parser
        .parse_string(step_content)
        .expect("Failed to parse STEP");

    println!("实体数量：{}", model.entities.len());

    for entity in &model.entities {
        if let StepEntityData::NurbsCurve {
            control_points,
            order,
            ..
        } = &entity.data
        {
            println!("NURBS 曲线:");
            println!("  阶数：{}", order);
            println!("  控制点数量：{}", control_points.len());
            for (i, cp) in control_points.iter().enumerate() {
                println!("    CP{}: ({}, {}, {})", i, cp[0], cp[1], cp[2]);
            }
        }
    }
}

/// 示例 4: 从文件加载 STEP
fn example_load_from_file() {
    println!("\n=== 示例 4: 从文件加载 STEP ===\n");

    // 创建一个临时的 STEP 文件用于演示
    let temp_dir = std::env::temp_dir();
    let step_path = temp_dir.join("demo_part.step");

    let step_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Demo Part'),'2;1');
FILE_NAME('demo_part.step','2024-01-01',('User'),('Org'),'CAD','1.0','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((0.0, 0.0));
#2 = LINE((0.0, 0.0), (100.0, 0.0));
#3 = CIRCLE((50.0, 50.0), 25.0);
ENDSEC;
END-ISO-10303-21;"#;

    std::fs::write(&step_path, step_content).expect("Failed to write temp file");

    let parser = StepParser::new().with_tolerance(1e-6).with_debug(false);
    let model = parser.parse(&step_path).expect("Failed to parse STEP file");

    println!("文件路径：{:?}", step_path);
    println!("模型名称：{:?}", model.name);
    println!("实体数量：{}", model.entities.len());

    // 清理临时文件
    let _ = std::fs::remove_file(&step_path);
}

/// 示例 5: 使用分析管线处理 STEP 模型
fn example_with_analysis_pipeline() {
    println!("\n=== 示例 5: 与分析管线集成 ===\n");

    let step_content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Floor Plan'),'2;1');
FILE_NAME('floor.step','','','','','','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((0.0, 0.0));
#2 = CARTESIAN_POINT((10.0, 0.0));
#3 = CARTESIAN_POINT((10.0, 8.0));
#4 = CARTESIAN_POINT((0.0, 8.0));
#5 = LINE((0.0, 0.0), (10.0, 0.0));
#6 = LINE((10.0, 0.0), (10.0, 8.0));
#7 = LINE((10.0, 8.0), (0.0, 8.0));
#8 = LINE((0.0, 8.0), (0.0, 0.0));
ENDSEC;
END-ISO-10303-21;"#;

    let parser = StepParser::new();
    let step_model = parser
        .parse_string(step_content)
        .expect("Failed to parse STEP");

    // 转换为图元
    let primitives = step_model.to_primitives();
    println!("从 STEP 提取的图元数量：{}", primitives.len());

    // 注意：当前 STEP 解析器将 3D 几何投影到 2D 进行分析
    // 完整的 3D 分析功能将在后续版本中实现

    println!("\n提示：要将 STEP 模型用于完整分析管线，请：");
    println!("1. 将图元转换为 AnalysisPipeline 支持的格式");
    println!("2. 使用 geometry_only 模式进行纯几何分析");
    println!("3. 或等待完整的 3D 分析支持");
}

fn main() {
    println!("CadAgent STEP 解析器示例");
    println!("========================\n");

    example_basic_step_parsing();
    example_3d_brep_parsing();
    example_nurbs_parsing();
    example_load_from_file();
    example_with_analysis_pipeline();

    println!("\n=== 所有示例完成 ===\n");
}
