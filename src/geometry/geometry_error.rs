//! 几何错误类型定义
//!
//! 提供详细的几何错误类型，支持错误来源追踪和结构化错误信息

use serde::{Deserialize, Serialize};
use std::fmt;

/// 几何错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeometryError {
    /// 无效坐标（NaN、Infinity 或超出范围）
    InvalidCoordinate {
        entity: String,
        coordinate: String,
        value: f64,
        reason: String,
    },

    /// 无效几何参数（如零长度线段、零半径圆）
    InvalidParameter {
        entity: String,
        parameter: String,
        value: f64,
        constraint: String,
    },

    /// 拓扑错误（如无法形成闭合回路）
    TopologyError { operation: String, reason: String },

    /// 布尔运算错误
    BooleanError {
        operation: String,
        operand1: String,
        operand2: String,
        reason: String,
    },

    /// 数值计算错误（如除零、溢出）
    NumericalError { operation: String, reason: String },

    /// 容差配置错误
    ToleranceError {
        parameter: String,
        value: f64,
        reason: String,
    },
}

impl GeometryError {
    /// 创建无效坐标错误
    pub fn invalid_coordinate(
        entity: impl Into<String>,
        coordinate: impl Into<String>,
        value: f64,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidCoordinate {
            entity: entity.into(),
            coordinate: coordinate.into(),
            value,
            reason: reason.into(),
        }
    }

    /// 创建无效参数错误
    pub fn invalid_parameter(
        entity: impl Into<String>,
        parameter: impl Into<String>,
        value: f64,
        constraint: impl Into<String>,
    ) -> Self {
        Self::InvalidParameter {
            entity: entity.into(),
            parameter: parameter.into(),
            value,
            constraint: constraint.into(),
        }
    }

    /// 创建拓扑错误
    pub fn topology(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::TopologyError {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// 创建布尔运算错误
    pub fn boolean(
        operation: impl Into<String>,
        operand1: impl Into<String>,
        operand2: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::BooleanError {
            operation: operation.into(),
            operand1: operand1.into(),
            operand2: operand2.into(),
            reason: reason.into(),
        }
    }

    /// 创建数值错误
    pub fn numerical(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::NumericalError {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// 创建容差错误
    pub fn tolerance(parameter: impl Into<String>, value: f64, reason: impl Into<String>) -> Self {
        Self::ToleranceError {
            parameter: parameter.into(),
            value,
            reason: reason.into(),
        }
    }

    /// 获取错误类别
    pub fn category(&self) -> &'static str {
        match self {
            Self::InvalidCoordinate { .. } => "InvalidCoordinate",
            Self::InvalidParameter { .. } => "InvalidParameter",
            Self::TopologyError { .. } => "Topology",
            Self::BooleanError { .. } => "Boolean",
            Self::NumericalError { .. } => "Numerical",
            Self::ToleranceError { .. } => "Tolerance",
        }
    }
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCoordinate {
                entity,
                coordinate,
                value,
                reason,
            } => write!(f, "无效的 {entity} 坐标 {coordinate}: {value} ({reason})"),
            Self::InvalidParameter {
                entity,
                parameter,
                value,
                constraint,
            } => write!(
                f,
                "无效的 {entity} 参数 {parameter}: {value} (约束：{constraint})"
            ),
            Self::TopologyError { operation, reason } => {
                write!(f, "拓扑错误：{operation} 失败 - {reason}")
            }
            Self::BooleanError {
                operation,
                operand1,
                operand2,
                reason,
            } => write!(
                f,
                "布尔运算错误：{operation}({operand1},{operand2}) 失败 - {reason}"
            ),
            Self::NumericalError { operation, reason } => {
                write!(f, "数值错误：{operation} - {reason}")
            }
            Self::ToleranceError {
                parameter,
                value,
                reason,
            } => write!(f, "容差错误：{parameter}={value} - {reason}"),
        }
    }
}

impl std::error::Error for GeometryError {}

/// 几何结果类型别名
pub type GeometryResult<T> = Result<T, GeometryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = GeometryError::invalid_coordinate("Point", "x", f64::NAN, "坐标不能为 NaN");
        assert!(err.to_string().contains("无效的 Point 坐标 x"));
        assert!(err.to_string().contains("NaN"));
    }

    #[test]
    fn test_error_category() {
        let err = GeometryError::invalid_parameter("Line", "length", 0.0, "长度必须大于 0");
        assert_eq!(err.category(), "InvalidParameter");
    }

    #[test]
    fn test_result_alias() {
        let result: GeometryResult<()> = Ok(());
        assert!(result.is_ok());
    }
}
