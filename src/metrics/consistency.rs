//! 几何一致性指标
//!
//! 评估生成的 DXF 文件中线段是否闭合、几何关系是否正确

use crate::geometry::{Line, Point, Polygon, Primitive};
use serde::{Deserialize, Serialize};

/// 几何一致性评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyResult {
    /// 是否通过检查
    pub passed: bool,
    /// 总体得分（0-1）
    pub score: f64,
    /// 详细检查结果
    pub checks: Vec<CheckResult>,
    /// 错误列表
    pub errors: Vec<String>,
}

/// 单项检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// 检查名称
    pub name: String,
    /// 是否通过
    pub passed: bool,
    /// 得分
    pub score: f64,
    /// 详细信息
    pub details: String,
}

/// 几何一致性检查器
pub struct ConsistencyChecker {
    /// 距离容差
    pub distance_tolerance: f64,
    /// 角度容差（度）
    pub angle_tolerance: f64,
}

impl ConsistencyChecker {
    pub fn new() -> Self {
        Self {
            distance_tolerance: 1.0,
            angle_tolerance: 1.0,
        }
    }

    /// 执行所有一致性检查
    pub fn check_all(&self, primitives: &[Primitive]) -> ConsistencyResult {
        let mut checks = Vec::new();
        let mut errors = Vec::new();
        let mut scores = Vec::new();

        // 检查 1: 回路闭合性
        let loop_check = self.check_loop_closure(primitives);
        scores.push(loop_check.score);
        if !loop_check.passed {
            errors.push(format!("回路闭合检查失败：{}", loop_check.details));
        }
        checks.push(loop_check);

        // 检查 2: 线段连接性
        let connection_check = self.check_line_connections(primitives);
        scores.push(connection_check.score);
        if !connection_check.passed {
            errors.push(format!("线段连接检查失败：{}", connection_check.details));
        }
        checks.push(connection_check);

        // 检查 3: 直角关系（建筑图纸中墙通常是垂直的）
        let ortho_check = self.check_orthogonality(primitives);
        scores.push(ortho_check.score);
        checks.push(ortho_check);

        // 检查 4: 平行关系
        let parallel_check = self.check_parallel_walls(primitives);
        scores.push(parallel_check.score);
        checks.push(parallel_check);

        // 检查 5: 重叠检测
        let overlap_check = self.check_overlaps(primitives);
        scores.push(overlap_check.score);
        if !overlap_check.passed {
            errors.push(format!("重叠检测警告：{}", overlap_check.details));
        }
        checks.push(overlap_check);

        let avg_score = if scores.is_empty() {
            1.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        };

        ConsistencyResult {
            passed: errors.is_empty(),
            score: avg_score,
            checks,
            errors,
        }
    }

    /// 检查回路闭合性
    fn check_loop_closure(&self, primitives: &[Primitive]) -> CheckResult {
        let polygons = self.extract_polygons(primitives);

        if polygons.is_empty() {
            return CheckResult {
                name: "loop_closure".to_string(),
                passed: true,
                score: 1.0,
                details: "未检测到多边形".to_string(),
            };
        }

        let mut unclosed_count = 0;
        for poly in &polygons {
            if !poly.closed {
                // 检查首尾点是否重合
                if let (Some(first), Some(last)) = (poly.vertices.first(), poly.vertices.last()) {
                    if first.distance(last) > self.distance_tolerance {
                        unclosed_count += 1;
                    }
                }
            }
        }

        let passed = unclosed_count == 0;
        let score = 1.0 - (unclosed_count as f64 / polygons.len() as f64);

        CheckResult {
            name: "loop_closure".to_string(),
            passed,
            score,
            details: if passed {
                "所有回路闭合良好".to_string()
            } else {
                format!("{} 个回路未闭合", unclosed_count)
            },
        }
    }

    /// 检查线段连接性
    fn check_line_connections(&self, primitives: &[Primitive]) -> CheckResult {
        let lines = self.extract_lines(primitives);

        if lines.is_empty() {
            return CheckResult {
                name: "line_connections".to_string(),
                passed: true,
                score: 1.0,
                details: "未检测到线段".to_string(),
            };
        }

        let mut disconnected_count = 0;
        for (i, line) in lines.iter().enumerate() {
            let mut start_connected = false;
            let mut end_connected = false;

            for (j, other) in lines.iter().enumerate() {
                if i == j {
                    continue;
                }

                // 检查起点是否连接
                if line.start.distance(&other.start) < self.distance_tolerance
                    || line.start.distance(&other.end) < self.distance_tolerance
                {
                    start_connected = true;
                }

                // 检查终点是否连接
                if line.end.distance(&other.start) < self.distance_tolerance
                    || line.end.distance(&other.end) < self.distance_tolerance
                {
                    end_connected = true;
                }
            }

            if !start_connected || !end_connected {
                disconnected_count += 1;
            }
        }

        let passed = disconnected_count == 0;
        let score = 1.0 - (disconnected_count as f64 / lines.len() as f64);

        CheckResult {
            name: "line_connections".to_string(),
            passed,
            score,
            details: if passed {
                "所有线段连接良好".to_string()
            } else {
                format!("{} 条线段存在连接问题", disconnected_count)
            },
        }
    }

    /// 检查直角关系
    fn check_orthogonality(&self, primitives: &[Primitive]) -> CheckResult {
        let lines = self.extract_lines(primitives);

        if lines.len() < 2 {
            return CheckResult {
                name: "orthogonality".to_string(),
                passed: true,
                score: 1.0,
                details: "线段数量不足".to_string(),
            };
        }

        let mut non_ortho_count = 0;
        let mut checked_pairs = 0;

        for (i, line1) in lines.iter().enumerate() {
            for (j, line2) in lines.iter().enumerate() {
                if i >= j {
                    continue;
                }

                // 检查相连的线段
                if line1.end.distance(&line2.start) < self.distance_tolerance
                    || line1.end.distance(&line2.end) < self.distance_tolerance
                {
                    checked_pairs += 1;

                    let dir1 = line1.direction();
                    let dir2 = line2.direction();
                    let dot = dir1.x * dir2.x + dir1.y * dir2.y;
                    let angle = (dot).abs().acos().to_degrees();

                    // 检查是否接近 90 度
                    if (angle - 90.0).abs() > self.angle_tolerance
                        && (angle).abs() > self.angle_tolerance
                        && (angle - 180.0).abs() > self.angle_tolerance
                    {
                        non_ortho_count += 1;
                    }
                }
            }
        }

        let passed = non_ortho_count == 0 || checked_pairs == 0;
        let score = if checked_pairs > 0 {
            1.0 - (non_ortho_count as f64 / checked_pairs as f64)
        } else {
            1.0
        };

        CheckResult {
            name: "orthogonality".to_string(),
            passed,
            score,
            details: format!(
                "检查了 {} 对相连线段，{} 对非正交/平行",
                checked_pairs, non_ortho_count
            ),
        }
    }

    /// 检查平行墙
    fn check_parallel_walls(&self, primitives: &[Primitive]) -> CheckResult {
        let lines = self.extract_lines(primitives);

        // 简化实现：统计主要方向的线段
        let mut horizontal_count = 0;
        let mut vertical_count = 0;

        for line in &lines {
            let dir = line.direction();
            if dir.x.abs() > dir.y.abs() {
                horizontal_count += 1;
            } else {
                vertical_count += 1;
            }
        }

        let total = horizontal_count + vertical_count;
        let score = if total > 0 {
            horizontal_count.max(vertical_count) as f64 / total as f64
        } else {
            1.0
        };

        CheckResult {
            name: "parallel_walls".to_string(),
            passed: true,
            score,
            details: format!(
                "水平线段：{}, 垂直线段：{}, 对齐度：{:.1}%",
                horizontal_count,
                vertical_count,
                score * 100.0
            ),
        }
    }

    /// 检查重叠
    fn check_overlaps(&self, primitives: &[Primitive]) -> CheckResult {
        let lines = self.extract_lines(primitives);
        let mut overlap_count = 0;

        for (i, line1) in lines.iter().enumerate() {
            for (j, line2) in lines.iter().enumerate() {
                if i >= j {
                    continue;
                }

                if self.lines_overlap(line1, line2) {
                    overlap_count += 1;
                }
            }
        }

        CheckResult {
            name: "overlaps".to_string(),
            passed: overlap_count == 0,
            score: if overlap_count > 0 { 0.5 } else { 1.0 },
            details: if overlap_count > 0 {
                format!("检测到 {} 处可能的重叠", overlap_count)
            } else {
                "未检测到重叠".to_string()
            },
        }
    }

    fn lines_overlap(&self, line1: &Line, line2: &Line) -> bool {
        // 简化检查：共线且有重叠
        let dir1 = line1.direction();
        let dir2 = line2.direction();

        // 检查是否平行（可能共线）
        let cross = dir1.x * dir2.y - dir1.y * dir2.x;
        if cross.abs() > 0.01 {
            return false;
        }

        // 检查是否有公共点
        if line1.start.distance(&line2.start) < self.distance_tolerance
            || line1.start.distance(&line2.end) < self.distance_tolerance
            || line1.end.distance(&line2.start) < self.distance_tolerance
            || line1.end.distance(&line2.end) < self.distance_tolerance
        {
            return false; // 仅共享端点不算重叠
        }

        // 检查投影重叠
        let t1 = self.project_point_on_line(&line2.start, line1);
        let t2 = self.project_point_on_line(&line2.end, line1);

        let t_min = t1.min(t2);
        let t_max = t1.max(t2);

        // 如果投影超出线段范围，则有重叠
        (t_min < 0.0 || t_max > 1.0) && (t_min < 1.0 && t_max > 0.0)
    }

    fn project_point_on_line(&self, point: &Point, line: &Line) -> f64 {
        let dx = line.end.x - line.start.x;
        let dy = line.end.y - line.start.y;
        let len_sq = dx * dx + dy * dy;

        if len_sq == 0.0 {
            return 0.0;
        }

        ((point.x - line.start.x) * dx + (point.y - line.start.y) * dy) / len_sq
    }

    fn extract_lines(&self, primitives: &[Primitive]) -> Vec<Line> {
        let mut lines = Vec::new();

        for prim in primitives {
            match prim {
                Primitive::Line(line) => lines.push(line.clone()),
                Primitive::Polygon(poly) => lines.extend(poly.to_lines()),
                Primitive::Rect(rect) => lines.extend(rect.to_polygon().to_lines()),
                _ => {}
            }
        }

        lines
    }

    fn extract_polygons(&self, primitives: &[Primitive]) -> Vec<Polygon> {
        let mut polygons = Vec::new();

        for prim in primitives {
            match prim {
                Primitive::Polygon(poly) => polygons.push(poly.clone()),
                Primitive::Rect(rect) => polygons.push(rect.to_polygon()),
                _ => {}
            }
        }

        polygons
    }
}

impl Default for ConsistencyChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 使用 tokitai 工具封装
#[derive(Default, Clone)]
pub struct ConsistencyTools;

use tokitai::tool;

#[tool]
impl ConsistencyTools {
    /// 执行一致性检查
    #[tool]
    pub fn check_consistency(&self, primitives: Vec<Primitive>) -> ConsistencyResult {
        let checker = ConsistencyChecker::new();
        checker.check_all(&primitives)
    }

    /// 获取一致性得分
    #[tool]
    pub fn get_consistency_score(&self, primitives: Vec<Primitive>) -> f64 {
        let checker = ConsistencyChecker::new();
        checker.check_all(&primitives).score
    }
}
