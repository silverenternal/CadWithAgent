//! 统一错误处理
//!
//! 提供 `CadAgent` 专用的错误类型，实现统一的错误处理机制
//!
//! # 错误类型设计原则
//!
//! 1. 所有错误都实现 `std::error::Error` trait，支持 `source()` 追溯错误链
//! 2. 使用 `thiserror` 自动生成 `Display` 和 `Error` 实现
//! 3. 错误类型变体包含足够的上下文信息
//! 4. 支持从底层错误自动转换

use serde::{Deserialize, Serialize};
use std::fmt;

// ==================== 共享几何配置 ====================

/// 共享几何容差配置
///
/// 用于统一各模块中的角度容差和距离容差配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryToleranceConfig {
    /// 角度容差（弧度）
    pub angle_tolerance: f64,
    /// 距离容差
    pub distance_tolerance: f64,
}

impl Default for GeometryToleranceConfig {
    fn default() -> Self {
        Self {
            angle_tolerance: 0.01, // ~0.57 度
            distance_tolerance: 0.01,
        }
    }
}

impl GeometryToleranceConfig {
    /// 创建新的容差配置
    pub fn new(angle_tolerance: f64, distance_tolerance: f64) -> Self {
        Self {
            angle_tolerance,
            distance_tolerance,
        }
    }

    /// 验证配置参数的合理性
    ///
    /// # Errors
    /// 如果配置参数无效，返回 `CadAgentError::Config`
    pub fn validate(&self) -> CadAgentResult<()> {
        // 验证角度容差：必须为正且不超过 90 度
        if self.angle_tolerance <= 0.0 {
            return Err(CadAgentError::config_invalid(
                "angle_tolerance",
                self.angle_tolerance,
                "角度容差必须为正数",
                Some("建议值：0.01（约 0.57 度）"),
            ));
        }
        if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            return Err(CadAgentError::config_invalid(
                "angle_tolerance",
                self.angle_tolerance,
                "角度容差过大，最大允许 90 度（π/2）".to_string(),
                Some("建议值：0.01"),
            ));
        }

        // 验证距离容差：必须为非负
        if self.distance_tolerance < 0.0 {
            return Err(CadAgentError::config_invalid(
                "distance_tolerance",
                self.distance_tolerance,
                "距离容差必须为非负数",
                None,
            ));
        }

        Ok(())
    }

    /// 验证并自动修正不合理的配置
    pub fn validate_or_fix(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let default = GeometryToleranceConfig::default();

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

        warnings
    }
}

// ==================== 统一几何配置 ====================

/// 统一几何配置
///
/// 提取各模块配置中的公共字段，减少重复
///
/// # 使用示例
///
/// ```rust
/// use cadagent::error::GeometryConfig;
///
/// let config = GeometryConfig::default();
/// assert_eq!(config.angle_tolerance, 0.01);
/// assert_eq!(config.distance_tolerance, 0.01);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryConfig {
    /// 角度容差（弧度）
    pub angle_tolerance: f64,
    /// 距离容差
    pub distance_tolerance: f64,
    /// 最小置信度阈值
    pub min_confidence: f64,
    /// 坐标归一化范围 [min, max]
    pub normalize_range: [f64; 2],
    /// 是否启用坐标归一化
    pub enable_normalization: bool,
}

impl Default for GeometryConfig {
    fn default() -> Self {
        Self {
            angle_tolerance: 0.01,
            distance_tolerance: 0.01,
            min_confidence: 0.8,
            normalize_range: [0.0, 100.0],
            enable_normalization: true,
        }
    }
}

impl GeometryConfig {
    /// 创建新的几何配置
    pub fn new(
        angle_tolerance: f64,
        distance_tolerance: f64,
        min_confidence: f64,
        normalize_range: [f64; 2],
    ) -> Self {
        Self {
            angle_tolerance,
            distance_tolerance,
            min_confidence,
            normalize_range,
            enable_normalization: true,
        }
    }

    /// 从容差配置创建
    pub fn from_tolerance(tolerance: GeometryToleranceConfig) -> Self {
        Self {
            angle_tolerance: tolerance.angle_tolerance,
            distance_tolerance: tolerance.distance_tolerance,
            ..Default::default()
        }
    }

    /// 验证配置参数的合理性
    ///
    /// # Errors
    /// 如果配置参数无效，返回 `CadAgentError::Config`
    pub fn validate(&self) -> CadAgentResult<()> {
        // 验证角度容差
        if self.angle_tolerance <= 0.0 {
            return Err(CadAgentError::config_invalid(
                "angle_tolerance",
                self.angle_tolerance,
                "角度容差必须为正数",
                Some("建议值：0.01（约 0.57 度）"),
            ));
        }
        if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            return Err(CadAgentError::config_invalid(
                "angle_tolerance",
                self.angle_tolerance,
                format!(
                    "角度容差过大（{} 弧度 ≈ {:.2} 度）",
                    self.angle_tolerance,
                    self.angle_tolerance.to_degrees()
                ),
                Some("最大允许 90 度（π/2）"),
            ));
        }

        // 验证距离容差
        if self.distance_tolerance < 0.0 {
            return Err(CadAgentError::config_invalid(
                "distance_tolerance",
                self.distance_tolerance,
                "距离容差必须为非负数",
                None,
            ));
        }

        // 验证置信度
        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            return Err(CadAgentError::config_invalid(
                "min_confidence",
                self.min_confidence,
                "最小置信度必须在 0 到 1 之间",
                None,
            ));
        }

        // 验证归一化范围
        if self.normalize_range[0] >= self.normalize_range[1] {
            return Err(CadAgentError::config_invalid(
                "normalize_range",
                self.normalize_range[0],
                format!(
                    "归一化范围无效：[{}, {}]",
                    self.normalize_range[0], self.normalize_range[1]
                ),
                Some("最小值必须小于最大值，建议值：[0.0, 100.0]"),
            ));
        }

        Ok(())
    }

    /// 验证并自动修正不合理的配置
    pub fn validate_or_fix(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let default = GeometryConfig::default();

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

        warnings
    }

    /// 转换为容差配置
    pub fn to_tolerance_config(&self) -> GeometryToleranceConfig {
        GeometryToleranceConfig {
            angle_tolerance: self.angle_tolerance,
            distance_tolerance: self.distance_tolerance,
        }
    }
}

// ==================== 几何错误类型 ====================

/// 几何错误详情
///
/// 用于提供更精确的几何错误信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryErrorDetail {
    /// 几何对象类型
    pub object_type: String,
    /// 错误字段
    pub field: String,
    /// 错误值
    pub value: f64,
    /// 错误描述
    pub message: String,
}

impl fmt::Display for GeometryErrorDetail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{} = {}: {}",
            self.object_type, self.field, self.value, self.message
        )
    }
}

// ==================== 错误类型定义 ====================

/// `CadAgent` 统一错误类型
///
/// # 设计原则
///
/// 1. 所有变体都包含人类可读的错误消息
/// 2. 部分变体支持携带底层错误源（通过 `source` 字段）
/// 3. 配置错误提供详细的参数信息
///
/// # 使用示例
///
/// ```rust
/// use cadagent::error::{CadAgentError, CadAgentResult};
///
/// fn validate_positive(value: f64, name: &str) -> CadAgentResult<()> {
///     if value <= 0.0 {
///         Err(CadAgentError::config_invalid(
///             name,
///             value,
///             "必须为正数",
///             None,
///         ))
///     } else {
///         Ok(())
///     }
/// }
/// ```
#[derive(Debug, thiserror::Error)]
pub enum CadAgentError {
    /// IO 错误
    #[error("IO 错误：{message}")]
    Io {
        message: String,
        #[source]
        source: Option<std::io::Error>,
    },

    /// JSON 解析/序列化错误
    #[error("JSON 错误：{message}")]
    Json {
        message: String,
        #[source]
        source: Option<serde_json::Error>,
    },

    /// 几何错误（无效的几何数据）
    #[error("几何错误：{detail}")]
    Geometry { detail: GeometryErrorDetail },

    /// 解析错误（SVG/DXF 解析失败）
    #[error("解析错误：{message}")]
    Parse { message: String, format: String },

    /// 配置错误（无效的配置参数）
    #[error("配置错误：{parameter} = {value}: {message}")]
    Config {
        parameter: String,
        value: f64,
        message: String,
        suggestion: Option<String>,
    },

    /// 工具执行错误
    #[error("工具执行错误：{message}")]
    Tool {
        message: String,
        tool_name: Option<String>,
    },

    /// API 调用错误（VLM/LLM API）
    #[error("API 错误：{message}")]
    Api {
        message: String,
        source_error: Option<String>,
    },

    /// 文件未找到
    #[error("文件未找到：{path}")]
    FileNotFound { path: String },

    /// 不支持的格式
    #[error("不支持的格式：{format}")]
    UnsupportedFormat { format: String },

    /// 验证错误
    #[error("验证错误：{message}")]
    Validation {
        message: String,
        failures: Vec<String>,
    },

    /// 资源限制错误（文件大小、内存等）
    #[error("资源限制：{message}")]
    ResourceLimit {
        message: String,
        limit: Option<u64>,
        actual: Option<u64>,
    },

    /// 内部错误（通用错误类型）
    #[error("内部错误：{message}")]
    Internal { message: String },

    /// 其他错误
    #[error("错误：{message}")]
    Other { message: String },
}

impl CadAgentError {
    // ==================== 构造辅助函数 ====================

    /// 创建 IO 错误
    pub fn io(message: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            message: message.into(),
            source: Some(source),
        }
    }

    /// 创建 JSON 错误
    pub fn json(message: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Json {
            message: message.into(),
            source: Some(source),
        }
    }

    /// 创建几何错误
    pub fn geometry(
        object_type: impl Into<String>,
        field: impl Into<String>,
        value: f64,
        message: impl Into<String>,
    ) -> Self {
        Self::Geometry {
            detail: GeometryErrorDetail {
                object_type: object_type.into(),
                field: field.into(),
                value,
                message: message.into(),
            },
        }
    }

    /// 创建解析错误
    pub fn parse(format: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            message: message.into(),
            format: format.into(),
        }
    }

    /// 创建配置错误（带详细信息）
    pub fn config_invalid(
        parameter: impl Into<String>,
        value: f64,
        message: impl Into<String>,
        suggestion: Option<&str>,
    ) -> Self {
        Self::Config {
            parameter: parameter.into(),
            value,
            message: message.into(),
            suggestion: suggestion.map(String::from),
        }
    }

    /// 创建工具执行错误
    pub fn tool(message: impl Into<String>, tool_name: Option<&str>) -> Self {
        Self::Tool {
            message: message.into(),
            tool_name: tool_name.map(String::from),
        }
    }

    /// 创建 API 错误
    pub fn api(message: impl Into<String>, source: Option<String>) -> Self {
        Self::Api {
            message: message.into(),
            source_error: source,
        }
    }

    /// 创建文件未找到错误
    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// 创建不支持的格式错误
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
        }
    }

    /// 创建验证错误
    pub fn validation(message: impl Into<String>, failures: Vec<String>) -> Self {
        Self::Validation {
            message: message.into(),
            failures,
        }
    }

    /// 创建资源限制错误
    pub fn resource_limit(
        message: impl Into<String>,
        limit: Option<u64>,
        actual: Option<u64>,
    ) -> Self {
        Self::ResourceLimit {
            message: message.into(),
            limit,
            actual,
        }
    }

    /// 创建内部错误
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// 获取错误类别（用于日志分类）
    pub fn category(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Json { .. } => "json",
            Self::Geometry { .. } => "geometry",
            Self::Parse { .. } => "parse",
            Self::Config { .. } => "config",
            Self::Tool { .. } => "tool",
            Self::Api { .. } => "api",
            Self::FileNotFound { .. } => "file_not_found",
            Self::UnsupportedFormat { .. } => "unsupported_format",
            Self::Validation { .. } => "validation",
            Self::ResourceLimit { .. } => "resource_limit",
            Self::Internal { .. } => "internal",
            Self::Other { .. } => "other",
        }
    }

    /// 获取错误码（用于程序化处理）
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "CADAGENT_IO_ERROR",
            Self::Json { .. } => "CADAGENT_JSON_ERROR",
            Self::Geometry { .. } => "CADAGENT_GEOMETRY_ERROR",
            Self::Parse { .. } => "CADAGENT_PARSE_ERROR",
            Self::Config { .. } => "CADAGENT_CONFIG_ERROR",
            Self::Tool { .. } => "CADAGENT_TOOL_ERROR",
            Self::Api { .. } => "CADAGENT_API_ERROR",
            Self::FileNotFound { .. } => "CADAGENT_FILE_NOT_FOUND",
            Self::UnsupportedFormat { .. } => "CADAGENT_UNSUPPORTED_FORMAT",
            Self::Validation { .. } => "CADAGENT_VALIDATION_ERROR",
            Self::ResourceLimit { .. } => "CADAGENT_RESOURCE_LIMIT",
            Self::Internal { .. } => "CADAGENT_INTERNAL_ERROR",
            Self::Other { .. } => "CADAGENT_OTHER",
        }
    }
}

// ==================== From 转换 trait ====================

impl From<std::io::Error> for CadAgentError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            source: Some(err),
        }
    }
}

impl From<serde_json::Error> for CadAgentError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json {
            message: err.to_string(),
            source: Some(err),
        }
    }
}

impl From<String> for CadAgentError {
    fn from(msg: String) -> Self {
        Self::Other { message: msg }
    }
}

impl From<&str> for CadAgentError {
    fn from(msg: &str) -> Self {
        Self::Other {
            message: msg.to_string(),
        }
    }
}

// ==================== VLM 错误转换 ====================

impl From<crate::bridge::vlm_client::VlmError> for CadAgentError {
    fn from(err: crate::bridge::vlm_client::VlmError) -> Self {
        Self::Api {
            message: format!("VLM 错误：{err}"),
            source_error: Some(err.to_string()),
        }
    }
}

/// 结果类型别名
pub type CadAgentResult<T> = Result<T, CadAgentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CadAgentError::geometry("Line", "length", 0.0, "长度不能为 0");
        assert!(err.to_string().contains("几何错误"));
        assert!(err.to_string().contains("Line.length"));

        let err = CadAgentError::file_not_found("/path/to/file.svg");
        assert_eq!(format!("{}", err), "文件未找到：/path/to/file.svg");

        let err = CadAgentError::config_invalid(
            "angle_tolerance",
            -0.01,
            "必须为正数",
            Some("建议值：0.01"),
        );
        assert!(err.to_string().contains("配置错误"));
        assert!(err.to_string().contains("angle_tolerance"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: CadAgentError = io_err.into();
        assert!(matches!(err, CadAgentError::Io { .. }));
        assert_eq!(err.category(), "io");
    }

    #[test]
    fn test_error_from_json() {
        let json_err = serde_json::from_str::<String>("invalid json").unwrap_err();
        let err: CadAgentError = json_err.into();
        assert!(matches!(err, CadAgentError::Json { .. }));
        assert_eq!(err.category(), "json");
    }

    #[test]
    fn test_error_source() {
        use std::error::Error;

        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = CadAgentError::io("无法读取文件", io_err);

        // 验证 source() 方法可用
        assert!(err.source().is_some());
    }

    #[test]
    fn test_result_alias() {
        let result: CadAgentResult<()> = Ok(());
        assert!(result.is_ok());

        let result: CadAgentResult<()> = Err(CadAgentError::Other {
            message: "test error".to_string(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_error_code() {
        let err = CadAgentError::geometry("Circle", "radius", 0.0, "半径必须为正数");
        assert_eq!(err.code(), "CADAGENT_GEOMETRY_ERROR");

        let err = CadAgentError::config_invalid("min_confidence", 1.5, "超出范围", None);
        assert_eq!(err.code(), "CADAGENT_CONFIG_ERROR");
    }
}
