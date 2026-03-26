//! 几何变换工具测试

#[cfg(test)]
mod tests {
    use crate::geometry::{
        Circle, GeometryTransform, Line, MirrorAxis, Point, Polygon, Primitive, Rect,
    };

    fn create_test_primitives() -> Vec<Primitive> {
        vec![
            Primitive::Point(Point::new(1.0, 2.0)),
            Primitive::Line(Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))),
            Primitive::Polygon(Polygon::new(vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(1.0, 1.0),
                Point::new(0.0, 1.0),
            ])),
            Primitive::Circle(Circle::new(Point::new(0.0, 0.0), 5.0)),
            Primitive::Rect(Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 3.0))),
            Primitive::Polyline {
                points: vec![
                    Point::new(0.0, 0.0),
                    Point::new(1.0, 1.0),
                    Point::new(2.0, 0.0),
                ],
                closed: false,
            },
            Primitive::Arc {
                center: Point::new(0.0, 0.0),
                radius: 5.0,
                start_angle: 0.0,
                end_angle: 90.0,
            },
            Primitive::Text {
                content: "Test".to_string(),
                position: Point::new(1.0, 1.0),
                height: 12.0,
            },
        ]
    }

    #[test]
    fn test_translate_point() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(1.0, 2.0));
        let result = transform.translate(vec![point], 3.0, 4.0);

        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 4.0).abs() < 1e-10);
            assert!((p.y - 6.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_translate_line() {
        let transform = GeometryTransform;
        let line = Primitive::Line(Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0)));
        let result = transform.translate(vec![line], 2.0, 3.0);

        if let Primitive::Line(l) = &result[0] {
            assert!((l.start.x - 2.0).abs() < 1e-10);
            assert!((l.start.y - 3.0).abs() < 1e-10);
            assert!((l.end.x - 3.0).abs() < 1e-10);
            assert!((l.end.y - 4.0).abs() < 1e-10);
        } else {
            panic!("Expected Line");
        }
    }

    #[test]
    fn test_translate_circle() {
        let transform = GeometryTransform;
        let circle = Primitive::Circle(Circle::new(Point::new(0.0, 0.0), 5.0));
        let result = transform.translate(vec![circle], 1.0, 2.0);

        if let Primitive::Circle(c) = &result[0] {
            assert!((c.center.x - 1.0).abs() < 1e-10);
            assert!((c.center.y - 2.0).abs() < 1e-10);
            assert!((c.radius - 5.0).abs() < 1e-10);
        } else {
            panic!("Expected Circle");
        }
    }

    #[test]
    fn test_translate_all_primitives() {
        let transform = GeometryTransform;
        let primitives = create_test_primitives();
        let result = transform.translate(primitives, 5.0, -3.0);

        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_rotate_point() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(1.0, 0.0));
        // 旋转 90 度，应该到 (0, 1)
        let result = transform.rotate(vec![point], 90.0, [0.0, 0.0]);

        if let Primitive::Point(p) = &result[0] {
            assert!(p.x.abs() < 1e-10);
            assert!((p.y - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_rotate_line() {
        let transform = GeometryTransform;
        let line = Primitive::Line(Line::new(Point::new(0.0, 0.0), Point::new(1.0, 0.0)));
        let result = transform.rotate(vec![line], 90.0, [0.0, 0.0]);

        if let Primitive::Line(l) = &result[0] {
            assert!(l.start.x.abs() < 1e-10);
            assert!(l.start.y.abs() < 1e-10);
            assert!(l.end.x.abs() < 1e-10);
            assert!((l.end.y - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Line");
        }
    }

    #[test]
    fn test_rotate_circle() {
        let transform = GeometryTransform;
        let circle = Primitive::Circle(Circle::new(Point::new(2.0, 0.0), 1.0));
        let result = transform.rotate(vec![circle], 90.0, [0.0, 0.0]);

        if let Primitive::Circle(c) = &result[0] {
            assert!(c.center.x.abs() < 1e-10);
            assert!((c.center.y - 2.0).abs() < 1e-10);
            assert!((c.radius - 1.0).abs() < 1e-10);
        } else {
            panic!("Expected Circle");
        }
    }

    #[test]
    fn test_rotate_about_custom_center() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(2.0, 0.0));
        // 绕 (1.0, 0.0) 旋转 180 度，应该到 (0.0, 0.0)
        let result = transform.rotate(vec![point], 180.0, [1.0, 0.0]);

        if let Primitive::Point(p) = &result[0] {
            assert!(p.x.abs() < 1e-10);
            assert!(p.y.abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_rotate_all_primitives() {
        let transform = GeometryTransform;
        let primitives = create_test_primitives();
        let result = transform.rotate(primitives, 45.0, [0.0, 0.0]);

        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_scale_point() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(2.0, 4.0));
        let result = transform.scale(vec![point], 0.5, [0.0, 0.0]);

        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 1.0).abs() < 1e-10);
            assert!((p.y - 2.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_scale_circle() {
        let transform = GeometryTransform;
        let circle = Primitive::Circle(Circle::new(Point::new(0.0, 0.0), 10.0));
        let result = transform.scale(vec![circle], 0.5, [0.0, 0.0]);

        if let Primitive::Circle(c) = &result[0] {
            assert!((c.center.x - 0.0).abs() < 1e-10);
            assert!((c.center.y - 0.0).abs() < 1e-10);
            assert!((c.radius - 5.0).abs() < 1e-10);
        } else {
            panic!("Expected Circle");
        }
    }

    #[test]
    fn test_scale_about_custom_center() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(3.0, 0.0));
        // 以 (1.0, 0.0) 为中心缩放 0.5 倍
        // 新位置 = 1.0 + (3.0 - 1.0) * 0.5 = 2.0
        let result = transform.scale(vec![point], 0.5, [1.0, 0.0]);

        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 2.0).abs() < 1e-10);
            assert!((p.y - 0.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_scale_rect() {
        let transform = GeometryTransform;
        let rect = Primitive::Rect(Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 4.0)));
        let result = transform.scale(vec![rect], 0.5, [0.0, 0.0]);

        if let Primitive::Rect(r) = &result[0] {
            assert!((r.min.x - 0.0).abs() < 1e-10);
            assert!((r.min.y - 0.0).abs() < 1e-10);
            assert!((r.max.x - 1.0).abs() < 1e-10);
            assert!((r.max.y - 2.0).abs() < 1e-10);
        } else {
            panic!("Expected Rect");
        }
    }

    #[test]
    fn test_scale_all_primitives() {
        let transform = GeometryTransform;
        let primitives = create_test_primitives();
        let result = transform.scale(primitives, 2.0, [0.0, 0.0]);

        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_mirror_x_axis() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(3.0, 4.0));
        let result = transform.mirror(vec![point], MirrorAxis::X);

        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 3.0).abs() < 1e-10);
            assert!((p.y - (-4.0)).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_mirror_y_axis() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(3.0, 4.0));
        let result = transform.mirror(vec![point], MirrorAxis::Y);

        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - (-3.0)).abs() < 1e-10);
            assert!((p.y - 4.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_mirror_circle() {
        let transform = GeometryTransform;
        let circle = Primitive::Circle(Circle::new(Point::new(2.0, 3.0), 5.0));
        let result = transform.mirror(vec![circle], MirrorAxis::X);

        if let Primitive::Circle(c) = &result[0] {
            assert!((c.center.x - 2.0).abs() < 1e-10);
            assert!((c.center.y - (-3.0)).abs() < 1e-10);
            assert!((c.radius - 5.0).abs() < 1e-10);
        } else {
            panic!("Expected Circle");
        }
    }

    #[test]
    fn test_mirror_all_primitives() {
        let transform = GeometryTransform;
        let primitives = create_test_primitives();
        let result = transform.mirror(primitives, MirrorAxis::Y);

        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_mirror_about_line() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(0.0, 2.0));
        // 关于 y=0 的直线（x 轴）镜像
        let result = transform.mirror_about_line(vec![point], [0.0, 0.0], [1.0, 0.0]);

        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 0.0).abs() < 1e-10);
            assert!((p.y - (-2.0)).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_mirror_about_diagonal_line() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(2.0, 0.0));
        // 关于 y=x 的直线镜像，(2,0) 应该变成 (0,2)
        let result = transform.mirror_about_line(vec![point], [0.0, 0.0], [1.0, 1.0]);

        if let Primitive::Point(p) = &result[0] {
            assert!((p.x - 0.0).abs() < 1e-10);
            assert!((p.y - 2.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_mirror_about_line_all_primitives() {
        let transform = GeometryTransform;
        let primitives = create_test_primitives();
        let result = transform.mirror_about_line(primitives, [0.0, 0.0], [1.0, 0.0]);

        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_mirror_axis_serialization() {
        let x_axis = MirrorAxis::X;
        let y_axis = MirrorAxis::Y;

        let x_json = serde_json::to_string(&x_axis).unwrap();
        let y_json = serde_json::to_string(&y_axis).unwrap();

        assert_eq!(x_json, "\"x\"");
        assert_eq!(y_json, "\"y\"");

        let x_deser: MirrorAxis = serde_json::from_str(&x_json).unwrap();
        let y_deser: MirrorAxis = serde_json::from_str(&y_json).unwrap();

        assert_eq!(x_deser, MirrorAxis::X);
        assert_eq!(y_deser, MirrorAxis::Y);
    }

    #[test]
    fn test_transform_chained_operations() {
        let transform = GeometryTransform;
        let point = Primitive::Point(Point::new(1.0, 0.0));

        // 先平移 (1,0)->(2,0)，再旋转 90 度 (2,0)->(0,2)，再缩放 2 倍 (0,2)->(0,4)
        let result = transform.translate(vec![point.clone()], 1.0, 0.0);
        let result = transform.rotate(result, 90.0, [0.0, 0.0]);
        let result = transform.scale(result, 2.0, [0.0, 0.0]);

        if let Primitive::Point(p) = &result[0] {
            assert!(p.x.abs() < 1e-10);
            assert!((p.y - 4.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_translate_polygon() {
        let transform = GeometryTransform;
        let polygon = Primitive::Polygon(Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
        ]));
        let result = transform.translate(vec![polygon], 3.0, 4.0);

        if let Primitive::Polygon(p) = &result[0] {
            assert_eq!(p.vertices.len(), 3);
            assert!((p.vertices[0].x - 3.0).abs() < 1e-10);
            assert!((p.vertices[0].y - 4.0).abs() < 1e-10);
        } else {
            panic!("Expected Polygon");
        }
    }

    #[test]
    fn test_rotate_arc() {
        let transform = GeometryTransform;
        let arc = Primitive::Arc {
            center: Point::new(1.0, 0.0),
            radius: 5.0,
            start_angle: 0.0,
            end_angle: 90.0,
        };
        let result = transform.rotate(vec![arc], 90.0, [0.0, 0.0]);

        if let Primitive::Arc {
            center,
            start_angle,
            end_angle,
            ..
        } = &result[0]
        {
            assert!(center.x.abs() < 1e-10);
            assert!((center.y - 1.0).abs() < 1e-10);
            assert!((start_angle - 90.0).abs() < 1e-10);
            assert!((end_angle - 180.0).abs() < 1e-10);
        } else {
            panic!("Expected Arc");
        }
    }

    #[test]
    fn test_scale_text() {
        let transform = GeometryTransform;
        let text = Primitive::Text {
            content: "Test".to_string(),
            position: Point::new(10.0, 20.0),
            height: 12.0,
        };
        let result = transform.scale(vec![text], 0.5, [0.0, 0.0]);

        if let Primitive::Text {
            position, height, ..
        } = &result[0]
        {
            assert!((position.x - 5.0).abs() < 1e-10);
            assert!((position.y - 10.0).abs() < 1e-10);
            assert!((height - 6.0).abs() < 1e-10);
        } else {
            panic!("Expected Text");
        }
    }

    #[test]
    fn test_mirror_arc() {
        let transform = GeometryTransform;
        let arc = Primitive::Arc {
            center: Point::new(0.0, 0.0),
            radius: 5.0,
            start_angle: 30.0,
            end_angle: 60.0,
        };

        // X 轴镜像
        let result_x = transform.mirror(vec![arc.clone()], MirrorAxis::X);
        if let Primitive::Arc {
            start_angle,
            end_angle,
            ..
        } = &result_x[0]
        {
            assert!((start_angle - (-30.0)).abs() < 1e-10);
            assert!((end_angle - (-60.0)).abs() < 1e-10);
        } else {
            panic!("Expected Arc");
        }

        // Y 轴镜像
        let result_y = transform.mirror(vec![arc], MirrorAxis::Y);
        if let Primitive::Arc {
            start_angle,
            end_angle,
            ..
        } = &result_y[0]
        {
            assert!((start_angle - 150.0).abs() < 1e-10);
            assert!((end_angle - 120.0).abs() < 1e-10);
        } else {
            panic!("Expected Arc");
        }
    }
}
