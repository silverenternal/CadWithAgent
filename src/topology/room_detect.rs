//! 房间检测
//!
//! 检测户型图中的房间区域

use crate::geometry::{Door, DoorDirection, Line, Point, Polygon, Primitive, Room, Window};
use crate::topology::loop_detect::{find_closed_loops, find_outer_boundary};

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
        // 提取顶点
        let vertices: Vec<Point> = loop_primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Line(line) => Some(line.start),
                _ => None,
            })
            .collect();

        if vertices.len() < 3 {
            continue;
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

/// 检测门
fn detect_doors(loop_primitives: &[Primitive], all_primitives: &[Primitive]) -> Vec<Door> {
    let mut doors = Vec::new();

    // 从回路中提取墙线段
    let walls: Vec<Line> = loop_primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line.clone()),
            _ => None,
        })
        .collect();

    for wall in &walls {
        // 检查是否有门
        if let Some(door) = detect_door_in_wall(wall, all_primitives) {
            doors.push(door);
        }
    }

    doors
}

/// 检测窗户
fn detect_windows(loop_primitives: &[Primitive], all_primitives: &[Primitive]) -> Vec<Window> {
    let mut windows = Vec::new();

    // 从回路中提取墙线段
    let walls: Vec<Line> = loop_primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Line(line) => Some(line.clone()),
            _ => None,
        })
        .collect();

    for wall in &walls {
        // 检查是否有窗
        if let Some(window) = detect_window_in_wall(wall, all_primitives) {
            windows.push(window);
        }
    }

    windows
}

/// 在墙中检测门
fn detect_door_in_wall(wall: &Line, all_primitives: &[Primitive]) -> Option<Door> {
    // 简化检测：查找墙上的缺口或特殊标记
    // 实际应用中可能需要更复杂的几何分析

    // 检查是否有文本标记为"门"或"D"
    for prim in all_primitives {
        if let Primitive::Text {
            content, position, ..
        } = prim
        {
            if content.to_lowercase().contains("门")
                || content.to_lowercase().contains("door")
                || content.to_lowercase() == "d"
            {
                // 检查文本是否在墙附近
                let dist = wall.midpoint().distance(position);
                if dist < 500.0 {
                    return Some(Door {
                        position: wall.midpoint(),
                        width: 900.0, // 标准门宽
                        direction: DoorDirection::Inward,
                    });
                }
            }
        }
    }

    // 检查墙是否有缺口（门的特征）
    // 这里简化处理，假设墙是连续的
    None
}

/// 在墙中检测窗户
fn detect_window_in_wall(wall: &Line, all_primitives: &[Primitive]) -> Option<Window> {
    // 简化检测：查找墙上的特殊标记

    // 检查是否有文本标记为"窗"或"W"
    for prim in all_primitives {
        if let Primitive::Text {
            content, position, ..
        } = prim
        {
            if content.to_lowercase().contains("窗")
                || content.to_lowercase().contains("window")
                || content.to_lowercase() == "w"
            {
                // 检查文本是否在墙附近
                let dist = wall.midpoint().distance(position);
                if dist < 500.0 {
                    return Some(Window {
                        position: wall.midpoint(),
                        width: 1200.0, // 标准窗宽
                        height: 1500.0,
                    });
                }
            }
        }
    }

    None
}

/// 生成房间名称
fn generate_room_name(index: usize, area: f64, doors: &[Door]) -> String {
    // 根据面积和门的数量推测房间类型
    let room_type = if area < 50000.0 {
        if doors.len() <= 1 {
            "卫生间"
        } else {
            "储藏室"
        }
    } else if area < 150000.0 {
        if doors.len() == 1 {
            "卧室"
        } else {
            "书房"
        }
    } else if area < 300000.0 {
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
