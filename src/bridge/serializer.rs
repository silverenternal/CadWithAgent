//! 序列化器
//!
//! 处理模型输入输出的序列化

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::geometry::Primitive;
use crate::tools::{ToolCallRequest, ToolCallResponse};

/// 模型调用请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    /// 模型名称
    pub model: String,
    /// 消息列表
    pub messages: Vec<Message>,
    /// 可用工具定义
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// 温度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// 最大 token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 角色
    pub role: String,
    /// 内容
    pub content: MessageContent,
}

/// 消息内容（支持多模态）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    MultiModal(Vec<ContentPart>),
}

/// 内容部分
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentPart {
    Text { text: String },
    Image { image_url: ImageUrl },
}

/// 图片 URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具类型
    #[serde(default = "default_tool_type")]
    pub r#type: String,
    /// 函数定义
    pub function: FunctionDefinition,
}

fn default_tool_type() -> String {
    "function".to_string()
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// 函数名称
    pub name: String,
    /// 函数描述
    pub description: String,
    /// 参数 schema
    pub parameters: serde_json::Value,
}

/// 模型响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    /// 选择列表
    pub choices: Vec<Choice>,
    /// 使用统计
    pub usage: Option<Usage>,
}

/// 选择
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// 消息
    pub message: ResponseMessage,
    /// 完成原因
    pub finish_reason: String,
}

/// 响应消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    /// 角色
    pub role: String,
    /// 内容
    pub content: Option<String>,
    /// 工具调用
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具名称
    pub name: String,
    /// 工具参数
    pub arguments: String,
}

/// 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

/// 解析模型响应中的工具调用
pub fn parse_tool_calls(tool_calls: &[ToolCall]) -> Vec<ToolCallRequest> {
    tool_calls
        .iter()
        .filter_map(|call| {
            serde_json::from_str::<Value>(&call.arguments)
                .ok()
                .map(|args| ToolCallRequest {
                    name: call.name.clone(),
                    arguments: args,
                })
        })
        .collect()
}

/// 创建工具调用响应消息
pub fn create_tool_response(
    tool_results: &[ToolCallResponse],
    tool_names: &[&str],
) -> Message {
    let content = tool_results
        .iter()
        .zip(tool_names.iter())
        .map(|(r, name)| {
            let result_str = if r.success {
                r.data.as_ref().map(|v| v.to_string()).unwrap_or_default()
            } else {
                r.error.as_ref().map(|e| e.to_string()).unwrap_or_default()
            };
            format!(
                "工具'{}'执行{}: {}",
                name,
                if r.success { "成功" } else { "失败" },
                result_str
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    Message {
        role: "tool".to_string(),
        content: MessageContent::Text(content),
    }
}

/// 创建系统提示
pub fn create_system_prompt(additional_instructions: Option<&str>) -> Message {
    let base_prompt = r#"你是一个 CAD 图纸分析助手，专门处理建筑平面图相关的几何问题。
你可以使用以下工具：
- 测量长度、面积、角度
- 检测房间、门、窗
- 图形变换（平移、旋转、缩放）
- 导出 DXF 文件

在回答问题时，请先思考：
1. 需要哪些几何信息
2. 应该调用哪些工具
3. 如何解释工具返回的结果

然后给出准确的答案。"#;

    let prompt = match additional_instructions {
        Some(instr) => format!("{}\n\n额外指令：{}", base_prompt, instr),
        None => base_prompt.to_string(),
    };

    Message {
        role: "system".to_string(),
        content: MessageContent::Text(prompt),
    }
}

/// 创建用户消息（带图像）
pub fn create_user_message(image_url: &str, instruction: &str) -> Message {
    Message {
        role: "user".to_string(),
        content: MessageContent::MultiModal(vec![
            ContentPart::Image {
                image_url: ImageUrl {
                    url: image_url.to_string(),
                },
            },
            ContentPart::Text {
                text: instruction.to_string(),
            },
        ]),
    }
}

/// 创建纯文本用户消息
pub fn create_text_user_message(text: &str) -> Message {
    Message {
        role: "user".to_string(),
        content: MessageContent::Text(text.to_string()),
    }
}

/// 创建助手消息
pub fn create_assistant_message(
    content: Option<String>,
    _tool_calls: Vec<ToolCall>,
) -> Message {
    Message {
        role: "assistant".to_string(),
        content: MessageContent::Text(content.unwrap_or_default()),
    }
}

/// 将图元列表转换为 JSON schema 描述
pub fn primitives_to_schema(primitives: &[Primitive]) -> Value {
    let mut items = Vec::new();
    
    for prim in primitives {
        items.push(primitive_to_schema(prim));
    }
    
    Value::Array(items)
}

fn primitive_to_schema(prim: &Primitive) -> Value {
    match prim {
        Primitive::Line(line) => {
            serde_json::json!({
                "type": "line",
                "start": [line.start.x, line.start.y],
                "end": [line.end.x, line.end.y],
                "length": line.length(),
            })
        }
        Primitive::Polygon(poly) => {
            serde_json::json!({
                "type": "polygon",
                "vertices": poly.vertices.iter().map(|p| [p.x, p.y]).collect::<Vec<_>>(),
                "area": poly.area(),
                "perimeter": poly.perimeter(),
            })
        }
        Primitive::Circle(circle) => {
            serde_json::json!({
                "type": "circle",
                "center": [circle.center.x, circle.center.y],
                "radius": circle.radius,
                "area": circle.area(),
            })
        }
        Primitive::Rect(rect) => {
            serde_json::json!({
                "type": "rect",
                "min": [rect.min.x, rect.min.y],
                "max": [rect.max.x, rect.max.y],
                "width": rect.width(),
                "height": rect.height(),
                "area": rect.area(),
            })
        }
        _ => serde_json::json!({"type": "unknown"})
    }
}
