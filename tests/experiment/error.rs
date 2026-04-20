//! 实验错误处理模块
//!
//! 提供统一的错误类型、断言工具和错误恢复建议。
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::experiment::error::{ExperimentError, ExperimentResult, assert_within_tolerance};
//!
//! fn run_test() -> ExperimentResult<()> {
//!     let expected = 10.0;
//!     let actual = 10.0001;
//!     let tolerance = 1e-3;
//!     
//!     assert_within_tolerance!(actual, expected, tolerance, "测量值应在容差范围内")?;
//!     
//!     Ok(())
//! }
//! ```

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

/// 实验错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExperimentError {
    /// 断言失败
    AssertionFailed {
        /// 错误消息
        message: String,
        /// 期望值
        expected: Option<f64>,
        /// 实际值
        actual: Option<f64>,
        /// 容差
        tolerance: Option<f64>,
        /// 修复建议
        suggestion: String,
    },
    /// 超时错误
    Timeout {
        /// 操作名称
        operation: String,
        /// 限制时间（秒）
        limit_secs: f64,
        /// 实际耗时（秒）
        actual_secs: f64,
    },
    /// 数据验证失败
    ValidationFailed {
        /// 字段名称
        field: String,
        /// 错误消息
        message: String,
        /// 修复建议
        suggestion: String,
    },
    /// 资源不可用
    ResourceUnavailable {
        /// 资源名称
        resource: String,
        /// 错误消息
        message: String,
    },
    /// 内部错误
    Internal {
        /// 错误消息
        message: String,
        /// 堆栈跟踪（可选）
        backtrace: Option<String>,
    },
}

impl ExperimentError {
    /// 创建断言失败错误
    pub fn assertion_failed(
        message: impl Into<String>,
        expected: impl Into<Option<f64>>,
        actual: impl Into<Option<f64>>,
        tolerance: impl Into<Option<f64>>,
    ) -> Self {
        let msg = message.into();
        let exp_val = expected.into();
        let act_val = actual.into();
        let tol_val = tolerance.into();
        let suggestion = Self::generate_assertion_suggestion(&msg, exp_val, act_val, tol_val);

        Self::AssertionFailed {
            message: msg,
            expected: exp_val,
            actual: act_val,
            tolerance: tol_val,
            suggestion,
        }
    }

    /// 创建超时错误
    pub fn timeout(operation: impl Into<String>, limit_secs: f64, actual_secs: f64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            limit_secs,
            actual_secs,
        }
    }

    /// 创建验证失败错误
    pub fn validation_failed(
        field: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::ValidationFailed {
            field: field.into(),
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// 创建资源不可用错误
    pub fn resource_unavailable(resource: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ResourceUnavailable {
            resource: resource.into(),
            message: message.into(),
        }
    }

    /// 创建内部错误
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            backtrace: None,
        }
    }

    /// 获取错误消息
    pub fn message(&self) -> &str {
        match self {
            Self::AssertionFailed { message, .. } => message,
            Self::ValidationFailed { message, .. } => message,
            Self::ResourceUnavailable { message, .. } => message,
            Self::Internal { message, .. } => message,
            Self::Timeout {
                operation,
                limit_secs: _,
                actual_secs: _,
            } => {
                // Timeout 消息使用 operation 字段作为返回（简化处理）
                operation
            }
        }
    }

    /// 获取修复建议
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            Self::AssertionFailed { suggestion, .. } => Some(suggestion),
            Self::ValidationFailed { suggestion, .. } => Some(suggestion),
            _ => None,
        }
    }

    /// 生成断言失败的修复建议
    fn generate_assertion_suggestion(
        message: &str,
        expected: Option<f64>,
        actual: Option<f64>,
        tolerance: Option<f64>,
    ) -> String {
        let mut suggestions = Vec::new();

        // 基于错误消息的建议
        if message.contains("角度") || message.contains("angle") {
            suggestions.push("检查角度单位是否正确（度数 vs 弧度）".to_string());
            suggestions.push("验证 measure_angle() 是否已内部转换为度数".to_string());
        }

        if message.contains("长度") || message.contains("length") {
            suggestions.push("检查容差设置是否合理".to_string());
            suggestions.push("验证浮点数精度是否满足要求".to_string());
        }

        if message.contains("性能")
            || message.contains("performance")
            || message.contains("throughput")
        {
            suggestions.push("检查系统负载是否影响性能".to_string());
            suggestions.push("考虑增加 warmup 迭代".to_string());
        }

        // 基于数值的建议
        if let (Some(exp), Some(act)) = (expected, actual) {
            if exp != 0.0 {
                let rel_error = (act - exp).abs() / exp.abs();
                if rel_error > 0.1 {
                    suggestions.push(format!(
                        "相对误差较大 ({:.2}%)，检查算法逻辑",
                        rel_error * 100.0
                    ));
                }
            }
        }

        // 基于容差的建议
        if let Some(tol) = tolerance {
            if tol < 1e-12 {
                suggestions.push("容差过小，可能受浮点数精度限制".to_string());
            }
            if tol > 1e-3 {
                suggestions.push("容差过大，可能掩盖真实问题".to_string());
            }
        }

        if suggestions.is_empty() {
            "检查测试逻辑和输入数据".to_string()
        } else {
            suggestions.join("; ")
        }
    }

    /// 格式化为详细字符串
    pub fn to_detailed_string(&self) -> String {
        let mut output = format!("错误：{}", self.message());

        if let Some(suggestion) = self.suggestion() {
            output.push_str(&format!("\n建议：{}", suggestion));
        }

        if let Self::AssertionFailed {
            expected,
            actual,
            tolerance,
            ..
        } = self
        {
            if let (Some(e), Some(a)) = (expected, actual) {
                output.push_str(&format!("\n期望：{}, 实际：{}", e, a));
                if let Some(t) = tolerance {
                    output.push_str(&format!(", 容差：{}", t));
                }
                let abs_error = (a - e).abs();
                let rel_error = if *e != 0.0 { abs_error / e.abs() } else { 0.0 };
                output.push_str(&format!(
                    "\n绝对误差：{:.2e}, 相对误差：{:.2e}%",
                    abs_error,
                    rel_error * 100.0
                ));
            }
        }

        output
    }
}

impl fmt::Display for ExperimentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for ExperimentError {}

/// 实验结果类型（Result 别名）
pub type ExperimentResult<T> = Result<T, ExperimentError>;

/// 断言宏：检查值是否在容差范围内
#[macro_export]
macro_rules! assert_within_tolerance {
    ($actual:expr, $expected:expr, $tolerance:expr $(, $msg:expr)?) => {
        {
            let actual = $actual;
            let expected = $expected;
            let tolerance = $tolerance;
            let abs_error = (actual - expected).abs();

            if abs_error > tolerance {
                let msg = format!($($msg)?);
                return Err($crate::experiment::error::ExperimentError::assertion_failed(
                    msg,
                    expected,
                    actual,
                    tolerance,
                ));
            }
        }
    };
}

/// 断言宏：检查条件是否为真
#[macro_export]
macro_rules! assert_experiment {
    ($cond:expr $(, $msg:expr)? $(, expected = $expected:expr, actual = $actual:expr, tolerance = $tolerance:expr)?) => {
        if !$cond {
            return Err($crate::experiment::error::ExperimentError::assertion_failed(
                format!($($msg)?),
                $($expected,)*
                $($actual,)*
                $($tolerance,)*
            ));
        }
    };
}

/// 断言宏：检查性能指标是否达标
#[macro_export]
macro_rules! assert_performance {
    ($metric:expr, $threshold:expr, ge $(, $msg:expr)?) => {
        assert_performance!($metric, $threshold, >= $(, $msg)?);
    };
    ($metric:expr, $threshold:expr, le $(, $msg:expr)?) => {
        assert_performance!($metric, $threshold, <= $(, $msg)?);
    };
    ($metric:expr, $threshold:expr, gt $(, $msg:expr)?) => {
        assert_performance!($metric, $threshold, > $(, $msg)?);
    };
    ($metric:expr, $threshold:expr, lt $(, $msg:expr)?) => {
        assert_performance!($metric, $threshold, < $(, $msg)?);
    };
    ($metric:expr, $threshold:expr, >= $(, $msg:expr)?) => {
        {
            let metric = $metric;
            let threshold = $threshold;
            if metric < threshold {
                let msg = format!($($msg)?);
                return Err($crate::experiment::error::ExperimentError::assertion_failed(
                    format!("性能指标未达标：{}", msg),
                    Some(threshold),
                    Some(metric),
                    None,
                ));
            }
        }
    };
    ($metric:expr, $threshold:expr, <= $(, $msg:expr)?) => {
        {
            let metric = $metric;
            let threshold = $threshold;
            if metric > threshold {
                let msg = format!($($msg)?);
                return Err($crate::experiment::error::ExperimentError::assertion_failed(
                    format!("性能指标未达标：{}", msg),
                    Some(threshold),
                    Some(metric),
                    None,
                ));
            }
        }
    };
    ($metric:expr, $threshold:expr, > $(, $msg:expr)?) => {
        {
            let metric = $metric;
            let threshold = $threshold;
            if metric <= threshold {
                let msg = format!($($msg)?);
                return Err($crate::experiment::error::ExperimentError::assertion_failed(
                    format!("性能指标未达标：{}", msg),
                    Some(threshold),
                    Some(metric),
                    None,
                ));
            }
        }
    };
    ($metric:expr, $threshold:expr, < $(, $msg:expr)?) => {
        {
            let metric = $metric;
            let threshold = $threshold;
            if metric >= threshold {
                let msg = format!($($msg)?);
                return Err($crate::experiment::error::ExperimentError::assertion_failed(
                    format!("性能指标未达标：{}", msg),
                    Some(threshold),
                    Some(metric),
                    None,
                ));
            }
        }
    };
}

/// 断言宏：检查是否超时
#[macro_export]
macro_rules! assert_not_timeout {
    ($elapsed:expr, $limit:expr $(, $msg:expr)?) => {
        {
            let elapsed = $elapsed.as_secs_f64();
            let limit = $limit;

            if elapsed > limit {
                return Err($crate::experiment::error::ExperimentError::timeout(
                    format!($($msg)?),
                    limit,
                    elapsed,
                ));
            }
        }
    };
}

/// 工具函数：统一的容差断言
pub fn assert_within_tolerance_fn(
    actual: f64,
    expected: f64,
    tolerance: f64,
    message: &str,
) -> ExperimentResult<()> {
    let abs_error = (actual - expected).abs();

    if abs_error <= tolerance {
        Ok(())
    } else {
        let rel_error = if expected != 0.0 {
            abs_error / expected.abs()
        } else {
            0.0
        };

        Err(ExperimentError::assertion_failed(
            format!(
                "{}: 绝对误差={:.2e}, 相对误差={:.2e}%",
                message,
                abs_error,
                rel_error * 100.0
            ),
            expected,
            actual,
            tolerance,
        ))
    }
}

/// 工具函数：检查布尔条件
pub fn assert_true_fn(condition: bool, message: &str) -> ExperimentResult<()> {
    if condition {
        Ok(())
    } else {
        Err(ExperimentError::assertion_failed(message, None, None, None))
    }
}

/// 工具函数：检查性能阈值
pub fn assert_performance_fn(
    metric: f64,
    threshold: f64,
    comparison: PerformanceComparison,
    message: &str,
) -> ExperimentResult<()> {
    let passed = match comparison {
        PerformanceComparison::Greater => metric > threshold,
        PerformanceComparison::GreaterEq => metric >= threshold,
        PerformanceComparison::Less => metric < threshold,
        PerformanceComparison::LessEq => metric <= threshold,
    };

    if passed {
        Ok(())
    } else {
        Err(ExperimentError::assertion_failed(
            format!("{}: 指标={:.4}, 阈值={:.4}", message, metric, threshold),
            Some(threshold),
            Some(metric),
            None,
        ))
    }
}

/// 性能比较类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PerformanceComparison {
    Greater,
    GreaterEq,
    Less,
    LessEq,
}

/// 错误收集器：收集多个错误而不立即返回
#[derive(Debug, Default)]
pub struct ErrorCollector {
    errors: Vec<ExperimentError>,
    warnings: Vec<String>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加错误
    pub fn add_error(&mut self, error: ExperimentError) {
        self.errors.push(error);
    }

    /// 添加警告
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 获取错误数量
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 获取警告数量
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// 转换为结果（如果有错误则返回第一个错误）
    pub fn into_result(self) -> ExperimentResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.into_iter().next().unwrap())
        }
    }

    /// 获取所有错误
    pub fn errors(&self) -> &[ExperimentError] {
        &self.errors
    }

    /// 获取所有警告
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// 格式化为字符串
    pub fn to_summary_string(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "错误统计：{} 个错误，{} 个警告\n\n",
            self.errors.len(),
            self.warnings.len()
        ));

        if !self.errors.is_empty() {
            output.push_str("错误详情:\n");
            for (i, error) in self.errors.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, error.to_detailed_string()));
            }
            output.push('\n');
        }

        if !self.warnings.is_empty() {
            output.push_str("警告详情:\n");
            for (i, warning) in self.warnings.iter().enumerate() {
                output.push_str(&format!("  {}. {}\n", i + 1, warning));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_within_tolerance_success() {
        let result = assert_within_tolerance_fn(10.0001, 10.0, 1e-3, "测试");
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_within_tolerance_failure() {
        let result = assert_within_tolerance_fn(10.1, 10.0, 1e-3, "测试");
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(e.suggestion().is_some());
        }
    }

    #[test]
    fn test_assert_true_fn_success() {
        let result = assert_true_fn(true, "测试");
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_true_fn_failure() {
        let result = assert_true_fn(false, "条件应为真");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_collector() {
        let mut collector = ErrorCollector::new();
        collector.add_error(ExperimentError::internal("测试错误"));
        collector.add_warning("测试警告");

        assert!(collector.has_errors());
        assert_eq!(collector.error_count(), 1);
        assert_eq!(collector.warning_count(), 1);
    }

    #[test]
    fn test_error_to_detailed_string() {
        let error = ExperimentError::assertion_failed("测试失败", 10.0, 10.5, 0.1);

        let detailed = error.to_detailed_string();
        assert!(detailed.contains("期望：10"));
        assert!(detailed.contains("实际：10.5"));
        assert!(detailed.contains("建议："));
    }
}
