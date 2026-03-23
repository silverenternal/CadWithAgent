//! IoU（交并比）指标
//!
//! 计算房间面积与真实面积的 IoU 误差

use crate::geometry::{Polygon, Point, Rect};
use serde::{Deserialize, Serialize};

/// IoU 计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IouResult {
    /// IoU 值（0-1）
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

/// IoU 评估器
pub struct IouEvaluator;

impl IouEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// 计算两个多边形的 IoU
    pub fn calculate_iou(&self, predicted: &Polygon, ground_truth: &Polygon) -> IouResult {
        let predicted_area = predicted.area();
        let ground_truth_area = ground_truth.area();

        // 简化实现：使用包围盒近似计算 IoU
        // 完整实现需要使用多边形裁剪算法
        let (intersection, union) = self.appimate_intersection_union(predicted, ground_truth);

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

    /// 计算多个房间的 IoU
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
    fn appimate_intersection_union(
        &self,
        poly1: &Polygon,
        poly2: &Polygon,
    ) -> (f64, f64) {
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

            if ((vi.y > point.y) != (vj.y > point.y))
                && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
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

/// 聚合 IoU 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateIouResult {
    /// 平均 IoU
    pub mean_iou: f64,
    /// 总体 IoU
    pub overall_iou: f64,
    /// 各个房间的 IoU
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
    /// 计算 IoU
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
