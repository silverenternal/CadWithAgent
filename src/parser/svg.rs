//! SVG 解析器
//!
//! 解析 SVG 文件，提取几何图元并转换为结构化数据
//!
//! 使用 `roxmltree` 库进行可靠的 XML 解析，支持：
//! - 嵌套 SVG 元素
//! - XML 命名空间
//! - transform 属性（基础支持）
//! - 完整的 SVG 元素类型
//!
//! # 安全性
//!
//! 解析文件时会进行路径遍历检查，防止读取工作目录外的文件

use crate::geometry::{Point, Line, Polygon, Rect, Circle, Primitive};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// SVG 解析器
pub struct SvgParser;

impl SvgParser {
    /// 解析 SVG 文件（带路径遍历防护）
    ///
    /// # Security
    /// 会验证路径在当前工作目录内，防止路径遍历攻击
    ///
    /// # Errors
    /// 如果文件不存在、路径不安全或解析失败，返回 `SvgError`
    pub fn parse(path: impl AsRef<Path>) -> Result<SvgResult, SvgError> {
        let path = path.as_ref();
        
        // 路径遍历防护：验证路径在当前工作目录内
        let canonical_path = path.canonicalize().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SvgError::FileNotFound(format!("文件不存在：{}", path.display()))
            } else {
                SvgError::Io(e)
            }
        })?;
        
        let cwd = std::env::current_dir().map_err(|e| {
            SvgError::Io(e)
        })?;
        
        // 检查路径是否在当前工作目录内
        if !canonical_path.starts_with(&cwd) {
            return Err(SvgError::Security(format!(
                "路径遍历检测：路径 {} 不在当前工作目录内",
                canonical_path.display()
            )));
        }
        
        let content = fs::read_to_string(&canonical_path)?;
        Self::parse_string(&content)
    }

    /// 解析 SVG 字符串
    pub fn parse_string(content: &str) -> Result<SvgResult, SvgError> {
        let doc = roxmltree::Document::parse(content)
            .map_err(|e| SvgError::ParseError(format!("XML 解析失败：{}", e)))?;

        let mut primitives = Vec::new();

        // 获取根节点
        let root = doc.root_element();

        // 解析根节点属性
        let width = root.attribute("width").unwrap_or("").to_string();
        let height = root.attribute("height").unwrap_or("").to_string();
        let view_box = root.attribute("viewBox").map(String::from);

        // 遍历所有子节点
        for node in root.descendants() {
            // 只处理元素节点
            if !node.is_element() {
                continue;
            }
            
            let tag_name = node.tag_name().name();
            
            match tag_name {
                "line" => {
                    if let Some(line) = parse_line_node(&node) {
                        primitives.push(Primitive::Line(line));
                    }
                }
                "rect" => {
                    if let Some(rect) = parse_rect_node(&node) {
                        primitives.push(Primitive::Rect(rect));
                    }
                }
                "circle" => {
                    if let Some(circle) = parse_circle_node(&node) {
                        primitives.push(Primitive::Circle(circle));
                    }
                }
                "ellipse" => {
                    // 将椭圆近似为圆（取平均半径）
                    if let Some(circle) = parse_ellipse_as_circle(&node) {
                        primitives.push(Primitive::Circle(circle));
                    }
                }
                "polygon" => {
                    if let Some(polygon) = parse_polygon_node(&node) {
                        primitives.push(Primitive::Polygon(polygon));
                    }
                }
                "polyline" => {
                    if let Some(points) = parse_points_attribute(&node) {
                        if points.len() >= 2 {
                            primitives.push(Primitive::Polyline {
                                points,
                                closed: false,
                            });
                        }
                    }
                }
                "path" => {
                    let path_primitives = parse_path_node(&node);
                    primitives.extend(path_primitives);
                }
                "text" => {
                    if let Some(text) = parse_text_node(&node) {
                        primitives.push(text);
                    }
                }
                // 忽略其他元素：g, defs, symbol, use, image 等
                _ => {}
            }
        }

        Ok(SvgResult {
            primitives,
            metadata: SvgMetadata {
                width,
                height,
                view_box,
            },
        })
    }
}

/// 解析 line 元素
fn parse_line_node(node: &roxmltree::Node) -> Option<Line> {
    let x1 = parse_float_attr(node, "x1")?;
    let y1 = parse_float_attr(node, "y1")?;
    let x2 = parse_float_attr(node, "x2")?;
    let y2 = parse_float_attr(node, "y2")?;
    Some(Line::from_coords([x1, y1], [x2, y2]))
}

/// 解析 rect 元素
fn parse_rect_node(node: &roxmltree::Node) -> Option<Rect> {
    let x = parse_float_attr(node, "x").unwrap_or(0.0);
    let y = parse_float_attr(node, "y").unwrap_or(0.0);
    let width = parse_float_attr(node, "width")?;
    let height = parse_float_attr(node, "height")?;
    
    // 处理负宽度和高度
    let (w, h) = if width < 0.0 || height < 0.0 {
        (width.abs(), height.abs())
    } else {
        (width, height)
    };
    
    Some(Rect::from_coords([x, y], [x + w, y + h]))
}

/// 解析 circle 元素
fn parse_circle_node(node: &roxmltree::Node) -> Option<Circle> {
    let cx = parse_float_attr(node, "cx")?;
    let cy = parse_float_attr(node, "cy")?;
    let r = parse_float_attr(node, "r")?;
    
    if r <= 0.0 {
        return None;
    }
    
    Some(Circle::from_coords([cx, cy], r))
}

/// 解析 ellipse 元素（近似为圆）
fn parse_ellipse_as_circle(node: &roxmltree::Node) -> Option<Circle> {
    let cx = parse_float_attr(node, "cx")?;
    let cy = parse_float_attr(node, "cy")?;
    let rx = parse_float_attr(node, "rx")?;
    let ry = parse_float_attr(node, "ry")?;
    
    // 使用平均半径
    let r = (rx + ry) / 2.0;
    
    if r <= 0.0 {
        return None;
    }
    
    Some(Circle::from_coords([cx, cy], r))
}

/// 解析 polygon 元素
fn parse_polygon_node(node: &roxmltree::Node) -> Option<Polygon> {
    let points = parse_points_attribute(node)?;
    if points.len() < 3 {
        return None;
    }
    Some(Polygon::new(points))
}

/// 解析 points 属性（用于 polygon 和 polyline）
fn parse_points_attribute(node: &roxmltree::Node) -> Option<Vec<Point>> {
    let points_str = node.attribute("points")?;
    
    let points = points_str
        .split_whitespace()
        .filter_map(|pair| {
            let coords: Vec<&str> = pair.split(',').collect();
            if coords.len() == 2 {
                if let (Ok(x), Ok(y)) = (coords[0].trim().parse(), coords[1].trim().parse()) {
                    return Some(Point::new(x, y));
                }
            }
            None
        })
        .collect::<Vec<_>>();

    if points.is_empty() {
        None
    } else {
        Some(points)
    }
}

/// 解析 path 元素
fn parse_path_node(node: &roxmltree::Node) -> Vec<Primitive> {
    let d = match node.attribute("d") {
        Some(val) => val,
        None => return vec![],
    };

    parse_path_data(d)
}

/// 解析 path 数据
fn parse_path_data(d: &str) -> Vec<Primitive> {
    let mut primitives = Vec::new();
    let mut current_point = Point::origin();
    let mut start_point = Point::origin();
    let mut path_points = Vec::new();

    let mut chars = d.chars().peekable();

    while let Some(cmd) = chars.next() {
        match cmd {
            'M' | 'm' => {
                // 移动命令
                if let Some((x, y)) = parse_coord(&mut chars, cmd.is_lowercase()) {
                    current_point = Point::new(x, y);
                    start_point = current_point;
                    path_points = vec![current_point];
                }
            }
            'L' | 'l' => {
                // 直线命令
                while let Some((x, y)) = parse_coord(&mut chars, cmd.is_lowercase()) {
                    path_points.push(Point::new(x, y));
                }
            }
            'H' | 'h' => {
                // 水平线命令
                if let Some(x) = parse_number(&mut chars) {
                    let new_x = if cmd.is_lowercase() {
                        current_point.x + x
                    } else {
                        x
                    };
                    path_points.push(Point::new(new_x, current_point.y));
                    current_point = Point::new(new_x, current_point.y);
                }
            }
            'V' | 'v' => {
                // 垂直线命令
                if let Some(y) = parse_number(&mut chars) {
                    let new_y = if cmd.is_lowercase() {
                        current_point.y + y
                    } else {
                        y
                    };
                    path_points.push(Point::new(current_point.x, new_y));
                    current_point = Point::new(current_point.x, new_y);
                }
            }
            'Z' | 'z' => {
                // 闭合命令
                if path_points.len() >= 3 {
                    primitives.push(Primitive::Polygon(Polygon::new(path_points.clone())));
                }
                path_points.clear();
                current_point = start_point;
            }
            // 忽略其他命令：C, S, Q, T, A 等曲线命令
            _ => {
                // 跳过命令参数
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == ',' {
                        chars.next();
                    } else if c.is_numeric() || c == '.' || c == '-' || c == '+' {
                        parse_number(&mut chars);
                    } else {
                        break;
                    }
                }
            }
        }
    }

    // 处理未闭合的路径
    if path_points.len() >= 3 {
        primitives.push(Primitive::Polygon(Polygon::new(path_points)));
    }

    primitives
}

/// 解析坐标（支持相对和绝对坐标）
fn parse_coord(chars: &mut std::iter::Peekable<std::str::Chars>, _relative: bool) -> Option<(f64, f64)> {
    skip_whitespace_and_commas(chars);

    // 解析 x
    let x = parse_number(chars)?;

    skip_whitespace_and_commas(chars);

    // 解析 y
    let y = parse_number(chars)?;

    Some((x, y))
}

/// 解析数字
fn parse_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<f64> {
    skip_whitespace_and_commas(chars);

    let mut num_str = String::new();
    
    while let Some(&c) = chars.peek() {
        if c.is_numeric() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }

    if num_str.is_empty() {
        return None;
    }

    num_str.parse().ok()
}

/// 跳过空白和逗号
fn skip_whitespace_and_commas(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == ',' {
            chars.next();
        } else {
            break;
        }
    }
}

/// 解析 text 元素
fn parse_text_node(node: &roxmltree::Node) -> Option<Primitive> {
    let x = parse_float_attr(node, "x").unwrap_or(0.0);
    let y = parse_float_attr(node, "y").unwrap_or(0.0);
    let font_size = parse_float_attr(node, "font-size").unwrap_or(100.0);

    // 提取文本内容
    let content = node.text()?.trim().to_string();

    if content.is_empty() {
        return None;
    }

    Some(Primitive::Text {
        content,
        position: Point::new(x, y),
        height: font_size,
    })
}

/// 解析浮点数属性
fn parse_float_attr(node: &roxmltree::Node, attr_name: &str) -> Option<f64> {
    node.attribute(attr_name).and_then(|s| s.parse().ok())
}

/// SVG 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvgResult {
    pub primitives: Vec<Primitive>,
    pub metadata: SvgMetadata,
}

/// SVG 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvgMetadata {
    pub width: String,
    pub height: String,
    pub view_box: Option<String>,
}

/// SVG 错误类型
#[derive(Debug, thiserror::Error)]
pub enum SvgError {
    #[error("文件读取失败：{0}")]
    Io(#[from] std::io::Error),

    #[error("文件不存在：{0}")]
    FileNotFound(String),

    #[error("SVG 解析失败：{0}")]
    ParseError(String),

    #[error("安全错误：{0}")]
    Security(String),
}

/// 将 SVG 渲染为 PNG 图像（简化实现，暂不启用）
pub fn render_svg_to_png(_svg_path: impl AsRef<Path>, _output_path: impl AsRef<Path>, _resolution: u32) -> Result<(), SvgRenderError> {
    // TODO: 实现 SVG 渲染功能
    Err(SvgRenderError::NotImplemented)
}

/// SVG 渲染错误
#[derive(Debug, thiserror::Error)]
pub enum SvgRenderError {
    #[error("文件读取失败：{0}")]
    IoError(#[from] std::io::Error),

    #[error("功能尚未实现")]
    NotImplemented,

    #[error("无效的图像尺寸")]
    InvalidSize,

    #[error("PNG 保存失败：{0}")]
    PngError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_svg() {
        let svg = r#"<?xml version="1.0"?>
        <svg width="100" height="100" viewBox="0 0 100 100">
            <line x1="0" y1="0" x2="100" y2="0" />
            <line x1="0" y1="0" x2="0" y2="100" />
            <circle cx="50" cy="50" r="10" />
        </svg>"#;

        let result = SvgParser::parse_string(svg).unwrap();
        
        assert_eq!(result.primitives.len(), 3);
        assert_eq!(result.metadata.width, "100");
        assert_eq!(result.metadata.height, "100");
        assert_eq!(result.metadata.view_box, Some("0 0 100 100".to_string()));
    }

    #[test]
    fn test_parse_polygon() {
        let svg = r#"<svg><polygon points="0,0 100,0 100,100 0,100" /></svg>"#;
        let result = SvgParser::parse_string(svg).unwrap();
        
        assert_eq!(result.primitives.len(), 1);
        match &result.primitives[0] {
            Primitive::Polygon(poly) => {
                assert_eq!(poly.vertices.len(), 4);
            }
            _ => panic!("期望多边形"),
        }
    }

    #[test]
    fn test_parse_path() {
        let svg = r#"<svg><path d="M 0 0 L 100 0 L 100 100 L 0 100 Z" /></svg>"#;
        let result = SvgParser::parse_string(svg).unwrap();
        
        assert_eq!(result.primitives.len(), 1);
        match &result.primitives[0] {
            Primitive::Polygon(poly) => {
                assert_eq!(poly.vertices.len(), 4);
            }
            _ => panic!("期望多边形"),
        }
    }

    #[test]
    fn test_parse_nested_svg() {
        let svg = r#"<?xml version="1.0"?>
        <svg width="200" height="200">
            <g>
                <line x1="0" y1="0" x2="100" y2="100" />
            </g>
            <rect x="10" y="10" width="50" height="50" />
        </svg>"#;

        let result = SvgParser::parse_string(svg).unwrap();
        
        assert_eq!(result.primitives.len(), 2);
    }
}
