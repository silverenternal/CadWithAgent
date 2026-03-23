//! DXF 导出器
//!
//! 将几何图元导出为 DXF 文件格式

use crate::geometry::{Primitive, Point, Room};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// DXF 导出器
pub struct DxfExporter;

impl DxfExporter {
    /// 导出图元到 DXF 文件
    pub fn export(primitives: &[Primitive], output_path: impl AsRef<Path>) -> Result<DxfExportResult, DxfExportError> {
        let mut dxf = DxfDocument::new();

        // 添加图元
        for primitive in primitives {
            Self::add_primitive(&mut dxf, primitive);
        }

        // 写入文件
        dxf.write_to(output_path.as_ref())?;

        Ok(DxfExportResult {
            success: true,
            path: output_path.as_ref().to_string_lossy().to_string(),
            entity_count: primitives.len(),
        })
    }

    fn add_primitive(dxf: &mut DxfDocument, primitive: &Primitive) {
        match primitive {
            Primitive::Point(point) => {
                dxf.add_point(point.x, point.y, 0.0);
            }
            Primitive::Line(line) => {
                dxf.add_line(line.start.x, line.start.y, 0.0, line.end.x, line.end.y, 0.0);
            }
            Primitive::Polygon(poly) => {
                if poly.vertices.len() >= 2 {
                    for i in 0..poly.vertices.len() {
                        let j = (i + 1) % poly.vertices.len();
                        let p1 = &poly.vertices[i];
                        let p2 = &poly.vertices[j];
                        dxf.add_line(p1.x, p1.y, 0.0, p2.x, p2.y, 0.0);
                    }
                }
            }
            Primitive::Circle(circle) => {
                dxf.add_circle(circle.center.x, circle.center.y, 0.0, circle.radius);
            }
            Primitive::Rect(rect) => {
                let corners = [
                    rect.min,
                    Point::new(rect.max.x, rect.min.y),
                    rect.max,
                    Point::new(rect.min.x, rect.max.y),
                ];
                for i in 0..4 {
                    let j = (i + 1) % 4;
                    dxf.add_line(corners[i].x, corners[i].y, 0.0, corners[j].x, corners[j].y, 0.0);
                }
            }
            Primitive::Polyline { points, closed } => {
                if points.len() >= 2 {
                    for i in 0..points.len() - 1 {
                        let p1 = &points[i];
                        let p2 = &points[i + 1];
                        dxf.add_line(p1.x, p1.y, 0.0, p2.x, p2.y, 0.0);
                    }
                    if *closed {
                        let p1 = points.last().unwrap();
                        let p2 = &points[0];
                        dxf.add_line(p1.x, p1.y, 0.0, p2.x, p2.y, 0.0);
                    }
                }
            }
            Primitive::Arc { center, radius, start_angle, end_angle } => {
                dxf.add_arc(center.x, center.y, 0.0, *radius, *start_angle, *end_angle);
            }
            Primitive::Text { content, position, height } => {
                dxf.add_text(content, position.x, position.y, *height);
            }
        }
    }

    /// 从结构化 JSON 导出 DXF
    pub fn export_from_json(json_str: &str, output_path: impl AsRef<Path>) -> Result<DxfExportResult, DxfExportError> {
        let primitives: Vec<Primitive> = serde_json::from_str(json_str)?;
        Self::export(&primitives, output_path)
    }

    /// 导出房间数据
    pub fn export_rooms(rooms: &[Room], output_path: impl AsRef<Path>) -> Result<DxfExportResult, DxfExportError> {
        let mut dxf = DxfDocument::new();

        for room in rooms {
            // 添加房间边界
            for line in room.boundary.to_lines() {
                dxf.add_line(line.start.x, line.start.y, 0.0, line.end.x, line.end.y, 0.0);
            }

            // 添加门
            for door in &room.doors {
                dxf.add_text(&format!("DOOR {}", door.width), door.position.x, door.position.y, 100.0);
            }

            // 添加窗户
            for window in &room.windows {
                dxf.add_text(&format!("WINDOW {}", window.width), window.position.x, window.position.y, 100.0);
            }

            // 添加房间名称和面积
            dxf.add_text(&room.name, room.boundary.vertices[0].x, room.boundary.vertices[0].y, 150.0);
            dxf.add_text(&format!("Area: {:.2}", room.area), room.boundary.vertices[0].x, room.boundary.vertices[0].y - 200.0, 100.0);
        }

        dxf.write_to(output_path.as_ref())?;

        Ok(DxfExportResult {
            success: true,
            path: output_path.as_ref().to_string_lossy().to_string(),
            entity_count: rooms.len(),
        })
    }
}

/// DXF 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DxfExportResult {
    pub success: bool,
    pub path: String,
    pub entity_count: usize,
}

/// DXF 导出错误
#[derive(Debug, thiserror::Error)]
pub enum DxfExportError {
    #[error("文件写入失败：{0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON 解析失败：{0}")]
    JsonError(#[from] serde_json::Error),

    #[error("DXF 格式错误：{0}")]
    DxfError(String),
}

/// DXF 文档结构
struct DxfDocument {
    entities: Vec<DxfEntity>,
}

impl DxfDocument {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }

    fn add_point(&mut self, x: f64, y: f64, z: f64) {
        self.entities.push(DxfEntity::Point { x, y, z });
    }

    fn add_line(&mut self, x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) {
        self.entities.push(DxfEntity::Line {
            start: [x1, y1, z1],
            end: [x2, y2, z2],
        });
    }

    fn add_circle(&mut self, cx: f64, cy: f64, cz: f64, radius: f64) {
        self.entities.push(DxfEntity::Circle {
            center: [cx, cy, cz],
            radius,
        });
    }

    fn add_arc(&mut self, cx: f64, cy: f64, cz: f64, radius: f64, start_angle: f64, end_angle: f64) {
        self.entities.push(DxfEntity::Arc {
            center: [cx, cy, cz],
            radius,
            start_angle,
            end_angle,
        });
    }

    fn add_text(&mut self, content: &str, x: f64, y: f64, height: f64) {
        self.entities.push(DxfEntity::Text {
            content: content.to_string(),
            position: [x, y, 0.0],
            height,
        });
    }

    fn write_to(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let mut file = File::create(path)?;

        // 写入 DXF 文件头
        writeln!(file, "0")?;
        writeln!(file, "SECTION")?;
        writeln!(file, "2")?;
        writeln!(file, "HEADER")?;
        writeln!(file, "0")?;
        writeln!(file, "ENDSEC")?;

        // 写入表部分
        writeln!(file, "0")?;
        writeln!(file, "SECTION")?;
        writeln!(file, "2")?;
        writeln!(file, "TABLES")?;
        writeln!(file, "0")?;
        writeln!(file, "ENDSEC")?;

        // 写入实体部分
        writeln!(file, "0")?;
        writeln!(file, "SECTION")?;
        writeln!(file, "2")?;
        writeln!(file, "ENTITIES")?;

        for entity in &self.entities {
            self.write_entity(&mut file, entity)?;
        }

        writeln!(file, "0")?;
        writeln!(file, "ENDSEC")?;

        // 文件结束
        writeln!(file, "0")?;
        writeln!(file, "EOF")?;

        Ok(())
    }

    fn write_entity(&self, writer: &mut impl Write, entity: &DxfEntity) -> Result<(), std::io::Error> {
        match entity {
            DxfEntity::Point { x, y, z } => {
                writeln!(writer, "0")?;
                writeln!(writer, "POINT")?;
                writeln!(writer, "10")?;
                writeln!(writer, "{}", x)?;
                writeln!(writer, "20")?;
                writeln!(writer, "{}", y)?;
                writeln!(writer, "30")?;
                writeln!(writer, "{}", z)?;
            }
            DxfEntity::Line { start, end } => {
                writeln!(writer, "0")?;
                writeln!(writer, "LINE")?;
                writeln!(writer, "10")?;
                writeln!(writer, "{}", start[0])?;
                writeln!(writer, "20")?;
                writeln!(writer, "{}", start[1])?;
                writeln!(writer, "30")?;
                writeln!(writer, "{}", start[2])?;
                writeln!(writer, "11")?;
                writeln!(writer, "{}", end[0])?;
                writeln!(writer, "21")?;
                writeln!(writer, "{}", end[1])?;
                writeln!(writer, "31")?;
                writeln!(writer, "{}", end[2])?;
            }
            DxfEntity::Circle { center, radius } => {
                writeln!(writer, "0")?;
                writeln!(writer, "CIRCLE")?;
                writeln!(writer, "10")?;
                writeln!(writer, "{}", center[0])?;
                writeln!(writer, "20")?;
                writeln!(writer, "{}", center[1])?;
                writeln!(writer, "30")?;
                writeln!(writer, "{}", center[2])?;
                writeln!(writer, "40")?;
                writeln!(writer, "{}", radius)?;
            }
            DxfEntity::Arc { center, radius, start_angle, end_angle } => {
                writeln!(writer, "0")?;
                writeln!(writer, "ARC")?;
                writeln!(writer, "10")?;
                writeln!(writer, "{}", center[0])?;
                writeln!(writer, "20")?;
                writeln!(writer, "{}", center[1])?;
                writeln!(writer, "30")?;
                writeln!(writer, "{}", center[2])?;
                writeln!(writer, "40")?;
                writeln!(writer, "{}", radius)?;
                writeln!(writer, "50")?;
                writeln!(writer, "{}", start_angle)?;
                writeln!(writer, "51")?;
                writeln!(writer, "{}", end_angle)?;
            }
            DxfEntity::Text { content, position, height } => {
                writeln!(writer, "0")?;
                writeln!(writer, "TEXT")?;
                writeln!(writer, "1")?;
                writeln!(writer, "{}", content)?;
                writeln!(writer, "10")?;
                writeln!(writer, "{}", position[0])?;
                writeln!(writer, "20")?;
                writeln!(writer, "{}", position[1])?;
                writeln!(writer, "30")?;
                writeln!(writer, "{}", position[2])?;
                writeln!(writer, "40")?;
                writeln!(writer, "{}", height)?;
            }
        }
        Ok(())
    }
}

/// DXF 实体
enum DxfEntity {
    Point { x: f64, y: f64, z: f64 },
    Line { start: [f64; 3], end: [f64; 3] },
    Circle { center: [f64; 3], radius: f64 },
    Arc { center: [f64; 3], radius: f64, start_angle: f64, end_angle: f64 },
    Text { content: String, position: [f64; 3], height: f64 },
}
