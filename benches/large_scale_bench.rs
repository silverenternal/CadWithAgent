//! 大规模场景性能基准测试
//!
//! 测试 1000+ 几何基元的性能表现

use cadagent::cad_reasoning::GeometricRelationReasoner;
use cadagent::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// 创建大型楼层平面（真实场景）
fn create_large_floor_plan(num_rooms: usize) -> Vec<Primitive> {
    let mut primitives = Vec::new();

    // 外墙
    let building_width = 1000.0;
    let building_height = 800.0;
    primitives.extend(create_rectangle(0.0, 0.0, building_width, building_height));

    // 内部房间网格 - 限制最小房间尺寸
    let max_rooms_per_dim = std::cmp::min((num_rooms as f64).sqrt() as usize, 8);
    let rooms_per_row = max_rooms_per_dim;
    let rooms_per_col = max_rooms_per_dim;
    let room_width = building_width / rooms_per_row as f64;
    let room_height = building_height / rooms_per_col as f64;

    for row in 0..rooms_per_row {
        for col in 0..rooms_per_col {
            let x = col as f64 * room_width;
            let y = row as f64 * room_height;

            // 房间隔墙（留出门窗开口）
            if col < rooms_per_col - 1 {
                primitives.push(Primitive::Line(Line::from_coords_unchecked(
                    [x + room_width, y + 50.0],
                    [x + room_width, y + room_height - 50.0],
                )));
            }

            if row < rooms_per_row - 1 {
                primitives.push(Primitive::Line(Line::from_coords_unchecked(
                    [x + 50.0, y + room_height],
                    [x + room_width - 50.0, y + room_height],
                )));
            }
        }
    }

    // 添加门窗
    for row in 0..rooms_per_row {
        for col in 0..rooms_per_col {
            let x = col as f64 * room_width;
            let y = row as f64 * room_height;

            // 窗户（外墙上的小线段）
            if col == 0 || col == rooms_per_col - 1 || row == 0 || row == rooms_per_row - 1 {
                primitives.push(Primitive::Line(Line::from_coords_unchecked(
                    [x + room_width / 2.0 - 40.0, y],
                    [x + room_width / 2.0 + 40.0, y],
                )));
            }
        }
    }

    primitives
}

/// 创建密集机械零件图
fn create_dense_mechanical_part() -> Vec<Primitive> {
    let mut primitives = Vec::new();

    // 外轮廓
    let center = Point::new(500.0, 500.0);
    let outer_radius = 400.0;

    // 多边形外轮廓
    let num_sides = 20;
    for i in 0..num_sides {
        let angle1 = (2.0 * std::f64::consts::PI * i as f64) / num_sides as f64;
        let angle2 = (2.0 * std::f64::consts::PI * (i + 1) as f64) / num_sides as f64;

        let x1 = center.x + outer_radius * angle1.cos();
        let y1 = center.y + outer_radius * angle1.sin();
        let x2 = center.x + outer_radius * angle2.cos();
        let y2 = center.y + outer_radius * angle2.sin();

        primitives.push(Primitive::Line(Line::from_coords([x1, y1], [x2, y2])));
    }

    // 内部孔洞（圆）
    let bolt_circle_radius = 300.0;
    let num_bolts = 16;
    for i in 0..num_bolts {
        let angle = (2.0 * std::f64::consts::PI * i as f64) / num_bolts as f64;
        let hole_center = Point::new(
            center.x + bolt_circle_radius * angle.cos(),
            center.y + bolt_circle_radius * angle.sin(),
        );
        primitives.push(Primitive::Circle(Circle::new(hole_center, 15.0)));
    }

    // 内部加强筋（辐射状线段）
    for i in 0..num_bolts {
        let angle = (2.0 * std::f64::consts::PI * i as f64) / num_bolts as f64;
        let inner_radius = 100.0;
        let outer_rad = 350.0;

        let x1 = center.x + inner_radius * angle.cos();
        let y1 = center.y + inner_radius * angle.sin();
        let x2 = center.x + outer_rad * angle.cos();
        let y2 = center.y + outer_rad * angle.sin();

        primitives.push(Primitive::Line(Line::from_coords([x1, y1], [x2, y2])));
    }

    // 同心圆环
    for radius in [150.0, 200.0, 250.0, 300.0].iter() {
        primitives.push(Primitive::Circle(Circle::new(center, *radius)));
    }

    primitives
}

/// 创建参数化建筑平面
fn create_parametric_building(num_floors: usize, rooms_per_floor: usize) -> Vec<Primitive> {
    let mut primitives = Vec::new();

    let floor_height = 300.0;
    let building_width = 800.0;
    let building_depth = 600.0;

    for floor in 0..num_floors {
        let z = floor as f64 * floor_height;

        // 楼板
        primitives.extend(create_rectangle(0.0, z, building_width, building_depth));

        // 房间分隔
        let room_width = building_width / rooms_per_floor as f64;
        for room in 0..rooms_per_floor {
            let x = room as f64 * room_width;

            // 房间隔墙
            if room > 0 {
                primitives.push(Primitive::Line(Line::from_coords(
                    [x, z],
                    [x, z + building_depth],
                )));
            }
        }
    }

    primitives
}

fn create_rectangle(x: f64, y: f64, width: f64, height: f64) -> Vec<Primitive> {
    vec![
        Primitive::Line(Line::from_coords([x, y], [x + width, y])),
        Primitive::Line(Line::from_coords([x + width, y], [x + width, y + height])),
        Primitive::Line(Line::from_coords([x + width, y + height], [x, y + height])),
        Primitive::Line(Line::from_coords([x, y + height], [x, y])),
    ]
}

fn bench_large_floor_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_floor_plan");

    for num_rooms in [100, 400, 900, 1600].iter() {
        let primitives = create_large_floor_plan(*num_rooms);
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_rooms", num_rooms)),
            &primitives,
            |b, primitives| b.iter(|| reasoner.find_all_relations(black_box(primitives))),
        );
    }
    group.finish();
}

fn bench_mechanical_part(c: &mut Criterion) {
    let primitives = create_dense_mechanical_part();
    let reasoner = GeometricRelationReasoner::with_defaults();

    c.bench_function("dense_mechanical_part", |b| {
        b.iter(|| reasoner.find_all_relations(black_box(&primitives)))
    });
}

fn bench_parametric_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("parametric_building");

    for num_floors in [5, 10, 20, 50].iter() {
        let primitives = create_parametric_building(*num_floors, 8);
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_floors", num_floors)),
            &primitives,
            |b, primitives| b.iter(|| reasoner.find_all_relations(black_box(primitives))),
        );
    }
    group.finish();
}

fn bench_scalability_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability_analysis");

    // 分析性能随基元数量的扩展
    for primitive_count in [100, 200, 500, 1000, 2000, 5000].iter() {
        let primitives = create_large_floor_plan(*primitive_count / 4); // 近似
        let reasoner = GeometricRelationReasoner::with_defaults();

        group.bench_with_input(
            BenchmarkId::from_parameter(primitive_count),
            &primitives,
            |b, primitives| b.iter(|| reasoner.find_all_relations(black_box(primitives))),
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_large_floor_plan,
    bench_mechanical_part,
    bench_parametric_building,
    bench_scalability_analysis,
);

criterion_main!(benches);
