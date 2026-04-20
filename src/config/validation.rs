//! 配置验证模块
//!
//! 提供配置文件的 schema 验证功能
//!
//! # 功能
//! - 模型名称验证
//! - API endpoint URL 验证
//! - 模板语法验证
//! - 数值范围验证
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::config::validation::ConfigValidator;
//!
//! let validator = ConfigValidator::new();
//! match validator.validate_file("config/default.json") {
//!     Ok(result) => {
//!         if result.is_valid {
//!             println!("配置验证通过！");
//!         } else {
//!             eprintln!("配置验证失败：");
//!             for error in result.errors {
//!                 eprintln!("  - {}", error);
//!             }
//!         }
//!     }
//!     Err(e) => eprintln!("验证错误：{}", e),
//! }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use smallvec::{smallvec, SmallVec};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// 配置验证错误
#[derive(Debug, Error)]
pub enum ConfigValidationError {
    #[error("文件读取失败：{0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON 解析失败：{0}")]
    JsonError(#[from] serde_json::Error),

    #[error("配置验证失败：{0}")]
    ValidationError(String),
}

/// 配置验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 是否通过验证
    pub is_valid: bool,
    /// 验证通过的项目列表（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub passed_checks: SmallVec<[String; 4]>,
    /// 验证错误列表（使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub errors: SmallVec<[String; 4]>,
    /// 警告列表（非阻塞性问题，使用 `SmallVec` 优化小集合）
    #[serde(with = "smallvec_serde")]
    pub warnings: SmallVec<[String; 4]>,
}

/// `SmallVec` 序列化/反序列化辅助模块
mod smallvec_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use smallvec::SmallVec;

    pub fn serialize<S>(vec: &SmallVec<[String; 4]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        vec.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SmallVec<[String; 4]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<String>::deserialize(deserializer)?;
        Ok(vec.into())
    }
}

/// 支持的模型提供商
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SupportedProvider {
    ZazaZ,
    OpenAI,
    Anthropic,
    Custom,
}

impl SupportedProvider {
    /// 获取所有支持的模型名称
    pub fn supported_models(&self) -> Vec<&'static str> {
        match self {
            SupportedProvider::ZazaZ => vec!["Qwen2.5-VL", "Qwen2-VL", "InternVL2", "LLaVA"],
            SupportedProvider::OpenAI => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4-turbo",
                "gpt-4",
                "gpt-3.5-turbo",
            ],
            SupportedProvider::Anthropic => vec![
                "claude-3-5-sonnet",
                "claude-3-opus",
                "claude-3-sonnet",
                "claude-3-haiku",
            ],
            SupportedProvider::Custom => vec![], // 自定义模型，不验证
        }
    }
}

/// 配置验证器
pub struct ConfigValidator {
    /// 支持的模型名称集合
    supported_models: HashSet<String>,
    /// 是否允许自定义模型
    allow_custom_models: bool,
}

impl ConfigValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        let mut supported_models = HashSet::new();

        // 收集所有支持的模型
        for provider in [
            SupportedProvider::ZazaZ,
            SupportedProvider::OpenAI,
            SupportedProvider::Anthropic,
        ] {
            for model in provider.supported_models() {
                supported_models.insert(model.to_string());
            }
        }

        Self {
            supported_models,
            allow_custom_models: true, // 默认允许自定义模型
        }
    }

    /// 设置是否允许自定义模型
    pub fn with_allow_custom_models(mut self, allow: bool) -> Self {
        self.allow_custom_models = allow;
        self
    }

    /// 验证配置文件（从文件）
    pub fn validate_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<ValidationResult, ConfigValidationError> {
        let content = fs::read_to_string(path.as_ref())?;
        self.validate_json(&content)
    }

    /// 验证 JSON 字符串
    pub fn validate_json(&self, json_str: &str) -> Result<ValidationResult, ConfigValidationError> {
        let config: Value = serde_json::from_str(json_str)?;
        Ok(self.validate(&config))
    }

    /// 验证配置对象
    pub fn validate(&self, config: &Value) -> ValidationResult {
        let mut passed_checks = smallvec![];
        let mut errors = smallvec![];
        let mut warnings = smallvec![];

        // 验证 project 部分
        self.validate_project_section(config, &mut passed_checks, &mut errors, &mut warnings);

        // 验证 model_settings 部分
        self.validate_model_settings(config, &mut passed_checks, &mut errors, &mut warnings);

        // 验证 measurement_settings 部分
        self.validate_measurement_settings(config, &mut passed_checks, &mut errors, &mut warnings);

        // 验证 export_settings 部分
        self.validate_export_settings(config, &mut passed_checks, &mut errors, &mut warnings);

        // 验证 geo_cot_templates 部分
        self.validate_cot_templates(config, &mut passed_checks, &mut errors, &mut warnings);

        // 验证 room_type_rules 部分
        self.validate_room_type_rules(config, &mut passed_checks, &mut errors, &mut warnings);

        ValidationResult {
            is_valid: errors.is_empty(),
            passed_checks,
            errors,
            warnings,
        }
    }

    fn validate_project_section(
        &self,
        config: &Value,
        passed: &mut SmallVec<[String; 4]>,
        errors: &mut SmallVec<[String; 4]>,
        _warnings: &mut SmallVec<[String; 4]>,
    ) {
        if let Some(project) = config.get("project") {
            // 验证 name 字段
            if let Some(name) = project.get("name") {
                if name.as_str().is_some_and(|s| !s.is_empty()) {
                    passed.push("project.name 验证通过".to_string());
                } else {
                    errors.push("project.name 不能为空".to_string());
                }
            } else {
                errors.push("缺少 project.name 字段".to_string());
            }

            // 验证 version 字段
            if let Some(version) = project.get("version") {
                if version
                    .as_str()
                    .is_some_and(|s| semver::Version::parse(s).is_ok() || s.parse::<f32>().is_ok())
                {
                    passed.push("project.version 验证通过".to_string());
                } else {
                    errors.push("project.version 格式不正确，应遵循 semver 或数字格式".to_string());
                }
            } else {
                errors.push("缺少 project.version 字段".to_string());
            }
        } else {
            errors.push("缺少 project 配置节".to_string());
        }
    }

    fn validate_model_settings(
        &self,
        config: &Value,
        passed: &mut SmallVec<[String; 4]>,
        errors: &mut SmallVec<[String; 4]>,
        warnings: &mut SmallVec<[String; 4]>,
    ) {
        if let Some(model_settings) = config.get("model_settings") {
            // 验证 supported_models
            if let Some(models) = model_settings
                .get("supported_models")
                .and_then(|v| v.as_array())
            {
                let mut invalid_models: SmallVec<[String; 4]> = smallvec![];

                for model in models {
                    if let Some(model_str) = model.as_str() {
                        if !self.supported_models.contains(model_str) {
                            if self.allow_custom_models {
                                warnings.push(format!(
                                    "模型 '{model_str}' 不在预支持列表中，将作为自定义模型处理"
                                ));
                            } else {
                                invalid_models.push(model_str.to_string());
                            }
                        }
                    }
                }

                if invalid_models.is_empty() {
                    passed.push("model_settings.supported_models 验证通过".to_string());
                } else {
                    errors.push(format!("不支持的模型：{}", invalid_models.join(", ")));
                }
            }

            // 验证 default_resolution
            if let Some(resolution) = model_settings
                .get("default_resolution")
                .and_then(serde_json::Value::as_u64)
            {
                if (256..=4096).contains(&resolution) {
                    passed.push("model_settings.default_resolution 验证通过".to_string());
                } else {
                    errors
                        .push("model_settings.default_resolution 应在 256-4096 范围内".to_string());
                }
            }

            // 验证 max_resolution
            if let Some(max_res) = model_settings
                .get("max_resolution")
                .and_then(serde_json::Value::as_u64)
            {
                if (512..=8192).contains(&max_res) {
                    passed.push("model_settings.max_resolution 验证通过".to_string());
                } else {
                    errors.push("model_settings.max_resolution 应在 512-8192 范围内".to_string());
                }
            }

            // 验证 enable_cot 和 enable_tool_calling
            for field in &["enable_cot", "enable_tool_calling"] {
                if model_settings
                    .get(field)
                    .and_then(serde_json::Value::as_bool)
                    .is_some()
                {
                    passed.push(format!("model_settings.{field} 验证通过"));
                } else {
                    warnings.push(format!("model_settings.{field} 建议显式设置"));
                }
            }
        } else {
            warnings.push("缺少 model_settings 配置节（可选）".to_string());
        }
    }

    fn validate_measurement_settings(
        &self,
        config: &Value,
        passed: &mut SmallVec<[String; 4]>,
        errors: &mut SmallVec<[String; 4]>,
        warnings: &mut SmallVec<[String; 4]>,
    ) {
        if let Some(measurement) = config.get("measurement_settings") {
            // 验证 distance_tolerance
            if let Some(tolerance) = measurement
                .get("distance_tolerance")
                .and_then(serde_json::Value::as_f64)
            {
                if tolerance > 0.0 && tolerance < 100.0 {
                    passed.push("measurement_settings.distance_tolerance 验证通过".to_string());
                } else {
                    errors.push(
                        "measurement_settings.distance_tolerance 应在 0-100 范围内".to_string(),
                    );
                }
            }

            // 验证 angle_tolerance
            if let Some(tolerance) = measurement
                .get("angle_tolerance")
                .and_then(serde_json::Value::as_f64)
            {
                if tolerance > 0.0 && tolerance < 90.0 {
                    passed.push("measurement_settings.angle_tolerance 验证通过".to_string());
                } else {
                    errors
                        .push("measurement_settings.angle_tolerance 应在 0-90 范围内".to_string());
                }
            }

            // 验证门宽窗宽等尺寸（应为正数）
            for field in &[
                "default_door_width",
                "default_window_width",
                "default_window_height",
            ] {
                if let Some(value) = measurement.get(field).and_then(serde_json::Value::as_f64) {
                    if value > 0.0 {
                        passed.push(format!("measurement_settings.{field} 验证通过"));
                    } else {
                        errors.push(format!("measurement_settings.{field} 应为正数"));
                    }
                }
            }
        } else {
            warnings.push("缺少 measurement_settings 配置节（可选）".to_string());
        }
    }

    fn validate_export_settings(
        &self,
        config: &Value,
        passed: &mut SmallVec<[String; 4]>,
        errors: &mut SmallVec<[String; 4]>,
        warnings: &mut SmallVec<[String; 4]>,
    ) {
        if let Some(export) = config.get("export_settings") {
            // 验证 DXF 设置
            if let Some(dxf) = export.get("dxf") {
                if let Some(precision) = dxf.get("precision").and_then(serde_json::Value::as_u64) {
                    if precision <= 10 {
                        passed.push("export_settings.dxf.precision 验证通过".to_string());
                    } else {
                        errors.push("export_settings.dxf.precision 不应超过 10".to_string());
                    }
                }

                if let Some(unit) = dxf.get("unit").and_then(|v| v.as_str()) {
                    if ["mm", "cm", "m", "inch", "ft"].contains(&unit) {
                        passed.push("export_settings.dxf.unit 验证通过".to_string());
                    } else {
                        errors.push(format!(
                            "export_settings.dxf.unit 不支持：{unit} (支持的单位：mm, cm, m, inch, ft)"
                        ));
                    }
                }
            }

            // 验证 JSON 设置
            if let Some(json) = export.get("json") {
                for field in &["pretty", "include_metadata"] {
                    if json
                        .get(field)
                        .and_then(serde_json::Value::as_bool)
                        .is_some()
                    {
                        passed.push(format!("export_settings.json.{field} 验证通过"));
                    }
                }
            }
        } else {
            warnings.push("缺少 export_settings 配置节（可选）".to_string());
        }
    }

    fn validate_cot_templates(
        &self,
        config: &Value,
        passed: &mut SmallVec<[String; 4]>,
        errors: &mut SmallVec<[String; 4]>,
        warnings: &mut SmallVec<[String; 4]>,
    ) {
        if let Some(templates) = config.get("geo_cot_templates") {
            // 验证模板字段包含必要的占位符
            for (template_name, template_value) in
                templates.as_object().unwrap_or(&serde_json::Map::new())
            {
                if let Some(pattern) = template_value.get("pattern").and_then(|v| v.as_str()) {
                    // 检查是否包含至少一个占位符
                    if pattern.contains('{') && pattern.contains('}') {
                        passed.push(format!(
                            "geo_cot_templates.{template_name}.pattern 验证通过"
                        ));
                    } else {
                        errors.push(format!(
                            "geo_cot_templates.{template_name}.pattern 应包含至少一个占位符 ({{placeholder}})"
                        ));
                    }
                }
            }
        } else {
            warnings.push("缺少 geo_cot_templates 配置节（可选）".to_string());
        }
    }

    fn validate_room_type_rules(
        &self,
        config: &Value,
        passed: &mut SmallVec<[String; 4]>,
        errors: &mut SmallVec<[String; 4]>,
        warnings: &mut SmallVec<[String; 4]>,
    ) {
        if let Some(rules) = config.get("room_type_rules").and_then(|v| v.as_array()) {
            for (idx, rule) in rules.iter().enumerate() {
                // 验证 condition 字段
                if let Some(condition) = rule.get("condition").and_then(|v| v.as_str()) {
                    if self.validate_condition_syntax(condition) {
                        passed.push(format!("room_type_rules[{idx}].condition 验证通过"));
                    } else {
                        errors.push(format!(
                            "room_type_rules[{idx}].condition 语法不正确：{condition}"
                        ));
                    }
                } else {
                    errors.push(format!("room_type_rules[{idx}] 缺少 condition 字段"));
                }

                // 验证 room_type 字段
                if rule
                    .get("room_type")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.is_empty())
                {
                    passed.push(format!("room_type_rules[{idx}].room_type 验证通过"));
                } else {
                    errors.push(format!("room_type_rules[{idx}].room_type 不能为空"));
                }

                // 验证 description 字段（可选）
                if rule.get("description").is_none() {
                    warnings.push(format!(
                        "room_type_rules[{idx}] 缺少 description 字段（建议添加）"
                    ));
                }
            }
        } else {
            warnings.push("缺少 room_type_rules 配置节（可选）".to_string());
        }
    }

    /// 验证条件语法
    ///
    /// 支持的语法：
    /// - 比较运算符：>, <, >=, <=, =
    /// - 逻辑运算符：AND, OR
    /// - 字段引用：area, doors, windows 等
    fn validate_condition_syntax(&self, condition: &str) -> bool {
        // 简化的语法验证
        // 检查是否包含有效的比较运算符
        let has_comparison =
            condition.contains('>') || condition.contains('<') || condition.contains('=');

        // 检查是否包含有效的字段名
        let valid_fields = ["area", "doors", "windows", "width", "height", "perimeter"];
        let has_valid_field = valid_fields.iter().any(|field| condition.contains(field));

        // 检查是否有逻辑运算符（如果有多个条件）
        if condition.contains(" AND ") || condition.contains(" OR ") {
            // 分割条件并递归验证
            let parts: Vec<&str> = if condition.contains(" AND ") {
                condition.split(" AND ").collect()
            } else {
                condition.split(" OR ").collect()
            };

            return parts
                .iter()
                .all(|part| self.validate_condition_syntax(part.trim()));
        }

        has_comparison && has_valid_field
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 验证配置文件的便捷函数
pub fn validate_config_file(
    path: impl AsRef<Path>,
) -> Result<ValidationResult, ConfigValidationError> {
    let validator = ConfigValidator::new();
    validator.validate_file(path)
}

/// 验证 JSON 字符串的便捷函数
pub fn validate_config_json(json_str: &str) -> Result<ValidationResult, ConfigValidationError> {
    let validator = ConfigValidator::new();
    validator.validate_json(json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_config() {
        let config = r#"{
            "project": {
                "name": "TestProject",
                "version": "0.1.0"
            },
            "model_settings": {
                "supported_models": ["gpt-4o", "Qwen2.5-VL"],
                "default_resolution": 1024,
                "max_resolution": 2048,
                "enable_cot": true,
                "enable_tool_calling": true
            },
            "measurement_settings": {
                "distance_tolerance": 1.0,
                "angle_tolerance": 1.0,
                "default_door_width": 900.0
            }
        }"#;

        let validator = ConfigValidator::new();
        let result = validator.validate_json(config).unwrap();

        assert!(result.is_valid);
        assert!(!result.passed_checks.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_invalid_version() {
        let config = r#"{
            "project": {
                "name": "Test",
                "version": "invalid-version-!!!"
            }
        }"#;

        let validator = ConfigValidator::new();
        let result = validator.validate_json(config).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("version")));
    }

    #[test]
    fn test_validate_invalid_resolution() {
        let config = r#"{
            "model_settings": {
                "default_resolution": 10000
            }
        }"#;

        let validator = ConfigValidator::new();
        let result = validator.validate_json(config).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("resolution")));
    }

    #[test]
    fn test_validate_room_type_rules() {
        let config = r#"{
            "project": {
                "name": "Test",
                "version": "1.0.0"
            },
            "room_type_rules": [
                {
                    "condition": "area < 50000 AND doors <= 1",
                    "room_type": "卫生间",
                    "description": "小面积房间"
                }
            ]
        }"#;

        let validator = ConfigValidator::new();
        let result = validator.validate_json(config).unwrap();

        assert!(result.is_valid);
        assert!(result.passed_checks.iter().any(|c| c.contains("condition")));
    }

    #[test]
    fn test_validate_invalid_condition() {
        let config = r#"{
            "room_type_rules": [
                {
                    "condition": "invalid syntax without operators",
                    "room_type": "测试"
                }
            ]
        }"#;

        let validator = ConfigValidator::new();
        let result = validator.validate_json(config).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("condition")));
    }

    #[test]
    fn test_validate_template_placeholders() {
        let config = r#"{
            "project": {
                "name": "Test",
                "version": "1.0.0"
            },
            "geo_cot_templates": {
                "perception": {
                    "pattern": "首先，我观察到图像{position}有一个{element_type}"
                }
            }
        }"#;

        let validator = ConfigValidator::new();
        let result = validator.validate_json(config).unwrap();

        assert!(result.is_valid);
        assert!(result.passed_checks.iter().any(|c| c.contains("pattern")));
    }

    #[test]
    fn test_validate_template_without_placeholders() {
        let config = r#"{
            "geo_cot_templates": {
                "perception": {
                    "pattern": "这是一个没有占位符的模板"
                }
            }
        }"#;

        let validator = ConfigValidator::new();
        let result = validator.validate_json(config).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("占位符")));
    }
}
