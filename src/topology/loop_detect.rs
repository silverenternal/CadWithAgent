//! 回路检测
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
//!
//! 检测几何图元中的闭合回路
//!
//! # 性能优化
//!
//! 使用面遍历算法（Face Traversal），优化策略：
//! 1. 在每个顶点处按极角排序边
//! 2. 通过"始终左转"策略遍历面
//! 3. 时间复杂度 O(n log n)，远优于 DFS 的指数级复杂度

use crate::geometry::{Line, Point, Polygon, Primitive};
use std::collections::{HashMap, HashSet};

/// 查找所有闭合回路
///
/// 使用面遍历算法（Face Traversal）：
/// 1. 构建邻接表，记录每个顶点的边
/// 2. 在每个顶点处，按极角排序边
/// 3. 从未使用的边开始，沿"左转"方向遍历形成回路
pub fn find_closed_loops(primitives: &[Primitive]) -> Vec<Vec<Primitive>> {
    // 提取所有线段
    let lines = extract_lines(primitives);

    if lines.is_empty() {
        return vec![];
    }

    // 构建邻接表：每个顶点 -> [(相邻顶点索引，边索引)]
    let mut adjacency: HashMap<PointKey, Vec<(PointKey, usize)>> = HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        let start_key = point_to_key(line.start);
        let end_key = point_to_key(line.end);

        // 无向边：两个方向都添加
        adjacency.entry(start_key).or_default().push((end_key, i));
        adjacency.entry(end_key).or_default().push((start_key, i));
    }

    // 在每个顶点处，按极角排序邻接边
    // 这样在遍历时可以按顺序找到"下一条边"
    let sorted_adjacency = sort_adjacency_by_angle(&adjacency, &lines);

    // 使用面遍历算法查找回路
    find_faces(&sorted_adjacency, &lines)
}

/// 按极角排序邻接表
///
/// 对于每个顶点，将其邻接边按角度排序
/// 这样在遍历时，给定入边，可以快速找到下一条出边（左转）
fn sort_adjacency_by_angle(
    adjacency: &HashMap<PointKey, Vec<(PointKey, usize)>>,
    _lines: &[Line],
) -> HashMap<PointKey, Vec<(PointKey, usize, f64)>> {
    let mut sorted: HashMap<PointKey, Vec<(PointKey, usize, f64)>> = HashMap::new();

    for (&vertex, neighbors) in adjacency {
        let mut neighbor_angles: Vec<(PointKey, usize, f64)> = neighbors
            .iter()
            .map(|&(neighbor, edge_idx)| {
                let angle = compute_angle(vertex, neighbor);
                (neighbor, edge_idx, angle)
            })
            .collect();

        // 按角度排序
        neighbor_angles.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        sorted.insert(vertex, neighbor_angles);
    }

    sorted
}

/// 计算从 p1 到 p2 的角度（弧度）
fn compute_angle(p1: PointKey, p2: PointKey) -> f64 {
    let dx = (p2.0 - p1.0) as f64;
    let dy = (p2.1 - p1.1) as f64;
    dx.atan2(dy)
}

/// 面遍历算法
///
/// 核心思想：
/// 1. 每条边有两个方向，每个方向属于一个面
/// 2. 从未使用的有向边开始遍历
/// 3. 在每个顶点，选择"最左转"的下一条边
/// 4. 回到起点时形成一个回路
///
/// 注意：此算法会找到所有面（包括顺时针和逆时针）
/// 我们只保留逆时针回路（正面积，内部房间）
fn find_faces(
    sorted_adjacency: &HashMap<PointKey, Vec<(PointKey, usize, f64)>>,
    lines: &[Line],
) -> Vec<Vec<Primitive>> {
    // 记录已使用的有向边：(边索引，起点)
    let mut used_directed_edges: HashSet<(usize, PointKey)> = HashSet::new();
    let mut all_loops = Vec::new();

    // 遍历所有顶点
    for (&start_vertex, neighbors) in sorted_adjacency {
        // 遍历从该顶点出发的所有边
        for &(next_vertex, edge_idx, _angle) in neighbors {
            let directed_edge = (edge_idx, start_vertex);

            if used_directed_edges.contains(&directed_edge) {
                continue;
            }

            // 开始遍历这个面
            let mut path: Vec<usize> = vec![edge_idx];
            let mut current = next_vertex;
            let mut from = start_vertex;

            used_directed_edges.insert(directed_edge);

            // 遍历直到回到起点
            while current != start_vertex {
                // 找到当前顶点的邻接表
                let current_neighbors = &sorted_adjacency[&current];

                // 找到从来边到当前顶点的角度
                let incoming_angle = compute_angle(from, current);

                // 找到下一条边：在角度排序中，找到第一个角度大于入边的边（左转）
                // 如果找不到，选择第一条边（ wraps around）
                let next_edge = find_next_edge(current_neighbors, from, incoming_angle);

                if let Some((next_vertex, edge_idx, _)) = next_edge {
                    let directed_edge = (edge_idx, current);

                    if used_directed_edges.contains(&directed_edge) {
                        // 这条边已经用过，尝试下一条
                        break;
                    }

                    path.push(edge_idx);
                    used_directed_edges.insert(directed_edge);
                    from = current;
                    current = next_vertex;
                } else {
                    break;
                }
            }

            // 检查是否形成有效回路（至少 3 条边，且回到起点）
            if current == start_vertex && path.len() >= 3 {
                let loop_primitives: Vec<Primitive> = path
                    .iter()
                    .map(|&idx| Primitive::Line(lines[idx].clone()))
                    .collect();
                all_loops.push(loop_primitives);
            }
        }
    }

    // 过滤：只保留逆时针回路（正面积）
    // 顺时针回路是外边界或"洞"，逆时针回路是内部房间
    let mut filtered_loops = Vec::new();
    for loop_primitives in all_loops {
        if let Some(area) = compute_loop_signed_area(&loop_primitives) {
            if area > 0.0 {
                filtered_loops.push(loop_primitives);
            }
        }
    }

    // 去重：移除相同的回路（可能因为起点不同而被重复找到）
    deduplicate_loops(filtered_loops)
}

/// 计算回路的有向面积
/// 正值表示逆时针，负值表示顺时针
fn compute_loop_signed_area(loop_primitives: &[Primitive]) -> Option<f64> {
    let vertices: Vec<Point> = loop_primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line.start),
            _ => None,
        })
        .collect();

    if vertices.len() < 3 {
        return None;
    }

    let mut area = 0.0;
    let n = vertices.len();
    for i in 0..n {
        let j = (i + 1) % n;
        area += vertices[i].x * vertices[j].y;
        area -= vertices[j].x * vertices[i].y;
    }
    Some(area * 0.5)
}

/// 去重回路
/// 通过比较顶点集合来识别相同的回路
fn deduplicate_loops(loops: Vec<Vec<Primitive>>) -> Vec<Vec<Primitive>> {
    let mut unique_loops = Vec::new();
    let mut seen_signatures: HashSet<Vec<PointKey>> = HashSet::new();

    for loop_primitives in loops {
        // 创建回路的签名：排序后的顶点集合
        let mut vertices: Vec<PointKey> = loop_primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Line(line) => Some(point_to_key(line.start)),
                _ => None,
            })
            .collect();

        // 排序以创建与起点无关的签名
        vertices.sort();

        if !seen_signatures.contains(&vertices) {
            seen_signatures.insert(vertices.clone());
            unique_loops.push(loop_primitives);
        }
    }

    unique_loops
}

/// 找到下一条边（左转）
///
/// 在已排序的邻接表中，找到第一条角度大于入边的边
/// 如果所有边角度都小于入边，选择第一条边（循环）
fn find_next_edge(
    neighbors: &[(PointKey, usize, f64)],
    from: PointKey,
    incoming_angle: f64,
) -> Option<(PointKey, usize, f64)> {
    // 找到第一条角度大于入边的边
    for &(next, edge_idx, angle) in neighbors {
        // 跳过回边（从来时的方向）
        if next == from {
            continue;
        }

        // 角度差：出边角度 - 入边角度
        let mut angle_diff = angle - incoming_angle;

        // 规范化到 [-π, π]
        while angle_diff > std::f64::consts::PI {
            angle_diff -= 2.0 * std::f64::consts::PI;
        }
        while angle_diff < -std::f64::consts::PI {
            angle_diff += 2.0 * std::f64::consts::PI;
        }

        // 左转意味着角度差为正（逆时针）
        // 选择最小的正角度差（最左转）
        if angle_diff > 0.0 {
            return Some((next, edge_idx, angle));
        }
    }

    // 如果没有正角度差的边，选择最小的负角度差（最右转）
    let mut best: Option<(PointKey, usize, f64)> = None;
    let mut best_diff = 0.0;

    for &(next, edge_idx, angle) in neighbors {
        if next == from {
            continue;
        }

        let mut angle_diff = angle - incoming_angle;
        while angle_diff > std::f64::consts::PI {
            angle_diff -= 2.0 * std::f64::consts::PI;
        }
        while angle_diff < -std::f64::consts::PI {
            angle_diff += 2.0 * std::f64::consts::PI;
        }

        if best.is_none() || angle_diff > best_diff {
            best = Some((next, edge_idx, angle));
            best_diff = angle_diff;
        }
    }

    best
}

/// 从图元中提取线段
fn extract_lines(primitives: &[Primitive]) -> Vec<Line> {
    let mut lines = Vec::new();

    for primitive in primitives {
        match primitive {
            Primitive::Line(line) => {
                lines.push(line.clone());
            }
            Primitive::Polygon(poly) => {
                lines.extend(poly.to_lines());
            }
            Primitive::Rect(rect) => {
                lines.extend(rect.to_polygon().to_lines());
            }
            Primitive::Polyline { points, closed } => {
                for i in 0..points.len().saturating_sub(1) {
                    lines.push(Line::new(points[i], points[i + 1]));
                }
                if *closed && points.len() >= 2 {
                    if let (Some(last), Some(first)) = (points.last(), points.first()) {
                        lines.push(Line::new(*last, *first));
                    }
                }
            }
            _ => {}
        }
    }

    lines
}

/// 将点转换为可哈希的键（考虑浮点精度）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
struct PointKey(i64, i64);

fn point_to_key(point: Point) -> PointKey {
    // 使用固定精度将浮点数转换为整数
    const SCALE: f64 = 10000.0;
    PointKey(
        (point.x * SCALE).round() as i64,
        (point.y * SCALE).round() as i64,
    )
}

/// 查找最大的闭合回路（可能是外墙轮廓）
pub fn find_outer_boundary(primitives: &[Primitive]) -> Option<Polygon> {
    let loops = find_closed_loops(primitives);

    if loops.is_empty() {
        return None;
    }

    // 找到面积最大的回路
    let mut max_area = 0.0;
    let mut boundary = None;

    for loop_primitives in loops {
        let vertices: Vec<Point> = loop_primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Line(line) => Some(line.start),
                _ => None,
            })
            .collect();

        if vertices.len() >= 3 {
            let poly = Polygon::new(vertices);
            let area = poly.area();
            if area > max_area {
                max_area = area;
                boundary = Some(poly);
            }
        }
    }

    boundary
}

/// 查找内部回路（可能是房间）
pub fn find_inner_loops(primitives: &[Primitive]) -> Vec<Polygon> {
    let loops = find_closed_loops(primitives);
    let outer = find_outer_boundary(primitives);

    let mut inner_loops = Vec::new();

    for loop_primitives in loops {
        let vertices: Vec<Point> = loop_primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Line(line) => Some(line.start),
                _ => None,
            })
            .collect();

        if vertices.len() >= 3 {
            let poly = Polygon::new(vertices);

            // 检查是否是内部回路（面积小于外边界）
            if let Some(outer_poly) = &outer {
                if poly.area() < outer_poly.area() {
                    inner_loops.push(poly);
                }
            }
        }
    }

    inner_loops
}
