//! 准确率评估模块
//!
//! 计算几何推理任务的 F1 分数、准确率、召回率等指标
//!
//! # 评估指标
//!
//! - **精确率 (Precision)**: TP / (TP + FP)
//! - **召回率 (Recall)**: TP / (TP + FN)
//! - **F1 分数**: 2 * (Precision * Recall) / (Precision + Recall)
//! - **IoU**: 交并比，用于房间/区域检测
//!
//! # 快速开始
//!
//! ## 房间检测评估
//!
//! ```rust
//! use cadagent::metrics::evaluator::{MetricEvaluator, RoomDetection};
//!
//! let evaluator = MetricEvaluator::new();
//!
//! let ground_truth = vec![
//!     RoomDetection {
//!         id: 1,
//!         room_type: "living_room".to_string(),
//!         bbox: [0.0, 0.0, 100.0, 100.0],
//!         area: 10000.0,
//!     },
//! ];
//!
//! let predictions = vec![
//!     RoomDetection {
//!         id: 1,
//!         room_type: "living_room".to_string(),
//!         bbox: [5.0, 5.0, 105.0, 105.0],  // 轻微偏移
//!         area: 10000.0,
//!     },
//! ];
//!
//! let result = evaluator.evaluate_room_detection(&predictions, &ground_truth);
//! assert!(result.f1_score > 0.9);
//! assert!(result.iou.unwrap() > 0.8);
//! ```
//!
//! ## 尺寸提取评估
//!
//! ```rust
//! use cadagent::metrics::evaluator::{MetricEvaluator, DimensionExtraction};
//!
//! // 1% 误差容忍度
//! let evaluator = MetricEvaluator::with_thresholds(0.5, 0.01);
//!
//! let ground_truth = vec![
//!     DimensionExtraction {
//!         primitive_id: 0,
//!         dimension_type: "length".to_string(),
//!         value: 100.0,
//!         unit: "mm".to_string(),
//!     },
//! ];
//!
//! let predictions = vec![
//!     DimensionExtraction {
//!         primitive_id: 0,
//!         dimension_type: "length".to_string(),
//!         value: 100.5,  // 0.5% 误差
//!         unit: "mm".to_string(),
//!     },
//! ];
//!
//! let result = evaluator.evaluate_dimension_extraction(&predictions, &ground_truth);
//! assert_eq!(result.true_positives, 1);
//! assert_eq!(result.f1_score, 1.0);
//! ```
//!
//! ## 冲突检测评估
//!
//! ```rust
//! use cadagent::metrics::evaluator::{MetricEvaluator, ConflictDetection};
//!
//! let evaluator = MetricEvaluator::new();
//!
//! let ground_truth = vec![
//!     ConflictDetection {
//!         conflict_id: 1,
//!         primitive_ids: vec![0, 1],
//!         conflict_type: "parallel_perpendicular".to_string(),
//!     },
//! ];
//!
//! let predictions = vec![
//!     ConflictDetection {
//!         conflict_id: 1,
//!         primitive_ids: vec![0, 1],
//!         conflict_type: "parallel_perpendicular".to_string(),
//!     },
//! ];
//!
//! let result = evaluator.evaluate_conflict_detection(&predictions, &ground_truth, 1000);
//! assert_eq!(result.f1_score, 1.0);
//! ```
//!
//! ## 综合评估
//!
//! ```rust,no_run
//! use cadagent::metrics::evaluator::{MetricEvaluator, RoomDetection, DimensionExtraction, ConflictDetection};
//!
//! let evaluator = MetricEvaluator::new();
//!
//! // 准备数据
//! let room_gt = vec![/* ... */];
//! let room_pred = vec![/* ... */];
//! let dim_gt = vec![/* ... */];
//! let dim_pred = vec![/* ... */];
//! let conflict_gt = vec![/* ... */];
//! let conflict_pred = vec![/* ... */];
//!
//! let results = evaluator.run_comprehensive_evaluation(
//!     &room_pred, &room_gt,
//!     &dim_pred, &dim_gt,
//!     &conflict_pred, &conflict_gt,
//!     10000,  // 总检查次数
//! );
//!
//! println!("Room F1: {}", results.get("room_detection").unwrap().f1_score);
//! println!("Dimension F1: {}", results.get("dimension_extraction").unwrap().f1_score);
//! println!("Conflict F1: {}", results.get("conflict_detection").unwrap().f1_score);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 评估结果
///
/// 包含分类任务的各项评估指标
///
/// # 字段说明
///
/// - `true_positives`: 真正例数量（正确预测为正例）
/// - `false_positives`: 假正例数量（错误预测为正例）
/// - `false_negatives`: 假负例数量（错误预测为负例）
/// - `true_negatives`: 真负例数量（正确预测为负例）
/// - `precision`: 精确率，预测为正例的样本中实际为正例的比例
/// - `recall`: 召回率，实际为正例的样本中被正确预测的比例
/// - `f1_score`: F1 分数，精确率和召回率的调和平均
/// - `accuracy`: 准确率，预测正确的样本占总数的比例
/// - `iou`: 交并比（可选），用于目标检测任务
///
/// # 示例
///
/// ```rust
/// use cadagent::metrics::evaluator::EvaluationResult;
///
/// // 从混淆矩阵创建
/// let result = EvaluationResult::from_confusion_matrix(80, 20, 10, 90);
///
/// println!("Precision: {:.2}", result.precision);  // 0.80
/// println!("Recall: {:.2}", result.recall);        // 0.89
/// println!("F1 Score: {:.2}", result.f1_score);    // 0.84
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// 真正例数量
    pub true_positives: usize,
    /// 假正例数量
    pub false_positives: usize,
    /// 假负例数量
    pub false_negatives: usize,
    /// 真负例数量
    pub true_negatives: usize,
    /// 精确率
    pub precision: f64,
    /// 召回率
    pub recall: f64,
    /// F1 分数
    pub f1_score: f64,
    /// 准确率
    pub accuracy: f64,
    /// IoU (交并比)
    pub iou: Option<f64>,
}

impl EvaluationResult {
    /// 创建新的评估结果
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tp: usize,
        fp: usize,
        fn_: usize,
        tn: usize,
        precision: f64,
        recall: f64,
        f1: f64,
        accuracy: f64,
        iou: Option<f64>,
    ) -> Self {
        Self {
            true_positives: tp,
            false_positives: fp,
            false_negatives: fn_,
            true_negatives: tn,
            precision,
            recall,
            f1_score: f1,
            accuracy,
            iou,
        }
    }

    /// 从混淆矩阵计算评估指标
    pub fn from_confusion_matrix(tp: usize, fp: usize, fn_: usize, tn: usize) -> Self {
        let precision = if tp + fp > 0 {
            tp as f64 / (tp + fp) as f64
        } else {
            0.0
        };

        let recall = if tp + fn_ > 0 {
            tp as f64 / (tp + fn_) as f64
        } else {
            0.0
        };

        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        let total = tp + fp + fn_ + tn;
        let accuracy = if total > 0 {
            (tp + tn) as f64 / total as f64
        } else {
            0.0
        };

        Self::new(tp, fp, fn_, tn, precision, recall, f1, accuracy, None)
    }

    /// 计算 IoU
    pub fn with_iou(mut self, iou: f64) -> Self {
        self.iou = Some(iou);
        self
    }
}

/// 房间检测结果
#[derive(Debug, Clone, PartialEq)]
pub struct RoomDetection {
    /// 房间 ID
    pub id: usize,
    /// 房间类型
    pub room_type: String,
    /// 边界框 [x1, y1, x2, y2]
    pub bbox: [f64; 4],
    /// 面积
    pub area: f64,
}

/// 尺寸提取结果
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionExtraction {
    /// 基元 ID
    pub primitive_id: usize,
    /// 尺寸类型
    pub dimension_type: String,
    /// 尺寸值
    pub value: f64,
    /// 单位
    pub unit: String,
}

/// 冲突检测结果
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConflictDetection {
    /// 冲突 ID
    pub conflict_id: usize,
    /// 涉及的基元 ID
    pub primitive_ids: Vec<usize>,
    /// 冲突类型
    pub conflict_type: String,
}

/// 准确率评估器
///
/// 用于评估几何推理任务的性能，支持房间检测、尺寸提取、冲突检测等多种任务
///
/// # 配置参数
///
/// - `iou_threshold`: IoU 阈值，用于房间检测匹配（默认 0.5）
/// - `distance_threshold`: 距离/误差容忍阈值，用于尺寸提取（默认 1e-6）
///
/// # 示例
///
/// ## 默认配置
///
/// ```rust
/// use cadagent::metrics::evaluator::MetricEvaluator;
///
/// let evaluator = MetricEvaluator::new();
/// // 使用默认阈值：IoU=0.5, 距离=1e-6
/// ```
///
/// ## 自定义阈值
///
/// ```rust
/// use cadagent::metrics::evaluator::MetricEvaluator;
///
/// // IoU 阈值 0.7（更严格），尺寸误差容忍 5%
/// let evaluator = MetricEvaluator::with_thresholds(0.7, 0.05);
/// ```
///
/// ## 房间检测评估
///
/// ```rust
/// use cadagent::metrics::evaluator::{MetricEvaluator, RoomDetection};
///
/// let evaluator = MetricEvaluator::new();
/// let ground_truth = vec![
///     RoomDetection {
///         id: 1,
///         room_type: "living_room".to_string(),
///         bbox: [0.0, 0.0, 100.0, 100.0],
///         area: 10000.0,
///     },
/// ];
/// let predictions = vec![
///     RoomDetection {
///         id: 1,
///         room_type: "living_room".to_string(),
///         bbox: [10.0, 10.0, 110.0, 110.0],
///         area: 10000.0,
///     },
/// ];
///
/// let result = evaluator.evaluate_room_detection(&predictions, &ground_truth);
/// println!("F1: {:.2}, IoU: {:.2}", result.f1_score, result.iou.unwrap_or(0.0));
/// ```
pub struct MetricEvaluator {
    /// IoU 阈值（用于房间检测）
    iou_threshold: f64,
    /// 距离阈值（用于尺寸提取）
    distance_threshold: f64,
}

impl MetricEvaluator {
    /// 创建新的评估器
    pub fn new() -> Self {
        Self {
            iou_threshold: 0.5,
            distance_threshold: 1e-6,
        }
    }

    /// 创建带自定义阈值的评估器
    pub fn with_thresholds(iou_threshold: f64, distance_threshold: f64) -> Self {
        Self {
            iou_threshold,
            distance_threshold,
        }
    }

    /// 评估房间检测
    ///
    /// # Arguments
    /// * `predictions` - 预测的房间列表
    /// * `ground_truth` - 真实标注的房间列表
    ///
    /// # Returns
    /// 评估结果，包含 F1 分数、IoU 等指标
    pub fn evaluate_room_detection(
        &self,
        predictions: &[RoomDetection],
        ground_truth: &[RoomDetection],
    ) -> EvaluationResult {
        let mut tp = 0;
        let mut fp = 0;
        let mut fn_ = 0;

        let mut matched_gt = vec![false; ground_truth.len()];

        for pred in predictions {
            let mut best_iou = 0.0;
            let mut best_match = None;

            for (gt_idx, gt) in ground_truth.iter().enumerate() {
                if matched_gt[gt_idx] {
                    continue;
                }

                // 类型必须匹配
                if pred.room_type != gt.room_type {
                    continue;
                }

                // 计算 IoU
                let iou = self.compute_bbox_iou(&pred.bbox, &gt.bbox);
                if iou > best_iou && iou >= self.iou_threshold {
                    best_iou = iou;
                    best_match = Some(gt_idx);
                }
            }

            if let Some(match_idx) = best_match {
                tp += 1;
                matched_gt[match_idx] = true;
            } else {
                fp += 1;
            }
        }

        // 未匹配的 ground truth 为 FN
        for &matched in &matched_gt {
            if !matched {
                fn_ += 1;
            }
        }

        let tn = 0; // 房间检测通常不计算 TN
        let mut result = EvaluationResult::from_confusion_matrix(tp, fp, fn_, tn);

        // 计算平均 IoU
        if tp > 0 {
            result.iou = Some(self.compute_average_iou(predictions, ground_truth));
        }

        result
    }

    /// 评估尺寸提取
    ///
    /// # Arguments
    /// * `predictions` - 预测的尺寸列表
    /// * `ground_truth` - 真实标注的尺寸列表
    ///
    /// # Returns
    /// 评估结果，包含 F1 分数等指标
    pub fn evaluate_dimension_extraction(
        &self,
        predictions: &[DimensionExtraction],
        ground_truth: &[DimensionExtraction],
    ) -> EvaluationResult {
        let mut tp = 0;
        let mut fp = 0;
        let mut fn_ = 0;

        let mut matched_gt = vec![false; ground_truth.len()];

        for pred in predictions {
            let mut found_match = false;

            for (gt_idx, gt) in ground_truth.iter().enumerate() {
                if matched_gt[gt_idx] {
                    continue;
                }

                // 基元 ID 和类型必须匹配
                if pred.primitive_id != gt.primitive_id || pred.dimension_type != gt.dimension_type
                {
                    continue;
                }

                // 检查数值是否在阈值范围内
                let diff = (pred.value - gt.value).abs();
                let relative_error = diff / gt.value.max(1e-10);

                if relative_error <= self.distance_threshold {
                    tp += 1;
                    matched_gt[gt_idx] = true;
                    found_match = true;
                    break;
                }
            }

            if !found_match {
                fp += 1;
            }
        }

        // 未匹配的 ground truth 为 FN
        for &matched in &matched_gt {
            if !matched {
                fn_ += 1;
            }
        }

        let tn = 0;
        EvaluationResult::from_confusion_matrix(tp, fp, fn_, tn)
    }

    /// 评估冲突检测
    ///
    /// # Arguments
    /// * `predictions` - 预测的冲突列表
    /// * `ground_truth` - 真实标注的冲突列表
    /// * `total_checks` - 总检查次数（用于计算 TN）
    ///
    /// # Returns
    /// 评估结果，包含 F1 分数等指标
    pub fn evaluate_conflict_detection(
        &self,
        predictions: &[ConflictDetection],
        ground_truth: &[ConflictDetection],
        total_checks: usize,
    ) -> EvaluationResult {
        let mut tp = 0;
        let mut fp = 0;

        let mut matched_gt = vec![false; ground_truth.len()];

        for pred in predictions {
            let mut found_match = false;

            for (gt_idx, gt) in ground_truth.iter().enumerate() {
                if matched_gt[gt_idx] {
                    continue;
                }

                // 冲突类型和涉及的基元必须匹配
                if pred.conflict_type != gt.conflict_type {
                    continue;
                }

                let mut pred_ids = pred.primitive_ids.clone();
                let mut gt_ids = gt.primitive_ids.clone();
                pred_ids.sort();
                gt_ids.sort();

                if pred_ids == gt_ids {
                    tp += 1;
                    matched_gt[gt_idx] = true;
                    found_match = true;
                    break;
                }
            }

            if !found_match {
                fp += 1;
            }
        }

        // 未匹配的 ground truth 为 FN
        let fn_ = ground_truth.len() - matched_gt.iter().filter(|&&m| m).count();

        // TN = 总检查数 - TP - FP - FN
        let tn = total_checks.saturating_sub(tp + fp + fn_);

        EvaluationResult::from_confusion_matrix(tp, fp, fn_, tn)
    }

    /// 计算两个边界框的 IoU
    fn compute_bbox_iou(&self, bbox1: &[f64; 4], bbox2: &[f64; 4]) -> f64 {
        // 计算交集
        let x1 = bbox1[0].max(bbox2[0]);
        let y1 = bbox1[1].max(bbox2[1]);
        let x2 = bbox1[2].min(bbox2[2]);
        let y2 = bbox1[3].min(bbox2[3]);

        let intersection_area = if x1 < x2 && y1 < y2 {
            (x2 - x1) * (y2 - y1)
        } else {
            0.0
        };

        // 计算并集
        let area1 = (bbox1[2] - bbox1[0]) * (bbox1[3] - bbox1[1]);
        let area2 = (bbox2[2] - bbox2[0]) * (bbox2[3] - bbox2[1]);
        let union_area = area1 + area2 - intersection_area;

        if union_area > 0.0 {
            intersection_area / union_area
        } else {
            0.0
        }
    }

    /// 计算平均 IoU
    fn compute_average_iou(
        &self,
        predictions: &[RoomDetection],
        ground_truth: &[RoomDetection],
    ) -> f64 {
        let mut total_iou = 0.0;
        let mut count = 0;

        for pred in predictions {
            for gt in ground_truth {
                if pred.room_type == gt.room_type {
                    let iou = self.compute_bbox_iou(&pred.bbox, &gt.bbox);
                    if iou > 0.0 {
                        total_iou += iou;
                        count += 1;
                    }
                }
            }
        }

        if count > 0 {
            total_iou / count as f64
        } else {
            0.0
        }
    }

    /// 运行综合评估
    ///
    /// # Arguments
    /// * `room_predictions` - 房间检测预测
    /// * `room_ground_truth` - 房间检测真实标注
    /// * `dim_predictions` - 尺寸提取预测
    /// * `dim_ground_truth` - 尺寸提取真实标注
    /// * `conflict_predictions` - 冲突检测预测
    /// * `conflict_ground_truth` - 冲突检测真实标注
    /// * `total_conflict_checks` - 冲突检测总检查次数
    ///
    /// # Returns
    /// 包含各项指标的 HashMap
    #[allow(clippy::too_many_arguments)]
    pub fn run_comprehensive_evaluation(
        &self,
        room_predictions: &[RoomDetection],
        room_ground_truth: &[RoomDetection],
        dim_predictions: &[DimensionExtraction],
        dim_ground_truth: &[DimensionExtraction],
        conflict_predictions: &[ConflictDetection],
        conflict_ground_truth: &[ConflictDetection],
        total_conflict_checks: usize,
    ) -> HashMap<String, EvaluationResult> {
        let mut results = HashMap::new();

        results.insert(
            "room_detection".to_string(),
            self.evaluate_room_detection(room_predictions, room_ground_truth),
        );

        results.insert(
            "dimension_extraction".to_string(),
            self.evaluate_dimension_extraction(dim_predictions, dim_ground_truth),
        );

        results.insert(
            "conflict_detection".to_string(),
            self.evaluate_conflict_detection(
                conflict_predictions,
                conflict_ground_truth,
                total_conflict_checks,
            ),
        );

        results
    }
}

impl Default for MetricEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluation_result_from_confusion_matrix() {
        let result = EvaluationResult::from_confusion_matrix(80, 20, 10, 90);

        assert_eq!(result.true_positives, 80);
        assert_eq!(result.false_positives, 20);
        assert_eq!(result.false_negatives, 10);
        assert_eq!(result.true_negatives, 90);

        // Precision = 80 / (80 + 20) = 0.8
        assert!((result.precision - 0.8).abs() < 1e-6);

        // Recall = 80 / (80 + 10) = 0.888...
        assert!((result.recall - 0.888_888_888_888_888_8).abs() < 1e-6);

        // F1 = 2 * 0.8 * 0.888... / (0.8 + 0.888...) = 0.842...
        assert!((result.f1_score - 0.842_105_263_157_894_7).abs() < 1e-6);
    }

    #[test]
    fn test_room_detection_evaluation() {
        let evaluator = MetricEvaluator::new();

        let ground_truth = vec![
            RoomDetection {
                id: 1,
                room_type: "living_room".to_string(),
                bbox: [0.0, 0.0, 100.0, 100.0],
                area: 10000.0,
            },
            RoomDetection {
                id: 2,
                room_type: "bedroom".to_string(),
                bbox: [100.0, 0.0, 200.0, 100.0],
                area: 10000.0,
            },
        ];

        // 完美预测
        let predictions = ground_truth.clone();

        let result = evaluator.evaluate_room_detection(&predictions, &ground_truth);

        assert_eq!(result.true_positives, 2);
        assert_eq!(result.false_positives, 0);
        assert_eq!(result.false_negatives, 0);
        assert_eq!(result.f1_score, 1.0);
    }

    #[test]
    fn test_dimension_extraction_evaluation() {
        let evaluator = MetricEvaluator::with_thresholds(0.5, 0.01); // 1% 误差容忍

        let ground_truth = vec![
            DimensionExtraction {
                primitive_id: 0,
                dimension_type: "length".to_string(),
                value: 100.0,
                unit: "mm".to_string(),
            },
            DimensionExtraction {
                primitive_id: 1,
                dimension_type: "width".to_string(),
                value: 50.0,
                unit: "mm".to_string(),
            },
        ];

        // 预测值有轻微误差
        let predictions = vec![
            DimensionExtraction {
                primitive_id: 0,
                dimension_type: "length".to_string(),
                value: 100.5, // 0.5% 误差
                unit: "mm".to_string(),
            },
            DimensionExtraction {
                primitive_id: 1,
                dimension_type: "width".to_string(),
                value: 50.0,
                unit: "mm".to_string(),
            },
        ];

        let result = evaluator.evaluate_dimension_extraction(&predictions, &ground_truth);

        assert_eq!(result.true_positives, 2);
        assert!(result.f1_score > 0.9);
    }

    #[test]
    fn test_conflict_detection_evaluation() {
        let evaluator = MetricEvaluator::new();

        let ground_truth = vec![ConflictDetection {
            conflict_id: 1,
            primitive_ids: vec![0, 1],
            conflict_type: "parallel_perpendicular".to_string(),
        }];

        // 正确检测
        let predictions = vec![ConflictDetection {
            conflict_id: 1,
            primitive_ids: vec![0, 1],
            conflict_type: "parallel_perpendicular".to_string(),
        }];

        let result = evaluator.evaluate_conflict_detection(&predictions, &ground_truth, 100);

        assert_eq!(result.true_positives, 1);
        assert_eq!(result.false_positives, 0);
        assert_eq!(result.f1_score, 1.0);
    }

    #[test]
    fn test_bbox_iou_computation() {
        let evaluator = MetricEvaluator::new();

        // 完全重叠
        let bbox1 = [0.0, 0.0, 100.0, 100.0];
        let bbox2 = [0.0, 0.0, 100.0, 100.0];
        assert!((evaluator.compute_bbox_iou(&bbox1, &bbox2) - 1.0).abs() < 1e-6);

        // 无重叠
        let bbox3 = [0.0, 0.0, 50.0, 50.0];
        let bbox4 = [100.0, 100.0, 150.0, 150.0];
        assert!((evaluator.compute_bbox_iou(&bbox3, &bbox4)).abs() < 1e-6);

        // 部分重叠
        let bbox5 = [0.0, 0.0, 100.0, 100.0];
        let bbox6 = [50.0, 50.0, 150.0, 150.0];
        let iou = evaluator.compute_bbox_iou(&bbox5, &bbox6);
        // 交集：50*50 = 2500
        // 并集：10000 + 10000 - 2500 = 17500
        // IoU = 2500/17500 = 0.1428...
        assert!((iou - 0.142_857_142_857_142_85).abs() < 1e-6);
    }

    #[test]
    fn test_comprehensive_evaluation() {
        let evaluator = MetricEvaluator::new();

        let room_gt = vec![RoomDetection {
            id: 1,
            room_type: "living_room".to_string(),
            bbox: [0.0, 0.0, 100.0, 100.0],
            area: 10000.0,
        }];

        let dim_gt = vec![DimensionExtraction {
            primitive_id: 0,
            dimension_type: "length".to_string(),
            value: 100.0,
            unit: "mm".to_string(),
        }];

        let conflict_gt = vec![ConflictDetection {
            conflict_id: 1,
            primitive_ids: vec![0, 1],
            conflict_type: "parallel_perpendicular".to_string(),
        }];

        let results = evaluator.run_comprehensive_evaluation(
            &room_gt,
            &room_gt,
            &dim_gt,
            &dim_gt,
            &conflict_gt,
            &conflict_gt,
            100,
        );

        assert!(results.contains_key("room_detection"));
        assert!(results.contains_key("dimension_extraction"));
        assert!(results.contains_key("conflict_detection"));

        // 完美预测应该得到 F1 = 1.0
        assert!(results.get("room_detection").unwrap().f1_score > 0.9);
        assert!(results.get("dimension_extraction").unwrap().f1_score > 0.9);
        assert!(results.get("conflict_detection").unwrap().f1_score > 0.9);
    }
}
