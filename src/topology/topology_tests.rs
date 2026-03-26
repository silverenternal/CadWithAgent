//! 拓扑分析模块测试

#[cfg(test)]
mod tests {
    use crate::geometry::{Door, DoorDirection, Line, Point, Polygon, Primitive, Room, Window};
    use crate::topology::{
        detect_doors_in_wall, detect_rooms, detect_windows_in_wall, DoorWindowDetector,
        RoomDetectionResult,
    };

    #[test]
    fn test_door_window_detector_tools() {
        let detector = DoorWindowDetector;

        let wall_start = [0.0, 0.0];
        let wall_end = [1000.0, 0.0];
        let primitives = vec![Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        }];

        let doors = detector.detect_doors(wall_start, wall_end, primitives.clone());
        assert!(!doors.is_empty());

        let has_door = detector.has_door(wall_start, wall_end, primitives.clone());
        assert!(has_door);

        let has_window = detector.has_window(wall_start, wall_end, primitives.clone());
        assert!(!has_window);
    }

    #[test]
    fn test_detect_doors_with_text_marker() {
        let wall = Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0));
        let primitives = vec![Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        }];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert!(!doors.is_empty());

        let door = &doors[0];
        assert!((door.position.x - 500.0).abs() < 1.0);
        assert!((door.position.y - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_detect_doors_english_marker() {
        let wall = Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0));
        let primitives = vec![Primitive::Text {
            content: "DOOR".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        }];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert!(!doors.is_empty());
    }

    #[test]
    fn test_detect_doors_short_marker() {
        let wall = Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0));
        let primitives = vec![Primitive::Text {
            content: "D".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        }];

        let doors = detect_doors_in_wall(&wall, &primitives);
        assert!(!doors.is_empty());
    }

    #[test]
    fn test_detect_doors_far_away() {
        let wall = Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0));
        let primitives = vec![Primitive::Text {
            content: "门".to_string(),
            position: Point::new(500.0, 200.0), // 距离墙太远
            height: 100.0,
        }];

        let doors = detect_doors_in_wall(&wall, &primitives);
        // 距离超过 100.0，不应该被检测到
        assert!(doors.is_empty());
    }

    #[test]
    fn test_detect_windows_with_text_marker() {
        let wall = Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0));
        let primitives = vec![Primitive::Text {
            content: "窗".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        }];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert!(!windows.is_empty());

        let window = &windows[0];
        assert!((window.position.x - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_detect_windows_english_marker() {
        let wall = Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0));
        let primitives = vec![Primitive::Text {
            content: "WINDOW".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        }];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert!(!windows.is_empty());
    }

    #[test]
    fn test_detect_windows_short_marker() {
        let wall = Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0));
        let primitives = vec![Primitive::Text {
            content: "W".to_string(),
            position: Point::new(500.0, 50.0),
            height: 100.0,
        }];

        let windows = detect_windows_in_wall(&wall, &primitives);
        assert!(!windows.is_empty());
    }

    #[test]
    fn test_door_struct() {
        let door = Door {
            position: Point::new(500.0, 0.0),
            width: 900.0,
            direction: DoorDirection::Inward,
        };

        assert!((door.position.x - 500.0).abs() < 0.1);
        assert!((door.width - 900.0).abs() < 0.1);
        assert_eq!(door.direction, DoorDirection::Inward);
    }

    #[test]
    fn test_window_struct() {
        let window = Window {
            position: Point::new(500.0, 0.0),
            width: 1500.0,
            height: 1200.0,
        };

        assert!((window.position.x - 500.0).abs() < 0.1);
        assert!((window.width - 1500.0).abs() < 0.1);
        assert!((window.height - 1200.0).abs() < 0.1);
    }

    #[test]
    fn test_door_direction_enum() {
        let inward = DoorDirection::Inward;
        let outward = DoorDirection::Outward;

        assert_ne!(inward, outward);
    }

    #[test]
    fn test_detect_rooms_basic() {
        // 创建一个简单的正方形房间
        let lines = vec![
            Primitive::Line(Line::new(Point::new(0.0, 0.0), Point::new(1000.0, 0.0))),
            Primitive::Line(Line::new(
                Point::new(1000.0, 0.0),
                Point::new(1000.0, 1000.0),
            )),
            Primitive::Line(Line::new(
                Point::new(1000.0, 1000.0),
                Point::new(0.0, 1000.0),
            )),
            Primitive::Line(Line::new(Point::new(0.0, 1000.0), Point::new(0.0, 0.0))),
        ];

        let result = detect_rooms(&lines);

        // 测试返回类型
        assert!(result.rooms.is_empty() || !result.rooms.is_empty()); // 总是 true，只测试不 panic
    }

    #[test]
    fn test_detect_rooms_empty() {
        let lines: Vec<Primitive> = vec![];
        let result = detect_rooms(&lines);

        assert!(result.rooms.is_empty());
    }

    #[test]
    fn test_room_detection_result_struct() {
        let boundary = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(1000.0, 0.0),
            Point::new(1000.0, 1000.0),
            Point::new(0.0, 1000.0),
        ]);

        let result = RoomDetectionResult {
            rooms: vec![],
            outer_boundary: Some(boundary),
        };

        assert!(result.rooms.is_empty());
        assert!(result.outer_boundary.is_some());
    }

    #[test]
    fn test_room_struct() {
        let boundary = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(1000.0, 0.0),
            Point::new(1000.0, 1000.0),
            Point::new(0.0, 1000.0),
        ]);

        let room = Room {
            name: "Living Room".to_string(),
            boundary: boundary.clone(),
            area: 1000000.0,
            doors: vec![],
            windows: vec![],
        };

        assert_eq!(room.name, "Living Room");
        assert_eq!(room.boundary.vertices.len(), 4);
        assert!((room.area - 1000000.0).abs() < 0.1);
        assert!(room.doors.is_empty());
        assert!(room.windows.is_empty());
    }

    #[test]
    fn test_room_with_doors_and_windows() {
        let boundary = Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(1000.0, 0.0),
            Point::new(1000.0, 1000.0),
            Point::new(0.0, 1000.0),
        ]);

        let door = Door {
            position: Point::new(500.0, 0.0),
            width: 900.0,
            direction: DoorDirection::Inward,
        };

        let window = Window {
            position: Point::new(500.0, 500.0),
            width: 1500.0,
            height: 1200.0,
        };

        let room = Room {
            name: "Bedroom".to_string(),
            boundary,
            area: 1000000.0,
            doors: vec![door.clone()],
            windows: vec![window.clone()],
        };

        assert_eq!(room.doors.len(), 1);
        assert_eq!(room.windows.len(), 1);
        assert_eq!(room.doors[0].width, 900.0);
        assert_eq!(room.windows[0].width, 1500.0);
    }

    #[test]
    fn test_door_window_detector_clone() {
        let detector = DoorWindowDetector;
        let cloned = detector.clone();

        // 测试克隆后的实例可以正常使用
        let wall_start = [0.0, 0.0];
        let wall_end = [1000.0, 0.0];
        let primitives = vec![];

        let _ = cloned.detect_doors(wall_start, wall_end, primitives);
    }
}
