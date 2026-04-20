//! Geo-CoT 模板定义
//!
//! 定义生成思维链的文本模板

use serde::{Deserialize, Serialize};

/// 感知模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionTemplate {
    pub pattern: String,
}

impl Default for PerceptionTemplate {
    fn default() -> Self {
        Self {
            pattern: "首先，我观察到图像{position}有一个{element_type}，由{vertex_count}个顶点组成，坐标依次为：{coords}。".to_string(),
        }
    }
}

impl PerceptionTemplate {
    pub fn format(&self, element_type: &str, coords: &str, vertex_count: usize) -> String {
        self.pattern
            .replace("{element_type}", element_type)
            .replace("{coords}", coords)
            .replace("{vertex_count}", &vertex_count.to_string())
            .replace("{position}", "外部")
    }
}

/// 推理模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningTemplate {
    pub pattern: String,
}

impl Default for ReasoningTemplate {
    fn default() -> Self {
        Self {
            pattern: "根据{rule}，我分析{observation}，得出结论：{conclusion}。".to_string(),
        }
    }
}

impl ReasoningTemplate {
    pub fn format(&self, rule: &str, observation: &str, conclusion: &str) -> String {
        self.pattern
            .replace("{rule}", rule)
            .replace("{observation}", observation)
            .replace("{conclusion}", conclusion)
    }
}

/// 总结模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryTemplate {
    pub pattern: String,
}

impl Default for SummaryTemplate {
    fn default() -> Self {
        Self {
            pattern: "综上所述，{summary}。".to_string(),
        }
    }
}

impl SummaryTemplate {
    pub fn format(&self, summary: &str) -> String {
        self.pattern.replace("{summary}", summary)
    }
}

/// QA 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaTemplate {
    pub question_patterns: Vec<String>,
    pub answer_patterns: Vec<String>,
}

impl Default for QaTemplate {
    fn default() -> Self {
        Self {
            question_patterns: vec![
                "{element}的{attribute}是多少？".to_string(),
                "请计算{element}的{attribute}。".to_string(),
                "{element}有多大？".to_string(),
            ],
            answer_patterns: vec![
                "<thinking>{thinking}</thinking>{element}的{attribute}是{value}。".to_string(),
                "根据计算，{element}的{attribute}为{value}。".to_string(),
            ],
        }
    }
}

impl QaTemplate {
    pub fn format_question(&self, element: &str, attribute: &str) -> String {
        self.question_patterns.first().map_or_else(
            || format!("What is the {attribute} of the {element}?"),
            |p| {
                p.replace("{element}", element)
                    .replace("{attribute}", attribute)
            },
        )
    }

    pub fn format_answer(
        &self,
        thinking: &str,
        element: &str,
        attribute: &str,
        value: &str,
    ) -> String {
        self.answer_patterns.first().map_or_else(
            || format!("Based on calculation, the {attribute} of the {element} is {value}."),
            |p| {
                p.replace("{thinking}", thinking)
                    .replace("{element}", element)
                    .replace("{attribute}", attribute)
                    .replace("{value}", value)
            },
        )
    }
}

/// 房间类型推断模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTypeTemplate {
    pub rules: Vec<RoomTypeRule>,
}

impl Default for RoomTypeTemplate {
    fn default() -> Self {
        Self {
            rules: vec![
                RoomTypeRule {
                    condition: "面积 < 50000 且 门数量 <= 1".to_string(),
                    room_type: "卫生间".to_string(),
                },
                RoomTypeRule {
                    condition: "50000 <= 面积 < 150000 且 门数量 = 1".to_string(),
                    room_type: "卧室".to_string(),
                },
                RoomTypeRule {
                    condition: "150000 <= 面积 < 300000".to_string(),
                    room_type: "客厅".to_string(),
                },
                RoomTypeRule {
                    condition: "面积 >= 300000".to_string(),
                    room_type: "大厅".to_string(),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTypeRule {
    pub condition: String,
    pub room_type: String,
}
