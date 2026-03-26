//! 统一错误处理
//!
//! 提供 CadAgent 专用的错误类型，实现统一的错误处理机制

use serde::{Deserialize, Serialize};
use std::fmt;
use std::io;

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
        // 验证角度容差
        if self.angle_tolerance <= 0.0 {
            return Err(CadAgentError::Config(format!(
                "角度容差必须为正数，当前值：{}。建议值：0.01（约 0.57 度）",
                self.angle_tolerance
            )));
        }
        if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            return Err(CadAgentError::Config(format!(
                "角度容差过大（{} 弧度 ≈ {:.2} 度），最大允许 90 度（π/2）。建议值：0.01",
                self.angle_tolerance,
                self.angle_tolerance.to_degrees()
            )));
        }

        // 验证距离容差
        if self.distance_tolerance < 0.0 {
            return Err(CadAgentError::Config(format!(
                "距离容差必须为非负数，当前值：{}",
                self.distance_tolerance
            )));
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
            return Err(CadAgentError::Config(format!(
                "角度容差必须为正数，当前值：{}。建议值：0.01（约 0.57 度）",
                self.angle_tolerance
            )));
        }
        if self.angle_tolerance > std::f64::consts::FRAC_PI_2 {
            return Err(CadAgentError::Config(format!(
                "角度容差过大（{} 弧度 ≈ {:.2} 度），最大允许 90 度（π/2）。建议值：0.01",
                self.angle_tolerance,
                self.angle_tolerance.to_degrees()
            )));
        }

        // 验证距离容差
        if self.distance_tolerance < 0.0 {
            return Err(CadAgentError::Config(format!(
                "距离容差必须为非负数，当前值：{}",
                self.distance_tolerance
            )));
        }

        // 验证置信度
        if self.min_confidence < 0.0 || self.min_confidence > 1.0 {
            return Err(CadAgentError::Config(format!(
                "最小置信度必须在 0 到 1 之间，当前值：{}",
                self.min_confidence
            )));
        }

        // 验证归一化范围
        if self.normalize_range[0] >= self.normalize_range[1] {
            return Err(CadAgentError::Config(format!(
                "归一化范围无效：[{}, {}]。最小值必须小于最大值。建议值：[0.0, 100.0]",
                self.normalize_range[0], self.normalize_range[1]
            )));
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

// ==================== 错误类型定义 ====================

/// CadAgent 统一错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CadAgentError {
    /// IO 错误
    Io(String),

    /// JSON 解析/序列化错误
    Json(String),

    /// 几何错误（无效的几何数据）
    Geometry(String),

    /// 解析错误（SVG/DXF 解析失败）
    Parse(String),

    /// 配置错误（无效的配置参数）
    Config(String),

    /// 工具执行错误
    Tool(String),

    /// API 调用错误（VLM/LLM API）
    Api(String),

    /// 文件未找到
    FileNotFound(String),

    /// 不支持的格式
    UnsupportedFormat(String),

    /// 验证错误
    Validation(String),

    /// 其他错误
    Other(String),
}

impl fmt::Display for CadAgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CadAgentError::Io(err) => write!(f, "IO 错误：{}", err),
            CadAgentError::Json(err) => write!(f, "JSON 错误：{}", err),
            CadAgentError::Geometry(msg) => write!(f, "几何错误：{}", msg),
            CadAgentError::Parse(msg) => write!(f, "解析错误：{}", msg),
            CadAgentError::Config(msg) => write!(f, "配置错误：{}", msg),
            CadAgentError::Tool(msg) => write!(f, "工具执行错误：{}", msg),
            CadAgentError::Api(msg) => write!(f, "API 错误：{}", msg),
            CadAgentError::FileNotFound(path) => write!(f, "文件未找到：{}", path),
            CadAgentError::UnsupportedFormat(fmt) => write!(f, "不支持的格式：{}", fmt),
            CadAgentError::Validation(msg) => write!(f, "验证错误：{}", msg),
            CadAgentError::Other(msg) => write!(f, "错误：{}", msg),
        }
    }
}

impl std::error::Error for CadAgentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

// 实现 From 转换 trait
impl From<io::Error> for CadAgentError {
    fn from(err: io::Error) -> Self {
        CadAgentError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for CadAgentError {
    fn from(err: serde_json::Error) -> Self {
        CadAgentError::Json(err.to_string())
    }
}

impl From<String> for CadAgentError {
    fn from(msg: String) -> Self {
        CadAgentError::Other(msg)
    }
}

impl From<&str> for CadAgentError {
    fn from(msg: &str) -> Self {
        CadAgentError::Other(msg.to_string())
    }
}

/// 结果类型别名
pub type CadAgentResult<T> = Result<T, CadAgentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CadAgentError::Geometry("无效的线段".to_string());
        assert_eq!(format!("{}", err), "几何错误：无效的线段");

        let err = CadAgentError::FileNotFound("/path/to/file.svg".to_string());
        assert_eq!(format!("{}", err), "文件未找到：/path/to/file.svg");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err: CadAgentError = io_err.into();
        assert!(matches!(err, CadAgentError::Io(_)));
    }

    #[test]
    fn test_error_from_json() {
        let json_err = serde_json::from_str::<String>("invalid json").unwrap_err();
        let err: CadAgentError = json_err.into();
        assert!(matches!(err, CadAgentError::Json(_)));
    }

    #[test]
    fn test_result_alias() {
        let result: CadAgentResult<()> = Ok(());
        assert!(result.is_ok());

        let result: CadAgentResult<()> = Err(CadAgentError::Other("test error".to_string()));
        assert!(result.is_err());
    }
}
