//! 门窗检测
//!
//! 检测墙体中的门和窗户

use crate::geometry::{Door, DoorDirection, Line, Point, Primitive, Window};

/// 检测墙中的门
pub fn detect_doors_in_wall(wall: &Line, primitives: &[Primitive]) -> Vec<Door> {
    let mut doors = Vec::new();

    // 方法 1: 查找文本标记
    for prim in primitives {
        if let Primitive::Text {
            content, position, ..
        } = prim
        {
            let content_lower = content.to_lowercase();
            if content_lower.contains("门")
                || content_lower.contains("door")
                || content_lower == "d"
            {
                // 检查是否在墙附近
                let dist = distance_point_to_line(position, wall);
                if dist < 100.0 {
                    doors.push(Door {
                        position: *position,
                        width: estimate_door_width(wall),
                        direction: DoorDirection::Inward,
                    });
                }
            }
        }
    }

    // 方法 2: 查找墙上的缺口
    if let Some(door) = detect_door_gap(wall, primitives) {
        doors.push(door);
    }

    doors
}

/// 检测墙中的窗户
pub fn detect_windows_in_wall(wall: &Line, primitives: &[Primitive]) -> Vec<Window> {
    let mut windows = Vec::new();

    // 方法 1: 查找文本标记
    for prim in primitives {
        if let Primitive::Text {
            content, position, ..
        } = prim
        {
            let content_lower = content.to_lowercase();
            if content_lower.contains("窗")
                || content_lower.contains("window")
                || content_lower == "w"
            {
                // 检查是否在墙附近
                let dist = distance_point_to_line(position, wall);
                if dist < 100.0 {
                    windows.push(Window {
                        position: *position,
                        width: estimate_window_width(wall),
                        height: 1500.0,
                    });
                }
            }
        }
    }

    // 方法 2: 查找墙上的双线或特殊标记
    if let Some(window) = detect_window_lines(wall, primitives) {
        windows.push(window);
    }

    windows
}

/// 检测墙上的缺口（门的特征）
fn detect_door_gap(wall: &Line, primitives: &[Primitive]) -> Option<Door> {
    // 查找与墙平行但不连续的线段
    let parallel_lines: Vec<&Line> = primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line),
            _ => None,
        })
        .filter(|line| {
            // 检查是否平行
            is_parallel(wall, line, 5.0) // 5 度容差
        })
        .collect();

    if parallel_lines.is_empty() {
        return None;
    }

    // 检查是否有缺口
    let gap = find_gap_in_wall(wall, &parallel_lines);

    if let Some((gap_center, gap_width)) = gap {
        // 标准门宽约 800-1000mm
        if gap_width > 700.0 && gap_width < 1200.0 {
            return Some(Door {
                position: gap_center,
                width: gap_width,
                direction: DoorDirection::Inward,
            });
        }
    }

    None
}

/// 检测墙上的窗户（双线特征）
fn detect_window_lines(wall: &Line, primitives: &[Primitive]) -> Option<Window> {
    // 查找墙内的平行双线
    let parallel_lines: Vec<&Line> = primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line),
            _ => None,
        })
        .filter(|line| is_parallel(wall, line, 5.0))
        .collect();

    if parallel_lines.len() < 2 {
        return None;
    }

    // 查找距离合适的双线
    for i in 0..parallel_lines.len() {
        for j in (i + 1)..parallel_lines.len() {
            let dist = line_distance(parallel_lines[i], parallel_lines[j]);
            // 窗户厚度约 200-300mm
            if dist > 150.0 && dist < 400.0 {
                let mid1 = parallel_lines[i].midpoint();
                let mid2 = parallel_lines[j].midpoint();
                let center = Point::new((mid1.x + mid2.x) / 2.0, (mid1.y + mid2.y) / 2.0);
                return Some(Window {
                    position: center,
                    width: parallel_lines[i].length(),
                    height: dist,
                });
            }
        }
    }

    None
}

/// 检查两条线段是否平行
fn is_parallel(line1: &Line, line2: &Line, tolerance_degrees: f64) -> bool {
    let dir1 = line1.direction();
    let dir2 = line2.direction();

    // 计算方向向量的叉积
    let cross = dir1.x * dir2.y - dir1.y * dir2.x;
    let angle_diff = (cross).abs().asin().to_degrees();

    angle_diff < tolerance_degrees
}

/// 计算点到线段的距离
fn distance_point_to_line(point: &Point, line: &Line) -> f64 {
    let a = (point.x - line.start.x) * (line.end.y - line.start.y)
        - (point.y - line.start.y) * (line.end.x - line.start.x);
    let b = line.length();

    if b == 0.0 {
        return point.distance(&line.start);
    }

    a.abs() / b
}

/// 计算两条平行线之间的距离
fn line_distance(line1: &Line, line2: &Line) -> f64 {
    distance_point_to_line(&line2.start, line1)
}

/// 在墙中查找缺口
fn find_gap_in_wall(wall: &Line, parallel_lines: &[&Line]) -> Option<(Point, f64)> {
    // 将平行线段投影到墙上，查找缺口
    let mut projections: Vec<(f64, f64)> = parallel_lines
        .iter()
        .filter_map(|line| {
            // 计算线段在墙上的投影区间
            let t1 = project_point_on_line(&line.start, wall);
            let t2 = project_point_on_line(&line.end, wall);

            if t1.is_finite() && t2.is_finite() {
                Some((t1.min(t2), t1.max(t2)))
            } else {
                None
            }
        })
        .collect();

    if projections.is_empty() {
        return None;
    }

    // 排序
    projections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // 查找缺口
    for i in 0..projections.len() - 1 {
        let gap_start = projections[i].1;
        let gap_end = projections[i + 1].0;

        if gap_end - gap_start > 50.0 {
            // 找到缺口
            let gap_center_t = (gap_start + gap_end) / 2.0;
            let gap_width = gap_end - gap_start;

            let gap_center = Point::new(
                wall.start.x + gap_center_t * (wall.end.x - wall.start.x),
                wall.start.y + gap_center_t * (wall.end.y - wall.start.y),
            );

            return Some((gap_center, gap_width));
        }
    }

    None
}

/// 将点投影到线上，返回参数 t
fn project_point_on_line(point: &Point, line: &Line) -> f64 {
    let dx = line.end.x - line.start.x;
    let dy = line.end.y - line.start.y;
    let len_sq = dx * dx + dy * dy;

    if len_sq == 0.0 {
        return 0.0;
    }

    ((point.x - line.start.x) * dx + (point.y - line.start.y) * dy) / len_sq
}

/// 估算门宽
fn estimate_door_width(_wall: &Line) -> f64 {
    900.0 // 标准门宽 900mm
}

/// 估算窗户宽度
fn estimate_window_width(wall: &Line) -> f64 {
    // 窗户宽度约为墙长的 1/3 到 1/2
    (wall.length() * 0.4).clamp(1000.0, 2000.0)
}

/// 使用 tokitai 工具封装的门窗检测器
#[derive(Default, Clone)]
pub struct DoorWindowDetector;

use tokitai::tool;

#[tool]
impl DoorWindowDetector {
    /// 检测门
    #[tool]
    pub fn detect_doors(
        &self,
        wall_start: [f64; 2],
        wall_end: [f64; 2],
        primitives: Vec<Primitive>,
    ) -> Vec<Door> {
        let wall = Line::from_coords(wall_start, wall_end);
        detect_doors_in_wall(&wall, &primitives)
    }

    /// 检测窗户
    #[tool]
    pub fn detect_windows(
        &self,
        wall_start: [f64; 2],
        wall_end: [f64; 2],
        primitives: Vec<Primitive>,
    ) -> Vec<Window> {
        let wall = Line::from_coords(wall_start, wall_end);
        detect_windows_in_wall(&wall, &primitives)
    }

    /// 检查墙是否有门
    #[tool]
    pub fn has_door(
        &self,
        wall_start: [f64; 2],
        wall_end: [f64; 2],
        primitives: Vec<Primitive>,
    ) -> bool {
        let wall = Line::from_coords(wall_start, wall_end);
        !detect_doors_in_wall(&wall, &primitives).is_empty()
    }

    /// 检查墙是否有窗
    #[tool]
    pub fn has_window(
        &self,
        wall_start: [f64; 2],
        wall_end: [f64; 2],
        primitives: Vec<Primitive>,
    ) -> bool {
        let wall = Line::from_coords(wall_start, wall_end);
        !detect_windows_in_wall(&wall, &primitives).is_empty()
    }
}
