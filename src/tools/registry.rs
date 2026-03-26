//! 工具注册表
//!
//! 统一管理所有可调用工具

use crate::geometry::primitives::Primitive;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// 工具调用结果
pub type ToolResult = Result<Value, ToolError>;

/// 工具错误
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("工具未找到：{0}")]
    NotFound(String),

    #[error("工具调用失败：{0}")]
    CallFailed(String),

    #[error("参数验证失败：{0}")]
    InvalidArgs(String),
}

/// 工具函数类型
pub type ToolFn = Arc<dyn Fn(Value) -> ToolResult + Send + Sync>;

/// 工具定义
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub function: ToolFn,
}

/// 工具注册表
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        // 注册所有工具
        registry.register_all();

        registry
    }

    /// 注册所有工具
    fn register_all(&mut self) {
        // 几何测量工具
        self.register_measurement_tools();

        // 几何变换工具
        self.register_transform_tools();

        // 拓扑分析工具
        self.register_topology_tools();

        // IO 工具
        self.register_io_tools();

        // Geo-CoT 工具
        self.register_cot_tools();
    }

    fn register_measurement_tools(&mut self) {
        use crate::geometry::GeometryMeasurer;

        self.register(ToolDefinition {
            name: "measure_length",
            description: "测量两点之间的线段长度",
            function: Arc::new(move |args| {
                let measurer = GeometryMeasurer;
                let start = args["start"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("start 必须是数组".into()))?;
                let end = args["end"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("end 必须是数组".into()))?;

                let start_arr = [
                    start[0]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("start[0] 必须是数字".into()))?,
                    start[1]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("start[1] 必须是数字".into()))?,
                ];
                let end_arr = [
                    end[0]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("end[0] 必须是数字".into()))?,
                    end[1]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("end[1] 必须是数字".into()))?,
                ];

                Ok(Value::Number(
                    serde_json::Number::from_f64(measurer.measure_length(start_arr, end_arr))
                        .unwrap(),
                ))
            }),
        });

        self.register(ToolDefinition {
            name: "measure_area",
            description: "计算多边形面积（使用鞋带公式）",
            function: Arc::new(move |args| {
                let measurer = GeometryMeasurer;
                let vertices = args["vertices"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("vertices 必须是数组".into()))?;

                let coords: Vec<[f64; 2]> = vertices
                    .iter()
                    .map(|v| {
                        let arr = v.as_array().unwrap();
                        [arr[0].as_f64().unwrap(), arr[1].as_f64().unwrap()]
                    })
                    .collect();

                Ok(Value::Number(
                    serde_json::Number::from_f64(measurer.measure_area(coords)).unwrap(),
                ))
            }),
        });
    }

    fn register_transform_tools(&mut self) {
        use crate::geometry::transform::{GeometryTransform, MirrorAxis};

        self.register(ToolDefinition {
            name: "translate",
            description: "平移图元（X 和 Y 方向移动）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;
                let dx = args["dx"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("dx 必须是数字".into()))?;
                let dy = args["dy"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("dy 必须是数字".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let result = transform.translate(primitives, dx, dy);
                Ok(serde_json::to_value(&result).unwrap())
            }),
        });

        self.register(ToolDefinition {
            name: "rotate",
            description: "旋转图元（绕指定中心点，角度为度数）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;
                let angle = args["angle"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("angle 必须是数字".into()))?;
                let center = args["center"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("center 必须是数组".into()))?;

                let center_arr = [
                    center[0]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("center[0] 必须是数字".into()))?,
                    center[1]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("center[1] 必须是数字".into()))?,
                ];

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let result = transform.rotate(primitives, angle, center_arr);
                Ok(serde_json::to_value(&result).unwrap())
            }),
        });

        self.register(ToolDefinition {
            name: "scale",
            description: "缩放图元（相对于指定中心点）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;
                let factor = args["factor"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("factor 必须是数字".into()))?;
                let center = args["center"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("center 必须是数组".into()))?;

                let center_arr = [
                    center[0]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("center[0] 必须是数字".into()))?,
                    center[1]
                        .as_f64()
                        .ok_or_else(|| ToolError::InvalidArgs("center[1] 必须是数字".into()))?,
                ];

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let result = transform.scale(primitives, factor, center_arr);
                Ok(serde_json::to_value(&result).unwrap())
            }),
        });

        self.register(ToolDefinition {
            name: "mirror",
            description: "镜像图元（关于 X 轴或 Y 轴）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;
                let axis = args["axis"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("axis 必须是字符串".into()))?;

                let mirror_axis = match axis.to_lowercase().as_str() {
                    "x" => MirrorAxis::X,
                    "y" => MirrorAxis::Y,
                    _ => return Err(ToolError::InvalidArgs("镜像轴必须是 'x' 或 'y'".into())),
                };

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let result = transform.mirror(primitives, mirror_axis);
                Ok(serde_json::to_value(&result).unwrap())
            }),
        });
    }

    fn register_topology_tools(&mut self) {
        use crate::topology::loop_detect::find_closed_loops;
        use crate::topology::room_detect::RoomDetector;

        // 房间检测工具
        self.register(ToolDefinition {
            name: "detect_rooms",
            description: "检测户型图中的所有房间",
            function: Arc::new(move |args| {
                let detector = RoomDetector;
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let rooms = detector.detect_rooms(primitives);
                Ok(serde_json::to_value(&rooms).unwrap())
            }),
        });

        self.register(ToolDefinition {
            name: "count_rooms",
            description: "统计房间数量",
            function: Arc::new(move |args| {
                let detector = RoomDetector;
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let count = detector.count_rooms(primitives);
                Ok(Value::Number(serde_json::Number::from(count)))
            }),
        });

        self.register(ToolDefinition {
            name: "detect_doors",
            description: "检测门的位置和数量",
            function: Arc::new(move |args| {
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                // 使用 detect_rooms 并提取门信息
                let detector = RoomDetector;
                let result = detector.detect_rooms(primitives);
                let doors: Vec<_> = result.rooms.iter().flat_map(|r| r.doors.clone()).collect();
                Ok(serde_json::to_value(&doors).unwrap())
            }),
        });

        self.register(ToolDefinition {
            name: "detect_windows",
            description: "检测窗户的位置和数量",
            function: Arc::new(move |args| {
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                // 使用 detect_rooms 并提取窗户信息
                let detector = RoomDetector;
                let result = detector.detect_rooms(primitives);
                let windows: Vec<_> = result
                    .rooms
                    .iter()
                    .flat_map(|r| r.windows.clone())
                    .collect();
                Ok(serde_json::to_value(&windows).unwrap())
            }),
        });

        self.register(ToolDefinition {
            name: "find_closed_loop",
            description: "查找闭合回路（用于房间边界检测）",
            function: Arc::new(move |args| {
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let loops = find_closed_loops(&primitives);
                Ok(serde_json::to_value(&loops).unwrap())
            }),
        });

        self.register(ToolDefinition {
            name: "max_room_area",
            description: "找出最大房间面积",
            function: Arc::new(move |args| {
                let detector = RoomDetector;
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let max_area = detector.max_room_area(primitives);
                Ok(Value::Number(
                    serde_json::Number::from_f64(max_area).unwrap(),
                ))
            }),
        });
    }

    fn register_io_tools(&mut self) {
        use crate::export::dxf::DxfExporter;
        use crate::export::json::JsonExporter;
        use crate::parser::dxf::DxfParser;
        use crate::parser::svg::SvgParser;

        // SVG 解析工具
        self.register(ToolDefinition {
            name: "parse_svg",
            description: "解析 SVG 文件，提取几何图元",
            function: Arc::new(move |args| {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("path 必须是字符串".into()))?;

                let result = SvgParser::parse(path)
                    .map_err(|e| ToolError::CallFailed(format!("SVG 解析失败：{}", e)))?;

                Ok(serde_json::to_value(&result.primitives).unwrap())
            }),
        });

        // DXF 解析工具
        self.register(ToolDefinition {
            name: "parse_dxf",
            description: "解析 DXF 文件，提取几何图元",
            function: Arc::new(move |args| {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("path 必须是字符串".into()))?;

                let result = DxfParser::parse(path)
                    .map_err(|e| ToolError::CallFailed(format!("DXF 解析失败：{}", e)))?;

                Ok(serde_json::to_value(&result.primitives).unwrap())
            }),
        });

        // DXF 导出工具
        self.register(ToolDefinition {
            name: "export_dxf",
            description: "将几何图元导出为 DXF 文件",
            function: Arc::new(move |args| {
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;
                let output_path = args["output_path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("output_path 必须是字符串".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let result = DxfExporter::export(&primitives, output_path)
                    .map_err(|e| ToolError::CallFailed(format!("DXF 导出失败：{}", e)))?;

                Ok(serde_json::to_value(&result).unwrap())
            }),
        });

        // JSON 导出工具
        self.register(ToolDefinition {
            name: "export_json",
            description: "将几何图元导出为 JSON 文件",
            function: Arc::new(move |args| {
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;
                let output_path = args["output_path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("output_path 必须是字符串".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let result = JsonExporter::export(&primitives, output_path)
                    .map_err(|e| ToolError::CallFailed(format!("JSON 导出失败：{}", e)))?;

                Ok(serde_json::to_value(&result).unwrap())
            }),
        });
    }

    fn register_cot_tools(&mut self) {
        use crate::cot::generator::GeoCotGenerator;
        use crate::cot::qa::QaGenerator;

        // Geo-CoT 生成工具
        self.register(ToolDefinition {
            name: "generate_geo_cot",
            description: "生成几何思维链数据",
            function: Arc::new(move |args| {
                let generator = GeoCotGenerator::new();
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;
                let task = args["task"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("task 必须是字符串".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let cot_data = generator.generate(&primitives, task);
                Ok(serde_json::to_value(&cot_data).unwrap())
            }),
        });

        // QA 生成工具
        self.register(ToolDefinition {
            name: "generate_qa",
            description: "生成问答对数据集",
            function: Arc::new(move |args| {
                let generator = QaGenerator::new();
                let primitives = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?;

                let primitives: Vec<Primitive> =
                    serde_json::from_value(Value::Array(primitives.clone()))
                        .map_err(|e| ToolError::InvalidArgs(format!("解析图元失败：{}", e)))?;

                let qa_pairs = generator.generate_all(&primitives);
                Ok(serde_json::to_value(&qa_pairs).unwrap())
            }),
        });

        // 思维链评估工具
        self.register(ToolDefinition {
            name: "evaluate_cot",
            description: "评估思维链质量",
            function: Arc::new(move |args| {
                // 简化实现：返回基础评估指标
                let thinking = args["thinking"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("thinking 必须是字符串".into()))?;
                let answer = args["answer"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("answer 必须是字符串".into()))?;

                Ok(serde_json::json!({
                    "thinking_length": thinking.len(),
                    "answer_length": answer.len(),
                    "has_structure": thinking.contains("<thinking>") || thinking.contains("思考"),
                    "quality_score": 0.8 // 简化评分
                }))
            }),
        });
    }

    /// 注册单个工具
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.to_string(), tool);
    }

    /// 调用工具
    pub fn call(&self, name: &str, args: Value) -> ToolResult {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        (tool.function)(args)
    }

    /// 获取所有工具定义
    pub fn list_tools(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|t| ToolInfo {
                name: t.name.to_string(),
                description: t.description.to_string(),
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 工具信息（用于序列化）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// 工具调用请求
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 工具调用响应
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl From<ToolResult> for ToolCallResponse {
    fn from(result: ToolResult) -> Self {
        match result {
            Ok(data) => ToolCallResponse {
                success: true,
                data: Some(data),
                error: None,
            },
            Err(e) => ToolCallResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            },
        }
    }
}
