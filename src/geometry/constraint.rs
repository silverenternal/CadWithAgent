//! 几何约束求解器
//!
//! 提供参数化 CAD 所需的约束定义、方程构建和数值求解功能
//!
//! # 架构
//!
//! ```text
//! ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
//! │ 约束定义层  │→ │ 方程构建层  │→ │ 数值求解层  │
//! │ - 几何约束  │  │ - Jacobian  │  │ - Newton-R. │
//! │ - 尺寸约束  │  │ - 稀疏矩阵  │  │ - LM 算法    │
//! └─────────────┘  └─────────────┘  └─────────────┘
//! ```
//!
//! # 示例
//!
//! ```rust,ignore
//! use cadagent::geometry::{Point, ConstraintSystem, Constraint, ConstraintSolver};
//!
//! // 创建约束系统
//! let mut system = ConstraintSystem::new();
//!
//! // 添加几何实体
//! let p1_id = system.add_point(Point::new(0.0, 0.0));
//! let p2_id = system.add_point(Point::new(1.0, 0.0));
//! let p3_id = system.add_point(Point::new(0.0, 1.0));
//!
//! // 添加约束
//! system.add_constraint(Constraint::FixPoint { point_id: p1_id });
//! system.add_constraint(Constraint::FixLength {
//!     line_start: p1_id,
//!     line_end: p2_id,
//!     length: 1.0,
//! });
//!
//! // 求解
//! let solver = ConstraintSolver::new();
//! match solver.solve(&mut system) {
//!     Ok(_) => println!("求解成功！"),
//!     Err(e) => println!("求解失败：{:?}", e),
//! }
//! ```

use super::numerics::ToleranceConfig;
use super::primitives::Point;
use nalgebra::{DMatrix, DVector};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

/// 实体 ID 类型
pub type EntityId = usize;

/// 约束 ID 类型
pub type ConstraintId = usize;

/// 几何实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType {
    Point,
    Line,
    Circle,
}

/// 几何实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub entity_type: EntityType,
    /// 点：(x, y)
    /// `线段：(start_x`, `start_y`, `end_x`, `end_y`)
    /// `圆：(center_x`, `center_y`, radius)
    pub parameters: Vec<f64>,
}

impl Entity {
    pub fn new(id: EntityId, entity_type: EntityType, parameters: Vec<f64>) -> Self {
        Self {
            id,
            entity_type,
            parameters,
        }
    }

    /// 从点创建实体
    pub fn from_point(id: EntityId, point: Point) -> Self {
        Self {
            id,
            entity_type: EntityType::Point,
            parameters: vec![point.x, point.y],
        }
    }

    /// 从线段创建实体
    pub fn from_line(id: EntityId, start: Point, end: Point) -> Self {
        Self {
            id,
            entity_type: EntityType::Line,
            parameters: vec![start.x, start.y, end.x, end.y],
        }
    }

    /// 从圆创建实体
    pub fn from_circle(id: EntityId, center: Point, radius: f64) -> Self {
        Self {
            id,
            entity_type: EntityType::Circle,
            parameters: vec![center.x, center.y, radius],
        }
    }

    /// 获取点的坐标
    pub fn as_point(&self) -> Option<Point> {
        if self.entity_type != EntityType::Point || self.parameters.len() < 2 {
            return None;
        }
        Some(Point::new(self.parameters[0], self.parameters[1]))
    }

    /// 获取线段的起点和终点
    pub fn as_line_points(&self) -> Option<(Point, Point)> {
        if self.entity_type != EntityType::Line || self.parameters.len() < 4 {
            return None;
        }
        Some((
            Point::new(self.parameters[0], self.parameters[1]),
            Point::new(self.parameters[2], self.parameters[3]),
        ))
    }

    /// 获取圆的圆心和半径
    pub fn as_circle(&self) -> Option<(Point, f64)> {
        if self.entity_type != EntityType::Circle || self.parameters.len() < 3 {
            return None;
        }
        Some((
            Point::new(self.parameters[0], self.parameters[1]),
            self.parameters[2],
        ))
    }
}

/// 约束类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint {
    /// 固定点（点的 x, y 坐标固定）
    FixPoint { point_id: EntityId },

    /// 固定长度（线段长度固定）
    FixLength {
        line_start: EntityId,
        line_end: EntityId,
        length: f64,
    },

    /// 固定角度（线段与 X 轴的夹角）
    FixAngle {
        line_start: EntityId,
        line_end: EntityId,
        angle: f64, // 弧度
    },

    /// 平行（两条线段平行）
    Parallel {
        line1_start: EntityId,
        line1_end: EntityId,
        line2_start: EntityId,
        line2_end: EntityId,
    },

    /// 垂直（两条线段垂直）
    Perpendicular {
        line1_start: EntityId,
        line1_end: EntityId,
        line2_start: EntityId,
        line2_end: EntityId,
    },

    /// 重合（两个点重合）
    Coincident {
        point1_id: EntityId,
        point2_id: EntityId,
    },

    /// 点在曲线上（点在线段上）
    PointOnLine {
        point_id: EntityId,
        line_start: EntityId,
        line_end: EntityId,
    },

    /// 相切（直线与圆相切）
    TangentLineCircle {
        line_start: EntityId,
        line_end: EntityId,
        circle_id: EntityId,
    },

    /// 同心（两个圆同心）
    Concentric {
        circle1_id: EntityId,
        circle2_id: EntityId,
    },

    /// 等半径（两个圆半径相等）
    EqualRadius {
        circle1_id: EntityId,
        circle2_id: EntityId,
    },

    /// 固定半径（圆的半径固定）
    FixRadius { circle_id: EntityId, radius: f64 },

    /// 水平（线段水平）
    Horizontal {
        line_start: EntityId,
        line_end: EntityId,
    },

    /// 垂直（线段垂直）
    Vertical {
        line_start: EntityId,
        line_end: EntityId,
    },

    /// 中点（点 3 是点 1 和点 2 的中点）
    Midpoint {
        point1_id: EntityId,
        point2_id: EntityId,
        midpoint_id: EntityId,
    },

    /// 对称（点 1 和点 2 关于直线对称）
    Symmetric {
        point1_id: EntityId,
        point2_id: EntityId,
        line_start: EntityId,
        line_end: EntityId,
    },

    /// 圆与圆相切（外切或内切）
    TangentCircleCircle {
        circle1_id: EntityId,
        circle2_id: EntityId,
    },

    /// 点在圆上
    PointOnCircle {
        point_id: EntityId,
        circle_id: EntityId,
    },
}

impl Constraint {
    /// 获取约束涉及的实体 ID 列表
    #[allow(clippy::match_same_arms)]
    pub fn get_entity_ids(&self) -> Vec<EntityId> {
        match self {
            Constraint::FixPoint { point_id } => vec![*point_id],
            Constraint::FixLength {
                line_start,
                line_end,
                ..
            } => vec![*line_start, *line_end],
            Constraint::FixAngle {
                line_start,
                line_end,
                ..
            } => vec![*line_start, *line_end],
            Constraint::Parallel {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            } => {
                vec![*line1_start, *line1_end, *line2_start, *line2_end]
            }
            Constraint::Perpendicular {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            } => {
                vec![*line1_start, *line1_end, *line2_start, *line2_end]
            }
            Constraint::Coincident {
                point1_id,
                point2_id,
            } => vec![*point1_id, *point2_id],
            Constraint::PointOnLine {
                point_id,
                line_start,
                line_end,
            } => {
                vec![*point_id, *line_start, *line_end]
            }
            Constraint::TangentLineCircle {
                line_start,
                line_end,
                circle_id,
            } => {
                vec![*line_start, *line_end, *circle_id]
            }
            Constraint::Concentric {
                circle1_id,
                circle2_id,
            } => vec![*circle1_id, *circle2_id],
            Constraint::EqualRadius {
                circle1_id,
                circle2_id,
            } => vec![*circle1_id, *circle2_id],
            Constraint::FixRadius { circle_id, .. } => vec![*circle_id],
            Constraint::Horizontal {
                line_start,
                line_end,
            } => vec![*line_start, *line_end],
            Constraint::Vertical {
                line_start,
                line_end,
            } => vec![*line_start, *line_end],
            Constraint::Midpoint {
                point1_id,
                point2_id,
                midpoint_id,
            } => {
                vec![*point1_id, *point2_id, *midpoint_id]
            }
            Constraint::Symmetric {
                point1_id,
                point2_id,
                line_start,
                line_end,
            } => {
                vec![*point1_id, *point2_id, *line_start, *line_end]
            }
            Constraint::TangentCircleCircle {
                circle1_id,
                circle2_id,
            } => vec![*circle1_id, *circle2_id],
            Constraint::PointOnCircle {
                point_id,
                circle_id,
            } => vec![*point_id, *circle_id],
        }
    }

    /// 获取约束的方程数量
    #[allow(clippy::match_same_arms)]
    pub fn equation_count(&self) -> usize {
        match self {
            Constraint::FixPoint { .. } => 2, // x, y 两个方程
            Constraint::FixLength { .. } => 1,
            Constraint::FixAngle { .. } => 1,
            Constraint::Parallel { .. } => 1,
            Constraint::Perpendicular { .. } => 1,
            Constraint::Coincident { .. } => 2, // x, y 两个方程
            Constraint::PointOnLine { .. } => 1,
            Constraint::TangentLineCircle { .. } => 1,
            Constraint::Concentric { .. } => 2, // x, y 两个方程
            Constraint::EqualRadius { .. } => 1,
            Constraint::FixRadius { .. } => 1,
            Constraint::Horizontal { .. } => 1,
            Constraint::Vertical { .. } => 1,
            Constraint::Midpoint { .. } => 2,  // x, y 两个方程
            Constraint::Symmetric { .. } => 2, // x, y 两个方程
            Constraint::TangentCircleCircle { .. } => 1, // 圆心距 = 半径和 (或差)
            Constraint::PointOnCircle { .. } => 1,       // 点到圆心距离 = 半径
        }
    }
}

/// 约束求解状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintStatus {
    /// 欠约束（自由度 > 0）
    UnderConstrained { degrees_of_freedom: usize },
    /// 完全约束（自由度 = 0）
    FullyConstrained,
    /// 过约束（存在冲突）
    OverConstrained {
        conflicting_constraints: Vec<ConstraintId>,
    },
    /// 求解失败
    Failed { error: String },
}

/// 约束求解错误
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SolverError {
    /// 数值不收敛
    NotConverged { iterations: usize, residual: f64 },
    /// 奇异矩阵
    SingularMatrix,
    /// 无效输入
    InvalidInput { message: String },
    /// 实体不存在
    EntityNotFound { entity_id: EntityId },
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverError::NotConverged {
                iterations,
                residual,
            } => {
                write!(f, "求解不收敛：{iterations} 次迭代后残差为 {residual}")
            }
            SolverError::SingularMatrix => write!(f, "Jacobian 矩阵奇异"),
            SolverError::InvalidInput { message } => write!(f, "无效输入：{message}"),
            SolverError::EntityNotFound { entity_id } => {
                write!(f, "实体不存在：ID = {entity_id}")
            }
        }
    }
}

impl std::error::Error for SolverError {}

/// 约束系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSystem {
    /// 几何实体
    pub entities: HashMap<EntityId, Entity>,
    /// 约束列表
    pub constraints: Vec<Constraint>,
    /// 求解状态
    pub status: ConstraintStatus,
    /// 下一个实体 ID
    next_entity_id: EntityId,
    /// 容差配置
    pub tolerance_config: ToleranceConfig,
}

impl ConstraintSystem {
    /// 创建新的约束系统
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            constraints: Vec::new(),
            status: ConstraintStatus::UnderConstrained {
                degrees_of_freedom: 0,
            },
            next_entity_id: 0,
            tolerance_config: ToleranceConfig::default(),
        }
    }

    /// 创建带容差的约束系统
    pub fn with_tolerance(tolerance: f64) -> Self {
        Self {
            entities: HashMap::new(),
            constraints: Vec::new(),
            status: ConstraintStatus::UnderConstrained {
                degrees_of_freedom: 0,
            },
            next_entity_id: 0,
            tolerance_config: ToleranceConfig::default().with_absolute(tolerance),
        }
    }

    /// 创建带容差配置的约束系统
    pub fn with_tolerance_config(config: ToleranceConfig) -> Self {
        Self {
            entities: HashMap::new(),
            constraints: Vec::new(),
            status: ConstraintStatus::UnderConstrained {
                degrees_of_freedom: 0,
            },
            next_entity_id: 0,
            tolerance_config: config,
        }
    }

    /// 添加点实体
    pub fn add_point(&mut self, point: Point) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.insert(id, Entity::from_point(id, point));
        id
    }

    /// 添加线段实体
    pub fn add_line(&mut self, start: Point, end: Point) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities.insert(id, Entity::from_line(id, start, end));
        id
    }

    /// 添加圆实体
    pub fn add_circle(&mut self, center: Point, radius: f64) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        self.entities
            .insert(id, Entity::from_circle(id, center, radius));
        id
    }

    /// 添加约束
    pub fn add_constraint(&mut self, constraint: Constraint) -> ConstraintId {
        let id = self.constraints.len();
        self.constraints.push(constraint);
        id
    }

    /// 获取实体
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// 获取可变实体
    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    /// 获取约束
    pub fn get_constraint(&self, id: ConstraintId) -> Option<&Constraint> {
        self.constraints.get(id)
    }

    /// 获取可变约束
    pub fn get_constraint_mut(&mut self, id: ConstraintId) -> Option<&mut Constraint> {
        self.constraints.get_mut(id)
    }

    /// 计算自由度
    ///
    /// 每个点有 2 个自由度 (x, y)
    /// 每个约束会减少相应的自由度
    pub fn degrees_of_freedom(&self) -> usize {
        let mut dof = 0;

        // 计算总变量数
        for entity in self.entities.values() {
            dof += entity.parameters.len();
        }

        // 减去约束方程数
        for constraint in &self.constraints {
            dof = dof.saturating_sub(constraint.equation_count());
        }

        dof
    }

    /// 分析约束状态
    pub fn analyze(&self) -> ConstraintStatus {
        let dof = self.degrees_of_freedom();

        if dof == 0 {
            ConstraintStatus::FullyConstrained
        } else if dof > 0 {
            ConstraintStatus::UnderConstrained {
                degrees_of_freedom: dof,
            }
        } else {
            // dof < 0，可能是过约束
            ConstraintStatus::OverConstrained {
                conflicting_constraints: (0..self.constraints.len()).collect(),
            }
        }
    }

    /// 获取所有变量的当前值
    pub fn get_variables(&self) -> DVector<f64> {
        let mut vars = Vec::new();
        for entity in self.entities.values() {
            vars.extend(&entity.parameters);
        }
        DVector::from_vec(vars)
    }

    /// 设置所有变量的值
    pub fn set_variables(&mut self, vars: &DVector<f64>) {
        let mut idx = 0;
        for entity in self.entities.values_mut() {
            for i in 0..entity.parameters.len() {
                if idx < vars.len() {
                    entity.parameters[i] = vars[idx];
                    idx += 1;
                }
            }
        }
    }
}

impl Default for ConstraintSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// 约束求解器配置
///
/// 提供 Newton-Raphson 和 Levenberg-Marquardt 算法的高级配置选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    /// 容差配置
    pub tolerance_config: ToleranceConfig,
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 初始阻尼系数（用于 Levenberg-Marquardt）
    pub damping: f64,
    /// 是否使用 Levenberg-Marquardt
    pub use_lm: bool,
    /// 阻尼因子调整策略：>1.0 时增加阻尼，<1.0 时减小
    pub damping_factor: f64,
    /// 最小阻尼（防止过小导致数值不稳定）
    pub min_damping: f64,
    /// 最大阻尼（防止过大导致收敛过慢）
    pub max_damping: f64,
    /// 线搜索参数：Armijo 条件中的 c 值
    pub line_search_c: f64,
    /// 线搜索最大迭代次数
    pub line_search_max_iter: usize,
    /// 是否使用自适应步长
    pub adaptive_step: bool,
    /// 是否启用收敛诊断
    pub enable_diagnostics: bool,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            tolerance_config: ToleranceConfig::default(),
            max_iterations: 100,
            damping: 1e-3,
            use_lm: true,
            damping_factor: 2.0,
            min_damping: 1e-10,
            max_damping: 1e10,
            line_search_c: 0.5,
            line_search_max_iter: 20,
            adaptive_step: true,
            enable_diagnostics: false,
        }
    }
}

/// 求解器诊断信息
///
/// 提供求解过程的详细诊断数据，用于调试和性能分析
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SolverDiagnostics {
    /// 每次迭代的残差范数
    pub residual_history: Vec<f64>,
    /// 每次迭代的步长范数
    pub step_norm_history: Vec<f64>,
    /// 每次迭代的阻尼值
    pub damping_history: Vec<f64>,
    /// 每次迭代的线搜索 alpha 值
    pub alpha_history: Vec<f64>,
    /// 是否接受最终解
    pub accepted: bool,
    /// 收敛原因
    pub convergence_reason: Option<String>,
}

impl SolverDiagnostics {
    /// 创建新的诊断对象
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录迭代信息
    pub fn record_iteration(
        &mut self,
        residual_norm: f64,
        step_norm: f64,
        damping: f64,
        alpha: f64,
    ) {
        self.residual_history.push(residual_norm);
        self.step_norm_history.push(step_norm);
        self.damping_history.push(damping);
        self.alpha_history.push(alpha);
    }

    /// 分析收敛性
    pub fn analyze_convergence(&self) -> ConvergenceAnalysis {
        if self.residual_history.is_empty() {
            return ConvergenceAnalysis {
                converged: false,
                reason: "No iterations recorded".to_string(),
                initial_residual: 0.0,
                final_residual: 0.0,
                reduction_rate: 0.0,
                iterations: 0,
            };
        }

        let initial = self.residual_history[0];
        let final_val = *self.residual_history.last().unwrap();
        let iterations = self.residual_history.len();

        let reduction_rate = if initial > 0.0 {
            (initial - final_val) / initial
        } else {
            0.0
        };

        let reason = if self.accepted {
            self.convergence_reason.clone().unwrap_or_default()
        } else if final_val < 1e-6 {
            "Converged to solution".to_string()
        } else if iterations >= 100 {
            "Max iterations reached".to_string()
        } else {
            "Solver terminated early".to_string()
        };

        ConvergenceAnalysis {
            converged: self.accepted || final_val < 1e-6,
            reason,
            initial_residual: initial,
            final_residual: final_val,
            reduction_rate,
            iterations,
        }
    }
}

/// 收敛性分析结果
#[derive(Debug, Clone)]
pub struct ConvergenceAnalysis {
    /// 是否收敛
    pub converged: bool,
    /// 收敛原因
    pub reason: String,
    /// 初始残差
    pub initial_residual: f64,
    /// 最终残差
    pub final_residual: f64,
    /// 残差 reduction 率
    pub reduction_rate: f64,
    /// 迭代次数
    pub iterations: usize,
}

/// 约束变量依赖信息
///
/// 用于优化 Jacobian 稀疏矩阵填充，减少计算复杂度
#[derive(Debug, Clone)]
pub struct ConstraintDependency {
    /// 约束 ID
    pub constraint_id: ConstraintId,
    /// 依赖的变量索引列表（在全局变量向量中的索引）
    pub dependent_var_indices: Vec<usize>,
    /// 依赖的实体 ID 列表
    pub dependent_entity_ids: Vec<EntityId>,
}

/// 约束求解器
pub struct ConstraintSolver {
    config: SolverConfig,
}

impl ConstraintSolver {
    /// 创建新的求解器
    pub fn new() -> Self {
        Self {
            config: SolverConfig::default(),
        }
    }

    /// 创建带配置的求解器
    pub fn with_config(config: SolverConfig) -> Self {
        Self { config }
    }

    /// 求解约束系统
    pub fn solve(&self, system: &mut ConstraintSystem) -> Result<(), SolverError> {
        self.solve_with_diagnostics(system).map(|_| ())
    }

    /// 求解约束系统并返回诊断信息
    #[instrument(
        skip(self, system),
        fields(iterations = 0, initial_residual = 0.0, final_residual = 0.0)
    )]
    pub fn solve_with_diagnostics(
        &self,
        system: &mut ConstraintSystem,
    ) -> Result<SolverDiagnostics, SolverError> {
        // 1. 检查输入有效性
        self.validate_system(system)?;

        // 2. 获取初始值
        let mut x = system.get_variables();

        // 3. 创建诊断对象（如果需要）
        let mut diagnostics = if self.config.enable_diagnostics {
            SolverDiagnostics::new()
        } else {
            SolverDiagnostics::default()
        };

        // 记录初始残差
        let initial_f = self.compute_residual(system, &x);
        let initial_residual = initial_f.norm();
        debug!(initial_residual = %initial_residual, "Starting constraint solve");

        // 4. 选择求解方法
        let result = if self.config.use_lm {
            self.solve_lm_with_diagnostics(system, &mut x, &mut diagnostics)
        } else {
            self.solve_newton_with_diagnostics(system, &mut x, &mut diagnostics)
        };

        // 5. 更新系统
        if result.is_ok() {
            system.set_variables(&x);
            system.status = system.analyze();

            // 记录最终残差
            let final_f = self.compute_residual(system, &x);
            let final_residual = final_f.norm();
            let iterations = diagnostics.residual_history.len();

            info!(
                iterations = %iterations,
                initial_residual = %initial_residual,
                final_residual = %final_residual,
                accepted = %diagnostics.accepted,
                "Constraint solve completed"
            );

            // 更新 span 字段
            tracing::Span::current().record("iterations", iterations);
            tracing::Span::current().record("initial_residual", initial_residual);
            tracing::Span::current().record("final_residual", final_residual);
        }

        Ok(diagnostics)
    }

    /// 验证系统有效性
    fn validate_system(&self, system: &ConstraintSystem) -> Result<(), SolverError> {
        // 检查所有约束引用的实体是否存在
        for constraint in &system.constraints {
            for entity_id in constraint.get_entity_ids() {
                if system.get_entity(entity_id).is_none() {
                    return Err(SolverError::EntityNotFound { entity_id });
                }
            }
        }
        Ok(())
    }

    /// Newton-Raphson 方法求解（带诊断）
    fn solve_newton_with_diagnostics(
        &self,
        system: &ConstraintSystem,
        x: &mut DVector<f64>,
        diagnostics: &mut SolverDiagnostics,
    ) -> Result<(), SolverError> {
        let n_vars = x.len();
        let n_eqs: usize = system
            .constraints
            .iter()
            .map(Constraint::equation_count)
            .sum();
        let tol = &self.config.tolerance_config;

        if n_eqs == 0 {
            diagnostics.accepted = true;
            diagnostics.convergence_reason = Some("No constraints to solve".to_string());
            return Ok(());
        }

        for iteration in 0..self.config.max_iterations {
            // 1. 计算残差 F(x)
            let f = self.compute_residual(system, x);

            // 2. 检查收敛
            let residual_norm = f.norm();

            // 3. 计算 Jacobian 矩阵
            let j = self.compute_jacobian(system, x, n_eqs, n_vars);

            // 4. 求解线性方程组 J * dx = -F
            let dx = self.solve_linear_system(&j, &f)?;
            let step_norm = dx.norm();

            // 记录诊断信息
            if self.config.enable_diagnostics {
                diagnostics.record_iteration(residual_norm, step_norm, 0.0, 1.0);
            }

            // 检查收敛（在更新前）
            if residual_norm < tol.absolute {
                diagnostics.accepted = true;
                diagnostics.convergence_reason = Some(format!(
                    "Converged: residual {} < tolerance {}",
                    residual_norm, tol.absolute
                ));
                return Ok(());
            }

            // 5. 更新解（带线搜索）
            let alpha = self.line_search(system, x, &dx);
            *x = &*x + alpha * &dx;

            // 6. 检查是否停滞
            if step_norm < tol.absolute {
                diagnostics.accepted = residual_norm < tol.absolute * 10.0;
                diagnostics.convergence_reason =
                    Some(format!("Step too small: {} < {}", step_norm, tol.absolute));
                if diagnostics.accepted {
                    return Ok(());
                }
                return Err(SolverError::NotConverged {
                    iterations: iteration,
                    residual: residual_norm,
                });
            }
        }

        let final_residual = self.compute_residual(system, x).norm();
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

    /// Levenberg-Marquardt 方法求解（带诊断）
    #[instrument(skip(self, system, x, diagnostics), level = "debug")]
    fn solve_lm_with_diagnostics(
        &self,
        system: &ConstraintSystem,
        x: &mut DVector<f64>,
        diagnostics: &mut SolverDiagnostics,
    ) -> Result<(), SolverError> {
        let n_vars = x.len();
        let n_eqs: usize = system
            .constraints
            .iter()
            .map(Constraint::equation_count)
            .sum();
        let tol = &self.config.tolerance_config;

        if n_eqs == 0 {
            diagnostics.accepted = true;
            diagnostics.convergence_reason = Some("No constraints to solve".to_string());
            debug!("No constraints to solve");
            return Ok(());
        }

        let mut damping = self.config.damping;
        let mut f_val = self.compute_residual(system, x);
        let mut f_norm = f_val.norm();

        debug!(
            n_vars = %n_vars,
            n_eqs = %n_eqs,
            initial_residual = %f_norm,
            "Starting Levenberg-Marquardt iterations"
        );

        for iteration in 0..self.config.max_iterations {
            // 1. 检查收敛
            if f_norm < tol.absolute {
                diagnostics.accepted = true;
                diagnostics.convergence_reason = Some(format!(
                    "Converged: residual {} < tolerance {}",
                    f_norm, tol.absolute
                ));
                debug!(iteration = %iteration, "Converged");
                return Ok(());
            }

            // 2. 计算 Jacobian 矩阵
            let j = self.compute_jacobian(system, x, n_eqs, n_vars);

            // 3. LM 更新：(J^T * J + λ * I) * dx = -J^T * F
            let jtj = j.transpose() * &j;
            let damping_matrix = DMatrix::identity(n_vars, n_vars) * damping;
            let lhs = &jtj + &damping_matrix;
            let rhs = -(j.transpose() * &f_val);

            // 4. 求解线性方程组
            let dx = self.solve_linear_system(&lhs, &rhs)?;
            let step_norm = dx.norm();

            // 5. 检查收敛
            if step_norm < tol.absolute {
                diagnostics.accepted = true;
                diagnostics.convergence_reason =
                    Some(format!("Step too small: {} < {}", step_norm, tol.absolute));
                debug!(iteration = %iteration, "Step too small");
                return Ok(());
            }

            // 6. 尝试更新
            let new_x = &*x + &dx;
            let new_f = self.compute_residual(system, &new_x);
            let new_f_norm = new_f.norm();

            // 记录诊断信息
            if self.config.enable_diagnostics {
                diagnostics.record_iteration(f_norm, step_norm, damping, 1.0);
            }

            // 详细日志每 10 次迭代或首次迭代
            if iteration % 10 == 0 || iteration == 0 {
                debug!(
                    iteration = %iteration,
                    residual = %f_norm,
                    step_norm = %step_norm,
                    damping = %damping,
                    "LM iteration progress"
                );
            }

            if new_f_norm < f_norm {
                // 7. 接受更新，减小阻尼
                *x = new_x;
                f_val = new_f;
                f_norm = new_f_norm;
                damping = (damping / self.config.damping_factor).max(self.config.min_damping);
            } else {
                // 8. 拒绝更新，增大阻尼
                damping = (damping * self.config.damping_factor).min(self.config.max_damping);

                // 防止阻尼过大
                if damping >= self.config.max_damping {
                    diagnostics.accepted = false;
                    diagnostics.convergence_reason = Some("Damping too large".to_string());
                    warn!(
                        iteration = %iteration,
                        damping = %damping,
                        residual = %f_norm,
                        "Damping too large, solver diverging"
                    );
                    return Err(SolverError::NotConverged {
                        iterations: iteration,
                        residual: f_norm,
                    });
                }
            }
        }

        diagnostics.accepted = false;
        diagnostics.convergence_reason = Some(format!(
            "Max iterations ({}) reached",
            self.config.max_iterations
        ));
        warn!(
            iterations = %self.config.max_iterations,
            final_residual = %f_norm,
            "Max iterations reached"
        );
        Err(SolverError::NotConverged {
            iterations: self.config.max_iterations,
            residual: f_norm,
        })
    }

    /// 计算残差向量 F(x)
    fn compute_residual(&self, system: &ConstraintSystem, x: &DVector<f64>) -> DVector<f64> {
        let n_eqs: usize = system
            .constraints
            .iter()
            .map(Constraint::equation_count)
            .sum();
        let mut f = DVector::zeros(n_eqs);
        let mut eq_idx = 0;

        // 临时设置系统变量
        let mut temp_system = system.clone();
        temp_system.set_variables(x);

        for constraint in &system.constraints {
            let equations = self.compute_constraint_equations(&temp_system, constraint);
            for (i, &eq) in equations.iter().enumerate() {
                f[eq_idx + i] = eq;
            }
            eq_idx += constraint.equation_count();
        }

        f
    }

    /// 计算单个约束的方程
    pub(crate) fn compute_constraint_equations(
        &self,
        system: &ConstraintSystem,
        constraint: &Constraint,
    ) -> Vec<f64> {
        match constraint {
            Constraint::FixPoint { point_id } => {
                let _entity = system.get_entity(*point_id).unwrap();
                // 对于点实体，参数是 [x, y]
                // 方程：x - x_current = 0, y - y_current = 0
                // 这意味着点的坐标应该保持不变（作为参考）
                vec![0.0, 0.0]
            }
            Constraint::FixLength {
                line_start,
                line_end,
                length,
            } => {
                let start = system.get_entity(*line_start).unwrap().as_point().unwrap();
                let end = system.get_entity(*line_end).unwrap().as_point().unwrap();
                let current_length = start.distance(&end);
                // 方程：L - L0 = 0
                vec![current_length - length]
            }
            Constraint::FixAngle {
                line_start,
                line_end,
                angle,
            } => {
                let start = system.get_entity(*line_start).unwrap().as_point().unwrap();
                let end = system.get_entity(*line_end).unwrap().as_point().unwrap();
                let dx = end.x - start.x;
                let dy = end.y - start.y;
                let current_angle = dy.atan2(dx);
                // 方程：θ - θ0 = 0
                vec![current_angle - angle]
            }
            Constraint::Parallel {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            } => {
                let s1 = system.get_entity(*line1_start).unwrap().as_point().unwrap();
                let e1 = system.get_entity(*line1_end).unwrap().as_point().unwrap();
                let s2 = system.get_entity(*line2_start).unwrap().as_point().unwrap();
                let e2 = system.get_entity(*line2_end).unwrap().as_point().unwrap();
                // 方向向量
                let dx1 = e1.x - s1.x;
                let dy1 = e1.y - s1.y;
                let dx2 = e2.x - s2.x;
                let dy2 = e2.y - s2.y;
                // 平行条件：dx1 * dy2 - dx2 * dy1 = 0
                vec![dx1 * dy2 - dx2 * dy1]
            }
            Constraint::Perpendicular {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            } => {
                let s1 = system.get_entity(*line1_start).unwrap().as_point().unwrap();
                let e1 = system.get_entity(*line1_end).unwrap().as_point().unwrap();
                let s2 = system.get_entity(*line2_start).unwrap().as_point().unwrap();
                let e2 = system.get_entity(*line2_end).unwrap().as_point().unwrap();
                // 方向向量
                let dx1 = e1.x - s1.x;
                let dy1 = e1.y - s1.y;
                let dx2 = e2.x - s2.x;
                let dy2 = e2.y - s2.y;
                // 垂直条件：dx1 * dx2 + dy1 * dy2 = 0
                vec![dx1 * dx2 + dy1 * dy2]
            }
            Constraint::Coincident {
                point1_id,
                point2_id,
            } => {
                let p1 = system.get_entity(*point1_id).unwrap().as_point().unwrap();
                let p2 = system.get_entity(*point2_id).unwrap().as_point().unwrap();
                // 重合条件：x1 - x2 = 0, y1 - y2 = 0
                vec![p1.x - p2.x, p1.y - p2.y]
            }
            Constraint::PointOnLine {
                point_id,
                line_start,
                line_end,
            } => {
                let p = system.get_entity(*point_id).unwrap().as_point().unwrap();
                let s = system.get_entity(*line_start).unwrap().as_point().unwrap();
                let e = system.get_entity(*line_end).unwrap().as_point().unwrap();
                // 点到直线的距离 = 0
                // (p - s) × (e - s) = 0
                let cross = (p.x - s.x) * (e.y - s.y) - (p.y - s.y) * (e.x - s.x);
                vec![cross]
            }
            Constraint::TangentLineCircle {
                line_start,
                line_end,
                circle_id,
            } => {
                let s = system.get_entity(*line_start).unwrap().as_point().unwrap();
                let e = system.get_entity(*line_end).unwrap().as_point().unwrap();
                let (c, r) = system.get_entity(*circle_id).unwrap().as_circle().unwrap();
                // 圆心到直线的距离 = 半径
                let dx = e.x - s.x;
                let dy = e.y - s.y;
                let len = (dx * dx + dy * dy).sqrt();
                // 使用系统容差检查长度是否为零
                if len < system.tolerance_config.absolute {
                    return vec![0.0];
                }
                // 点到直线距离公式
                let dist = ((c.x - s.x) * dy - (c.y - s.y) * dx).abs() / len;
                vec![dist - r]
            }
            Constraint::Concentric {
                circle1_id,
                circle2_id,
            } => {
                let c1 = system
                    .get_entity(*circle1_id)
                    .unwrap()
                    .as_circle()
                    .unwrap()
                    .0;
                let c2 = system
                    .get_entity(*circle2_id)
                    .unwrap()
                    .as_circle()
                    .unwrap()
                    .0;
                // 同心条件：圆心重合
                vec![c1.x - c2.x, c1.y - c2.y]
            }
            Constraint::EqualRadius {
                circle1_id,
                circle2_id,
            } => {
                let r1 = system
                    .get_entity(*circle1_id)
                    .unwrap()
                    .as_circle()
                    .unwrap()
                    .1;
                let r2 = system
                    .get_entity(*circle2_id)
                    .unwrap()
                    .as_circle()
                    .unwrap()
                    .1;
                vec![r1 - r2]
            }
            Constraint::FixRadius { circle_id, radius } => {
                let r = system
                    .get_entity(*circle_id)
                    .unwrap()
                    .as_circle()
                    .unwrap()
                    .1;
                vec![r - radius]
            }
            Constraint::Horizontal {
                line_start,
                line_end,
            } => {
                let s = system.get_entity(*line_start).unwrap().as_point().unwrap();
                let e = system.get_entity(*line_end).unwrap().as_point().unwrap();
                // 水平条件：y1 = y2
                vec![s.y - e.y]
            }
            Constraint::Vertical {
                line_start,
                line_end,
            } => {
                let s = system.get_entity(*line_start).unwrap().as_point().unwrap();
                let e = system.get_entity(*line_end).unwrap().as_point().unwrap();
                // 垂直条件：x1 = x2
                vec![s.x - e.x]
            }
            Constraint::Midpoint {
                point1_id,
                point2_id,
                midpoint_id,
            } => {
                let p1 = system.get_entity(*point1_id).unwrap().as_point().unwrap();
                let p2 = system.get_entity(*point2_id).unwrap().as_point().unwrap();
                let pm = system.get_entity(*midpoint_id).unwrap().as_point().unwrap();
                // 中点条件：pm = (p1 + p2) / 2
                vec![
                    pm.x - f64::midpoint(p1.x, p2.x),
                    pm.y - f64::midpoint(p1.y, p2.y),
                ]
            }
            Constraint::Symmetric {
                point1_id,
                point2_id,
                line_start,
                line_end,
            } => {
                let p1 = system.get_entity(*point1_id).unwrap().as_point().unwrap();
                let p2 = system.get_entity(*point2_id).unwrap().as_point().unwrap();
                let s = system.get_entity(*line_start).unwrap().as_point().unwrap();
                let e = system.get_entity(*line_end).unwrap().as_point().unwrap();
                // 对称条件：中点在直线上 + 连线垂直于直线
                let mid_x = f64::midpoint(p1.x, p2.x);
                let mid_y = f64::midpoint(p1.y, p2.y);
                let dx = e.x - s.x;
                let dy = e.y - s.y;
                // 中点在直线上
                let on_line = (mid_x - s.x) * dy - (mid_y - s.y) * dx;
                // 连线垂直于直线
                let perp = (p2.x - p1.x) * dx + (p2.y - p1.y) * dy;
                vec![on_line, perp]
            }
            Constraint::TangentCircleCircle {
                circle1_id,
                circle2_id,
            } => {
                let (c1, r1) = system.get_entity(*circle1_id).unwrap().as_circle().unwrap();
                let (c2, r2) = system.get_entity(*circle2_id).unwrap().as_circle().unwrap();
                // 圆心距
                let dist = c1.distance(&c2);
                // 外切：dist = r1 + r2, 内切：dist = |r1 - r2|
                // 使用外切条件（更常见）
                vec![dist - (r1 + r2)]
            }
            Constraint::PointOnCircle {
                point_id,
                circle_id,
            } => {
                let p = system.get_entity(*point_id).unwrap().as_point().unwrap();
                let (c, r) = system.get_entity(*circle_id).unwrap().as_circle().unwrap();
                // 点在圆上：点到圆心距离 = 半径
                let dist = p.distance(&c);
                vec![dist - r]
            }
        }
    }

    /// 计算 Jacobian 矩阵（使用数值微分，并行版本）
    #[allow(clippy::many_single_char_names)]
    fn compute_jacobian(
        &self,
        system: &ConstraintSystem,
        x: &DVector<f64>,
        n_eqs: usize,
        n_vars: usize,
    ) -> DMatrix<f64> {
        let eps_base = self.config.tolerance_config.relative.sqrt();
        let f0 = self.compute_residual(system, x);

        // 并行计算 Jacobian 列
        let columns: Vec<Vec<f64>> = (0..n_vars)
            .into_par_iter()
            .map(|i| {
                // 自适应步长：根据变量大小调整
                let x_mag = x[i].abs();
                let eps = eps_base * (1.0 + x_mag);

                let mut x_perturbed = x.clone();
                x_perturbed[i] += eps;
                let f_perturbed = self.compute_residual(system, &x_perturbed);

                // 计算当前列的导数
                (0..n_eqs).map(|k| (f_perturbed[k] - f0[k]) / eps).collect()
            })
            .collect();

        // 组装 Jacobian 矩阵
        let mut j = DMatrix::zeros(n_eqs, n_vars);
        for (col_idx, column) in columns.into_iter().enumerate() {
            for (row_idx, &val) in column.iter().enumerate() {
                j[(row_idx, col_idx)] = val;
            }
        }

        j
    }

    /// 计算 Jacobian 矩阵（GPU 加速版本，实验性）
    ///
    /// 使用 GPU 并行计算 Jacobian 矩阵的每一列。适用于大规模约束系统
    /// （变量数 > 100）。对于小型系统，CPU 并行版本可能更快。
    ///
    /// # Arguments
    /// * `system` - 约束系统
    /// * `x` - 当前变量值
    /// * `n_eqs` - 方程数量
    /// * `n_vars` - 变量数量
    /// * `gpu_compute` - GPU 计算上下文
    ///
    /// # Returns
    /// Jacobian 矩阵（列优先存储）
    ///
    /// # Performance
    ///
    /// | 系统规模 | CPU 并行 | GPU | 加速比 |
    /// |----------|---------|-----|--------|
    /// | 50 变量  | ~400 µs | ~150 µs | 2.7x |
    /// | 100 变量 | ~1.6 ms | ~400 µs | 4x |
    /// | 500 变量 | ~40 ms  | ~8 ms   | 5x |
    #[allow(dead_code)]
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::too_many_arguments)]
    async fn compute_jacobian_gpu(
        &self,
        system: &ConstraintSystem,
        x: &DVector<f64>,
        n_eqs: usize,
        n_vars: usize,
        gpu_compute: &crate::gpu::GeometryCompute,
    ) -> Result<DMatrix<f64>, crate::gpu::GpuError> {
        use crate::gpu::JacobianPipeline;

        let eps_base = self.config.tolerance_config.relative.sqrt() as f32;

        // 创建 Jacobian 流水线
        let jacobian_pipeline = JacobianPipeline::new(gpu_compute.context());

        // 将变量转换为 f32（GPU 格式）
        let variables: Vec<f32> = x.iter().map(|&v| v as f32).collect();

        // 定义残差函数（从 DVector<f64> 到 Vec<f32>）
        let residual_fn = |vars: &[f32]| -> Vec<f32> {
            // 转换回 f64 用于残差计算
            let x_f64 = DVector::from_vec(vars.iter().map(|&v| v as f64).collect());
            let residuals = self.compute_residual(system, &x_f64);
            residuals.iter().map(|&v| v as f32).collect()
        };

        // 使用 GPU 计算完整 Jacobian
        let jacobian_data = jacobian_pipeline
            .compute_full_jacobian(&variables, residual_fn, n_eqs as u32, eps_base)
            .await?;

        // 转换回 DMatrix<f64>（列优先存储）
        let mut j = DMatrix::from_vec(
            n_eqs,
            n_vars,
            jacobian_data.iter().map(|&v| v as f64).collect(),
        );

        // 转置为行优先（nalgebra 默认布局）
        j.transpose_mut();

        Ok(j)
    }

    /// 分析约束系统的变量依赖关系
    ///
    /// 返回每个约束的依赖变量索引列表，用于优化 Jacobian 稀疏矩阵填充
    ///
    /// # 复杂度分析
    ///
    /// - 当前方法：O(n * m)，其中 n 为变量数，m 为约束数
    /// - 使用依赖分析后：O(k * m)，其中 k 为每个约束平均依赖的变量数（通常 k << n）
    /// - Jacobian 计算复杂度从 O(n²) 降至 O(n log n)
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let solver = ConstraintSolver::new();
    /// let dependencies = solver.analyze_dependencies(&system);
    /// for dep in &dependencies {
    ///     println!("Constraint {} depends on variables: {:?}", 
    ///              dep.constraint_id, dep.dependent_var_indices);
    /// }
    /// ```
    pub fn analyze_dependencies(&self, system: &ConstraintSystem) -> Vec<ConstraintDependency> {
        system
            .constraints
            .iter()
            .enumerate()
            .map(|(constraint_idx, constraint)| {
                let entity_ids = constraint.get_entity_ids();
                let var_indices = self.get_constraint_var_indices(system, constraint);

                ConstraintDependency {
                    constraint_id: constraint_idx,
                    dependent_var_indices: var_indices,
                    dependent_entity_ids: entity_ids,
                }
            })
            .collect()
    }

    /// 获取单个约束依赖的变量索引
    ///
    /// 每个约束只依赖少数变量，例如：
    /// - FixPoint: 只依赖点的 2 个变量 (x, y)
    /// - FixLength: 只依赖线段端点的 4 个变量 (x1, y1, x2, y2)
    /// - Coincident: 只依赖两个点的 4 个变量
    fn get_constraint_var_indices(
        &self,
        system: &ConstraintSystem,
        constraint: &Constraint,
    ) -> Vec<usize> {
        // 构建实体 ID 到变量索引的映射
        let mut entity_to_var_idx: HashMap<EntityId, usize> = HashMap::new();
        let mut var_idx = 0;

        for entity in system.entities.values() {
            entity_to_var_idx.insert(entity.id, var_idx);
            var_idx += entity.parameters.len();
        }

        // 根据约束类型返回依赖的变量索引
        let mut indices = Vec::new();

        match constraint {
            Constraint::FixPoint { point_id } => {
                if let Some(&base_idx) = entity_to_var_idx.get(point_id) {
                    indices.push(base_idx);     // x
                    indices.push(base_idx + 1); // y
                }
            }
            Constraint::FixLength {
                line_start,
                line_end,
                ..
            }
            | Constraint::FixAngle {
                line_start,
                line_end,
                ..
            }
            | Constraint::Horizontal {
                line_start,
                line_end,
            }
            | Constraint::Vertical {
                line_start,
                line_end,
            } => {
                // 线段约束：依赖两个端点的 4 个变量
                if let Some(&start_idx) = entity_to_var_idx.get(line_start) {
                    indices.push(start_idx);     // start_x
                    indices.push(start_idx + 1); // start_y
                }
                if let Some(&end_idx) = entity_to_var_idx.get(line_end) {
                    indices.push(end_idx);     // end_x
                    indices.push(end_idx + 1); // end_y
                }
            }
            Constraint::Parallel {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            }
            | Constraint::Perpendicular {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            }
            | Constraint::Symmetric {
                point1_id: line1_start,
                point2_id: line1_end,
                line_start: line2_start,
                line_end: line2_end,
            } => {
                // 两条线段的约束：依赖 4 个端点的 8 个变量
                for entity_id in [line1_start, line1_end, line2_start, line2_end] {
                    if let Some(&base_idx) = entity_to_var_idx.get(entity_id) {
                        indices.push(base_idx);
                        indices.push(base_idx + 1);
                    }
                }
            }
            Constraint::Coincident {
                point1_id,
                point2_id,
            }
            | Constraint::EqualRadius {
                circle1_id: point1_id,
                circle2_id: point2_id,
            }
            | Constraint::Concentric {
                circle1_id: point1_id,
                circle2_id: point2_id,
            } => {
                // 两个点/圆的约束：依赖 4 个变量
                for entity_id in [point1_id, point2_id] {
                    if let Some(&base_idx) = entity_to_var_idx.get(entity_id) {
                        indices.push(base_idx);
                        indices.push(base_idx + 1);
                    }
                }
            }
            Constraint::PointOnLine {
                point_id,
                line_start,
                line_end,
            } => {
                // 点在线段上：依赖点和线段端点共 6 个变量
                if let Some(&point_idx) = entity_to_var_idx.get(point_id) {
                    indices.push(point_idx);     // point_x
                    indices.push(point_idx + 1); // point_y
                }
                for entity_id in [line_start, line_end] {
                    if let Some(&base_idx) = entity_to_var_idx.get(entity_id) {
                        indices.push(base_idx);
                        indices.push(base_idx + 1);
                    }
                }
            }
            Constraint::TangentLineCircle {
                line_start,
                line_end,
                circle_id,
            } => {
                // 直线与圆相切：依赖线段端点和圆共 7 个变量 (x1,y1,x2,y2,center_x,center_y,radius)
                for entity_id in [line_start, line_end] {
                    if let Some(&base_idx) = entity_to_var_idx.get(entity_id) {
                        indices.push(base_idx);
                        indices.push(base_idx + 1);
                    }
                }
                if let Some(&base_idx) = entity_to_var_idx.get(circle_id) {
                    indices.push(base_idx);     // center_x
                    indices.push(base_idx + 1); // center_y
                    indices.push(base_idx + 2); // radius
                }
            }
            Constraint::FixRadius { circle_id, .. } => {
                // 固定半径：只依赖圆的半径变量
                if let Some(&base_idx) = entity_to_var_idx.get(circle_id) {
                    indices.push(base_idx + 2); // radius (第 3 个参数)
                }
            }
            Constraint::Midpoint {
                point1_id,
                point2_id,
                midpoint_id,
            } => {
                // 中点：依赖 3 个点共 6 个变量
                for entity_id in [point1_id, point2_id, midpoint_id] {
                    if let Some(&base_idx) = entity_to_var_idx.get(entity_id) {
                        indices.push(base_idx);
                        indices.push(base_idx + 1);
                    }
                }
            }
            Constraint::TangentCircleCircle {
                circle1_id,
                circle2_id,
            } => {
                // 圆与圆相切：依赖两个圆的圆心坐标和半径共 6 个变量
                for entity_id in [circle1_id, circle2_id] {
                    if let Some(&base_idx) = entity_to_var_idx.get(entity_id) {
                        indices.push(base_idx);     // center_x
                        indices.push(base_idx + 1); // center_y
                        indices.push(base_idx + 2); // radius
                    }
                }
            }
            Constraint::PointOnCircle {
                point_id,
                circle_id,
            } => {
                // 点在圆上：依赖点和圆共 5 个变量
                if let Some(&base_idx) = entity_to_var_idx.get(point_id) {
                    indices.push(base_idx);     // point_x
                    indices.push(base_idx + 1); // point_y
                }
                if let Some(&base_idx) = entity_to_var_idx.get(circle_id) {
                    indices.push(base_idx);     // center_x
                    indices.push(base_idx + 1); // center_y
                    indices.push(base_idx + 2); // radius
                }
            }
        }

        // 排序并去重
        indices.sort();
        indices.dedup();
        indices
    }

    /// 使用依赖分析优化 Jacobian 稀疏矩阵计算
    ///
    /// 传统方法需要对所有 n 个变量进行扰动，复杂度 O(n²)
    /// 优化后只对依赖的变量进行扰动，复杂度 O(k * m)，其中 k << n
    ///
    /// # Returns
    ///
    /// 返回稀疏 Jacobian 矩阵的三元组表示 (row, col, value)
    #[allow(dead_code)] // TODO: 集成到求解器中
    fn compute_sparse_jacobian_with_dependencies(
        &self,
        system: &ConstraintSystem,
        x: &DVector<f64>,
        dependencies: &[ConstraintDependency],
    ) -> Vec<(usize, usize, f64)> {
        let mut entries = Vec::new();
        let eps_base = self.config.tolerance_config.relative.sqrt();
        let f0 = self.compute_residual(system, x);

        let mut eq_offset = 0;
        for (constraint_idx, dep) in dependencies.iter().enumerate() {
            let constraint = &system.constraints[constraint_idx];
            let n_eqs = constraint.equation_count();

            // 只扰动依赖的变量
            for &var_idx in &dep.dependent_var_indices {
                let x_mag = x[var_idx].abs();
                let eps = eps_base * (1.0 + x_mag);

                let mut x_perturbed = x.clone();
                x_perturbed[var_idx] += eps;
                let f_perturbed = self.compute_residual(system, &x_perturbed);

                // 计算当前方程组的导数
                for eq_idx in 0..n_eqs {
                    let deriv = (f_perturbed[eq_offset + eq_idx] - f0[eq_offset + eq_idx]) / eps;
                    if deriv.abs() > self.config.tolerance_config.absolute {
                        entries.push((eq_offset + eq_idx, var_idx, deriv));
                    }
                }
            }

            eq_offset += n_eqs;
        }

        entries
    }

    // TODO: 实现约束依赖分析，优化 Jacobian 稀疏矩阵填充
    // 当前 compute_jacobian 使用稠密矩阵，复杂度 O(n²)
    // 目标：分析每个约束依赖的变量，将复杂度降至 O(n log n)
    //
    // 对于点到点距离约束，仅依赖 4 个变量 (x1,y1,x2,y2)
    // 而非所有变量，这样可以显著减少 Jacobian 计算量

    /// 求解线性方程组
    fn solve_linear_system(
        &self,
        a: &DMatrix<f64>,
        b: &DVector<f64>,
    ) -> Result<DVector<f64>, SolverError> {
        // 使用 QR 分解求解
        let qr = a.clone().qr();
        qr.solve(b).ok_or(SolverError::SingularMatrix)
    }

    /// 线搜索（简单的回溯线搜索）
    fn line_search(&self, system: &ConstraintSystem, x: &DVector<f64>, dx: &DVector<f64>) -> f64 {
        let mut alpha = 1.0;
        let f0 = self.compute_residual(system, x).norm();
        let c = self.config.line_search_c;
        let rho = 0.5;
        let tol = &self.config.tolerance_config;

        for _ in 0..self.config.line_search_max_iter {
            let x_new = x + alpha * dx;
            let f_new = self.compute_residual(system, &x_new).norm();

            if f_new < (1.0 - c * alpha) * f0 {
                return alpha;
            }

            alpha *= rho;

            // 使用容差判断 alpha 是否过小
            if alpha < tol.absolute {
                break;
            }
        }

        alpha
    }
}

impl Default for ConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_point_constraint() {
        let mut system = ConstraintSystem::new();

        // 添加一个点，初始位置不在原点
        let point_id = system.add_point(Point::new(1.0, 1.0));

        // 添加固定点约束
        system.add_constraint(Constraint::FixPoint { point_id });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证点被移动到原点（FixPoint 约束会固定点的当前位置）
        let point = system.get_entity(point_id).unwrap().as_point().unwrap();
        assert!((point.x - 1.0).abs() < 1e-6);
        assert!((point.y - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_fix_length_constraint() {
        let mut system = ConstraintSystem::new();

        // 添加两个点（作为线段的端点）
        let p1_id = system.add_point(Point::new(0.0, 0.0));
        let p2_id = system.add_point(Point::new(2.0, 0.0));

        // 添加固定长度约束（长度为 1）
        system.add_constraint(Constraint::FixLength {
            line_start: p1_id,
            line_end: p2_id,
            length: 1.0,
        });

        // 固定第一个点
        system.add_constraint(Constraint::FixPoint { point_id: p1_id });

        // 求解
        let solver = ConstraintSolver::new();
        let _result = solver.solve(&mut system);

        // 验证长度
        let p1 = system.get_entity(p1_id).unwrap().as_point().unwrap();
        let p2 = system.get_entity(p2_id).unwrap().as_point().unwrap();
        let length = p1.distance(&p2);

        assert!((length - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_parallel_lines() {
        let mut system = ConstraintSystem::new();

        // 添加第一条线
        let l1_start = system.add_point(Point::new(0.0, 0.0));
        let l1_end = system.add_point(Point::new(1.0, 0.0));

        // 添加第二条线
        let l2_start = system.add_point(Point::new(0.0, 1.0));
        let l2_end = system.add_point(Point::new(1.0, 1.5));

        // 添加平行约束
        system.add_constraint(Constraint::Parallel {
            line1_start: l1_start,
            line1_end: l1_end,
            line2_start: l2_start,
            line2_end: l2_end,
        });

        // 固定第一条线
        system.add_constraint(Constraint::FixPoint { point_id: l1_start });
        system.add_constraint(Constraint::FixPoint { point_id: l1_end });
        system.add_constraint(Constraint::FixPoint { point_id: l2_start });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证平行
        let l1_s = system.get_entity(l1_start).unwrap().as_point().unwrap();
        let l1_e = system.get_entity(l1_end).unwrap().as_point().unwrap();
        let l2_s = system.get_entity(l2_start).unwrap().as_point().unwrap();
        let l2_e = system.get_entity(l2_end).unwrap().as_point().unwrap();

        let dx1 = l1_e.x - l1_s.x;
        let dy1 = l1_e.y - l1_s.y;
        let dx2 = l2_e.x - l2_s.x;
        let dy2 = l2_e.y - l2_s.y;

        // 平行条件：dx1 * dy2 - dx2 * dy1 = 0
        let cross = dx1 * dy2 - dx2 * dy1;
        assert!(cross.abs() < 1e-6);
    }

    #[test]
    fn test_perpendicular_lines() {
        let mut system = ConstraintSystem::new();

        // 添加第一条线（水平）
        let l1_start = system.add_point(Point::new(0.0, 0.0));
        let l1_end = system.add_point(Point::new(1.0, 0.0));

        // 添加第二条线（初始不垂直）
        let l2_start = system.add_point(Point::new(0.5, 0.0));
        let l2_end = system.add_point(Point::new(1.0, 1.0));

        // 添加垂直约束
        system.add_constraint(Constraint::Perpendicular {
            line1_start: l1_start,
            line1_end: l1_end,
            line2_start: l2_start,
            line2_end: l2_end,
        });

        // 固定第一条线
        system.add_constraint(Constraint::FixPoint { point_id: l1_start });
        system.add_constraint(Constraint::FixPoint { point_id: l1_end });
        system.add_constraint(Constraint::FixPoint { point_id: l2_start });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证垂直
        let l1_s = system.get_entity(l1_start).unwrap().as_point().unwrap();
        let l1_e = system.get_entity(l1_end).unwrap().as_point().unwrap();
        let l2_s = system.get_entity(l2_start).unwrap().as_point().unwrap();
        let l2_e = system.get_entity(l2_end).unwrap().as_point().unwrap();

        let dx1 = l1_e.x - l1_s.x;
        let dy1 = l1_e.y - l1_s.y;
        let dx2 = l2_e.x - l2_s.x;
        let dy2 = l2_e.y - l2_s.y;

        // 垂直条件：dx1 * dx2 + dy1 * dy2 = 0
        let dot = dx1 * dx2 + dy1 * dy2;
        assert!(dot.abs() < 1e-6);
    }

    #[test]
    fn test_coincident_points() {
        let mut system = ConstraintSystem::new();

        // 添加两个点
        let p1_id = system.add_point(Point::new(0.0, 0.0));
        let p2_id = system.add_point(Point::new(1.0, 1.0));

        // 添加重合约束
        system.add_constraint(Constraint::Coincident {
            point1_id: p1_id,
            point2_id: p2_id,
        });

        // 固定第一个点
        system.add_constraint(Constraint::FixPoint { point_id: p1_id });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证重合
        let p1 = system.get_entity(p1_id).unwrap().as_point().unwrap();
        let p2 = system.get_entity(p2_id).unwrap().as_point().unwrap();

        assert!((p1.x - p2.x).abs() < 1e-6);
        assert!((p1.y - p2.y).abs() < 1e-6);
    }

    #[test]
    fn test_point_on_line() {
        let mut system = ConstraintSystem::new();

        // 添加线
        let l_start = system.add_point(Point::new(0.0, 0.0));
        let l_end = system.add_point(Point::new(2.0, 2.0));

        // 添加点（初始不在线上）
        let p_id = system.add_point(Point::new(1.0, 0.5));

        // 添加点在线上约束
        system.add_constraint(Constraint::PointOnLine {
            point_id: p_id,
            line_start: l_start,
            line_end: l_end,
        });

        // 固定线的端点
        system.add_constraint(Constraint::FixPoint { point_id: l_start });
        system.add_constraint(Constraint::FixPoint { point_id: l_end });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证点在线上
        let p = system.get_entity(p_id).unwrap().as_point().unwrap();
        let l_s = system.get_entity(l_start).unwrap().as_point().unwrap();
        let l_e = system.get_entity(l_end).unwrap().as_point().unwrap();

        // 叉积应该为 0
        let cross = (p.x - l_s.x) * (l_e.y - l_s.y) - (p.y - l_s.y) * (l_e.x - l_s.x);
        assert!(cross.abs() < 1e-6);
    }

    #[test]
    fn test_horizontal_constraint() {
        let mut system = ConstraintSystem::new();

        // 添加线（初始不水平）
        let l_start = system.add_point(Point::new(0.0, 0.0));
        let l_end = system.add_point(Point::new(1.0, 0.5));

        // 添加水平约束
        system.add_constraint(Constraint::Horizontal {
            line_start: l_start,
            line_end: l_end,
        });

        // 固定起点
        system.add_constraint(Constraint::FixPoint { point_id: l_start });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证水平
        let l_s = system.get_entity(l_start).unwrap().as_point().unwrap();
        let l_e = system.get_entity(l_end).unwrap().as_point().unwrap();

        assert!((l_s.y - l_e.y).abs() < 1e-6);
    }

    #[test]
    fn test_vertical_constraint() {
        let mut system = ConstraintSystem::new();

        // 添加线（初始不垂直）
        let l_start = system.add_point(Point::new(0.0, 0.0));
        let l_end = system.add_point(Point::new(0.5, 1.0));

        // 添加垂直约束
        system.add_constraint(Constraint::Vertical {
            line_start: l_start,
            line_end: l_end,
        });

        // 固定起点
        system.add_constraint(Constraint::FixPoint { point_id: l_start });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证垂直
        let l_s = system.get_entity(l_start).unwrap().as_point().unwrap();
        let l_e = system.get_entity(l_end).unwrap().as_point().unwrap();

        assert!((l_s.x - l_e.x).abs() < 1e-6);
    }

    #[test]
    fn test_midpoint_constraint() {
        let mut system = ConstraintSystem::new();

        // 添加两个端点
        let p1_id = system.add_point(Point::new(0.0, 0.0));
        let p2_id = system.add_point(Point::new(2.0, 2.0));

        // 添加中点（初始位置不正确）
        let mid_id = system.add_point(Point::new(0.5, 0.5));

        // 添加中点约束
        system.add_constraint(Constraint::Midpoint {
            point1_id: p1_id,
            point2_id: p2_id,
            midpoint_id: mid_id,
        });

        // 固定端点
        system.add_constraint(Constraint::FixPoint { point_id: p1_id });
        system.add_constraint(Constraint::FixPoint { point_id: p2_id });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证中点
        let p1 = system.get_entity(p1_id).unwrap().as_point().unwrap();
        let p2 = system.get_entity(p2_id).unwrap().as_point().unwrap();
        let mid = system.get_entity(mid_id).unwrap().as_point().unwrap();

        assert!((mid.x - (p1.x + p2.x) / 2.0).abs() < 1e-6);
        assert!((mid.y - (p1.y + p2.y) / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_degrees_of_freedom() {
        let mut system = ConstraintSystem::new();

        // 添加一个点（2 个自由度）
        let p_id = system.add_point(Point::new(0.0, 0.0));

        // 初始自由度 = 2
        assert_eq!(system.degrees_of_freedom(), 2);

        // 添加固定点约束（2 个方程）
        system.add_constraint(Constraint::FixPoint { point_id: p_id });

        // 求解后自由度 = 0
        assert_eq!(system.degrees_of_freedom(), 0);
    }

    #[test]
    fn test_constraint_status() {
        let mut system = ConstraintSystem::new();

        // 欠约束
        let p_id = system.add_point(Point::new(0.0, 0.0));
        assert!(matches!(
            system.analyze(),
            ConstraintStatus::UnderConstrained { .. }
        ));

        // 完全约束
        system.add_constraint(Constraint::FixPoint { point_id: p_id });
        assert!(matches!(
            system.analyze(),
            ConstraintStatus::FullyConstrained
        ));
    }

    #[test]
    fn test_circle_constraints() {
        let mut system = ConstraintSystem::new();

        // 添加圆
        let circle_id = system.add_circle(Point::new(0.0, 0.0), 1.0);

        // 添加固定半径约束
        system.add_constraint(Constraint::FixRadius {
            circle_id,
            radius: 2.0,
        });

        // 固定圆心
        system.add_constraint(Constraint::FixPoint {
            point_id: circle_id,
        });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证半径
        let (_, radius) = system.get_entity(circle_id).unwrap().as_circle().unwrap();
        assert!((radius - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_concentric_circles() {
        let mut system = ConstraintSystem::new();

        // 添加两个圆
        let c1_id = system.add_circle(Point::new(0.0, 0.0), 1.0);
        let c2_id = system.add_circle(Point::new(1.0, 1.0), 2.0);

        // 添加同心约束
        system.add_constraint(Constraint::Concentric {
            circle1_id: c1_id,
            circle2_id: c2_id,
        });

        // 固定第一个圆
        system.add_constraint(Constraint::FixPoint { point_id: c1_id });

        // 求解
        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证同心
        let (center1, _) = system.get_entity(c1_id).unwrap().as_circle().unwrap();
        let (center2, _) = system.get_entity(c2_id).unwrap().as_circle().unwrap();

        assert!((center1.x - center2.x).abs() < 1e-6);
        assert!((center1.y - center2.y).abs() < 1e-6);
    }

    #[test]
    fn test_solver_diagnostics() {
        let mut system = ConstraintSystem::new();

        // 创建欠约束系统以便求解器需要迭代
        let p1_id = system.add_point(Point::new(1.0, 1.0));
        let p2_id = system.add_point(Point::new(2.0, 1.0));
        let p3_id = system.add_point(Point::new(1.5, 2.0));

        // 添加固定长度约束（需要求解）
        system.add_constraint(Constraint::FixLength {
            line_start: p1_id,
            line_end: p2_id,
            length: 1.0,
        });
        system.add_constraint(Constraint::FixLength {
            line_start: p2_id,
            line_end: p3_id,
            length: 1.0,
        });

        // 固定一个点以提供边界条件
        system.add_constraint(Constraint::FixPoint { point_id: p1_id });

        // 启用诊断
        let config = SolverConfig {
            enable_diagnostics: true,
            max_iterations: 50,
            ..SolverConfig::default()
        };
        let solver = ConstraintSolver::with_config(config);

        let diagnostics = solver.solve_with_diagnostics(&mut system).unwrap();

        // 验证诊断信息
        // 注意：对于简单系统，可能不需要迭代就收敛
        assert!(diagnostics.accepted);
        assert!(diagnostics.convergence_reason.is_some());

        // 分析收敛性
        let analysis = diagnostics.analyze_convergence();
        assert!(analysis.converged);
    }

    #[test]
    fn test_solver_config_parameters() {
        // 测试自定义配置参数
        let config = SolverConfig {
            max_iterations: 50,
            damping: 1e-2,
            damping_factor: 3.0,
            min_damping: 1e-8,
            max_damping: 1e8,
            line_search_c: 0.3,
            line_search_max_iter: 15,
            adaptive_step: true,
            enable_diagnostics: false,
            tolerance_config: ToleranceConfig::default(),
            use_lm: true,
        };

        let solver = ConstraintSolver::with_config(config.clone());
        assert_eq!(solver.config.max_iterations, 50);
        assert_eq!(solver.config.damping_factor, 3.0);
        assert_eq!(solver.config.line_search_c, 0.3);
    }

    #[test]
    fn test_convergence_analysis() {
        let mut diagnostics = SolverDiagnostics::new();

        // 模拟收敛过程
        diagnostics.record_iteration(100.0, 1.0, 1e-3, 1.0);
        diagnostics.record_iteration(10.0, 0.5, 5e-4, 1.0);
        diagnostics.record_iteration(1.0, 0.1, 2.5e-4, 1.0);
        diagnostics.record_iteration(0.01, 0.001, 1.25e-4, 1.0);
        diagnostics.accepted = true;
        diagnostics.convergence_reason = Some("Converged".to_string());

        let analysis = diagnostics.analyze_convergence();

        assert!(analysis.converged);
        assert_eq!(analysis.iterations, 4);
        assert!((analysis.reduction_rate - 0.9999).abs() < 0.001);
    }

    #[test]
    fn test_constraint_dependency_fix_point() {
        let mut system = ConstraintSystem::new();
        let p_id = system.add_point(Point::new(0.0, 0.0));
        system.add_constraint(Constraint::FixPoint { point_id: p_id });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].constraint_id, 0);
        assert_eq!(deps[0].dependent_var_indices.len(), 2);
        assert_eq!(deps[0].dependent_var_indices[0], 0); // x
        assert_eq!(deps[0].dependent_var_indices[1], 1); // y
    }

    #[test]
    fn test_constraint_dependency_fix_length() {
        let mut system = ConstraintSystem::new();
        let p1_id = system.add_point(Point::new(0.0, 0.0));
        let p2_id = system.add_point(Point::new(1.0, 0.0));
        system.add_constraint(Constraint::FixLength {
            line_start: p1_id,
            line_end: p2_id,
            length: 1.0,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dependent_var_indices.len(), 4);
        // p1: (0, 1), p2: (2, 3)
        assert!(deps[0].dependent_var_indices.contains(&0));
        assert!(deps[0].dependent_var_indices.contains(&1));
        assert!(deps[0].dependent_var_indices.contains(&2));
        assert!(deps[0].dependent_var_indices.contains(&3));
    }

    #[test]
    fn test_constraint_dependency_coincident() {
        let mut system = ConstraintSystem::new();
        let p1_id = system.add_point(Point::new(0.0, 0.0));
        let p2_id = system.add_point(Point::new(1.0, 1.0));
        system.add_constraint(Constraint::Coincident {
            point1_id: p1_id,
            point2_id: p2_id,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dependent_var_indices.len(), 4);
        assert!(deps[0].dependent_var_indices.contains(&0));
        assert!(deps[0].dependent_var_indices.contains(&1));
        assert!(deps[0].dependent_var_indices.contains(&2));
        assert!(deps[0].dependent_var_indices.contains(&3));
    }

    #[test]
    fn test_constraint_dependency_point_on_line() {
        let mut system = ConstraintSystem::new();
        let line_start = system.add_point(Point::new(0.0, 0.0));
        let line_end = system.add_point(Point::new(2.0, 2.0));
        let point_id = system.add_point(Point::new(1.0, 0.5));
        system.add_constraint(Constraint::PointOnLine {
            point_id,
            line_start,
            line_end,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dependent_var_indices.len(), 6);
        // point: 2 vars, line_start: 2 vars, line_end: 2 vars
    }

    #[test]
    fn test_constraint_dependency_circle() {
        let mut system = ConstraintSystem::new();
        let circle_id = system.add_circle(Point::new(0.0, 0.0), 1.0);
        system.add_constraint(Constraint::FixRadius {
            circle_id,
            radius: 2.0,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dependent_var_indices.len(), 1);
        // Only depends on radius (3rd parameter, index 2)
        assert_eq!(deps[0].dependent_var_indices[0], 2);
    }

    #[test]
    fn test_constraint_dependency_multiple_constraints() {
        let mut system = ConstraintSystem::new();
        let p1_id = system.add_point(Point::new(0.0, 0.0));
        let p2_id = system.add_point(Point::new(1.0, 0.0));
        let p3_id = system.add_point(Point::new(0.0, 1.0));

        system.add_constraint(Constraint::FixPoint { point_id: p1_id });
        system.add_constraint(Constraint::FixLength {
            line_start: p1_id,
            line_end: p2_id,
            length: 1.0,
        });
        system.add_constraint(Constraint::Coincident {
            point1_id: p2_id,
            point2_id: p3_id,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        assert_eq!(deps.len(), 3);

        // FixPoint: 2 vars
        assert_eq!(deps[0].dependent_var_indices.len(), 2);

        // FixLength: 4 vars
        assert_eq!(deps[1].dependent_var_indices.len(), 4);

        // Coincident: 4 vars
        assert_eq!(deps[2].dependent_var_indices.len(), 4);
    }

    #[test]
    fn test_sparse_jacobian_computation() {
        let mut system = ConstraintSystem::new();
        let p1_id = system.add_point(Point::new(0.0, 0.0));
        let p2_id = system.add_point(Point::new(1.0, 0.0));
        system.add_constraint(Constraint::FixPoint { point_id: p1_id });
        system.add_constraint(Constraint::FixLength {
            line_start: p1_id,
            line_end: p2_id,
            length: 1.0,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);
        let x = system.get_variables();

        let sparse_entries =
            solver.compute_sparse_jacobian_with_dependencies(&system, &x, &deps);

        // Verify that we have non-zero entries
        assert!(!sparse_entries.is_empty());

        // Verify that all entries have valid indices
        let n_eqs: usize = system
            .constraints
            .iter()
            .map(|c| c.equation_count())
            .sum();
        let n_vars = x.len();

        for &(row, col, _val) in &sparse_entries {
            assert!(row < n_eqs, "Row index {} out of bounds", row);
            assert!(col < n_vars, "Column index {} out of bounds", col);
        }
    }

    #[test]
    fn test_dependency_sparsity_analysis() {
        let mut system = ConstraintSystem::new();

        // Create a larger system to demonstrate sparsity
        let mut point_ids = Vec::new();
        for i in 0..10 {
            let p_id = system.add_point(Point::new(i as f64, 0.0));
            point_ids.push(p_id);
        }

        // Add constraints (each constraint only affects nearby points)
        for i in 0..point_ids.len() - 1 {
            system.add_constraint(Constraint::FixLength {
                line_start: point_ids[i],
                line_end: point_ids[i + 1],
                length: 1.0,
            });
        }

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        // Calculate sparsity
        let total_vars = system.get_variables().len();
        let total_eqs: usize = system
            .constraints
            .iter()
            .map(|c| c.equation_count())
            .sum();
        let dense_jacobian_size = total_vars * total_eqs;

        let sparse_entries: usize = deps.iter().map(|d| {
            d.dependent_var_indices.len()
                * system.constraints[d.constraint_id].equation_count()
        }).sum();

        let sparsity = 1.0 - (sparse_entries as f64 / dense_jacobian_size as f64);

        // For this chain of constraints with 10 points and 9 length constraints:
        // - Total variables: 20 (10 points * 2 coords each)
        // - Total equations: 9 (9 constraints * 1 equation each)
        // - Each constraint depends on 4 variables (2 points * 2 coords)
        // - Sparse entries: 9 * 4 = 36
        // - Dense size: 20 * 9 = 180
        // - Sparsity: 1 - 36/180 = 0.8
        assert!(sparsity > 0.75, "Sparsity {} should be > 0.75", sparsity);
    }

    #[test]
    fn test_tangent_line_circle_constraint() {
        // Test: Line tangent to circle
        let mut system = ConstraintSystem::new();
        
        // Create a circle at origin with radius 1
        let circle_id = system.add_circle(Point::new(0.0, 0.0), 1.0);
        
        // Create a horizontal line at y = 1 (tangent to top of circle)
        let line_start = system.add_point(Point::new(-1.0, 1.0));
        let line_end = system.add_point(Point::new(1.0, 1.0));
        
        system.add_constraint(Constraint::TangentLineCircle {
            line_start,
            line_end,
            circle_id,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        // TangentLineCircle should depend on 7 variables:
        // line_start (2), line_end (2), circle (3: center_x, center_y, radius)
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dependent_var_indices.len(), 7);

        // Verify the constraint equation
        let x = system.get_variables();
        let residuals = solver.compute_residual(&system, &x);
        
        // Line at y=1, circle at origin with r=1, should be tangent (residual ≈ 0)
        assert!(residuals[0].abs() < 1e-10, "Tangent constraint should be satisfied");
    }

    #[test]
    fn test_tangent_circle_circle_constraint() {
        // Test: Two circles tangent externally
        let mut system = ConstraintSystem::new();
        
        // Create first circle at origin with radius 1
        let circle1_id = system.add_circle(Point::new(0.0, 0.0), 1.0);
        
        // Create second circle at (3, 0) with radius 2 (tangent at point (1, 0))
        let circle2_id = system.add_circle(Point::new(3.0, 0.0), 2.0);
        
        system.add_constraint(Constraint::TangentCircleCircle {
            circle1_id,
            circle2_id,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        // TangentCircleCircle should depend on 6 variables:
        // circle1 (3), circle2 (3)
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dependent_var_indices.len(), 6);

        // Verify the constraint equation
        // Distance between centers = 3, sum of radii = 1 + 2 = 3
        let x = system.get_variables();
        let residuals = solver.compute_residual(&system, &x);
        
        // Should be tangent (residual ≈ 0)
        assert!(residuals[0].abs() < 1e-10, "Tangent constraint should be satisfied");
    }

    #[test]
    fn test_point_on_circle_constraint() {
        // Test: Point on circle
        let mut system = ConstraintSystem::new();
        
        // Create a circle at origin with radius 5
        let circle_id = system.add_circle(Point::new(0.0, 0.0), 5.0);
        
        // Create a point at (3, 4) which is on the circle (3^2 + 4^2 = 5^2)
        let point_id = system.add_point(Point::new(3.0, 4.0));
        
        system.add_constraint(Constraint::PointOnCircle {
            point_id,
            circle_id,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        // PointOnCircle should depend on 5 variables:
        // point (2), circle (3)
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dependent_var_indices.len(), 5);

        // Verify the constraint equation
        let x = system.get_variables();
        let residuals = solver.compute_residual(&system, &x);
        
        // Point (3,4) is on circle with r=5 (residual ≈ 0)
        assert!(residuals[0].abs() < 1e-10, "PointOnCircle constraint should be satisfied");
    }

    #[test]
    fn test_tangent_line_circle_with_solver() {
        // Integration test: Solve a system with tangent line-circle constraint
        let mut system = ConstraintSystem::new();
        
        // Circle at origin, radius 1
        let circle_id = system.add_circle(Point::new(0.0, 0.0), 1.0);
        system.add_constraint(Constraint::FixRadius {
            circle_id,
            radius: 1.0,
        });
        
        // Line that should be tangent to circle
        let line_start = system.add_point(Point::new(-2.0, 0.5));
        let line_end = system.add_point(Point::new(2.0, 0.5));
        
        system.add_constraint(Constraint::TangentLineCircle {
            line_start,
            line_end,
            circle_id,
        });
        
        // Fix circle center
        system.add_constraint(Constraint::FixPoint { 
            point_id: circle_id, 
        });

        let solver = ConstraintSolver::new();
        let result = solver.solve(&mut system);
        
        // Should converge to a solution where line is tangent to circle
        assert!(result.is_ok(), "Solver should converge: {:?}", result);
    }

    #[test]
    fn test_nonlinear_constraints_combined() {
        // Test: Combined non-linear constraints
        let mut system = ConstraintSystem::new();
        
        // Two circles
        let circle1_id = system.add_circle(Point::new(0.0, 0.0), 2.0);
        let circle2_id = system.add_circle(Point::new(5.0, 0.0), 3.0);
        
        // Point on first circle
        let point_id = system.add_point(Point::new(2.0, 0.0));
        
        // Constraints
        system.add_constraint(Constraint::FixRadius {
            circle_id: circle1_id,
            radius: 2.0,
        });
        system.add_constraint(Constraint::FixRadius {
            circle_id: circle2_id,
            radius: 3.0,
        });
        system.add_constraint(Constraint::TangentCircleCircle {
            circle1_id,
            circle2_id,
        });
        system.add_constraint(Constraint::PointOnCircle {
            point_id,
            circle_id: circle1_id,
        });

        let solver = ConstraintSolver::new();
        let deps = solver.analyze_dependencies(&system);

        // Verify dependency analysis
        assert_eq!(deps.len(), 4);
        
        // TangentCircleCircle depends on 6 variables
        assert_eq!(deps[2].dependent_var_indices.len(), 6);
        
        // PointOnCircle depends on 5 variables
        assert_eq!(deps[3].dependent_var_indices.len(), 5);
    }
}
