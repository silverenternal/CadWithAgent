//! IGES 文件解析器
//!
//! IGES (Initial Graphics Exchange Specification) 是早期的 CAD 数据交换标准
//! 虽然已被 STEP 取代，但仍在某些行业中使用
//!
//! # 支持的版本
//! - IGES 5.3 (最新版)
//!
//! # 支持的实体类型
//! - 100: Circle
//! - 102: Circular Arc
//! - 106: Ellipse
//! - 108: Polyline
//! - 110: Line
//! - 116: Point
//! - 126: NURBS Curve
//! - 144: Trimmed NURBS
//!
//! # 使用示例
//!
//! ```rust,no_run,ignore
//! use cadagent::parser::iges::IgesParser;
//! use cadagent::error::CadAgentResult;
//!
//! # fn example() -> CadAgentResult<()> {
//! let parser = IgesParser::new();
//! let model = parser.parse("path/to/file.iges")?;
//! let primitives = model.to_primitives();
//! # Ok(())
//! # }
//! ```

use crate::error::{CadAgentError, CadAgentResult};
use crate::geometry::nurbs::{NurbsCurve, Point3D};
use crate::geometry::primitives::{Circle, Line, Point, Polygon, Primitive};
use crate::parser::parser_common::{CadMetadata, ParserConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, info, instrument, warn};

/// IGES 模型表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgesModel {
    /// 模型名称
    pub name: Option<String>,
    /// 几何实体列表
    pub entities: Vec<IgesEntity>,
    /// 元数据
    pub metadata: CadMetadata,
}

impl IgesModel {
    /// 创建空的 IGES 模型
    pub fn new() -> Self {
        Self {
            name: None,
            entities: Vec::new(),
            metadata: CadMetadata::default(),
        }
    }

    /// 转换为 `CadAgent` 图元列表
    pub fn to_primitives(&self) -> Vec<Primitive> {
        let mut primitives = Vec::new();

        for entity in &self.entities {
            if let Some(primitive) = self.entity_to_primitive(entity) {
                primitives.push(primitive);
            }
        }

        primitives
    }

    /// 将单个 IGES 实体转换为图元
    fn entity_to_primitive(&self, entity: &IgesEntity) -> Option<Primitive> {
        match &entity.data {
            IgesEntityData::Point { x, y } => Some(Primitive::Point(Point { x: *x, y: *y })),
            IgesEntityData::Line { start, end } => Some(Primitive::Line(Line {
                start: Point {
                    x: start[0],
                    y: start[1],
                },
                end: Point {
                    x: end[0],
                    y: end[1],
                },
            })),
            IgesEntityData::Circle { center, radius } => Some(Primitive::Circle(Circle {
                center: Point {
                    x: center[0],
                    y: center[1],
                },
                radius: *radius,
            })),
            IgesEntityData::Arc {
                center,
                radius,
                start_angle,
                end_angle,
            } => Some(Primitive::Arc {
                center: Point {
                    x: center[0],
                    y: center[1],
                },
                radius: *radius,
                start_angle: *start_angle,
                end_angle: *end_angle,
            }),
            IgesEntityData::Ellipse {
                center,
                major_axis,
                minor_axis,
                rotation: _,
            } => {
                // 将椭圆转换为 Polygon（离散化近似）
                // IGES 椭圆参数：中心、长轴、短轴、旋转角
                let num_points = 32;
                let mut points = Vec::with_capacity(num_points);

                for i in 0..num_points {
                    let angle = (i as f64 / num_points as f64) * 2.0 * std::f64::consts::PI;
                    let x = center[0] + major_axis * angle.cos();
                    let y = center[1] + minor_axis * angle.sin();
                    points.push([x, y]);
                }

                Some(Primitive::Polygon(Polygon::from_coords(points)))
            }
            IgesEntityData::Polygon { vertices } => {
                if vertices.len() >= 3 {
                    let polygon = Polygon::from_coords(vertices.clone());
                    Some(Primitive::Polygon(polygon))
                } else {
                    None
                }
            }
            IgesEntityData::Polyline { points } => {
                if points.len() >= 2 {
                    let polygon = Polygon::from_coords(points.clone());
                    Some(Primitive::Polygon(polygon))
                } else {
                    None
                }
            }
            IgesEntityData::NurbsCurve {
                control_points,
                weights,
                knot_vector,
                order,
            } => {
                let points: Vec<Point3D> = control_points
                    .iter()
                    .map(|cp| Point3D::new(cp[0], cp[1], cp.get(2).copied().unwrap_or(0.0)))
                    .collect();

                if let Ok(nurbs) =
                    NurbsCurve::new(points, weights.clone(), knot_vector.clone(), *order)
                {
                    let tessellated = nurbs.tessellate(0.01);
                    if tessellated.len() >= 2 {
                        let coords: Vec<[f64; 2]> =
                            tessellated.iter().map(|p| [p.x, p.y]).collect();
                        let polygon = Polygon::from_coords(coords);
                        return Some(Primitive::Polygon(polygon));
                    }
                }
                None
            }
            _ => None,
        }
    }
}

impl Default for IgesModel {
    fn default() -> Self {
        Self::new()
    }
}

/// IGES 实体数据类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IgesEntityData {
    /// 点
    Point { x: f64, y: f64 },
    /// 直线
    Line { start: [f64; 2], end: [f64; 2] },
    /// 圆
    Circle { center: [f64; 2], radius: f64 },
    /// 圆弧
    Arc {
        center: [f64; 2],
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    },
    /// 椭圆
    Ellipse {
        center: [f64; 2],
        major_axis: f64,
        minor_axis: f64,
        rotation: f64,
    },
    /// 多段线
    Polyline { points: Vec<[f64; 2]> },
    /// 多边形
    Polygon { vertices: Vec<[f64; 2]> },
    /// NURBS 曲线
    NurbsCurve {
        control_points: Vec<[f64; 3]>,
        weights: Vec<f64>,
        knot_vector: Vec<f64>,
        order: usize,
    },
    /// 参数样条曲线
    ParametricSpline {
        control_points: Vec<[f64; 2]>,
        degree: usize,
    },
    /// 曲面
    Surface {
        vertices: Vec<[f64; 3]>,
        patches: Vec<[usize; 4]>,
    },
    /// 文本
    Text {
        position: [f64; 2],
        content: String,
        height: f64,
    },
    /// 其他未解析的实体
    Other { entity_type: u16, raw_data: String },
}

/// IGES 实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgesEntity {
    /// 实体 ID
    pub id: usize,
    /// 实体类型编号
    pub entity_type: u16,
    /// 实体数据
    pub data: IgesEntityData,
    /// 层号
    pub layer: u16,
    /// 颜色号
    pub color: u16,
    /// 线型
    pub line_style: u16,
}

/// IGES 解析器
pub struct IgesParser {
    /// Parser configuration
    config: ParserConfig,
}

impl IgesParser {
    /// 创建新的 IGES 解析器
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// 设置容差
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.config = self.config.with_tolerance(tolerance);
        self
    }

    /// 启用调试模式
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.config = self.config.with_debug(debug);
        self
    }

    /// 解析 IGES 文件
    ///
    /// # Errors
    /// 如果文件不存在或格式错误，返回错误
    pub fn parse(&self, path: &Path) -> CadAgentResult<IgesModel> {
        if !path.exists() {
            return Err(CadAgentError::file_not_found(
                path.to_string_lossy().to_string(),
            ));
        }

        let content =
            fs::read_to_string(path).map_err(|e| CadAgentError::io("无法读取 IGES 文件", e))?;

        self.parse_string(&content)
    }

    /// 从字符串解析 IGES 内容
    ///
    /// # Errors
    /// 如果格式错误，返回错误
    #[instrument(skip(self, content), fields(entities_count = 0))]
    pub fn parse_string(&self, content: &str) -> CadAgentResult<IgesModel> {
        let mut model = IgesModel::new();

        // IGES 文件分为 5 个段，每行 80 字符
        // S: Start - 基本信息
        // G: Global - 全局参数
        // D: Directory Entry - 目录项
        // P: Parameter Data - 参数数据
        // T: Terminate - 结束段

        // 解析全局段
        self.parse_global(content, &mut model)?;

        // 解析目录项和参数数据
        self.parse_entities(content, &mut model)?;

        // 记录解析结果
        let span = tracing::Span::current();
        span.record("entities_count", model.entities.len());

        if model.entities.is_empty() {
            warn!("IGES 文件未解析到任何实体");
        } else {
            info!("成功解析 {} 个 IGES 实体", model.entities.len());
        }

        Ok(model)
    }

    /// 解析全局段
    fn parse_global(&self, content: &str, model: &mut IgesModel) -> CadAgentResult<()> {
        // 查找 Global 段（以 G 开头的行）
        for line in content.lines() {
            if line.len() < 80 {
                continue;
            }

            let section = line.chars().nth(72).unwrap_or(' ');
            if section == 'G' {
                // 解析 Global 段参数
                // IGES Global 段有 19 个参数，用逗号分隔
                let params: Vec<&str> = line[..72].split(',').collect();

                if params.len() > 15 {
                    // 参数 15: 版本号
                    model.metadata.source_software = Some(params[15].trim().to_string());
                }

                if params.len() > 13 {
                    // 参数 14: 文件名
                    model.metadata.name = Some(params[13].trim().to_string());
                }

                if params.len() > 16 {
                    // 参数 17: 单位标识
                    let units = match params[16].trim() {
                        "1" => "inches",
                        "2" => "mm",
                        "3" => "meters",
                        _ => "unknown",
                    };
                    model.metadata.units = Some(units.to_string());
                }

                break;
            }
        }

        Ok(())
    }

    /// 解析实体
    fn parse_entities(&self, content: &str, model: &mut IgesModel) -> CadAgentResult<()> {
        // IGES 使用两行表示一个实体：
        // 第一行：Directory Entry (10 个字段)
        // 第二行：Parameter Data

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            if line.len() < 80 {
                i += 1;
                continue;
            }

            let section = line.chars().nth(72).unwrap_or(' ');

            // 目录项段
            if section == 'D' {
                // 解析 Directory Entry
                // 格式：每个字段 8 个字符
                let entity_type_str = line[0..8].trim();
                if let Ok(entity_type) = entity_type_str.parse::<u16>() {
                    // 读取参数数据行
                    if i + 1 < lines.len() {
                        let param_line = lines[i + 1];
                        if param_line.len() >= 80
                            && param_line.chars().nth(72).unwrap_or(' ') == 'P'
                        {
                            // 解析参数
                            if let Some(entity) =
                                self.parse_iges_entity(entity_type, &param_line[..72], line)?
                            {
                                model.entities.push(entity);
                            }
                        }
                        i += 2;
                        continue;
                    }
                }
            }

            i += 1;
        }

        Ok(())
    }

    /// 解析单个 IGES 实体
    fn parse_iges_entity(
        &self,
        entity_type: u16,
        param_data: &str,
        _dir_entry: &str,
    ) -> CadAgentResult<Option<IgesEntity>> {
        // 解析参数
        let params = self.parse_iges_parameters(param_data);

        let entity_data = match entity_type {
            // Type 116: Point
            116 => self.parse_iges_point(&params)?,
            // Type 110: Line
            110 => self.parse_iges_line(&params)?,
            // Type 100: Circle
            100 => self.parse_iges_circle(&params)?,
            // Type 102: Arc
            102 => self.parse_iges_arc(&params)?,
            // Type 106: Ellipse
            106 => self.parse_iges_ellipse(&params)?,
            // Type 108: Polyline
            108 => self.parse_iges_polyline(&params)?,
            // Type 126: NURBS Curve
            126 => self.parse_iges_nurbs(&params)?,
            // Type 144: Trimmed NURBS
            144 => self.parse_iges_trimmed_nurbs(&params)?,
            _ => IgesEntityData::Other {
                entity_type,
                raw_data: param_data.to_string(),
            },
        };

        Ok(Some(IgesEntity {
            id: 0,
            entity_type,
            data: entity_data,
            layer: 0,
            color: 0,
            line_style: 0,
        }))
    }

    /// 解析 IGES 参数
    fn parse_iges_parameters(&self, param_data: &str) -> Vec<String> {
        let mut params = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut paren_depth = 0;

        for ch in param_data.chars() {
            match ch {
                '\'' | '"' => {
                    in_string = !in_string;
                    current.push(ch);
                }
                '(' => {
                    paren_depth += 1;
                    current.push(ch);
                }
                ')' => {
                    paren_depth -= 1;
                    current.push(ch);
                }
                ',' if !in_string && paren_depth == 0 => {
                    params.push(current.trim().to_string());
                    current = String::new();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            params.push(current.trim().to_string());
        }

        params
    }

    /// 解析点 (Type 116)
    fn parse_iges_point(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        if params.len() < 2 {
            return Err(CadAgentError::parse("IGES", "POINT 参数不足"));
        }

        let x = params[0]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的 X 坐标"))?;
        let y = params[1]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的 Y 坐标"))?;

        Ok(IgesEntityData::Point { x, y })
    }

    /// 解析直线 (Type 110)
    fn parse_iges_line(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        if params.len() < 4 {
            return Err(CadAgentError::parse("IGES", "LINE 参数不足"));
        }

        let x1 = params[0]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的起点 X 坐标"))?;
        let y1 = params[1]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的起点 Y 坐标"))?;
        let x2 = params[2]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的终点 X 坐标"))?;
        let y2 = params[3]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的终点 Y 坐标"))?;

        Ok(IgesEntityData::Line {
            start: [x1, y1],
            end: [x2, y2],
        })
    }

    /// 解析圆 (Type 100)
    fn parse_iges_circle(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        if params.len() < 4 {
            return Err(CadAgentError::parse("IGES", "CIRCLE 参数不足"));
        }

        let cx = params[0]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 X 坐标"))?;
        let cy = params[1]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 Y 坐标"))?;
        let _cz = params[2]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 Z 坐标"))?;
        let radius = params[3]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的半径"))?;

        Ok(IgesEntityData::Circle {
            center: [cx, cy],
            radius,
        })
    }

    /// 解析圆弧 (Type 102)
    fn parse_iges_arc(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        if params.len() < 7 {
            return Err(CadAgentError::parse("IGES", "ARC 参数不足"));
        }

        let cx = params[0]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 X 坐标"))?;
        let cy = params[1]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 Y 坐标"))?;
        let _cz = params[2]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 Z 坐标"))?;
        let radius = params[3]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的半径"))?;
        let start_angle = params[4]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的起始角度"))?;
        let end_angle = params[5]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的终止角度"))?;
        let _ccw = params[6].parse::<f64>().unwrap_or(1.0);

        Ok(IgesEntityData::Arc {
            center: [cx, cy],
            radius,
            start_angle,
            end_angle,
        })
    }

    /// 解析椭圆 (Type 106)
    fn parse_iges_ellipse(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        if params.len() < 6 {
            return Err(CadAgentError::parse("IGES", "ELLIPSE 参数不足"));
        }

        let cx = params[0]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 X 坐标"))?;
        let cy = params[1]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 Y 坐标"))?;
        let _cz = params[2]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的圆心 Z 坐标"))?;
        let major_axis = params[3]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的长轴"))?;
        let minor_axis = params[4]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的短轴"))?;
        let rotation = params[5]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("IGES", "无效的旋转角度"))?;

        Ok(IgesEntityData::Ellipse {
            center: [cx, cy],
            major_axis,
            minor_axis,
            rotation,
        })
    }

    /// 解析多段线 (Type 108)
    fn parse_iges_polyline(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        let mut points = Vec::new();
        let mut i = 0;

        while i + 1 < params.len() {
            let x = params[i].parse::<f64>().ok();
            let y = params[i + 1].parse::<f64>().ok();

            if let (Some(x), Some(y)) = (x, y) {
                points.push([x, y]);
            }
            i += 2;
        }

        Ok(IgesEntityData::Polyline { points })
    }

    /// 解析 NURBS 曲线 (Type 126)
    fn parse_iges_nurbs(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        // IGES NURBS 126 格式:
        // NURBS 参数按以下顺序排列:
        // 1. 维度 (通常为 3 表示 3D)
        // 2. 阶数 (order = degree + 1)
        // 3. 控制点数量
        // 4. 是否闭合 (0=开，1=闭，2=周期)
        // 5. 是否是有理曲线 (0=非有理，1=有理)
        // 6. 控制点坐标 (x, y, z 重复 n 次)
        // 7. 权重 (如果有理曲线)
        // 8. 节点向量值

        if params.len() < 6 {
            return Err(CadAgentError::parse("IGES", "NURBS 参数不足"));
        }

        // 解析基本参数
        let _dimension = params[0].parse::<usize>().unwrap_or(3);
        let order = params[1].parse::<usize>().unwrap_or(3);
        let num_control_points = params[2].parse::<usize>().unwrap_or(4);
        let _closed = params[3].parse::<i32>().unwrap_or(0);
        let _rational = params[4].parse::<i32>().unwrap_or(1);

        // 解析控制点
        let mut control_points = Vec::with_capacity(num_control_points);
        let mut idx = 5;

        for _ in 0..num_control_points {
            if idx + 2 >= params.len() {
                break;
            }

            let x = params[idx].parse::<f64>().unwrap_or(0.0);
            let y = params[idx + 1].parse::<f64>().unwrap_or(0.0);
            let z = params[idx + 2].parse::<f64>().unwrap_or(0.0);

            control_points.push([x, y, z]);
            idx += 3;
        }

        if control_points.is_empty() {
            return Err(CadAgentError::parse("IGES", "NURBS 缺少控制点"));
        }

        // 解析权重 (如果有理曲线)
        let mut weights = vec![1.0; control_points.len()];
        if _rational != 0 && idx < params.len() {
            for i in 0..control_points.len() {
                if idx + i >= params.len() {
                    break;
                }
                weights[i] = params[idx + i].parse::<f64>().unwrap_or(1.0);
            }
            idx += control_points.len();
        }

        // 解析节点向量
        let num_knots = num_control_points + order;
        let mut knot_vector = Vec::with_capacity(num_knots);

        for i in 0..num_knots {
            if idx + i >= params.len() {
                // 如果节点向量不完整，使用均匀节点向量
                knot_vector.push(i as f64 / (num_knots - 1) as f64);
            } else {
                knot_vector.push(params[idx + i].parse::<f64>().unwrap_or(i as f64));
            }
        }

        debug!(
            "解析 NURBS: {} 个控制点，阶数={}, 节点向量长度={}",
            control_points.len(),
            order,
            knot_vector.len()
        );

        Ok(IgesEntityData::NurbsCurve {
            control_points,
            weights,
            knot_vector,
            order,
        })
    }

    /// 解析修剪 NURBS (Type 144)
    fn parse_iges_trimmed_nurbs(&self, params: &[String]) -> CadAgentResult<IgesEntityData> {
        // 简化处理
        self.parse_iges_nurbs(params)
    }
}

impl Default for IgesParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iges_parser_creation() {
        let parser = IgesParser::new();
        assert_eq!(parser.config.tolerance, 1e-6);
        assert!(!parser.config.debug);

        let parser = IgesParser::new().with_tolerance(1e-8).with_debug(true);
        assert_eq!(parser.config.tolerance, 1e-8);
        assert!(parser.config.debug);
    }

    #[test]
    fn test_iges_model_creation() {
        let mut model = IgesModel::new();
        assert!(model.name.is_none());
        assert!(model.entities.is_empty());

        model.name = Some("Test".to_string());
        model.entities.push(IgesEntity {
            id: 1,
            entity_type: 110,
            data: IgesEntityData::Line {
                start: [0.0, 0.0],
                end: [1.0, 1.0],
            },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        assert_eq!(model.name, Some("Test".to_string()));
        assert_eq!(model.entities.len(), 1);
    }

    #[test]
    fn test_iges_entity_conversion() {
        let mut model = IgesModel::new();
        model.entities.push(IgesEntity {
            id: 1,
            entity_type: 100,
            data: IgesEntityData::Circle {
                center: [0.0, 0.0],
                radius: 5.0,
            },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        let primitives = model.to_primitives();
        assert_eq!(primitives.len(), 1);

        if let Primitive::Circle(circle) = &primitives[0] {
            assert_eq!(circle.center.x, 0.0);
            assert_eq!(circle.center.y, 0.0);
            assert_eq!(circle.radius, 5.0);
        } else {
            panic!("Expected Circle primitive");
        }
    }

    #[test]
    fn test_iges_arc_conversion() {
        let mut model = IgesModel::new();
        model.entities.push(IgesEntity {
            id: 1,
            entity_type: 102,
            data: IgesEntityData::Arc {
                center: [0.0, 0.0],
                radius: 5.0,
                start_angle: 0.0,
                end_angle: std::f64::consts::PI / 2.0,
            },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        let primitives = model.to_primitives();
        assert_eq!(primitives.len(), 1);

        if let Primitive::Arc {
            center,
            radius,
            start_angle,
            end_angle,
        } = &primitives[0]
        {
            assert_eq!(center.x, 0.0);
            assert_eq!(center.y, 0.0);
            assert_eq!(*radius, 5.0);
            assert!((start_angle - 0.0).abs() < 1e-6);
            assert!((end_angle - std::f64::consts::PI / 2.0).abs() < 1e-6);
        } else {
            panic!("Expected Arc primitive");
        }
    }

    #[test]
    fn test_iges_ellipse_conversion() {
        let mut model = IgesModel::new();
        model.entities.push(IgesEntity {
            id: 1,
            entity_type: 106,
            data: IgesEntityData::Ellipse {
                center: [0.0, 0.0],
                major_axis: 10.0,
                minor_axis: 5.0,
                rotation: 0.0,
            },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        let primitives = model.to_primitives();
        assert_eq!(primitives.len(), 1);

        // 椭圆被转换为多边形（离散化近似）
        if let Primitive::Polygon(polygon) = &primitives[0] {
            // 应该有 32 个离散点
            assert_eq!(polygon.vertices.len(), 32);
        } else {
            panic!("Expected Polygon primitive (ellipse tessellated)");
        }
    }

    #[test]
    fn test_iges_polyline_conversion() {
        let mut model = IgesModel::new();
        model.entities.push(IgesEntity {
            id: 1,
            entity_type: 108,
            data: IgesEntityData::Polyline {
                points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        let primitives = model.to_primitives();
        assert_eq!(primitives.len(), 1);

        if let Primitive::Polygon(polygon) = &primitives[0] {
            assert_eq!(polygon.vertices.len(), 4);
        } else {
            panic!("Expected Polygon primitive");
        }
    }

    #[test]
    fn test_iges_nurbs_parsing() {
        let parser = IgesParser::new();

        // 简化的 NURBS 参数：维度=3, 阶数=3, 控制点数=4, 开曲线，有理曲线
        // 然后是 4 个控制点 (x,y,z), 4 个权重，节点向量
        let params = vec![
            "3".to_string(), // 维度
            "3".to_string(), // 阶数
            "4".to_string(), // 控制点数
            "0".to_string(), // 开曲线
            "1".to_string(), // 有理曲线
            "0.0".to_string(),
            "0.0".to_string(),
            "0.0".to_string(), // 控制点 1
            "1.0".to_string(),
            "0.0".to_string(),
            "0.0".to_string(), // 控制点 2
            "1.0".to_string(),
            "1.0".to_string(),
            "0.0".to_string(), // 控制点 3
            "0.0".to_string(),
            "1.0".to_string(),
            "0.0".to_string(), // 控制点 4
            "1.0".to_string(),
            "1.0".to_string(),
            "1.0".to_string(),
            "1.0".to_string(), // 权重
            "0.0".to_string(),
            "0.33".to_string(),
            "0.67".to_string(),
            "1.0".to_string(), // 节点向量
        ];

        let result = parser.parse_iges_nurbs(&params);
        assert!(result.is_ok());

        if let IgesEntityData::NurbsCurve {
            control_points,
            weights,
            knot_vector,
            order,
        } = result.unwrap()
        {
            assert_eq!(control_points.len(), 4);
            assert_eq!(weights.len(), 4);
            assert_eq!(order, 3);
            assert!(knot_vector.len() >= 4);
        } else {
            panic!("Expected NurbsCurve");
        }
    }

    #[test]
    fn test_parse_iges_parameters() {
        let parser = IgesParser::new();
        let params = parser.parse_iges_parameters("1, 2, 3, 4");
        assert_eq!(params, vec!["1", "2", "3", "4"]);

        let params = parser.parse_iges_parameters("'string', 1, 2");
        assert_eq!(params, vec!["'string'", "1", "2"]);
    }

    #[test]
    fn test_iges_multiple_entities() {
        let mut model = IgesModel::new();

        // 添加不同类型的实体
        model.entities.push(IgesEntity {
            id: 1,
            entity_type: 116,
            data: IgesEntityData::Point { x: 0.0, y: 0.0 },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        model.entities.push(IgesEntity {
            id: 2,
            entity_type: 110,
            data: IgesEntityData::Line {
                start: [0.0, 0.0],
                end: [1.0, 1.0],
            },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        model.entities.push(IgesEntity {
            id: 3,
            entity_type: 100,
            data: IgesEntityData::Circle {
                center: [0.5, 0.5],
                radius: 2.0,
            },
            layer: 0,
            color: 0,
            line_style: 0,
        });

        let primitives = model.to_primitives();
        assert_eq!(primitives.len(), 3);

        // 验证类型
        assert!(matches!(primitives[0], Primitive::Point(_)));
        assert!(matches!(primitives[1], Primitive::Line(_)));
        assert!(matches!(primitives[2], Primitive::Circle(_)));
    }
}
