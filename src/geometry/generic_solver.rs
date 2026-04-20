//! 通用约束求解器 trait 和共享实现
//!
//! 提供 2D 和 3D 约束求解器的通用抽象，减少代码重复
//!
//! # 设计目标
//!
//! - 统一的求解器接口
//! - 共享的数值算法实现（Jacobian 计算、LM 算法等）
//! - 类型安全的泛型设计

use nalgebra::{DMatrix, DVector};
use std::collections::HashMap;
use tracing::{debug, info, instrument};

use super::constraint::{SolverDiagnostics, SolverError};
use super::numerics::ToleranceConfig;

/// 约束系统 trait
///
/// 定义了约束求解器所需的通用接口，2D 和 3D 系统都实现此 trait
pub trait ConstraintSystemTrait {
    /// 实体 ID 类型
    type EntityId: Copy + Clone + std::fmt::Debug + Eq + std::hash::Hash;

    /// 约束类型
    type Constraint;

    /// 获取自由度数量
    fn degrees_of_freedom(&self) -> usize;

    /// 获取约束方程总数
    fn total_equations(&self) -> usize;

    /// 获取所有变量
    fn get_variables(&self) -> Vec<f64>;

    /// 设置所有变量
    fn set_variables(&mut self, vars: &[f64]);

    /// 获取约束列表
    fn constraints(&self) -> &[Self::Constraint];

    /// 验证系统有效性
    fn validate(&self) -> Result<(), String>;
}

/// 约束 trait
///
/// 定义了单个约束的行为
pub trait ConstraintTrait {
    /// 关联的实体 ID 类型
    type EntityId: Copy + Clone + std::fmt::Debug + Eq + std::hash::Hash;

    /// 获取约束涉及的实体 ID
    fn get_entity_ids(&self) -> Vec<Self::EntityId>;

    /// 获取约束方程数量
    fn equation_count(&self) -> usize;
}

/// 通用约束求解器配置
#[derive(Debug, Clone)]
pub struct GenericSolverConfig {
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛容差
    pub tolerance: f64,
    /// 初始阻尼
    pub damping: f64,
    /// 阻尼因子（用于 LM 算法）
    pub damping_factor: f64,
    /// 最小阻尼
    pub min_damping: f64,
    /// 最大阻尼
    pub max_damping: f64,
    /// 线搜索参数
    pub line_search_c: f64,
    /// 线搜索最大迭代次数
    pub line_search_max_iter: usize,
    /// 使用 Levenberg-Marquardt 算法（否则使用 Newton-Raphson）
    pub use_lm: bool,
    /// 启用诊断信息
    pub enable_diagnostics: bool,
    /// 容差配置
    pub tolerance_config: ToleranceConfig,
}

impl Default for GenericSolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-8,
            damping: 1e-3,
            damping_factor: 2.0,
            min_damping: 1e-10,
            max_damping: 1e10,
            line_search_c: 0.5,
            line_search_max_iter: 20,
            use_lm: true,
            enable_diagnostics: false,
            tolerance_config: ToleranceConfig::default(),
        }
    }
}

/// 通用约束求解器
///
/// 实现了 2D 和 3D 约束求解的共享算法逻辑
pub struct GenericConstraintSolver<Sys: ConstraintSystemTrait> {
    config: GenericSolverConfig,
    _marker: std::marker::PhantomData<Sys>,
}

impl<Sys: ConstraintSystemTrait> GenericConstraintSolver<Sys> {
    /// 创建新的求解器
    pub fn new(config: GenericSolverConfig) -> Self {
        Self {
            config,
            _marker: std::marker::PhantomData,
        }
    }

    /// 使用默认配置创建求解器
    pub fn with_defaults() -> Self {
        Self::new(GenericSolverConfig::default())
    }

    /// 求解约束系统
    #[instrument(
        skip(self, system),
        fields(iterations = 0, initial_residual = 0.0, final_residual = 0.0)
    )]
    pub fn solve(&self, system: &mut Sys) -> Result<SolverDiagnostics, SolverError> {
        // 验证系统
        if let Err(e) = system.validate() {
            return Err(SolverError::InvalidInput { message: e });
        }

        // 检查空系统
        if system.total_equations() == 0 {
            let mut diagnostics = SolverDiagnostics::new();
            diagnostics.accepted = true;
            diagnostics.convergence_reason = Some("No constraints to solve".to_string());
            return Ok(diagnostics);
        }

        // 获取初始值
        let mut x = system.get_variables();

        // 记录初始残差
        let initial_residual = self.compute_residual_norm(system, &x);
        debug!(initial_residual = %initial_residual, "Starting constraint solve");

        // 创建诊断对象
        let mut diagnostics = if self.config.enable_diagnostics {
            SolverDiagnostics::new()
        } else {
            SolverDiagnostics::default()
        };

        // 选择求解方法
        let result = if self.config.use_lm {
            self.solve_lm(system, &mut x, &mut diagnostics)
        } else {
            self.solve_newton(system, &mut x, &mut diagnostics)
        };

        // 记录最终结果
        if result.is_ok() {
            let final_residual = self.compute_residual_norm(system, &x);
            let iterations = diagnostics.residual_history.len();

            info!(
                iterations = %iterations,
                initial_residual = %initial_residual,
                final_residual = %final_residual,
                accepted = %diagnostics.accepted,
                "Constraint solve completed"
            );

            // 更新系统状态
            system.set_variables(&x);

            // 记录到 tracing span
            let span = tracing::Span::current();
            span.record("iterations", iterations);
            span.record("initial_residual", initial_residual);
            span.record("final_residual", final_residual);
        }

        result
    }

    /// Newton-Raphson 方法求解
    fn solve_newton(
        &self,
        system: &Sys,
        x: &mut Vec<f64>,
        diagnostics: &mut SolverDiagnostics,
    ) -> Result<SolverDiagnostics, SolverError> {
        let n_vars = x.len();
        let tol = &self.config.tolerance_config;

        for iteration in 0..self.config.max_iterations {
            // 计算残差
            let f = self.compute_residual_vector(system, x);
            let residual_norm = f.norm();

            // 计算 Jacobian
            let j = self.compute_jacobian(system, x);

            // 求解线性方程组 J * dx = -F
            // 将矩阵转换为 Vec<f64> 用于求解
            let j_vec: Vec<f64> = j.as_slice().to_vec();
            let f_vec: Vec<f64> = f.as_slice().to_vec();
            let dx = self.solve_linear_system(&j_vec, &f_vec, n_vars)?;
            let dx_vec = DVector::from_vec(dx.clone());
            let step_norm = dx_vec.norm();

            // 记录诊断信息
            if self.config.enable_diagnostics {
                diagnostics.record_iteration(residual_norm, step_norm, 0.0, 1.0);
            }

            // 检查收敛
            if residual_norm < tol.absolute {
                diagnostics.accepted = true;
                diagnostics.convergence_reason = Some(format!(
                    "Converged: residual {} < tolerance {}",
                    residual_norm, tol.absolute
                ));
                return Ok(diagnostics.clone());
            }

            // 线搜索
            let alpha = self.line_search(system, x, &dx);
            *x = x
                .iter()
                .zip(dx.iter())
                .map(|(&xi, &dxi)| xi + alpha * dxi)
                .collect();

            // 检查是否停滞
            if step_norm < tol.absolute {
                diagnostics.accepted = residual_norm < tol.absolute * 10.0;
                diagnostics.convergence_reason =
                    Some(format!("Step too small: {} < {}", step_norm, tol.absolute));
                if diagnostics.accepted {
                    return Ok(diagnostics.clone());
                }
                return Err(SolverError::NotConverged {
                    iterations: iteration,
                    residual: residual_norm,
                });
            }
        }

        let final_residual = self.compute_residual_norm(system, x);
        diagnostics.accepted = false;
        diagnostics.convergence_reason = Some(format!(
            "Max iterations ({}) reached",
            self.config.max_iterations
        ));
        Err(SolverError::NotConverged {
            iterations: self.config.max_iterations,
            residual: final_residual,
        })
    }

    /// Levenberg-Marquardt 方法求解
    fn solve_lm(
        &self,
        system: &Sys,
        x: &mut Vec<f64>,
        diagnostics: &mut SolverDiagnostics,
    ) -> Result<SolverDiagnostics, SolverError> {
        let n_vars = x.len();
        let mut damping = self.config.damping;
        let tol = &self.config.tolerance_config;

        let mut residual = self.compute_residual_norm(system, x);

        for iteration in 0..self.config.max_iterations {
            // 计算 Jacobian
            let jacobian = self.compute_jacobian(system, x);

            // 计算残差向量
            let residuals = self.compute_residual_vector(system, x);

            // 构建法方程：(J^T * J + damping * I) * dx = -J^T * r
            let jtj = self.compute_jtj(&jacobian, n_vars);
            let residuals_vec: Vec<f64> = residuals.as_slice().to_vec();
            let jtr = self.compute_jtr(&jacobian, &residuals_vec, n_vars);

            // 添加阻尼
            let mut augmented = jtj;
            for i in 0..n_vars {
                augmented[i * n_vars + i] += damping;
            }

            // 求解线性方程组
            let dx = self.solve_linear_system(&augmented, &jtr, n_vars)?;
            let dx_norm = DVector::from_vec(dx.clone()).norm();

            // 更新解
            let x_new: Vec<f64> = x
                .iter()
                .zip(dx.iter())
                .map(|(&xi, &dxi)| xi - dxi)
                .collect();

            // 计算新残差
            let residual_new = self.compute_residual_norm(system, &x_new);

            // 检查收敛
            if residual_new < tol.absolute {
                *x = x_new;
                diagnostics.accepted = true;
                diagnostics.convergence_reason = Some(format!(
                    "LM converged: residual {} < tolerance {}",
                    residual_new, tol.absolute
                ));

                if self.config.enable_diagnostics {
                    diagnostics.record_iteration(residual_new, dx_norm, damping, 1.0);
                }

                return Ok(diagnostics.clone());
            }

            // 记录诊断信息
            if self.config.enable_diagnostics {
                diagnostics.record_iteration(residual_new, dx_norm, damping, 1.0);
            }

            // 接受或拒绝更新
            if residual_new < residual {
                *x = x_new;
                residual = residual_new;
                damping /= self.config.damping_factor;
            } else {
                damping *= self.config.damping_factor;
            }

            // 限制阻尼范围
            damping = damping.clamp(self.config.min_damping, self.config.max_damping);

            if iteration % 10 == 0 {
                debug!(
                    "LM iteration {}: residual={}, damping={}",
                    iteration, residual, damping
                );
            }
        }

        diagnostics.accepted = false;
        diagnostics.convergence_reason = Some(format!(
            "LM max iterations ({}) reached",
            self.config.max_iterations
        ));
        Err(SolverError::NotConverged {
            iterations: self.config.max_iterations,
            residual,
        })
    }

    /// 计算残差范数
    fn compute_residual_norm(&self, system: &Sys, x: &[f64]) -> f64 {
        let residuals = self.compute_residual_vector(system, x);
        residuals.iter().map(|r| r * r).sum::<f64>().sqrt()
    }

    /// 计算残差向量（由具体系统实现）
    fn compute_residual_vector(&self, system: &Sys, x: &[f64]) -> DVector<f64> {
        // 默认实现：调用 trait 方法
        let residuals: Vec<f64> = self.compute_residuals_generic(system, x);
        DVector::from_vec(residuals)
    }

    /// 通用残差计算方法
    fn compute_residuals_generic(&self, system: &Sys, x: &[f64]) -> Vec<f64> {
        // 这个方法需要具体系统实现
        // 这里提供一个基于 HashMap 的通用实现框架
        let mut residuals = Vec::new();

        // 构建实体参数索引映射
        let mut idx = 0;
        let mut entity_params: HashMap<Sys::EntityId, Vec<f64>> = HashMap::new();

        // 注意：这里需要具体系统提供实体迭代接口
        // 由于 trait 限制，这个方法需要由具体实现覆盖
        let _ = (system, x, &mut residuals, &mut entity_params, &mut idx);

        residuals
    }

    /// 计算 Jacobian 矩阵（使用有限差分法）
    fn compute_jacobian(&self, system: &Sys, x: &[f64]) -> DMatrix<f64> {
        let n_vars = x.len();
        let n_eqs = system.total_equations();
        let epsilon = 1e-8;

        let mut jacobian = DMatrix::zeros(n_eqs, n_vars);
        let _f0 = self.compute_residual_vector(system, x);

        for j in 0..n_vars {
            // 中心差分
            let mut x_plus = x.to_vec();
            x_plus[j] += epsilon;
            let f_plus = self.compute_residual_vector(system, &x_plus);

            let mut x_minus = x.to_vec();
            x_minus[j] -= epsilon;
            let f_minus = self.compute_residual_vector(system, &x_minus);

            for i in 0..n_eqs {
                jacobian[(i, j)] = (f_plus[i] - f_minus[i]) / (2.0 * epsilon);
            }
        }

        jacobian
    }

    /// 计算 J^T * J
    fn compute_jtj(&self, jacobian: &DMatrix<f64>, n_vars: usize) -> Vec<f64> {
        let n_eqs = jacobian.nrows();
        let mut jtj = vec![0.0; n_vars * n_vars];

        for i in 0..n_vars {
            for j in 0..n_vars {
                let mut sum = 0.0;
                for k in 0..n_eqs {
                    sum += jacobian[(k, i)] * jacobian[(k, j)];
                }
                jtj[i * n_vars + j] = sum;
            }
        }

        jtj
    }

    /// 计算 J^T * r
    fn compute_jtr(&self, jacobian: &DMatrix<f64>, residuals: &[f64], n_vars: usize) -> Vec<f64> {
        let n_eqs = residuals.len();
        let mut jtr = vec![0.0; n_vars];

        for i in 0..n_vars {
            let mut sum = 0.0;
            for j in 0..n_eqs {
                sum += jacobian[(j, i)] * residuals[j];
            }
            jtr[i] = sum;
        }

        jtr
    }

    /// 求解线性方程组 Ax = b（使用高斯消元法）
    fn solve_linear_system(&self, a: &[f64], b: &[f64], n: usize) -> Result<Vec<f64>, SolverError> {
        // 创建增广矩阵
        let mut augmented = vec![vec![0.0; n + 1]; n];
        for i in 0..n {
            for j in 0..n {
                augmented[i][j] = a[i * n + j];
            }
            augmented[i][n] = b[i];
        }

        // 高斯消元
        for i in 0..n {
            // 查找主元
            let mut max_row = i;
            for k in (i + 1)..n {
                if augmented[k][i].abs() > augmented[max_row][i].abs() {
                    max_row = k;
                }
            }

            if augmented[max_row][i].abs() < 1e-12 {
                return Err(SolverError::SingularMatrix);
            }

            // 交换行
            augmented.swap(i, max_row);

            // 消元
            for k in (i + 1)..n {
                let factor = augmented[k][i] / augmented[i][i];
                #[allow(clippy::needless_range_loop)]
                for j in i..=n {
                    augmented[k][j] -= factor * augmented[i][j];
                }
            }
        }

        // 回代
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            let mut sum = augmented[i][n];
            for j in (i + 1)..n {
                sum -= augmented[i][j] * x[j];
            }
            x[i] = sum / augmented[i][i];
        }

        Ok(x)
    }

    /// 线搜索：找到合适的步长
    fn line_search(&self, system: &Sys, x: &[f64], dx: &[f64]) -> f64 {
        let mut alpha = 1.0;
        let c = self.config.line_search_c;

        let f0 = self.compute_residual_norm(system, x);

        for _ in 0..self.config.line_search_max_iter {
            let x_new: Vec<f64> = x
                .iter()
                .zip(dx.iter())
                .map(|(&xi, &dxi)| xi + alpha * dxi)
                .collect();
            let f_new = self.compute_residual_norm(system, &x_new);

            if f_new < f0 {
                return alpha;
            }

            alpha *= c;
        }

        alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 Mock 约束系统
    struct MockConstraintSystem {
        entities: Vec<Vec<f64>>,
        constraints: Vec<MockConstraint>,
    }

    struct MockConstraint {
        #[allow(dead_code)]
        entity_ids: Vec<usize>,
        equation_count: usize,
    }

    impl ConstraintSystemTrait for MockConstraintSystem {
        type EntityId = usize;
        type Constraint = MockConstraint;

        fn degrees_of_freedom(&self) -> usize {
            self.entities.iter().map(|e| e.len()).sum()
        }

        fn total_equations(&self) -> usize {
            self.constraints.iter().map(|c| c.equation_count).sum()
        }

        fn get_variables(&self) -> Vec<f64> {
            self.entities.iter().flatten().copied().collect()
        }

        fn set_variables(&mut self, vars: &[f64]) {
            let mut idx = 0;
            for entity in &mut self.entities {
                let count = entity.len();
                entity.copy_from_slice(&vars[idx..idx + count]);
                idx += count;
            }
        }

        fn constraints(&self) -> &[Self::Constraint] {
            &self.constraints
        }

        fn validate(&self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn test_generic_solver_config() {
        let config = GenericSolverConfig::default();
        assert_eq!(config.max_iterations, 100);
        assert_eq!(config.tolerance, 1e-8);
        assert!(config.use_lm);
    }

    #[test]
    fn test_generic_solver_creation() {
        let solver = GenericConstraintSolver::<MockConstraintSystem>::with_defaults();
        assert!(solver.config.use_lm);
    }
}
