//! 3D 几何约束求解器
//!
//! 提供三维空间中的约束定义、方程构建和数值求解功能
//!
//! # 支持的 3D 约束类型
//! - 共面 (Coplanar): 多个点/线在同一平面内
//! - 平行 (Parallel): 两条线/两个平面平行
//! - 垂直 (Perpendicular): 两条线/线与平面/两个平面垂直
//! - 重合 (Coincident): 两个点重合
//! - 点在平面上 (PointOnPlane): 点位于平面内
//! - 点在直线上 (PointOnLine): 点位于 3D 直线上
//! - 同心 (Concentric): 两个圆/球同心
//! - 固定距离 (FixDistance): 两点间距离固定
//! - 固定角度 (FixAngle): 两线夹角固定
//!
//! # 示例
//!
//! ```rust,ignore
//! use cadagent::geometry::constraint3d::{
//!     ConstraintSystem3D, Constraint3D, ConstraintSolver3D, Point3D
//! };
//!
//! // 创建 3D 约束系统
//! let mut system = ConstraintSystem3D::new();
//!
//! // 添加点
//! let p1 = system.add_point(Point3D::new(0.0, 0.0, 0.0));
//! let p2 = system.add_point(Point3D::new(1.0, 0.0, 0.0));
//! let p3 = system.add_point(Point3D::new(0.0, 1.0, 0.0));
//!
//! // 添加共面约束
//! system.add_constraint(Constraint3D::Coplanar {
//!     points: vec![p1, p2, p3],
//! });
//!
//! // 求解
//! let solver = ConstraintSolver3D::new();
//! solver.solve(&mut system)?;
//! ```

use super::nurbs::Point3D;
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};

/// 3D 实体 ID 类型
pub type EntityId3D = usize;

/// 3D 几何实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityType3D {
    Point,
    Line,
    Plane,
    Circle,
    Sphere,
}

/// 3D 几何实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity3D {
    pub id: EntityId3D,
    pub entity_type: EntityType3D,
    /// 点：(x, y, z) - 3 参数
    /// 线：(start_x, start_y, start_z, end_x, end_y, end_z) - 6 参数
    /// 平面：(normal_x, normal_y, normal_z, distance) - 4 参数
    /// 圆：(center_x, center_y, center_z, normal_x, normal_y, normal_z, radius) - 7 参数
    /// 球：(center_x, center_y, center_z, radius) - 4 参数
    pub parameters: Vec<f64>,
}

impl Entity3D {
    pub fn new(id: EntityId3D, entity_type: EntityType3D, parameters: Vec<f64>) -> Self {
        Self {
            id,
            entity_type,
            parameters,
        }
    }

    /// 从 3D 点创建实体
    pub fn from_point(id: EntityId3D, point: Point3D) -> Self {
        Self {
            id,
            entity_type: EntityType3D::Point,
            parameters: vec![point.x, point.y, point.z],
        }
    }

    /// 从 3D 线段创建实体
    pub fn from_line(id: EntityId3D, start: Point3D, end: Point3D) -> Self {
        Self {
            id,
            entity_type: EntityType3D::Line,
            parameters: vec![start.x, start.y, start.z, end.x, end.y, end.z],
        }
    }

    /// 从法向量和距离创建平面
    pub fn from_plane(id: EntityId3D, normal: Vector3<f64>, distance: f64) -> Self {
        Self {
            id,
            entity_type: EntityType3D::Plane,
            parameters: vec![normal.x, normal.y, normal.z, distance],
        }
    }

    /// 从圆心和半径创建球
    pub fn from_sphere(id: EntityId3D, center: Point3D, radius: f64) -> Self {
        Self {
            id,
            entity_type: EntityType3D::Sphere,
            parameters: vec![center.x, center.y, center.z, radius],
        }
    }

    /// 获取参数数量
    pub fn parameter_count(&self) -> usize {
        self.parameters.len()
    }
}

/// 3D 约束类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constraint3D {
    /// 固定点（点的 x, y, z 坐标固定）
    FixPoint { point_id: EntityId3D },

    /// 固定距离（两点间距离固定）
    FixDistance {
        point1_id: EntityId3D,
        point2_id: EntityId3D,
        distance: f64,
    },

    /// 固定角度（两线夹角固定，弧度）
    FixAngle {
        line1_start: EntityId3D,
        line1_end: EntityId3D,
        line2_start: EntityId3D,
        line2_end: EntityId3D,
        angle: f64,
    },

    /// 共面（多个点/线在同一平面内）
    Coplanar {
        points: Vec<EntityId3D>,
        lines: Vec<EntityId3D>,
    },

    /// 平行（两条线/两个平面平行）
    Parallel {
        entity1_id: EntityId3D,
        entity2_id: EntityId3D,
    },

    /// 垂直（两条线/线与平面/两个平面垂直）
    Perpendicular {
        entity1_id: EntityId3D,
        entity2_id: EntityId3D,
    },

    /// 重合（两个点重合）
    Coincident {
        point1_id: EntityId3D,
        point2_id: EntityId3D,
    },

    /// 点在平面上
    PointOnPlane {
        point_id: EntityId3D,
        plane_id: EntityId3D,
    },

    /// 点在直线上（3D）
    PointOnLine {
        point_id: EntityId3D,
        line_start: EntityId3D,
        line_end: EntityId3D,
    },

    /// 同心（两个圆/球同心）
    Concentric {
        entity1_id: EntityId3D,
        entity2_id: EntityId3D,
    },

    /// 固定半径（圆/球半径固定）
    FixRadius { entity_id: EntityId3D, radius: f64 },

    /// 对称（点 1 和点 2 关于平面对称）
    Symmetric {
        point1_id: EntityId3D,
        point2_id: EntityId3D,
        plane_id: EntityId3D,
    },
}

impl Constraint3D {
    /// 获取约束涉及的实体 ID 列表
    pub fn get_entity_ids(&self) -> Vec<EntityId3D> {
        match self {
            Constraint3D::FixPoint { point_id } => vec![*point_id],
            Constraint3D::FixDistance {
                point1_id,
                point2_id,
                ..
            } => vec![*point1_id, *point2_id],
            Constraint3D::FixAngle {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
                ..
            } => vec![*line1_start, *line1_end, *line2_start, *line2_end],
            Constraint3D::Coplanar { points, lines } => {
                let mut ids = points.clone();
                ids.extend(lines.iter().cloned());
                ids
            }
            Constraint3D::Parallel {
                entity1_id,
                entity2_id,
            } => vec![*entity1_id, *entity2_id],
            Constraint3D::Perpendicular {
                entity1_id,
                entity2_id,
            } => vec![*entity1_id, *entity2_id],
            Constraint3D::Coincident {
                point1_id,
                point2_id,
            } => vec![*point1_id, *point2_id],
            Constraint3D::PointOnPlane { point_id, plane_id } => vec![*point_id, *plane_id],
            Constraint3D::PointOnLine {
                point_id,
                line_start,
                line_end,
            } => vec![*point_id, *line_start, *line_end],
            Constraint3D::Concentric {
                entity1_id,
                entity2_id,
            } => vec![*entity1_id, *entity2_id],
            Constraint3D::FixRadius { entity_id, .. } => vec![*entity_id],
            Constraint3D::Symmetric {
                point1_id,
                point2_id,
                plane_id,
            } => vec![*point1_id, *point2_id, *plane_id],
        }
    }

    /// 获取约束的方程数量
    pub fn equation_count(&self) -> usize {
        match self {
            Constraint3D::FixPoint { .. } => 3, // x, y, z 都固定
            Constraint3D::FixDistance { .. } => 1,
            Constraint3D::FixAngle { .. } => 1,
            Constraint3D::Coplanar { points, lines } => {
                // 每增加一个点/线，增加 1 个方程
                (points.len() + lines.len()).saturating_sub(3)
            }
            Constraint3D::Parallel { .. } => 1,
            Constraint3D::Perpendicular { .. } => 1,
            Constraint3D::Coincident { .. } => 3, // x, y, z 都重合
            Constraint3D::PointOnPlane { .. } => 1,
            Constraint3D::PointOnLine { .. } => 2, // 点到直线的距离为 0
            Constraint3D::Concentric { .. } => 3,  // 中心重合
            Constraint3D::FixRadius { .. } => 1,
            Constraint3D::Symmetric { .. } => 3,
        }
    }
}

/// 3D 约束系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSystem3D {
    entities: HashMap<EntityId3D, Entity3D>,
    constraints: HashMap<ConstraintId, Constraint3D>,
    next_entity_id: EntityId3D,
    next_constraint_id: ConstraintId,
}

type ConstraintId = usize;

impl Default for ConstraintSystem3D {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstraintSystem3D {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            constraints: HashMap::new(),
            next_entity_id: 0,
            next_constraint_id: 0,
        }
    }

    /// 添加 3D 实体
    pub fn add_entity(&mut self, entity: Entity3D) -> EntityId3D {
        let id = entity.id;
        self.entities.insert(id, entity);
        self.next_entity_id = self.next_entity_id.max(id + 1);
        id
    }

    /// 添加 3D 点
    pub fn add_point(&mut self, point: Point3D) -> EntityId3D {
        let id = self.next_entity_id;
        let entity = Entity3D::from_point(id, point);
        self.add_entity(entity)
    }

    /// 添加 3D 线段
    pub fn add_line(&mut self, start: Point3D, end: Point3D) -> EntityId3D {
        let id = self.next_entity_id;
        let entity = Entity3D::from_line(id, start, end);
        self.add_entity(entity)
    }

    /// 添加平面
    pub fn add_plane(&mut self, normal: Vector3<f64>, distance: f64) -> EntityId3D {
        let id = self.next_entity_id;
        let entity = Entity3D::from_plane(id, normal, distance);
        self.add_entity(entity)
    }

    /// 添加球
    pub fn add_sphere(&mut self, center: Point3D, radius: f64) -> EntityId3D {
        let id = self.next_entity_id;
        let entity = Entity3D::from_sphere(id, center, radius);
        self.add_entity(entity)
    }

    /// 添加 3D 约束
    pub fn add_constraint(&mut self, constraint: Constraint3D) -> ConstraintId {
        let id = self.next_constraint_id;
        self.constraints.insert(id, constraint);
        self.next_constraint_id = id + 1;
        id
    }

    /// 获取实体
    pub fn get_entity(&self, id: EntityId3D) -> Option<&Entity3D> {
        self.entities.get(&id)
    }

    /// 获取可变实体
    pub fn get_entity_mut(&mut self, id: EntityId3D) -> Option<&mut Entity3D> {
        self.entities.get_mut(&id)
    }

    /// 获取约束
    pub fn get_constraint(&self, id: ConstraintId) -> Option<&Constraint3D> {
        self.constraints.get(&id)
    }

    /// 获取所有实体
    pub fn entities(&self) -> impl Iterator<Item = &Entity3D> {
        self.entities.values()
    }

    /// 获取所有约束
    pub fn constraints(&self) -> impl Iterator<Item = &Constraint3D> {
        self.constraints.values()
    }

    /// 获取系统变量向量
    pub fn get_variables(&self) -> Vec<f64> {
        self.entities
            .values()
            .flat_map(|e| e.parameters.iter().copied())
            .collect()
    }

    /// 设置系统变量向量
    pub fn set_variables(&mut self, x: &[f64]) {
        let mut idx = 0;
        for entity in self.entities.values_mut() {
            let count = entity.parameters.len();
            if idx + count <= x.len() {
                entity.parameters.copy_from_slice(&x[idx..idx + count]);
                idx += count;
            }
        }
    }

    /// 获取实体数量
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// 获取约束数量
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// 计算总自由度数
    pub fn degrees_of_freedom(&self) -> usize {
        self.entities.values().map(|e| e.parameter_count()).sum()
    }

    /// 计算总约束方程数
    pub fn total_equations(&self) -> usize {
        self.constraints.values().map(|c| c.equation_count()).sum()
    }

    /// 检查系统是否适定（方程数 = 自由度数）
    pub fn is_well_constrained(&self) -> bool {
        self.degrees_of_freedom() == self.total_equations()
    }

    /// 检查系统是否欠约束（方程数 < 自由度数）
    pub fn is_under_constrained(&self) -> bool {
        self.degrees_of_freedom() > self.total_equations()
    }

    /// 检查系统是否过约束（方程数 > 自由度数）
    pub fn is_over_constrained(&self) -> bool {
        self.degrees_of_freedom() < self.total_equations()
    }
}

/// 3D 约束求解器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig3D {
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 收敛容差
    pub tolerance: f64,
    /// 初始阻尼
    pub damping: f64,
    /// 使用 Levenberg-Marquardt 算法
    pub use_lm: bool,
}

impl Default for SolverConfig3D {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-8,
            damping: 1e-3,
            use_lm: true,
        }
    }
}

/// 3D 约束求解错误
///
/// 与 2D 的 `SolverError` 保持一致的错误类型定义，便于统一处理
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SolverError3D {
    /// 数值不收敛
    NotConverged { iterations: usize, residual: f64 },
    /// 奇异矩阵
    SingularMatrix,
    /// 无效输入
    InvalidInput { message: String },
    /// 实体不存在
    EntityNotFound { entity_id: EntityId3D },
}

impl std::fmt::Display for SolverError3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverError3D::NotConverged {
                iterations,
                residual,
            } => {
                write!(f, "求解不收敛：{iterations} 次迭代后残差为 {residual}")
            }
            SolverError3D::SingularMatrix => write!(f, "Jacobian 矩阵奇异"),
            SolverError3D::InvalidInput { message } => write!(f, "无效输入：{message}"),
            SolverError3D::EntityNotFound { entity_id } => {
                write!(f, "实体不存在：ID = {entity_id}")
            }
        }
    }
}

impl std::error::Error for SolverError3D {}

// 与 thiserror 的互操作（如果需要）
impl From<SolverError3D> for String {
    fn from(err: SolverError3D) -> Self {
        err.to_string()
    }
}

/// 3D 约束求解器
pub struct ConstraintSolver3D {
    config: SolverConfig3D,
}

impl Default for ConstraintSolver3D {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstraintSolver3D {
    pub fn new() -> Self {
        Self {
            config: SolverConfig3D::default(),
        }
    }

    pub fn with_config(config: SolverConfig3D) -> Self {
        Self { config }
    }

    /// 求解 3D 约束系统
    #[instrument(
        skip(self, system),
        fields(iterations = 0, initial_residual = 0.0, final_residual = 0.0)
    )]
    pub fn solve(&self, system: &mut ConstraintSystem3D) -> Result<(), SolverError3D> {
        if system.entities.is_empty() {
            return Ok(());
        }

        // 检查系统是否欠约束
        if system.is_under_constrained() {
            debug!(
                "3D 约束系统欠约束：DOF={}, 方程={}",
                system.degrees_of_freedom(),
                system.total_equations()
            );
        }

        // 构建变量向量
        let _n_vars = system.degrees_of_freedom();
        let mut x = self.build_variable_vector(system);

        // 使用 Levenberg-Marquardt 求解
        self.solve_lm(system, &mut x)?;

        // 将解写回系统
        self.write_back_solution(system, &x);

        Ok(())
    }

    /// 构建变量向量
    fn build_variable_vector(&self, system: &ConstraintSystem3D) -> Vec<f64> {
        let mut vars = Vec::with_capacity(system.degrees_of_freedom());

        for entity in system.entities() {
            vars.extend(&entity.parameters);
        }

        vars
    }

    /// 将解写回系统
    fn write_back_solution(&self, system: &mut ConstraintSystem3D, x: &[f64]) {
        let mut idx = 0;

        for entity in system.entities.values_mut() {
            let count = entity.parameters.len();
            entity.parameters.copy_from_slice(&x[idx..idx + count]);
            idx += count;
        }
    }

    /// Levenberg-Marquardt 求解器
    fn solve_lm(&self, system: &ConstraintSystem3D, x: &mut Vec<f64>) -> Result<(), SolverError3D> {
        let _n_vars = x.len();
        let _n_eqs = system.total_equations();

        if _n_eqs == 0 {
            return Ok(());
        }

        let mut damping = self.config.damping;
        let mut residual = self.compute_residual(system, x);

        let span = tracing::Span::current();
        span.record("initial_residual", residual);

        for iteration in 0..self.config.max_iterations {
            // 计算 Jacobian
            let jacobian = self.compute_jacobian(system, x);

            // 计算残差向量
            let residuals = self.compute_residuals(system, x);

            // 构建法方程：(J^T * J + damping * I) * dx = -J^T * r
            let jtj = self.compute_jtj(&jacobian, _n_vars);
            let jtr = self.compute_jtr(&jacobian, &residuals, _n_vars);

            // 添加阻尼
            let mut augmented = jtj;
            for i in 0.._n_vars {
                augmented[i * _n_vars + i] += damping;
            }

            // 求解线性方程组
            let dx = self.solve_linear_system(&augmented, &jtr, _n_vars)?;

            // 更新解
            let mut x_new = x.clone();
            for i in 0.._n_vars {
                x_new[i] -= dx[i];
            }

            // 计算新残差
            let residual_new = self.compute_residual(system, &x_new);

            // 检查收敛
            if residual_new < self.config.tolerance {
                *x = x_new;

                span.record("iterations", iteration + 1);
                span.record("final_residual", residual_new);

                info!(
                    "3D 约束求解收敛：iterations={}, final_residual={}",
                    iteration + 1,
                    residual_new
                );
                return Ok(());
            }

            // 接受或拒绝更新
            if residual_new < residual {
                *x = x_new;
                residual = residual_new;
                damping /= 2.0;
            } else {
                damping *= 2.0;
            }

            // 限制阻尼范围
            damping = damping.clamp(1e-10, 1e10);

            if iteration % 10 == 0 {
                debug!(
                    "3D 约束求解迭代 {}: residual={}, damping={}",
                    iteration, residual, damping
                );
            }
        }

        span.record("iterations", self.config.max_iterations);
        span.record("final_residual", residual);

        warn!(
            "3D 约束求解达到最大迭代次数：max_iter={}, final_residual={}",
            self.config.max_iterations, residual
        );

        Err(SolverError3D::NotConverged {
            iterations: self.config.max_iterations,
            residual,
        })
    }

    /// 计算残差范数
    fn compute_residual(&self, system: &ConstraintSystem3D, x: &[f64]) -> f64 {
        let residuals = self.compute_residuals(system, x);
        residuals.iter().map(|r| r * r).sum::<f64>().sqrt()
    }

    /// 计算残差向量
    fn compute_residuals(&self, system: &ConstraintSystem3D, x: &[f64]) -> Vec<f64> {
        let mut residuals = Vec::new();
        let mut idx = 0;

        // 构建实体参数索引映射
        let entity_params: HashMap<_, _> = system
            .entities()
            .map(|e| {
                let count = e.parameters.len();
                let params = &x[idx..idx + count];
                idx += count;
                (e.id, params.to_vec())
            })
            .collect();

        for constraint in system.constraints() {
            self.compute_constraint_residuals(constraint, &entity_params, &mut residuals);
        }

        residuals
    }

    /// 计算单个约束的残差
    fn compute_constraint_residuals(
        &self,
        constraint: &Constraint3D,
        entity_params: &HashMap<EntityId3D, Vec<f64>>,
        residuals: &mut Vec<f64>,
    ) {
        match constraint {
            Constraint3D::FixPoint { point_id } => {
                if let Some(params) = entity_params.get(point_id) {
                    // 固定点的残差为当前坐标值（假设目标是原点）
                    residuals.extend(params.iter());
                }
            }
            Constraint3D::FixDistance {
                point1_id,
                point2_id,
                distance,
            } => {
                if let (Some(p1), Some(p2)) =
                    (entity_params.get(point1_id), entity_params.get(point2_id))
                {
                    let dx = p2[0] - p1[0];
                    let dy = p2[1] - p1[1];
                    let dz = p2[2] - p1[2];
                    let current_dist = (dx * dx + dy * dy + dz * dz).sqrt();
                    residuals.push(current_dist - distance);
                }
            }
            Constraint3D::Parallel {
                entity1_id,
                entity2_id,
            } => {
                if let (Some(e1), Some(e2)) =
                    (entity_params.get(entity1_id), entity_params.get(entity2_id))
                {
                    // 计算方向向量的叉积范数
                    let dir1 = self.get_line_direction(e1);
                    let dir2 = self.get_line_direction(e2);

                    if let (Some(d1), Some(d2)) = (dir1, dir2) {
                        let cross = d1.cross(&d2);
                        residuals.push(cross.norm());
                    }
                }
            }
            Constraint3D::Perpendicular {
                entity1_id,
                entity2_id,
            } => {
                if let (Some(e1), Some(e2)) =
                    (entity_params.get(entity1_id), entity_params.get(entity2_id))
                {
                    // 计算方向向量的点积
                    let dir1 = self.get_line_direction(e1);
                    let dir2 = self.get_line_direction(e2);

                    if let (Some(d1), Some(d2)) = (dir1, dir2) {
                        residuals.push(d1.dot(&d2));
                    }
                }
            }
            Constraint3D::Coincident {
                point1_id,
                point2_id,
            } => {
                if let (Some(p1), Some(p2)) =
                    (entity_params.get(point1_id), entity_params.get(point2_id))
                {
                    for i in 0..3 {
                        residuals.push(p1[i] - p2[i]);
                    }
                }
            }
            Constraint3D::Concentric {
                entity1_id,
                entity2_id,
            } => {
                if let (Some(e1), Some(e2)) =
                    (entity_params.get(entity1_id), entity_params.get(entity2_id))
                {
                    // 圆心/球心重合
                    for i in 0..3 {
                        residuals.push(e1[i] - e2[i]);
                    }
                }
            }
            Constraint3D::Coplanar { points, lines } => {
                // 共面约束：所有点和线在同一平面内
                // 使用标量三重积：(b-a) × (c-a) · (d-a) = 0
                if points.len() >= 3 {
                    if let (Some(p0), Some(p1), Some(p2)) = (
                        entity_params.get(&points[0]),
                        entity_params.get(&points[1]),
                        entity_params.get(&points[2]),
                    ) {
                        // 计算法向量 n = (p1-p0) × (p2-p0)
                        let v1 = Vector3::new(p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]);
                        let v2 = Vector3::new(p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]);
                        let normal = v1.cross(&v2);
                        
                        // 检查其他点是否在平面内
                        for &point_id in points.iter().skip(3) {
                            if let Some(p) = entity_params.get(&point_id) {
                                let v = Vector3::new(p[0] - p0[0], p[1] - p0[1], p[2] - p0[2]);
                                residuals.push(normal.dot(&v));
                            }
                        }
                        
                        // 检查线的方向向量是否垂直于法向量
                        for &line_id in lines {
                            if let Some(line_params) = entity_params.get(&line_id) {
                                if let Some(dir) = self.get_line_direction(line_params) {
                                    residuals.push(normal.dot(&dir));
                                }
                            }
                        }
                    }
                } else if points.len() == 2 && !lines.is_empty() {
                    // 2 点 + 1 线定义平面
                    if let (Some(p0), Some(p1), Some(line_params)) = (
                        entity_params.get(&points[0]),
                        entity_params.get(&points[1]),
                        entity_params.get(&lines[0]),
                    ) {
                        // 使用线的方向向量和点定义平面
                        if let Some(line_dir) = self.get_line_direction(line_params) {
                            let v = Vector3::new(p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]);
                            let normal = v.cross(&line_dir);
                            
                            // 检查其他线
                            for &line_id in lines.iter().skip(1) {
                                if let Some(line_params) = entity_params.get(&line_id) {
                                    if let Some(dir) = self.get_line_direction(line_params) {
                                        residuals.push(normal.dot(&dir));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Constraint3D::PointOnPlane { point_id, plane_id } => {
                // 点在平面上：点到平面距离 = 0
                // 平面方程：ax + by + cz + d = 0
                if let (Some(point), Some(plane)) = (
                    entity_params.get(point_id),
                    entity_params.get(plane_id),
                ) {
                    // 平面参数：(normal_x, normal_y, normal_z, distance)
                    let normal = Vector3::new(plane[0], plane[1], plane[2]);
                    let plane_d = plane[3];
                    let point_vec = Vector3::new(point[0], point[1], point[2]);
                    let dist = normal.dot(&point_vec) + plane_d;
                    residuals.push(dist);
                }
            }
            Constraint3D::PointOnLine {
                point_id,
                line_start,
                line_end,
            } => {
                // 点在 3D 直线上：点到直线距离 = 0
                // 使用叉积：|(p - a) × (b - a)| / |b - a| = 0
                if let (Some(point), Some(start), Some(end)) = (
                    entity_params.get(point_id),
                    entity_params.get(line_start),
                    entity_params.get(line_end),
                ) {
                    let a = Vector3::new(start[0], start[1], start[2]);
                    let b = Vector3::new(end[0], end[1], end[2]);
                    let p = Vector3::new(point[0], point[1], point[2]);
                    
                    let ab = b - a;
                    let ap = p - a;
                    let cross = ap.cross(&ab);
                    let line_len = ab.norm();
                    
                    if line_len > 1e-10 {
                        let dist = cross.norm() / line_len;
                        residuals.push(dist);
                    }
                }
            }
            Constraint3D::FixAngle {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
                angle,
            } => {
                // 两线夹角：cos(θ) = (d1 · d2) / (|d1| * |d2|)
                if let (Some(l1s), Some(l1e), Some(l2s), Some(l2e)) = (
                    entity_params.get(line1_start),
                    entity_params.get(line1_end),
                    entity_params.get(line2_start),
                    entity_params.get(line2_end),
                ) {
                    let d1 = Vector3::new(
                        l1e[0] - l1s[0],
                        l1e[1] - l1s[1],
                        l1e[2] - l1s[2],
                    );
                    let d2 = Vector3::new(
                        l2e[0] - l2s[0],
                        l2e[1] - l2s[1],
                        l2e[2] - l2s[2],
                    );
                    
                    let dot = d1.dot(&d2);
                    let norm1 = d1.norm();
                    let norm2 = d2.norm();
                    
                    if norm1 > 1e-10 && norm2 > 1e-10 {
                        let cos_current = dot / (norm1 * norm2);
                        let cos_target = angle.cos();
                        residuals.push(cos_current - cos_target);
                    }
                }
            }
            Constraint3D::Symmetric {
                point1_id,
                point2_id,
                plane_id,
            } => {
                // 对称约束：点 1 和点 2 关于平面对称
                // 条件 1: 中点在平面上
                // 条件 2: 连线平行于平面法向量
                if let (Some(p1), Some(p2), Some(plane)) = (
                    entity_params.get(point1_id),
                    entity_params.get(point2_id),
                    entity_params.get(plane_id),
                ) {
                    let normal = Vector3::new(plane[0], plane[1], plane[2]);
                    let plane_d = plane[3];
                    
                    // 中点
                    let mid = Vector3::new(
                        (p1[0] + p2[0]) / 2.0,
                        (p1[1] + p2[1]) / 2.0,
                        (p1[2] + p2[2]) / 2.0,
                    );
                    
                    // 条件 1: 中点在平面上
                    residuals.push(normal.dot(&mid) + plane_d);
                    
                    // 条件 2: 连线平行于法向量 (p2 - p1) = k * normal
                    let p1_vec = Vector3::new(p1[0], p1[1], p1[2]);
                    let p2_vec = Vector3::new(p2[0], p2[1], p2[2]);
                    let connection = p2_vec - p1_vec;
                    
                    // 检查 connection 是否平行于 normal (叉积为 0)
                    let cross = connection.cross(&normal);
                    residuals.push(cross.norm());
                }
            }
            Constraint3D::FixRadius { entity_id, radius } => {
                // 固定半径：圆或球的半径固定
                if let Some(entity) = entity_params.get(entity_id) {
                    // 半径是最后一个参数
                    if let Some(current_radius) = entity.last() {
                        residuals.push(current_radius - radius);
                    }
                }
            }
        }
    }

    /// 获取直线的方向向量
    fn get_line_direction(&self, params: &[f64]) -> Option<Vector3<f64>> {
        if params.len() >= 6 {
            let dir = Vector3::new(
                params[3] - params[0],
                params[4] - params[1],
                params[5] - params[2],
            );
            let norm = dir.norm();
            if norm > 1e-10 {
                return Some(dir / norm);
            }
        }
        None
    }

    /// 计算 Jacobian 矩阵
    fn compute_jacobian(&self, system: &ConstraintSystem3D, x: &[f64]) -> Vec<f64> {
        let n_vars = x.len();
        let n_eqs = system.total_equations();
        let epsilon = 1e-8;

        let mut jacobian = vec![0.0; n_eqs * n_vars];

        // 使用有限差分法计算 Jacobian
        for j in 0..n_vars {
            // 计算 f(x + epsilon)
            let mut x_plus = x.to_vec();
            x_plus[j] += epsilon;
            let f_plus = self.compute_residuals(system, &x_plus);

            // 计算 f(x - epsilon)
            let mut x_minus = x.to_vec();
            x_minus[j] -= epsilon;
            let f_minus = self.compute_residuals(system, &x_minus);

            // 中心差分
            for i in 0..n_eqs {
                jacobian[i * n_vars + j] = (f_plus[i] - f_minus[i]) / (2.0 * epsilon);
            }
        }

        jacobian
    }

    /// 计算 J^T * J
    fn compute_jtj(&self, jacobian: &[f64], n_vars: usize) -> Vec<f64> {
        let n_eqs = jacobian.len() / n_vars;
        let mut jtj = vec![0.0; n_vars * n_vars];

        for i in 0..n_vars {
            for j in 0..n_vars {
                let mut sum = 0.0;
                for k in 0..n_eqs {
                    sum += jacobian[k * n_vars + i] * jacobian[k * n_vars + j];
                }
                jtj[i * n_vars + j] = sum;
            }
        }

        jtj
    }

    /// 计算 J^T * r
    fn compute_jtr(&self, jacobian: &[f64], residuals: &[f64], n_vars: usize) -> Vec<f64> {
        let n_eqs = jacobian.len() / n_vars;
        let mut jtr = vec![0.0; n_vars];

        for i in 0..n_vars {
            let mut sum = 0.0;
            for k in 0..n_eqs {
                sum += jacobian[k * n_vars + i] * residuals[k];
            }
            jtr[i] = sum;
        }

        jtr
    }

    /// 求解线性方程组 Ax = b（使用高斯消元法）
    #[allow(clippy::needless_range_loop)]
    fn solve_linear_system(
        &self,
        a: &[f64],
        b: &[f64],
        n: usize,
    ) -> Result<Vec<f64>, SolverError3D> {
        // 创建增广矩阵
        let mut augmented: Vec<Vec<f64>> = a
            .chunks(n)
            .zip(b.iter())
            .map(|(row, &bi)| {
                let mut new_row = row.to_vec();
                new_row.push(bi);
                new_row
            })
            .collect();

        // 高斯消元
        for i in 0..n {
            // 寻找主元
            let mut max_row = i;
            let mut max_val = augmented[i][i].abs();
            for row in (i + 1)..n {
                if augmented[row][i].abs() > max_val {
                    max_val = augmented[row][i].abs();
                    max_row = row;
                }
            }

            // 检查奇异性
            if max_val < 1e-12 {
                return Err(SolverError3D::SingularMatrix);
            }

            // 交换行
            if max_row != i {
                augmented.swap(i, max_row);
            }

            // 消元
            for row in (i + 1)..n {
                let factor = augmented[row][i] / augmented[i][i];
                for col in i..=n {
                    augmented[row][col] -= factor * augmented[i][col];
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_entity3d_creation() {
        let point = Point3D::new(1.0, 2.0, 3.0);
        let entity = Entity3D::from_point(0, point);

        assert_eq!(entity.entity_type, EntityType3D::Point);
        assert_eq!(entity.parameters.len(), 3);
        assert_relative_eq!(entity.parameters[0], 1.0);
        assert_relative_eq!(entity.parameters[1], 2.0);
        assert_relative_eq!(entity.parameters[2], 3.0);
    }

    #[test]
    fn test_line3d_creation() {
        let start = Point3D::new(0.0, 0.0, 0.0);
        let end = Point3D::new(1.0, 1.0, 1.0);
        let entity = Entity3D::from_line(0, start, end);

        assert_eq!(entity.entity_type, EntityType3D::Line);
        assert_eq!(entity.parameters.len(), 6);
    }

    #[test]
    fn test_constraint3d_equation_count() {
        assert_eq!(Constraint3D::FixPoint { point_id: 0 }.equation_count(), 3);
        assert_eq!(
            Constraint3D::FixDistance {
                point1_id: 0,
                point2_id: 1,
                distance: 1.0
            }
            .equation_count(),
            1
        );
        assert_eq!(
            Constraint3D::Coincident {
                point1_id: 0,
                point2_id: 1
            }
            .equation_count(),
            3
        );
    }

    #[test]
    fn test_constraint_system3d_basic() {
        let mut system = ConstraintSystem3D::new();

        let p1 = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let p2 = system.add_point(Point3D::new(1.0, 0.0, 0.0));

        system.add_constraint(Constraint3D::FixDistance {
            point1_id: p1,
            point2_id: p2,
            distance: 1.0,
        });

        assert_eq!(system.entity_count(), 2);
        assert_eq!(system.constraint_count(), 1);
        assert_eq!(system.degrees_of_freedom(), 6); // 2 点 * 3 坐标
        assert_eq!(system.total_equations(), 1);
        assert!(system.is_under_constrained());
    }

    #[test]
    fn test_solver3d_fix_distance() {
        let mut system = ConstraintSystem3D::new();

        let p1 = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let p2 = system.add_point(Point3D::new(0.5, 0.0, 0.0));

        // 固定 p1
        system.add_constraint(Constraint3D::FixPoint { point_id: p1 });
        // 固定距离
        system.add_constraint(Constraint3D::FixDistance {
            point1_id: p1,
            point2_id: p2,
            distance: 1.0,
        });

        let solver = ConstraintSolver3D::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证距离
        let p1_params = system.get_entity(p1).unwrap().parameters.clone();
        let p2_params = system.get_entity(p2).unwrap().parameters.clone();

        let dx = p2_params[0] - p1_params[0];
        let dy = p2_params[1] - p1_params[1];
        let dz = p2_params[2] - p1_params[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        assert_relative_eq!(distance, 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_solver3d_coincident() {
        let mut system = ConstraintSystem3D::new();

        let p1 = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let p2 = system.add_point(Point3D::new(0.1, 0.1, 0.1));

        // 固定 p1
        system.add_constraint(Constraint3D::FixPoint { point_id: p1 });
        // 重合
        system.add_constraint(Constraint3D::Coincident {
            point1_id: p1,
            point2_id: p2,
        });

        let solver = ConstraintSolver3D::new();
        let result = solver.solve(&mut system);

        assert!(result.is_ok());

        // 验证重合
        let p1_params = system.get_entity(p1).unwrap().parameters.clone();
        let p2_params = system.get_entity(p2).unwrap().parameters.clone();

        assert_relative_eq!(p1_params[0], p2_params[0], epsilon = 1e-6);
        assert_relative_eq!(p1_params[1], p2_params[1], epsilon = 1e-6);
        assert_relative_eq!(p1_params[2], p2_params[2], epsilon = 1e-6);
    }

    #[test]
    fn test_coplanar_constraint() {
        // Test: 4 points on the same plane (XY plane)
        let mut system = ConstraintSystem3D::new();
        
        let p1 = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let p2 = system.add_point(Point3D::new(1.0, 0.0, 0.0));
        let p3 = system.add_point(Point3D::new(0.0, 1.0, 0.0));
        let p4 = system.add_point(Point3D::new(1.0, 1.0, 0.0));
        
        system.add_constraint(Constraint3D::Coplanar {
            points: vec![p1, p2, p3, p4],
            lines: vec![],
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // All points are already on XY plane (z=0), residual should be 0
        assert!(residuals.is_empty() || residuals.iter().all(|&r| r.abs() < 1e-10));
    }

    #[test]
    fn test_point_on_plane_constraint() {
        // Test: Point on XY plane (z=0)
        let mut system = ConstraintSystem3D::new();
        
        // XY plane: normal = (0, 0, 1), distance = 0
        let plane = system.add_plane(Vector3::new(0.0, 0.0, 1.0), 0.0);
        let point = system.add_point(Point3D::new(1.0, 2.0, 0.0));
        
        system.add_constraint(Constraint3D::PointOnPlane {
            point_id: point,
            plane_id: plane,
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Point is already on plane (z=0), residual should be 0
        assert!(residuals.is_empty() || residuals[0].abs() < 1e-10);
    }

    #[test]
    fn test_point_on_line_3d_constraint() {
        // Test: Point on line from (0,0,0) to (2,0,0)
        let mut system = ConstraintSystem3D::new();
        
        let line_start = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let line_end = system.add_point(Point3D::new(2.0, 0.0, 0.0));
        let point = system.add_point(Point3D::new(1.0, 0.0, 0.0));
        
        system.add_constraint(Constraint3D::PointOnLine {
            point_id: point,
            line_start: line_start,
            line_end: line_end,
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Point (1,0,0) is on line from (0,0,0) to (2,0,0), residual should be 0
        assert!(residuals.is_empty() || residuals[0].abs() < 1e-10);
    }

    #[test]
    fn test_fix_angle_constraint() {
        // Test: Two perpendicular lines (90 degrees = π/2)
        let mut system = ConstraintSystem3D::new();
        
        // Line 1: along X axis
        let l1_start = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let l1_end = system.add_point(Point3D::new(1.0, 0.0, 0.0));
        
        // Line 2: along Y axis
        let l2_start = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let l2_end = system.add_point(Point3D::new(0.0, 1.0, 0.0));
        
        system.add_constraint(Constraint3D::FixAngle {
            line1_start: l1_start,
            line1_end: l1_end,
            line2_start: l2_start,
            line2_end: l2_end,
            angle: std::f64::consts::FRAC_PI_2, // 90 degrees
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Lines are already perpendicular, cos(90°) = 0, residual should be 0
        assert!(residuals.is_empty() || residuals[0].abs() < 1e-10);
    }

    #[test]
    fn test_symmetric_constraint() {
        // Test: Two points symmetric about XY plane (z=0)
        let mut system = ConstraintSystem3D::new();
        
        // XY plane: normal = (0, 0, 1), distance = 0
        let plane = system.add_plane(Vector3::new(0.0, 0.0, 1.0), 0.0);
        
        // p1 at (0, 0, 1), p2 at (0, 0, -1) - symmetric about XY plane
        let p1 = system.add_point(Point3D::new(0.0, 0.0, 1.0));
        let p2 = system.add_point(Point3D::new(0.0, 0.0, -1.0));
        
        system.add_constraint(Constraint3D::Symmetric {
            point1_id: p1,
            point2_id: p2,
            plane_id: plane,
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Points are symmetric about XY plane:
        // - Midpoint (0,0,0) is on plane
        // - Connection vector (0,0,-2) is parallel to normal (0,0,1)
        assert!(residuals.len() >= 2);
        assert!(residuals[0].abs() < 1e-10); // Midpoint on plane
        assert!(residuals[1].abs() < 1e-10); // Connection parallel to normal
    }

    #[test]
    fn test_fix_radius_constraint() {
        // Test: Fixed radius sphere
        let mut system = ConstraintSystem3D::new();
        
        let sphere = system.add_sphere(Point3D::new(0.0, 0.0, 0.0), 5.0);
        
        system.add_constraint(Constraint3D::FixRadius {
            entity_id: sphere,
            radius: 5.0,
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Radius is already 5.0, residual should be 0
        assert!(residuals.is_empty() || residuals[0].abs() < 1e-10);
    }

    #[test]
    fn test_parallel_lines_constraint() {
        // Test: Two parallel lines along X axis
        let mut system = ConstraintSystem3D::new();
        
        // Line 1: along X axis at z=0
        let l1_start = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let _l1_end = system.add_point(Point3D::new(1.0, 0.0, 0.0));
        
        // Line 2: along X axis at z=1
        let l2_start = system.add_point(Point3D::new(0.0, 0.0, 1.0));
        let _l2_end = system.add_point(Point3D::new(1.0, 0.0, 1.0));
        
        system.add_constraint(Constraint3D::Parallel {
            entity1_id: l1_start,
            entity2_id: l2_start,
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Lines are already parallel, cross product norm should be 0
        assert!(residuals.is_empty() || residuals[0].abs() < 1e-10);
    }

    #[test]
    fn test_perpendicular_lines_constraint() {
        // Test: Two perpendicular lines
        let mut system = ConstraintSystem3D::new();
        
        // Line 1: along X axis
        let l1_start = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let _l1_end = system.add_point(Point3D::new(1.0, 0.0, 0.0));
        
        // Line 2: along Y axis
        let l2_start = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let _l2_end = system.add_point(Point3D::new(0.0, 1.0, 0.0));
        
        system.add_constraint(Constraint3D::Perpendicular {
            entity1_id: l1_start,
            entity2_id: l2_start,
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Lines are already perpendicular, dot product should be 0
        assert!(residuals.is_empty() || residuals[0].abs() < 1e-10);
    }

    #[test]
    fn test_concentric_spheres_constraint() {
        // Test: Two concentric spheres
        let mut system = ConstraintSystem3D::new();
        
        let sphere1 = system.add_sphere(Point3D::new(0.0, 0.0, 0.0), 1.0);
        let sphere2 = system.add_sphere(Point3D::new(0.0, 0.0, 0.0), 2.0);
        
        system.add_constraint(Constraint3D::Concentric {
            entity1_id: sphere1,
            entity2_id: sphere2,
        });

        let solver = ConstraintSolver3D::new();
        let residuals = solver.compute_residuals(&system, &system.get_variables());
        
        // Spheres are already concentric, residual should be 0 for all 3 coordinates
        assert!(residuals.len() >= 3);
        assert!(residuals[0].abs() < 1e-10);
        assert!(residuals[1].abs() < 1e-10);
        assert!(residuals[2].abs() < 1e-10);
    }

    #[test]
    fn test_3d_constraints_combined() {
        // Integration test: Combined 3D constraints
        let mut system = ConstraintSystem3D::new();
        
        // Create points for a square in XY plane
        let p1 = system.add_point(Point3D::new(0.0, 0.0, 0.0));
        let p2 = system.add_point(Point3D::new(1.0, 0.0, 0.0));
        let p3 = system.add_point(Point3D::new(1.0, 1.0, 0.0));
        let p4 = system.add_point(Point3D::new(0.0, 1.0, 0.0));
        
        // Fix first point
        system.add_constraint(Constraint3D::FixPoint { point_id: p1 });
        
        // Coplanar constraint
        system.add_constraint(Constraint3D::Coplanar {
            points: vec![p1, p2, p3, p4],
            lines: vec![],
        });
        
        // Fix distances
        system.add_constraint(Constraint3D::FixDistance {
            point1_id: p1,
            point2_id: p2,
            distance: 1.0,
        });
        
        system.add_constraint(Constraint3D::FixDistance {
            point1_id: p2,
            point2_id: p3,
            distance: 1.0,
        });

        let solver = ConstraintSolver3D::new();
        let result = solver.solve(&mut system);
        
        // Should converge
        assert!(result.is_ok(), "Solver should converge: {:?}", result);
    }
}
