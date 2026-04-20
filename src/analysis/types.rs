//! 分析管线数据类型
//!
//! 定义统一的几何分析管线使用的数据结构

use crate::analysis::closed_region_detector::{AreaStatistics, RoomTypeStatistics};
use crate::cad_reasoning::GeometricRelation;
use crate::cad_verifier::VerificationResult;
use crate::geometry::primitives::Primitive;
use crate::prompt_builder::StructuredPrompt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 文字识别结果（OCR）
///
/// 记录从 CAD 图纸或户型图中提取的文字信息，
/// 用于房间标注、尺寸标注等语义识别。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OcrResult {
    /// 识别到的文字列表
    pub texts: Vec<TextAnnotation>,
    /// 文字总数
    pub text_count: usize,
}

/// 文字标注信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    /// 文字内容
    pub content: String,
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
    /// 文字高度
    pub height: Option<f64>,
    /// 文字宽度
    pub width: Option<f64>,
    /// 旋转角度（度）
    pub rotation: Option<f64>,
    /// 所属图层
    pub layer: Option<String>,
    /// 置信度
    pub confidence: Option<f64>,
}

/// 封闭区域信息
///
/// 表示户型图中识别出的一个封闭房间区域，
/// 包含边界、面积、房间类型等语义信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedRegion {
    /// 区域唯一标识
    pub id: usize,
    /// 构成封闭区域的基元 ID 列表
    pub boundary_primitive_ids: Vec<usize>,
    /// 区域顶点（按顺时针或逆时针排序）
    pub vertices: Vec<Point>,
    /// 区域面积（使用鞋带公式计算）
    pub area: f64,
    /// 区域周长
    pub perimeter: f64,
    /// 区域质心
    pub centroid: Point,
    /// 房间类型（从 OCR 文字推断）
    pub room_type: Option<String>,
    /// 置信度
    pub confidence: f64,
    /// 是否为外边界
    pub is_outer_boundary: bool,
    /// 矩形度（0-1，1 表示完美矩形）
    pub rectangularity: Option<f64>,
    /// 紧凑度（0-1，1 表示完美圆形）
    pub compactness: Option<f64>,
    /// 长宽比（长度/宽度）
    pub aspect_ratio: Option<f64>,
    /// 凸度（0-1，1 表示凸多边形）
    pub convexity: Option<f64>,
    /// 方向角（主轴与 X 轴夹角，弧度）
    pub orientation: Option<f64>,
    /// 圆度（0-1，1 表示完美圆形）
    pub circularity: Option<f64>,
    /// 形状因子（形状复杂度度量）
    pub shape_factor: Option<f64>,
}

/// 区域邻接关系
///
/// 表示两个封闭区域之间的邻接关系，
/// 包含共享边界信息和邻接类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionAdjacency {
    /// 区域 1 的 ID
    pub region_id_1: usize,
    /// 区域 2 的 ID
    pub region_id_2: usize,
    /// 共享的基元 ID 列表（共用墙体）
    pub shared_primitive_ids: Vec<usize>,
    /// 邻接类型（如：共墙、共点等）
    pub adjacency_type: String,
    /// 共享边界的长度
    pub shared_length: f64,
    /// 置信度
    pub confidence: f64,
}

/// 区域邻接图
///
/// 表示所有封闭区域之间的邻接关系网络，
/// 可用于分析房间连通性和布局结构。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegionAdjacencyGraph {
    /// 邻接关系列表
    pub adjacencies: Vec<RegionAdjacency>,
    /// 邻接矩阵（用于快速查询）
    pub adjacency_matrix: HashMap<(usize, usize), Vec<usize>>,
}

impl RegionAdjacencyGraph {
    /// 创建新的邻接图
    pub fn new() -> Self {
        Self {
            adjacencies: Vec::new(),
            adjacency_matrix: HashMap::new(),
        }
    }

    /// 添加邻接关系
    pub fn add_adjacency(&mut self, adjacency: RegionAdjacency) {
        // 存储双向关系
        let shared_ids = adjacency.shared_primitive_ids.clone();
        self.adjacency_matrix.insert(
            (adjacency.region_id_1, adjacency.region_id_2),
            shared_ids.clone(),
        );
        self.adjacency_matrix
            .insert((adjacency.region_id_2, adjacency.region_id_1), shared_ids);
        self.adjacencies.push(adjacency);
    }

    /// 查询两个区域是否邻接
    pub fn is_adjacent(&self, region_id_1: usize, region_id_2: usize) -> bool {
        self.adjacency_matrix
            .contains_key(&(region_id_1, region_id_2))
    }

    /// 获取某个区域的所有邻接区域
    pub fn get_adjacent_regions(&self, region_id: usize) -> Vec<usize> {
        self.adjacency_matrix
            .iter()
            .filter_map(|((id1, id2), _)| {
                if *id1 == region_id {
                    Some(*id2)
                } else if *id2 == region_id {
                    Some(*id1)
                } else {
                    None
                }
            })
            .collect()
    }

    /// 获取邻接区域数量
    pub fn count(&self) -> usize {
        self.adjacencies.len()
    }
}

/// 二维点（简化版，用于封闭区域）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl From<crate::geometry::primitives::Point> for Point {
    fn from(p: crate::geometry::primitives::Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl Point {
    /// 计算到另一点的距离
    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

impl Default for TextAnnotation {
    fn default() -> Self {
        Self {
            content: String::new(),
            x: 0.0,
            y: 0.0,
            height: None,
            width: None,
            rotation: None,
            layer: None,
            confidence: None,
        }
    }
}

/// VLM（Vision-Language Model）响应信息
///
/// 记录 VLM 模型对几何分析请求的完整响应数据，
/// 包括内容、模型信息、Token 统计和性能指标。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlmResponseInfo {
    /// 模型返回的内容
    pub content: String,
    /// 使用的模型名称
    pub model: String,
    /// Token 使用统计
    pub usage: Option<TokenUsageInfo>,
    /// 延迟（毫秒）
    pub latency_ms: u64,
}

/// Token 使用统计信息
///
/// 记录 VLM 模型调用过程中的 Token 消耗情况。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageInfo {
    /// 提示词 token 数
    pub prompt_tokens: u32,
    /// 完成 token 数
    pub completion_tokens: u32,
    /// 总 token 数
    pub total_tokens: u32,
}

/// 工具调用链中的单个步骤
///
/// 记录工具调用过程中每个步骤的执行信息，包括性能指标、
/// 置信度和依赖关系，用于可解释性分析和实验追踪。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallStep {
    /// 步骤序号
    pub step: usize,
    /// 工具名称
    pub tool_name: String,
    /// 工具描述
    pub description: String,
    /// 执行耗时（毫秒）
    pub latency_ms: u64,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 输出统计信息
    pub output_stats: HashMap<String, serde_json::Value>,
    /// 置信度（1.0 表示确定性，<1.0 表示 VLM 推断的不确定性）
    pub confidence: f64,
    /// 前驱步骤 ID 列表（用于置信度传播和依赖追踪）
    pub predecessors: Vec<usize>,
}

/// 工具调用链追踪
///
/// 记录完整的工具调用序列，支持置信度传播和依赖追踪。
/// 用于分析 VLM 工具调用的可解释性、性能和质量评估。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallChain {
    /// 调用步骤列表
    pub steps: Vec<ToolCallStep>,
    /// 总耗时（毫秒）
    pub total_latency_ms: u64,
    /// 是否全部成功
    pub all_success: bool,
    /// 整体置信度（通过置信度传播计算）
    pub overall_confidence: f64,
}

impl ToolCallChain {
    /// 创建新的调用链
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加步骤
    pub fn add_step(&mut self, step: ToolCallStep) {
        self.all_success = self.all_success && step.success;
        self.total_latency_ms += step.latency_ms;
        self.steps.push(step);
    }

    /// 转换为 JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "steps": self.steps,
            "total_latency_ms": self.total_latency_ms,
            "all_success": self.all_success,
            "overall_confidence": self.overall_confidence
        })
    }

    /// 置信度传播算法
    ///
    /// 基于贝叶斯网络的置信度传播模型：
    /// - 每个步骤的置信度独立计算
    /// - 依赖步骤的置信度通过乘法传播：P(A∧B) = P(A) × P(B|A)
    /// - 无依赖步骤的置信度直接相乘
    /// - 失败步骤的置信度为 0
    ///
    /// # Returns
    /// 返回整体置信度（0.0 到 1.0 之间）
    pub fn propagate_confidence(&mut self) -> f64 {
        if self.steps.is_empty() {
            return 1.0; // 空链默认置信度为 1.0
        }

        // 构建步骤索引映射
        let step_map: HashMap<usize, &ToolCallStep> =
            self.steps.iter().map(|s| (s.step, s)).collect();

        // 计算每个步骤的传播置信度
        let mut propagated_confidences: HashMap<usize, f64> = HashMap::new();

        for step in &self.steps {
            let propagated =
                self.calculate_propagated_confidence(step, &step_map, &mut propagated_confidences);
            propagated_confidences.insert(step.step, propagated);
        }

        // 整体置信度为所有步骤传播置信度的乘积
        let overall: f64 = propagated_confidences.values().product();
        self.overall_confidence = overall;
        overall
    }

    /// 计算单个步骤的传播置信度（递归）
    fn calculate_propagated_confidence(
        &self,
        step: &ToolCallStep,
        step_map: &HashMap<usize, &ToolCallStep>,
        cache: &mut HashMap<usize, f64>,
    ) -> f64 {
        // 如果已计算过，直接返回缓存值
        if let Some(&cached) = cache.get(&step.step) {
            return cached;
        }

        // 失败步骤的置信度为 0
        if !step.success {
            return 0.0;
        }

        // 如果没有前驱，返回自身置信度
        if step.predecessors.is_empty() {
            return step.confidence;
        }

        // 递归计算前驱步骤的传播置信度
        let predecessors_confidence: f64 = step
            .predecessors
            .iter()
            .filter_map(|&pred_id| step_map.get(&pred_id))
            .map(|pred_step| self.calculate_propagated_confidence(pred_step, step_map, cache))
            .product();

        // 传播置信度 = 前驱置信度乘积 × 当前步骤置信度
        predecessors_confidence * step.confidence
    }

    /// 获取置信度追溯图
    ///
    /// # Returns
    /// 返回每个步骤的置信度追溯信息，包括：
    /// - 步骤 ID
    /// - 原始置信度
    /// - 传播后的置信度
    /// - 依赖的前驱步骤
    pub fn get_confidence_trace(&mut self) -> Vec<ConfidenceTrace> {
        if self.steps.is_empty() {
            return Vec::new();
        }

        // 先执行置信度传播
        self.propagate_confidence();

        let step_map: HashMap<usize, &ToolCallStep> =
            self.steps.iter().map(|s| (s.step, s)).collect();

        let mut traces = Vec::new();
        let mut cache: HashMap<usize, f64> = HashMap::new();

        for step in &self.steps {
            let propagated = self.calculate_propagated_confidence(step, &step_map, &mut cache);
            traces.push(ConfidenceTrace {
                step_id: step.step,
                tool_name: step.tool_name.clone(),
                original_confidence: step.confidence,
                propagated_confidence: propagated,
                predecessors: step.predecessors.clone(),
                success: step.success,
            });
        }

        traces
    }

    /// 设置步骤的置信度
    pub fn set_step_confidence(&mut self, step_id: usize, confidence: f64) -> bool {
        if let Some(step) = self.steps.iter_mut().find(|s| s.step == step_id) {
            step.confidence = confidence.clamp(0.0, 1.0);
            true
        } else {
            false
        }
    }

    /// 添加步骤依赖关系
    pub fn add_dependency(&mut self, step_id: usize, predecessor_id: usize) -> bool {
        if let Some(step) = self.steps.iter_mut().find(|s| s.step == step_id) {
            if !step.predecessors.contains(&predecessor_id) {
                step.predecessors.push(predecessor_id);
            }
            true
        } else {
            false
        }
    }
}

/// 置信度追溯信息
///
/// 记录工具调用步骤的置信度传播详情，用于追溯分析
/// 每个步骤的置信度来源和依赖影响。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceTrace {
    /// 步骤 ID
    pub step_id: usize,
    /// 工具名称
    pub tool_name: String,
    /// 原始置信度
    pub original_confidence: f64,
    /// 传播后的置信度
    pub propagated_confidence: f64,
    /// 依赖的前驱步骤 ID 列表
    pub predecessors: Vec<usize>,
    /// 步骤是否成功
    pub success: bool,
}

/// 分析管线配置
///
/// 统一配置各子模块参数，减少重复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// 是否启用坐标归一化
    pub enable_normalization: bool,
    /// 归一化范围 [min, max]
    pub normalize_range: [f64; 2],
    /// 角度容差（弧度）
    pub angle_tolerance: f64,
    /// 距离容差
    pub distance_tolerance: f64,
    /// 最小置信度阈值
    pub min_confidence: f64,
    /// 是否跳过校验步骤
    pub skip_verification: bool,
    /// 是否包含详细日志
    pub verbose: bool,
    /// 最大基元显示数量
    pub max_primitives_display: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            enable_normalization: true,
            normalize_range: [0.0, 100.0],
            angle_tolerance: 0.01, // ~0.57 度
            distance_tolerance: 0.01,
            min_confidence: 0.8,
            skip_verification: false,
            verbose: false,
            max_primitives_display: 50,
        }
    }
}

impl From<crate::error::GeometryConfig> for AnalysisConfig {
    fn from(geo_config: crate::error::GeometryConfig) -> Self {
        Self {
            enable_normalization: geo_config.enable_normalization,
            normalize_range: geo_config.normalize_range,
            angle_tolerance: geo_config.angle_tolerance,
            distance_tolerance: geo_config.distance_tolerance,
            min_confidence: geo_config.min_confidence,
            skip_verification: false,
            verbose: false,
            max_primitives_display: 50,
        }
    }
}

impl AnalysisConfig {
    /// 验证配置参数的合理性
    pub fn validate(&self) -> crate::error::CadAgentResult<()> {
        use crate::error::CadAgentError;

        // 验证归一化范围
        if self.normalize_range[0] >= self.normalize_range[1] {
            return Err(CadAgentError::Config {
                parameter: "normalize_range".to_string(),
                value: self.normalize_range[0],
                message: format!(
                    "归一化范围无效：[{}, {}]。最小值必须小于最大值。建议值：[0.0, 100.0]",
                    self.normalize_range[0], self.normalize_range[1]
                ),
                suggestion: Some("使用 [0.0, 100.0] 作为归一化范围".to_string()),
            });
        }

        // 验证角度容差
        if self.angle_tolerance <= 0.0 {
            return Err(CadAgentError::Config {
                parameter: "angle_tolerance".to_string(),
                value: self.angle_tolerance,
                message: format!(
                    "角度容差必须为正数，当前值：{}。建议值：0.01（约 0.57 度）",
                    self.angle_tolerance
                ),
                suggestion: Some("使用 0.01 作为角度容差".to_string()),
            });
        }
        if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            return Err(CadAgentError::Config {
                parameter: "angle_tolerance".to_string(),
                value: self.angle_tolerance,
                message: format!(
                    "角度容差过大（{} 弧度 ≈ {:.2} 度），最大允许 90 度（π/2）。建议值：0.01",
                    self.angle_tolerance,
                    self.angle_tolerance.to_degrees()
                ),
                suggestion: Some("使用 0.01 作为角度容差".to_string()),
            });
        }

        // 验证距离容差
        if self.distance_tolerance < 0.0 {
            return Err(CadAgentError::Config {
                parameter: "distance_tolerance".to_string(),
                value: self.distance_tolerance,
                message: format!("距离容差必须为非负数，当前值：{}", self.distance_tolerance),
                suggestion: Some("使用 0.1 作为距离容差".to_string()),
            });
        }

        // 验证置信度
        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            return Err(CadAgentError::Config {
                parameter: "min_confidence".to_string(),
                value: self.min_confidence,
                message: format!(
                    "最小置信度必须在 0 到 1 之间，当前值：{}",
                    self.min_confidence
                ),
                suggestion: Some("使用 0.5 作为最小置信度".to_string()),
            });
        }

        Ok(())
    }

    /// 验证并自动修正不合理的配置
    pub fn validate_or_fix(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let default = AnalysisConfig::default();

        if self.normalize_range[0] >= self.normalize_range[1] {
            warnings.push(format!(
                "归一化范围 [{}, {}] 无效，已修正为默认值 [{}, {}]",
                self.normalize_range[0],
                self.normalize_range[1],
                default.normalize_range[0],
                default.normalize_range[1]
            ));
            self.normalize_range = default.normalize_range;
        }

        if self.angle_tolerance <= 0.0 || self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            warnings.push(format!(
                "角度容差 {} 无效，已修正为默认值 {}",
                self.angle_tolerance, default.angle_tolerance
            ));
            self.angle_tolerance = default.angle_tolerance;
        }

        if self.distance_tolerance < 0.0 {
            warnings.push(format!(
                "距离容差 {} 无效，已修正为默认值 {}",
                self.distance_tolerance, default.distance_tolerance
            ));
            self.distance_tolerance = default.distance_tolerance;
        }

        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            warnings.push(format!(
                "最小置信度 {} 无效，已修正为默认值 {}",
                self.min_confidence, default.min_confidence
            ));
            self.min_confidence = default.min_confidence;
        }

        warnings
    }
}

/// 分析管线结果
///
/// 汇总几何分析管线的完整输出，包括基元提取、约束推理、
/// 校验结果和性能指标。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// 提取的基元
    pub primitives: Vec<Primitive>,
    /// 推理的约束关系
    pub relations: Vec<GeometricRelation>,
    /// 校验结果（如果启用）
    pub verification: Option<VerificationResult>,
    /// 生成的结构化提示词
    pub prompt: StructuredPrompt,
    /// 管线执行日志
    pub execution_log: Vec<String>,
    /// 总耗时（毫秒）
    pub total_latency_ms: u64,
    /// VLM 推理结果（如果执行了 VLM 推理）
    pub vlm_response: Option<VlmResponseInfo>,
    /// 工具调用链追踪（用于可解释性和实验分析）
    pub tool_call_chain: Option<ToolCallChain>,
    /// OCR 文字识别结果
    pub ocr_result: Option<OcrResult>,
    /// 封闭区域列表（房间识别结果）
    pub closed_regions: Vec<ClosedRegion>,
    /// 区域邻接图（房间连通性分析）
    pub region_adjacency: Option<RegionAdjacencyGraph>,
    /// 额外信息（用于扩展）
    pub additional: serde_json::Value,
}

/// 区域导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionExport {
    /// 区域 ID
    pub id: usize,
    /// 房间类型
    pub room_type: String,
    /// 面积
    pub area: f64,
    /// 周长
    pub perimeter: f64,
    /// 质心 X
    pub centroid_x: f64,
    /// 质心 Y
    pub centroid_y: f64,
    /// 矩形度
    pub rectangularity: Option<f64>,
    /// 紧凑度
    pub compactness: Option<f64>,
    /// 长宽比
    pub aspect_ratio: Option<f64>,
    /// 凸度
    pub convexity: Option<f64>,
    /// 方向角（弧度）
    pub orientation: Option<f64>,
    /// 圆度
    pub circularity: Option<f64>,
    /// 形状因子
    pub shape_factor: Option<f64>,
    /// 边界基元数量
    pub boundary_count: usize,
    /// 是否为外边界
    pub is_outer_boundary: bool,
    /// 置信度
    pub confidence: f64,
}

impl From<&ClosedRegion> for RegionExport {
    fn from(region: &ClosedRegion) -> Self {
        Self {
            id: region.id,
            room_type: region
                .room_type
                .clone()
                .unwrap_or_else(|| "未知".to_string()),
            area: region.area,
            perimeter: region.perimeter,
            centroid_x: region.centroid.x,
            centroid_y: region.centroid.y,
            rectangularity: region.rectangularity,
            compactness: region.compactness,
            aspect_ratio: region.aspect_ratio,
            convexity: region.convexity,
            orientation: region.orientation,
            circularity: region.circularity,
            shape_factor: region.shape_factor,
            boundary_count: region.boundary_primitive_ids.len(),
            is_outer_boundary: region.is_outer_boundary,
            confidence: region.confidence,
        }
    }
}

/// 户型图分析报告
///
/// 包含完整的户型分析结果，用于生成报告或导出数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorPlanReport {
    /// 报告标题
    pub title: String,
    /// 生成时间
    pub generated_at: String,
    /// 基本信息
    pub summary: ReportSummary,
    /// 区域详细列表
    pub regions: Vec<RegionExport>,
    /// 面积统计
    pub area_statistics: AreaStatistics,
    /// 房间类型统计
    pub room_type_statistics: RoomTypeStatistics,
    /// 邻接关系摘要
    pub adjacency_summary: AdjacencySummary,
    /// 分析建议
    pub recommendations: Vec<String>,
}

/// 报告摘要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    /// 总房间数（不包括外边界）
    pub total_rooms: usize,
    /// 总面积（不包括外边界）
    pub total_area: f64,
    /// 平均房间面积
    pub avg_room_area: f64,
    /// 最大房间面积
    pub max_room_area: f64,
    /// 最小房间面积
    pub min_room_area: f64,
    /// 房间类型数量
    pub room_type_count: usize,
    /// 主要房间类型（数量最多的类型）
    pub dominant_room_type: Option<String>,
    /// 户型紧凑度（所有房间紧凑度平均值）
    pub overall_compactness: Option<f64>,
    /// 户型矩形度（所有房间矩形度平均值）
    pub overall_rectangularity: Option<f64>,
}

/// 邻接关系摘要
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdjacencySummary {
    /// 总邻接关系数
    pub total_adjacencies: usize,
    /// 平均每个房间的邻接数
    pub avg_adjacencies_per_room: f64,
    /// 邻接度最高的房间 ID
    pub most_connected_room_id: Option<usize>,
    /// 邻接度最高的房间的邻接数
    pub max_adjacencies: usize,
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalysisResult {
    /// 创建新的分析结果
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            relations: Vec::new(),
            verification: None,
            prompt: StructuredPrompt {
                full_prompt: String::new(),
                system_prompt: String::new(),
                user_prompt: String::new(),
                metadata: crate::prompt_builder::PromptMetadata {
                    primitive_count: 0,
                    constraint_count: 0,
                    prompt_length: 0,
                    template: crate::prompt_builder::PromptTemplate::Analysis,
                    injected_context: Vec::new(),
                },
            },
            execution_log: Vec::new(),
            total_latency_ms: 0,
            vlm_response: None,
            tool_call_chain: None,
            ocr_result: None,
            closed_regions: Vec::new(),
            region_adjacency: None,
            additional: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// 获取基元数量
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// 获取关系数量
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    /// 获取工具调用链的 JSON 表示
    pub fn tool_chain_json(&self) -> serde_json::Value {
        self.tool_call_chain
            .as_ref()
            .map_or_else(|| serde_json::json!(null), ToolCallChain::to_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_call_chain_confidence_propagation_no_dependencies() {
        // 测试无依赖关系的置信度传播
        let mut chain = ToolCallChain::new();

        // 添加三个独立步骤，置信度分别为 0.9, 0.8, 0.7
        chain.add_step(ToolCallStep {
            step: 1,
            tool_name: "tool1".to_string(),
            description: "Step 1".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.9,
            predecessors: Vec::new(),
        });

        chain.add_step(ToolCallStep {
            step: 2,
            tool_name: "tool2".to_string(),
            description: "Step 2".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.8,
            predecessors: Vec::new(),
        });

        chain.add_step(ToolCallStep {
            step: 3,
            tool_name: "tool3".to_string(),
            description: "Step 3".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.7,
            predecessors: Vec::new(),
        });

        // 整体置信度应该是 0.9 * 0.8 * 0.7 = 0.504
        let overall = chain.propagate_confidence();
        assert!((overall - 0.504).abs() < 1e-10);
    }

    #[test]
    fn test_tool_call_chain_confidence_propagation_with_dependencies() {
        // 测试有依赖关系的置信度传播
        let mut chain = ToolCallChain::new();

        // Step 1: 置信度 0.9，无依赖
        chain.add_step(ToolCallStep {
            step: 1,
            tool_name: "tool1".to_string(),
            description: "Step 1".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.9,
            predecessors: Vec::new(),
        });

        // Step 2: 置信度 0.8，依赖于 Step 1
        chain.add_step(ToolCallStep {
            step: 2,
            tool_name: "tool2".to_string(),
            description: "Step 2".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.8,
            predecessors: vec![1],
        });

        // Step 3: 置信度 0.7，依赖于 Step 2
        chain.add_step(ToolCallStep {
            step: 3,
            tool_name: "tool3".to_string(),
            description: "Step 3".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.7,
            predecessors: vec![2],
        });

        let overall = chain.propagate_confidence();

        // Step 1: 0.9
        // Step 2: 0.9 * 0.8 = 0.72
        // Step 3: 0.72 * 0.7 = 0.504
        // Overall: 0.9 * 0.72 * 0.504 = 0.326592
        assert!((overall - 0.326592).abs() < 1e-10);
    }

    #[test]
    fn test_tool_call_chain_confidence_failed_step() {
        // 测试失败步骤的置信度传播
        let mut chain = ToolCallChain::new();

        chain.add_step(ToolCallStep {
            step: 1,
            tool_name: "tool1".to_string(),
            description: "Step 1".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.9,
            predecessors: Vec::new(),
        });

        // 失败步骤
        chain.add_step(ToolCallStep {
            step: 2,
            tool_name: "tool2".to_string(),
            description: "Step 2".to_string(),
            latency_ms: 100,
            success: false,
            error: Some("Error".to_string()),
            output_stats: HashMap::new(),
            confidence: 0.8,
            predecessors: Vec::new(),
        });

        let overall = chain.propagate_confidence();

        // 由于 Step 2 失败，其置信度为 0，整体置信度应为 0
        assert_eq!(overall, 0.0);
        assert!(!chain.all_success);
    }

    #[test]
    fn test_tool_call_chain_confidence_trace() {
        // 测试置信度追溯功能
        let mut chain = ToolCallChain::new();

        chain.add_step(ToolCallStep {
            step: 1,
            tool_name: "tool1".to_string(),
            description: "Step 1".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.9,
            predecessors: Vec::new(),
        });

        chain.add_step(ToolCallStep {
            step: 2,
            tool_name: "tool2".to_string(),
            description: "Step 2".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.8,
            predecessors: vec![1],
        });

        let trace = chain.get_confidence_trace();

        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].step_id, 1);
        assert_eq!(trace[0].original_confidence, 0.9);
        assert_eq!(trace[0].predecessors.len(), 0);

        assert_eq!(trace[1].step_id, 2);
        assert_eq!(trace[1].original_confidence, 0.8);
        assert_eq!(trace[1].predecessors, vec![1]);
        // Step 2 的传播置信度 = 0.9 * 0.8 = 0.72
        assert!((trace[1].propagated_confidence - 0.72).abs() < 1e-10);
    }

    #[test]
    fn test_tool_call_chain_set_confidence() {
        let mut chain = ToolCallChain::new();

        chain.add_step(ToolCallStep {
            step: 1,
            tool_name: "tool1".to_string(),
            description: "Step 1".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.9,
            predecessors: Vec::new(),
        });

        // 修改置信度
        assert!(chain.set_step_confidence(1, 0.95));
        assert_eq!(chain.steps[0].confidence, 0.95);

        // 置信度应该被限制在 [0, 1] 范围内
        assert!(chain.set_step_confidence(1, 1.5));
        assert_eq!(chain.steps[0].confidence, 1.0);

        // 不存在的步骤
        assert!(!chain.set_step_confidence(999, 0.5));
    }

    #[test]
    fn test_tool_call_chain_add_dependency() {
        let mut chain = ToolCallChain::new();

        chain.add_step(ToolCallStep {
            step: 1,
            tool_name: "tool1".to_string(),
            description: "Step 1".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.9,
            predecessors: Vec::new(),
        });

        chain.add_step(ToolCallStep {
            step: 2,
            tool_name: "tool2".to_string(),
            description: "Step 2".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.8,
            predecessors: Vec::new(),
        });

        // 添加依赖关系
        assert!(chain.add_dependency(2, 1));
        assert_eq!(chain.steps[1].predecessors, vec![1]);

        // 重复添加应该不会增加
        assert!(chain.add_dependency(2, 1));
        assert_eq!(chain.steps[1].predecessors, vec![1]);

        // 不存在的步骤
        assert!(!chain.add_dependency(999, 1));
    }

    #[test]
    fn test_tool_call_chain_empty_confidence() {
        let mut chain = ToolCallChain::new();
        let overall = chain.propagate_confidence();
        assert_eq!(overall, 1.0); // 空链默认置信度为 1.0
    }

    #[test]
    fn test_tool_call_chain_to_json() {
        let mut chain = ToolCallChain::new();

        chain.add_step(ToolCallStep {
            step: 1,
            tool_name: "tool1".to_string(),
            description: "Step 1".to_string(),
            latency_ms: 100,
            success: true,
            error: None,
            output_stats: HashMap::new(),
            confidence: 0.9,
            predecessors: Vec::new(),
        });

        let json = chain.to_json();
        assert!(json.get("steps").is_some());
        assert!(json.get("total_latency_ms").is_some());
        assert!(json.get("all_success").is_some());
        assert!(json.get("overall_confidence").is_some());
    }
}
