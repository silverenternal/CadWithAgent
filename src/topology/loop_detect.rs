//! 回路检测
//!
//! 检测几何图元中的闭合回路

use crate::geometry::{Point, Line, Primitive, Polygon};
use std::collections::{HashMap, HashSet};

/// 查找所有闭合回路
pub fn find_closed_loops(primitives: &[Primitive]) -> Vec<Vec<Primitive>> {
    // 提取所有线段
    let lines = extract_lines(primitives);
    
    if lines.is_empty() {
        return vec![];
    }

    // 构建邻接表
    let mut adjacency: HashMap<PointKey, Vec<usize>> = HashMap::new();
    
    for (i, line) in lines.iter().enumerate() {
        let start_key = point_to_key(line.start);
        let end_key = point_to_key(line.end);
        
        adjacency.entry(start_key).or_default().push(i);
        adjacency.entry(end_key).or_default().push(i);
    }

    // 使用 DFS 查找回路
    let mut visited = HashSet::new();
    let mut loops = Vec::new();
    
    for start_point in adjacency.keys() {
        if visited.contains(start_point) {
            continue;
        }
        
        let mut path = Vec::new();
        let mut path_lines = Vec::new();
        
        if dfs_find_loop(
            *start_point,
            *start_point,
            &mut path,
            &mut path_lines,
            &lines,
            &adjacency,
            &mut visited,
        )
            && path_lines.len() >= 3 {
                let loop_primitives: Vec<Primitive> = path_lines
                    .iter()
                    .map(|&i| Primitive::Line(lines[i].clone()))
                    .collect();
                loops.push(loop_primitives);
            }
    }

    loops
}

fn dfs_find_loop(
    current: PointKey,
    start: PointKey,
    path: &mut Vec<PointKey>,
    path_lines: &mut Vec<usize>,
    lines: &[Line],
    adjacency: &HashMap<PointKey, Vec<usize>>,
    visited: &mut HashSet<PointKey>,
) -> bool {
    path.push(current);
    visited.insert(current);

    if let Some(_line_idx) = adjacency.get(&current) {
        for &neighbor_line_idx in adjacency.get(&current).unwrap_or(&vec![]) {
            let line = &lines[neighbor_line_idx];
            let neighbor = if point_to_key(line.start) == current {
                point_to_key(line.end)
            } else {
                point_to_key(line.start)
            };

            if neighbor == start && path.len() >= 3 {
                path_lines.push(neighbor_line_idx);
                return true;
            }

            if !visited.contains(&neighbor) {
                path_lines.push(neighbor_line_idx);
                if dfs_find_loop(neighbor, start, path, path_lines, lines, adjacency, visited) {
                    return true;
                }
            }
        }
    }

    path.pop();
    false
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
                for i in 0..points.len() - 1 {
                    lines.push(Line::new(points[i], points[i + 1]));
                }
                if *closed && points.len() >= 2 {
                    lines.push(Line::new(*points.last().unwrap(), points[0]));
                }
            }
            _ => {}
        }
    }

    lines
}

/// 将点转换为可哈希的键（考虑浮点精度）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PointKey(i64, i64);

fn point_to_key(point: Point) -> PointKey {
    // 使用固定精度将浮点数转换为整数
    const SCALE: f64 = 10000.0;
    PointKey((point.x * SCALE).round() as i64, (point.y * SCALE).round() as i64)
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
