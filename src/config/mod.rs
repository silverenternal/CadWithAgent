//! 配置模块
//!
//! 提供配置文件的加载、验证和管理功能
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::config::validate_config_file;
//!
//! // 验证配置文件
//! match validate_config_file("config/default.json") {
//!     Ok(result) => {
//!         if result.is_valid {
//!             println!("配置验证通过！");
//!             println!("通过检查：{} 项", result.passed_checks.len());
//!         } else {
//!             eprintln!("配置验证失败：");
//!             for error in &result.errors {
//!                 eprintln!("  - {}", error);
//!             }
//!         }
//!     }
//!     Err(e) => eprintln!("验证错误：{}", e),
//! }
//! ```

pub mod validation;

pub use validation::*;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// CadAgent 配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadConfig {
    /// 项目信息
    pub project: ProjectConfig,
    /// Geo-CoT 模板配置
    #[serde(default)]
    pub geo_cot_templates: GeoCotTemplates,
    /// 房间类型规则
    #[serde(default)]
    pub room_type_rules: Vec<RoomTypeRule>,
    /// 测量设置
    #[serde(default)]
    pub measurement_settings: MeasurementSettings,
    /// 导出设置
    #[serde(default)]
    pub export_settings: ExportSettings,
    /// 模型设置
    #[serde(default)]
    pub model_settings: ModelSettings,
}

/// 项目配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// 项目名称
    pub name: String,
    /// 版本号
    pub version: String,
}

/// Geo-CoT 模板配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoCotTemplates {
    /// 感知模板
    #[serde(default)]
    pub perception: TemplateConfig,
    /// 推理模板
    #[serde(default)]
    pub reasoning: TemplateConfig,
    /// 总结模板
    #[serde(default)]
    pub summary: TemplateConfig,
}

/// 模板配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateConfig {
    /// 模板模式字符串
    #[serde(default)]
    pub pattern: String,
}

/// 房间类型规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTypeRule {
    /// 条件表达式
    pub condition: String,
    /// 房间类型
    pub room_type: String,
    /// 描述（可选）
    #[serde(default)]
    pub description: String,
}

/// 测量设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementSettings {
    /// 距离容差
    #[serde(default = "default_distance_tolerance")]
    pub distance_tolerance: f64,
    /// 角度容差（度）
    #[serde(default = "default_angle_tolerance")]
    pub angle_tolerance: f64,
    /// 默认门宽（mm）
    #[serde(default = "default_door_width")]
    pub default_door_width: f64,
    /// 默认窗宽（mm）
    #[serde(default = "default_window_width")]
    pub default_window_width: f64,
    /// 默认窗高（mm）
    #[serde(default = "default_window_height")]
    pub default_window_height: f64,
}

fn default_distance_tolerance() -> f64 { 1.0 }
fn default_angle_tolerance() -> f64 { 1.0 }
fn default_door_width() -> f64 { 900.0 }
fn default_window_width() -> f64 { 1200.0 }
fn default_window_height() -> f64 { 1500.0 }

impl Default for MeasurementSettings {
    fn default() -> Self {
        Self {
            distance_tolerance: default_distance_tolerance(),
            angle_tolerance: default_angle_tolerance(),
            default_door_width: default_door_width(),
            default_window_width: default_window_width(),
            default_window_height: default_window_height(),
        }
    }
}

/// 导出设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportSettings {
    /// DXF 导出设置
    #[serde(default)]
    pub dxf: DxfExportSettings,
    /// JSON 导出设置
    #[serde(default)]
    pub json: JsonExportSettings,
}

/// DXF 导出设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfExportSettings {
    /// 精度（小数位数）
    #[serde(default = "default_dxf_precision")]
    pub precision: u32,
    /// 单位
    #[serde(default = "default_dxf_unit")]
    pub unit: String,
}

fn default_dxf_precision() -> u32 { 4 }
fn default_dxf_unit() -> String { "mm".to_string() }

impl Default for DxfExportSettings {
    fn default() -> Self {
        Self {
            precision: default_dxf_precision(),
            unit: default_dxf_unit(),
        }
    }
}

/// JSON 导出设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsonExportSettings {
    /// 是否美化输出
    #[serde(default = "default_json_pretty")]
    pub pretty: bool,
    /// 是否包含元数据
    #[serde(default = "default_json_include_metadata")]
    pub include_metadata: bool,
}

fn default_json_pretty() -> bool { true }
fn default_json_include_metadata() -> bool { true }

/// 模型设置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelSettings {
    /// 支持的模型列表
    #[serde(default)]
    pub supported_models: Vec<String>,
    /// 默认分辨率
    #[serde(default = "default_model_resolution")]
    pub default_resolution: u32,
    /// 最大分辨率
    #[serde(default = "default_max_resolution")]
    pub max_resolution: u32,
    /// 是否启用 CoT
    #[serde(default = "default_enable_cot")]
    pub enable_cot: bool,
    /// 是否启用工具调用
    #[serde(default = "default_enable_tool_calling")]
    pub enable_tool_calling: bool,
}

fn default_model_resolution() -> u32 { 1024 }
fn default_max_resolution() -> u32 { 2048 }
fn default_enable_cot() -> bool { true }
fn default_enable_tool_calling() -> bool { true }

impl CadConfig {
    /// 从文件加载配置
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::IoError(format!("读取配置文件失败：{}", e)))?;

        let config: CadConfig = serde_json::from_str(&content)
            .map_err(|e| ConfigError::JsonError(format!("解析配置文件失败：{}", e)))?;

        Ok(config)
    }

    /// 从 JSON 字符串加载配置
    pub fn from_json(json_str: &str) -> Result<Self, ConfigError> {
        let config: CadConfig = serde_json::from_str(json_str)
            .map_err(|e| ConfigError::JsonError(format!("解析 JSON 失败：{}", e)))?;

        Ok(config)
    }

    /// 加载配置并验证
    pub fn load_and_validate(path: impl AsRef<Path>) -> Result<(Self, ValidationResult), ConfigError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| ConfigError::IoError(format!("读取配置文件失败：{}", e)))?;

        // 先验证
        let validator = ConfigValidator::new();
        let validation_result = validator.validate_json(&content)
            .map_err(|e| ConfigError::ValidationError(format!("验证失败：{}", e)))?;

        if !validation_result.is_valid {
            return Err(ConfigError::ValidationError(format!(
                "配置验证失败：{}",
                validation_result.errors.join("; ")
            )));
        }

        // 再解析
        let config: CadConfig = serde_json::from_str(&content)
            .map_err(|e| ConfigError::JsonError(format!("解析配置文件失败：{}", e)))?;

        Ok((config, validation_result))
    }

    /// 验证配置
    pub fn validate(&self) -> ValidationResult {
        let validator = ConfigValidator::new();
        let config_json = serde_json::to_value(self).unwrap_or_default();
        validator.validate(&config_json)
    }
}

/// 配置错误
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO 错误：{0}")]
    IoError(String),

    #[error("JSON 解析错误：{0}")]
    JsonError(String),

    #[error("验证错误：{0}")]
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_valid_config() {
        let config_json = r#"{
            "project": {
                "name": "TestProject",
                "version": "0.1.0"
            },
            "measurement_settings": {
                "distance_tolerance": 1.0,
                "angle_tolerance": 1.0,
                "default_door_width": 900.0,
                "default_window_width": 1200.0,
                "default_window_height": 1500.0
            }
        }"#;

        let config = CadConfig::from_json(config_json).unwrap();
        assert_eq!(config.project.name, "TestProject");
        assert_eq!(config.project.version, "0.1.0");
    }

    #[test]
    fn test_load_and_validate() {
        let config_json = r#"{
            "project": {
                "name": "TestProject",
                "version": "0.1.0"
            },
            "geo_cot_templates": {
                "perception": {
                    "pattern": "测试{placeholder}"
                },
                "reasoning": {
                    "pattern": "推理{step}"
                },
                "summary": {
                    "pattern": "总结{conclusion}"
                }
            },
            "model_settings": {
                "supported_models": ["gpt-4o"],
                "default_resolution": 1024,
                "max_resolution": 2048,
                "enable_cot": true,
                "enable_tool_calling": true
            }
        }"#;

        let result = CadConfig::from_json(config_json);
        assert!(result.is_ok());

        let config = result.unwrap();
        let validation = config.validate();
        
        // 验证应该通过
        assert!(validation.is_valid, "验证错误：{:?}", validation.errors);
    }

    #[test]
    fn test_default_values() {
        let config_json = r#"{
            "project": {
                "name": "Test",
                "version": "1.0.0"
            },
            "geo_cot_templates": {
                "perception": {
                    "pattern": "测试{placeholder}"
                }
            },
            "model_settings": {
                "supported_models": ["gpt-4o"],
                "default_resolution": 1024,
                "max_resolution": 2048,
                "enable_cot": true,
                "enable_tool_calling": true
            }
        }"#;

        let config = CadConfig::from_json(config_json).unwrap();

        // 检查默认值
        assert_eq!(config.measurement_settings.distance_tolerance, 1.0);
        assert_eq!(config.measurement_settings.default_door_width, 900.0);
        assert_eq!(config.export_settings.dxf.precision, 4);
        assert_eq!(config.model_settings.default_resolution, 1024);
    }
}
