//! 序列化器
//!
//! 处理模型输入输出的序列化，支持 `OpenAI` 兼容格式

use crate::geometry::Primitive;
use crate::tools::{ToolCallRequest, ToolCallResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 模型调用请求
///
/// 表示发送到 LLM/VLM 模型的聊天完成请求。
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

/// 聊天消息
///
/// 表示对话中的单条消息，包含角色和内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 角色
    pub role: String,
    /// 内容
    pub content: MessageContent,
}

/// 消息内容（支持多模态）
///
/// 支持纯文本或多模态内容（文本 + 图像）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// 纯文本内容
    Text(String),
    /// 多模态内容（文本 + 图像）
    MultiModal(Vec<ContentPart>),
}

/// 内容部分
///
/// 多模态消息中的单个内容单元。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentPart {
    /// 文本内容
    Text { text: String },
    /// 图像内容
    Image { image_url: ImageUrl },
}

/// 图片 URL
///
/// 表示多模态消息中的图像引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// 工具定义
///
/// 向模型注册的可用工具（函数）定义。
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
///
/// 描述工具的名称、参数和用途。
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
///
/// LLM/VLM 模型返回的聊天完成响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    /// 选择列表
    pub choices: Vec<Choice>,
    /// 使用统计
    pub usage: Option<Usage>,
}

/// 响应选择
///
/// 模型返回的单个候选响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// 消息
    pub message: ResponseMessage,
    /// 完成原因
    pub finish_reason: String,
}

/// 响应消息
///
/// 模型生成的回复消息，可能包含工具调用。
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
///
/// 模型请求调用外部工具的指令。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具名称
    pub name: String,
    /// 工具参数（JSON 字符串格式）
    pub arguments: String,
}

/// Token 使用统计
///
/// 记录模型调用的 Token 消耗情况。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// 提示词 token 数
    pub prompt_tokens: i32,
    /// 完成 token 数
    pub completion_tokens: i32,
    /// 总 token 数
    pub total_tokens: i32,
}

/// 解析模型响应中的工具调用
#[must_use]
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
#[must_use]
pub fn create_tool_response(tool_results: &[ToolCallResponse], tool_names: &[&str]) -> Message {
    let content = tool_results
        .iter()
        .zip(tool_names.iter())
        .map(|(r, name)| {
            let result_str = if r.success {
                r.data
                    .as_ref()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default()
            } else {
                r.error
                    .as_ref()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default()
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
#[must_use]
pub fn create_system_prompt(additional_instructions: Option<&str>) -> Message {
    let base_prompt = r"你是一个 CAD 图纸分析助手，专门处理建筑平面图相关的几何问题。
你可以使用以下工具：
- 测量长度、面积、角度
- 检测房间、门、窗
- 图形变换（平移、旋转、缩放）
- 导出 DXF 文件

在回答问题时，请先思考：
1. 需要哪些几何信息
2. 应该调用哪些工具
3. 如何解释工具返回的结果

然后给出准确的答案。";

    let prompt = match additional_instructions {
        Some(instr) => format!("{base_prompt}\n\n额外指令：{instr}"),
        None => base_prompt.to_string(),
    };

    Message {
        role: "system".to_string(),
        content: MessageContent::Text(prompt),
    }
}

/// 创建用户消息（带图像）
#[must_use]
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
#[must_use]
pub fn create_text_user_message(text: &str) -> Message {
    Message {
        role: "user".to_string(),
        content: MessageContent::Text(text.to_string()),
    }
}

/// 创建助手消息
#[must_use]
pub fn create_assistant_message(content: Option<String>, _tool_calls: Vec<ToolCall>) -> Message {
    Message {
        role: "assistant".to_string(),
        content: MessageContent::Text(content.unwrap_or_default()),
    }
}

/// 将图元列表转换为 JSON schema 描述
#[must_use]
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
        _ => serde_json::json!({"type": "unknown"}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Circle, Line, Point, Polygon, Rect};

    // ===== MessageContent 测试 =====

    #[test]
    fn test_message_content_text() {
        let MessageContent::Text(text) = MessageContent::Text("Hello".to_string()) else {
            panic!("Expected Text variant");
        };
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_message_content_multimodal() {
        // Test with placeholder base64 image (valid format for testing)
        let parts = vec![
            ContentPart::Text {
                text: "Describe this".to_string(),
            },
            ContentPart::Image {
                image_url: ImageUrl {
                    url: "data:image/png;base64,placeholder_for_testing".to_string(),
                },
            },
        ];
        let content = MessageContent::MultiModal(parts);
        let MessageContent::MultiModal(items) = content else {
            panic!("Expected MultiModal variant");
        };
        assert_eq!(items.len(), 2);
        let ContentPart::Text { text } = &items[0] else {
            panic!("Expected Text part");
        };
        assert_eq!(text, "Describe this");
    }

    // ===== ContentPart 测试 =====

    #[test]
    fn test_content_part_text() {
        let ContentPart::Text { text } = (ContentPart::Text {
            text: "Test".to_string(),
        }) else {
            panic!("Expected Text variant");
        };
        assert_eq!(text, "Test");
    }

    #[test]
    fn test_content_part_image() {
        let ContentPart::Image { image_url } = (ContentPart::Image {
            image_url: ImageUrl {
                url: "http://example.com/img.png".to_string(),
            },
        }) else {
            panic!("Expected Image variant");
        };
        assert_eq!(image_url.url, "http://example.com/img.png");
    }

    // ===== Message 测试 =====

    #[test]
    fn test_message_system() {
        let msg = Message {
            role: "system".to_string(),
            content: MessageContent::Text("You are helpful".to_string()),
        };
        assert_eq!(msg.role, "system");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert_eq!(text, "You are helpful");
    }

    #[test]
    fn test_message_user() {
        let msg = Message {
            role: "user".to_string(),
            content: MessageContent::Text("What is CAD?".to_string()),
        };
        assert_eq!(msg.role, "user");
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message {
            role: "assistant".to_string(),
            content: MessageContent::Text("CAD stands for...".to_string()),
        };
        assert_eq!(msg.role, "assistant");
    }

    // ===== ToolDefinition 测试 =====

    #[test]
    fn test_tool_definition_default_type() {
        let tool = ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "measure_length".to_string(),
                description: "Measure length".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        };
        assert_eq!(tool.r#type, "function");
    }

    #[test]
    fn test_function_definition() {
        let func = FunctionDefinition {
            name: "detect_rooms".to_string(),
            description: "Detect rooms".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        };
        assert_eq!(func.name, "detect_rooms");
        assert_eq!(func.description, "Detect rooms");
    }

    // ===== ModelResponse 测试 =====

    #[test]
    fn test_choice() {
        let choice = Choice {
            message: ResponseMessage {
                role: "assistant".to_string(),
                content: Some("Hello".to_string()),
                tool_calls: vec![],
            },
            finish_reason: "stop".to_string(),
        };
        assert_eq!(choice.finish_reason, "stop");
        assert_eq!(choice.message.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_response_message_with_content() {
        let msg = ResponseMessage {
            role: "assistant".to_string(),
            content: Some("Test".to_string()),
            tool_calls: vec![],
        };
        assert_eq!(msg.content, Some("Test".to_string()));
    }

    #[test]
    fn test_response_message_without_content() {
        let msg = ResponseMessage {
            role: "assistant".to_string(),
            content: None,
            tool_calls: vec![],
        };
        assert_eq!(msg.content, None);
    }

    // ===== ToolCall 测试 =====

    #[test]
    fn test_tool_call() {
        let call = ToolCall {
            name: "measure_length".to_string(),
            arguments: "{\"line_id\": 1}".to_string(),
        };
        assert_eq!(call.name, "measure_length");
        assert_eq!(call.arguments, "{\"line_id\": 1}");
    }

    #[test]
    fn test_parse_tool_calls_success() {
        let calls = vec![
            ToolCall {
                name: "measure_length".to_string(),
                arguments: "{\"line_id\": 1}".to_string(),
            },
            ToolCall {
                name: "detect_rooms".to_string(),
                arguments: "{\"threshold\": 0.5}".to_string(),
            },
        ];

        let requests = parse_tool_calls(&calls);
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].name, "measure_length");
        assert_eq!(requests[1].name, "detect_rooms");
    }

    #[test]
    fn test_parse_tool_calls_empty() {
        let calls: Vec<ToolCall> = vec![];
        let requests = parse_tool_calls(&calls);
        assert!(requests.is_empty());
    }

    #[test]
    fn test_parse_tool_calls_invalid_json() {
        let calls = vec![ToolCall {
            name: "test".to_string(),
            arguments: "invalid json".to_string(),
        }];
        let requests = parse_tool_calls(&calls);
        assert!(requests.is_empty()); // 应该过滤掉无效的 JSON
    }

    // ===== Usage 测试 =====

    #[test]
    fn test_usage() {
        let usage = Usage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    // ===== create_tool_response 测试 =====

    #[test]
    fn test_create_tool_response_success() {
        let results = vec![ToolCallResponse {
            success: true,
            data: Some(serde_json::json!({"length": 100})),
            error: None,
        }];
        let names = vec!["measure_length"];

        let msg = create_tool_response(&results, &names);
        assert_eq!(msg.role, "tool");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert!(text.contains("成功"));
        assert!(text.contains("measure_length"));
    }

    #[test]
    fn test_create_tool_response_error() {
        let results = vec![ToolCallResponse {
            success: false,
            data: None,
            error: Some("Invalid input".to_string()),
        }];
        let names = vec!["measure_length"];

        let msg = create_tool_response(&results, &names);
        assert_eq!(msg.role, "tool");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert!(text.contains("失败"));
        assert!(text.contains("Invalid input"));
    }

    #[test]
    fn test_create_tool_response_multiple() {
        let results = vec![
            ToolCallResponse {
                success: true,
                data: Some(serde_json::json!({"length": 100})),
                error: None,
            },
            ToolCallResponse {
                success: true,
                data: Some(serde_json::json!({"area": 500})),
                error: None,
            },
        ];
        let names = vec!["measure_length", "measure_area"];

        let msg = create_tool_response(&results, &names);
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert!(text.contains("measure_length"));
        assert!(text.contains("measure_area"));
    }

    // ===== create_system_prompt 测试 =====

    #[test]
    fn test_create_system_prompt_basic() {
        let msg = create_system_prompt(None);
        assert_eq!(msg.role, "system");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert!(text.contains("CAD"));
        assert!(text.contains("工具"));
    }

    #[test]
    fn test_create_system_prompt_with_instructions() {
        let msg = create_system_prompt(Some("只回答几何问题"));
        assert_eq!(msg.role, "system");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert!(text.contains("额外指令"));
        assert!(text.contains("只回答几何问题"));
    }

    // ===== create_user_message 测试 =====

    #[test]
    fn test_create_user_message_multimodal() {
        // Test with placeholder base64 image URL
        let msg = create_user_message(
            "data:image/png;base64,placeholder_for_testing",
            "Describe the floor plan",
        );
        assert_eq!(msg.role, "user");
        let MessageContent::MultiModal(ref parts) = msg.content else {
            panic!("Expected MultiModal content");
        };
        assert_eq!(parts.len(), 2);
        let ContentPart::Image { image_url } = &parts[0] else {
            panic!("Expected Image part");
        };
        assert_eq!(
            image_url.url,
            "data:image/png;base64,placeholder_for_testing"
        );
        let ContentPart::Text { text } = &parts[1] else {
            panic!("Expected Text part");
        };
        assert_eq!(text, "Describe the floor plan");
    }

    // ===== create_text_user_message 测试 =====

    #[test]
    fn test_create_text_user_message() {
        let msg = create_text_user_message("What is the area?");
        assert_eq!(msg.role, "user");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert_eq!(text, "What is the area?");
    }

    // ===== create_assistant_message 测试 =====

    #[test]
    fn test_create_assistant_message_with_content() {
        let msg = create_assistant_message(Some("The area is 100".to_string()), vec![]);
        assert_eq!(msg.role, "assistant");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert_eq!(text, "The area is 100");
    }

    #[test]
    fn test_create_assistant_message_without_content() {
        let msg = create_assistant_message(None, vec![]);
        assert_eq!(msg.role, "assistant");
        let MessageContent::Text(ref text) = msg.content else {
            panic!("Expected Text content");
        };
        assert_eq!(text, "");
    }

    // ===== primitives_to_schema 测试 =====

    #[test]
    fn test_primitives_to_schema_line() {
        let primitives = vec![Primitive::Line(Line::from_coords([0.0, 0.0], [3.0, 4.0]))];
        let result = primitives_to_schema(&primitives);

        let Value::Array(items) = result else {
            panic!("Expected Array");
        };
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item["type"], "line");
        assert_eq!(item["length"], 5.0);
    }

    #[test]
    fn test_primitives_to_schema_polygon() {
        let poly =
            Polygon::from_coords(vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]);
        let primitives = vec![Primitive::Polygon(poly)];
        let result = primitives_to_schema(&primitives);

        let Value::Array(items) = result else {
            panic!("Expected Array");
        };
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item["type"], "polygon");
        assert_eq!(item["area"], 10000.0);
    }

    #[test]
    fn test_primitives_to_schema_circle() {
        let circle = Circle::from_coords([0.0, 0.0], 5.0);
        let primitives = vec![Primitive::Circle(circle)];
        let result = primitives_to_schema(&primitives);

        let Value::Array(items) = result else {
            panic!("Expected Array");
        };
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item["type"], "circle");
        assert_eq!(item["radius"], 5.0);
        assert!(item["area"].as_f64().unwrap() > 78.0);
    }

    #[test]
    fn test_primitives_to_schema_rect() {
        let rect = Rect::from_coords([0.0, 0.0], [10.0, 20.0]);
        let primitives = vec![Primitive::Rect(rect)];
        let result = primitives_to_schema(&primitives);

        let Value::Array(items) = result else {
            panic!("Expected Array");
        };
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item["type"], "rect");
        assert_eq!(item["width"], 10.0);
        assert_eq!(item["height"], 20.0);
        assert_eq!(item["area"], 200.0);
    }

    #[test]
    fn test_primitives_to_schema_multiple() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 1.0])),
            Primitive::Circle(Circle::from_coords([0.0, 0.0], 1.0)),
        ];
        let result = primitives_to_schema(&primitives);

        let Value::Array(items) = result else {
            panic!("Expected Array");
        };
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["type"], "line");
        assert_eq!(items[1]["type"], "circle");
    }

    #[test]
    fn test_primitives_to_schema_empty() {
        let primitives: Vec<Primitive> = vec![];
        let result = primitives_to_schema(&primitives);

        let Value::Array(items) = result else {
            panic!("Expected Array");
        };
        assert!(items.is_empty());
    }

    #[test]
    fn test_primitives_to_schema_unknown() {
        // 创建 Text 图元（会被识别为 unknown）
        let primitives = vec![Primitive::Text {
            content: "Test".to_string(),
            position: Point::origin(),
            height: 10.0,
        }];
        let result = primitives_to_schema(&primitives);

        match result {
            Value::Array(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0]["type"], "unknown");
            }
            _ => unreachable!("Expected Array"),
        }
    }

    // ===== ModelRequest 测试 =====

    #[test]
    fn test_model_request_minimal() {
        let req = ModelRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            tools: None,
            temperature: None,
            max_tokens: None,
        };
        assert_eq!(req.model, "gpt-4");
        assert!(req.tools.is_none());
    }

    #[test]
    fn test_model_request_full() {
        let req = ModelRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
            }],
            tools: Some(vec![]),
            temperature: Some(0.7),
            max_tokens: Some(1000),
        };
        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.temperature, Some(0.7));
        assert_eq!(req.max_tokens, Some(1000));
    }

    // ===== ModelResponse 测试 =====

    #[test]
    fn test_model_response_with_usage() {
        let resp = ModelResponse {
            choices: vec![],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };
        assert!(resp.usage.is_some());
        assert_eq!(resp.usage.as_ref().unwrap().total_tokens, 15);
    }

    #[test]
    fn test_model_response_without_usage() {
        let resp = ModelResponse {
            choices: vec![],
            usage: None,
        };
        assert!(resp.usage.is_none());
    }
}
