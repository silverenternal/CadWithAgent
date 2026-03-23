//! JSON 导出器测试

#[cfg(test)]
mod tests {
    use crate::geometry::{Point, Primitive, Line, Circle, Polygon, Rect};
    use crate::export::json::{
        JsonExporter, JsonExportResult, JsonExportError,
        CotExportData, TrainingData, GroundTruth, QAPair, export_qa_dataset,
    };
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
    fn test_json_export_result_serialization() {
        let result = JsonExportResult {
            success: true,
            path: "/tmp/test.json".to_string(),
            entity_count: 10,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("success"));
        assert!(json.contains("path"));
        assert!(json.contains("entity_count"));

        let deserialized: JsonExportResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.success);
        assert_eq!(deserialized.entity_count, 10);
    }

    #[test]
    fn test_json_export_result_debug() {
        let result = JsonExportResult {
            success: true,
            path: "/tmp/test.json".to_string(),
            entity_count: 5,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("JsonExportResult"));
    }

    #[test]
    fn test_cot_export_data() {
        let primitives = create_test_primitives();
        let cot_data = CotExportData {
            primitives: primitives.clone(),
            thinking: "This is the thinking process".to_string(),
            answer: "This is the answer".to_string(),
        };

        let json = serde_json::to_string(&cot_data).unwrap();
        assert!(json.contains("thinking"));
        assert!(json.contains("answer"));
        assert!(json.contains("primitives"));

        let deserialized: CotExportData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.primitives.len(), 5);
        assert_eq!(deserialized.thinking, "This is the thinking process");
        assert_eq!(deserialized.answer, "This is the answer");
    }

    #[test]
    fn test_training_data_structure() {
        let primitives = create_test_primitives();
        let training_data = TrainingData {
            image: "/path/to/image.png".to_string(),
            instruction: "Detect all rooms".to_string(),
            grounding: GroundTruth {
                primitives: primitives.clone(),
            },
            thinking: "Analyzing the image...".to_string(),
            answer: "Found 3 rooms".to_string(),
        };

        let json = serde_json::to_string(&training_data).unwrap();
        assert!(json.contains("image"));
        assert!(json.contains("instruction"));
        assert!(json.contains("grounding"));
        assert!(json.contains("thinking"));
        assert!(json.contains("answer"));

        let deserialized: TrainingData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.image, "/path/to/image.png");
        assert_eq!(deserialized.grounding.primitives.len(), 5);
    }

    #[test]
    fn test_ground_truth_structure() {
        let primitives = create_test_primitives();
        let ground_truth = GroundTruth {
            primitives: primitives.clone(),
        };

        let json = serde_json::to_string(&ground_truth).unwrap();
        assert!(json.contains("primitives"));

        let deserialized: GroundTruth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.primitives.len(), 5);
    }

    #[test]
    fn test_qa_pair_structure() {
        let qa_pair = QAPair {
            question: "How many rooms are there?".to_string(),
            answer: "There are 3 rooms".to_string(),
            thinking: Some("Counting the rooms...".to_string()),
        };

        let json = serde_json::to_string(&qa_pair).unwrap();
        assert!(json.contains("question"));
        assert!(json.contains("answer"));
        assert!(json.contains("thinking"));

        let deserialized: QAPair = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.question, "How many rooms are there?");
        assert!(deserialized.thinking.is_some());
    }

    #[test]
    fn test_qa_pair_without_thinking() {
        let qa_pair = QAPair {
            question: "What color is the wall?".to_string(),
            answer: "White".to_string(),
            thinking: None,
        };

        let json = serde_json::to_string(&qa_pair).unwrap();
        assert!(json.contains("question"));
        assert!(json.contains("answer"));

        let deserialized: QAPair = serde_json::from_str(&json).unwrap();
        assert!(deserialized.thinking.is_none());
    }

    #[test]
    fn test_export_primitives_to_file() {
        let primitives = create_test_primitives();
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = JsonExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 5);

        // 验证文件内容（使用小写的类型名，因为 serde 使用 rename_all = "snake_case"）
        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("point"));
        assert!(content.contains("line"));

        // 验证可以反序列化
        let deserialized: Vec<Primitive> = serde_json::from_str(&content).unwrap();
        assert_eq!(deserialized.len(), 5);
    }

    #[test]
    fn test_export_with_cot() {
        let primitives = create_test_primitives();
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = JsonExporter::export_with_cot(
            &primitives,
            "Thinking about the geometry...",
            "The answer is 42",
            &temp_path,
        ).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 5);

        // 验证文件内容
        let content = fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("thinking"));
        assert!(content.contains("answer"));
        assert!(content.contains("primitives"));

        let deserialized: CotExportData = serde_json::from_str(&content).unwrap();
        assert_eq!(deserialized.primitives.len(), 5);
        assert_eq!(deserialized.thinking, "Thinking about the geometry...");
        assert_eq!(deserialized.answer, "The answer is 42");
    }

    #[test]
    fn test_export_qa_dataset() {
        let qa_pairs = vec![
            QAPair {
                question: "Q1".to_string(),
                answer: "A1".to_string(),
                thinking: Some("T1".to_string()),
            },
            QAPair {
                question: "Q2".to_string(),
                answer: "A2".to_string(),
                thinking: None,
            },
        ];

        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = export_qa_dataset(&qa_pairs, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 2);

        let content = fs::read_to_string(&temp_path).unwrap();
        let deserialized: Vec<QAPair> = serde_json::from_str(&content).unwrap();
        assert_eq!(deserialized.len(), 2);
    }

    #[test]
    fn test_json_export_error_io_error() {
        // 尝试写入一个无效路径
        let primitives = create_test_primitives();
        let invalid_path = "/nonexistent/directory/test.json";

        let result = JsonExporter::export(&primitives, invalid_path);
        assert!(result.is_err());

        if let Err(e) = result {
            let error_str = format!("{}", e);
            assert!(error_str.contains("文件写入失败") || error_str.contains("No such file"));
        }
    }

    #[test]
    fn test_json_export_result_clone() {
        let result = JsonExportResult {
            success: true,
            path: "/tmp/test.json".to_string(),
            entity_count: 10,
        };

        let cloned = result.clone();
        assert_eq!(cloned.success, result.success);
        assert_eq!(cloned.path, result.path);
        assert_eq!(cloned.entity_count, result.entity_count);
    }

    #[test]
    fn test_cot_export_data_clone() {
        let primitives = create_test_primitives();
        let cot_data = CotExportData {
            primitives: primitives.clone(),
            thinking: "Thinking...".to_string(),
            answer: "Answer".to_string(),
        };

        let cloned = cot_data.clone();
        assert_eq!(cloned.primitives.len(), cot_data.primitives.len());
        assert_eq!(cloned.thinking, cot_data.thinking);
    }

    #[test]
    fn test_training_data_clone() {
        let primitives = create_test_primitives();
        let training_data = TrainingData {
            image: "/img.png".to_string(),
            instruction: "Detect".to_string(),
            grounding: GroundTruth {
                primitives: primitives.clone(),
            },
            thinking: "Think".to_string(),
            answer: "Ans".to_string(),
        };

        let cloned = training_data.clone();
        assert_eq!(cloned.image, training_data.image);
        assert_eq!(cloned.grounding.primitives.len(), training_data.grounding.primitives.len());
    }

    #[test]
    fn test_qa_pair_clone() {
        let qa_pair = QAPair {
            question: "Q".to_string(),
            answer: "A".to_string(),
            thinking: Some("T".to_string()),
        };

        let cloned = qa_pair.clone();
        assert_eq!(cloned.question, qa_pair.question);
        assert_eq!(cloned.answer, qa_pair.answer);
    }

    #[test]
    fn test_export_empty_primitives() {
        let primitives: Vec<Primitive> = vec![];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = JsonExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 0);

        let content = fs::read_to_string(&temp_path).unwrap();
        let deserialized: Vec<Primitive> = serde_json::from_str(&content).unwrap();
        assert!(deserialized.is_empty());
    }

    #[test]
    fn test_export_single_point() {
        let primitives = vec![Primitive::Point(Point::new(5.0, 10.0))];
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        let result = JsonExporter::export(&primitives, &temp_path).unwrap();

        assert!(result.success);
        assert_eq!(result.entity_count, 1);

        let content = fs::read_to_string(&temp_path).unwrap();
        let deserialized: Vec<Primitive> = serde_json::from_str(&content).unwrap();
        assert_eq!(deserialized.len(), 1);

        if let Primitive::Point(p) = &deserialized[0] {
            assert!((p.x - 5.0).abs() < 1e-10);
            assert!((p.y - 10.0).abs() < 1e-10);
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_json_error_from_io() {
        use std::io;

        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
        let json_error: JsonExportError = JsonExportError::from(io_error);

        let error_str = format!("{}", json_error);
        assert!(error_str.contains("文件写入失败"));
    }

    #[test]
    fn test_json_error_from_json() {
        use serde_json;

        // 创建一个无效的 JSON 字符串来触发错误
        let invalid_json = "invalid json";
        let parse_error: serde_json::Error = serde_json::from_str::<serde_json::Value>(invalid_json).unwrap_err();
        let json_error: JsonExportError = JsonExportError::from(parse_error);

        let error_str = format!("{}", json_error);
        assert!(error_str.contains("JSON 序列化失败"));
    }
}
