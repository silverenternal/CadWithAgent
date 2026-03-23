//! CAD 基元提取工具
//!
//! 从 CAD 图纸（SVG/DXF/图像）中提取结构化几何基元
//!
//! # 功能特性
//!
//! - 支持 SVG/DXF 矢量格式解析
//! - 支持图像格式（待实现 OCR 和边缘检测）
//! - 提取线、圆、弧、点、多边形、文本等基元
//! - 坐标归一化处理
//! - 图层过滤与分组
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::cad_extractor::{CadPrimitiveExtractor, ExtractorConfig};
//!
//! let config = ExtractorConfig::default();
//! let extractor = CadPrimitiveExtractor::new(config);
//! let result = extractor.extract_from_svg("floor_plan.svg").unwrap();
//!
//! println!("提取了 {} 个基元", result.primitives.len());
//! ```

use crate::geometry::primitives::{Point, Line, Polygon, Circle, Rect, Primitive};
use crate::parser::svg::{SvgParser, SvgResult};
use crate::error::{CadAgentError, CadAgentResult, GeometryConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokitai::tool;

/// 基元提取结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveExtractionResult {
    /// 提取的基元列表
    pub primitives: Vec<Primitive>,
    /// 基元统计信息
    pub statistics: PrimitiveStatistics,
    /// 坐标准确性信息
    pub coordinate_info: CoordinateInfo,
}

/// 基元统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveStatistics {
    pub point_count: usize,
    pub line_count: usize,
    pub circle_count: usize,
    pub arc_count: usize,
    pub polygon_count: usize,
    pub rect_count: usize,
    pub polyline_count: usize,
    pub text_count: usize,
    pub total_count: usize,
}

/// 坐标信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateInfo {
    /// 原始坐标范围
    pub original_range: Option<CoordRange>,
    /// 归一化后坐标范围
    pub normalized_range: CoordRange,
    /// 归一化变换参数
    pub transform: TransformParams,
}

/// 坐标范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordRange {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// 变换参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformParams {
    pub scale_x: f64,
    pub scale_y: f64,
    pub translate_x: f64,
    pub translate_y: f64,
}

/// 提取器配置
///
/// 使用组合模式，组合 `GeometryConfig` 和提取器特有配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    /// 几何配置（组合共享配置）
    #[serde(flatten)]
    pub geometry: GeometryConfig,
    /// 最小线段长度（过滤短线）
    pub min_line_length: f64,
    /// 最小圆半径
    pub min_circle_radius: f64,
    /// 是否过滤文本
    pub filter_text: bool,
    /// 图层过滤列表（None 表示不过滤）
    pub layer_filter: Option<Vec<String>>,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            geometry: GeometryConfig::default(),
            min_line_length: 0.1,
            min_circle_radius: 0.1,
            filter_text: false,
            layer_filter: None,
        }
    }
}

impl ExtractorConfig {
    /// 验证配置参数的合理性
    ///
    /// # Errors
    /// 如果配置参数无效，返回 `CadAgentError::Config`
    pub fn validate(&self) -> CadAgentResult<()> {
        // 验证几何配置
        self.geometry.validate()?;

        // 验证最小线段长度
        if self.min_line_length < 0.0 {
            return Err(CadAgentError::Config(format!(
                "最小线段长度必须为非负数，当前值：{}",
                self.min_line_length
            )));
        }

        // 验证最小圆半径
        if self.min_circle_radius < 0.0 {
            return Err(CadAgentError::Config(format!(
                "最小圆半径必须为非负数，当前值：{}",
                self.min_circle_radius
            )));
        }

        Ok(())
    }

    /// 验证并自动修正不合理的配置
    pub fn validate_or_fix(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();

        // 验证并修正几何配置
        warnings.extend(self.geometry.validate_or_fix());

        if self.min_line_length < 0.0 {
            warnings.push(format!(
                "最小线段长度 {} 无效，已修正为默认值 0.1",
                self.min_line_length
            ));
            self.min_line_length = 0.1;
        }

        if self.min_circle_radius < 0.0 {
            warnings.push(format!(
                "最小圆半径 {} 无效，已修正为默认值 0.1",
                self.min_circle_radius
            ));
            self.min_circle_radius = 0.1;
        }

        warnings
    }

    /// 便捷访问几何配置中的归一化范围
    pub fn normalize_range(&self) -> [f64; 2] {
        self.geometry.normalize_range
    }

    /// 便捷访问几何配置中的归一化启用状态
    pub fn enable_normalization(&self) -> bool {
        self.geometry.enable_normalization
    }
}

/// CAD 基元提取器
#[derive(Debug, Clone)]
pub struct CadPrimitiveExtractor {
    config: ExtractorConfig,
}

impl CadPrimitiveExtractor {
    /// 创建新的提取器
    pub fn new(config: ExtractorConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建提取器
    pub fn with_defaults() -> Self {
        Self::new(ExtractorConfig::default())
    }

    /// 从 SVG 文件提取基元
    ///
    /// # Errors
    /// 如果 SVG 解析失败，返回 `CadAgentError::Parse`
    pub fn extract_from_svg(&self, path: impl AsRef<Path>) -> CadAgentResult<PrimitiveExtractionResult> {
        let svg_result = SvgParser::parse(path)
            .map_err(|e| CadAgentError::Parse(format!("SVG 解析失败：{}", e)))?;

        Ok(self.process_svg_result(svg_result))
    }

    /// 从 SVG 字符串提取基元
    ///
    /// # Errors
    /// 如果 SVG 解析失败，返回 `CadAgentError::Parse`
    pub fn extract_from_svg_string(&self, content: &str) -> CadAgentResult<PrimitiveExtractionResult> {
        let svg_result = SvgParser::parse_string(content)
            .map_err(|e| CadAgentError::Parse(format!("SVG 解析失败：{}", e)))?;

        Ok(self.process_svg_result(svg_result))
    }

    /// 处理 SVG 解析结果
    fn process_svg_result(&self, svg_result: SvgResult) -> PrimitiveExtractionResult {
        let mut primitives = svg_result.primitives;

        // 计算原始坐标范围
        let original_range = self.compute_coordinate_range(&primitives);

        // 坐标归一化
        if self.config.geometry.enable_normalization {
            let transform = self.compute_transform(&original_range);
            primitives = self.apply_normalization(primitives, &transform);
        } else {
            primitives = self.filter_primitives(primitives);
        }

        // 计算统计信息
        let statistics = self.compute_statistics(&primitives);

        // 坐标信息
        let normalized_range = CoordRange {
            min_x: self.config.geometry.normalize_range[0],
            min_y: self.config.geometry.normalize_range[0],
            max_x: self.config.geometry.normalize_range[1],
            max_y: self.config.geometry.normalize_range[1],
        };

        let transform = self.compute_transform(&original_range);
        let coordinate_info = CoordinateInfo {
            original_range,
            normalized_range,
            transform,
        };

        PrimitiveExtractionResult {
            primitives,
            statistics,
            coordinate_info,
        }
    }

    /// 计算坐标范围
    fn compute_coordinate_range(&self, primitives: &[Primitive]) -> Option<CoordRange> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for prim in primitives {
            if let Some(bbox) = prim.bounding_box() {
                min_x = min_x.min(bbox.min.x);
                min_y = min_y.min(bbox.min.y);
                max_x = max_x.max(bbox.max.x);
                max_y = max_y.max(bbox.max.y);
            }
        }

        if min_x.is_finite() && min_y.is_finite() {
            Some(CoordRange { min_x, min_y, max_x, max_y })
        } else {
            None
        }
    }

    /// 计算变换参数
    fn compute_transform(&self, range: &Option<CoordRange>) -> TransformParams {
        let default = TransformParams {
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        };

        match range {
            None => default,
            Some(range) => {
                let src_width = range.max_x - range.min_x;
                let src_height = range.max_y - range.min_y;
                let dst_width = self.config.geometry.normalize_range[1] - self.config.geometry.normalize_range[0];
                let dst_height = dst_width;

                let scale_x = if src_width > 0.0 { dst_width / src_width } else { 1.0 };
                let scale_y = if src_height > 0.0 { dst_height / src_height } else { 1.0 };

                TransformParams {
                    scale_x,
                    scale_y,
                    translate_x: -range.min_x * scale_x + self.config.geometry.normalize_range[0],
                    translate_y: -range.min_y * scale_y + self.config.geometry.normalize_range[0],
                }
            }
        }
    }

    /// 应用坐标归一化
    fn apply_normalization(&self, primitives: Vec<Primitive>, transform: &TransformParams) -> Vec<Primitive> {
        primitives.into_iter()
            .map(|p| self.transform_primitive(p, transform))
            .filter(|p| !self.should_filter(p))
            .collect()
    }

    /// 变换单个基元
    fn transform_primitive(&self, prim: Primitive, transform: &TransformParams) -> Primitive {
        match prim {
            Primitive::Point(p) => Primitive::Point(self.transform_point(p, transform)),
            Primitive::Line(line) => Primitive::Line(Line {
                start: self.transform_point(line.start, transform),
                end: self.transform_point(line.end, transform),
            }),
            Primitive::Polygon(poly) => Primitive::Polygon(Polygon {
                vertices: poly.vertices.into_iter()
                    .map(|p| self.transform_point(p, transform))
                    .collect(),
                closed: poly.closed,
            }),
            Primitive::Circle(circle) => {
                let center = self.transform_point(circle.center, transform);
                let radius = circle.radius * transform.scale_x.min(transform.scale_y);
                Primitive::Circle(Circle { center, radius })
            }
            Primitive::Rect(rect) => {
                let min = self.transform_point(rect.min, transform);
                let max = self.transform_point(rect.max, transform);
                Primitive::Rect(Rect { min, max })
            }
            Primitive::Polyline { points, closed } => Primitive::Polyline {
                points: points.into_iter()
                    .map(|p| self.transform_point(p, transform))
                    .collect(),
                closed,
            },
            Primitive::Arc { center, radius, start_angle, end_angle } => {
                let center = self.transform_point(center, transform);
                let new_radius = radius * transform.scale_x.min(transform.scale_y);
                Primitive::Arc {
                    center,
                    radius: new_radius,
                    start_angle,
                    end_angle,
                }
            }
            Primitive::Text { content, position, height } => Primitive::Text {
                position: self.transform_point(position, transform),
                height: height * transform.scale_y,
                content,
            },
        }
    }

    /// 变换点
    fn transform_point(&self, p: Point, transform: &TransformParams) -> Point {
        Point {
            x: p.x * transform.scale_x + transform.translate_x,
            y: p.y * transform.scale_y + transform.translate_y,
        }
    }

    /// 过滤基元
    fn filter_primitives(&self, primitives: Vec<Primitive>) -> Vec<Primitive> {
        primitives.into_iter()
            .filter(|p| !self.should_filter(p))
            .collect()
    }

    /// 判断是否应该过滤
    fn should_filter(&self, prim: &Primitive) -> bool {
        match prim {
            Primitive::Line(line) => line.length() < self.config.min_line_length,
            Primitive::Circle(circle) => circle.radius < self.config.min_circle_radius,
            Primitive::Text { .. } => self.config.filter_text,
            _ => false,
        }
    }

    /// 计算统计信息
    fn compute_statistics(&self, primitives: &[Primitive]) -> PrimitiveStatistics {
        let mut stats = PrimitiveStatistics {
            point_count: 0,
            line_count: 0,
            circle_count: 0,
            arc_count: 0,
            polygon_count: 0,
            rect_count: 0,
            polyline_count: 0,
            text_count: 0,
            total_count: primitives.len(),
        };

        for prim in primitives {
            match prim {
                Primitive::Point(_) => stats.point_count += 1,
                Primitive::Line(_) => stats.line_count += 1,
                Primitive::Circle(_) => stats.circle_count += 1,
                Primitive::Arc { .. } => stats.arc_count += 1,
                Primitive::Polygon(_) => stats.polygon_count += 1,
                Primitive::Rect(_) => stats.rect_count += 1,
                Primitive::Polyline { .. } => stats.polyline_count += 1,
                Primitive::Text { .. } => stats.text_count += 1,
            }
        }

        stats
    }
}

/// CAD 基元提取工具（tokitai 工具封装）
#[derive(Default, Clone)]
pub struct CadExtractorTools;

#[tool]
impl CadExtractorTools {
    /// 从 SVG 图纸提取几何基元
    ///
    /// # 参数
    ///
    /// * `svg_content` - SVG 文件内容或路径
    /// * `normalize` - 是否归一化坐标（默认 true）
    /// * `normalize_range` - 归一化范围 [min, max]（默认 [0, 100]）
    ///
    /// # 返回
    ///
    /// 包含基元列表、统计信息和坐标信息的结构化结果
    #[tool(name = "cad_extract_primitives")]
    pub fn extract_primitives(
        &self,
        svg_content: String,
        normalize: Option<bool>,
        normalize_range: Option<Vec<f64>>,
    ) -> serde_json::Value {
        let mut config = ExtractorConfig::default();

        if let Some(normalize) = normalize {
            config.geometry.enable_normalization = normalize;
        }

        if let Some(range) = normalize_range {
            if range.len() == 2 {
                config.geometry.normalize_range = [range[0], range[1]];
            }
        }

        let extractor = CadPrimitiveExtractor::new(config);

        // 尝试作为文件路径解析
        if std::path::Path::new(&svg_content).exists() {
            match extractor.extract_from_svg(&svg_content) {
                Ok(result) => self.result_to_json(result),
                Err(e) => serde_json::json!({
                    "success": false,
                    "error": e
                }),
            }
        } else {
            // 作为 SVG 字符串解析
            match extractor.extract_from_svg_string(&svg_content) {
                Ok(result) => self.result_to_json(result),
                Err(e) => serde_json::json!({
                    "success": false,
                    "error": e
                }),
            }
        }
    }

    /// 获取基元统计信息
    #[tool(name = "cad_get_primitive_stats")]
    pub fn get_statistics(&self, primitives_json: String) -> serde_json::Value {
        let primitives: Vec<Primitive> = match serde_json::from_str(&primitives_json) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析失败：{}", e)
                });
            }
        };

        let extractor = CadPrimitiveExtractor::with_defaults();
        let stats = extractor.compute_statistics(&primitives);

        serde_json::json!({
            "success": true,
            "statistics": stats
        })
    }

    /// 坐标归一化
    #[tool(name = "cad_normalize_coordinates")]
    pub fn normalize_coordinates(
        &self,
        primitives_json: String,
        target_range: Vec<f64>,
    ) -> serde_json::Value {
        let primitives: Vec<Primitive> = match serde_json::from_str(&primitives_json) {
            Ok(p) => p,
            Err(e) => {
                return serde_json::json!({
                    "success": false,
                    "error": format!("解析失败：{}", e)
                });
            }
        };

        let mut config = ExtractorConfig::default();
        config.geometry.normalize_range = [target_range[0], target_range[1]];

        let extractor = CadPrimitiveExtractor::new(config);
        let range = extractor.compute_coordinate_range(&primitives);
        let transform = extractor.compute_transform(&range);

        let normalized = extractor.apply_normalization(primitives, &transform);

        serde_json::json!({
            "success": true,
            "primitives": normalized,
            "transform": transform
        })
    }
}

impl CadExtractorTools {
    fn result_to_json(&self, result: PrimitiveExtractionResult) -> serde_json::Value {
        serde_json::json!({
            "success": true,
            "primitives": result.primitives,
            "statistics": result.statistics,
            "coordinate_info": result.coordinate_info
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::primitives::{Point, Line, Circle};

    #[test]
    fn test_extractor_creation() {
        let extractor = CadPrimitiveExtractor::with_defaults();
        assert!(extractor.config.geometry.enable_normalization);
        assert_eq!(extractor.config.geometry.normalize_range, [0.0, 100.0]);
    }

    #[test]
    fn test_compute_statistics() {
        let primitives = vec![
            Primitive::Point(Point::origin()),
            Primitive::Line(Line::from_coords([0.0, 0.0], [1.0, 1.0])),
            Primitive::Circle(Circle::from_coords([0.0, 0.0], 1.0)),
        ];

        let extractor = CadPrimitiveExtractor::with_defaults();
        let stats = extractor.compute_statistics(&primitives);

        assert_eq!(stats.point_count, 1);
        assert_eq!(stats.line_count, 1);
        assert_eq!(stats.circle_count, 1);
        assert_eq!(stats.total_count, 3);
    }

    #[test]
    fn test_coordinate_transform() {
        let primitives = vec![
            Primitive::Point(Point::new(0.0, 0.0)),
            Primitive::Point(Point::new(100.0, 100.0)),
        ];

        let extractor = CadPrimitiveExtractor::with_defaults();
        let range = extractor.compute_coordinate_range(&primitives).unwrap();
        let transform = extractor.compute_transform(&Some(range));

        assert!((transform.scale_x - 1.0).abs() < 1e-10);
        assert!((transform.scale_y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_extract_primitives_tool() {
        let tools = CadExtractorTools::default();
        
        let svg = r#"<svg width="100" height="100">
            <line x1="0" y1="0" x2="10" y2="10" />
            <circle cx="50" cy="50" r="10" />
        </svg>"#;

        let result = tools.extract_primitives(
            svg.to_string(),
            Some(true),
            Some(vec![0.0, 100.0]),
        );

        assert!(result["success"].as_bool().unwrap_or(false));
        assert!(result["primitives"].as_array().unwrap().len() > 0);
    }
}
