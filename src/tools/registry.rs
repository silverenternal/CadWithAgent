//! 工具注册表
//!
//! 基于 tokitai 协议统一管理所有几何工具
//!
//! # 架构设计
//!
//! 本模块使用 tokitai 协议的工具定义系统：
//! 1. 所有工具使用 `#[tool]` 宏定义
//! 2. 工具自动注册到 tokitai 协议
//! 3. 提供统一的工具调用接口
//!
//! # 工具分类
//!
//! - **测量工具**: `measure_length`, `measure_area`, `measure_angle`
//! - **变换工具**: `translate`, `rotate`, `scale`, `mirror`
//! - **拓扑工具**: `detect_rooms`, `count_rooms`, `detect_doors`, `detect_windows`
//! - **IO 工具**: `parse_svg`, `parse_dxf`, `export_dxf`, `export_json`
//! - **`CoT` 工具**: `generate_geo_cot`, `generate_qa`
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::tools::ToolRegistry;
//! use serde_json::json;
//!
//! let registry = ToolRegistry::new();
//!
//! // 调用测量工具
//! let result = registry.call("measure_length", json!({
//!     "start": [0.0, 0.0],
//!     "end": [3.0, 4.0]
//! })).unwrap();
//!
//! assert_eq!(result.as_f64().unwrap(), 5.0);
//! ```

use crate::geometry::primitives::Primitive;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// 工具调用结果
pub type ToolResult = Result<Value, ToolError>;

/// 工具调用错误类型
///
/// 表示工具注册和调用过程中可能发生的错误。
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    /// 请求的工具不存在或未注册
    #[error("工具未找到：{0}")]
    NotFound(String),

    /// 工具调用执行失败
    #[error("工具调用失败：{0}")]
    CallFailed(String),

    /// 传入参数验证失败
    #[error("参数验证失败：{0}")]
    InvalidArgs(String),

    /// 工具执行过程中发生错误
    #[error("工具执行错误：{tool} - {message}")]
    ExecutionError { tool: String, message: String },
}

impl From<serde_json::Error> for ToolError {
    fn from(err: serde_json::Error) -> Self {
        ToolError::InvalidArgs(format!("JSON 解析失败：{err}"))
    }
}

// ==================== JSON 参数解析辅助函数 ====================

/// 从 JSON 数组中提取 2D 坐标 [x, y]
fn parse_coord_2d(value: &Value, field_name: &str) -> Result<[f64; 2], ToolError> {
    let arr = value[field_name]
        .as_array()
        .ok_or_else(|| ToolError::InvalidArgs(format!("{field_name} 必须是数组")))?;

    if arr.len() < 2 {
        return Err(ToolError::InvalidArgs(format!(
            "{field_name} 必须包含至少 2 个元素"
        )));
    }

    Ok([
        arr[0]
            .as_f64()
            .ok_or_else(|| ToolError::InvalidArgs(format!("{field_name}[0] 必须是数字")))?,
        arr[1]
            .as_f64()
            .ok_or_else(|| ToolError::InvalidArgs(format!("{field_name}[1] 必须是数字")))?,
    ])
}

/// 从 JSON 数组中提取点列表（用于多边形顶点等）
fn parse_coord_array(value: &Value, field_name: &str) -> Result<Vec<[f64; 2]>, ToolError> {
    let arr = value[field_name]
        .as_array()
        .ok_or_else(|| ToolError::InvalidArgs(format!("{field_name} 必须是数组")))?;

    arr.iter()
        .enumerate()
        .map(|(i, v)| {
            let coord_arr = v
                .as_array()
                .ok_or_else(|| ToolError::InvalidArgs(format!("{field_name}[{i}] 必须是数组")))?;

            if coord_arr.len() < 2 {
                return Err(ToolError::InvalidArgs(format!(
                    "{field_name}[{i}] 必须包含至少 2 个元素"
                )));
            }

            Ok([
                coord_arr[0].as_f64().ok_or_else(|| {
                    ToolError::InvalidArgs(format!("{field_name}[{i}][0] 必须是数字"))
                })?,
                coord_arr[1].as_f64().ok_or_else(|| {
                    ToolError::InvalidArgs(format!("{field_name}[{i}][1] 必须是数字"))
                })?,
            ])
        })
        .collect()
}

/// 将 f64 结果序列化为 JSON
fn f64_to_json(value: f64) -> Result<Value, ToolError> {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| ToolError::InvalidArgs("测量结果无法序列号为数字".into()))
}

// ==================== 工具定义 ====================

/// 工具函数类型
pub type ToolFn = Arc<dyn Fn(Value) -> ToolResult + Send + Sync>;

/// 工具定义
///
/// 包含工具的元数据和可调用函数。
#[derive(Clone)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub function: ToolFn,
}

/// 工具信息
///
/// 用于序列化和文档生成的工具元数据。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
}

/// 工具注册表
///
/// 管理所有可用的几何工具，提供统一的调用接口
///
/// # 线程安全
///
/// 本注册表是线程安全的，可以在多个线程间共享
#[derive(Clone)]
pub struct ToolRegistry {
    tools: Arc<HashMap<String, ToolDefinition>>,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    ///
    /// 自动注册所有可用工具
    pub fn new() -> Self {
        let mut registry = Self {
            tools: Arc::new(HashMap::new()),
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

    /// 注册测量工具
    fn register_measurement_tools(&mut self) {
        use crate::geometry::GeometryMeasurer;

        self.register(ToolDefinition {
            name: "measure_length",
            description: "测量两点之间的线段长度",
            function: Arc::new(move |args| {
                let mut measurer = GeometryMeasurer::new();
                let start = parse_coord_2d(&args, "start")?;
                let end = parse_coord_2d(&args, "end")?;

                let length = measurer.measure_length(start, end);
                if !length.is_finite() {
                    return Err(ToolError::InvalidArgs(
                        "计算结果无效，可能输入了非法坐标".into(),
                    ));
                }
                f64_to_json(length)
            }),
        });

        self.register(ToolDefinition {
            name: "measure_area",
            description: "计算多边形面积（使用鞋带公式）",
            function: Arc::new(move |args| {
                let mut measurer = GeometryMeasurer::new();
                let coords = parse_coord_array(&args, "vertices")?;

                let area = measurer.measure_area(coords);
                f64_to_json(area)
            }),
        });

        self.register(ToolDefinition {
            name: "measure_angle",
            description: "测量三个点形成的角度（顶点在中间）",
            function: Arc::new(move |args| {
                let mut measurer = GeometryMeasurer::new();
                let p1 = parse_coord_2d(&args, "p1")?;
                let p2 = parse_coord_2d(&args, "p2")?; // 顶点
                let p3 = parse_coord_2d(&args, "p3")?;

                let angle = measurer.measure_angle(p1, p2, p3);
                f64_to_json(angle)
            }),
        });

        self.register(ToolDefinition {
            name: "measure_perimeter",
            description: "计算多边形周长",
            function: Arc::new(move |args| {
                let mut measurer = GeometryMeasurer::new();
                let coords = parse_coord_array(&args, "vertices")?;

                let perimeter = measurer.measure_perimeter(coords);
                f64_to_json(perimeter)
            }),
        });

        self.register(ToolDefinition {
            name: "check_parallel",
            description: "检查两条线段是否平行",
            function: Arc::new(move |args| {
                let mut measurer = GeometryMeasurer::new();
                let line1_start = parse_coord_2d(&args, "line1_start")?;
                let line1_end = parse_coord_2d(&args, "line1_end")?;
                let line2_start = parse_coord_2d(&args, "line2_start")?;
                let line2_end = parse_coord_2d(&args, "line2_end")?;

                let result =
                    measurer.check_parallel(line1_start, line1_end, line2_start, line2_end);
                Ok(serde_json::json!({
                    "is_parallel": result.is_parallel,
                    "angle_diff": result.angle_diff
                }))
            }),
        });

        self.register(ToolDefinition {
            name: "check_perpendicular",
            description: "检查两条线段是否垂直",
            function: Arc::new(move |args| {
                let mut measurer = GeometryMeasurer::new();
                let line1_start = parse_coord_2d(&args, "line1_start")?;
                let line1_end = parse_coord_2d(&args, "line1_end")?;
                let line2_start = parse_coord_2d(&args, "line2_start")?;
                let line2_end = parse_coord_2d(&args, "line2_end")?;

                let result =
                    measurer.check_perpendicular(line1_start, line1_end, line2_start, line2_end);
                Ok(serde_json::json!({
                    "is_perpendicular": result.is_perpendicular,
                    "angle_diff": result.angle_diff
                }))
            }),
        });
    }

    /// 注册变换工具
    fn register_transform_tools(&mut self) {
        use crate::geometry::transform::{GeometryTransform, MirrorAxis};

        self.register(ToolDefinition {
            name: "translate",
            description: "平移图元（X 和 Y 方向移动）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let dx = args["dx"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("dx 必须是数字".into()))?;
                let dy = args["dy"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("dy 必须是数字".into()))?;

                let result = transform.translate(primitives, dx, dy);
                serde_json::to_value(&result)
                    .map_err(|e| ToolError::CallFailed(format!("序列化变换结果失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "rotate",
            description: "旋转图元（绕指定中心点，角度为度数）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let angle = args["angle"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("angle 必须是数字".into()))?;
                let center = parse_coord_2d(&args, "center")?;

                let result = transform.rotate(primitives, angle, center);
                serde_json::to_value(&result)
                    .map_err(|e| ToolError::CallFailed(format!("序列化变换结果失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "scale",
            description: "缩放图元（相对于指定中心点）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let factor = args["factor"]
                    .as_f64()
                    .ok_or_else(|| ToolError::InvalidArgs("factor 必须是数字".into()))?;
                let center = parse_coord_2d(&args, "center")?;

                let result = transform.scale(primitives, factor, center);
                serde_json::to_value(&result)
                    .map_err(|e| ToolError::CallFailed(format!("序列化变换结果失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "mirror",
            description: "镜像图元（关于 X 轴或 Y 轴）",
            function: Arc::new(move |args| {
                let transform = GeometryTransform;
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let axis = args["axis"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("axis 必须是字符串".into()))?;

                let mirror_axis = match axis.to_lowercase().as_str() {
                    "x" => MirrorAxis::X,
                    "y" => MirrorAxis::Y,
                    _ => return Err(ToolError::InvalidArgs("镜像轴必须是 'x' 或 'y'".into())),
                };

                let result = transform.mirror(primitives, mirror_axis);
                serde_json::to_value(&result)
                    .map_err(|e| ToolError::CallFailed(format!("序列化变换结果失败：{e}")))
            }),
        });
    }

    /// 注册拓扑分析工具
    fn register_topology_tools(&mut self) {
        use crate::topology::loop_detect::find_closed_loops;
        use crate::topology::room_detect::RoomDetector;

        self.register(ToolDefinition {
            name: "detect_rooms",
            description: "检测户型图中的所有房间",
            function: Arc::new(move |args| {
                let detector = RoomDetector;
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let rooms = detector.detect_rooms(primitives);
                serde_json::to_value(&rooms)
                    .map_err(|e| ToolError::CallFailed(format!("序列化房间数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "count_rooms",
            description: "统计房间数量",
            function: Arc::new(move |args| {
                let detector = RoomDetector;
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let count = detector.count_rooms(primitives);
                Ok(Value::Number(serde_json::Number::from(count)))
            }),
        });

        self.register(ToolDefinition {
            name: "detect_doors",
            description: "检测门的位置和数量",
            function: Arc::new(move |args| {
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let detector = RoomDetector;
                let result = detector.detect_rooms(primitives);
                let doors: Vec<_> = result.rooms.iter().flat_map(|r| r.doors.clone()).collect();
                serde_json::to_value(&doors)
                    .map_err(|e| ToolError::CallFailed(format!("序列化门数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "detect_windows",
            description: "检测窗户的位置和数量",
            function: Arc::new(move |args| {
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let detector = RoomDetector;
                let result = detector.detect_rooms(primitives);
                let windows: Vec<_> = result
                    .rooms
                    .iter()
                    .flat_map(|r| r.windows.clone())
                    .collect();
                serde_json::to_value(&windows)
                    .map_err(|e| ToolError::CallFailed(format!("序列化窗户数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "find_closed_loop",
            description: "查找闭合回路（用于房间边界检测）",
            function: Arc::new(move |args| {
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let loops = find_closed_loops(&primitives);
                serde_json::to_value(&loops)
                    .map_err(|e| ToolError::CallFailed(format!("序列化闭合回路数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "max_room_area",
            description: "找出最大房间面积",
            function: Arc::new(move |args| {
                let detector = RoomDetector;
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let max_area = detector.max_room_area(primitives);
                Ok(Value::Number(
                    serde_json::Number::from_f64(max_area).unwrap_or(serde_json::Number::from(0)),
                ))
            }),
        });
    }

    /// 注册 IO 工具
    fn register_io_tools(&mut self) {
        use crate::export::dxf::DxfExporter;
        use crate::export::json::JsonExporter;
        use crate::parser::dxf::DxfParser;
        use crate::parser::svg::SvgParser;

        self.register(ToolDefinition {
            name: "parse_svg",
            description: "解析 SVG 文件，提取几何图元",
            function: Arc::new(move |args| {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("path 必须是字符串".into()))?;

                // 安全检查：限制文件大小
                const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
                let metadata = std::fs::metadata(path)
                    .map_err(|e| ToolError::CallFailed(format!("无法访问文件：{e}")))?;

                if metadata.len() > MAX_FILE_SIZE {
                    return Err(ToolError::CallFailed(format!(
                        "文件过大：{} MB，最大允许 {} MB",
                        metadata.len() / 1024 / 1024,
                        MAX_FILE_SIZE / 1024 / 1024
                    )));
                }

                let result = SvgParser::parse(path)
                    .map_err(|e| ToolError::CallFailed(format!("SVG 解析失败：{e}")))?;

                serde_json::to_value(&result.primitives)
                    .map_err(|e| ToolError::CallFailed(format!("序列化图元数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "parse_dxf",
            description: "解析 DXF 文件，提取几何图元",
            function: Arc::new(move |args| {
                let path = args["path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("path 必须是字符串".into()))?;

                // 安全检查：限制文件大小
                const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB
                let metadata = std::fs::metadata(path)
                    .map_err(|e| ToolError::CallFailed(format!("无法访问文件：{e}")))?;

                if metadata.len() > MAX_FILE_SIZE {
                    return Err(ToolError::CallFailed(format!(
                        "文件过大：{} MB，最大允许 {} MB",
                        metadata.len() / 1024 / 1024,
                        MAX_FILE_SIZE / 1024 / 1024
                    )));
                }

                let result = DxfParser::parse(path)
                    .map_err(|e| ToolError::CallFailed(format!("DXF 解析失败：{e}")))?;

                serde_json::to_value(&result.primitives)
                    .map_err(|e| ToolError::CallFailed(format!("序列化图元数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "export_dxf",
            description: "将几何图元导出为 DXF 文件",
            function: Arc::new(move |args| {
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let output_path = args["output_path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("output_path 必须是字符串".into()))?;

                let result = DxfExporter::export(&primitives, output_path)
                    .map_err(|e| ToolError::CallFailed(format!("DXF 导出失败：{e}")))?;

                serde_json::to_value(&result)
                    .map_err(|e| ToolError::CallFailed(format!("序列化导出结果失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "export_json",
            description: "将几何图元导出为 JSON 文件",
            function: Arc::new(move |args| {
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let output_path = args["output_path"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("output_path 必须是字符串".into()))?;

                let result = JsonExporter::export(&primitives, output_path)
                    .map_err(|e| ToolError::CallFailed(format!("JSON 导出失败：{e}")))?;

                serde_json::to_value(&result)
                    .map_err(|e| ToolError::CallFailed(format!("序列化导出结果失败：{e}")))
            }),
        });
    }

    /// 注册 Geo-CoT 工具
    fn register_cot_tools(&mut self) {
        use crate::cot::generator::GeoCotGenerator;
        use crate::cot::qa::QaGenerator;

        self.register(ToolDefinition {
            name: "generate_geo_cot",
            description: "生成几何思维链数据",
            function: Arc::new(move |args| {
                let generator = GeoCotGenerator::new();
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let task = args["task"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("task 必须是字符串".into()))?;

                let cot_data = generator.generate(&primitives, task);
                serde_json::to_value(&cot_data)
                    .map_err(|e| ToolError::CallFailed(format!("序列化 CoT 数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "generate_qa",
            description: "生成问答对数据集",
            function: Arc::new(move |args| {
                let generator = QaGenerator::new();
                let primitives: Vec<Primitive> = args["primitives"]
                    .as_array()
                    .ok_or_else(|| ToolError::InvalidArgs("primitives 必须是数组".into()))?
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();

                let qa_pairs = generator.generate_all(&primitives);
                serde_json::to_value(&qa_pairs)
                    .map_err(|e| ToolError::CallFailed(format!("序列化 QA 数据失败：{e}")))
            }),
        });

        self.register(ToolDefinition {
            name: "evaluate_cot",
            description: "评估思维链质量",
            function: Arc::new(move |args| {
                let thinking = args["thinking"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("thinking 必须是字符串".into()))?;
                let answer = args["answer"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("answer 必须是字符串".into()))?;

                // 简化实现：返回基础评估指标
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
        // 使用 Arc::make_mut 实现写时复制
        let tools = Arc::make_mut(&mut self.tools);
        tools.insert(tool.name.to_string(), tool);
    }

    /// 调用工具
    ///
    /// # Errors
    /// 如果工具未找到或执行失败，返回 `ToolError`
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

    /// 获取工具数量
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// 检查工具是否存在
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 工具调用请求
///
/// 表示调用工具的请求，包含工具名称和参数。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 工具调用响应
///
/// 工具调用的执行结果，包含成功状态、数据或错误信息。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_new() {
        let registry = ToolRegistry::new();
        assert!(registry.tool_count() > 0);
    }

    #[test]
    fn test_tool_registry_default() {
        let registry = ToolRegistry::default();
        assert!(registry.tool_count() > 0);
    }

    #[test]
    fn test_tool_registry_list_tools() {
        let registry = ToolRegistry::new();
        let tools = registry.list_tools();
        assert!(!tools.is_empty());

        // 验证工具信息包含必要字段
        for tool in tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
        }
    }

    #[test]
    fn test_tool_registry_has_tool() {
        let registry = ToolRegistry::new();
        assert!(registry.has_tool("measure_length"));
        assert!(registry.has_tool("measure_area"));
        assert!(!registry.has_tool("nonexistent_tool"));
    }

    #[test]
    fn test_tool_error_display() {
        let err = ToolError::NotFound("test_tool".to_string());
        assert!(err.to_string().contains("test_tool"));
        assert!(err.to_string().contains("工具未找到"));

        let err = ToolError::CallFailed("execution error".to_string());
        assert!(err.to_string().contains("工具调用失败"));

        let err = ToolError::InvalidArgs("invalid args".to_string());
        assert!(err.to_string().contains("参数验证失败"));
    }

    #[test]
    fn test_tool_call_not_found() {
        let registry = ToolRegistry::new();
        let result = registry.call("nonexistent_tool", serde_json::json!({}));
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::NotFound(name) => assert_eq!(name, "nonexistent_tool"),
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_measure_length() {
        let registry = ToolRegistry::new();
        let args = serde_json::json!({
            "start": [0.0, 0.0],
            "end": [3.0, 4.0]
        });

        let result = registry.call("measure_length", args);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!((value.as_f64().unwrap() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_measure_length_invalid_args() {
        let registry = ToolRegistry::new();

        // Missing start
        let args = serde_json::json!({"end": [3.0, 4.0]});
        let result = registry.call("measure_length", args);
        assert!(result.is_err());

        // Invalid start type
        let args = serde_json::json!({"start": "invalid", "end": [3.0, 4.0]});
        let result = registry.call("measure_length", args);
        assert!(result.is_err());

        // Invalid coordinate type
        let args = serde_json::json!({"start": ["a", 0.0], "end": [3.0, 4.0]});
        let result = registry.call("measure_length", args);
        assert!(result.is_err());
    }

    #[test]
    fn test_measure_area() {
        let registry = ToolRegistry::new();
        // Unit square
        let args = serde_json::json!({
            "vertices": [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
        });

        let result = registry.call("measure_area", args);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!((value.as_f64().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_translate() {
        let registry = ToolRegistry::new();
        let primitives = serde_json::json!([
            {"type": "point", "x": 0.0, "y": 0.0}
        ]);
        let args = serde_json::json!({
            "primitives": primitives,
            "dx": 5.0,
            "dy": 10.0
        });

        let result = registry.call("translate", args);
        assert!(result.is_ok());
        let value = result.unwrap();
        let arr = value.as_array().unwrap();
        assert_eq!(arr.len(), 1);
    }
}
