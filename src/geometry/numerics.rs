//! 数值精度与容差工具
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
//!
//! 提供浮点数比较、容差配置和数值稳定性工具
//!
//! # 设计原则
//!
//! 1. **相对容差**: 根据数值大小自动调整容差
//! 2. **绝对容差**: 处理接近零的情况
//! 3. **角度容差**: 专门处理角度比较
//! 4. **数值稳定性**: 避免浮点数误差累积
//!
//! # 示例
//!
//! ```rust
//! use cadagent::geometry::numerics::{ToleranceConfig, almost_equals};
//!
//! // 使用默认容差
//! let tol = ToleranceConfig::default();
//! assert!(tol.almost_equals(1.0, 1.0 + 1e-10));
//!
//! // 自定义容差
//! let tol = ToleranceConfig::new()
//!     .with_absolute(1e-12)
//!     .with_relative(1e-9);
//! assert!(tol.almost_equals(1000.0, 1000.0 + 1e-9));
//!
//! // 角度比较
//! assert!(tol.almost_equals_angle(std::f64::consts::PI / 2.0, std::f64::consts::PI / 2.0 + 1e-10));
//! ```

use serde::{Deserialize, Serialize};
use std::f64;

/// 默认绝对容差
pub const DEFAULT_ABSOLUTE_TOLERANCE: f64 = 1e-12;

/// 默认相对容差
pub const DEFAULT_RELATIVE_TOLERANCE: f64 = 1e-9;

/// 默认角度容差（弧度）
pub const DEFAULT_ANGULAR_TOLERANCE: f64 = 1e-9;

/// 机器精度（用于数值稳定性）
pub const MACHINE_EPSILON: f64 = f64::EPSILON;

/// 容差配置
///
/// 提供多层次的容差控制，适应不同的几何计算场景
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ToleranceConfig {
    /// 绝对容差：用于接近零的数值比较
    pub absolute: f64,
    /// 相对容差：用于大数值比较
    pub relative: f64,
    /// 角度容差（弧度）：用于角度比较
    pub angular: f64,
    /// 零值阈值：判断是否为零的阈值
    pub zero_threshold: f64,
}

impl Default for ToleranceConfig {
    fn default() -> Self {
        Self {
            absolute: DEFAULT_ABSOLUTE_TOLERANCE,
            relative: DEFAULT_RELATIVE_TOLERANCE,
            angular: DEFAULT_ANGULAR_TOLERANCE,
            zero_threshold: DEFAULT_ABSOLUTE_TOLERANCE,
        }
    }
}

impl ToleranceConfig {
    /// 创建新的容差配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建高精度容差配置（用于精密 CAD 操作）
    pub fn high_precision() -> Self {
        Self {
            absolute: 1e-15,
            relative: 1e-12,
            angular: 1e-12,
            zero_threshold: 1e-15,
        }
    }

    /// 创建低精度容差配置（用于快速预览或 LOD）
    pub fn low_precision() -> Self {
        Self {
            absolute: 1e-8,
            relative: 1e-6,
            angular: 1e-6,
            zero_threshold: 1e-8,
        }
    }

    /// 设置绝对容差
    pub fn with_absolute(mut self, tol: f64) -> Self {
        self.absolute = tol;
        self.zero_threshold = tol;
        self
    }

    /// 设置相对容差
    pub fn with_relative(mut self, tol: f64) -> Self {
        self.relative = tol;
        self
    }

    /// 设置角度容差
    pub fn with_angular(mut self, tol: f64) -> Self {
        self.angular = tol;
        self
    }

    /// 设置零值阈值
    pub fn with_zero_threshold(mut self, tol: f64) -> Self {
        self.zero_threshold = tol;
        self
    }

    /// 计算给定数值的实际容差
    ///
    /// 结合绝对容差和相对容差：
    /// `tolerance = max(absolute, relative * |value|)`
    pub fn compute_tolerance(&self, value: f64) -> f64 {
        let abs_value = value.abs();
        self.absolute.max(self.relative * abs_value)
    }

    /// 判断两个浮点数是否几乎相等
    ///
    /// 使用组合容差：
    /// `|a - b| <= max(absolute, relative * max(|a|, |b|))`
    pub fn almost_equals(&self, a: f64, b: f64) -> bool {
        let diff = (a - b).abs();
        let max_val = a.abs().max(b.abs());
        let tolerance = self.absolute.max(self.relative * max_val);
        diff <= tolerance
    }

    /// 判断浮点数是否几乎为零
    pub fn is_zero(&self, value: f64) -> bool {
        value.abs() <= self.zero_threshold
    }

    /// 判断浮点数是否大于零（考虑容差）
    pub fn is_positive(&self, value: f64) -> bool {
        value > self.zero_threshold
    }

    /// 判断浮点数是否小于零（考虑容差）
    pub fn is_negative(&self, value: f64) -> bool {
        value < -self.zero_threshold
    }

    /// 判断 a 是否几乎小于 b
    pub fn almost_less(&self, a: f64, b: f64) -> bool {
        a < b - self.compute_tolerance(b.abs().max(a.abs()))
    }

    /// 判断 a 是否几乎大于 b
    pub fn almost_greater(&self, a: f64, b: f64) -> bool {
        a > b + self.compute_tolerance(b.abs().max(a.abs()))
    }

    /// 判断两个角度是否几乎相等（弧度）
    ///
    /// 自动处理角度周期性（2π）
    pub fn almost_equals_angle(&self, a: f64, b: f64) -> bool {
        let mut diff = (a - b).abs();
        let two_pi = 2.0 * std::f64::consts::PI;

        // 处理角度周期性
        while diff > two_pi {
            diff = (diff - two_pi).abs();
        }
        if diff > std::f64::consts::PI {
            diff = two_pi - diff;
        }

        diff <= self.angular
    }

    /// 规范化角度到 [-π, π] 范围
    pub fn normalize_angle(&self, angle: f64) -> f64 {
        let two_pi = 2.0 * std::f64::consts::PI;
        let mut normalized = angle % two_pi;

        if normalized > std::f64::consts::PI {
            normalized -= two_pi;
        } else if normalized < -std::f64::consts::PI {
            normalized += two_pi;
        }

        normalized
    }

    /// 将数值舍入到容差级别
    ///
    /// 用于消除数值噪声，提高稳定性
    pub fn snap_to_tolerance(&self, value: f64) -> f64 {
        if self.is_zero(value) {
            return 0.0;
        }

        // 找到最接近的"整洁"值
        let scale = value.abs().log10().floor();
        let unit = 10f64.powi(scale as i32);
        let normalized = value / unit;

        // 舍入到相对容差级别
        let precision = (1.0 / self.relative).log10().ceil() as i32;
        let rounded = (normalized * 10f64.powi(precision)).round() / 10f64.powi(precision);

        rounded * unit
    }
}

/// 使用默认容差判断两个浮点数是否几乎相等
#[inline]
pub fn almost_equals(a: f64, b: f64) -> bool {
    ToleranceConfig::default().almost_equals(a, b)
}

/// 使用默认容差判断浮点数是否几乎为零
#[inline]
pub fn is_zero(value: f64) -> bool {
    ToleranceConfig::default().is_zero(value)
}

/// 使用默认容差判断两个角度是否几乎相等
#[inline]
pub fn almost_equals_angle(a: f64, b: f64) -> bool {
    ToleranceConfig::default().almost_equals_angle(a, b)
}

/// 安全除法
///
/// 当除数接近零时返回 None，避免数值不稳定
pub fn safe_divide(numerator: f64, denominator: f64, tol: &ToleranceConfig) -> Option<f64> {
    if tol.is_zero(denominator) {
        None
    } else {
        Some(numerator / denominator)
    }
}

/// 安全的平方根计算
///
/// 当输入为负但接近零时，返回 0 而非 NaN
///
/// # Returns
///
/// - `Ok(f64)`: 计算结果
/// - `Err(f64)`: 输入为明显负值（超出容差范围），返回原始值作为错误指示
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::numerics::{safe_sqrt, ToleranceConfig};
///
/// let tol = ToleranceConfig::default();
/// assert_eq!(safe_sqrt(4.0, &tol), Ok(2.0));
/// assert_eq!(safe_sqrt(0.0, &tol), Ok(0.0));
/// assert_eq!(safe_sqrt(-1e-15, &tol), Ok(0.0)); // 接近零的负值
/// assert!(safe_sqrt(-1.0, &tol).is_err()); // 明显负值
/// ```
pub fn safe_sqrt(value: f64, tol: &ToleranceConfig) -> Result<f64, f64> {
    if value < 0.0 {
        if tol.is_zero(value) {
            // 接近零的负值，视为零处理
            Ok(0.0)
        } else {
            // 明显负值，返回错误（Debug 和 Release 行为一致）
            Err(value)
        }
    } else {
        Ok(value.sqrt())
    }
}

/// 安全的平方根计算（静默版本）
///
/// 当输入为负时，无论是否接近零都返回 0
/// 适用于不希望处理错误的场景
pub fn safe_sqrt_silent(value: f64, _tol: &ToleranceConfig) -> f64 {
    if value < 0.0 {
        0.0
    } else {
        value.sqrt()
    }
}

/// 稳定的线性插值
///
/// 避免在 t 接近 0 或 1 时的数值误差
pub fn stable_lerp(a: f64, b: f64, t: f64, tol: &ToleranceConfig) -> f64 {
    if tol.almost_equals(t, 0.0) {
        return a;
    }
    if tol.almost_equals(t, 1.0) {
        return b;
    }
    a * (1.0 - t) + b * t
}

/// 计算两个向量的点积（2D）
#[inline]
pub fn dot2(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * bx + ay * by
}

/// 计算两个向量的叉积（2D，返回标量）
#[inline]
pub fn cross2(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
}

/// 计算向量长度（2D）
#[inline]
pub fn length2(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// 计算向量长度的平方（2D，避免开方）
#[inline]
pub fn length_squared2(x: f64, y: f64) -> f64 {
    x * x + y * y
}

/// 归一化向量（2D）
pub fn normalize2(x: f64, y: f64, tol: &ToleranceConfig) -> Option<(f64, f64)> {
    let len = length2(x, y);
    if tol.is_zero(len) {
        return None;
    }
    Some((x / len, y / len))
}

/// 条件数估计
///
/// 用于判断矩阵或方程组的数值稳定性
pub struct ConditionNumber {
    pub value: f64,
    pub is_ill_conditioned: bool,
}

impl ConditionNumber {
    /// 从比值估计条件数
    pub fn from_ratio(largest: f64, smallest: f64, tol: &ToleranceConfig) -> Self {
        if tol.is_zero(smallest) {
            return Self {
                value: f64::INFINITY,
                is_ill_conditioned: true,
            };
        }

        let value = largest.abs() / smallest.abs();
        // 条件数大于 1/relative_tolerance 时认为病态
        let threshold = 1.0 / tol.relative;

        Self {
            value,
            is_ill_conditioned: value > threshold,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tolerance_config_default() {
        let tol = ToleranceConfig::default();
        assert_eq!(tol.absolute, DEFAULT_ABSOLUTE_TOLERANCE);
        assert_eq!(tol.relative, DEFAULT_RELATIVE_TOLERANCE);
        assert_eq!(tol.angular, DEFAULT_ANGULAR_TOLERANCE);
    }

    #[test]
    fn test_almost_equals_small_values() {
        let tol = ToleranceConfig::default();

        // 小数值比较
        assert!(tol.almost_equals(1e-13, 0.0));
        assert!(tol.almost_equals(-1e-13, 0.0));
        assert!(!tol.almost_equals(1e-10, 0.0));
    }

    #[test]
    fn test_almost_equals_large_values() {
        let tol = ToleranceConfig::default();

        // 大数值比较（相对容差生效）
        assert!(tol.almost_equals(1000.0, 1000.0 + 1e-6));
        assert!(!tol.almost_equals(1000.0, 1000.0 + 1e-3));

        // 非常大数值
        assert!(tol.almost_equals(1e10, 1e10 + 10.0));
    }

    #[test]
    fn test_is_zero() {
        let tol = ToleranceConfig::default();

        assert!(tol.is_zero(0.0));
        assert!(tol.is_zero(1e-13));
        assert!(tol.is_zero(-1e-13));
        assert!(!tol.is_zero(1e-10));
        assert!(!tol.is_zero(-1e-10));
    }

    #[test]
    fn test_is_positive_negative() {
        let tol = ToleranceConfig::default();

        assert!(tol.is_positive(1e-10));
        assert!(!tol.is_positive(1e-13));
        assert!(!tol.is_positive(0.0));
        assert!(!tol.is_positive(-1e-13));

        assert!(tol.is_negative(-1e-10));
        assert!(!tol.is_negative(-1e-13));
        assert!(!tol.is_negative(0.0));
    }

    #[test]
    fn test_almost_equals_angle() {
        let tol = ToleranceConfig::default();

        // 基本角度比较
        assert!(tol.almost_equals_angle(0.0, 0.0));
        assert!(tol.almost_equals_angle(
            std::f64::consts::PI / 2.0,
            std::f64::consts::PI / 2.0 + 1e-10
        ));

        // 周期性处理
        assert!(tol.almost_equals_angle(0.0, 2.0 * std::f64::consts::PI));
        assert!(tol.almost_equals_angle(-std::f64::consts::PI, std::f64::consts::PI));
        assert!(tol.almost_equals_angle(0.1, 2.0 * std::f64::consts::PI + 0.1));
    }

    #[test]
    fn test_normalize_angle() {
        let tol = ToleranceConfig::default();

        assert!(tol.almost_equals(tol.normalize_angle(0.0), 0.0));
        assert!(tol.almost_equals(tol.normalize_angle(2.0 * std::f64::consts::PI), 0.0));
        assert!(tol.almost_equals(
            tol.normalize_angle(3.0 * std::f64::consts::PI),
            std::f64::consts::PI
        ));
        assert!(tol.almost_equals(
            tol.normalize_angle(-std::f64::consts::PI / 2.0),
            -std::f64::consts::PI / 2.0
        ));
    }

    #[test]
    fn test_snap_to_tolerance() {
        let tol = ToleranceConfig::default();

        // 接近零的值应该被舍入到零
        assert_eq!(tol.snap_to_tolerance(1e-13), 0.0);
        assert_eq!(tol.snap_to_tolerance(-1e-13), 0.0);

        // 其他值应该被舍入到合理精度
        let snapped = tol.snap_to_tolerance(1.0000000001);
        assert!(tol.almost_equals(snapped, 1.0));
    }

    #[test]
    fn test_compute_tolerance() {
        let tol = ToleranceConfig::default();

        // 小值使用绝对容差
        assert_eq!(tol.compute_tolerance(0.0), tol.absolute);
        assert_eq!(tol.compute_tolerance(1e-10), tol.absolute);

        // 大值使用相对容差
        assert!(tol.compute_tolerance(1000.0) > tol.absolute);
        assert!((tol.compute_tolerance(1000.0) - tol.relative * 1000.0).abs() < 1e-15);
    }

    #[test]
    fn test_safe_divide() {
        let tol = ToleranceConfig::default();

        assert_eq!(safe_divide(10.0, 2.0, &tol), Some(5.0));
        assert_eq!(safe_divide(10.0, 1e-13, &tol), None);
        assert_eq!(safe_divide(10.0, 0.0, &tol), None);
    }

    #[test]
    fn test_safe_sqrt() {
        let tol = ToleranceConfig::default();

        // 正常情况
        assert_eq!(safe_sqrt(4.0, &tol), Ok(2.0));
        assert_eq!(safe_sqrt(0.0, &tol), Ok(0.0));

        // 接近零的负值应该返回 Ok(0.0)
        assert_eq!(safe_sqrt(-1e-15, &tol), Ok(0.0));
        assert_eq!(safe_sqrt(-1e-13, &tol), Ok(0.0));

        // 明显负值应该返回 Err
        assert!(safe_sqrt(-1.0, &tol).is_err());
        assert!(safe_sqrt(-1e-6, &tol).is_err());

        // 验证 Err 返回的是原始值
        if let Err(original) = safe_sqrt(-5.0, &tol) {
            assert_eq!(original, -5.0);
        } else {
            panic!("Expected Err for -5.0");
        }
    }

    #[test]
    fn test_safe_sqrt_silent() {
        let tol = ToleranceConfig::default();

        // 正常情况
        assert_eq!(safe_sqrt_silent(4.0, &tol), 2.0);
        assert_eq!(safe_sqrt_silent(0.0, &tol), 0.0);

        // 所有负值都返回 0.0
        assert_eq!(safe_sqrt_silent(-1e-15, &tol), 0.0);
        assert_eq!(safe_sqrt_silent(-1.0, &tol), 0.0);
        assert_eq!(safe_sqrt_silent(-100.0, &tol), 0.0);
    }

    #[test]
    fn test_stable_lerp() {
        let tol = ToleranceConfig::default();

        assert!(tol.almost_equals(stable_lerp(0.0, 10.0, 0.0, &tol), 0.0));
        assert!(tol.almost_equals(stable_lerp(0.0, 10.0, 1.0, &tol), 10.0));
        assert!(tol.almost_equals(stable_lerp(0.0, 10.0, 0.5, &tol), 5.0));
    }

    #[test]
    fn test_dot_cross_products() {
        // 点积
        assert_eq!(dot2(1.0, 0.0, 0.0, 1.0), 0.0);
        assert_eq!(dot2(1.0, 0.0, 1.0, 0.0), 1.0);
        assert_eq!(dot2(1.0, 1.0, 2.0, 2.0), 4.0);

        // 叉积
        assert_eq!(cross2(1.0, 0.0, 0.0, 1.0), 1.0);
        assert_eq!(cross2(1.0, 0.0, 1.0, 0.0), 0.0);
        assert_eq!(cross2(1.0, 1.0, 1.0, -1.0), -2.0);
    }

    #[test]
    fn test_length_functions() {
        let tol = ToleranceConfig::default();

        assert!(tol.almost_equals(length2(3.0, 4.0), 5.0));
        assert!(tol.almost_equals(length_squared2(3.0, 4.0), 25.0));

        if let Some((x, y)) = normalize2(3.0, 4.0, &tol) {
            assert!(tol.almost_equals(length2(x, y), 1.0));
        } else {
            panic!("normalize2 should return Some");
        }

        // 零向量
        assert!(normalize2(0.0, 0.0, &tol).is_none());
    }

    #[test]
    fn test_condition_number() {
        let tol = ToleranceConfig::default();

        // 良态系统
        let cond = ConditionNumber::from_ratio(100.0, 1.0, &tol);
        assert!(!cond.is_ill_conditioned);
        assert_eq!(cond.value, 100.0);

        // 病态系统
        let cond = ConditionNumber::from_ratio(1e10, 1e-10, &tol);
        assert!(cond.is_ill_conditioned);
        assert!(cond.value > 1e19);

        // 除数为零
        let cond = ConditionNumber::from_ratio(100.0, 0.0, &tol);
        assert!(cond.is_ill_conditioned);
        assert_eq!(cond.value, f64::INFINITY);
    }

    #[test]
    fn test_high_low_precision() {
        let high = ToleranceConfig::high_precision();
        let low = ToleranceConfig::low_precision();
        let default = ToleranceConfig::default();

        // 高精度应该更严格
        assert!(high.absolute < default.absolute);
        assert!(high.relative < default.relative);

        // 低精度应该更宽松
        assert!(low.absolute > default.absolute);
        assert!(low.relative > default.relative);
    }

    #[test]
    fn test_almost_less_greater() {
        let tol = ToleranceConfig::default();

        assert!(tol.almost_less(1.0, 2.0));
        assert!(!tol.almost_less(2.0, 1.0));
        assert!(!tol.almost_less(1.0, 1.0 + 1e-13)); // 在容差范围内

        assert!(tol.almost_greater(2.0, 1.0));
        assert!(!tol.almost_greater(1.0, 2.0));
        assert!(!tol.almost_greater(1.0 + 1e-13, 1.0)); // 在容差范围内
    }
}
