//! QA 生成器
//!
//! 基于几何图元自动生成问答对

use crate::cot::templates::QaTemplate;
use crate::geometry::{Point, Polygon, Primitive, Room};
use serde::{Deserialize, Serialize};

/// QA 对
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAPair {
    pub question: String,
    pub answer: String,
    pub thinking: Option<String>,
    pub question_type: String,
}

/// QA 生成器
pub struct QaGenerator {
    template: QaTemplate,
}

impl QaGenerator {
    pub fn new() -> Self {
        Self {
            template: QaTemplate::default(),
        }
    }

    /// 生成所有类型的问答对
    pub fn generate_all(&self, primitives: &[Primitive]) -> Vec<QAPair> {
        let mut qa_pairs = Vec::new();

        // 面积类问题
        qa_pairs.extend(self.generate_area_questions(primitives));

        // 尺寸类问题
        qa_pairs.extend(self.generate_dimension_questions(primitives));

        // 数量类问题
        qa_pairs.extend(self.generate_count_questions(primitives));

        // 位置类问题
        qa_pairs.extend(self.generate_position_questions(primitives));

        // 关系类问题
        qa_pairs.extend(self.generate_relation_questions(primitives));

        qa_pairs
    }

    /// 生成面积类问题
    fn generate_area_questions(&self, primitives: &[Primitive]) -> Vec<QAPair> {
        let mut qa_pairs = Vec::new();
        let rooms = self.extract_rooms(primitives);

        for room in &rooms {
            // 单个房间面积
            qa_pairs.push(QAPair {
                question: self.template.format_question(&room.name, "面积"),
                answer: self.template.format_answer(
                    &self.generate_area_thinking(room),
                    &room.name,
                    "面积",
                    &format!("{:.2}平方单位", room.area),
                ),
                thinking: Some(self.generate_area_thinking(room)),
                question_type: "area".to_string(),
            });
        }

        // 总面积
        if !rooms.is_empty() {
            let total_area: f64 = rooms.iter().map(|r| r.area).sum();
            qa_pairs.push(QAPair {
                question: "所有房间的总面积是多少？".to_string(),
                answer: format!(
                    "<thinking>将所有房间面积相加：{}</thinking>总面积为{:.2}平方单位。",
                    rooms
                        .iter()
                        .map(|r| format!("{:.2}", r.area))
                        .collect::<Vec<_>>()
                        .join(" + "),
                    total_area
                ),
                thinking: Some(format!(
                    "将所有房间面积相加：{}",
                    rooms
                        .iter()
                        .map(|r| format!("{:.2}", r.area))
                        .collect::<Vec<_>>()
                        .join(" + ")
                )),
                question_type: "total_area".to_string(),
            });
        }

        qa_pairs
    }

    /// 生成尺寸类问题
    fn generate_dimension_questions(&self, primitives: &[Primitive]) -> Vec<QAPair> {
        let mut qa_pairs = Vec::new();

        // 查找外边界
        if let Some(boundary) = self.find_outer_boundary(primitives) {
            let bbox = self.calculate_bbox(&boundary);

            // 宽度问题
            qa_pairs.push(QAPair {
                question: "建筑的总宽度是多少？".to_string(),
                answer: format!("<thinking>查找建筑边界->左墙 x 坐标为{:.0}，右墙 x 坐标为{:.0}->计算宽度{:.0}-{:.0}={:.0}</thinking>建筑的总宽度是{:.0}个像素单位。",
                    bbox.0, bbox.2, bbox.2, bbox.0, bbox.2 - bbox.0, bbox.2 - bbox.0),
                thinking: Some(format!("查找建筑边界->左墙 x 坐标为{:.0}，右墙 x 坐标为{:.0}->计算宽度{:.0}-{:.0}={:.0}",
                    bbox.0, bbox.2, bbox.2, bbox.0, bbox.2 - bbox.0)),
                question_type: "width".to_string(),
            });

            // 高度问题
            qa_pairs.push(QAPair {
                question: "建筑的总高度是多少？".to_string(),
                answer: format!("<thinking>查找建筑边界->下墙 y 坐标为{:.0}，上墙 y 坐标为{:.1}->计算高度{:.1}-{:.0}={:.1}</thinking>建筑的总高度是{:.1}个像素单位。",
                    bbox.1, bbox.3, bbox.3, bbox.1, bbox.3 - bbox.1, bbox.3 - bbox.1),
                thinking: Some(format!("查找建筑边界->下墙 y 坐标为{:.0}，上墙 y 坐标为{:.1}->计算高度{:.1}-{:.0}={:.1}",
                    bbox.1, bbox.3, bbox.3, bbox.1, bbox.3 - bbox.1)),
                question_type: "height".to_string(),
            });
        }

        qa_pairs
    }

    /// 生成数量类问题
    fn generate_count_questions(&self, primitives: &[Primitive]) -> Vec<QAPair> {
        let mut qa_pairs = Vec::new();
        let rooms = self.extract_rooms(primitives);

        qa_pairs.push(QAPair {
            question: "图中有多少个房间？".to_string(),
            answer: format!("<thinking>检测所有闭合回路->排除外边界->统计内部回路数量</thinking>图中共有{}个房间。", rooms.len()),
            thinking: Some("检测所有闭合回路->排除外边界->统计内部回路数量".to_string()),
            question_type: "count".to_string(),
        });

        // 门的数量
        let door_count = self.count_doors(primitives);
        qa_pairs.push(QAPair {
            question: "图中有多少个门？".to_string(),
            answer: format!(
                "<thinking>查找文本标记'门'或'D'->检测墙体缺口</thinking>图中共有{}个门。",
                door_count
            ),
            thinking: Some("查找文本标记'门'或'D'->检测墙体缺口".to_string()),
            question_type: "count".to_string(),
        });

        qa_pairs
    }

    /// 生成位置类问题
    fn generate_position_questions(&self, primitives: &[Primitive]) -> Vec<QAPair> {
        let mut qa_pairs = Vec::new();
        let rooms = self.extract_rooms(primitives);

        for room in rooms.iter() {
            if let Some(centroid) = self.calculate_centroid(&room.boundary) {
                qa_pairs.push(QAPair {
                    question: format!("{}的中心位置在哪里？", room.name),
                    answer: format!("<thinking>计算{}边界顶点的平均值</thinking>{}的中心位置在坐标[{:.0}, {:.0}]。",
                        room.name, room.name, centroid.x, centroid.y),
                    thinking: Some(format!("计算{}边界顶点的平均值", room.name)),
                    question_type: "position".to_string(),
                });
            }
        }

        qa_pairs
    }

    /// 生成关系类问题
    fn generate_relation_questions(&self, primitives: &[Primitive]) -> Vec<QAPair> {
        let mut qa_pairs = Vec::new();
        let rooms = self.extract_rooms(primitives);

        if rooms.len() >= 2 {
            // 面积比较
            let mut sorted_rooms = rooms.clone();
            sorted_rooms.sort_by(|a, b| b.area.partial_cmp(&a.area).unwrap());

            qa_pairs.push(QAPair {
                question: "哪个房间的面积最大？".to_string(),
                answer: format!(
                    "<thinking>比较所有房间的面积->{}</thinking>{}的面积最大，为{:.2}平方单位。",
                    sorted_rooms
                        .iter()
                        .map(|r| format!("{}: {:.2}", r.name, r.area))
                        .collect::<Vec<_>>()
                        .join(", "),
                    sorted_rooms[0].name,
                    sorted_rooms[0].area
                ),
                thinking: Some(format!(
                    "比较所有房间的面积->{}",
                    sorted_rooms
                        .iter()
                        .map(|r| format!("{}: {:.2}", r.name, r.area))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                question_type: "relation".to_string(),
            });
        }

        qa_pairs
    }

    // 辅助方法

    fn generate_area_thinking(&self, room: &Room) -> String {
        format!(
            "查找{}边界->获取顶点坐标->使用鞋带公式计算面积->Sum(x_i * y_{{i+1}} - x_{{i+1}} * y_i) / 2 = {:.2}",
            room.name, room.area
        )
    }

    fn extract_rooms(&self, primitives: &[Primitive]) -> Vec<Room> {
        // 简化实现：从 Polygon 图元提取
        let mut rooms = Vec::new();

        for (i, prim) in primitives.iter().enumerate() {
            if let Primitive::Polygon(poly) = prim {
                rooms.push(Room {
                    name: format!("房间{}", i + 1),
                    boundary: poly.clone(),
                    area: poly.area(),
                    doors: vec![],
                    windows: vec![],
                });
            }
        }

        rooms
    }

    fn find_outer_boundary(&self, primitives: &[Primitive]) -> Option<Polygon> {
        let mut max_area = 0.0;
        let mut boundary = None;

        for prim in primitives {
            if let Primitive::Polygon(poly) = prim {
                let area = poly.area();
                if area > max_area {
                    max_area = area;
                    boundary = Some(poly.clone());
                }
            }
        }

        boundary
    }

    fn calculate_bbox(&self, polygon: &Polygon) -> (f64, f64, f64, f64) {
        polygon.vertices.iter().fold(
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
        )
    }

    fn calculate_centroid(&self, polygon: &Polygon) -> Option<Point> {
        if polygon.vertices.is_empty() {
            return None;
        }

        let sum_x: f64 = polygon.vertices.iter().map(|p| p.x).sum();
        let sum_y: f64 = polygon.vertices.iter().map(|p| p.y).sum();
        let n = polygon.vertices.len() as f64;

        Some(Point::new(sum_x / n, sum_y / n))
    }

    fn count_doors(&self, primitives: &[Primitive]) -> usize {
        primitives
            .iter()
            .filter(|p| {
                if let Primitive::Text { content, .. } = p {
                    content.contains("门") || content.to_lowercase() == "d"
                } else {
                    false
                }
            })
            .count()
    }
}

impl Default for QaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 使用 tokitai 工具封装
#[derive(Default, Clone)]
pub struct QaTools;

use tokitai::tool;

#[tool]
impl QaTools {
    /// 生成 QA 对
    #[tool]
    pub fn generate_qa(&self, primitives: Vec<Primitive>, question_type: String) -> Vec<QAPair> {
        let generator = QaGenerator::new();
        let all_qa = generator.generate_all(&primitives);

        if question_type.is_empty() {
            all_qa
        } else {
            all_qa
                .into_iter()
                .filter(|qa| qa.question_type == question_type)
                .collect()
        }
    }

    /// 生成面积问题
    #[tool]
    pub fn generate_area_qa(&self, primitives: Vec<Primitive>) -> Vec<QAPair> {
        let generator = QaGenerator::new();
        generator.generate_area_questions(&primitives)
    }

    /// 生成数量问题
    #[tool]
    pub fn generate_count_qa(&self, primitives: Vec<Primitive>) -> Vec<QAPair> {
        let generator = QaGenerator::new();
        generator.generate_count_questions(&primitives)
    }
}
