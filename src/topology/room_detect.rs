//! 房间检测
//!
//! 检测户型图中的房间区域

use crate::geometry::{Door, DoorDirection, Line, Point, Polygon, Primitive, Room, Window};
use crate::topology::loop_detect::{find_closed_loops, find_outer_boundary};

/// 拓扑检测配置常量
///
/// `这些常量在多个拓扑检测模块中共享（room_detect`, `door_window`）
pub mod config {
    /// 文本标记检测距离阈值（毫米）
    pub const TEXT_MARK_DETECTION_DIST: f64 = 300.0;

    /// 标准门宽（毫米）
    pub const STANDARD_DOOR_WIDTH: f64 = 900.0;

    /// 标准窗宽（毫米）
    pub const STANDARD_WINDOW_WIDTH: f64 = 1200.0;

    /// 标准窗高（毫米）
    pub const STANDARD_WINDOW_HEIGHT: f64 = 1500.0;

    /// 小房间面积阈值（平方毫米）- 用于区分卫生间/储藏室
    pub const SMALL_ROOM_AREA_THRESHOLD: f64 = 50000.0;

    /// 中等房间面积阈值（平方毫米）- 用于区分卧室/书房
    pub const MEDIUM_ROOM_AREA_THRESHOLD: f64 = 150_000.0;

    /// 大房间面积阈值（平方毫米）- 用于区分客厅
    pub const LARGE_ROOM_AREA_THRESHOLD: f64 = 300_000.0;

    // ==================== 门窗检测配置 ====================

    /// 门窗检测平行容差（弧度）- 约 5 度
    pub const DOOR_WINDOW_PARALLEL_TOLERANCE_RAD: f64 = 0.087_266_5; // 5.0 * PI / 180

    /// 门窗检测平行容差（度）- 用于兼容旧代码
    #[deprecated(since = "0.1.1", note = "使用 DOOR_WINDOW_PARALLEL_TOLERANCE_RAD 代替")]
    pub const DOOR_WINDOW_PARALLEL_TOLERANCE_DEG: f64 = 5.0;

    /// 门宽最小阈值（毫米）
    pub const MIN_DOOR_WIDTH: f64 = 700.0;

    /// 门宽最大阈值（毫米）
    pub const MAX_DOOR_WIDTH: f64 = 1200.0;

    /// 窗户线间距最小阈值（毫米）
    pub const MIN_WINDOW_LINE_DIST: f64 = 150.0;

    /// 窗户线间距最大阈值（毫米）
    pub const MAX_WINDOW_LINE_DIST: f64 = 400.0;

    /// 墙缺口检测距离阈值（毫米）
    pub const WALL_GAP_DIST_THRESHOLD: f64 = 50.0;

    /// 标准窗宽估计容差
    pub const WINDOW_WIDTH_TOLERANCE: f64 = 0.4;

    /// 窗宽估计最小墙长（毫米）
    pub const MIN_WALL_LENGTH_FOR_WINDOW: f64 = 1000.0;

    /// 窗宽估计最大墙长（毫米）
    pub const MAX_WALL_LENGTH_FOR_WINDOW: f64 = 2000.0;
}

/// 房间检测结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoomDetectionResult {
    pub rooms: Vec<Room>,
    pub outer_boundary: Option<Polygon>,
}

/// 检测所有房间
pub fn detect_rooms(primitives: &[Primitive]) -> RoomDetectionResult {
    // 查找所有闭合回路
    let loops = find_closed_loops(primitives);

    // 查找外边界
    let outer_boundary = find_outer_boundary(primitives);

    let mut rooms = Vec::new();

    for (i, loop_primitives) in loops.iter().enumerate() {
        // 提取顶点：直接使用回路中的线段端点
        // 回路中的线段是首尾相连的，按顺序提取即可
        let mut vertices: Vec<Point> = Vec::new();

        // 提取第一条线段的端点
        #[allow(clippy::collapsible_match)]
        if let Some(first_prim) = loop_primitives.first() {
            if let Primitive::Line(first_line) = first_prim {
                vertices.push(first_line.start);
                vertices.push(first_line.end);
            }
        }

        // 继续添加后续线段的端点（去除重复）
        for prim in loop_primitives.iter().skip(1) {
            if let Primitive::Line(line) = prim {
                let last = vertices.last().unwrap();
                if line.start == *last {
                    vertices.push(line.end);
                } else if line.end == *last {
                    vertices.push(line.start);
                } else if vertices.len() == 2 {
                    // 第二条线段就不连接，可能是方向问题
                    if line.start == vertices[0] {
                        vertices.push(line.end);
                    } else if line.end == vertices[0] {
                        vertices.push(line.start);
                    }
                }
            }
        }

        if vertices.len() < 3 {
            continue;
        }

        // 移除最后一个顶点（如果与第一个相同）
        if vertices.len() > 3 && vertices.first() == vertices.last() {
            vertices.pop();
        }

        let boundary = Polygon::new(vertices.clone());
        let area = boundary.area();

        // 跳过外边界
        if let Some(outer) = &outer_boundary {
            if area >= outer.area() {
                continue;
            }
        }

        // 检测门和窗
        let doors = detect_doors(loop_primitives, primitives);
        let windows = detect_windows(loop_primitives, primitives);

        // 生成房间名称
        let room_name = generate_room_name(i, area, &doors);

        rooms.push(Room {
            name: room_name,
            boundary,
            area,
            doors,
            windows,
        });
    }

    RoomDetectionResult {
        rooms,
        outer_boundary,
    }
}

/// 从回路图元中提取墙线段
fn extract_walls_from_loop(loop_primitives: &[Primitive]) -> Vec<Line> {
    loop_primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line.clone()),
            _ => None,
        })
        .collect()
}

/// 检测门
fn detect_doors(loop_primitives: &[Primitive], all_primitives: &[Primitive]) -> Vec<Door> {
    let mut doors = Vec::new();

    // 从回路中提取墙线段
    let walls = extract_walls_from_loop(loop_primitives);

    // 预处理：提取所有门窗文本标签，避免重复遍历
    let door_texts: Vec<&Point> = all_primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text {
                content, position, ..
            } => {
                let content_lower = content.to_lowercase();
                if content_lower.contains("门")
                    || content_lower.contains("door")
                    || content_lower == "d"
                {
                    Some(position)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    for wall in &walls {
        let mut wall_doors = Vec::new();

        // 方法 1: 查找文本标记（使用预处理结果）
        for &position in &door_texts {
            let dist = crate::topology::door_window::distance_point_to_line(position, wall);
            if dist < config::TEXT_MARK_DETECTION_DIST {
                wall_doors.push(Door {
                    position: *position,
                    width: config::STANDARD_DOOR_WIDTH,
                    direction: DoorDirection::Inward,
                });
            }
        }

        // 方法 2: 查找墙上的缺口（只使用 loop_primitives，避免外边界干扰）
        if let Some(door) = crate::topology::door_window::detect_door_gap(wall, loop_primitives) {
            wall_doors.push(door);
        }

        doors.extend(wall_doors);
    }

    // 去重：按位置去重门（避免同一门被多面墙检测到）
    doors.sort_by(|a, b| {
        a.position
            .x
            .partial_cmp(&b.position.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.position
                    .y
                    .partial_cmp(&b.position.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    doors.dedup_by(|a, b| a.position.distance(&b.position) < config::TEXT_MARK_DETECTION_DIST);

    doors
}

/// 检测窗户
fn detect_windows(loop_primitives: &[Primitive], all_primitives: &[Primitive]) -> Vec<Window> {
    let mut windows = Vec::new();

    // 从回路中提取墙线段
    let walls = extract_walls_from_loop(loop_primitives);

    // 预处理：提取所有窗户文本标签，避免重复遍历
    let window_texts: Vec<&Point> = all_primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text {
                content, position, ..
            } => {
                let content_lower = content.to_lowercase();
                if content_lower.contains("窗")
                    || content_lower.contains("window")
                    || content_lower == "w"
                {
                    Some(position)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    for wall in &walls {
        let mut wall_windows = Vec::new();

        // 方法 1: 查找文本标记（使用预处理结果）
        for &position in &window_texts {
            let dist = crate::topology::door_window::distance_point_to_line(position, wall);
            if dist < config::TEXT_MARK_DETECTION_DIST {
                wall_windows.push(Window {
                    position: *position,
                    width: config::STANDARD_WINDOW_WIDTH,
                    height: config::STANDARD_WINDOW_HEIGHT,
                });
            }
        }

        // 方法 2: 查找墙上的双线（只使用 loop_primitives，避免外边界干扰）
        if let Some(window) =
            crate::topology::door_window::detect_window_lines(wall, loop_primitives)
        {
            wall_windows.push(window);
        }

        windows.extend(wall_windows);
    }

    // 去重：按位置去重窗户（避免同一窗户被多面墙检测到）
    windows.sort_by(|a, b| {
        a.position
            .x
            .partial_cmp(&b.position.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.position
                    .y
                    .partial_cmp(&b.position.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    windows.dedup_by(|a, b| a.position.distance(&b.position) < config::TEXT_MARK_DETECTION_DIST);

    windows
}

/// 生成房间名称
fn generate_room_name(index: usize, area: f64, doors: &[Door]) -> String {
    // 根据面积和门的数量推测房间类型
    let room_type = if area < config::SMALL_ROOM_AREA_THRESHOLD {
        if doors.len() <= 1 {
            "卫生间"
        } else {
            "储藏室"
        }
    } else if area < config::MEDIUM_ROOM_AREA_THRESHOLD {
        if doors.len() == 1 {
            "卧室"
        } else {
            "书房"
        }
    } else if area < config::LARGE_ROOM_AREA_THRESHOLD {
        "客厅"
    } else {
        "大厅"
    };

    format!("{}{}", room_type, index + 1)
}

/// 使用 tokitai 工具封装的房间检测器
#[derive(Default, Clone)]
pub struct RoomDetector;

use tokitai::tool;

#[tool]
impl RoomDetector {
    /// 检测房间
    #[tool]
    pub fn detect_rooms(&self, primitives: Vec<Primitive>) -> RoomDetectionResult {
        detect_rooms(&primitives)
    }

    /// 获取房间数量
    #[tool]
    pub fn count_rooms(&self, primitives: Vec<Primitive>) -> usize {
        let result = detect_rooms(&primitives);
        result.rooms.len()
    }

    /// 获取最大房间面积
    #[tool]
    pub fn max_room_area(&self, primitives: Vec<Primitive>) -> f64 {
        let result = detect_rooms(&primitives);
        result.rooms.iter().map(|r| r.area).fold(0.0, f64::max)
    }

    /// 获取外边界
    #[tool]
    pub fn get_outer_boundary(&self, primitives: Vec<Primitive>) -> Option<Polygon> {
        find_outer_boundary(&primitives)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    // ===== detect_rooms 测试 =====

    #[test]
    fn test_detect_rooms_empty() {
        let primitives: Vec<Primitive> = vec![];
        let result = detect_rooms(&primitives);
        assert!(result.rooms.is_empty());
        assert!(result.outer_boundary.is_none());
    }

    #[test]
    fn test_debug_find_closed_loops() {
        // 调试 find_closed_loops
        let rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1000.0, 0.0])),
            Primitive::Line(Line::from_coords([1000.0, 0.0], [1000.0, 800.0])),
            Primitive::Line(Line::from_coords([1000.0, 800.0], [0.0, 800.0])),
            Primitive::Line(Line::from_coords([0.0, 800.0], [0.0, 0.0])),
        ];

        let loops = find_closed_loops(&rect);
        println!("Found {} loops", loops.len());
        for (i, loop_prims) in loops.iter().enumerate() {
            println!("Loop {}: {} primitives", i, loop_prims.len());
        }

        assert_eq!(loops.len(), 1, "Should find exactly one closed loop");
        assert_eq!(loops[0].len(), 4, "Loop should have 4 lines");
    }

    #[test]
    fn test_debug_detect_rooms() {
        // 调试 detect_rooms - 创建一个外边界内包含一个房间
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let inner_room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1100.0])),
            Primitive::Line(Line::from_coords([1500.0, 1100.0], [500.0, 1100.0])),
            Primitive::Line(Line::from_coords([500.0, 1100.0], [500.0, 500.0])),
        ];

        let mut primitives = outer_rect;
        primitives.extend(inner_room);

        let result = detect_rooms(&primitives);
        println!("Found {} rooms", result.rooms.len());
        println!("Outer boundary: {:?}", result.outer_boundary.is_some());
        for (i, room) in result.rooms.iter().enumerate() {
            println!("Room {}: name={}, area={}", i, room.name, room.area);
        }

        assert!(!result.rooms.is_empty(), "Should detect at least one room");
    }

    #[test]
    fn test_detect_rooms_single_triangle() {
        // 创建一个三角形房间（使用线段）- 在外边界内
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let triangle = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1000.0, 1366.0])),
            Primitive::Line(Line::from_coords([1000.0, 1366.0], [500.0, 500.0])),
        ];

        let mut primitives = outer_rect;
        primitives.extend(triangle);
        let result = detect_rooms(&primitives);

        // 三角形面积 = 0.5 * 1000 * 866 = 433000
        assert!(!result.rooms.is_empty());
        assert!(result.rooms[0].area > 430000.0);
        assert!(result.rooms[0].area < 435000.0);
    }

    #[test]
    fn test_detect_rooms_rectangle() {
        // 创建一个矩形房间（使用线段）- 在外边界内
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let rect = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];

        let mut primitives = outer_rect;
        primitives.extend(rect);
        let result = detect_rooms(&primitives);

        // 矩形面积 = 1000 * 800 = 800000
        assert!(!result.rooms.is_empty());
        assert!(result.rooms[0].area > 799000.0);
        assert!(result.rooms[0].area < 801000.0);
    }

    #[test]
    fn test_detect_rooms_multiple_rooms() {
        // 创建两个独立的矩形房间（使用线段）- 在外边界内
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room1 = vec![
            Primitive::Line(Line::from_coords([100.0, 100.0], [600.0, 100.0])),
            Primitive::Line(Line::from_coords([600.0, 100.0], [600.0, 500.0])),
            Primitive::Line(Line::from_coords([600.0, 500.0], [100.0, 500.0])),
            Primitive::Line(Line::from_coords([100.0, 500.0], [100.0, 100.0])),
        ];
        let room2 = vec![
            Primitive::Line(Line::from_coords([700.0, 100.0], [1200.0, 100.0])),
            Primitive::Line(Line::from_coords([1200.0, 100.0], [1200.0, 500.0])),
            Primitive::Line(Line::from_coords([1200.0, 500.0], [700.0, 500.0])),
            Primitive::Line(Line::from_coords([700.0, 500.0], [700.0, 100.0])),
        ];
        let mut primitives = outer_rect;
        primitives.extend(room1);
        primitives.extend(room2);
        let result = detect_rooms(&primitives);

        assert_eq!(result.rooms.len(), 2);
    }

    #[test]
    fn test_detect_rooms_with_door_label() {
        // 创建一个带门标签的房间（使用线段）- 在外边界内
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(1000.0, 500.0),
            height: 100.0,
        };
        let mut primitives = outer_rect;
        primitives.extend(room);
        primitives.push(door_text);
        let result = detect_rooms(&primitives);

        assert!(!result.rooms.is_empty());
        assert_eq!(result.rooms[0].doors.len(), 1);
        assert_eq!(result.rooms[0].doors[0].width, 900.0);
        assert_eq!(result.rooms[0].doors[0].direction, DoorDirection::Inward);
    }

    #[test]
    fn test_detect_rooms_with_window_label() {
        // 创建一个带窗标签的房间（使用线段）- 在外边界内
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(1000.0, 500.0),
            height: 100.0,
        };
        let mut primitives = outer_rect;
        primitives.extend(room);
        primitives.push(window_text);
        let result = detect_rooms(&primitives);

        assert!(!result.rooms.is_empty());
        assert_eq!(result.rooms[0].windows.len(), 1);
        assert_eq!(result.rooms[0].windows[0].width, 1200.0);
        assert_eq!(result.rooms[0].windows[0].height, 1500.0);
    }

    #[test]
    fn test_detect_rooms_with_door_english() {
        // 测试英文门标签
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let door_text = Primitive::Text {
            content: "Door".to_string(),
            position: Point::new(1000.0, 500.0),
            height: 100.0,
        };
        let mut primitives = outer_rect;
        primitives.extend(room);
        primitives.push(door_text);
        let result = detect_rooms(&primitives);

        assert_eq!(result.rooms[0].doors.len(), 1);
    }

    #[test]
    fn test_detect_rooms_with_window_english() {
        // 测试英文窗标签
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let window_text = Primitive::Text {
            content: "Window".to_string(),
            position: Point::new(1000.0, 500.0),
            height: 100.0,
        };
        let mut primitives = outer_rect;
        primitives.extend(room);
        primitives.push(window_text);
        let result = detect_rooms(&primitives);

        assert_eq!(result.rooms[0].windows.len(), 1);
    }

    #[test]
    fn test_detect_rooms_with_d_label() {
        // 测试简写门标签
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let door_text = Primitive::Text {
            content: "D".to_string(),
            position: Point::new(1000.0, 500.0),
            height: 100.0,
        };
        let mut primitives = outer_rect;
        primitives.extend(room);
        primitives.push(door_text);
        let result = detect_rooms(&primitives);

        assert_eq!(result.rooms[0].doors.len(), 1);
    }

    #[test]
    fn test_detect_rooms_with_w_label() {
        // 测试简写窗标签
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let window_text = Primitive::Text {
            content: "W".to_string(),
            position: Point::new(1000.0, 500.0),
            height: 100.0,
        };
        let mut primitives = outer_rect;
        primitives.extend(room);
        primitives.push(window_text);
        let result = detect_rooms(&primitives);

        assert_eq!(result.rooms[0].windows.len(), 1);
    }

    #[test]
    fn test_detect_rooms_door_too_far() {
        // 门标签距离太远，不应被检测到
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(5000.0, 5000.0),
            height: 100.0,
        };
        let mut primitives = outer_rect;
        primitives.extend(room);
        primitives.push(door_text);
        let result = detect_rooms(&primitives);

        assert_eq!(result.rooms[0].doors.len(), 0);
    }

    #[test]
    fn test_detect_rooms_room_naming_small_room() {
        // 小房间命名测试（面积 < 50000，1 个门）
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let small_rect = vec![
            Primitive::Line(Line::from_coords([100.0, 100.0], [300.0, 100.0])),
            Primitive::Line(Line::from_coords([300.0, 100.0], [300.0, 300.0])),
            Primitive::Line(Line::from_coords([300.0, 300.0], [100.0, 300.0])),
            Primitive::Line(Line::from_coords([100.0, 300.0], [100.0, 100.0])),
        ];
        let mut primitives = outer_rect;
        primitives.extend(small_rect);
        let result = detect_rooms(&primitives);

        assert!(!result.rooms.is_empty());
        assert!(result.rooms[0].name.contains("卫生间") || result.rooms[0].name.contains("储藏室"));
    }

    #[test]
    fn test_detect_rooms_room_naming_medium_room() {
        // 中等房间命名测试（50000 < 面积 < 150000，1 个门）
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let medium_rect = vec![
            Primitive::Line(Line::from_coords([100.0, 100.0], [400.0, 100.0])),
            Primitive::Line(Line::from_coords([400.0, 100.0], [400.0, 500.0])),
            Primitive::Line(Line::from_coords([400.0, 500.0], [100.0, 500.0])),
            Primitive::Line(Line::from_coords([100.0, 500.0], [100.0, 100.0])),
        ];
        let mut primitives = outer_rect;
        primitives.extend(medium_rect);
        let result = detect_rooms(&primitives);

        assert!(!result.rooms.is_empty());
        assert!(result.rooms[0].name.contains("卧室") || result.rooms[0].name.contains("书房"));
    }

    #[test]
    fn test_detect_rooms_room_naming_large_room() {
        // 大房间命名测试（150000 < 面积 < 300000）
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let large_rect = vec![
            Primitive::Line(Line::from_coords([100.0, 100.0], [600.0, 100.0])),
            Primitive::Line(Line::from_coords([600.0, 100.0], [600.0, 500.0])),
            Primitive::Line(Line::from_coords([600.0, 500.0], [100.0, 500.0])),
            Primitive::Line(Line::from_coords([100.0, 500.0], [100.0, 100.0])),
        ];
        let mut primitives = outer_rect;
        primitives.extend(large_rect);
        let result = detect_rooms(&primitives);

        assert!(!result.rooms.is_empty());
        assert!(result.rooms[0].name.contains("客厅"));
    }

    #[test]
    fn test_detect_rooms_room_naming_hall() {
        // 大厅命名测试（面积 > 300000）
        let outer_rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [3000.0, 0.0])),
            Primitive::Line(Line::from_coords([3000.0, 0.0], [3000.0, 2000.0])),
            Primitive::Line(Line::from_coords([3000.0, 2000.0], [0.0, 2000.0])),
            Primitive::Line(Line::from_coords([0.0, 2000.0], [0.0, 0.0])),
        ];
        let hall_rect = vec![
            Primitive::Line(Line::from_coords([100.0, 100.0], [900.0, 100.0])),
            Primitive::Line(Line::from_coords([900.0, 100.0], [900.0, 600.0])),
            Primitive::Line(Line::from_coords([900.0, 600.0], [100.0, 600.0])),
            Primitive::Line(Line::from_coords([100.0, 600.0], [100.0, 100.0])),
        ];
        let mut primitives = outer_rect;
        primitives.extend(hall_rect);
        let result = detect_rooms(&primitives);

        assert!(!result.rooms.is_empty());
        assert!(result.rooms[0].name.contains("大厅"));
    }

    // ===== detect_doors 测试 =====

    #[test]
    fn test_detect_doors_empty() {
        let primitives: Vec<Primitive> = vec![];
        let loops = find_closed_loops(&primitives);
        if let Some(loop_prims) = loops.first() {
            let doors = detect_doors(loop_prims, &primitives);
            assert!(doors.is_empty());
        }
    }

    #[test]
    fn test_detect_doors_single_door() {
        let rect = Primitive::Rect(Rect::from_coords([0.0, 0.0], [1000.0, 800.0]));
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 0.0),
            height: 100.0,
        };
        let primitives = vec![rect, door_text];
        let loops = find_closed_loops(&primitives);

        if let Some(loop_prims) = loops.first() {
            let doors = detect_doors(loop_prims, &primitives);
            assert_eq!(doors.len(), 1);
            assert_eq!(doors[0].width, 900.0);
        }
    }

    // ===== detect_windows 测试 =====

    #[test]
    fn test_detect_windows_empty() {
        let primitives: Vec<Primitive> = vec![];
        let loops = find_closed_loops(&primitives);
        if let Some(loop_prims) = loops.first() {
            let windows = detect_windows(loop_prims, &primitives);
            assert!(windows.is_empty());
        }
    }

    #[test]
    fn test_detect_windows_single_window() {
        let rect = Primitive::Rect(Rect::from_coords([0.0, 0.0], [1000.0, 800.0]));
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 0.0),
            height: 100.0,
        };
        let primitives = vec![rect, window_text];
        let loops = find_closed_loops(&primitives);

        if let Some(loop_prims) = loops.first() {
            let windows = detect_windows(loop_prims, &primitives);
            assert_eq!(windows.len(), 1);
            assert_eq!(windows[0].width, 1200.0);
            assert_eq!(windows[0].height, 1500.0);
        }
    }

    // ===== detect_door_in_wall 测试 =====
    // 注意：这些测试现在使用 door_window 模块的函数

    #[test]
    fn test_detect_door_in_wall_with_chinese_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![door_text];

        let doors = crate::topology::door_window::detect_doors_in_wall(&wall, &primitives);
        assert!(!doors.is_empty());
        assert_eq!(doors[0].width, 900.0);
    }

    #[test]
    fn test_detect_door_in_wall_no_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let primitives: Vec<Primitive> = vec![];

        let doors = crate::topology::door_window::detect_doors_in_wall(&wall, &primitives);
        assert!(doors.is_empty());
    }

    #[test]
    fn test_detect_door_in_wall_label_too_far() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let door_text = Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 1000.0),
            height: 100.0,
        };
        let primitives = vec![door_text];

        let doors = crate::topology::door_window::detect_doors_in_wall(&wall, &primitives);
        assert!(doors.is_empty());
    }

    // ===== detect_window_in_wall 测试 =====
    // 注意：这些测试现在使用 door_window 模块的函数

    #[test]
    fn test_detect_window_in_wall_with_chinese_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        };
        let primitives = vec![window_text];

        let windows = crate::topology::door_window::detect_windows_in_wall(&wall, &primitives);
        assert!(!windows.is_empty());
        let window = &windows[0];
        assert_eq!(window.width, 1200.0);
        assert_eq!(window.height, 1500.0);
    }

    #[test]
    fn test_detect_window_in_wall_no_label() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let primitives: Vec<Primitive> = vec![];

        let windows = crate::topology::door_window::detect_windows_in_wall(&wall, &primitives);
        assert!(windows.is_empty());
    }

    #[test]
    fn test_detect_window_in_wall_label_too_far() {
        let wall = Line::from_coords([0.0, 0.0], [1000.0, 0.0]);
        let window_text = Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 1000.0),
            height: 100.0,
        };
        let primitives = vec![window_text];

        let windows = crate::topology::door_window::detect_windows_in_wall(&wall, &primitives);
        assert!(windows.is_empty());
    }

    // ===== generate_room_name 测试 =====

    #[test]
    fn test_generate_room_name_bathroom() {
        // 小面积，少门 -> 卫生间
        let name = generate_room_name(0, 40000.0, &[]);
        assert!(name.contains("卫生间") || name.contains("储藏室"));
    }

    #[test]
    fn test_generate_room_name_bedroom() {
        // 中等面积，1 门 -> 卧室
        let doors = vec![Door {
            position: Point::origin(),
            width: 900.0,
            direction: DoorDirection::Inward,
        }];
        let name = generate_room_name(0, 100000.0, &doors);
        assert!(name.contains("卧室"));
    }

    #[test]
    fn test_generate_room_name_study() {
        // 中等面积，多门 -> 书房
        let doors = vec![
            Door {
                position: Point::origin(),
                width: 900.0,
                direction: DoorDirection::Inward,
            },
            Door {
                position: Point::new(100.0, 0.0),
                width: 900.0,
                direction: DoorDirection::Inward,
            },
        ];
        let name = generate_room_name(0, 100000.0, &doors);
        assert!(name.contains("书房"));
    }

    #[test]
    fn test_generate_room_name_living_room() {
        // 大面积 -> 客厅
        let name = generate_room_name(0, 200000.0, &[]);
        assert!(name.contains("客厅"));
    }

    #[test]
    fn test_generate_room_name_hall() {
        // 超大面积 -> 大厅
        let name = generate_room_name(0, 400000.0, &[]);
        assert!(name.contains("大厅"));
    }

    #[test]
    fn test_generate_room_name_index() {
        // 测试索引
        let name = generate_room_name(5, 200000.0, &[]);
        assert!(name.contains("6")); // 索引从 0 开始，所以是 6
    }

    // ===== RoomDetector 工具测试 =====

    #[test]
    fn test_room_detector_detect_rooms() {
        let detector = RoomDetector;
        // 需要一个外边界 + 内部房间
        let outer = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room = vec![
            Primitive::Line(Line::from_coords([500.0, 500.0], [1500.0, 500.0])),
            Primitive::Line(Line::from_coords([1500.0, 500.0], [1500.0, 1300.0])),
            Primitive::Line(Line::from_coords([1500.0, 1300.0], [500.0, 1300.0])),
            Primitive::Line(Line::from_coords([500.0, 1300.0], [500.0, 500.0])),
        ];
        let mut primitives = outer;
        primitives.extend(room);

        let result = detector.detect_rooms(primitives);
        assert!(!result.rooms.is_empty());
    }

    #[test]
    fn test_room_detector_count_rooms() {
        let detector = RoomDetector;
        // 需要一个外边界 + 两个内部房间
        let outer = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room1 = vec![
            Primitive::Line(Line::from_coords([100.0, 100.0], [600.0, 100.0])),
            Primitive::Line(Line::from_coords([600.0, 100.0], [600.0, 500.0])),
            Primitive::Line(Line::from_coords([600.0, 500.0], [100.0, 500.0])),
            Primitive::Line(Line::from_coords([100.0, 500.0], [100.0, 100.0])),
        ];
        let room2 = vec![
            Primitive::Line(Line::from_coords([700.0, 100.0], [1200.0, 100.0])),
            Primitive::Line(Line::from_coords([1200.0, 100.0], [1200.0, 500.0])),
            Primitive::Line(Line::from_coords([1200.0, 500.0], [700.0, 500.0])),
            Primitive::Line(Line::from_coords([700.0, 500.0], [700.0, 100.0])),
        ];
        let mut primitives = outer;
        primitives.extend(room1);
        primitives.extend(room2);

        let count = detector.count_rooms(primitives);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_room_detector_count_rooms_empty() {
        let detector = RoomDetector;
        let primitives: Vec<Primitive> = vec![];

        let count = detector.count_rooms(primitives);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_room_detector_max_room_area() {
        let detector = RoomDetector;
        // 需要一个外边界 + 两个内部房间
        let outer = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [2000.0, 0.0])),
            Primitive::Line(Line::from_coords([2000.0, 0.0], [2000.0, 1600.0])),
            Primitive::Line(Line::from_coords([2000.0, 1600.0], [0.0, 1600.0])),
            Primitive::Line(Line::from_coords([0.0, 1600.0], [0.0, 0.0])),
        ];
        let room1 = vec![
            Primitive::Line(Line::from_coords([100.0, 100.0], [600.0, 100.0])),
            Primitive::Line(Line::from_coords([600.0, 100.0], [600.0, 500.0])),
            Primitive::Line(Line::from_coords([600.0, 500.0], [100.0, 500.0])),
            Primitive::Line(Line::from_coords([100.0, 500.0], [100.0, 100.0])),
        ]; // 200000
        let room2 = vec![
            Primitive::Line(Line::from_coords([700.0, 100.0], [1200.0, 100.0])),
            Primitive::Line(Line::from_coords([1200.0, 100.0], [1200.0, 700.0])),
            Primitive::Line(Line::from_coords([1200.0, 700.0], [700.0, 700.0])),
            Primitive::Line(Line::from_coords([700.0, 700.0], [700.0, 100.0])),
        ]; // 300000
        let mut primitives = outer;
        primitives.extend(room1);
        primitives.extend(room2);

        let max_area = detector.max_room_area(primitives);
        assert!(max_area > 299000.0);
    }

    #[test]
    fn test_room_detector_max_room_area_empty() {
        let detector = RoomDetector;
        let primitives: Vec<Primitive> = vec![];

        let max_area = detector.max_room_area(primitives);
        assert_eq!(max_area, 0.0);
    }

    #[test]
    fn test_room_detector_get_outer_boundary() {
        let detector = RoomDetector;
        let rect = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1000.0, 0.0])),
            Primitive::Line(Line::from_coords([1000.0, 0.0], [1000.0, 800.0])),
            Primitive::Line(Line::from_coords([1000.0, 800.0], [0.0, 800.0])),
            Primitive::Line(Line::from_coords([0.0, 800.0], [0.0, 0.0])),
        ];

        let boundary = detector.get_outer_boundary(rect);
        assert!(boundary.is_some());
        assert!(boundary.unwrap().area() > 799000.0);
    }

    #[test]
    fn test_room_detector_get_outer_boundary_empty() {
        let detector = RoomDetector;
        let primitives: Vec<Primitive> = vec![];

        let boundary = detector.get_outer_boundary(primitives);
        assert!(boundary.is_none());
    }
}
