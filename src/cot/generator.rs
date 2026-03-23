//! Geo-CoT 生成器
//!
//! 基于几何图元自动生成思维链数据

use crate::geometry::{Primitive, Point, Polygon, Room};
use crate::cot::templates::{PerceptionTemplate, ReasoningTemplate, SummaryTemplate};
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
        let doors: Vec<_> = primitives.iter()
            .filter_map(|p| match p {
                Primitive::Text { content, position, .. } if content.contains("门") || content.to_lowercase() == "d" => {
                    Some((*position, content.clone()))
                }
                _ => None,
            })
            .collect();
        
        for (pos, content) in &doors {
            observations.push(format!("在坐标 [{:.0}, {:.0}] 处检测到文本标记'{}'，可能是门的位置", 
                pos.x, pos.y, content));
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
                    room.name, room.boundary.vertices.len(), room.area
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
            return format!("总面积为{:.2}平方单位。各房间面积：{}", 
                total_area,
                rooms.iter().map(|r| format!("{}: {:.2}", r.name, r.area)).collect::<Vec<_>>().join(", "));
        }

        if task.contains("宽度") {
            if let Some(boundary) = self.find_outer_boundary(primitives) {
                let bbox = boundary.vertices.iter()
                    .fold((f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
                        |(min_x, min_y, max_x, max_y), p| {
                            (min_x.min(p.x), min_y.min(p.y), max_x.max(p.x), max_y.max(p.y))
                        });
                let width = bbox.2 - bbox.0;
                return format!("建筑总宽度为{:.2}单位（从 x={:.2}到 x={:.2}）。", width, bbox.0, bbox.2);
            }
        }

        // 默认总结
        let rooms = self.find_rooms(primitives);
        format!(
            "检测到{}个房间，外边界由{}个点组成。",
            rooms.len(),
            self.find_outer_boundary(primitives).map(|p| p.vertices.len()).unwrap_or(0)
        )
    }

    /// 格式化思维链
    fn format_thinking(&self, perception: &str, reasoning: &str) -> String {
        format!("<thinking>\n{}\n\n{}\n</thinking>", perception, reasoning)
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
        loops.into_iter().max_by(|a, b| a.area().partial_cmp(&b.area()).unwrap())
    }

    fn find_rooms(&self, primitives: &[Primitive]) -> Vec<Room> {
        // 简化实现：查找所有闭合回路（排除最大的外边界）
        let mut loops = self.find_all_loops(primitives);

        if loops.is_empty() {
            return vec![];
        }

        // 移除最大的（外边界）
        if loops.len() > 1 {
            loops.sort_by(|a, b| b.area().partial_cmp(&a.area()).unwrap());
            loops.remove(0);
        }

        loops.into_iter().enumerate().map(|(i, boundary)| {
            Room {
                name: format!("房间{}", i + 1),
                boundary: boundary.clone(),
                area: boundary.area(),
                doors: vec![],
                windows: vec![],
            }
        }).collect()
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
