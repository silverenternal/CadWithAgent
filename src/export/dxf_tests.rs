//! DXF 导出器测试

#[cfg(test)]
mod tests {
    use crate::export::dxf::{DxfExportError, DxfExportResult, DxfExporter};
    use crate::geometry::{Circle, Line, Point, Polygon, Primitive, Rect};
    use std::fs;
    use tempfile::NamedTempFile;

    fn create_test_primitives() -> Vec<Primitive> {
        vec![
            Primitive::Point(Point::new(1.0, 2.0)),
            Primitive::Line(Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))),
            Primitive::Circle(Circle::new(Point::new(0.0, 0.0), 5.0)),
            Primitive::Rect(Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 3.0))),
            Primitive::Polygon(Polygon::new(vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(1.0, 1.0),
            ])),
        ]
    }

    #[test]
    fn test_dxf_export_result_serialization() {
        let result = DxfExportResult {
            success: true,
            path: "/tmp/test.dxf".to_string(),
            entity_count: 10,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("path"));
        assert!(json.contains("entity_count"));

        let deserialized: DxfExportResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.entity_count, 10);
    }

    #[test]
    fn test_dxf_export_result_debug() {
        let result = DxfExportResult {
            success: true,
            path: "/tmp/test.dxf".to_string(),
            entity_count: 5,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("DxfExportResult"));
    }

    #[test]
    fn test_dxf_export_result_clone() {
        let result = DxfExportResult {
            success: true,
            path: "/tmp/test.dxf".to_string(),
            entity_count: 10,
        };

        let cloned = result.clone();
        assert_eq!(cloned.success, result.success);
        assert_eq!(cloned.path, result.path);
        assert_eq!(cloned.entity_count, result.entity_count);
    }

    #[test]
    fn test_export_primitives_to_dxf() {
        let primitives = create_test_primitives();
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 5);

        // 验证 DXF 文件内容
        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("SECTION"));
        assert!(content.contains("ENTITIES"));
        assert!(content.contains("EOF"));
        assert!(content.contains("POINT"));
        assert!(content.contains("LINE"));
        assert!(content.contains("CIRCLE"));
    }

    #[test]
    fn test_export_point() {
        let primitives = vec![Primitive::Point(Point::new(5.0, 10.0))];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("POINT"));
        assert!(content.contains("5"));
        assert!(content.contains("10"));
    }

    #[test]
    fn test_export_line() {
        let primitives = vec![Primitive::Line(Line::new(
            Point::new(0.0, 0.0),
            Point::new(5.0, 5.0),
        ))];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("LINE"));
    }

    #[test]
    fn test_export_circle() {
        let primitives = vec![Primitive::Circle(Circle::new(Point::new(0.0, 0.0), 10.0))];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("CIRCLE"));
        assert!(content.contains("10")); // 半径
    }

    #[test]
    fn test_export_polygon() {
        let primitives = vec![Primitive::Polygon(Polygon::new(vec![
            Point::new(0.0, 0.0),
            Point::new(5.0, 0.0),
            Point::new(5.0, 5.0),
            Point::new(0.0, 5.0),
        ]))];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        // 多边形应该被转换为 4 条 LINE 实体
        let line_count = content.matches("LINE").count();
        assert_eq!(line_count, 4);
    }

    #[test]
    fn test_export_rect() {
        let primitives = vec![Primitive::Rect(Rect::new(
            Point::new(0.0, 0.0),
            Point::new(10.0, 10.0),
        ))];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        // 矩形应该被转换为 4 条 LINE 实体
        let line_count = content.matches("LINE").count();
        assert_eq!(line_count, 4);
    }

    #[test]
    fn test_export_arc() {
        let primitives = vec![Primitive::Arc {
            center: Point::new(0.0, 0.0),
            radius: 10.0,
            start_angle: 0.0,
            end_angle: 90.0,
        }];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("ARC"));
        assert!(content.contains("90"));
    }

    #[test]
    fn test_export_text() {
        let primitives = vec![Primitive::Text {
            content: "Hello".to_string(),
            position: Point::new(5.0, 5.0),
            height: 12.0,
        }];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("TEXT"));
        assert!(content.contains("Hello"));
    }

    #[test]
    fn test_export_polyline_open() {
        let primitives = vec![Primitive::Polyline {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(5.0, 5.0),
                Point::new(10.0, 0.0),
            ],
            closed: false,
        }];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        // 开放的折线应该有 2 条 LINE 实体
        let line_count = content.matches("LINE").count();
        assert_eq!(line_count, 2);
    }

    #[test]
    fn test_export_polyline_closed() {
        let primitives = vec![Primitive::Polyline {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(5.0, 5.0),
                Point::new(10.0, 0.0),
            ],
            closed: true,
        }];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        // 闭合的折线应该有 3 条 LINE 实体
        let line_count = content.matches("LINE").count();
        assert_eq!(line_count, 3);
    }

    #[test]
    fn test_export_empty_primitives() {
        let primitives: Vec<Primitive> = vec![];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 0);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("SECTION"));
        assert!(content.contains("EOF"));
    }

    #[test]
    fn test_export_error_io_error() {
        let primitives = create_test_primitives();
        let invalid_path = "/nonexistent/directory/test.dxf";

        let result = DxfExporter::export(&primitives, invalid_path);
        assert!(result.is_err());

        if let Err(e) = result {
            let error_str = format!("{}", e);
            assert!(error_str.contains("文件写入失败") || error_str.contains("No such file"));
        }
    }

    #[test]
    fn test_dxf_error_from_io() {
        use std::io;

        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
        let dxf_error: DxfExportError = DxfExportError::from(io_error);

        let error_str = format!("{}", dxf_error);
        assert!(error_str.contains("文件写入失败"));
    }

    #[test]
    fn test_dxf_error_from_json() {
        let invalid_json = "invalid json";
        let parse_error: serde_json::Error =
            serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let dxf_error: DxfExportError = DxfExportError::from(parse_error);

        let error_str = format!("{}", dxf_error);
        assert!(error_str.contains("JSON 解析失败"));
    }

    #[test]
    fn test_dxf_error_custom() {
        let dxf_error = DxfExportError::DxfError("Custom error".to_string());
        let error_str = format!("{}", dxf_error);
        assert!(error_str.contains("Custom error"));
        assert!(error_str.contains("DXF 格式错误"));
    }

    #[test]
    fn test_dxf_document_structure() {
        let primitives = create_test_primitives();
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);

        // 验证 DXF 文件结构
        let content = fs::read_to_string(&temp_path).unwrap();

        // 检查 DXF 文件头
        assert!(content.contains("SECTION"));
        assert!(content.contains("HEADER"));
        assert!(content.contains("ENDSEC"));

        // 检查表部分
        assert!(content.contains("TABLES"));

        // 检查实体部分
        assert!(content.contains("ENTITIES"));

        // 检查文件结束标记
        assert!(content.contains("EOF"));
    }

    #[test]
    fn test_export_from_json() {
        let primitives = create_test_primitives();
        let json_str = serde_json::to_string(&primitives).unwrap();

        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export_from_json(&json_str, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 5);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("ENTITIES"));
        assert!(content.contains("EOF"));
    }

    #[test]
    fn test_export_from_json_invalid() {
        let invalid_json = "invalid json";
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export_from_json(invalid_json, &temp_path);
        assert!(result.is_err());

        if let Err(e) = result {
            let error_str = format!("{}", e);
            assert!(error_str.contains("JSON 解析失败"));
        }
    }

    #[test]
    fn test_dxf_export_all_primitive_types() {
        let primitives = vec![
            Primitive::Point(Point::new(0.0, 0.0)),
            Primitive::Line(Line::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))),
            Primitive::Polygon(Polygon::new(vec![
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0),
                Point::new(1.0, 1.0),
            ])),
            Primitive::Circle(Circle::new(Point::new(0.0, 0.0), 5.0)),
            Primitive::Rect(Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 3.0))),
            Primitive::Polyline {
                points: vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)],
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
        ];

        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = DxfExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 8);

        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("POINT"));
        assert!(content.contains("LINE"));
        assert!(content.contains("CIRCLE"));
        assert!(content.contains("ARC"));
        assert!(content.contains("TEXT"));
    }
}
