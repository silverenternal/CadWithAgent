//! 实验数据模块
//!
//! 提供实验所需的原始数据和预处理数据。

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 获取数据目录路径
pub fn data_dir() -> PathBuf {
    PathBuf::from("tests/experiment/data")
}

/// 准确性测试数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyTestData {
    /// 测试用例 ID
    pub id: String,
    /// 测试类型
    pub test_type: String,
    /// 输入数据
    pub input: serde_json::Value,
    /// 预期输出
    pub expected: serde_json::Value,
    /// 容差
    pub tolerance: f64,
}

/// 性能测试数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTestData {
    /// 测试用例 ID
    pub id: String,
    /// 数据规模
    pub size: usize,
    /// 几何元素
    pub elements: Vec<GeometryElement>,
}

/// 几何元素
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GeometryElement {
    Point { x: f64, y: f64 },
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
    Circle { cx: f64, cy: f64, r: f64 },
    Polygon { vertices: Vec<[f64; 2]> },
}

/// VLM 测试数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VLMTestData {
    /// 测试用例 ID
    pub id: String,
    /// 图像描述
    pub image_description: String,
    /// 问题
    pub question: String,
    /// 预期答案
    pub expected_answer: String,
    /// 预期推理步骤
    pub expected_reasoning_steps: Vec<String>,
    /// 难度等级
    pub difficulty: String,
}

/// 案例研究数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseStudyData {
    /// 案例 ID
    pub id: String,
    /// 案例名称
    pub name: String,
    /// 案例描述
    pub description: String,
    /// 输入文件路径
    pub input_files: Vec<String>,
    /// 预期输出
    pub expected_output: serde_json::Value,
    /// 评估标准
    pub evaluation_criteria: Vec<String>,
}

/// 加载准确性测试数据
pub fn load_accuracy_data() -> Vec<AccuracyTestData> {
    let path = data_dir().join("accuracy_test_data.json");

    // 如果文件不存在，返回生成的测试数据
    if !path.exists() {
        return generate_accuracy_data();
    }

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| generate_accuracy_data()),
        Err(_) => generate_accuracy_data(),
    }
}

/// 生成准确性测试数据
pub fn generate_accuracy_data() -> Vec<AccuracyTestData> {
    vec![
        AccuracyTestData {
            id: "length_001".to_string(),
            test_type: "length_measurement".to_string(),
            input: serde_json::json!({
                "x1": 0.0, "y1": 0.0,
                "x2": 3.0, "y2": 4.0
            }),
            expected: serde_json::json!({
                "length": 5.0
            }),
            tolerance: 1e-10,
        },
        AccuracyTestData {
            id: "area_001".to_string(),
            test_type: "area_measurement".to_string(),
            input: serde_json::json!({
                "type": "rectangle",
                "width": 10.0,
                "height": 5.0
            }),
            expected: serde_json::json!({
                "area": 50.0
            }),
            tolerance: 1e-10,
        },
        AccuracyTestData {
            id: "angle_001".to_string(),
            test_type: "angle_measurement".to_string(),
            input: serde_json::json!({
                "p1": [0.0, 0.0],
                "p2": [1.0, 0.0],
                "p3": [0.0, 1.0]
            }),
            expected: serde_json::json!({
                "angle_deg": 90.0
            }),
            tolerance: 1e-8,
        },
    ]
}

/// 加载性能测试数据
pub fn load_performance_data() -> Vec<PerformanceTestData> {
    let sizes = [100, 500, 1000, 5000, 10000];

    sizes
        .iter()
        .map(|&size| {
            let elements: Vec<GeometryElement> = (0..size)
                .map(|i| {
                    let x = (i as f64 * 0.1).sin() * 1000.0;
                    let y = (i as f64 * 0.13).cos() * 1000.0;
                    if i % 3 == 0 {
                        GeometryElement::Point { x, y }
                    } else if i % 3 == 1 {
                        let x2 = ((i + 1) as f64 * 0.1).sin() * 1000.0;
                        let y2 = ((i + 1) as f64 * 0.13).cos() * 1000.0;
                        GeometryElement::Line {
                            x1: x,
                            y1: y,
                            x2,
                            y2,
                        }
                    } else {
                        GeometryElement::Circle {
                            cx: x,
                            cy: y,
                            r: 10.0,
                        }
                    }
                })
                .collect();

            PerformanceTestData {
                id: format!("perf_{}", size),
                size,
                elements,
            }
        })
        .collect()
}

/// 加载 VLM 测试数据
pub fn load_vlm_data() -> Vec<VLMTestData> {
    vec![
        VLMTestData {
            id: "vlm_001".to_string(),
            image_description: "一个简单的几何图形，包含一个正方形和一个内切圆".to_string(),
            question: "正方形的边长是圆的直径的多少倍？".to_string(),
            expected_answer: "1 倍，因为圆内切于正方形，所以正方形边长等于圆的直径".to_string(),
            expected_reasoning_steps: vec![
                "识别正方形的四条边".to_string(),
                "识别内切圆".to_string(),
                "理解内切的几何关系".to_string(),
                "得出边长等于直径的结论".to_string(),
            ],
            difficulty: "easy".to_string(),
        },
        VLMTestData {
            id: "vlm_002".to_string(),
            image_description: "一个机械零件的三视图".to_string(),
            question: "这个零件有多少个孔？".to_string(),
            expected_answer: "4 个孔".to_string(),
            expected_reasoning_steps: vec![
                "分析主视图中的圆形特征".to_string(),
                "分析俯视图中的圆形特征".to_string(),
                "分析侧视图中的圆形特征".to_string(),
                "综合三视图，确定孔的数量".to_string(),
            ],
            difficulty: "medium".to_string(),
        },
        VLMTestData {
            id: "vlm_003".to_string(),
            image_description: "一个复杂的装配体图纸".to_string(),
            question: "这个装配体中哪些零件需要修改以提高可制造性？".to_string(),
            expected_answer: "零件 A 的壁厚过薄，零件 B 的倒角半径过小".to_string(),
            expected_reasoning_steps: vec![
                "识别所有零件及其几何特征".to_string(),
                "分析各零件的制造约束".to_string(),
                "识别潜在的制造问题".to_string(),
                "提出改进建议".to_string(),
            ],
            difficulty: "hard".to_string(),
        },
    ]
}

/// 加载案例研究数据
pub fn load_case_study_data() -> Vec<CaseStudyData> {
    vec![
        CaseStudyData {
            id: "case_001".to_string(),
            name: "机械零件图处理".to_string(),
            description: "处理一个包含 156 个几何元素的机械零件 DXF 图纸".to_string(),
            input_files: vec!["mechanical_part.dxf".to_string()],
            expected_output: serde_json::json!({
                "elements_count": 156,
                "dimensions_count": 45,
                "accuracy_threshold": 0.95
            }),
            evaluation_criteria: vec![
                "几何元素提取完整性".to_string(),
                "尺寸标注识别准确率".to_string(),
                "几何关系检测正确率".to_string(),
            ],
        },
        CaseStudyData {
            id: "case_002".to_string(),
            name: "建筑平面图处理".to_string(),
            description: "处理一个建筑平面图 SVG，识别房间、墙体、门窗".to_string(),
            input_files: vec!["floor_plan.svg".to_string()],
            expected_output: serde_json::json!({
                "rooms_count": 12,
                "walls_count": 48,
                "doors_count": 15,
                "windows_count": 24
            }),
            evaluation_criteria: vec![
                "房间识别准确率".to_string(),
                "墙体连接正确率".to_string(),
                "门窗位置准确率".to_string(),
            ],
        },
        CaseStudyData {
            id: "case_003".to_string(),
            name: "电路原理图处理".to_string(),
            description: "处理电路原理图，识别元件和连接关系".to_string(),
            input_files: vec!["circuit_diagram.png".to_string()],
            expected_output: serde_json::json!({
                "components_count": 45,
                "connections_count": 78,
                "nets_count": 12
            }),
            evaluation_criteria: vec![
                "元件识别准确率".to_string(),
                "连接关系正确率".to_string(),
                "网络表生成正确率".to_string(),
            ],
        },
    ]
}

/// 保存实验数据到文件
pub fn save_data<T: Serialize>(data: &T, filename: &str) -> std::io::Result<()> {
    let path = data_dir().join(filename);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(data)?;
    fs::write(path, content)
}

/// 加载或生成基准测试数据
pub fn get_benchmark_data() -> HashMap<String, Vec<f64>> {
    // 模拟各对比方法的基准性能数据
    let mut data = HashMap::new();

    data.insert("CadAgent".to_string(), vec![0.95, 0.94, 0.96, 0.95, 0.93]);
    data.insert("AutoCAD".to_string(), vec![0.93, 0.92, 0.94, 0.93, 0.92]);
    data.insert("LibreCAD".to_string(), vec![0.85, 0.84, 0.86, 0.85, 0.83]);
    data.insert(
        "Traditional".to_string(),
        vec![0.78, 0.76, 0.79, 0.77, 0.75],
    );

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_accuracy_data() {
        let data = generate_accuracy_data();
        assert!(!data.is_empty());
        assert_eq!(data[0].test_type, "length_measurement");
    }

    #[test]
    fn test_load_performance_data() {
        let data = load_performance_data();
        assert_eq!(data.len(), 5);
        assert_eq!(data[0].size, 100);
    }

    #[test]
    fn test_load_vlm_data() {
        let data = load_vlm_data();
        assert_eq!(data.len(), 3);
        assert_eq!(data[0].difficulty, "easy");
    }

    #[test]
    fn test_get_benchmark_data() {
        let data = get_benchmark_data();
        assert!(data.contains_key("CadAgent"));
        assert_eq!(data.get("CadAgent").unwrap().len(), 5);
    }
}
