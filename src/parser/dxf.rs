//! DXF 解析器
//!
//! 解析 DXF 文件，提取几何图元并转换为结构化数据
//!
//! 支持的 DXF 实体：
//! - LINE (线段)
//! - CIRCLE (圆)
//! - ARC (圆弧)
//! - LWPOLYLINE (轻量多段线)
//! - POLYLINE (多段线)
//! - TEXT (文本)
//! - MTEXT (多行文本)
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use cadagent::parser::dxf::DxfParser;
//!
//! let result = DxfParser::parse("floor_plan.dxf").unwrap();
//! println!("解析到 {} 个图元", result.primitives.len());
//! ```

use crate::geometry::{Point, Line, Polygon, Circle, Primitive};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Cursor;
use std::path::Path;

/// DXF 解析器
pub struct DxfParser;

impl DxfParser {
    /// 解析 DXF 文件（带路径遍历防护）
    ///
    /// # Security
    /// 会验证路径在当前工作目录内，防止路径遍历攻击
    ///
    /// # Errors
    /// 如果文件不存在、路径不安全或解析失败，返回 `DxfError`
    pub fn parse(path: impl AsRef<Path>) -> Result<DxfResult, DxfError> {
        let path = path.as_ref();

        // 路径遍历防护：验证路径在当前工作目录内
        let canonical_path = path.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DxfError::FileNotFound(format!("文件不存在：{}", path.display()))
            } else {
                DxfError::Io(e)
            }
        })?;

        let cwd = std::env::current_dir().map_err(DxfError::Io)?;

        // 检查路径是否在当前工作目录内
        if !canonical_path.starts_with(&cwd) {
            return Err(DxfError::Security(format!(
                "路径遍历检测：路径 {} 不在当前工作目录内",
                canonical_path.display()
            )));
        }

        let mut file = fs::File::open(&canonical_path)?;
        let drawing = dxf::Drawing::load(&mut file)
            .map_err(|e| DxfError::ParseError(format!("DXF 加载失败：{}", e)))?;

        Self::from_drawing(&drawing)
    }

    /// 解析 DXF 字符串
    ///
    /// # Errors
    /// 如果解析失败，返回 `DxfError`
    pub fn parse_string(content: &str) -> Result<DxfResult, DxfError> {
        let mut cursor = Cursor::new(content.as_bytes());
        let drawing = dxf::Drawing::load(&mut cursor)
            .map_err(|e| DxfError::ParseError(format!("DXF 解析失败：{}", e)))?;

        Self::from_drawing(&drawing)
    }

    /// 从 DXF Drawing 提取图元
    fn from_drawing(drawing: &dxf::Drawing) -> Result<DxfResult, DxfError> {
        let mut primitives = Vec::new();
        let mut metadata = DxfMetadata::default();

        // 提取元数据
        // 注意：DXF Header 字段名称较为复杂，当前简化处理
        metadata.version = drawing.header.version.to_string();
        // TODO: 完善 header 字段访问
        metadata.units = "Unknown".to_string();

        // 解析实体 - 使用迭代器 API
        // 注意：DXF crate API 较为复杂，当前为简化实现
        // TODO: 完善所有实体类型的解析
        for entity_item in drawing.entities() {
            let entity = entity_item;
            match &entity.specific {
                dxf::entities::EntityType::Line(line) => {
                    primitives.push(Primitive::Line(Line::new(
                        Point::new(line.p1.x, line.p1.y),
                        Point::new(line.p2.x, line.p2.y),
                    )));
                }
                dxf::entities::EntityType::Circle(circle) => {
                    if circle.radius > 0.0 {
                        primitives.push(Primitive::Circle(Circle::from_coords(
                            [circle.center.x, circle.center.y],
                            circle.radius,
                        )));
                    }
                }
                dxf::entities::EntityType::Arc(arc) => {
                    if arc.radius > 0.0 {
                        primitives.push(Primitive::Arc {
                            center: Point::new(arc.center.x, arc.center.y),
                            radius: arc.radius,
                            start_angle: arc.start_angle,
                            end_angle: arc.end_angle,
                        });
                    }
                }
                dxf::entities::EntityType::LwPolyline(lwpolyline) => {
                    // 使用 vertices 字段访问
                    let points: Vec<Point> = lwpolyline.vertices.iter()
                        .map(|v| Point::new(v.x, v.y))
                        .collect();
                    
                    if points.len() >= 2 {
                        let is_closed = lwpolyline.is_closed();
                        if points.len() >= 3 && is_closed {
                            primitives.push(Primitive::Polygon(Polygon::new(points)));
                        } else {
                            primitives.push(Primitive::Polyline { points, closed: is_closed });
                        }
                    }
                }
                dxf::entities::EntityType::Polyline(polyline) => {
                    // 使用 vertices() 方法访问（返回迭代器）
                    let points: Vec<Point> = polyline.vertices()
                        .map(|v| Point::new(v.location.x, v.location.y))
                        .collect();
                    
                    if points.len() >= 2 {
                        let is_closed = polyline.is_closed();
                        if points.len() >= 3 && is_closed {
                            primitives.push(Primitive::Polygon(Polygon::new(points)));
                        }
                    }
                }
                dxf::entities::EntityType::Text(text) => {
                    // Text 字段名称可能不同，使用 value 作为备选
                    let text_content = text.value.clone();
                    if !text_content.is_empty() {
                        primitives.push(Primitive::Text {
                            content: text_content,
                            position: Point::new(text.location.x, text.location.y),
                            height: text.text_height,
                        });
                    }
                }
                dxf::entities::EntityType::MText(mtext) => {
                    if !mtext.text.is_empty() {
                        let cleaned_text = clean_mtext_formatting(&mtext.text);
                        primitives.push(Primitive::Text {
                            content: cleaned_text,
                            position: Point::new(mtext.insertion_point.x, mtext.insertion_point.y),
                            height: mtext.initial_text_height,
                        });
                    }
                }
                _ => {
                    // 忽略其他实体类型（Spline, Ellipse, Point 等）
                    // 后续可以添加更多支持
                }
            }
        }

        Ok(DxfResult {
            primitives,
            metadata,
        })
    }
}

/// 清理 MTEXT 格式代码
fn clean_mtext_formatting(text: &str) -> String {
    // 移除常见的 MTEXT 格式代码
    text.replace("\\P", "\n")
        .replace("\\X", "")
        .replace("{\\", "")
        .replace("}", "")
        .replace("\\f", "")
        .replace("\\q", "")
        .replace("\\l", "")
        .replace("\\r", "")
        .replace("\\c", "")
        .split(';')
        .collect::<Vec<_>>()
        .join("")
}

/// DXF 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfResult {
    pub primitives: Vec<Primitive>,
    pub metadata: DxfMetadata,
}

/// DXF 元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DxfMetadata {
    /// DXF 版本
    pub version: String,
    /// 单位编码
    pub units: String,
    /// 最小范围 [x, y, z]
    pub extmin: Option<[f64; 3]>,
    /// 最大范围 [x, y, z]
    pub extmax: Option<[f64; 3]>,
}

/// DXF 错误类型
#[derive(Debug, thiserror::Error)]
pub enum DxfError {
    #[error("文件读取失败：{0}")]
    Io(#[from] std::io::Error),

    #[error("文件不存在：{0}")]
    FileNotFound(String),

    #[error("DXF 解析失败：{0}")]
    ParseError(String),

    #[error("安全错误：{0}")]
    Security(String),

    #[error("不支持的 DXF 版本：{0}")]
    UnsupportedVersion(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_dxf() {
        // 创建一个简单的 DXF 字符串
        let dxf_content = r#"0
SECTION
2
HEADER
9
$ACADVER
1
AC1015
0
ENDSEC
0
SECTION
2
ENTITIES
0
LINE
8
0
10
0.0
20
0.0
30
0.0
11
100.0
21
0.0
31
0.0
0
CIRCLE
8
0
10
50.0
20
50.0
30
0.0
40
25.0
0
ENDSEC
0
EOF
"#;

        let result = DxfParser::parse_string(dxf_content).unwrap();

        assert_eq!(result.primitives.len(), 2);
        assert_eq!(result.metadata.version, "AC1015");

        // 检查线段
        match &result.primitives[0] {
            Primitive::Line(line) => {
                assert!((line.start.x - 0.0).abs() < 0.001);
                assert!((line.start.y - 0.0).abs() < 0.001);
                assert!((line.end.x - 100.0).abs() < 0.001);
                assert!((line.end.y - 0.0).abs() < 0.001);
            }
            _ => panic!("期望线段"),
        }

        // 检查圆
        match &result.primitives[1] {
            Primitive::Circle(circle) => {
                assert!((circle.center.x - 50.0).abs() < 0.001);
                assert!((circle.center.y - 50.0).abs() < 0.001);
                assert!((circle.radius - 25.0).abs() < 0.001);
            }
            _ => panic!("期望圆"),
        }
    }

    #[test]
    fn test_parse_polygon() {
        let dxf_content = r#"0
SECTION
2
HEADER
9
$ACADVER
1
AC1015
0
ENDSEC
0
SECTION
2
ENTITIES
0
LWPOLYLINE
8
0
90
4
70
1
10
0.0
20
0.0
10
100.0
20
0.0
10
100.0
20
100.0
10
0.0
20
100.0
0
ENDSEC
0
EOF
"#;

        let result = DxfParser::parse_string(dxf_content).unwrap();

        assert_eq!(result.primitives.len(), 1);
        match &result.primitives[0] {
            Primitive::Polygon(poly) => {
                assert_eq!(poly.vertices.len(), 4);
            }
            _ => panic!("期望多边形"),
        }
    }

    #[test]
    fn test_parse_text() {
        let dxf_content = r#"0
SECTION
2
HEADER
9
$ACADVER
1
AC1015
0
ENDSEC
0
SECTION
2
ENTITIES
0
TEXT
8
0
10
50.0
20
50.0
30
0.0
40
100.0
1
Hello World
0
ENDSEC
0
EOF
"#;

        let result = DxfParser::parse_string(dxf_content).unwrap();

        assert_eq!(result.primitives.len(), 1);
        match &result.primitives[0] {
            Primitive::Text { content, position, height } => {
                assert_eq!(content, "Hello World");
                assert!((position.x - 50.0).abs() < 0.001);
                assert!((position.y - 50.0).abs() < 0.001);
                assert!((height - 100.0).abs() < 0.001);
            }
            _ => panic!("期望文本"),
        }
    }

    #[test]
    fn test_parse_arc() {
        let dxf_content = r#"0
SECTION
2
HEADER
9
$ACADVER
1
AC1015
0
ENDSEC
0
SECTION
2
ENTITIES
0
ARC
8
0
10
0.0
20
0.0
30
0.0
40
50.0
50
0.0
51
90.0
0
ENDSEC
0
EOF
"#;

        let result = DxfParser::parse_string(dxf_content).unwrap();

        assert_eq!(result.primitives.len(), 1);
        match &result.primitives[0] {
            Primitive::Arc { center, radius, start_angle, end_angle } => {
                assert!((center.x - 0.0).abs() < 0.001);
                assert!((center.y - 0.0).abs() < 0.001);
                assert!((radius - 50.0).abs() < 0.001);
                assert!((start_angle - 0.0).abs() < 0.001);
                assert!((end_angle - 90.0).abs() < 0.001);
            }
            _ => panic!("期望圆弧"),
        }
    }
}
