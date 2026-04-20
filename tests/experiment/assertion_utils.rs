//! 测试断言工具模块
//!
//! 提供统一的测试断言函数，避免重复代码和逻辑错误。
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::experiment::assertion_utils::*;
//!
//! // 长度测量断言
//! assert_length_measurement(measured, expected, tolerance)?;
//!
//! // 角度测量断言（自动处理度数）
//! assert_angle_measurement(measured_deg, expected_deg, tolerance)?;
//!
//! // 布尔条件断言
//! assert_condition(result.is_some(), "结果应该存在")?;
//! ```

#![allow(dead_code)]

use super::error::{ExperimentError, ExperimentResult, PerformanceComparison};

/// 容差类型
#[derive(Debug, Clone, Copy)]
pub enum ToleranceType {
    /// 绝对容差
    Absolute(f64),
    /// 相对容差
    Relative(f64),
    /// 混合容差（绝对 + 相对）
    Mixed { absolute: f64, relative: f64 },
}

impl ToleranceType {
    /// 检查误差是否在容差范围内
    pub fn is_within(&self, actual: f64, expected: f64) -> bool {
        match self {
            Self::Absolute(tol) => {
                let abs_error = (actual - expected).abs();
                abs_error <= *tol
            }
            Self::Relative(tol) => {
                if expected == 0.0 {
                    actual == 0.0
                } else {
                    let rel_error = (actual - expected).abs() / expected.abs();
                    rel_error <= *tol
                }
            }
            Self::Mixed { absolute, relative } => {
                let abs_error = (actual - expected).abs();
                let rel_error = if expected != 0.0 {
                    abs_error / expected.abs()
                } else {
                    0.0
                };
                abs_error <= *absolute || rel_error <= *relative
            }
        }
    }

    /// 计算误差信息
    pub fn compute_error_info(&self, actual: f64, expected: f64) -> ErrorInfo {
        let abs_error = (actual - expected).abs();
        let rel_error = if expected != 0.0 {
            abs_error / expected.abs()
        } else {
            0.0
        };

        ErrorInfo {
            absolute_error: abs_error,
            relative_error: rel_error,
            passed: self.is_within(actual, expected),
        }
    }
}

/// 误差信息
#[derive(Debug, Clone, Copy)]
pub struct ErrorInfo {
    pub absolute_error: f64,
    pub relative_error: f64,
    pub passed: bool,
}

impl ErrorInfo {
    pub fn display(&self, tolerance: &ToleranceType) -> String {
        format!(
            "绝对误差={:.2e}, 相对误差={:.2e}%, 容差={:?}",
            self.absolute_error,
            self.relative_error * 100.0,
            tolerance
        )
    }
}

/// 长度测量断言
///
/// # 参数
/// - `measured`: 测量值
/// - `expected`: 期望值
/// - `tolerance`: 容差（绝对容差）
/// - `message`: 错误消息
pub fn assert_length_measurement(
    measured: f64,
    expected: f64,
    tolerance: f64,
    message: &str,
) -> ExperimentResult<ErrorInfo> {
    let tol = ToleranceType::Absolute(tolerance);
    let info = tol.compute_error_info(measured, expected);

    if info.passed {
        Ok(info)
    } else {
        Err(ExperimentError::assertion_failed(
            format!("{}: 长度测量失败", message),
            expected,
            measured,
            tolerance,
        ))
    }
}

/// 角度测量断言
///
/// # 注意
/// 此函数假设输入已经是度数（degree），不需要再转换
///
/// # 参数
/// - `measured_deg`: 测量值（度数）
/// - `expected_deg`: 期望值（度数）
/// - `tolerance_deg`: 容差（度数）
/// - `message`: 错误消息
pub fn assert_angle_measurement(
    measured_deg: f64,
    expected_deg: f64,
    tolerance_deg: f64,
    message: &str,
) -> ExperimentResult<ErrorInfo> {
    // 角度使用绝对容差（度数）
    let tol = ToleranceType::Absolute(tolerance_deg);
    let info = tol.compute_error_info(measured_deg, expected_deg);

    if info.passed {
        Ok(info)
    } else {
        Err(ExperimentError::assertion_failed(
            format!("{}: 角度测量失败（单位：度）", message),
            expected_deg,
            measured_deg,
            tolerance_deg,
        ))
    }
}

/// 面积测量断言
pub fn assert_area_measurement(
    measured: f64,
    expected: f64,
    tolerance: ToleranceType,
    message: &str,
) -> ExperimentResult<ErrorInfo> {
    let info = tolerance.compute_error_info(measured, expected);

    if info.passed {
        Ok(info)
    } else {
        Err(ExperimentError::assertion_failed(
            format!("{}: 面积测量失败", message),
            expected,
            measured,
            None,
        ))
    }
}

/// 周长测量断言
pub fn assert_perimeter_measurement(
    measured: f64,
    expected: f64,
    tolerance: f64,
    message: &str,
) -> ExperimentResult<ErrorInfo> {
    assert_length_measurement(measured, expected, tolerance, message)
}

/// 坐标变换断言
pub fn assert_transform_result(
    measured_x: f64,
    measured_y: f64,
    expected_x: f64,
    expected_y: f64,
    tolerance: f64,
    message: &str,
) -> ExperimentResult<()> {
    let abs_error_x = (measured_x - expected_x).abs();
    let abs_error_y = (measured_y - expected_y).abs();
    let max_error = abs_error_x.max(abs_error_y);

    if max_error <= tolerance {
        Ok(())
    } else {
        Err(ExperimentError::assertion_failed(
            format!("{}: 变换结果超出容差", message),
            expected_x,
            measured_x,
            tolerance,
        ))
    }
}

/// 布尔条件断言
pub fn assert_condition(condition: bool, message: &str) -> ExperimentResult<()> {
    if condition {
        Ok(())
    } else {
        Err(ExperimentError::assertion_failed(message, None, None, None))
    }
}

/// 性能指标断言
pub fn assert_performance_metric(
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
            format!("{}: 性能指标未达标", message),
            Some(threshold),
            Some(metric),
            None,
        ))
    }
}

/// 吞吐量断言（应该大于阈值）
pub fn assert_throughput(
    throughput: f64,
    min_throughput: f64,
    message: &str,
) -> ExperimentResult<()> {
    assert_performance_metric(
        throughput,
        min_throughput,
        PerformanceComparison::GreaterEq,
        message,
    )
}

/// 延迟断言（应该小于阈值）
pub fn assert_latency(latency: f64, max_latency: f64, message: &str) -> ExperimentResult<()> {
    assert_performance_metric(latency, max_latency, PerformanceComparison::LessEq, message)
}

/// 准确率断言（应该大于阈值）
pub fn assert_accuracy(accuracy: f64, min_accuracy: f64, message: &str) -> ExperimentResult<()> {
    assert_performance_metric(
        accuracy,
        min_accuracy,
        PerformanceComparison::GreaterEq,
        message,
    )
}

/// 统计检验断言（t 检验）
pub fn assert_t_test(
    t_value: f64,
    p_value: f64,
    expected_direction: TTestDirection,
    significance_level: f64,
    message: &str,
) -> ExperimentResult<()> {
    // 检查显著性
    if p_value >= significance_level {
        return Err(ExperimentError::assertion_failed(
            format!(
                "{}: p 值 ({:.4}) >= 显著性水平 ({:.4}), 差异不显著",
                message, p_value, significance_level
            ),
            Some(significance_level),
            Some(p_value),
            None,
        ));
    }

    // 检查方向
    let direction_correct = match expected_direction {
        TTestDirection::Positive => t_value > 0.0,
        TTestDirection::Negative => t_value < 0.0,
        TTestDirection::Any => true,
    };

    if !direction_correct {
        return Err(ExperimentError::assertion_failed(
            format!(
                "{}: t 值方向错误 (期望={:?}, 实际={:.4})",
                message, expected_direction, t_value
            ),
            None,
            Some(t_value),
            None,
        ));
    }

    Ok(())
}

/// t 检验期望方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TTestDirection {
    /// 期望 t 值为正（after > before）
    Positive,
    /// 期望 t 值为负（after < before）
    Negative,
    /// 任意方向
    Any,
}

/// 关系检测断言
pub fn assert_relation_detected<T>(
    relations: &[T],
    matcher: impl Fn(&T) -> bool,
    message: &str,
) -> ExperimentResult<()> {
    let found = relations.iter().any(matcher);

    if found {
        Ok(())
    } else {
        Err(ExperimentError::assertion_failed(
            format!("{}: 未检测到期望的关系", message),
            None,
            None,
            None,
        ))
    }
}

/// 集合大小断言
pub fn assert_collection_size<T>(
    collection: &[T],
    expected_size: usize,
    message: &str,
) -> ExperimentResult<()> {
    let actual_size = collection.len();

    if actual_size == expected_size {
        Ok(())
    } else {
        Err(ExperimentError::assertion_failed(
            format!("{}: 集合大小不匹配", message),
            Some(expected_size as f64),
            Some(actual_size as f64),
            None,
        ))
    }
}

/// 非空集合断言
pub fn assert_non_empty<T>(collection: &[T], message: &str) -> ExperimentResult<()> {
    assert_condition(!collection.is_empty(), message)
}

/// 可选值断言
pub fn assert_some<T>(option: Option<T>, message: &str) -> ExperimentResult<T> {
    match option {
        Some(value) => Ok(value),
        None => Err(ExperimentError::assertion_failed(
            message,
            Some(1.0),
            Some(0.0),
            None,
        )),
    }
}

/// 结果断言（Ok 变体）
pub fn assert_ok<T, E>(result: Result<T, E>, message: &str) -> ExperimentResult<T> {
    match result {
        Ok(value) => Ok(value),
        Err(_) => Err(ExperimentError::assertion_failed(
            message,
            Some(1.0),
            Some(0.0),
            None,
        )),
    }
}

/// 批量测试执行器
pub struct TestBatch {
    total: usize,
    passed: usize,
    errors: Vec<String>,
}

impl TestBatch {
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            errors: Vec::new(),
        }
    }

    /// 执行单个测试用例
    pub fn run_case<F>(&mut self, test_fn: F, case_name: &str)
    where
        F: FnOnce() -> ExperimentResult<()>,
    {
        self.total += 1;

        match test_fn() {
            Ok(()) => self.passed += 1,
            Err(e) => {
                self.errors.push(format!("{}: {}", case_name, e.message()));
            }
        }
    }

    /// 获取通过率
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            1.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }

    /// 获取结果
    pub fn result(&self) -> BatchResult {
        BatchResult {
            total: self.total,
            passed: self.passed,
            failed: self.total - self.passed,
            pass_rate: self.pass_rate(),
            errors: self.errors.clone(),
        }
    }

    /// 转换为最终结果
    pub fn into_result(self, summary_message: &str) -> ExperimentResult<BatchResult> {
        let result = self.result();

        if result.all_passed() {
            Ok(result)
        } else {
            Err(ExperimentError::assertion_failed(
                format!(
                    "{}: {}/{} 通过",
                    summary_message, result.passed, result.total
                ),
                Some(result.total as f64),
                Some(result.passed as f64),
                None,
            ))
        }
    }
}

impl Default for TestBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// 批量测试结果
#[derive(Debug, Clone)]
pub struct BatchResult {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    pub errors: Vec<String>,
}

impl BatchResult {
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    pub fn summary(&self) -> String {
        format!(
            "总计：{}, 通过：{}, 失败：{}, 通过率：{:.2}%",
            self.total,
            self.passed,
            self.failed,
            self.pass_rate * 100.0
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tolerance_absolute() {
        let tol = ToleranceType::Absolute(0.001);
        assert!(tol.is_within(10.0001, 10.0));
        assert!(!tol.is_within(10.01, 10.0));
    }

    #[test]
    fn test_tolerance_relative() {
        let tol = ToleranceType::Relative(0.01); // 1%
        assert!(tol.is_within(100.5, 100.0));
        assert!(!tol.is_within(102.0, 100.0));
    }

    #[test]
    fn test_assert_length_measurement() {
        let result = assert_length_measurement(10.0001, 10.0, 1e-3, "测试");
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_angle_measurement() {
        let result = assert_angle_measurement(90.0001, 90.0, 1e-3, "测试");
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_condition() {
        assert!(assert_condition(true, "应该为真").is_ok());
        assert!(assert_condition(false, "应该为真").is_err());
    }

    #[test]
    fn test_test_batch() {
        let mut batch = TestBatch::new();

        batch.run_case(|| Ok(()), "case1");
        batch.run_case(|| Ok(()), "case2");
        batch.run_case(|| Err(ExperimentError::internal("失败")), "case3");

        assert_eq!(batch.total, 3);
        assert_eq!(batch.passed, 2);
        assert_eq!(batch.pass_rate(), 2.0 / 3.0);
    }
}
