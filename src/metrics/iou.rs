//! IoU（交并比）指标
#![allow(clippy::cast_lossless)]
//!
//! 计算房间面积与真实面积的 `IoU` 误差

use crate::geometry::{Point, Polygon, Rect};
use serde::{Deserialize, Serialize};

/// `IoU` 计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IouResult {
    /// `IoU` 值（0-1）
    pub iou: f64,
    /// 交集面积
    pub intersection: f64,
    /// 并集面积
    pub union: f64,
    /// 预测面积
    pub predicted_area: f64,
    /// 真实面积
    pub ground_truth_area: f64,
}

/// `IoU` 评估器
pub struct IouEvaluator;

impl IouEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// 计算两个多边形的 `IoU`
    pub fn calculate_iou(&self, predicted: &Polygon, ground_truth: &Polygon) -> IouResult {
        let predicted_area = predicted.area();
        let ground_truth_area = ground_truth.area();

        // 简化实现：使用包围盒近似计算 IoU
        // 完整实现需要使用多边形裁剪算法
        let (intersection, union) = self.approximate_intersection_union(predicted, ground_truth);

        let iou = if union > 0.0 {
            intersection / union
        } else {
            0.0
        };

        IouResult {
            iou,
            intersection,
            union,
            predicted_area,
            ground_truth_area,
        }
    }

    /// 计算多个房间的 `IoU`
    pub fn evaluate_rooms(
        &self,
        predicted_rooms: &[Polygon],
        ground_truth_rooms: &[Polygon],
    ) -> AggregateIouResult {
        let mut ious = Vec::new();
        let mut total_intersection = 0.0;
        let mut total_union = 0.0;

        // 为每个预测房间找到最佳匹配的真实房间
        for pred_room in predicted_rooms {
            let mut best_iou = 0.0;
            let mut best_gt_idx = None;

            for (gt_idx, gt_room) in ground_truth_rooms.iter().enumerate() {
                let result = self.calculate_iou(pred_room, gt_room);
                if result.iou > best_iou {
                    best_iou = result.iou;
                    best_gt_idx = Some(gt_idx);
                }
            }

            if let Some(idx) = best_gt_idx {
                ious.push(best_iou);
                let result = self.calculate_iou(pred_room, &ground_truth_rooms[idx]);
                total_intersection += result.intersection;
                total_union += result.union;
            }
        }

        let mean_iou = if ious.is_empty() {
            0.0
        } else {
            ious.iter().sum::<f64>() / ious.len() as f64
        };

        let overall_iou = if total_union > 0.0 {
            total_intersection / total_union
        } else {
            0.0
        };

        AggregateIouResult {
            mean_iou,
            overall_iou,
            individual_ious: ious.clone(),
            total_intersection,
            total_union,
            matched_count: ious.len(),
            total_predicted: predicted_rooms.len(),
            total_ground_truth: ground_truth_rooms.len(),
        }
    }

    /// 近似计算交集和并集（使用网格采样）
    fn approximate_intersection_union(&self, poly1: &Polygon, poly2: &Polygon) -> (f64, f64) {
        // 获取包围盒
        let bbox1 = self.get_bbox(poly1);
        let bbox2 = self.get_bbox(poly2);

        // 计算合并包围盒
        let union_bbox = Rect::new(
            Point::new(bbox1.min.x.min(bbox2.min.x), bbox1.min.y.min(bbox2.min.y)),
            Point::new(bbox1.max.x.max(bbox2.max.x), bbox1.max.y.max(bbox2.max.y)),
        );

        // 网格采样
        let grid_size = 100;
        let cell_width = union_bbox.width() / grid_size as f64;
        let cell_height = union_bbox.height() / grid_size as f64;

        let mut intersection_count = 0;
        let mut union_count = 0;

        for i in 0..grid_size {
            for j in 0..grid_size {
                let x = union_bbox.min.x + i as f64 * cell_width + cell_width / 2.0;
                let y = union_bbox.min.y + j as f64 * cell_height + cell_height / 2.0;
                let point = Point::new(x, y);

                let in_poly1 = self.point_in_polygon(&point, poly1);
                let in_poly2 = self.point_in_polygon(&point, poly2);

                if in_poly1 && in_poly2 {
                    intersection_count += 1;
                    union_count += 1;
                } else if in_poly1 || in_poly2 {
                    union_count += 1;
                }
            }
        }

        let cell_area = cell_width * cell_height;
        let intersection = intersection_count as f64 * cell_area;
        let union = union_count as f64 * cell_area;

        (intersection, union)
    }

    fn get_bbox(&self, polygon: &Polygon) -> Rect {
        if polygon.vertices.is_empty() {
            return Rect::from_coords([0.0, 0.0], [0.0, 0.0]);
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for v in &polygon.vertices {
            min_x = min_x.min(v.x);
            min_y = min_y.min(v.y);
            max_x = max_x.max(v.x);
            max_y = max_y.max(v.y);
        }

        Rect::new(Point::new(min_x, min_y), Point::new(max_x, max_y))
    }

    fn point_in_polygon(&self, point: &Point, polygon: &Polygon) -> bool {
        let mut inside = false;
        let n = polygon.vertices.len();

        if n < 3 {
            return false;
        }

        let mut j = n - 1;
        for i in 0..n {
            let vi = &polygon.vertices[i];
            let vj = &polygon.vertices[j];

            // Check for horizontal edge to avoid division by zero
            let dy = vj.y - vi.y;
            if dy.abs() < f64::EPSILON {
                j = i;
                continue;
            }

            if ((vi.y > point.y) != (vj.y > point.y))
                && (point.x < (vj.x - vi.x) * (point.y - vi.y) / dy + vi.x)
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }
}

impl Default for IouEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// 聚合 `IoU` 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateIouResult {
    /// 平均 `IoU`
    pub mean_iou: f64,
    /// 总体 `IoU`
    pub overall_iou: f64,
    /// 各个房间的 `IoU`
    pub individual_ious: Vec<f64>,
    /// 总交集面积
    pub total_intersection: f64,
    /// 总并集面积
    pub total_union: f64,
    /// 匹配数量
    pub matched_count: usize,
    /// 预测房间总数
    pub total_predicted: usize,
    /// 真实房间总数
    pub total_ground_truth: usize,
}

/// 面积误差指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AreaErrorMetrics {
    /// 绝对误差
    pub absolute_error: f64,
    /// 相对误差（百分比）
    pub relative_error: f64,
    /// 是否可接受（误差 < 5%）
    pub acceptable: bool,
}

/// 计算面积误差
pub fn calculate_area_error(predicted: f64, ground_truth: f64) -> AreaErrorMetrics {
    let absolute_error = (predicted - ground_truth).abs();
    let relative_error = if ground_truth > 0.0 {
        absolute_error / ground_truth * 100.0
    } else {
        0.0
    };

    AreaErrorMetrics {
        absolute_error,
        relative_error,
        acceptable: relative_error < 5.0,
    }
}

/// 使用 tokitai 工具封装
#[derive(Default, Clone)]
pub struct IouTools;

use tokitai::tool;

#[tool]
impl IouTools {
    /// 计算 `IoU`
    #[tool]
    pub fn calculate_iou(
        &self,
        predicted_vertices: Vec<[f64; 2]>,
        ground_truth_vertices: Vec<[f64; 2]>,
    ) -> IouResult {
        let evaluator = IouEvaluator::new();
        let predicted = Polygon::from_coords(predicted_vertices);
        let ground_truth = Polygon::from_coords(ground_truth_vertices);
        evaluator.calculate_iou(&predicted, &ground_truth)
    }

    /// 计算面积误差
    #[tool]
    pub fn calculate_area_error(&self, predicted: f64, ground_truth: f64) -> AreaErrorMetrics {
        calculate_area_error(predicted, ground_truth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_square(size: f64) -> Polygon {
        Polygon::from_coords(vec![[0.0, 0.0], [size, 0.0], [size, size], [0.0, size]])
    }

    #[test]
    fn test_evaluator_new() {
        let evaluator = IouEvaluator::new();
        assert!(
            evaluator
                .approximate_intersection_union(&create_square(100.0), &create_square(100.0))
                .0
                > 0.0
        );
    }

    #[test]
    fn test_evaluator_default() {
        let evaluator = IouEvaluator;
        assert!(
            evaluator
                .approximate_intersection_union(&create_square(100.0), &create_square(100.0))
                .0
                > 0.0
        );
    }

    #[test]
    fn test_calculate_iou_identical() {
        let evaluator = IouEvaluator::new();
        let poly = create_square(100.0);

        let result = evaluator.calculate_iou(&poly, &poly);

        assert!(result.iou > 0.9);
        assert!((result.predicted_area - 10000.0).abs() < 1.0);
        assert!((result.ground_truth_area - 10000.0).abs() < 1.0);
    }

    #[test]
    fn test_calculate_iou_disjoint() {
        let evaluator = IouEvaluator::new();
        let poly1 = create_square(100.0);
        let poly2 = Polygon::from_coords(vec![
            [200.0, 200.0],
            [300.0, 200.0],
            [300.0, 300.0],
            [200.0, 300.0],
        ]);

        let result = evaluator.calculate_iou(&poly1, &poly2);

        assert!(result.iou < 0.1);
    }

    #[test]
    fn test_calculate_iou_partial_overlap() {
        let evaluator = IouEvaluator::new();
        let poly1 = create_square(100.0);
        let poly2 = Polygon::from_coords(vec![
            [50.0, 0.0],
            [150.0, 0.0],
            [150.0, 100.0],
            [50.0, 100.0],
        ]);

        let result = evaluator.calculate_iou(&poly1, &poly2);

        assert!(result.iou > 0.0);
        assert!(result.iou < 1.0);
        assert!(result.intersection > 0.0);
        assert!(result.union > 0.0);
    }

    #[test]
    fn test_calculate_iou_containment() {
        let evaluator = IouEvaluator::new();
        let large = create_square(200.0);
        let small = Polygon::from_coords(vec![
            [50.0, 50.0],
            [150.0, 50.0],
            [150.0, 150.0],
            [50.0, 150.0],
        ]);

        let result = evaluator.calculate_iou(&small, &large);

        assert!(result.iou > 0.0);
        assert!(result.iou < 1.0);
    }

    #[test]
    fn test_evaluate_rooms_multiple() {
        let evaluator = IouEvaluator::new();
        let pred_rooms = vec![create_square(100.0)];
        let gt_rooms = vec![create_square(100.0)];

        let result = evaluator.evaluate_rooms(&pred_rooms, &gt_rooms);

        assert!(result.mean_iou > 0.9);
        assert!(result.overall_iou > 0.9);
        assert_eq!(result.matched_count, 1);
        assert_eq!(result.total_predicted, 1);
        assert_eq!(result.total_ground_truth, 1);
    }

    #[test]
    fn test_evaluate_rooms_empty_pred() {
        let evaluator = IouEvaluator::new();
        let pred_rooms: Vec<Polygon> = vec![];
        let gt_rooms = vec![create_square(100.0)];

        let result = evaluator.evaluate_rooms(&pred_rooms, &gt_rooms);

        assert_eq!(result.mean_iou, 0.0);
        assert_eq!(result.overall_iou, 0.0);
        assert_eq!(result.matched_count, 0);
    }

    #[test]
    fn test_evaluate_rooms_empty_gt() {
        let evaluator = IouEvaluator::new();
        let pred_rooms = vec![create_square(100.0)];
        let gt_rooms: Vec<Polygon> = vec![];

        let result = evaluator.evaluate_rooms(&pred_rooms, &gt_rooms);

        assert_eq!(result.mean_iou, 0.0);
        assert_eq!(result.overall_iou, 0.0);
    }

    #[test]
    fn test_evaluate_rooms_multiple_matches() {
        let evaluator = IouEvaluator::new();
        let pred_rooms = vec![
            create_square(100.0),
            Polygon::from_coords(vec![
                [150.0, 0.0],
                [250.0, 0.0],
                [250.0, 100.0],
                [150.0, 100.0],
            ]),
        ];
        let gt_rooms = vec![
            create_square(100.0),
            Polygon::from_coords(vec![
                [150.0, 0.0],
                [250.0, 0.0],
                [250.0, 100.0],
                [150.0, 100.0],
            ]),
        ];

        let result = evaluator.evaluate_rooms(&pred_rooms, &gt_rooms);

        assert!(result.mean_iou > 0.5);
        assert_eq!(result.matched_count, 2);
    }

    #[test]
    fn test_get_bbox() {
        let evaluator = IouEvaluator::new();
        let poly = create_square(100.0);

        let bbox = evaluator.get_bbox(&poly);

        assert!((bbox.min.x - 0.0).abs() < 0.01);
        assert!((bbox.min.y - 0.0).abs() < 0.01);
        assert!((bbox.max.x - 100.0).abs() < 0.01);
        assert!((bbox.max.y - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_get_bbox_empty() {
        let evaluator = IouEvaluator::new();
        let poly = Polygon::from_coords(vec![]);

        let bbox = evaluator.get_bbox(&poly);

        assert_eq!(bbox.min.x, 0.0);
        assert_eq!(bbox.min.y, 0.0);
    }

    #[test]
    fn test_point_in_polygon_inside() {
        let evaluator = IouEvaluator::new();
        let poly = create_square(100.0);
        let point = Point::new(50.0, 50.0);

        assert!(evaluator.point_in_polygon(&point, &poly));
    }

    #[test]
    fn test_point_in_polygon_outside() {
        let evaluator = IouEvaluator::new();
        let poly = create_square(100.0);
        let point = Point::new(150.0, 150.0);

        assert!(!evaluator.point_in_polygon(&point, &poly));
    }

    #[test]
    fn test_point_in_polygon_on_edge() {
        let evaluator = IouEvaluator::new();
        let poly = create_square(100.0);
        let point = Point::new(50.0, 0.0);

        // 边界情况可能返回 true 或 false，取决于实现
        let _ = evaluator.point_in_polygon(&point, &poly);
    }

    #[test]
    fn test_point_in_polygon_degenerate() {
        let evaluator = IouEvaluator::new();
        let poly = Polygon::from_coords(vec![[0.0, 0.0], [1.0, 1.0]]);
        let point = Point::new(0.5, 0.5);

        assert!(!evaluator.point_in_polygon(&point, &poly));
    }

    #[test]
    fn test_approximate_intersection_union_identical() {
        let evaluator = IouEvaluator::new();
        let poly = create_square(100.0);

        let (intersection, union) = evaluator.approximate_intersection_union(&poly, &poly);

        assert!(intersection > 0.0);
        assert!(union > 0.0);
        assert!(intersection <= union);
    }

    #[test]
    fn test_iou_result_clone() {
        let result = IouResult {
            iou: 0.8,
            intersection: 8000.0,
            union: 10000.0,
            predicted_area: 9000.0,
            ground_truth_area: 9000.0,
        };

        let cloned = result.clone();
        assert_eq!(cloned.iou, result.iou);
        assert_eq!(cloned.intersection, result.intersection);
    }

    #[test]
    fn test_iou_result_debug() {
        let result = IouResult {
            iou: 0.75,
            intersection: 7500.0,
            union: 10000.0,
            predicted_area: 8000.0,
            ground_truth_area: 9000.0,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("IouResult"));
    }

    #[test]
    fn test_aggregate_result_clone() {
        let result = AggregateIouResult {
            mean_iou: 0.8,
            overall_iou: 0.75,
            individual_ious: vec![0.8, 0.7],
            total_intersection: 15000.0,
            total_union: 20000.0,
            matched_count: 2,
            total_predicted: 2,
            total_ground_truth: 2,
        };

        let cloned = result.clone();
        assert_eq!(cloned.mean_iou, result.mean_iou);
        assert_eq!(cloned.matched_count, result.matched_count);
    }

    #[test]
    fn test_aggregate_result_debug() {
        let result = AggregateIouResult {
            mean_iou: 0.85,
            overall_iou: 0.8,
            individual_ious: vec![0.85],
            total_intersection: 8500.0,
            total_union: 10000.0,
            matched_count: 1,
            total_predicted: 1,
            total_ground_truth: 1,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("AggregateIouResult"));
    }

    #[test]
    fn test_area_error_metrics_perfect() {
        let metrics = calculate_area_error(100.0, 100.0);

        assert!((metrics.absolute_error - 0.0).abs() < 0.01);
        assert!((metrics.relative_error - 0.0).abs() < 0.01);
        assert!(metrics.acceptable);
    }

    #[test]
    fn test_area_error_metrics_small_error() {
        let metrics = calculate_area_error(103.0, 100.0);

        assert!((metrics.absolute_error - 3.0).abs() < 0.01);
        assert!((metrics.relative_error - 3.0).abs() < 0.01);
        assert!(metrics.acceptable);
    }

    #[test]
    fn test_area_error_metrics_large_error() {
        let metrics = calculate_area_error(120.0, 100.0);

        assert!((metrics.absolute_error - 20.0).abs() < 0.01);
        assert!((metrics.relative_error - 20.0).abs() < 0.01);
        assert!(!metrics.acceptable);
    }

    #[test]
    fn test_area_error_metrics_zero_ground_truth() {
        let metrics = calculate_area_error(10.0, 0.0);

        assert!((metrics.absolute_error - 10.0).abs() < 0.01);
        assert!((metrics.relative_error - 0.0).abs() < 0.01);
        assert!(metrics.acceptable);
    }

    #[test]
    fn test_area_error_metrics_negative_error() {
        let metrics = calculate_area_error(95.0, 100.0);

        assert!((metrics.absolute_error - 5.0).abs() < 0.01);
        assert!((metrics.relative_error - 5.0).abs() < 0.01);
        assert!(!metrics.acceptable);
    }

    #[test]
    fn test_area_error_metrics_clone() {
        let metrics = calculate_area_error(100.0, 100.0);
        let cloned = metrics.clone();

        assert_eq!(cloned.absolute_error, metrics.absolute_error);
        assert_eq!(cloned.acceptable, metrics.acceptable);
    }

    #[test]
    fn test_area_error_metrics_debug() {
        let metrics = calculate_area_error(100.0, 100.0);
        let debug_str = format!("{:?}", metrics);

        assert!(debug_str.contains("AreaErrorMetrics"));
    }

    #[test]
    fn test_iou_tools_calculate_iou() {
        let tools = IouTools;
        let predicted = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
        let ground_truth = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];

        let result = tools.calculate_iou(predicted, ground_truth);

        assert!(result.iou > 0.9);
    }

    #[test]
    fn test_iou_tools_calculate_area_error() {
        let tools = IouTools;

        let metrics = tools.calculate_area_error(100.0, 100.0);

        assert!(metrics.acceptable);
    }
}
