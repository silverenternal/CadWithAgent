//! Geo-CoT 生成器
//!
//! 基于几何图元自动生成思维链数据

use crate::cot::templates::{PerceptionTemplate, ReasoningTemplate, SummaryTemplate};
use crate::geometry::{Point, Polygon, Primitive, Room};
use serde::{Deserialize, Serialize};

/// Geo-CoT 生成器
pub struct GeoCotGenerator {
    templates: GeoCotTemplates,
}

impl GeoCotGenerator {
    pub fn new() -> Self {
        Self {
            templates: GeoCotTemplates::default(),
        }
    }

    /// 生成完整的 Geo-CoT 数据
    pub fn generate(&self, primitives: &[Primitive], task: &str) -> GeoCotData {
        let perception = self.generate_perception(primitives);
        let reasoning = self.generate_reasoning(primitives, task);
        let summary = self.generate_summary(primitives, task);

        GeoCotData {
            task: task.to_string(),
            perception: perception.clone(),
            reasoning: reasoning.clone(),
            summary: summary.clone(),
            thinking: self.format_thinking(&perception, &reasoning),
            answer: summary,
        }
    }

    /// 生成感知阶段文本
    fn generate_perception(&self, primitives: &[Primitive]) -> String {
        let mut observations = Vec::new();

        // 检测外墙轮廓
        let outer_boundary = self.find_outer_boundary(primitives);
        if let Some(boundary) = &outer_boundary {
            let coords = self.format_coords(&boundary.vertices);
            observations.push(self.templates.perception.format(
                "外墙",
                &coords,
                boundary.vertices.len(),
            ));
        }

        // 检测内部房间
        let rooms = self.find_rooms(primitives);
        for (i, room) in rooms.iter().enumerate() {
            let coords = self.format_coords(&room.boundary.vertices);
            observations.push(self.templates.perception.format(
                &format!("房间{}", i + 1),
                &coords,
                room.boundary.vertices.len(),
            ));
        }

        // 检测门
        let doors: Vec<_> = primitives
            .iter()
            .filter_map(|p| match p {
                Primitive::Text {
                    content, position, ..
                } if content.contains("门") || content.to_lowercase() == "d" => {
                    Some((*position, content.clone()))
                }
                _ => None,
            })
            .collect();

        for (pos, content) in &doors {
            observations.push(format!(
                "在坐标 [{:.0}, {:.0}] 处检测到文本标记'{}'，可能是门的位置",
                pos.x, pos.y, content
            ));
        }

        observations.join("\n")
    }

    /// 生成推理阶段文本
    fn generate_reasoning(&self, primitives: &[Primitive], task: &str) -> String {
        let mut reasoning_steps = Vec::new();

        // 根据任务类型生成不同的推理步骤
        if task.contains("面积") || task.contains("大小") {
            reasoning_steps.push(self.templates.reasoning.format(
                "面积计算",
                "使用鞋带公式计算多边形面积",
                "Sum(x_i * y_{i+1} - x_{i+1} * y_i) / 2",
            ));

            let rooms = self.find_rooms(primitives);
            for room in &rooms {
                reasoning_steps.push(format!(
                    "房间'{}'的边界由{}个顶点组成，计算得到面积为{:.2}平方单位",
                    room.name,
                    room.boundary.vertices.len(),
                    room.area
                ));
            }
        }

        if task.contains("房间") || task.contains("数量") {
            let rooms = self.find_rooms(primitives);
            reasoning_steps.push(self.templates.reasoning.format(
                "房间检测",
                "查找所有闭合回路，排除外边界",
                &format!("检测到{}个房间", rooms.len()),
            ));
        }

        if task.contains("门") || task.contains("窗") {
            reasoning_steps.push(self.templates.reasoning.format(
                "门窗检测",
                "检测墙体上的缺口和文本标记",
                "识别门和窗户的位置",
            ));
        }

        if task.contains("宽度") || task.contains("长度") {
            reasoning_steps.push(self.templates.reasoning.format(
                "尺寸测量",
                "查找边界坐标，计算差值",
                "宽度 = x_max - x_min",
            ));
        }

        reasoning_steps.join("\n")
    }

    /// 生成总结阶段文本
    fn generate_summary(&self, primitives: &[Primitive], task: &str) -> String {
        if task.contains("房间") && task.contains("数量") {
            let rooms = self.find_rooms(primitives);
            return format!("该户型图共有{}个房间。", rooms.len());
        }

        if task.contains("面积") {
            let rooms = self.find_rooms(primitives);
            let total_area: f64 = rooms.iter().map(|r| r.area).sum();
            return format!(
                "总面积为{:.2}平方单位。各房间面积：{}",
                total_area,
                rooms
                    .iter()
                    .map(|r| format!("{}: {:.2}", r.name, r.area))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        if task.contains("宽度") {
            if let Some(boundary) = self.find_outer_boundary(primitives) {
                let bbox = boundary.vertices.iter().fold(
                    (
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ),
                    |(min_x, min_y, max_x, max_y), p| {
                        (
                            min_x.min(p.x),
                            min_y.min(p.y),
                            max_x.max(p.x),
                            max_y.max(p.y),
                        )
                    },
                );
                let width = bbox.2 - bbox.0;
                return format!(
                    "建筑总宽度为{:.2}单位（从 x={:.2}到 x={:.2}）。",
                    width, bbox.0, bbox.2
                );
            }
        }

        // 默认总结
        let rooms = self.find_rooms(primitives);
        format!(
            "检测到{}个房间，外边界由{}个点组成。",
            rooms.len(),
            self.find_outer_boundary(primitives)
                .map_or(0, |p| p.vertices.len())
        )
    }

    /// 格式化思维链
    fn format_thinking(&self, perception: &str, reasoning: &str) -> String {
        format!("<thinking>\n{perception}\n\n{reasoning}\n</thinking>")
    }

    /// 格式化坐标
    fn format_coords(&self, vertices: &[Point]) -> String {
        vertices
            .iter()
            .map(|p| format!("[{:.0}, {:.0}]", p.x, p.y))
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    fn find_outer_boundary(&self, primitives: &[Primitive]) -> Option<Polygon> {
        // 简化实现：查找面积最大的闭合回路
        let loops = self.find_all_loops(primitives);
        loops.into_iter().max_by(|a, b| {
            a.area()
                .partial_cmp(&b.area())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    fn find_rooms(&self, primitives: &[Primitive]) -> Vec<Room> {
        // 简化实现：查找所有闭合回路（排除最大的外边界）
        let mut loops = self.find_all_loops(primitives);

        if loops.is_empty() {
            return vec![];
        }

        // 移除最大的（外边界）
        if loops.len() > 1 {
            loops.sort_by(|a, b| {
                b.area()
                    .partial_cmp(&a.area())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            loops.remove(0);
        }

        loops
            .into_iter()
            .enumerate()
            .map(|(i, boundary)| Room {
                name: format!("房间{}", i + 1),
                boundary: boundary.clone(),
                area: boundary.area(),
                doors: vec![],
                windows: vec![],
            })
            .collect()
    }

    fn find_all_loops(&self, primitives: &[Primitive]) -> Vec<Polygon> {
        // 简化实现：从 Line 和 Polygon 图元中提取回路
        let mut loops = Vec::new();

        for prim in primitives {
            if let Primitive::Polygon(poly) = prim {
                if poly.vertices.len() >= 3 {
                    loops.push(poly.clone());
                }
            }
        }

        loops
    }
}

impl Default for GeoCotGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Geo-CoT 数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoCotData {
    pub task: String,
    pub perception: String,
    pub reasoning: String,
    pub summary: String,
    pub thinking: String,
    pub answer: String,
}

/// Geo-CoT 模板集合
#[derive(Default)]
struct GeoCotTemplates {
    perception: PerceptionTemplate,
    reasoning: ReasoningTemplate,
    #[allow(dead_code)]
    summary: SummaryTemplate,
}

/// 使用 tokitai 工具封装
#[derive(Default, Clone)]
pub struct GeoCotTools;

use tokitai::tool;

#[tool]
impl GeoCotTools {
    /// 生成 Geo-CoT 数据
    #[tool]
    pub fn generate_geo_cot(&self, primitives: Vec<Primitive>, task: String) -> GeoCotData {
        let generator = GeoCotGenerator::new();
        generator.generate(&primitives, &task)
    }

    /// 生成感知文本
    #[tool]
    pub fn generate_perception(&self, primitives: Vec<Primitive>) -> String {
        let generator = GeoCotGenerator::new();
        generator.generate_perception(&primitives)
    }

    /// 生成推理文本
    #[tool]
    pub fn generate_reasoning(&self, primitives: Vec<Primitive>, task: String) -> String {
        let generator = GeoCotGenerator::new();
        generator.generate_reasoning(&primitives, &task)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Line;

    fn create_test_polygon() -> Polygon {
        Polygon::from_coords(vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]])
    }

    #[test]
    fn test_generator_new() {
        let generator = GeoCotGenerator::new();
        assert!(!generator.templates.perception.pattern.is_empty());
    }

    #[test]
    fn test_generator_default() {
        let generator = GeoCotGenerator::default();
        assert!(!generator.templates.perception.pattern.is_empty());
    }

    #[test]
    fn test_generate_area_task() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![Primitive::Polygon(create_test_polygon())];

        let result = generator.generate(&primitives, "计算房间面积");

        assert_eq!(result.task, "计算房间面积");
        assert!(result.perception.contains("房间"));
        assert!(result.reasoning.contains("面积计算") || result.reasoning.contains("鞋带公式"));
        assert!(result.summary.contains("面积"));
        assert!(result.thinking.contains("<thinking>"));
        assert!(result.answer.contains("面积"));
    }

    #[test]
    fn test_generate_room_count_task() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![Primitive::Polygon(create_test_polygon())];

        let result = generator.generate(&primitives, "统计房间数量");

        assert_eq!(result.task, "统计房间数量");
        assert!(result.summary.contains("房间"));
    }

    #[test]
    fn test_generate_width_task() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![Primitive::Polygon(create_test_polygon())];

        let result = generator.generate(&primitives, "计算建筑宽度");

        assert!(result.summary.contains("宽度"));
    }

    #[test]
    fn test_generate_door_task() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![
            Primitive::Polygon(create_test_polygon()),
            Primitive::Text {
                content: "门".to_string(),
                position: Point { x: 50.0, y: 0.0 },
                height: 10.0,
            },
        ];

        let result = generator.generate(&primitives, "检测门的位置");

        assert!(result.reasoning.contains("门窗检测"));
    }

    #[test]
    fn test_generate_dimension_task() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![Primitive::Polygon(create_test_polygon())];

        let result = generator.generate(&primitives, "测量房间宽度");

        assert!(result.reasoning.contains("尺寸测量"));
    }

    #[test]
    fn test_generate_empty_primitives() {
        let generator = GeoCotGenerator::new();
        let primitives: Vec<Primitive> = vec![];

        let result = generator.generate(&primitives, "分析户型图");

        assert_eq!(result.task, "分析户型图");
        // 空基元时，房间数量为 0
        assert!(result.summary.contains("0"));
    }

    #[test]
    fn test_generate_multiple_rooms() {
        let generator = GeoCotGenerator::new();
        let room1 = Polygon::from_coords(vec![[0.0, 0.0], [50.0, 0.0], [50.0, 50.0], [0.0, 50.0]]);
        let room2 =
            Polygon::from_coords(vec![[60.0, 0.0], [100.0, 0.0], [100.0, 50.0], [60.0, 50.0]]);
        let primitives = vec![Primitive::Polygon(room1), Primitive::Polygon(room2)];

        let result = generator.generate(&primitives, "分析所有房间");

        assert!(result.perception.contains("房间"));
    }

    #[test]
    fn test_format_coords() {
        let generator = GeoCotGenerator::new();
        let points = vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 100.0 }];

        let formatted = generator.format_coords(&points);

        assert!(formatted.contains("[0, 0]"));
        assert!(formatted.contains("[100, 100]"));
        assert!(formatted.contains(" -> "));
    }

    #[test]
    fn test_find_outer_boundary() {
        let generator = GeoCotGenerator::new();
        let large_room =
            Polygon::from_coords(vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]]);
        let small_room =
            Polygon::from_coords(vec![[10.0, 10.0], [50.0, 10.0], [50.0, 50.0], [10.0, 50.0]]);
        let primitives = vec![
            Primitive::Polygon(large_room),
            Primitive::Polygon(small_room),
        ];

        let boundary = generator.find_outer_boundary(&primitives);

        assert!(boundary.is_some());
        assert_eq!(boundary.unwrap().vertices.len(), 4);
    }

    #[test]
    fn test_find_rooms() {
        let generator = GeoCotGenerator::new();
        let outer =
            Polygon::from_coords(vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]);
        let inner =
            Polygon::from_coords(vec![[10.0, 10.0], [50.0, 10.0], [50.0, 50.0], [10.0, 50.0]]);
        let primitives = vec![Primitive::Polygon(outer), Primitive::Polygon(inner)];

        let rooms = generator.find_rooms(&primitives);

        assert_eq!(rooms.len(), 1);
        assert!(rooms[0].name.contains("房间"));
    }

    #[test]
    fn test_find_rooms_empty() {
        let generator = GeoCotGenerator::new();
        let primitives: Vec<Primitive> = vec![];

        let rooms = generator.find_rooms(&primitives);

        assert!(rooms.is_empty());
    }

    #[test]
    fn test_find_all_loops() {
        let generator = GeoCotGenerator::new();
        let poly = create_test_polygon();
        let primitives = vec![Primitive::Polygon(poly)];

        let loops = generator.find_all_loops(&primitives);

        assert_eq!(loops.len(), 1);
    }

    #[test]
    fn test_find_all_loops_no_polygons() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![Primitive::Line(Line::from_coords(
            [0.0, 0.0],
            [100.0, 100.0],
        ))];

        let loops = generator.find_all_loops(&primitives);

        assert!(loops.is_empty());
    }

    #[test]
    fn test_find_all_loops_small_polygon() {
        let generator = GeoCotGenerator::new();
        let small_poly = Polygon::from_coords(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        let primitives = vec![Primitive::Polygon(small_poly)];

        let loops = generator.find_all_loops(&primitives);

        assert_eq!(loops.len(), 1);
    }

    #[test]
    fn test_generate_perception_with_door_marker() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![
            Primitive::Polygon(create_test_polygon()),
            Primitive::Text {
                content: "D".to_string(),
                position: Point { x: 50.0, y: 0.0 },
                height: 10.0,
            },
        ];

        let perception = generator.generate_perception(&primitives);

        assert!(perception.contains("门"));
    }

    #[test]
    fn test_generate_reasoning_empty_task() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![Primitive::Polygon(create_test_polygon())];

        let reasoning = generator.generate_reasoning(&primitives, "未知任务");

        assert!(reasoning.is_empty() || reasoning.contains("房间"));
    }

    #[test]
    fn test_generate_summary_default() {
        let generator = GeoCotGenerator::new();
        let primitives = vec![Primitive::Polygon(create_test_polygon())];

        let summary = generator.generate_summary(&primitives, "分析图形");

        assert!(summary.contains("房间"));
    }

    #[test]
    fn test_format_thinking() {
        let generator = GeoCotGenerator::new();

        let thinking = generator.format_thinking("感知内容", "推理内容");

        assert!(thinking.contains("<thinking>"));
        assert!(thinking.contains("感知内容"));
        assert!(thinking.contains("推理内容"));
        assert!(thinking.contains("</thinking>"));
    }

    #[test]
    fn test_geo_cot_data_clone() {
        let data = GeoCotData {
            task: "测试".to_string(),
            perception: "感知".to_string(),
            reasoning: "推理".to_string(),
            summary: "总结".to_string(),
            thinking: "思维".to_string(),
            answer: "答案".to_string(),
        };

        let cloned = data.clone();
        assert_eq!(cloned.task, data.task);
        assert_eq!(cloned.answer, data.answer);
    }

    #[test]
    fn test_geo_cot_data_debug() {
        let data = GeoCotData {
            task: "测试".to_string(),
            perception: "感知".to_string(),
            reasoning: "推理".to_string(),
            summary: "总结".to_string(),
            thinking: "思维".to_string(),
            answer: "答案".to_string(),
        };

        let debug_str = format!("{:?}", data);
        assert!(debug_str.contains("GeoCotData"));
    }
}
