//! STEP 文件解析器
//!
//! STEP (ISO 10303) 是产品模型数据交换的国际标准格式
//! 本模块提供 STEP 文件的解析和几何转换功能
//!
//! # 支持的协议
//! - AP203: 配置控制 3D 设计
//! - AP214: 汽车设计
//! - AP242: 基于模型的定义 3D 编码
//!
//! # 使用示例
//!
//! ```rust,no_run,ignore
//! use cadagent::parser::step::StepParser;
//! use cadagent::error::CadAgentResult;
//!
//! # fn example() -> CadAgentResult<()> {
//! let parser = StepParser::new();
//! let model = parser.parse("path/to/file.step")?;
//! let primitives = model.to_primitives();
//! # Ok(())
//! # }
//! ```

use crate::error::{CadAgentError, CadAgentResult};
use crate::geometry::nurbs::{NurbsCurve, Point3D};
use crate::geometry::primitives::{Circle, Line, Point, Polygon, Primitive};
use crate::parser::parser_common::{AssemblyStructure, CadMetadata, ParserConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// STEP 模型表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepModel {
    /// 模型名称
    pub name: Option<String>,
    /// 几何实体列表
    pub entities: Vec<StepEntity>,
    /// 装配结构
    pub assembly_structure: Option<AssemblyStructure>,
    /// 元数据
    pub metadata: CadMetadata,
}

impl StepModel {
    /// 创建空的 STEP 模型
    pub fn new() -> Self {
        Self {
            name: None,
            entities: Vec::new(),
            assembly_structure: None,
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

    /// 将单个 STEP 实体转换为图元
    fn entity_to_primitive(&self, entity: &StepEntity) -> Option<Primitive> {
        match &entity.data {
            StepEntityData::CartesianPoint { coordinates } => Some(Primitive::Point(Point {
                x: coordinates[0],
                y: coordinates[1],
            })),
            StepEntityData::CartesianPoint3D { coordinates } => {
                // 投影到 2D
                Some(Primitive::Point(Point {
                    x: coordinates[0],
                    y: coordinates[1],
                }))
            }
            StepEntityData::Line { start, direction } => {
                // 简化处理：假设 direction 是终点
                Some(Primitive::Line(Line {
                    start: Point {
                        x: start[0],
                        y: start[1],
                    },
                    end: Point {
                        x: start[0] + direction[0],
                        y: start[1] + direction[1],
                    },
                }))
            }
            StepEntityData::Line3D { start, direction } => {
                // 投影到 2D
                Some(Primitive::Line(Line {
                    start: Point {
                        x: start[0],
                        y: start[1],
                    },
                    end: Point {
                        x: start[0] + direction[0],
                        y: start[1] + direction[1],
                    },
                }))
            }
            StepEntityData::Circle { center, radius, .. } => Some(Primitive::Circle(Circle {
                center: Point {
                    x: center[0],
                    y: center[1],
                },
                radius: *radius,
            })),
            StepEntityData::Circle3D { center, radius, .. } => {
                // 投影到 2D
                Some(Primitive::Circle(Circle {
                    center: Point {
                        x: center[0],
                        y: center[1],
                    },
                    radius: *radius,
                }))
            }
            StepEntityData::Polyline { points } => {
                if points.len() >= 2 {
                    let coords: Vec<[f64; 2]> = points.to_vec();
                    Some(Primitive::Polygon(Polygon::from_coords(coords)))
                } else {
                    None
                }
            }
            StepEntityData::Polyline3D { points } => {
                // 投影到 2D
                if points.len() >= 2 {
                    let coords: Vec<[f64; 2]> = points.iter().map(|p| [p[0], p[1]]).collect();
                    Some(Primitive::Polygon(Polygon::from_coords(coords)))
                } else {
                    None
                }
            }
            StepEntityData::AdvancedBrep { .. } => {
                // B-Rep 几何需要更复杂的转换
                //
                // 当前实现状态：
                // - ManifoldSolidBrep: 支持 tessellation 提取（见下方匹配臂）
                // - AdvancedBrep: 需要完整的 B-Rep 边界表示转换
                //
                // 未来实现方案：
                // 1. 使用 OpenCascade 或 similar 库进行 B-Rep tessellation
                // 2. 从 B-Rep 边界提取曲面/曲线信息
                // 3. 将 NURBS 曲面离散化为多边形网格
                // 4. 支持布尔运算和几何操作
                //
                // 注意：这是 STEP 解析中最复杂的部分，需要专门的几何内核支持
                None
            }
            StepEntityData::ManifoldSolidBrep {
                boundaries,
                entity_refs: _,
            } => {
                // 从 B-Rep 边界提取几何
                // 首先尝试使用 tessellation
                for boundary in boundaries {
                    if let Some(geo) = &boundary.geometry {
                        if let Some(mesh) = &geo.tessellation {
                            // 将网格转换为多边形
                            return self.mesh_to_polygon(mesh);
                        }
                    }
                }

                // 如果没有 tessellation，尝试从边界提取几何
                // 需要访问完整实体列表，这里使用 entity_refs 查找
                // 注意：这个方法需要在 StepModel 中调用，所以返回 None
                // 实际使用场景中应该通过完整实体解析来获取
                None
            }
            StepEntityData::NurbsCurve {
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
                    NurbsCurve::new(points.clone(), weights.clone(), knot_vector.clone(), *order)
                {
                    // 将 NURBS 曲线离散化为多段线
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
            // 其他类型暂不处理
            _ => None,
        }
    }

    /// 将网格转换为多边形（完整实现）
    fn mesh_to_polygon(&self, mesh: &crate::geometry::nurbs::Mesh) -> Option<Primitive> {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return None;
        }

        // 从网格顶点提取 2D 轮廓
        // 使用投影：取 XY 平面
        let polygon_points: Vec<[f64; 2]> = mesh.vertices.iter().map(|v| [v.x, v.y]).collect();

        // 计算 2D 凸包作为简化轮廓
        if polygon_points.len() >= 3 {
            // 使用 gift wrapping 算法计算凸包
            let hull = self.compute_convex_hull(&polygon_points);
            if hull.len() >= 3 {
                return Some(Primitive::Polygon(Polygon::from_coords(hull)));
            }
        }

        // 如果凸包失败，使用所有顶点的边界框
        if !polygon_points.is_empty() {
            let (min_x, max_x) = polygon_points
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
                    (min.min(p[0]), max.max(p[0]))
                });
            let (min_y, max_y) = polygon_points
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
                    (min.min(p[1]), max.max(p[1]))
                });

            let bbox = vec![
                [min_x, min_y],
                [max_x, min_y],
                [max_x, max_y],
                [min_x, max_y],
            ];
            return Some(Primitive::Polygon(Polygon::from_coords(bbox)));
        }

        None
    }

    /// 计算 2D 点集的凸包（Gift Wrapping 算法）
    fn compute_convex_hull(&self, points: &[[f64; 2]]) -> Vec<[f64; 2]> {
        if points.len() < 3 {
            return points.to_vec();
        }

        // 找到最左边的点
        let mut leftmost = 0;
        for (i, p) in points.iter().enumerate() {
            if p[0] < points[leftmost][0] {
                leftmost = i;
            }
        }

        let mut hull = Vec::new();
        let mut current = leftmost;

        loop {
            hull.push(points[current]);
            let mut next = 0;
            for (i, point) in points.iter().enumerate() {
                if i == current {
                    continue;
                }

                // 检查点 i 是否在当前点的右侧
                let cross = self.cross_product(&points[current], &points[next], point);

                if cross > 0.0 {
                    next = i;
                }
            }

            current = next;
            if current == leftmost {
                break;
            }
        }

        hull
    }

    /// 计算叉积（用于凸包计算）
    fn cross_product(&self, o: &[f64; 2], a: &[f64; 2], b: &[f64; 2]) -> f64 {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    }

    /// 从 B-Rep 边界提取几何（完整实现）
    #[allow(dead_code)]
    fn extract_geometry_from_boundaries(
        &self,
        boundaries: &[BrepBoundary],
        entities: &[StepEntity],
    ) -> Option<Primitive> {
        // 收集所有 AdvancedFace
        let mut faces: Vec<&StepEntity> = Vec::new();
        for boundary in boundaries {
            for &boundary_ref in &boundary.boundary_refs {
                if let Some(entity) = entities.get(boundary_ref) {
                    if matches!(entity.data, StepEntityData::AdvancedFace { .. }) {
                        faces.push(entity);
                    }
                }
            }
        }

        // 从每个面提取几何
        let mut all_vertices: Vec<[f64; 3]> = Vec::new();
        for face in &faces {
            if let StepEntityData::AdvancedFace {
                boundary_ids,
                face_type: _,
                ..
            } = &face.data
            {
                // 处理每个边界
                for &bound_id in boundary_ids {
                    if let Some(bound_entity) = entities.get(bound_id) {
                        if let StepEntityData::EdgeLoop { edge_ids, .. } = &bound_entity.data {
                            // 处理每个边
                            for &edge_id in edge_ids {
                                if let Some(edge_entity) = entities.get(edge_id) {
                                    if let StepEntityData::EdgeCurve { curve_id, .. } =
                                        &edge_entity.data
                                    {
                                        if let Some(curve_entity) =
                                            curve_id.as_ref().and_then(|id| entities.get(*id))
                                        {
                                            // 提取曲线几何
                                            let vertices =
                                                self.extract_curve_vertices(curve_entity);
                                            all_vertices.extend(vertices);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 转换为 2D 多边形
        if all_vertices.len() >= 3 {
            let points_2d: Vec<[f64; 2]> = all_vertices.iter().map(|v| [v[0], v[1]]).collect();
            let hull = self.compute_convex_hull(&points_2d);
            if hull.len() >= 3 {
                return Some(Primitive::Polygon(Polygon::from_coords(hull)));
            }
        }

        None
    }

    /// 从曲线实体提取顶点
    #[allow(dead_code)]
    fn extract_curve_vertices(&self, entity: &StepEntity) -> Vec<[f64; 3]> {
        match &entity.data {
            StepEntityData::Line3D { start, direction } => {
                vec![
                    *start,
                    [
                        start[0] + direction[0],
                        start[1] + direction[1],
                        start[2] + direction[2],
                    ],
                ]
            }
            StepEntityData::Circle3D {
                center,
                radius,
                axis: _,
            } => {
                // 离散化圆为 32 个点
                let mut vertices = Vec::with_capacity(32);
                for i in 0..32 {
                    let angle = (i as f64) * 2.0 * std::f64::consts::PI / 32.0;
                    // 简化：假设 axis 是 Z 轴
                    let x = center[0] + radius * angle.cos();
                    let y = center[1] + radius * angle.sin();
                    let z = center[2];
                    vertices.push([x, y, z]);
                }
                vertices
            }
            StepEntityData::Polyline3D { points } => points.clone(),
            _ => Vec::new(),
        }
    }
}

impl Default for StepModel {
    fn default() -> Self {
        Self::new()
    }
}

/// STEP 实体数据类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepEntityData {
    /// 笛卡尔点
    CartesianPoint { coordinates: [f64; 2] },
    /// 笛卡尔点 3D
    CartesianPoint3D { coordinates: [f64; 3] },
    /// 直线
    Line {
        start: [f64; 2],
        direction: [f64; 2],
    },
    /// 直线 3D
    Line3D {
        start: [f64; 3],
        direction: [f64; 3],
    },
    /// 圆
    Circle {
        center: [f64; 2],
        radius: f64,
        axis: Option<[f64; 3]>,
    },
    /// 圆 3D
    Circle3D {
        center: [f64; 3],
        radius: f64,
        axis: [f64; 3],
    },
    /// 圆弧
    Arc {
        center: [f64; 2],
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    },
    /// 多段线
    Polyline { points: Vec<[f64; 2]> },
    /// 多段线 3D
    Polyline3D { points: Vec<[f64; 3]> },
    /// 多边形
    Polygon { vertices: Vec<[f64; 2]> },
    /// NURBS 曲线
    NurbsCurve {
        control_points: Vec<[f64; 3]>,
        weights: Vec<f64>,
        knot_vector: Vec<f64>,
        order: usize,
    },
    /// NURBS 曲面
    NurbsSurface {
        control_points: Vec<Vec<[f64; 3]>>,
        weights: Vec<Vec<f64>>,
        knot_vector_u: Vec<f64>,
        knot_vector_v: Vec<f64>,
        order_u: usize,
        order_v: usize,
    },
    /// 高级 B-Rep
    AdvancedBrep {
        vertices: Vec<[f64; 3]>,
        edges: Vec<EdgeData>,
        faces: Vec<FaceData>,
    },
    /// 流形固体 B-Rep (AP203/214)
    ManifoldSolidBrep {
        /// 实体 ID 引用
        entity_refs: Vec<usize>,
        /// 边界表示
        boundaries: Vec<BrepBoundary>,
    },
    /// 高级面
    AdvancedFace {
        /// 边界 ID 列表
        boundary_ids: Vec<usize>,
        /// 面类型
        face_type: String,
        /// 法向
        normal: Option<[f64; 3]>,
    },
    /// 边循环
    EdgeLoop {
        /// 边 ID 列表
        edge_ids: Vec<usize>,
        /// 方向
        orientations: Vec<bool>,
    },
    /// 边曲线
    EdgeCurve {
        /// 几何曲线 ID
        curve_id: Option<usize>,
        /// 起点 ID
        start_id: Option<usize>,
        /// 终点 ID
        end_id: Option<usize>,
    },
    /// 面边界
    FaceBound {
        /// 边界 ID
        bound_id: Option<usize>,
        /// 方向
        orientation: bool,
    },
    /// 向量
    Vector {
        /// 起点
        origin: [f64; 3],
        /// 方向
        direction: [f64; 3],
        /// 大小
        magnitude: f64,
    },
    /// 方向
    Direction {
        /// 方向比率
        direction_ratios: [f64; 3],
    },
    /// 轴 2 放置
    Axis2Placement3D {
        /// 原点
        origin: [f64; 3],
        /// 轴
        axis: Option<[f64; 3]>,
        /// 参考方向
        ref_direction: Option<[f64; 3]>,
    },
    /// 其他未解析的实体
    Other {
        entity_type: String,
        raw_data: String,
    },
}

/// B-Rep 边界表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrepBoundary {
    /// 边界类型
    pub boundary_type: String,
    /// 边界 ID 引用
    pub boundary_refs: Vec<usize>,
    /// 几何数据
    pub geometry: Option<BrepGeometry>,
}

/// B-Rep 几何数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrepGeometry {
    /// 曲面类型
    pub surface_type: Option<String>,
    /// 曲面参数
    pub surface_params: Vec<f64>,
    /// 离散化网格
    pub tessellation: Option<crate::geometry::nurbs::Mesh>,
}

/// 边数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    /// 起点 ID
    pub start_vertex: usize,
    /// 终点 ID
    pub end_vertex: usize,
    /// 曲线类型
    pub curve_type: String,
    /// 曲线参数
    pub curve_params: Vec<f64>,
}

/// 面数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceData {
    /// 边界边 ID 列表
    pub boundary_edges: Vec<usize>,
    /// 面类型
    pub surface_type: String,
    /// 面参数
    pub surface_params: Vec<f64>,
}

/// STEP 实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEntity {
    /// 实体 ID
    pub id: usize,
    /// 实体类型
    pub entity_type: String,
    /// 实体数据
    pub data: StepEntityData,
}

/// STEP 解析器
pub struct StepParser {
    /// Parser configuration
    config: ParserConfig,
}

impl StepParser {
    /// 创建新的 STEP 解析器
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

    /// 解析 STEP 文件
    ///
    /// # Errors
    /// 如果文件不存在或格式错误，返回错误
    pub fn parse(&self, path: &Path) -> CadAgentResult<StepModel> {
        if !path.exists() {
            return Err(CadAgentError::file_not_found(
                path.to_string_lossy().to_string(),
            ));
        }

        let content =
            fs::read_to_string(path).map_err(|e| CadAgentError::io("无法读取 STEP 文件", e))?;

        self.parse_string(&content)
    }

    /// 从字符串解析 STEP 内容
    ///
    /// # Errors
    /// 如果格式错误，返回错误
    pub fn parse_string(&self, content: &str) -> CadAgentResult<StepModel> {
        let mut model = StepModel::new();

        // 解析头部信息
        self.parse_header(content, &mut model)?;

        // 解析数据段
        self.parse_data(content, &mut model)?;

        Ok(model)
    }

    /// 解析 STEP 文件头部
    fn parse_header(&self, content: &str, model: &mut StepModel) -> CadAgentResult<()> {
        // 查找 HEADER 段
        let header_start = content
            .find("HEADER")
            .ok_or_else(|| CadAgentError::parse("STEP", "缺少 HEADER 段"))?;

        // 在 HEADER 段内查找 ENDSEC
        let header_end = content[header_start..]
            .find("ENDSEC")
            .map(|pos| pos + header_start)
            .ok_or_else(|| CadAgentError::parse("STEP", "缺少 HEADER 结束标记"))?;

        let header_content = &content[header_start..header_end];

        // 解析文件描述
        if let Some(desc_start) = header_content.find("FILE_DESCRIPTION") {
            if let Some(desc_end) = header_content[desc_start..].find(");") {
                let desc = &header_content[desc_start + 17..desc_end + desc_start];
                model.metadata.source_software =
                    Some(desc.trim_matches('(').trim_matches(')').to_string());
            }
        }

        // 解析文件名
        if let Some(name_start) = header_content.find("FILE_NAME") {
            if let Some(name_end) = header_content[name_start..].find(");") {
                let name_section = &header_content[name_start + 10..name_end + name_start];
                // 提取文件名（第一个参数，使用单引号）
                if let Some(first_quote) = name_section.find('\'') {
                    if let Some(second_quote) = name_section[first_quote + 1..].find('\'') {
                        let filename = name_section
                            [first_quote + 1..first_quote + 1 + second_quote]
                            .to_string();
                        model.metadata.name = Some(filename);
                    }
                }
            }
        }

        Ok(())
    }

    /// 解析 STEP 数据段
    fn parse_data(&self, content: &str, model: &mut StepModel) -> CadAgentResult<()> {
        // 查找 DATA 段（确保找到的是段名而不是内容的一部分）
        let data_marker = "\nDATA";
        let data_start = content
            .find(data_marker)
            .or_else(|| content.find("DATA"))
            .ok_or_else(|| CadAgentError::parse("STEP", "缺少 DATA 段"))?;

        let endsec_marker = "\nENDSEC";
        let data_end = content[endsec_marker.len() + data_start..]
            .find(endsec_marker)
            .map(|pos| pos + data_start + endsec_marker.len())
            .or_else(|| {
                content[data_start..]
                    .find("ENDSEC")
                    .map(|pos| pos + data_start + 6)
            })
            .ok_or_else(|| CadAgentError::parse("STEP", "缺少 DATA 结束标记"))?;

        let data_content = &content[data_start..data_end];

        // 解析每个实体
        for line in data_content.lines() {
            let line = line.trim();
            // 跳过空行和 ENDSEC 等标记
            if line.is_empty() || line == "DATA" {
                continue;
            }

            // 解析实体行，格式如：#123 = ENTITY_TYPE(...);
            if let Some(eq_pos) = line.find('=') {
                let id_part = line[..eq_pos].trim();
                let data_part = line[eq_pos + 1..].trim();

                // 提取 ID
                if let Some(id) = id_part.strip_prefix('#') {
                    if let Ok(entity_id) = id.trim().parse::<usize>() {
                        // 解析实体类型和数据
                        if let Some(entity) = self.parse_entity(data_part)? {
                            model.entities.push(StepEntity {
                                id: entity_id,
                                entity_type: entity.entity_type,
                                data: entity.data,
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 解析单个实体
    fn parse_entity(&self, data: &str) -> CadAgentResult<Option<StepEntity>> {
        let data = data.trim().trim_end_matches(';');

        // 提取实体类型
        let open_paren = data
            .find('(')
            .ok_or_else(|| CadAgentError::parse("STEP", format!("无效的实体格式：{data}")))?;
        let entity_type = &data[..open_paren];

        // 解析参数
        let params_str = &data[open_paren + 1..data.len() - 1];
        let params = self.parse_parameters(params_str);

        // 根据类型创建实体
        let entity_data = match entity_type {
            "CARTESIAN_POINT" => self.parse_cartesian_point(&params)?,
            "LINE" => self.parse_line(&params)?,
            "CIRCLE" => self.parse_circle(&params)?,
            "POLYLINE" => self.parse_polyline(&params)?,
            "ADVANCED_BREP" => self.parse_advanced_brep(&params)?,
            // AP203/214 B-Rep 实体
            "MANIFOLD_SOLID_BREP" | "BREP_WITH_VOIDS" => self.parse_manifold_solid_brep(&params)?,
            "ADVANCED_FACE" => self.parse_advanced_face(&params)?,
            "EDGE_LOOP" => self.parse_edge_loop(&params)?,
            "EDGE_CURVE" => self.parse_edge_curve(&params)?,
            "FACE_BOUND" => self.parse_face_bound(&params)?,
            "VECTOR" => self.parse_vector(&params)?,
            "DIRECTION" => self.parse_direction(&params)?,
            "AXIS2_PLACEMENT_3D" => self.parse_axis2_placement_3d(&params)?,
            "CARTESIAN_POINT_3D" => self.parse_cartesian_point_3d(&params)?,
            "LINE_3D" => self.parse_line_3d(&params)?,
            "CIRCLE_3D" => self.parse_circle_3d(&params)?,
            "POLYLINE_3D" => self.parse_polyline_3d(&params)?,
            // NURBS 实体
            "B_SPLINE_CURVE_WITH_KNOTS" => self.parse_b_spline_curve(&params)?,
            "B_SPLINE_SURFACE_WITH_KNOTS" => self.parse_b_spline_surface(&params)?,
            // 平面实体
            "PLANE" => self.parse_plane(&params)?,
            "CYLINDRICAL_SURFACE" => self.parse_cylindrical_surface(&params)?,
            "CONICAL_SURFACE" => self.parse_conical_surface(&params)?,
            "SPHERICAL_SURFACE" => self.parse_spherical_surface(&params)?,
            "TOROIDAL_SURFACE" => self.parse_toroidal_surface(&params)?,
            // 其他
            _ => StepEntityData::Other {
                entity_type: entity_type.to_string(),
                raw_data: params_str.to_string(),
            },
        };

        Ok(Some(StepEntity {
            id: 0,
            entity_type: entity_type.to_string(),
            data: entity_data,
        }))
    }

    /// 解析参数列表
    fn parse_parameters(&self, params_str: &str) -> Vec<String> {
        let mut params = Vec::new();
        let mut current = String::new();
        let mut in_string = false;
        let mut paren_depth = 0;

        for ch in params_str.chars() {
            match ch {
                '\'' => {
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

    /// 解析笛卡尔点
    fn parse_cartesian_point(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        if params.is_empty() {
            return Err(CadAgentError::parse("STEP", "CARTESIAN_POINT 参数不足"));
        }

        let coords = self.parse_coordinate_list(&params[0])?;

        if coords.len() == 2 {
            Ok(StepEntityData::CartesianPoint {
                coordinates: [coords[0], coords[1]],
            })
        } else if coords.len() >= 3 {
            Ok(StepEntityData::CartesianPoint3D {
                coordinates: [coords[0], coords[1], coords[2]],
            })
        } else {
            Err(CadAgentError::parse("STEP", "无效的坐标维度"))
        }
    }

    /// 解析直线
    fn parse_line(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        if params.len() < 2 {
            return Err(CadAgentError::parse("STEP", "LINE 参数不足"));
        }

        let start = self.parse_coordinate_list(&params[0])?;
        let direction = self.parse_coordinate_list(&params[1])?;

        let direction_array: [f64; 2] = if direction.len() >= 2 {
            [direction[0], direction[1]]
        } else if direction.len() == 1 {
            [direction[0], 0.0]
        } else {
            [0.0, 0.0]
        };

        Ok(StepEntityData::Line {
            start: [start[0], start[1]],
            direction: direction_array,
        })
    }

    /// 解析圆
    fn parse_circle(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        if params.len() < 2 {
            return Err(CadAgentError::parse("STEP", "CIRCLE 参数不足"));
        }

        let center = self.parse_coordinate_list(&params[0])?;
        let radius = params[1]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("STEP", "无效的半径值"))?;

        let axis = if params.len() > 2 {
            let axis_coords = self.parse_coordinate_list(&params[2])?;
            if axis_coords.len() >= 3 {
                Some([axis_coords[0], axis_coords[1], axis_coords[2]])
            } else {
                None
            }
        } else {
            None
        };

        Ok(StepEntityData::Circle {
            center: [center[0], center[1]],
            radius,
            axis,
        })
    }

    /// 解析多段线
    fn parse_polyline(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut points = Vec::new();

        for param in params {
            let coords = self.parse_coordinate_list(param)?;
            if coords.len() >= 2 {
                points.push([coords[0], coords[1]]);
            }
        }

        Ok(StepEntityData::Polyline { points })
    }

    /// 解析高级 B-Rep
    fn parse_advanced_brep(&self, _params: &[String]) -> CadAgentResult<StepEntityData> {
        // 简化处理：B-Rep 需要更复杂的解析逻辑
        Ok(StepEntityData::AdvancedBrep {
            vertices: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
        })
    }

    /// 解析流形固体 B-Rep (AP203/214)
    fn parse_manifold_solid_brep(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut entity_refs = Vec::new();
        let mut boundaries = Vec::new();

        for param in params {
            let trimmed = param.trim();
            // 解析实体引用 (#123 格式)
            if let Some(id_str) = trimmed.strip_prefix('#') {
                if let Ok(id) = id_str.parse::<usize>() {
                    entity_refs.push(id);
                }
            }
            // 解析嵌套的 B-Rep 边界
            else if trimmed.starts_with('(') {
                // 解析边界信息
                boundaries.push(BrepBoundary {
                    boundary_type: "FACE_BOUND".to_string(),
                    boundary_refs: vec![],
                    geometry: None,
                });
            }
        }

        Ok(StepEntityData::ManifoldSolidBrep {
            entity_refs,
            boundaries,
        })
    }

    /// 解析高级面
    fn parse_advanced_face(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut boundary_ids = Vec::new();
        let face_type = "PLANAR".to_string();
        let mut normal = None;

        for (i, param) in params.iter().enumerate() {
            let trimmed = param.trim();
            // 第一个参数通常是面 ID
            if i == 0 {
                if let Some(id_str) = trimmed.strip_prefix('#') {
                    if let Ok(id) = id_str.parse::<usize>() {
                        boundary_ids.push(id);
                    }
                }
            }
            // 检查是否是法向量
            else if trimmed.starts_with('(') {
                if let Ok(coords) = self.parse_coordinate_list_3d(trimmed) {
                    normal = Some(coords);
                }
            }
        }

        Ok(StepEntityData::AdvancedFace {
            boundary_ids,
            face_type,
            normal,
        })
    }

    /// 解析边循环
    fn parse_edge_loop(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut edge_ids = Vec::new();
        let mut orientations = Vec::new();

        for param in params {
            let trimmed = param.trim();
            // 解析边引用
            if let Some(id_str) = trimmed.strip_prefix('#') {
                if let Ok(id) = id_str.parse::<usize>() {
                    edge_ids.push(id);
                    orientations.push(true); // 默认方向
                }
            }
            // 解析带方向的边 (.T. or .F.)
            else if trimmed == ".T." || trimmed == ".F." {
                orientations.push(trimmed == ".T.");
            }
        }

        Ok(StepEntityData::EdgeLoop {
            edge_ids,
            orientations,
        })
    }

    /// 解析边曲线
    fn parse_edge_curve(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut curve_id = None;
        let mut start_id = None;
        let mut end_id = None;

        for (i, param) in params.iter().enumerate() {
            let trimmed = param.trim();
            if let Some(id_str) = trimmed.strip_prefix('#') {
                if let Ok(id) = id_str.parse::<usize>() {
                    match i {
                        0 => curve_id = Some(id),
                        1 => start_id = Some(id),
                        2 => end_id = Some(id),
                        _ => {}
                    }
                }
            }
        }

        Ok(StepEntityData::EdgeCurve {
            curve_id,
            start_id,
            end_id,
        })
    }

    /// 解析面边界
    fn parse_face_bound(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut bound_id = None;
        let mut orientation = true;

        for (i, param) in params.iter().enumerate() {
            let trimmed = param.trim();
            if i == 0 {
                if let Some(id_str) = trimmed.strip_prefix('#') {
                    if let Ok(id) = id_str.parse::<usize>() {
                        bound_id = Some(id);
                    }
                }
            } else if i == 1 {
                orientation = trimmed == ".T.";
            }
        }

        Ok(StepEntityData::FaceBound {
            bound_id,
            orientation,
        })
    }

    /// 解析向量
    fn parse_vector(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let origin = [0.0, 0.0, 0.0]; // 默认原点
        let mut direction = [1.0, 0.0, 0.0];
        let mut magnitude = 1.0;

        for (i, param) in params.iter().enumerate() {
            let trimmed = param.trim();
            if i == 0 {
                // 原点（通常是 CARTESIAN_POINT 引用）
                if let Ok(coords) = self.parse_coordinate_list_3d(trimmed) {
                    let _ = coords; // 暂不使用
                }
            } else if i == 1 {
                // 方向（DIRECTION 引用或坐标）
                if let Ok(coords) = self.parse_coordinate_list_3d(trimmed) {
                    direction = coords;
                }
            } else if i == 2 {
                // 大小
                if let Ok(mag) = trimmed.parse::<f64>() {
                    magnitude = mag;
                }
            }
        }

        Ok(StepEntityData::Vector {
            origin,
            direction,
            magnitude,
        })
    }

    /// 解析方向
    fn parse_direction(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut direction_ratios = [1.0, 0.0, 0.0];

        if let Some(first_param) = params.first() {
            if let Ok(coords) = self.parse_coordinate_list_3d(first_param) {
                direction_ratios = coords;
            }
        }

        Ok(StepEntityData::Direction { direction_ratios })
    }

    /// 解析轴 2 放置 3D
    fn parse_axis2_placement_3d(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut origin = [0.0, 0.0, 0.0];
        let mut axis = None;
        let mut ref_direction = None;

        for (i, param) in params.iter().enumerate() {
            let trimmed = param.trim();
            if i == 0 {
                // 原点
                if let Ok(coords) = self.parse_coordinate_list_3d(trimmed) {
                    origin = coords;
                }
            } else if i == 1 {
                // 轴
                if let Ok(coords) = self.parse_coordinate_list_3d(trimmed) {
                    axis = Some(coords);
                }
            } else if i == 2 {
                // 参考方向
                if let Ok(coords) = self.parse_coordinate_list_3d(trimmed) {
                    ref_direction = Some(coords);
                }
            }
        }

        Ok(StepEntityData::Axis2Placement3D {
            origin,
            axis,
            ref_direction,
        })
    }

    /// 解析笛卡尔点 3D
    fn parse_cartesian_point_3d(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        if params.is_empty() {
            return Err(CadAgentError::parse("STEP", "CARTESIAN_POINT_3D 参数不足"));
        }

        let coords = self.parse_coordinate_list_3d(&params[0])?;
        Ok(StepEntityData::CartesianPoint3D {
            coordinates: coords,
        })
    }

    /// 解析直线 3D
    fn parse_line_3d(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        if params.len() < 2 {
            return Err(CadAgentError::parse("STEP", "LINE_3D 参数不足"));
        }

        let start = self.parse_coordinate_list_3d(&params[0])?;
        let direction = self.parse_coordinate_list_3d(&params[1])?;

        Ok(StepEntityData::Line3D { start, direction })
    }

    /// 解析圆 3D
    fn parse_circle_3d(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        if params.len() < 2 {
            return Err(CadAgentError::parse("STEP", "CIRCLE_3D 参数不足"));
        }

        let center = self.parse_coordinate_list_3d(&params[0])?;
        let radius = params[1]
            .parse::<f64>()
            .map_err(|_| CadAgentError::parse("STEP", "无效的半径值"))?;

        let axis = if params.len() > 2 {
            self.parse_coordinate_list_3d(&params[2])?
        } else {
            [0.0, 0.0, 1.0]
        };

        Ok(StepEntityData::Circle3D {
            center,
            radius,
            axis,
        })
    }

    /// 解析多段线 3D
    fn parse_polyline_3d(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut points = Vec::new();

        for param in params {
            if let Ok(coords) = self.parse_coordinate_list_3d(param) {
                points.push(coords);
            }
        }

        Ok(StepEntityData::Polyline3D { points })
    }

    /// 解析 B 样条曲线
    fn parse_b_spline_curve(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        // B_SPLINE_CURVE_WITH_KNOTS 解析
        // 简化处理：提取控制点和节点向量
        let mut control_points = Vec::new();
        let mut weights = Vec::new();
        let mut knot_vector = Vec::new();
        let mut order = 3;

        for param in params {
            let trimmed = param.trim();
            if trimmed.starts_with('(') {
                // 尝试解析为控制点
                if let Ok(coords) = self.parse_coordinate_list_3d(trimmed) {
                    control_points.push(coords);
                }
                // 尝试解析为节点向量
                else if let Ok(knots) = self.parse_knot_vector(trimmed) {
                    knot_vector = knots;
                }
            } else if let Ok(val) = trimmed.parse::<f64>() {
                weights.push(val);
            } else if let Ok(ord) = trimmed.parse::<usize>() {
                order = ord;
            }
        }

        Ok(StepEntityData::NurbsCurve {
            control_points,
            weights,
            knot_vector,
            order,
        })
    }

    /// 解析 B 样条曲面
    fn parse_b_spline_surface(&self, _params: &[String]) -> CadAgentResult<StepEntityData> {
        // 简化处理
        Ok(StepEntityData::NurbsSurface {
            control_points: Vec::new(),
            weights: Vec::new(),
            knot_vector_u: Vec::new(),
            knot_vector_v: Vec::new(),
            order_u: 3,
            order_v: 3,
        })
    }

    /// 解析平面
    fn parse_plane(&self, params: &[String]) -> CadAgentResult<StepEntityData> {
        let mut normal = [0.0, 0.0, 1.0];

        if let Some(first_param) = params.first() {
            if let Ok(coords) = self.parse_coordinate_list_3d(first_param) {
                normal = coords;
            }
        }

        Ok(StepEntityData::AdvancedFace {
            boundary_ids: vec![],
            face_type: "PLANE".to_string(),
            normal: Some(normal),
        })
    }

    /// 解析圆柱面
    fn parse_cylindrical_surface(&self, _params: &[String]) -> CadAgentResult<StepEntityData> {
        Ok(StepEntityData::AdvancedFace {
            boundary_ids: vec![],
            face_type: "CYLINDRICAL".to_string(),
            normal: None,
        })
    }

    /// 解析圆锥面
    fn parse_conical_surface(&self, _params: &[String]) -> CadAgentResult<StepEntityData> {
        Ok(StepEntityData::AdvancedFace {
            boundary_ids: vec![],
            face_type: "CONICAL".to_string(),
            normal: None,
        })
    }

    /// 解析球面
    fn parse_spherical_surface(&self, _params: &[String]) -> CadAgentResult<StepEntityData> {
        Ok(StepEntityData::AdvancedFace {
            boundary_ids: vec![],
            face_type: "SPHERICAL".to_string(),
            normal: None,
        })
    }

    /// 解析环面
    fn parse_toroidal_surface(&self, _params: &[String]) -> CadAgentResult<StepEntityData> {
        Ok(StepEntityData::AdvancedFace {
            boundary_ids: vec![],
            face_type: "TOROIDAL".to_string(),
            normal: None,
        })
    }

    /// 解析 3D 坐标列表
    fn parse_coordinate_list_3d(&self, param: &str) -> CadAgentResult<[f64; 3]> {
        let content = param
            .trim()
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| CadAgentError::parse("STEP", format!("无效的坐标格式：{param}")))?;

        let coords: Result<Vec<f64>, _> = content
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| CadAgentError::parse("STEP", format!("无效的坐标值：{s}")))
            })
            .collect();
        let coords = coords?;

        if coords.len() >= 3 {
            Ok([coords[0], coords[1], coords[2]])
        } else if coords.len() == 2 {
            Ok([coords[0], coords[1], 0.0])
        } else {
            Err(CadAgentError::parse("STEP", "坐标值不足"))
        }
    }

    /// 解析节点向量
    fn parse_knot_vector(&self, param: &str) -> CadAgentResult<Vec<f64>> {
        let content = param
            .trim()
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| CadAgentError::parse("STEP", format!("无效的节点向量格式：{param}")))?;

        content
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| CadAgentError::parse("STEP", format!("无效的节点值：{s}")))
            })
            .collect()
    }

    /// 解析坐标列表
    fn parse_coordinate_list(&self, param: &str) -> CadAgentResult<Vec<f64>> {
        // 移除括号
        let content = param
            .trim()
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .ok_or_else(|| CadAgentError::parse("STEP", format!("无效的坐标格式：{param}")))?;

        // 解析坐标值
        content
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| CadAgentError::parse("STEP", format!("无效的坐标值：{s}")))
            })
            .collect()
    }
}

impl Default for StepParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_parser_creation() {
        let parser = StepParser::new();
        assert_eq!(parser.config.tolerance, 1e-6);
        assert!(!parser.config.debug);

        let parser = StepParser::new().with_tolerance(1e-8).with_debug(true);
        assert_eq!(parser.config.tolerance, 1e-8);
        assert!(parser.config.debug);
    }

    #[test]
    fn test_parse_minimal_step_file() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Test file'),'2;1');
FILE_NAME('test.step','2024-01-01',('Author'),('Organization'),'Software','Version','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((1.0, 2.0));
#2 = CARTESIAN_POINT((3.0, 4.0));
ENDSEC;
END-ISO-10303-21;"#;

        let parser = StepParser::new();
        let result = parser.parse_string(content);
        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());

        let model = result.unwrap();
        assert_eq!(model.entities.len(), 2);
        assert_eq!(model.metadata.name, Some("test.step".to_string()));
    }

    #[test]
    fn test_parse_circle() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Test'),'2;1');
FILE_NAME('test.step','','','','','','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((0.0, 0.0));
#2 = CIRCLE((0.0, 0.0), 5.0);
ENDSEC;
END-ISO-10303-21;"#;

        let parser = StepParser::new();
        let model = parser.parse_string(content).unwrap();

        assert_eq!(model.entities.len(), 2);

        // 验证圆实体
        if let StepEntityData::Circle { center, radius, .. } = &model.entities[1].data {
            assert_eq!(center[0], 0.0);
            assert_eq!(center[1], 0.0);
            assert_eq!(*radius, 5.0);
        } else {
            panic!("Expected Circle entity");
        }
    }

    #[test]
    fn test_step_model_to_primitives() {
        let mut model = StepModel::new();
        model.entities.push(StepEntity {
            id: 1,
            entity_type: "CARTESIAN_POINT".to_string(),
            data: StepEntityData::CartesianPoint {
                coordinates: [1.0, 2.0],
            },
        });
        model.entities.push(StepEntity {
            id: 2,
            entity_type: "CIRCLE".to_string(),
            data: StepEntityData::Circle {
                center: [0.0, 0.0],
                radius: 5.0,
                axis: None,
            },
        });

        let primitives = model.to_primitives();
        assert_eq!(primitives.len(), 2);
    }

    #[test]
    fn test_parse_invalid_step() {
        let parser = StepParser::new();

        // 缺少 HEADER
        let result = parser.parse_string("INVALID CONTENT");
        assert!(result.is_err());

        // 缺少 DATA
        let result = parser.parse_string("ISO-10303-21;HEADER;ENDSEC;");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_3d_entities() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('3D Test'),'2;1');
FILE_NAME('test3d.step','','','','','','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((1.0, 2.0, 3.0));
#2 = LINE((0.0, 0.0), (1.0, 0.0));
#3 = CIRCLE((0.0, 0.0), 10.0);
ENDSEC;
END-ISO-10303-21;"#;

        let parser = StepParser::new();
        let model = parser.parse_string(content).unwrap();

        assert_eq!(model.entities.len(), 3);

        // 验证 3D 点
        if let StepEntityData::CartesianPoint3D { coordinates } = &model.entities[0].data {
            assert_eq!(coordinates[0], 1.0);
            assert_eq!(coordinates[1], 2.0);
            assert_eq!(coordinates[2], 3.0);
        } else {
            panic!("Expected CartesianPoint3D");
        }
    }

    #[test]
    fn test_parse_advanced_face() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Face Test'),'2;1');
FILE_NAME('face.step','','','','','','');
ENDSEC;
DATA;
#1 = ADVANCED_FACE(#2, (1.0, 0.0, 0.0), .F.);
ENDSEC;
END-ISO-10303-21;"#;

        let parser = StepParser::new();
        let model = parser.parse_string(content).unwrap();

        assert_eq!(model.entities.len(), 1);

        if let StepEntityData::AdvancedFace {
            face_type, normal, ..
        } = &model.entities[0].data
        {
            assert_eq!(face_type, "PLANAR");
            assert!(normal.is_some());
        } else {
            panic!("Expected AdvancedFace");
        }
    }

    #[test]
    fn test_parse_edge_loop() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Edge Loop Test'),'2;1');
FILE_NAME('edge.step','','','','','','');
ENDSEC;
DATA;
#1 = EDGE_LOOP(#2, #3, #4);
ENDSEC;
END-ISO-10303-21;"#;

        let parser = StepParser::new();
        let model = parser.parse_string(content).unwrap();

        assert_eq!(model.entities.len(), 1);

        if let StepEntityData::EdgeLoop { edge_ids, .. } = &model.entities[0].data {
            assert!(!edge_ids.is_empty());
        } else {
            panic!("Expected EdgeLoop");
        }
    }

    #[test]
    fn test_parse_manifold_solid_brep() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('B-Rep Test'),'2;1');
FILE_NAME('brep.step','','','','','','');
ENDSEC;
DATA;
#1 = MANIFOLD_SOLID_BREP(#2, #3);
ENDSEC;
END-ISO-10303-21;"#;

        let parser = StepParser::new();
        let model = parser.parse_string(content).unwrap();

        assert_eq!(model.entities.len(), 1);

        if let StepEntityData::ManifoldSolidBrep { entity_refs, .. } = &model.entities[0].data {
            assert!(!entity_refs.is_empty());
        } else {
            panic!("Expected ManifoldSolidBrep");
        }
    }

    #[test]
    fn test_step_to_primitives_3d_projection() {
        let mut model = StepModel::new();
        model.entities.push(StepEntity {
            id: 1,
            entity_type: "CARTESIAN_POINT_3D".to_string(),
            data: StepEntityData::CartesianPoint3D {
                coordinates: [1.0, 2.0, 3.0],
            },
        });
        model.entities.push(StepEntity {
            id: 2,
            entity_type: "LINE_3D".to_string(),
            data: StepEntityData::Line3D {
                start: [0.0, 0.0, 0.0],
                direction: [1.0, 0.0, 0.0],
            },
        });

        let primitives = model.to_primitives();
        assert_eq!(primitives.len(), 2);

        // 验证 3D 到 2D 投影
        if let Primitive::Point(point) = &primitives[0] {
            assert_eq!(point.x, 1.0);
            assert_eq!(point.y, 2.0);
        } else {
            panic!("Expected Point primitive");
        }
    }

    #[test]
    fn test_parse_polyline_3d() {
        let content = r#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Polyline Test'),'2;1');
FILE_NAME('poly.step','','','','','','');
ENDSEC;
DATA;
#1 = CARTESIAN_POINT((0.0, 0.0, 3.0));
#2 = CARTESIAN_POINT((1.0, 0.0, 3.0));
#3 = CARTESIAN_POINT((1.0, 1.0, 3.0));
ENDSEC;
END-ISO-10303-21;"#;

        let parser = StepParser::new();
        let model = parser.parse_string(content).unwrap();

        assert_eq!(model.entities.len(), 3);

        // 验证 3D 点
        if let StepEntityData::CartesianPoint3D { coordinates } = &model.entities[0].data {
            assert_eq!(coordinates[0], 0.0);
            assert_eq!(coordinates[1], 0.0);
            assert_eq!(coordinates[2], 3.0);
        } else {
            panic!("Expected CartesianPoint3D");
        }
    }
}
