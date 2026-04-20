//! 实验 Fixtures 模块
//!
//! 提供实验所需的测试数据和辅助函数。

#![allow(dead_code)]

use cadagent::prelude::*;
use std::path::PathBuf;

/// 获取 fixtures 目录路径
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from("tests/experiment/fixtures")
}

/// 获取数据目录路径
pub fn data_dir() -> PathBuf {
    PathBuf::from("tests/experiment/data")
}

/// 获取结果目录路径
pub fn results_dir() -> PathBuf {
    PathBuf::from("tests/experiment/results")
}

/// 创建测试用的简单几何图形
pub fn create_test_geometries() -> Vec<Primitive> {
    vec![
        // 线段
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([100.0, 0.0], [100.0, 100.0])),
        Primitive::Line(Line::from_coords([100.0, 100.0], [0.0, 100.0])),
        Primitive::Line(Line::from_coords([0.0, 100.0], [0.0, 0.0])),
        // 圆
        Primitive::Circle(Circle::from_coords([50.0, 50.0], 25.0)),
        // 点
        Primitive::Point(Point::new(0.0, 0.0)),
        Primitive::Point(Point::new(100.0, 100.0)),
    ]
}

/// 创建测试用的矩形
pub fn create_test_rect(width: f64, height: f64) -> Rect {
    Rect::from_coords([0.0, 0.0], [width, height])
}

/// 创建测试用的多边形
pub fn create_test_polygon(vertices: Vec<[f64; 2]>) -> Polygon {
    Polygon::from_coords(vertices)
}

/// 创建平行线测试数据
pub fn create_parallel_lines() -> Vec<Primitive> {
    vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([0.0, 50.0], [100.0, 50.0])),
    ]
}

/// 创建垂直线测试数据
pub fn create_perpendicular_lines() -> Vec<Primitive> {
    vec![
        Primitive::Line(Line::from_coords([0.0, 0.0], [100.0, 0.0])),
        Primitive::Line(Line::from_coords([50.0, 0.0], [50.0, 100.0])),
    ]
}

/// 创建同心圆测试数据
pub fn create_concentric_circles() -> Vec<Primitive> {
    vec![
        Primitive::Circle(Circle::from_coords([0.0, 0.0], 50.0)),
        Primitive::Circle(Circle::from_coords([0.0, 0.0], 30.0)),
    ]
}

/// 创建相切圆和线测试数据
pub fn create_tangent_circle_line() -> Vec<Primitive> {
    vec![
        Primitive::Circle(Circle::from_coords([0.0, 0.0], 50.0)),
        Primitive::Line(Line::from_coords([50.0, -50.0], [50.0, 50.0])),
    ]
}

/// 创建性能测试用的大规模几何数据
pub fn create_large_geometry_dataset(size: usize) -> Vec<Primitive> {
    (0..size)
        .map(|i| {
            let x = (i as f64 * 0.1).sin() * 1000.0;
            let y = (i as f64 * 0.13).cos() * 1000.0;
            if i % 3 == 0 {
                Primitive::Point(Point::new(x, y))
            } else if i % 3 == 1 {
                let x2 = ((i + 1) as f64 * 0.1).sin() * 1000.0;
                let y2 = ((i + 1) as f64 * 0.13).cos() * 1000.0;
                Primitive::Line(Line::from_coords([x, y], [x2, y2]))
            } else {
                Primitive::Circle(Circle::from_coords([x, y], 10.0))
            }
        })
        .collect()
}

/// 创建准确性测试用的标准几何图形
pub fn create_standard_geometries() -> Vec<(String, Primitive, f64)> {
    vec![
        // (名称，几何图形，预期面积)
        (
            "unit_square".to_string(),
            Primitive::Polygon(create_test_polygon(vec![
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
            ])),
            1.0,
        ),
        (
            "circle_r1".to_string(),
            Primitive::Circle(Circle::from_coords([0.0, 0.0], 1.0)),
            std::f64::consts::PI,
        ),
        (
            "triangle_3_4_5".to_string(),
            Primitive::Polygon(create_test_polygon(vec![
                [0.0, 0.0],
                [3.0, 0.0],
                [0.0, 4.0],
            ])),
            6.0,
        ),
    ]
}

/// VLM 推理测试用例
#[derive(Debug, Clone)]
pub struct VLMTestCase {
    /// 测试用例 ID
    pub id: usize,
    /// 输入图像路径 (模拟)
    pub image_path: String,
    /// 问题描述
    pub question: String,
    /// 预期答案
    pub expected_answer: String,
    /// 预期推理步骤
    pub expected_reasoning_steps: Vec<String>,
    /// 难度等级
    pub difficulty: DifficultyLevel,
}

/// 难度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// 创建 VLM 推理测试用例
pub fn create_vlm_test_cases() -> Vec<VLMTestCase> {
    vec![
        VLMTestCase {
            id: 1,
            image_path: "fixtures/vlm/geometry_basic.png".to_string(),
            question: "这个图形中有几个三角形？".to_string(),
            expected_answer: "3 个三角形".to_string(),
            expected_reasoning_steps: vec![
                "识别图形中的所有线段".to_string(),
                "找出所有由三条线段组成的闭合区域".to_string(),
                "统计三角形数量".to_string(),
            ],
            difficulty: DifficultyLevel::Easy,
        },
        VLMTestCase {
            id: 2,
            image_path: "fixtures/vlm/relations.png".to_string(),
            question: "图中有哪些平行线段？".to_string(),
            expected_answer: "AB 平行于 CD, EF 平行于 GH".to_string(),
            expected_reasoning_steps: vec![
                "识别所有线段及其端点".to_string(),
                "计算每条线段的方向向量".to_string(),
                "比较方向向量，找出平行的线段对".to_string(),
            ],
            difficulty: DifficultyLevel::Medium,
        },
        VLMTestCase {
            id: 3,
            image_path: "fixtures/vlm/complex_cad.png".to_string(),
            question: "这个零件的壁厚是否均匀？哪里需要加强？".to_string(),
            expected_answer: "壁厚不均匀，左侧区域需要加强".to_string(),
            expected_reasoning_steps: vec![
                "提取零件的内外轮廓".to_string(),
                "计算各点的壁厚".to_string(),
                "比较壁厚分布".to_string(),
                "识别薄弱区域".to_string(),
            ],
            difficulty: DifficultyLevel::Hard,
        },
    ]
}

/// 消融实验配置
#[derive(Debug, Clone)]
pub struct AblationConfig {
    /// 配置名称
    pub name: String,
    /// 是否启用空间索引
    pub enable_spatial_index: bool,
    /// 是否启用工具增强
    pub enable_tool_augmentation: bool,
    /// 是否启用上下文注入
    pub enable_context_injection: bool,
    /// 是否启用几何验证
    pub enable_geometry_verification: bool,
}

impl AblationConfig {
    pub fn full() -> Self {
        Self {
            name: "Full System".to_string(),
            enable_spatial_index: true,
            enable_tool_augmentation: true,
            enable_context_injection: true,
            enable_geometry_verification: true,
        }
    }

    pub fn without_spatial_index() -> Self {
        Self {
            name: "Without Spatial Index".to_string(),
            enable_spatial_index: false,
            enable_tool_augmentation: true,
            enable_context_injection: true,
            enable_geometry_verification: true,
        }
    }

    pub fn without_tool_augmentation() -> Self {
        Self {
            name: "Without Tool Augmentation".to_string(),
            enable_spatial_index: true,
            enable_tool_augmentation: false,
            enable_context_injection: true,
            enable_geometry_verification: true,
        }
    }

    pub fn without_context_injection() -> Self {
        Self {
            name: "Without Context Injection".to_string(),
            enable_spatial_index: true,
            enable_tool_augmentation: true,
            enable_context_injection: false,
            enable_geometry_verification: true,
        }
    }

    pub fn without_geometry_verification() -> Self {
        Self {
            name: "Without Geometry Verification".to_string(),
            enable_spatial_index: true,
            enable_tool_augmentation: true,
            enable_context_injection: true,
            enable_geometry_verification: false,
        }
    }
}

/// 获取所有消融实验配置
pub fn get_all_ablation_configs() -> Vec<AblationConfig> {
    vec![
        AblationConfig::full(),
        AblationConfig::without_spatial_index(),
        AblationConfig::without_tool_augmentation(),
        AblationConfig::without_context_injection(),
        AblationConfig::without_geometry_verification(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_geometries() {
        let geometries = create_test_geometries();
        assert!(geometries.len() >= 5);
    }

    #[test]
    fn test_create_parallel_lines() {
        let lines = create_parallel_lines();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_create_vlm_test_cases() {
        let cases = create_vlm_test_cases();
        assert_eq!(cases.len(), 3);
        assert_eq!(cases[0].difficulty, DifficultyLevel::Easy);
    }

    #[test]
    fn test_ablation_configs() {
        let configs = get_all_ablation_configs();
        assert_eq!(configs.len(), 5);
        assert!(configs[0].enable_spatial_index);
        assert!(!configs[1].enable_spatial_index);
    }
}
