//! 门窗检测
//!
//! 检测墙体中的门和窗户
//!
//! # 角度单位说明
//!
//! 本模块使用**弧度**作为角度单位，与 `GeometryConfig` 保持一致。
//! 旧代码中的 `DOOR_WINDOW_PARALLEL_TOLERANCE_DEG` 已标记为 deprecated。
//!
//! # 内部 API
//!
//! 以下函数为内部实现细节，外部模块不应直接调用：
//! - `detect_doors_in_wall` - 通过 `RoomDetector` 或 `DoorWindowDetector` 调用
//! - `detect_windows_in_wall` - 通过 `RoomDetector` 或 `DoorWindowDetector` 调用
//! - `detect_door_gap` - 内部实现细节
//! - `detect_window_lines` - 内部实现细节
//! - `distance_point_to_line` - 内部实现细节

use crate::geometry::{Door, DoorDirection, Line, Point, Primitive, Window};
use crate::topology::room_detect::config;

/// 角度转换工具
#[allow(dead_code)]
fn degrees_to_radians(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

/// 检测墙中的门（内部 API）
///
/// 此函数为内部实现细节，建议通过 `DoorWindowDetector` 工具类调用
pub(crate) fn detect_doors_in_wall(wall: &Line, primitives: &[Primitive]) -> Vec<Door> {
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
                if dist < config::TEXT_MARK_DETECTION_DIST {
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

/// 检测墙中的窗户（内部 API）
///
/// 此函数为内部实现细节，建议通过 `DoorWindowDetector` 工具类调用
pub(crate) fn detect_windows_in_wall(wall: &Line, primitives: &[Primitive]) -> Vec<Window> {
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
                if dist < config::TEXT_MARK_DETECTION_DIST {
                    windows.push(Window {
                        position: *position,
                        width: config::STANDARD_WINDOW_WIDTH, // 使用标准窗宽
                        height: config::STANDARD_WINDOW_HEIGHT,
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

/// 检测墙上的缺口（门的特征）（内部 API）
///
/// 此函数为内部实现细节，不推荐外部直接调用
pub(crate) fn detect_door_gap(wall: &Line, primitives: &[Primitive]) -> Option<Door> {
    // 查找与墙平行但不连续的线段
    let parallel_lines: Vec<&Line> = primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line),
            _ => None,
        })
        .filter(|line| {
            // 检查是否平行（使用弧度容差）
            is_parallel_rad(wall, line, config::DOOR_WINDOW_PARALLEL_TOLERANCE_RAD)
        })
        .collect();

    if parallel_lines.is_empty() {
        return None;
    }

    // 检查是否有缺口
    let gap = find_gap_in_wall(wall, &parallel_lines);

    if let Some((gap_center, gap_width)) = gap {
        // 标准门宽约 800-1000mm
        if gap_width > config::MIN_DOOR_WIDTH && gap_width < config::MAX_DOOR_WIDTH {
            return Some(Door {
                position: gap_center,
                width: gap_width,
                direction: DoorDirection::Inward,
            });
        }
    }

    None
}

/// 检测墙上的窗户（双线特征）（内部 API）
///
/// 此函数为内部实现细节，不推荐外部直接调用
pub(crate) fn detect_window_lines(wall: &Line, primitives: &[Primitive]) -> Option<Window> {
    // 查找墙内的平行双线
    let parallel_lines: Vec<&Line> = primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line),
            _ => None,
        })
        .filter(|line| is_parallel_rad(wall, line, config::DOOR_WINDOW_PARALLEL_TOLERANCE_RAD))
        .collect();

    if parallel_lines.len() < 2 {
        return None;
    }

    // 查找距离合适的双线
    for i in 0..parallel_lines.len() {
        for j in (i + 1)..parallel_lines.len() {
            let dist = line_distance(parallel_lines[i], parallel_lines[j]);
            // 窗户厚度约 200-300mm
            if dist > config::MIN_WINDOW_LINE_DIST && dist < config::MAX_WINDOW_LINE_DIST {
                let mid1 = parallel_lines[i].midpoint();
                let mid2 = parallel_lines[j].midpoint();
                let center =
                    Point::new(f64::midpoint(mid1.x, mid2.x), f64::midpoint(mid1.y, mid2.y));
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

/// 检查两条线段是否平行（使用弧度容差）
fn is_parallel_rad(line1: &Line, line2: &Line, tolerance_radians: f64) -> bool {
    let dir1 = line1.direction();
    let dir2 = line2.direction();

    // 计算方向向量的叉积
    let cross = dir1.x * dir2.y - dir1.y * dir2.x;
    // Clamp to [-1.0, 1.0] to avoid NaN from asin
    let angle_diff = (cross).abs().clamp(0.0, 1.0).asin();

    angle_diff < tolerance_radians
}

/// 检查两条线段是否平行（使用角度容差，已废弃）
#[deprecated(since = "0.1.1", note = "使用 is_parallel_rad 代替")]
#[allow(dead_code)]
fn is_parallel(line1: &Line, line2: &Line, tolerance_degrees: f64) -> bool {
    let dir1 = line1.direction();
    let dir2 = line2.direction();

    // 计算方向向量的叉积
    let cross = dir1.x * dir2.y - dir1.y * dir2.x;
    // Clamp to [-1.0, 1.0] to avoid NaN from asin
    let angle_diff = (cross).abs().clamp(0.0, 1.0).asin().to_degrees();

    angle_diff < tolerance_degrees
}

/// 计算点到线段的距离（内部 API）
///
/// 此函数为内部实现细节，不推荐外部直接调用
pub(crate) fn distance_point_to_line(point: &Point, line: &Line) -> f64 {
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
    projections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // 查找缺口
    for i in 0..projections.len() - 1 {
        let gap_start = projections[i].1;
        let gap_end = projections[i + 1].0;

        if gap_end - gap_start > config::WALL_GAP_DIST_THRESHOLD {
            // 找到缺口
            let gap_center_t = f64::midpoint(gap_start, gap_end);
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
    config::STANDARD_DOOR_WIDTH // 标准门宽 900mm
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== distance_point_to_line 测试 =====

    #[test]
    fn test_distance_point_to_line_on_line() {
        let line = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let point = Point::new(5.0, 0.0);
        let dist = distance_point_to_line(&point, &line);
        assert!(dist < 0.001);
    }

    #[test]
    fn test_distance_point_to_line_perpendicular() {
        let line = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let point = Point::new(5.0, 3.0);
        let dist = distance_point_to_line(&point, &line);
        assert!((dist - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_distance_point_to_line_diagonal() {
        let line = Line::from_coords([0.0, 0.0], [3.0, 4.0]);
        let point = Point::new(0.0, 0.0);
        let dist = distance_point_to_line(&point, &line);
        assert!(dist < 0.001);
    }

    #[test]
    fn test_distance_point_to_line_zero_length() {
        let line = Line::from_coords_unchecked([5.0, 5.0], [5.0, 5.0]);
        let point = Point::new(8.0, 9.0);
        let dist = distance_point_to_line(&point, &line);
        assert!((dist - 5.0).abs() < 0.001); // distance from (5,5) to (8,9) = 5
    }

    // ===== is_parallel_rad 测试 =====

    #[test]
    fn test_is_parallel_horizontal_lines() {
        let line1 = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let line2 = Line::from_coords([0.0, 5.0], [10.0, 5.0]);
        // 5 度 ≈ 0.087 弧度
        assert!(is_parallel_rad(&line1, &line2, 5.0_f64.to_radians()));
    }

    #[test]
    fn test_is_parallel_vertical_lines() {
        let line1 = Line::from_coords([0.0, 0.0], [0.0, 10.0]);
        let line2 = Line::from_coords([5.0, 0.0], [5.0, 10.0]);
        assert!(is_parallel_rad(&line1, &line2, 5.0_f64.to_radians()));
    }

    #[test]
    fn test_is_parallel_diagonal_lines() {
        let line1 = Line::from_coords([0.0, 0.0], [3.0, 4.0]);
        let line2 = Line::from_coords([1.0, 1.0], [4.0, 5.0]);
        assert!(is_parallel_rad(&line1, &line2, 5.0_f64.to_radians()));
    }

    #[test]
    fn test_is_parallel_not_parallel() {
        let line1 = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let line2 = Line::from_coords([0.0, 0.0], [0.0, 10.0]);
        assert!(!is_parallel_rad(&line1, &line2, 5.0_f64.to_radians()));
    }

    #[test]
    fn test_is_parallel_tolerance() {
        let line1 = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let line2 = Line::from_coords([0.0, 1.0], [10.0, 0.5]); // 轻微倾斜
        assert!(!is_parallel_rad(&line1, &line2, 1.0_f64.to_radians())); // 严格容差
        assert!(is_parallel_rad(&line1, &line2, 10.0_f64.to_radians())); // 宽松容差
    }

    // ===== line_distance 测试 =====

    #[test]
    fn test_line_distance_parallel_horizontal() {
        let line1 = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let line2 = Line::from_coords([0.0, 5.0], [10.0, 5.0]);
        let dist = line_distance(&line1, &line2);
        assert!((dist - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_line_distance_parallel_vertical() {
        let line1 = Line::from_coords([0.0, 0.0], [0.0, 10.0]);
        let line2 = Line::from_coords([7.0, 0.0], [7.0, 10.0]);
        let dist = line_distance(&line1, &line2);
        assert!((dist - 7.0).abs() < 0.001);
    }

    // ===== project_point_on_line 测试 =====

    #[test]
    fn test_project_point_on_line_start() {
        let line = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let point = Point::new(0.0, 0.0);
        let t = project_point_on_line(&point, &line);
        assert!(t < 0.001);
    }

    #[test]
    fn test_project_point_on_line_end() {
        let line = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let point = Point::new(10.0, 0.0);
        let t = project_point_on_line(&point, &line);
        assert!((t - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_project_point_on_line_midpoint() {
        let line = Line::from_coords([0.0, 0.0], [10.0, 0.0]);
        let point = Point::new(5.0, 0.0);
        let t = project_point_on_line(&point, &line);
        assert!((t - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_project_point_on_line_zero_length() {
        let line = Line::from_coords_unchecked([5.0, 5.0], [5.0, 5.0]);
        let point = Point::new(10.0, 10.0);
        let t = project_point_on_line(&point, &line);
        assert!(t < 0.001);
    }

    // ===== estimate_door_width 测试 =====

    #[test]
    fn test_estimate_door_width() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let width = estimate_door_width(&wall);
        assert_eq!(width, 900.0);
    }

    // ===== detect_doors_in_wall 测试 =====

    #[test]
    fn test_detect_doors_in_wall_with_chinese_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![door_text];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert_eq!(doors.len(), 1);
        assert_eq!(doors[0].width, 900.0);
        assert_eq!(doors[0].direction, DoorDirection::Inward);
    }

    #[test]
    fn test_detect_doors_in_wall_with_english_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let door_text = Primitive::Text {
            content: "Door".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![door_text];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert_eq!(doors.len(), 1);
    }

    #[test]
    fn test_detect_doors_in_wall_with_d_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let door_text = Primitive::Text {
            content: "D".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![door_text];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert_eq!(doors.len(), 1);
    }

    #[test]
    fn test_detect_doors_in_wall_label_too_far() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 500.0), // > 300mm away
            height: 100.0,
        };
        let primitives = vec![door_text];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert!(doors.is_empty());
    }

    #[test]
    fn test_detect_doors_in_wall_no_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let primitives: Vec<Primitive> = vec![];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert!(doors.is_empty());
    }

    #[test]
    fn test_detect_doors_in_wall_multiple_labels() {
        let wall = Line::from_coords([0.0, 0.0], [2000.0, 0.0]);
        let door1 = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let door2 = Primitive::Text {
            content: "D".to_string(),
            position: Point::new(1500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![door1, door2];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert_eq!(doors.len(), 2);
    }

    // ===== detect_windows_in_wall 测试 =====

    #[test]
    fn test_detect_windows_in_wall_with_chinese_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![window_text];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].height, 1500.0);
    }

    #[test]
    fn test_detect_windows_in_wall_with_english_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let window_text = Primitive::Text {
            content: "Window".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![window_text];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert_eq!(windows.len(), 1);
    }

    #[test]
    fn test_detect_windows_in_wall_with_w_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let window_text = Primitive::Text {
            content: "W".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![window_text];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert_eq!(windows.len(), 1);
    }

    #[test]
    fn test_detect_windows_in_wall_label_too_far() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 500.0), // > 300mm away
            height: 100.0,
        };
        let primitives = vec![window_text];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert!(windows.is_empty());
    }

    #[test]
    fn test_detect_windows_in_wall_no_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let primitives: Vec<Primitive> = vec![];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert!(windows.is_empty());
    }

    // ===== DoorWindowDetector 工具测试 =====

    #[test]
    fn test_door_window_detector_detect_doors() {
        let detector = DoorWindowDetector;
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![door_text];

        let doors = detector.detect_doors([0.0, 0.0], [1000.0, 0.0], primitives);
        assert_eq!(doors.len(), 1);
    }

    #[test]
    fn test_door_window_detector_detect_windows() {
        let detector = DoorWindowDetector;
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![window_text];

        let windows = detector.detect_windows([0.0, 0.0], [1000.0, 0.0], primitives);
        assert_eq!(windows.len(), 1);
    }

    #[test]
    fn test_door_window_detector_has_door_true() {
        let detector = DoorWindowDetector;
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![door_text];

        assert!(detector.has_door([0.0, 0.0], [1000.0, 0.0], primitives));
    }

    #[test]
    fn test_door_window_detector_has_door_false() {
        let detector = DoorWindowDetector;
        let primitives: Vec<Primitive> = vec![];

        assert!(!detector.has_door([0.0, 0.0], [1000.0, 0.0], primitives));
    }

    #[test]
    fn test_door_window_detector_has_window_true() {
        let detector = DoorWindowDetector;
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![window_text];

        assert!(detector.has_window([0.0, 0.0], [1000.0, 0.0], primitives));
    }

    #[test]
    fn test_door_window_detector_has_window_false() {
        let detector = DoorWindowDetector;
        let primitives: Vec<Primitive> = vec![];

        assert!(!detector.has_window([0.0, 0.0], [1000.0, 0.0], primitives));
    }
}
