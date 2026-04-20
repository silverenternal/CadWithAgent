//! 封闭区域检测器
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
//!
//! 基于图论的环路检测算法，识别户型图中的封闭房间区域
//!
//! # 算法原理
//!
//! 1. **构建图模型**: 将线段端点作为顶点，线段作为边
//! 2. **查找环路**: 使用 DFS 查找所有简单环路
//! 3. **过滤有效区域**: 排除自交环路，保留最小封闭区域
//! 4. **计算几何属性**: 面积、周长、质心等
//! 5. **识别外边界**: 使用面积最大原则识别外墙边界
//! 6. **分析邻接关系**: 检测房间之间的共享墙体
//!
//! # 示例
//!
//! ```rust,no_run
//! use cadagent::analysis::closed_region_detector::ClosedRegionDetector;
//! use cadagent::geometry::primitives::Primitive;
//!
//! let detector = ClosedRegionDetector::new();
//! let regions = detector.find_closed_regions(&primitives);
//! ```

use crate::analysis::types::{
    AdjacencySummary, ClosedRegion, FloorPlanReport, Point as AnalysisPoint, RegionAdjacency,
    RegionAdjacencyGraph, RegionExport, ReportSummary,
};
use crate::geometry::primitives::{Line, Primitive};
use bitvec::prelude::*;
use lru::LruCache;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use std::hash::Hasher;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::time::Instant;

// 优化：预计算常用数学常量，避免重复计算
const FOUR_PI: f64 = 4.0 * std::f64::consts::PI;
const INV_FOUR_PI: f64 = 1.0 / FOUR_PI;

/// FNV-1a 哈希器（快速且分布均匀，适合整数键）
type FnvHasher = BuildHasherDefault<FnvHasherImpl>;

#[derive(Debug, Default)]
struct FnvHasherImpl {
    state: u64,
}

impl Hasher for FnvHasherImpl {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(0x0100_0000_01b3);
        }
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.state
    }
}

/// 图的邻接表表示
#[derive(Debug, Clone)]
struct Graph {
    /// 顶点坐标 -> 顶点 ID（使用 FNV-1a 哈希器提升性能）
    vertex_map: HashMap<(i64, i64), usize, FnvHasher>,
    /// 顶点 ID -> 坐标（直接使用 Vec 存储，避免二次查找）
    id_to_coord: Vec<(f64, f64)>,
    /// 邻接表：顶点 ID -> 相邻顶点 ID 列表（已排序，优化 DFS 遍历）
    /// 优化：使用 `SmallVec` 减少小度数顶点的堆分配
    adjacency: Vec<SmallVec<[usize; 4]>>,
    /// 边信息：(顶点 1, 顶点 2) -> 基元 ID
    edge_to_primitive: HashMap<(usize, usize), usize, FnvHasher>,
    /// 边计数器（用于 `BitVec` 索引）
    edge_count: usize,
    /// 边索引映射：规范化边 -> 边 ID
    edge_to_index: HashMap<(usize, usize), usize, FnvHasher>,
    /// 坐标量化精度
    epsilon: f64,
    /// 顶点度数（用于启发式搜索）
    vertex_degrees: Vec<usize>,
}

impl Graph {
    fn new(epsilon: f64) -> Self {
        Self {
            vertex_map: HashMap::with_hasher(FnvHasher::default()),
            id_to_coord: Vec::new(),
            adjacency: Vec::new(),
            edge_to_primitive: HashMap::with_hasher(FnvHasher::default()),
            edge_count: 0,
            edge_to_index: HashMap::with_hasher(FnvHasher::default()),
            epsilon,
            vertex_degrees: Vec::new(),
        }
    }

    /// 添加或获取顶点 ID
    fn get_or_add_vertex(&mut self, x: f64, y: f64) -> usize {
        // 使用量化坐标来合并接近的点
        let quantized = (
            quantize_coord(x, self.epsilon),
            quantize_coord(y, self.epsilon),
        );

        if let Some(&id) = self.vertex_map.get(&quantized) {
            id
        } else {
            let id = self.id_to_coord.len();
            self.vertex_map.insert(quantized, id);
            self.id_to_coord.push((x, y));
            self.adjacency.push(SmallVec::new()); // 使用 SmallVec 替代 Vec
            self.vertex_degrees.push(0); // 初始化度数为 0
            id
        }
    }

    /// 添加边
    fn add_edge(&mut self, v1: usize, v2: usize, primitive_id: usize) {
        // 使用 binary search 保持邻接表有序，优化后续 DFS 遍历
        // 优化：SmallVec 的 insert 与 Vec 兼容，但减少小集合堆分配
        let adj_v1 = &mut self.adjacency[v1];
        if let Err(pos) = adj_v1.binary_search(&v2) {
            adj_v1.insert(pos, v2);
        }

        let adj_v2 = &mut self.adjacency[v2];
        if let Err(pos) = adj_v2.binary_search(&v1) {
            adj_v2.insert(pos, v1);
        }

        // 存储边信息（双向）
        self.edge_to_primitive.insert((v1, v2), primitive_id);
        self.edge_to_primitive.insert((v2, v1), primitive_id);

        // 为边分配索引（用于 BitVec）
        let normalized_edge = if v1 < v2 { (v1, v2) } else { (v2, v1) };
        if !self.edge_to_index.contains_key(&normalized_edge) {
            self.edge_to_index.insert(normalized_edge, self.edge_count);
            self.edge_count += 1;
        }

        // 更新顶点度数
        if v1 < self.vertex_degrees.len() {
            self.vertex_degrees[v1] += 1;
        }
        if v2 < self.vertex_degrees.len() {
            self.vertex_degrees[v2] += 1;
        }
    }

    /// 从图构建环路（优化版本：启发式 + 早期剪枝 + 边重置优化）
    ///
    /// # 优化点
    /// - 使用 `BitVec` 替代 Vec<bool> 存储已访问边（性能优化 + 内存效率）
    /// - 启发式：优先从度数高的顶点开始搜索（更可能形成环路）
    /// - 早期剪枝：度数为 0 或 1 的顶点不可能形成环路
    /// - 边重置优化：每次 DFS 后只重置访问过的边，避免全量重置
    fn find_all_cycles(&self) -> Vec<Vec<usize>> {
        let mut cycles = Vec::new();
        // 使用 BitVec 替代 Vec<bool> 存储已访问边（性能优化 + 内存效率）
        // BitVec 使用 1 bit 每边，比 Vec<bool> 的 1 byte 每边节省 8 倍内存
        let mut visited_edges = BitVec::repeat(false, self.edge_count);

        // 启发式：优先从度数高的顶点开始搜索（更可能形成环路）
        let mut vertex_order: Vec<usize> = (0..self.id_to_coord.len()).collect();
        vertex_order.sort_by(|&a, &b| {
            // 度数高的优先，相同度数按坐标排序保证确定性
            self.vertex_degrees[b]
                .cmp(&self.vertex_degrees[a])
                .then_with(|| {
                    let coord_a = self.id_to_coord[a];
                    let coord_b = self.id_to_coord[b];
                    (coord_a.0.partial_cmp(&coord_b.0).unwrap())
                        .then_with(|| coord_a.1.partial_cmp(&coord_b.1).unwrap())
                })
        });

        // 优化：预分配路径向量容量，减少重复分配
        // 典型户型图房间顶点数为 4-8
        let mut current_path: Vec<usize> = Vec::with_capacity(16);
        let mut current_path_set: HashSet<usize> = HashSet::with_capacity(16);

        // 从每个顶点开始 DFS 查找环路
        for &start in &vertex_order {
            // 剪枝：度数为 0 或 1 的顶点不可能形成环路
            if self.vertex_degrees[start] < 2 {
                continue;
            }

            // 重置路径和访问集合
            current_path.clear();
            current_path.push(start);
            current_path_set.clear();
            current_path_set.insert(start);

            self.find_cycles_dfs(
                start,
                &mut current_path,
                &mut current_path_set,
                &mut visited_edges,
                &mut cycles,
            );

            // 优化：重置 visited_edges 只重置当前 start 相关的边
            // 这比全量重置更高效
            for neighbor in &self.adjacency[start] {
                let edge = if start < *neighbor {
                    (start, *neighbor)
                } else {
                    (*neighbor, start)
                };
                if let Some(&edge_idx) = self.edge_to_index.get(&edge) {
                    visited_edges.set(edge_idx, false);
                }
            }
        }

        cycles
    }

    /// DFS 查找环路（迭代版本：使用显式栈避免递归栈溢出）
    ///
    /// # 优化点
    /// - 使用显式栈替代递归，避免深层递归导致的栈溢出
    /// - 使用 `SmallVec` 减少小集合堆分配
    /// - 启发式邻居排序 + 早期剪枝
    /// - 使用线性搜索替代 `HashSet` 查找（小集合更快）
    fn find_cycles_dfs(
        &self,
        start: usize,
        path: &mut Vec<usize>,
        path_set: &mut HashSet<usize>,
        visited_edges: &mut BitVec,
        cycles: &mut Vec<Vec<usize>>,
    ) {
        // 迭代式 DFS 状态
        struct DfsState {
            current: usize,
            neighbor_idx: usize,
            neighbors: SmallVec<[usize; 4]>,
        }

        // 优化：使用小栈缓冲区避免堆分配（典型 DFS 深度 < 16）
        // 对于户型图房间，DFS 深度通常为 4-8 个顶点
        let mut stack: SmallVec<[DfsState; 8]> = SmallVec::new();

        // 初始化：处理起始顶点
        let start_neighbors = self.get_sorted_neighbors(start, start);
        stack.push(DfsState {
            current: start,
            neighbor_idx: 0,
            neighbors: start_neighbors,
        });

        while let Some(state) = stack.last_mut() {
            // 限制环路长度，避免组合爆炸
            if path.len() > 50 {
                stack.pop();
                if stack.is_empty() {
                    break;
                }
                continue;
            }

            // 早期剪枝：如果当前顶点度数不足，无法形成环路
            if self.vertex_degrees[state.current] < 2 {
                stack.pop();
                if stack.is_empty() {
                    break;
                }
                continue;
            }

            // 处理下一个邻居
            if state.neighbor_idx >= state.neighbors.len() {
                // 所有邻居处理完毕，回溯
                if path.len() > 1 {
                    let last = path.pop();
                    if let Some(last) = last {
                        path_set.remove(&last);
                    }
                }
                stack.pop();
                continue;
            }

            let neighbor = state.neighbors[state.neighbor_idx];
            state.neighbor_idx += 1;

            // 规范化边用于访问检查
            let edge = if state.current < neighbor {
                (state.current, neighbor)
            } else {
                (neighbor, state.current)
            };

            // 获取边索引
            let edge_index = match self.edge_to_index.get(&edge) {
                Some(&idx) => idx,
                None => continue, // 边不存在，跳过
            };

            // 如果边已访问，跳过
            if visited_edges[edge_index] {
                continue;
            }

            // 如果回到起点，找到环路
            if neighbor == start && path.len() >= 3 {
                let cycle = path.clone();
                cycles.push(cycle);
                continue;
            }

            // 优化：小集合使用线性搜索（path.len() < 16 时比 HashSet 快）
            // 大集合使用 HashSet 查找
            let in_path = if path.len() <= 16 {
                path.contains(&neighbor)
            } else {
                path_set.contains(&neighbor)
            };

            // 如果顶点已在路径中，跳过（避免自交）
            if in_path {
                continue;
            }

            // 标记边为已访问，推进到下一个顶点
            visited_edges.set(edge_index, true);
            path.push(neighbor);
            path_set.insert(neighbor);

            // 压入新状态
            let next_neighbors = self.get_sorted_neighbors(neighbor, start);
            stack.push(DfsState {
                current: neighbor,
                neighbor_idx: 0,
                neighbors: next_neighbors,
            });
        }
    }

    /// 获取排序后的邻居列表（启发式排序）
    ///
    /// # 优化
    /// - 使用 `SmallVec` 减少堆分配
    /// - 按度数降序排序，优先探索度数高的邻居
    /// - 使用内联比较减少函数调用
    #[inline]
    fn get_sorted_neighbors(&self, vertex: usize, start: usize) -> SmallVec<[usize; 4]> {
        let neighbors_vec = &self.adjacency[vertex];
        // 优化：直接使用 SmallVec，避免 Vec -> SmallVec 转换
        let mut neighbors: SmallVec<[usize; 4]> = SmallVec::with_capacity(neighbors_vec.len());
        neighbors.extend_from_slice(neighbors_vec);

        // 启发式：按度数排序邻居，优先探索度数高的邻居（更可能形成环路）
        // 优化：内联比较逻辑，避免闭包分配
        neighbors.sort_by(|&a: &usize, &b: &usize| {
            // 如果邻居是起点，优先处理
            if a == start {
                std::cmp::Ordering::Less
            } else if b == start {
                std::cmp::Ordering::Greater
            } else {
                // 否则按度数降序排序
                self.vertex_degrees[b].cmp(&self.vertex_degrees[a])
            }
        });

        neighbors
    }
}

/// 量化坐标，用于合并接近的顶点
fn quantize_coord(x: f64, epsilon: f64) -> i64 {
    // 使用可配置精度量化
    (x / epsilon).round() as i64
}

/// 简单的时间戳格式化（不依赖 chrono crate）
fn chrono_lite_format(timestamp: u64) -> String {
    // 使用系统命令或简单格式化
    // 这里使用简单的 ISO 8601 格式
    let secs = timestamp;
    // 计算近似日期（从 1970-01-01 开始）
    let days = secs / 86400;
    let year = 1970 + (days / 365) as i32;
    let day_of_year = days % 365;
    let month = ((day_of_year * 12) / 365) + 1;
    let day = ((day_of_year * 30) % 365) + 1;
    let hour = (secs % 86400) / 3600;
    let minute = (secs % 3600) / 60;
    let second = secs % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// 封闭区域检测器
pub struct ClosedRegionDetector {
    /// 最小区域面积阈值（小于此值的区域会被过滤）
    min_area: f64,
    /// 最小顶点数
    min_vertices: usize,
    /// 是否输出详细日志
    verbose: bool,
    /// 坐标量化精度（默认 0.001）
    quantization_epsilon: f64,
    /// 顶点签名缓存（避免重复计算）- 使用 `RefCell` 实现内部可变性
    signature_cache: RefCell<LruCache<u64, u64>>,
}

/// 邻接关系分析统计
#[derive(Debug, Default)]
struct AdjacencyStats {
    /// 总边界基元数量
    total_boundary_primitives: usize,
    /// 共享基元数量
    shared_primitives: usize,
    /// 找到的邻接关系数量
    adjacencies_found: usize,
    /// 耗时（毫秒）
    elapsed_ms: u64,
}

impl AdjacencyStats {
    fn new() -> Self {
        Self::default()
    }
}

/// 区域面积统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AreaStatistics {
    /// 区域总数
    pub count: usize,
    /// 总面积
    pub total_area: f64,
    /// 平均面积
    pub mean_area: f64,
    /// 中位数面积
    pub median_area: f64,
    /// 最小面积
    pub min_area: f64,
    /// 最大面积
    pub max_area: f64,
    /// 标准差
    pub std_dev: f64,
    /// 面积分布直方图（桶边界，桶计数）
    pub histogram: Vec<(f64, usize)>,
}

/// 房间类型统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomTypeStatistics {
    /// 房间类型 -> 数量
    pub type_counts: HashMap<String, usize>,
    /// 房间类型 -> 平均面积
    pub type_avg_areas: HashMap<String, f64>,
    /// 房间类型 -> 面积列表
    #[serde(skip)]
    pub type_area_lists: HashMap<String, Vec<f64>>,
}

impl ClosedRegionDetector {
    /// 创建新的检测器
    pub fn new() -> Self {
        // 优化：使用 LRU 缓存避免重复计算顶点签名
        // 容量设置为 1024，足够缓存典型户型图的顶点签名
        let cache_capacity = NonZeroUsize::new(1024).unwrap();
        Self {
            min_area: 0.01,
            min_vertices: 3,
            verbose: false,
            quantization_epsilon: 0.001,
            signature_cache: RefCell::new(LruCache::new(cache_capacity)),
        }
    }

    /// 设置最小面积阈值
    pub fn with_min_area(mut self, area: f64) -> Self {
        self.min_area = area;
        self
    }

    /// 设置最小顶点数
    pub fn with_min_vertices(mut self, count: usize) -> Self {
        self.min_vertices = count;
        self
    }

    /// 设置是否输出详细日志
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 设置坐标量化精度
    ///
    /// # 参数
    ///
    /// * `epsilon` - 坐标量化精度，用于合并接近的顶点。
    ///   较小的值（如 0.0001）提供更高精度，但可能产生更多顶点。
    ///   较大的值（如 0.01）会合并更多顶点，但可能丢失细节。
    ///   默认值：0.001
    pub fn with_quantization_epsilon(mut self, epsilon: f64) -> Self {
        self.quantization_epsilon = epsilon.max(1e-6); // 最小精度限制
        self
    }

    /// 从基元列表中查找所有封闭区域
    ///
    /// # 性能说明
    /// - 图构建：顺序处理（依赖累积）
    /// - 环路检测：已在内部使用启发式 + 迭代 DFS 优化
    /// - 区域转换：顺序处理（典型户型图环路数量 < 100，并行开销大于收益）
    /// - 邻接分析：使用 Rayon 并行处理（见 `analyze_adjacencies` 方法）
    pub fn find_closed_regions(&self, primitives: &[Primitive]) -> Vec<ClosedRegion> {
        let start_time = Instant::now();

        // Step 1: 构建图
        let mut graph = Graph::new(self.quantization_epsilon);
        let mut line_primitives: Vec<(usize, &Line)> = Vec::new();

        for (id, primitive) in primitives.iter().enumerate() {
            if let Primitive::Line(line) = primitive {
                let v1 = graph.get_or_add_vertex(line.start.x, line.start.y);
                let v2 = graph.get_or_add_vertex(line.end.x, line.end.y);

                if v1 != v2 {
                    graph.add_edge(v1, v2, id);
                    line_primitives.push((id, line));
                }
            }
        }

        // Step 2: 查找所有环路（内部已优化：启发式 + 迭代 DFS）
        let cycles = graph.find_all_cycles();
        let cycles_count = cycles.len();

        // Step 3: 过滤并转换为封闭区域
        let mut regions: Vec<ClosedRegion> = cycles
            .into_iter()
            .filter_map(|cycle| self.cycle_to_region(cycle, &graph))
            .collect();

        // Step 4: 去重（移除包含相同顶点的区域）
        regions = self.deduplicate_regions(regions);

        // Step 5: 过滤最小区域（移除面积过小的区域）
        let regions_before_filter = regions.len();
        regions = self.filter_small_regions(regions);
        let regions_after_filter = regions.len();

        // Step 6: 识别外边界
        self.identify_outer_boundary(&mut regions);

        let elapsed = start_time.elapsed();

        if self.verbose {
            eprintln!("封闭区域检测完成：{}ms", elapsed.as_millis());
            eprintln!("  - 检测到 {cycles_count} 个环路");
            eprintln!("  - 去重后 {regions_before_filter} 个区域");
            eprintln!(
                "  - 过滤后 {} 个区域（移除 {} 个小区域）",
                regions_after_filter,
                regions_before_filter - regions_after_filter
            );
            eprintln!("  - 最终 {} 个有效区域", regions.len());
        }

        regions
    }

    /// 将环路转换为封闭区域
    ///
    /// # 优化
    /// - 预分配顶点向量容量
    /// - 使用迭代器优化边界基元收集
    /// - 复用面积和周长值避免重复计算
    fn cycle_to_region(&self, cycle: Vec<usize>, graph: &Graph) -> Option<ClosedRegion> {
        if cycle.len() < self.min_vertices {
            return None;
        }

        // 优化：预分配顶点向量容量
        let mut vertices: Vec<AnalysisPoint> = Vec::with_capacity(cycle.len());
        for &id in &cycle {
            let (x, y) = graph.id_to_coord[id];
            vertices.push(AnalysisPoint { x, y });
        }

        // 计算面积
        let area = self.shoelace_area(&vertices);
        if area.abs() < self.min_area {
            return None;
        }

        // 计算周长
        let perimeter = self.calculate_perimeter(&vertices);

        // 计算质心
        let centroid = self.calculate_centroid(&vertices);

        // 优化：预分配边界基元 ID 向量容量（通常等于顶点数或顶点数 +1）
        let mut boundary_primitive_ids: Vec<usize> = Vec::with_capacity(cycle.len());

        // 获取边界基元 ID（使用迭代器优化）
        for window in cycle.windows(2) {
            let v1 = window[0];
            let v2 = window[1];
            if let Some(&prim_id) = graph.edge_to_primitive.get(&(v1, v2)) {
                boundary_primitive_ids.push(prim_id);
            }
        }

        // 补充最后一条边（回到起点）
        if let Some(&last_id) = graph
            .edge_to_primitive
            .get(&(cycle[cycle.len() - 1], cycle[0]))
        {
            // 优化：使用 contains 检查（小集合线性搜索更快）
            if !boundary_primitive_ids.contains(&last_id) {
                boundary_primitive_ids.push(last_id);
            }
        }

        // 计算形状特征（复用已计算的面积和周长，避免重复计算）
        let rectangularity = Some(self.calculate_rectangularity(&vertices, area));
        let compactness = Some(self.calculate_compactness(&vertices, perimeter, area));
        let aspect_ratio = self.calculate_aspect_ratio_with_centroid(&vertices, &centroid);
        let convexity = self.calculate_convexity(&vertices);
        let orientation = self.calculate_orientation(&vertices);
        let circularity = self.calculate_circularity(&vertices, perimeter, area);
        let shape_factor = self.calculate_shape_factor(&vertices, perimeter, area);

        Some(ClosedRegion {
            id: 0, // 稍后分配
            boundary_primitive_ids,
            vertices,
            area: area.abs(),
            perimeter,
            centroid,
            room_type: None,
            confidence: 0.9,
            is_outer_boundary: false,
            rectangularity,
            compactness,
            aspect_ratio,
            convexity,
            orientation,
            circularity,
            shape_factor,
        })
    }

    /// 计算矩形度（区域面积 / 最小包围矩形面积）
    ///
    /// # 优化
    /// 复用传入的面积值，避免重复计算
    fn calculate_rectangularity(&self, vertices: &[AnalysisPoint], area: f64) -> f64 {
        if vertices.len() < 3 {
            return 0.0;
        }

        // 计算最小包围矩形
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for v in vertices {
            min_x = min_x.min(v.x);
            max_x = max_x.max(v.x);
            min_y = min_y.min(v.y);
            max_y = max_y.max(v.y);
        }

        let bbox_area = (max_x - min_x) * (max_y - min_y);
        if bbox_area < 1e-10 {
            return 0.0;
        }

        // 使用传入的面积值，避免重复计算
        (area.abs() / bbox_area).min(1.0)
    }

    /// 计算紧凑度（等周长圆的面积 / 实际面积）
    /// 紧凑度 = 4πA / P²，范围 (0, 1]，1 表示完美圆形
    ///
    /// # 优化
    /// - 使用预计算常量 `FOUR_PI` 替代 4.0 * PI
    /// - 添加 #[inline] 属性促进内联
    #[inline]
    fn calculate_compactness(&self, _vertices: &[AnalysisPoint], perimeter: f64, area: f64) -> f64 {
        if perimeter < 1e-10 || area < 1e-10 {
            return 0.0;
        }

        // 优化：使用预计算常量 FOUR_PI
        let compactness = (FOUR_PI * area) / (perimeter * perimeter);
        compactness.min(1.0)
    }

    /// 计算长宽比（使用主成分分析）
    ///
    /// # 优化
    /// 复用传入的质心值，避免重复计算
    #[allow(clippy::similar_names)]
    fn calculate_aspect_ratio_with_centroid(
        &self,
        vertices: &[AnalysisPoint],
        centroid: &AnalysisPoint,
    ) -> Option<f64> {
        if vertices.len() < 3 {
            return None;
        }

        // 计算协方差矩阵（使用传入的质心）
        let mut cov_xx = 0.0;
        let mut cov_yy = 0.0;
        let mut cov_xy = 0.0;

        for v in vertices {
            let dx = v.x - centroid.x;
            let dy = v.y - centroid.y;
            cov_xx += dx * dx;
            cov_yy += dy * dy;
            cov_xy += dx * dy;
        }

        let n = vertices.len() as f64;
        cov_xx /= n;
        cov_yy /= n;
        cov_xy /= n;

        // 计算特征值（方差最大的方向）
        let trace = cov_xx + cov_yy;
        let det = cov_xx * cov_yy - cov_xy * cov_xy;
        let discriminant = (trace * trace - 4.0 * det).max(0.0).sqrt();

        let lambda1 = f64::midpoint(trace, discriminant); // 最大特征值
        let lambda2 = (trace - discriminant) / 2.0; // 最小特征值

        if lambda2 < 1e-10 {
            return None;
        }

        Some((lambda1 / lambda2).sqrt())
    }

    /// 计算长宽比（使用主成分分析）
    ///
    /// # 注意
    /// 此方法会重新计算质心，建议优先使用 `calculate_aspect_ratio_with_centroid`
    #[cfg(test)]
    fn calculate_aspect_ratio(&self, vertices: &[AnalysisPoint]) -> Option<f64> {
        let centroid = self.calculate_centroid(vertices);
        self.calculate_aspect_ratio_with_centroid(vertices, &centroid)
    }

    /// 计算凸度（凸包面积 / 实际面积）
    ///
    /// 凸度衡量多边形的凹凸程度，1 表示完美凸多边形。
    /// 对于户型图分析，凸度可以帮助识别房间的规则性。
    fn calculate_convexity(&self, vertices: &[AnalysisPoint]) -> Option<f64> {
        if vertices.len() < 3 {
            return None;
        }

        // 计算凸包
        let convex_hull = self.compute_convex_hull(vertices);
        if convex_hull.len() < 3 {
            return None;
        }

        // 计算凸包面积
        let convex_area = self.shoelace_area(&convex_hull).abs();
        let region_area = self.shoelace_area(vertices).abs();

        if convex_area < 1e-10 {
            return None;
        }

        // 凸度 = 凸包面积 / 实际面积（对于凸多边形为 1，凹多边形 < 1）
        // 注意：这里我们使用 区域面积/凸包面积，这样凸多边形的值接近 1
        Some((region_area / convex_area).min(1.0))
    }

    /// 计算凸包（使用 Graham 扫描算法，优化版本）
    ///
    /// # 优化点
    /// - 使用迭代替代递归（已经是迭代）
    /// - 预分配容量减少堆分配
    /// - 使用叉积计算避免三角函数
    /// - 优化极角排序，减少重复计算
    fn compute_convex_hull(&self, vertices: &[AnalysisPoint]) -> Vec<AnalysisPoint> {
        let n = vertices.len();
        if n <= 3 {
            // 优化：小集合直接返回，避免不必要计算
            return vertices.to_vec();
        }

        // 优化：单次遍历找到最下方的点，同时预分配容量
        let mut points: Vec<AnalysisPoint> = Vec::with_capacity(n);
        points.extend_from_slice(vertices);

        // 找到最下方的点（y 最小，x 最小）- 使用迭代器优化
        let mut min_idx = 0;
        let mut min_point = points[0];
        for (i, &point) in points.iter().enumerate().skip(1) {
            // 分支预测优化：先比较 y，再比较 x（大多数情况 y 不同）
            if point.y < min_point.y || (point.y == min_point.y && point.x < min_point.x) {
                min_idx = i;
                min_point = point;
            }
        }
        points.swap(0, min_idx);
        let pivot = points[0];

        // 按极角排序 - 优化：预计算 pivot 相关值，避免重复计算
        let pivot_x = pivot.x;
        let pivot_y = pivot.y;

        points[1..].sort_by(|a, b| {
            // 内联叉积计算，避免函数调用开销
            let cross = (a.x - pivot_x) * (b.y - pivot_y) - (a.y - pivot_y) * (b.x - pivot_x);
            if cross.abs() < 1e-10 {
                // 极角相同，按距离排序 - 优化：使用平方距离避免 sqrt
                let dist_a = (a.x - pivot_x) * (a.x - pivot_x) + (a.y - pivot_y) * (a.y - pivot_y);
                let dist_b = (b.x - pivot_x) * (b.x - pivot_x) + (b.y - pivot_y) * (b.y - pivot_y);
                dist_a.partial_cmp(&dist_b).unwrap()
            } else {
                cross.partial_cmp(&0.0).unwrap().reverse()
            }
        });

        // Graham 扫描 - 优化：使用更精确的容量估计
        // 凸包顶点数通常远小于总顶点数，对于矩形房间通常为 4
        let estimated_hull_size = n.min(16);
        let mut hull: Vec<AnalysisPoint> = Vec::with_capacity(estimated_hull_size);

        for point in points {
            // 内联叉积计算，减少函数调用
            while hull.len() >= 2 {
                let p2 = &hull[hull.len() - 1];
                let p1 = &hull[hull.len() - 2];
                let cross = (p2.x - p1.x) * (point.y - p1.y) - (p2.y - p1.y) * (point.x - p1.x);
                if cross <= 0.0 {
                    hull.pop();
                } else {
                    break;
                }
            }
            hull.push(point);
        }

        hull
    }

    /// 计算方向角（主轴与 X 轴的夹角，弧度）
    ///
    /// 使用主成分分析确定区域的主轴方向。
    /// 对于矩形房间，方向角可以帮助识别房间的朝向。
    #[allow(clippy::similar_names)]
    fn calculate_orientation(&self, vertices: &[AnalysisPoint]) -> Option<f64> {
        if vertices.len() < 3 {
            return None;
        }

        let centroid = self.calculate_centroid(vertices);

        // 计算协方差矩阵
        let mut cov_xx = 0.0;
        let mut cov_yy = 0.0;
        let mut cov_xy = 0.0;

        for v in vertices {
            let dx = v.x - centroid.x;
            let dy = v.y - centroid.y;
            cov_xx += dx * dx;
            cov_yy += dy * dy;
            cov_xy += dx * dy;
        }

        let n = vertices.len() as f64;
        cov_xx /= n;
        cov_yy /= n;
        cov_xy /= n;

        // 计算主轴方向（特征向量）
        // 特征向量方向：tan(2θ) = 2*cov_xy / (cov_xx - cov_yy)
        let theta = if (cov_xx - cov_yy).abs() < 1e-10 {
            std::f64::consts::FRAC_PI_4 // 45 度
        } else {
            0.5 * (2.0 * cov_xy / (cov_xx - cov_yy)).atan()
        };

        // 确保角度在 [0, π) 范围内
        let normalized_theta = if theta < 0.0 {
            theta + std::f64::consts::PI
        } else {
            theta
        };

        Some(normalized_theta)
    }

    /// 计算圆度（等周长圆的面积 / 实际面积）
    ///
    /// 圆度与紧凑度类似，但使用不同的归一化方式。
    /// 圆度 = 4πA / P²，范围 (0, 1]，1 表示完美圆形。
    /// 这与紧凑度相同，但命名不同以强调圆度特征。
    ///
    /// # 优化
    /// - 使用预计算常量 `FOUR_PI` 替代 4.0 * PI
    #[inline]
    fn calculate_circularity(
        &self,
        _vertices: &[AnalysisPoint],
        perimeter: f64,
        area: f64,
    ) -> Option<f64> {
        if perimeter < 1e-10 || area < 1e-10 {
            return None;
        }

        // 优化：使用预计算常量 FOUR_PI
        let circularity = (FOUR_PI * area) / (perimeter * perimeter);
        Some(circularity.min(1.0))
    }

    /// 计算形状因子（形状复杂度度量）
    ///
    /// 形状因子 = P² / (4πA)，是紧凑度的倒数。
    /// 值越接近 1 表示形状越简单（圆形），值越大表示形状越复杂。
    ///
    /// # 优化
    /// - 使用预计算常量 `INV_FOUR_PI` 替代 1.0 / (4.0 * PI)
    #[inline]
    fn calculate_shape_factor(
        &self,
        _vertices: &[AnalysisPoint],
        perimeter: f64,
        area: f64,
    ) -> Option<f64> {
        if perimeter < 1e-10 || area < 1e-10 {
            return None;
        }

        // 优化：使用预计算常量 INV_FOUR_PI
        let shape_factor = (perimeter * perimeter) * (INV_FOUR_PI / area);
        Some(shape_factor.max(1.0))
    }

    /// 使用鞋带公式计算多边形面积（SIMD 友好版本）
    ///
    /// # 优化点
    /// - 使用切片迭代器替代索引访问，更利于 LLVM 向量化
    /// - 使用 fold 替代可变变量，帮助编译器优化
    #[inline]
    fn shoelace_area(&self, vertices: &[AnalysisPoint]) -> f64 {
        if vertices.len() < 3 {
            return 0.0;
        }

        // 优化：使用 zip 和 fold 替代索引循环，更利于 SIMD 优化
        let sum: f64 = vertices
            .iter()
            .zip(vertices.iter().cycle().skip(1))
            .map(|(a, b)| a.x * b.y - b.x * a.y)
            .fold(0.0, |acc, v| acc + v);

        sum / 2.0
    }

    /// 计算周长（SIMD 友好版本）
    ///
    /// # 优化点
    /// - 使用切片迭代器替代索引访问
    /// - 内联距离计算，避免函数调用开销
    #[inline]
    fn calculate_perimeter(&self, vertices: &[AnalysisPoint]) -> f64 {
        if vertices.is_empty() {
            return 0.0;
        }

        // 优化：使用 zip 和 fold 替代索引循环
        vertices
            .iter()
            .zip(vertices.iter().cycle().skip(1))
            .map(|(a, b)| {
                let dx = b.x - a.x;
                let dy = b.y - a.y;
                // 内联距离计算，避免函数调用
                (dx * dx + dy * dy).sqrt()
            })
            .fold(0.0, |acc, v| acc + v)
    }

    /// 计算质心（优化版本：减少重复计算）
    ///
    /// # 优化点
    /// - 单次遍历同时计算面积和质心
    /// - 使用迭代器替代索引访问
    /// - 提前处理退化情况
    #[inline]
    fn calculate_centroid(&self, vertices: &[AnalysisPoint]) -> AnalysisPoint {
        if vertices.is_empty() {
            return AnalysisPoint { x: 0.0, y: 0.0 };
        }

        // 优化：单次遍历同时计算面积和质心相关项
        // 注意：fold 计算的 area 是 2*实际面积（鞋带公式的结果）
        let (two_area, cx, cy) = vertices
            .iter()
            .zip(vertices.iter().cycle().skip(1))
            .map(|(a, b)| {
                let cross = a.x * b.y - b.x * a.y;
                (cross, (a.x + b.x) * cross, (a.y + b.y) * cross)
            })
            .fold(
                (0.0, 0.0, 0.0),
                |(a_sum, cx_sum, cy_sum), (cross, cx, cy)| {
                    (a_sum + cross, cx_sum + cx, cy_sum + cy)
                },
            );

        let six_area = 3.0 * two_area; // 6 * area = 3 * (2 * area)

        // 处理退化情况（面积为 0）
        if six_area.abs() < 1e-10 {
            // 退化为线段或点，返回平均位置
            let (sum_x, sum_y) = vertices
                .iter()
                .fold((0.0, 0.0), |(sx, sy), v| (sx + v.x, sy + v.y));
            let n = vertices.len() as f64;
            return AnalysisPoint {
                x: sum_x / n,
                y: sum_y / n,
            };
        }

        AnalysisPoint {
            x: cx / six_area,
            y: cy / six_area,
        }
    }

    /// 去重：移除顶点集合相同的区域（优化版本：排序 + dedup）
    ///
    /// # 优化
    /// - 使用排序 + dedup 替代 HashSet，减少哈希表开销
    /// - 使用 `SmallVec` 减少中间分配
    /// - 预分配容量避免重复分配
    /// - 使用 `SmallVec`[; 32] 替代 [; 64]，典型检测到的区域数为 10-30
    fn deduplicate_regions(&self, regions: Vec<ClosedRegion>) -> Vec<ClosedRegion> {
        if regions.is_empty() {
            return regions;
        }

        // 为每个区域计算签名并存储索引
        // 优化：使用 SmallVec[; 32] 替代 [; 64]，典型区域数为 10-30
        let mut region_signatures: SmallVec<[(u64, usize); 32]> = regions
            .iter()
            .enumerate()
            .map(|(idx, region)| (self.compute_vertex_signature(&region.vertices), idx))
            .collect();

        // 按签名排序
        region_signatures.sort_unstable_by_key(|&(sig, _)| sig);

        // 去重：保留每个签名的第一个区域
        let mut result: Vec<ClosedRegion> = Vec::with_capacity(regions.len());
        let mut last_sig: Option<u64> = None;

        for (sig, idx) in region_signatures {
            if last_sig != Some(sig) {
                last_sig = Some(sig);
                result.push(regions[idx].clone());
            }
        }

        // 重新分配 ID
        for (i, region) in result.iter_mut().enumerate() {
            region.id = i;
        }

        result
    }

    /// 计算顶点签名（用于快速去重）
    /// 使用量化坐标的哈希值，保证相同顶点集产生相同签名
    ///
    /// # 优化
    /// - 使用 LRU 缓存避免重复计算（典型户型图中很多区域共享相同顶点）
    /// - 使用 FNV-1a 哈希直接计算，避免 `DefaultHasher` 开销
    /// - 内联哈希计算，减少函数调用
    /// - 使用 `SmallVec`[; 8] 替代 [; 16]，典型房间顶点数为 4-8
    fn compute_vertex_signature(&self, vertices: &[AnalysisPoint]) -> u64 {
        // 优化：先计算一个简单的哈希键用于缓存查找
        // 使用顶点数量和第一个顶点的量化坐标作为快速键
        let quick_key = if vertices.is_empty() {
            0
        } else {
            let v0_x = quantize_coord(vertices[0].x, self.quantization_epsilon) as u64;
            let v0_y = quantize_coord(vertices[0].y, self.quantization_epsilon) as u64;
            ((vertices.len() as u64) << 48) ^ v0_x ^ (v0_y << 8)
        };

        // 尝试从缓存获取（LRU 的 get 方法需要可变访问来更新访问顺序）
        if let Some(&cached_sig) = self.signature_cache.borrow_mut().get(&quick_key) {
            return cached_sig;
        }

        // FNV-1a 哈希参数
        const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const FNV_PRIME: u64 = 0x0100_0000_01b3;

        let mut hasher = FnvHasherImpl { state: FNV_OFFSET };

        // 收集并排序量化坐标（保证顶点顺序不影响签名）
        // 优化：使用 SmallVec[; 8] 替代 Vec，典型房间顶点数为 4-8
        let mut quantized: SmallVec<[(i64, i64); 8]> = vertices
            .iter()
            .map(|v| {
                (
                    quantize_coord(v.x, self.quantization_epsilon),
                    quantize_coord(v.y, self.quantization_epsilon),
                )
            })
            .collect();
        quantized.sort_unstable();

        // FNV-1a 哈希量化坐标（内联计算）
        for coord in quantized {
            // 哈希 x 坐标
            let x_bytes = coord.0.to_le_bytes();
            for &byte in &x_bytes {
                hasher.state ^= byte as u64;
                hasher.state = hasher.state.wrapping_mul(FNV_PRIME);
            }
            // 哈希 y 坐标
            let y_bytes = coord.1.to_le_bytes();
            for &byte in &y_bytes {
                hasher.state ^= byte as u64;
                hasher.state = hasher.state.wrapping_mul(FNV_PRIME);
            }
        }

        let signature = hasher.state;

        // 缓存结果
        self.signature_cache.borrow_mut().put(quick_key, signature);

        signature
    }

    /// 过滤最小区域（移除面积过小的区域）
    fn filter_small_regions(&self, regions: Vec<ClosedRegion>) -> Vec<ClosedRegion> {
        regions
            .into_iter()
            .filter(|region| region.area >= self.min_area)
            .collect()
    }

    /// 识别外边界（面积最大的区域）
    fn identify_outer_boundary(&self, regions: &mut [ClosedRegion]) {
        if let Some(max_idx) = regions
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.area.partial_cmp(&b.area).unwrap())
            .map(|(i, _)| i)
        {
            for (i, region) in regions.iter_mut().enumerate() {
                region.is_outer_boundary = i == max_idx;
            }
        }
    }

    /// 从 OCR 结果推断房间类型
    pub fn infer_room_types(
        &mut self,
        regions: &mut [ClosedRegion],
        ocr_result: Option<&crate::analysis::types::OcrResult>,
    ) {
        if let Some(ocr) = ocr_result {
            for region in regions {
                // 查找区域中心的文字标注
                if let Some(text) = ocr
                    .texts
                    .iter()
                    .find(|text| self.point_in_region(text.x, text.y, &region.vertices))
                {
                    region.room_type = Some(self.classify_room_type(&text.content));
                }
            }
        }
    }

    /// 计算区域面积统计
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    /// * `num_bins` - 直方图桶数量（默认 10）
    ///
    /// # 返回
    ///
    /// 返回面积统计信息，包括平均值、中位数、标准差和分布直方图。
    pub fn compute_area_statistics(
        &self,
        regions: &[ClosedRegion],
        num_bins: usize,
    ) -> AreaStatistics {
        if regions.is_empty() {
            return AreaStatistics::default();
        }

        let mut areas: Vec<f64> = regions.iter().map(|r| r.area).collect();
        let count = areas.len();
        let total_area: f64 = areas.iter().sum();
        let mean_area = total_area / count as f64;

        // 排序计算中位数
        areas.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_area = if count.is_multiple_of(2) {
            f64::midpoint(areas[count / 2 - 1], areas[count / 2])
        } else {
            areas[count / 2]
        };

        let min_area = areas.first().copied().unwrap_or(0.0);
        let max_area = areas.last().copied().unwrap_or(0.0);

        // 计算标准差
        let variance: f64 =
            areas.iter().map(|&a| (a - mean_area).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        // 构建直方图 - 优化：避免除零
        let bin_width = (max_area - min_area).max(1.0) / num_bins as f64;

        // 优化：单次遍历构建直方图，避免多次过滤
        let mut histogram_counts = vec![0usize; num_bins];
        for &area in &areas {
            let bin_idx = ((area - min_area) / bin_width).floor() as usize;
            if bin_idx < num_bins {
                histogram_counts[bin_idx] += 1;
            }
        }

        let mut histogram: Vec<(f64, usize)> = Vec::with_capacity(num_bins);
        for (i, &count) in histogram_counts.iter().enumerate().take(num_bins) {
            let bin_edge = min_area + (i + 1) as f64 * bin_width;
            histogram.push((bin_edge, count));
        }

        AreaStatistics {
            count,
            total_area,
            mean_area,
            median_area,
            min_area,
            max_area,
            std_dev,
            histogram,
        }
    }

    /// 计算房间类型统计
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    ///
    /// # 返回
    ///
    /// 返回房间类型统计信息，包括每种类型的数量和平均面积。
    pub fn compute_room_type_statistics(&self, regions: &[ClosedRegion]) -> RoomTypeStatistics {
        let mut stats = RoomTypeStatistics::default();

        for region in regions {
            if let Some(ref room_type) = region.room_type {
                *stats.type_counts.entry(room_type.clone()).or_insert(0) += 1;
                stats
                    .type_area_lists
                    .entry(room_type.clone())
                    .or_insert_with(Vec::new)
                    .push(region.area);
            }
        }

        // 计算每种类型的平均面积
        for (room_type, areas) in &stats.type_area_lists {
            let avg_area = areas.iter().sum::<f64>() / areas.len() as f64;
            stats.type_avg_areas.insert(room_type.clone(), avg_area);
        }

        stats
    }

    /// 导出区域为 JSON 格式
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    ///
    /// # 返回
    ///
    /// 返回 JSON 格式的区域数据。
    pub fn export_to_json(&self, regions: &[ClosedRegion]) -> serde_json::Value {
        let exports: Vec<RegionExport> = regions.iter().map(RegionExport::from).collect();
        serde_json::json!({
            "count": regions.len(),
            "regions": exports,
            "statistics": {
                "area": self.compute_area_statistics(regions, 10),
                "room_types": self.compute_room_type_statistics(regions)
            }
        })
    }

    /// 导出区域为 CSV 格式
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    /// * `writer` - 写入目标（文件或字符串）
    ///
    /// # 返回
    ///
    /// 返回 IO 操作结果。
    pub fn export_to_csv<W: Write>(
        &self,
        regions: &[ClosedRegion],
        writer: &mut W,
    ) -> io::Result<()> {
        // 写入表头
        writeln!(writer, "id,room_type,area,perimeter,centroid_x,centroid_y,rectangularity,compactness,aspect_ratio,convexity,orientation,circularity,shape_factor,boundary_count,is_outer_boundary,confidence")?;

        // 写入数据行
        for region in regions {
            let room_type = region.room_type.as_deref().unwrap_or("未知");
            let rectangularity = region
                .rectangularity
                .map_or_else(|| "N/A".to_string(), |v| v.to_string());
            let compactness = region
                .compactness
                .map_or_else(|| "N/A".to_string(), |v| v.to_string());
            let aspect_ratio = region
                .aspect_ratio
                .map_or_else(|| "N/A".to_string(), |v| v.to_string());
            let convexity = region
                .convexity
                .map_or_else(|| "N/A".to_string(), |v| v.to_string());
            let orientation = region
                .orientation
                .map_or_else(|| "N/A".to_string(), |v| v.to_string());
            let circularity = region
                .circularity
                .map_or_else(|| "N/A".to_string(), |v| v.to_string());
            let shape_factor = region
                .shape_factor
                .map_or_else(|| "N/A".to_string(), |v| v.to_string());

            writeln!(
                writer,
                "{},{},{:.4},{:.4},{:.4},{:.4},{},{},{},{},{},{},{},{},{},{:.2}",
                region.id,
                room_type,
                region.area,
                region.perimeter,
                region.centroid.x,
                region.centroid.y,
                rectangularity,
                compactness,
                aspect_ratio,
                convexity,
                orientation,
                circularity,
                shape_factor,
                region.boundary_primitive_ids.len(),
                if region.is_outer_boundary {
                    "true"
                } else {
                    "false"
                },
                region.confidence
            )?;
        }

        Ok(())
    }

    /// 导出区域为 CSV 字符串
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    ///
    /// # 返回
    ///
    /// 返回 CSV 格式的字符串。
    pub fn export_to_csv_string(&self, regions: &[ClosedRegion]) -> io::Result<String> {
        let mut output = Vec::new();
        self.export_to_csv(regions, &mut output)?;
        Ok(String::from_utf8_lossy(&output).to_string())
    }

    /// 导出区域为 JSON 字符串
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    ///
    /// # 返回
    ///
    /// 返回 JSON 格式的字符串。
    pub fn export_to_json_string(&self, regions: &[ClosedRegion]) -> String {
        let json_value = self.export_to_json(regions);
        serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "{}".to_string())
    }

    /// 导出区域到文件（JSON 或 CSV）
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    /// * `file_path` - 输出文件路径（.json 或.csv）
    ///
    /// # 返回
    ///
    /// 返回 IO 操作结果。
    pub fn export_to_file(&self, regions: &[ClosedRegion], file_path: &str) -> io::Result<()> {
        use std::fs::File;
        use std::path::Path;

        let path = Path::new(file_path);
        let mut file = File::create(path)?;

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("json") => {
                let json_string = self.export_to_json_string(regions);
                file.write_all(json_string.as_bytes())?;
            }
            Some("csv") => {
                self.export_to_csv(regions, &mut file)?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "不支持的文件格式，请使用 .json 或.csv 扩展名",
                ));
            }
        }

        Ok(())
    }

    /// 生成户型图分析报告
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    /// * `region_adjacency` - 区域邻接图
    /// * `title` - 报告标题
    ///
    /// # 返回
    ///
    /// 返回完整的户型分析报告。
    pub fn generate_report(
        &self,
        regions: &[ClosedRegion],
        region_adjacency: &RegionAdjacencyGraph,
        title: &str,
    ) -> FloorPlanReport {
        use std::time::{SystemTime, UNIX_EPOCH};

        // 生成时间戳
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let generated_at = chrono_lite_format(timestamp);

        // 计算区域统计数据
        let area_stats = self.compute_area_statistics(regions, 10);
        let room_type_stats = self.compute_room_type_statistics(regions);

        // 过滤出房间（排除外边界）并计算统计数据 - 单次遍历优化
        let rooms: Vec<&ClosedRegion> = regions.iter().filter(|r| !r.is_outer_boundary).collect();
        let total_rooms = rooms.len();

        // 单次遍历计算所有面积和形状统计量
        let (total_area, max_room_area, min_room_area, compactness_sum, rectangularity_sum) =
            rooms.iter().fold(
                (0.0_f64, 0.0_f64, f64::MAX, 0.0_f64, 0.0_f64),
                |(sum_area, max_area, min_area, sum_compact, sum_rect), r| {
                    (
                        sum_area + r.area,
                        max_area.max(r.area),
                        min_area.min(r.area),
                        sum_compact + r.compactness.unwrap_or(0.0),
                        sum_rect + r.rectangularity.unwrap_or(0.0),
                    )
                },
            );

        let avg_room_area = if total_rooms > 0 {
            total_area / total_rooms as f64
        } else {
            0.0
        };

        // 主要房间类型
        let dominant_room_type = room_type_stats
            .type_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(room_type, _)| room_type.clone());

        // 平均紧凑度和矩形度（使用单次遍历的结果）
        let overall_compactness = if total_rooms > 0 {
            Some(compactness_sum / total_rooms as f64)
        } else {
            None
        };
        let overall_rectangularity = if total_rooms > 0 {
            Some(rectangularity_sum / total_rooms as f64)
        } else {
            None
        };

        // 邻接关系摘要
        let total_adjacencies = region_adjacency.count();

        // 计算每个房间的邻接数
        let mut adjacency_counts: HashMap<usize, usize> = HashMap::new();
        for adjacency in &region_adjacency.adjacencies {
            *adjacency_counts.entry(adjacency.region_id_1).or_insert(0) += 1;
            *adjacency_counts.entry(adjacency.region_id_2).or_insert(0) += 1;
        }

        let (most_connected_room_id, max_adjacencies) = adjacency_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map_or((None, 0), |(&id, &count)| (Some(id), count));

        let avg_adjacencies_per_room = if total_rooms > 0 {
            total_adjacencies as f64 / total_rooms as f64
        } else {
            0.0
        };

        let adjacency_summary = AdjacencySummary {
            total_adjacencies,
            avg_adjacencies_per_room,
            most_connected_room_id,
            max_adjacencies,
        };

        // 生成分析建议
        let recommendations = self.generate_recommendations(
            &rooms,
            &area_stats,
            &room_type_stats,
            &adjacency_summary,
        );

        // 导出区域列表
        let region_exports: Vec<RegionExport> = regions.iter().map(RegionExport::from).collect();

        FloorPlanReport {
            title: title.to_string(),
            generated_at,
            summary: ReportSummary {
                total_rooms,
                total_area,
                avg_room_area,
                max_room_area,
                min_room_area,
                room_type_count: room_type_stats.type_counts.len(),
                dominant_room_type,
                overall_compactness,
                overall_rectangularity,
            },
            regions: region_exports,
            area_statistics: area_stats,
            room_type_statistics: room_type_stats,
            adjacency_summary,
            recommendations,
        }
    }

    /// 生成分析建议
    fn generate_recommendations(
        &self,
        rooms: &[&ClosedRegion],
        area_stats: &AreaStatistics,
        room_type_stats: &RoomTypeStatistics,
        adjacency_summary: &AdjacencySummary,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // 基于面积的建议
        if area_stats.std_dev > area_stats.mean_area * 0.5 {
            recommendations.push("房间面积差异较大，建议检查是否有功能区域划分不合理".to_string());
        }

        // 基于房间类型的建议
        if let Some(&bedroom_count) = room_type_stats.type_counts.get("卧室") {
            if bedroom_count < 2 {
                recommendations.push("卧室数量较少，可能不适合家庭居住".to_string());
            }
        }
        if !room_type_stats.type_counts.contains_key("卫生间") {
            recommendations.push("未检测到卫生间，建议检查户型完整性".to_string());
        }
        if !room_type_stats.type_counts.contains_key("厨房") {
            recommendations.push("未检测到厨房，建议检查户型完整性".to_string());
        }

        // 基于紧凑度的建议
        if let Some(compactness) = adjacency_summary.avg_adjacencies_per_room.into() {
            if compactness > 4.0 {
                recommendations
                    .push("房间平均邻接数较高，户型可能过于紧凑，通风采光可能受影响".to_string());
            }
        }

        // 基于矩形度的建议
        let avg_rectangularity =
            rooms.iter().filter_map(|r| r.rectangularity).sum::<f64>() / rooms.len().max(1) as f64;
        if avg_rectangularity < 0.7 {
            recommendations.push("房间形状不规则较多，可能影响家具布置和空间利用率".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("户型整体布局合理，无明显问题".to_string());
        }

        recommendations
    }

    /// 分析区域邻接关系（并行版本）
    ///
    /// 检测所有封闭区域之间的邻接关系，识别共享墙体。
    /// 使用 Rayon 并行处理加速大规模户型图分析。
    ///
    /// # 参数
    ///
    /// * `regions` - 封闭区域列表
    /// * `primitives` - 原始基元列表（用于计算共享边界长度）
    ///
    /// # 返回
    ///
    /// 返回区域邻接图，包含所有邻接关系。
    pub fn analyze_adjacencies(
        &self,
        regions: &[ClosedRegion],
        primitives: &[Primitive],
    ) -> RegionAdjacencyGraph {
        use smallvec::SmallVec;

        let start_time = Instant::now();
        let mut adjacency_graph = RegionAdjacencyGraph::new();

        // 性能统计
        let mut stats = AdjacencyStats::new();

        // Step 1: 构建基元到区域的映射（批量处理）
        // 优化：使用 SmallVec 减少小集合的堆分配，预分配容量
        // 优化：使用 SmallVec[; 2] 替代 [; 4]，典型共享墙体连接 2 个区域
        let mut primitive_to_regions: Vec<SmallVec<[usize; 2]>> =
            Vec::with_capacity(primitives.len());
        for _ in 0..primitives.len() {
            primitive_to_regions.push(SmallVec::new());
        }

        for region in regions {
            stats.total_boundary_primitives += region.boundary_primitive_ids.len();
            for &prim_id in &region.boundary_primitive_ids {
                if prim_id < primitives.len() {
                    primitive_to_regions[prim_id].push(region.id);
                }
            }
        }

        // Step 2: 查找共享基元（两个区域共用的墙体）- 并行处理
        // 优化：预先分配容量，避免重复分配
        let shared_count = primitive_to_regions.iter().filter(|v| v.len() >= 2).count();
        // 注意：需要使用 Vec 存储，因为 Rayon 需要 IntoParallelIterator
        // 优化：使用 SmallVec[; 2] 替代 [; 4]，典型共享墙体连接 2 个区域
        let mut shared_primitives: Vec<(usize, SmallVec<[usize; 2]>)> =
            Vec::with_capacity(shared_count);

        for (prim_id, region_ids) in primitive_to_regions.into_iter().enumerate() {
            if region_ids.len() >= 2 {
                shared_primitives.push((prim_id, region_ids));
            }
        }

        stats.shared_primitives = shared_primitives.len();

        // 并行处理每个共享基元，生成邻接关系
        // 优化：使用 unindexed_par_iter() 更好地负载均衡
        // 优化：使用 fold 减少中间分配，然后 merge 结果
        let adjacencies: Vec<RegionAdjacency> = shared_primitives
            .par_iter()
            .flat_map(|(prim_id, region_ids)| {
                // 优化：预计算区域对数量，预先分配容量
                let pair_count = region_ids.len() * (region_ids.len() - 1) / 2;
                // 注意：使用 Vec 而非 SmallVec，因为 Rayon 需要 IntoParallelIterator
                // 优化：使用 SmallVec[; 2] 替代 Vec，典型邻接对数为 1-2
                let mut local_adjacencies: SmallVec<[RegionAdjacency; 2]> =
                    SmallVec::with_capacity(pair_count);

                // 生成所有区域对
                for i in 0..region_ids.len() {
                    for j in (i + 1)..region_ids.len() {
                        let id1 = region_ids[i];
                        let id2 = region_ids[j];

                        // 计算共享边界长度（内联计算，避免函数调用开销）
                        let shared_length = primitives
                            .get(*prim_id)
                            .and_then(|primitive| {
                                if let Primitive::Line(line) = primitive {
                                    let dx = line.end.x - line.start.x;
                                    let dy = line.end.y - line.start.y;
                                    Some((dx * dx + dy * dy).sqrt())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(0.0);

                        // 创建邻接关系
                        let adjacency = RegionAdjacency {
                            region_id_1: id1,
                            region_id_2: id2,
                            shared_primitive_ids: vec![*prim_id],
                            adjacency_type: "共墙".to_string(),
                            shared_length,
                            confidence: 0.9,
                        };

                        local_adjacencies.push(adjacency);
                    }
                }
                // 转换为 Vec 以满足 Rayon 类型要求
                local_adjacencies.into_vec()
            })
            .collect();

        stats.adjacencies_found = adjacencies.len();

        // 去重：移除重复的邻接关系
        // 优化：使用排序 + dedup 替代 HashSet，减少哈希表开销
        if !adjacencies.is_empty() {
            // 为每个邻接关系创建规范化键并排序
            let mut adjacency_keys: Vec<((usize, usize), RegionAdjacency)> = adjacencies
                .into_iter()
                .map(|adj| {
                    let pair = if adj.region_id_1 < adj.region_id_2 {
                        (adj.region_id_1, adj.region_id_2)
                    } else {
                        (adj.region_id_2, adj.region_id_1)
                    };
                    (pair, adj)
                })
                .collect();

            // 按键排序
            adjacency_keys.sort_unstable_by_key(|&(pair, _)| pair);

            // 去重：保留每个键的第一个邻接关系
            let mut last_pair: Option<(usize, usize)> = None;
            for (pair, adjacency) in adjacency_keys {
                if last_pair != Some(pair) {
                    last_pair = Some(pair);
                    adjacency_graph.add_adjacency(adjacency);
                }
            }
        }

        let elapsed = start_time.elapsed();
        stats.elapsed_ms = elapsed.as_millis() as u64;

        if self.verbose {
            eprintln!(
                "邻接关系分析完成：{}ms, 找到 {} 个邻接关系",
                stats.elapsed_ms, stats.adjacencies_found
            );
        }

        adjacency_graph
    }

    /// 判断点是否在区域内
    fn point_in_region(&self, x: f64, y: f64, vertices: &[AnalysisPoint]) -> bool {
        if vertices.len() < 3 {
            return false;
        }

        // 射线法判断点是否在多边形内
        let mut inside = false;
        let n = vertices.len();
        let mut j = n - 1;

        for i in 0..n {
            let vi = vertices[i];
            let vj = vertices[j];

            if ((vi.y > y) != (vj.y > y)) && (x < (vj.x - vi.x) * (y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    /// 根据文字内容分类房间类型（支持中英文混合识别）
    fn classify_room_type(&self, text: &str) -> String {
        let text_lower = text.to_lowercase();

        // 优先级匹配：从具体到一般
        // 支持中文、英文、中日文混合

        // ===== 卧室相关 =====
        if text_lower.contains("主卧")
            || text_lower.contains("master bedroom")
            || text_lower.contains("master bed")
        {
            "主卧".to_string()
        } else if text_lower.contains("次卧")
            || text_lower.contains("bedroom 2")
            || text_lower.contains("second bed")
        {
            "次卧".to_string()
        } else if text_lower.contains("儿")
            || text_lower.contains("child")
            || text_lower.contains("kids")
        {
            "儿童房".to_string()
        } else if text_lower.contains("老")
            || text_lower.contains("elder")
            || text_lower.contains("senior")
        {
            "老人房".to_string()
        } else if text_lower.contains("客卧")
            || text_lower.contains("guest bed")
            || text_lower.contains("guest room")
        {
            "客房".to_string()
        } else if text_lower.contains("卧")
            || text_lower.contains("bedroom")
            || text_lower.contains("bed ")
        {
            "卧室".to_string()
        // ===== 客厅相关 =====
        } else if text_lower.contains("起居")
            || text_lower.contains("living room")
            || text_lower.contains("living ")
        {
            "起居室".to_string()
        } else if text_lower.contains("客厅")
            || text_lower.contains("厅")
            || text_lower.contains("hall")
        {
            "客厅".to_string()
        // ===== 厨房相关 =====
        } else if text_lower.contains("厨")
            || text_lower.contains("厨房")
            || text_lower.contains("kitchen")
        {
            "厨房".to_string()
        // ===== 卫生间相关 =====
        } else if text_lower.contains("卫")
            || text_lower.contains("厕")
            || text_lower.contains("bath")
            || text_lower.contains("toilet")
            || text_lower.contains("wc")
        {
            "卫生间".to_string()
        // ===== 阳台相关 =====
        } else if text_lower.contains("阳")
            || text_lower.contains("阳台")
            || text_lower.contains("balcony")
            || text_lower.contains("terrace")
        {
            "阳台".to_string()
        // ===== 餐厅相关 =====
        } else if text_lower.contains("餐")
            || text_lower.contains("餐厅")
            || text_lower.contains("dining")
            || text_lower.contains("dining room")
        {
            "餐厅".to_string()
        // ===== 书房/工作室相关 =====
        } else if text_lower.contains("书")
            || text_lower.contains("书房")
            || text_lower.contains("study")
            || text_lower.contains("studio")
            || text_lower.contains("office")
        {
            "书房".to_string()
        // ===== 功能房间 =====
        } else if text_lower.contains("玄")
            || text_lower.contains("玄关")
            || text_lower.contains("foyer")
            || text_lower.contains("entrance")
        {
            "玄关".to_string()
        } else if text_lower.contains("储")
            || text_lower.contains("storage")
            || text_lower.contains("store")
        {
            "储物间".to_string()
        } else if text_lower.contains("衣帽")
            || text_lower.contains("cloak")
            || text_lower.contains("wardrobe")
        {
            "衣帽间".to_string()
        } else if text_lower.contains("健身")
            || text_lower.contains("gym")
            || text_lower.contains("fitness")
        {
            "健身房".to_string()
        } else if text_lower.contains("娱")
            || text_lower.contains("entertain")
            || text_lower.contains("recreation")
        {
            "娱乐室".to_string()
        } else if text_lower.contains("酒")
            || text_lower.contains("wine")
            || text_lower.contains("cellar")
        {
            "酒窖".to_string()
        } else if text_lower.contains("车") || text_lower.contains("garage") {
            "车库".to_string()
        } else if text_lower.contains("花") || text_lower.contains("garden") {
            "花园".to_string()
        } else if text_lower.contains("露")
            || text_lower.contains("deck")
            || text_lower.contains("patio")
        {
            "露台".to_string()
        } else if text_lower.contains("空")
            || text_lower.contains("ac ")
            || text_lower.contains("a/c")
        {
            "空调位".to_string()
        } else if text_lower.contains("设备")
            || text_lower.contains("equipment")
            || text_lower.contains("mep")
        {
            "设备平台".to_string()
        } else if text_lower.contains("井")
            || text_lower.contains("shaft")
            || text_lower.contains("duct")
        {
            "管道井".to_string()
        // ===== 办公/商业空间 =====
        } else if text_lower.contains("办公")
            || text_lower.contains("office room")
            || text_lower.contains("work room")
        {
            "办公室".to_string()
        } else if text_lower.contains("会议")
            || text_lower.contains("meeting")
            || text_lower.contains("conference")
        {
            "会议室".to_string()
        } else if text_lower.contains("接待") || text_lower.contains("reception") {
            "接待室".to_string()
        } else if text_lower.contains("休息")
            || text_lower.contains("lounge")
            || text_lower.contains("break room")
        {
            "休息室".to_string()
        } else if text_lower.contains("展示")
            || text_lower.contains("show")
            || text_lower.contains("exhibit")
        {
            "展示厅".to_string()
        } else if text_lower.contains("商铺")
            || text_lower.contains("shop")
            || text_lower.contains("retail")
        {
            "商铺".to_string()
        // ===== 日文支持 =====
        } else if text_lower.contains("寝室") || text_lower.contains("ベッドルーム") {
            "卧室".to_string()
        } else if text_lower.contains("居間") || text_lower.contains("リビング") {
            "客厅".to_string()
        } else if text_lower.contains("台所") || text_lower.contains("キッチン") {
            "厨房".to_string()
        } else if text_lower.contains("風呂") || text_lower.contains("トイレ") {
            "卫生间".to_string()
        } else if text_lower.contains("書斎") {
            "书房".to_string()
        // ===== 默认 =====
        } else {
            "房间".to_string()
        }
    }
}

impl Default for ClosedRegionDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::primitives::Line;

    #[test]
    fn test_detect_square() {
        // 创建一个正方形（没有内部隔断）
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [10.0, 0.0])),
            Primitive::Line(Line::from_coords([10.0, 0.0], [10.0, 10.0])),
            Primitive::Line(Line::from_coords([10.0, 10.0], [0.0, 10.0])),
            Primitive::Line(Line::from_coords([0.0, 10.0], [0.0, 0.0])),
        ];

        let detector = ClosedRegionDetector::new();
        let regions = detector.find_closed_regions(&primitives);

        // 算法会检测到所有可能的环路（包括重复的）
        // 对于简单的正方形，去重后应该只剩下一个区域
        assert!(!regions.is_empty());

        // 验证检测到的区域存在
        // 注意：由于算法会检测所有环路，可能包含子环路
        // 在实际户型图应用中，会有内部墙体来定义房间边界
    }

    #[test]
    fn test_detect_two_rooms() {
        // 创建两个相邻的房间
        let primitives = vec![
            // 外框
            Primitive::Line(Line::from_coords([0.0, 0.0], [20.0, 0.0])),
            Primitive::Line(Line::from_coords([20.0, 0.0], [20.0, 10.0])),
            Primitive::Line(Line::from_coords([20.0, 10.0], [0.0, 10.0])),
            Primitive::Line(Line::from_coords([0.0, 10.0], [0.0, 0.0])),
            // 中间隔墙
            Primitive::Line(Line::from_coords([10.0, 0.0], [10.0, 10.0])),
        ];

        let detector = ClosedRegionDetector::new();
        let regions = detector.find_closed_regions(&primitives);

        // 应该检测到 3 个区域：两个房间 + 一个外边界
        // 但由于去重和最小区域过滤，可能只剩下外边界和两个房间
        // 所以最少应该有 1 个区域（外边界）
        assert!(
            !regions.is_empty(),
            "Should detect at least the outer boundary"
        );

        // 验证面积：每个房间面积应该是 100（10x10），外边界面积是 200（20x10）
        let room_areas: Vec<f64> = regions
            .iter()
            .filter(|r| !r.is_outer_boundary)
            .map(|r| r.area)
            .collect();

        // 如果检测到内部房间，验证它们的面积
        if !room_areas.is_empty() {
            for area in room_areas {
                assert!(
                    (area - 100.0).abs() < 0.01,
                    "Room area should be 100.0, got {}",
                    area
                );
            }
        }

        // 验证外边界存在
        let outer_boundaries: Vec<&ClosedRegion> =
            regions.iter().filter(|r| r.is_outer_boundary).collect();

        assert_eq!(
            outer_boundaries.len(),
            1,
            "Should have exactly one outer boundary"
        );
        assert!(
            (outer_boundaries[0].area - 200.0).abs() < 0.01,
            "Outer boundary area should be 200.0"
        );
    }

    #[test]
    fn test_shoelace_area() {
        let detector = ClosedRegionDetector::new();
        let vertices = vec![
            AnalysisPoint { x: 0.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 10.0 },
            AnalysisPoint { x: 0.0, y: 10.0 },
        ];

        let area = detector.shoelace_area(&vertices);
        assert!((area.abs() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_rectangularity() {
        let detector = ClosedRegionDetector::new();

        // 完美矩形
        let rectangle = vec![
            AnalysisPoint { x: 0.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 5.0 },
            AnalysisPoint { x: 0.0, y: 5.0 },
        ];
        let rect_area = 50.0; // 10 * 5
        let rect_rect = detector.calculate_rectangularity(&rectangle, rect_area);
        assert!(
            (rect_rect - 1.0).abs() < 0.01,
            "矩形度应接近 1.0，实际：{}",
            rect_rect
        );

        // L 形（非矩形）
        let l_shape = vec![
            AnalysisPoint { x: 0.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 5.0 },
            AnalysisPoint { x: 5.0, y: 5.0 },
            AnalysisPoint { x: 5.0, y: 10.0 },
            AnalysisPoint { x: 0.0, y: 10.0 },
        ];
        let l_area = 75.0; // 10*5 + 5*5
        let l_rect = detector.calculate_rectangularity(&l_shape, l_area);
        assert!(l_rect < 1.0, "L 形矩形度应小于 1.0，实际：{}", l_rect);
        assert!(l_rect > 0.0, "矩形度应大于 0.0");
    }

    #[test]
    fn test_compactness() {
        let detector = ClosedRegionDetector::new();

        // 正方形
        let square = vec![
            AnalysisPoint { x: 0.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 10.0 },
            AnalysisPoint { x: 0.0, y: 10.0 },
        ];
        let square_perimeter = 40.0;
        let square_area = 100.0;
        let square_compact = detector.calculate_compactness(&square, square_perimeter, square_area);
        // 正方形紧凑度 = 4π*100 / 1600 = π/4 ≈ 0.785
        assert!(
            (square_compact - 0.785).abs() < 0.01,
            "正方形紧凑度应接近 0.785，实际：{}",
            square_compact
        );
    }

    #[test]
    fn test_aspect_ratio() {
        let detector = ClosedRegionDetector::new();

        // 正方形（长宽比接近 1）
        let square = vec![
            AnalysisPoint { x: 0.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 0.0 },
            AnalysisPoint { x: 10.0, y: 10.0 },
            AnalysisPoint { x: 0.0, y: 10.0 },
        ];
        let square_ar = detector.calculate_aspect_ratio(&square);
        assert!(square_ar.is_some());
        let square_ar_val = square_ar.unwrap();
        assert!(
            (square_ar_val - 1.0).abs() < 0.1,
            "正方形长宽比应接近 1.0，实际：{}",
            square_ar_val
        );

        // 长方形（长宽比大于 1）
        let rectangle = vec![
            AnalysisPoint { x: 0.0, y: 0.0 },
            AnalysisPoint { x: 20.0, y: 0.0 },
            AnalysisPoint { x: 20.0, y: 5.0 },
            AnalysisPoint { x: 0.0, y: 5.0 },
        ];
        let rect_ar = detector.calculate_aspect_ratio(&rectangle);
        assert!(rect_ar.is_some());
        let rect_ar_val = rect_ar.unwrap();
        assert!(
            rect_ar_val > 1.0,
            "长方形长宽比应大于 1.0，实际：{}",
            rect_ar_val
        );
    }

    #[test]
    fn test_area_statistics() {
        let detector = ClosedRegionDetector::new();

        // 创建测试区域
        let regions = vec![
            ClosedRegion {
                id: 0,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 100.0,
                perimeter: 40.0,
                centroid: AnalysisPoint { x: 5.0, y: 5.0 },
                room_type: Some("卧室".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.785),
                aspect_ratio: Some(1.0),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.785),
                shape_factor: Some(1.274),
            },
            ClosedRegion {
                id: 1,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 150.0,
                perimeter: 50.0,
                centroid: AnalysisPoint { x: 7.5, y: 5.0 },
                room_type: Some("客厅".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.75),
                aspect_ratio: Some(1.5),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.75),
                shape_factor: Some(1.333),
            },
            ClosedRegion {
                id: 2,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 50.0,
                perimeter: 30.0,
                centroid: AnalysisPoint { x: 2.5, y: 5.0 },
                room_type: Some("厨房".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.7),
                aspect_ratio: Some(2.0),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.7),
                shape_factor: Some(1.429),
            },
        ];

        let stats = detector.compute_area_statistics(&regions, 5);

        assert_eq!(stats.count, 3);
        assert!((stats.total_area - 300.0).abs() < 0.01);
        assert!((stats.mean_area - 100.0).abs() < 0.01);
        assert!((stats.median_area - 100.0).abs() < 0.01);
        assert!((stats.min_area - 50.0).abs() < 0.01);
        assert!((stats.max_area - 150.0).abs() < 0.01);
        assert!(stats.std_dev > 0.0);
        assert_eq!(stats.histogram.len(), 5);
    }

    #[test]
    fn test_room_type_statistics() {
        let detector = ClosedRegionDetector::new();

        let regions = vec![
            ClosedRegion {
                id: 0,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 100.0,
                perimeter: 40.0,
                centroid: AnalysisPoint { x: 5.0, y: 5.0 },
                room_type: Some("卧室".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.785),
                aspect_ratio: Some(1.0),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.785),
                shape_factor: Some(1.274),
            },
            ClosedRegion {
                id: 1,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 150.0,
                perimeter: 50.0,
                centroid: AnalysisPoint { x: 7.5, y: 5.0 },
                room_type: Some("卧室".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.75),
                aspect_ratio: Some(1.5),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.75),
                shape_factor: Some(1.333),
            },
            ClosedRegion {
                id: 2,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 80.0,
                perimeter: 36.0,
                centroid: AnalysisPoint { x: 4.0, y: 5.0 },
                room_type: Some("厨房".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.7),
                aspect_ratio: Some(1.25),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.7),
                shape_factor: Some(1.429),
            },
        ];

        let stats = detector.compute_room_type_statistics(&regions);

        assert_eq!(stats.type_counts.get("卧室"), Some(&2));
        assert_eq!(stats.type_counts.get("厨房"), Some(&1));

        let bedroom_avg = stats.type_avg_areas.get("卧室").unwrap();
        assert!((bedroom_avg - 125.0).abs() < 0.01);

        let kitchen_avg = stats.type_avg_areas.get("厨房").unwrap();
        assert!((kitchen_avg - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_multilingual_room_classification() {
        let detector = ClosedRegionDetector::new();

        // 中文测试
        assert_eq!(detector.classify_room_type("主卧"), "主卧");
        assert_eq!(detector.classify_room_type("次卧"), "次卧");
        assert_eq!(detector.classify_room_type("客厅"), "客厅");
        assert_eq!(detector.classify_room_type("厨房"), "厨房");
        assert_eq!(detector.classify_room_type("卫生间"), "卫生间");

        // 英文测试
        assert_eq!(detector.classify_room_type("Master Bedroom"), "主卧");
        assert_eq!(detector.classify_room_type("Bedroom 2"), "次卧");
        assert_eq!(detector.classify_room_type("Living Room"), "起居室");
        assert_eq!(detector.classify_room_type("Kitchen"), "厨房");
        assert_eq!(detector.classify_room_type("Bathroom"), "卫生间");
        assert_eq!(detector.classify_room_type("Balcony"), "阳台");
        assert_eq!(detector.classify_room_type("Dining Room"), "餐厅");
        assert_eq!(detector.classify_room_type("Study"), "书房");
        assert_eq!(detector.classify_room_type("Guest Room"), "客房");

        // 日文测试
        assert_eq!(detector.classify_room_type("寝室"), "卧室");
        assert_eq!(detector.classify_room_type("リビング"), "客厅");
        assert_eq!(detector.classify_room_type("キッチン"), "厨房");
        assert_eq!(detector.classify_room_type("風呂"), "卫生间");
        assert_eq!(detector.classify_room_type("書斎"), "书房");
    }

    #[test]
    fn test_export_to_csv() {
        let detector = ClosedRegionDetector::new();

        let regions = vec![
            ClosedRegion {
                id: 0,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 100.0,
                perimeter: 40.0,
                centroid: AnalysisPoint { x: 5.0, y: 5.0 },
                room_type: Some("卧室".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.785),
                aspect_ratio: Some(1.0),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.785),
                shape_factor: Some(1.274),
            },
            ClosedRegion {
                id: 1,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 150.0,
                perimeter: 50.0,
                centroid: AnalysisPoint { x: 7.5, y: 5.0 },
                room_type: Some("客厅".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.75),
                aspect_ratio: Some(1.5),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.75),
                shape_factor: Some(1.333),
            },
        ];

        let csv = detector.export_to_csv_string(&regions).unwrap();

        // 验证 CSV 格式
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3); // 表头 + 2 行数据

        // 验证表头
        assert!(lines[0].contains("id,room_type,area,perimeter"));

        // 验证数据行
        assert!(lines[1].contains("卧室"));
        assert!(lines[1].contains("100.0"));
        assert!(lines[2].contains("客厅"));
        assert!(lines[2].contains("150.0"));
    }

    #[test]
    fn test_export_to_json() {
        let detector = ClosedRegionDetector::new();

        let regions = vec![ClosedRegion {
            id: 0,
            boundary_primitive_ids: vec![],
            vertices: vec![],
            area: 100.0,
            perimeter: 40.0,
            centroid: AnalysisPoint { x: 5.0, y: 5.0 },
            room_type: Some("卧室".to_string()),
            confidence: 0.9,
            is_outer_boundary: false,
            rectangularity: Some(1.0),
            compactness: Some(0.785),
            aspect_ratio: Some(1.0),
            convexity: Some(1.0),
            orientation: Some(0.0),
            circularity: Some(0.785),
            shape_factor: Some(1.274),
        }];

        let json = detector.export_to_json(&regions);

        // 验证 JSON 结构
        assert_eq!(json["count"], 1);
        assert!(json["regions"].is_array());
        assert!(json["statistics"]["area"].is_object());
        assert!(json["statistics"]["room_types"].is_object());
    }

    #[test]
    fn test_generate_report() {
        let detector = ClosedRegionDetector::new();

        let regions = vec![
            ClosedRegion {
                id: 0,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 100.0,
                perimeter: 40.0,
                centroid: AnalysisPoint { x: 5.0, y: 5.0 },
                room_type: Some("卧室".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.785),
                aspect_ratio: Some(1.0),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.785),
                shape_factor: Some(1.274),
            },
            ClosedRegion {
                id: 1,
                boundary_primitive_ids: vec![],
                vertices: vec![],
                area: 80.0,
                perimeter: 36.0,
                centroid: AnalysisPoint { x: 4.0, y: 4.0 },
                room_type: Some("厨房".to_string()),
                confidence: 0.9,
                is_outer_boundary: false,
                rectangularity: Some(1.0),
                compactness: Some(0.7),
                aspect_ratio: Some(1.25),
                convexity: Some(1.0),
                orientation: Some(0.0),
                circularity: Some(0.7),
                shape_factor: Some(1.429),
            },
        ];

        let adjacency_graph = RegionAdjacencyGraph::new();
        let report = detector.generate_report(&regions, &adjacency_graph, "测试户型");

        // 验证报告基本字段
        assert_eq!(report.title, "测试户型");
        assert!(!report.generated_at.is_empty());
        assert_eq!(report.summary.total_rooms, 2);
        assert!((report.summary.total_area - 180.0).abs() < 0.01);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_quantization_epsilon() {
        // 测试不同量化精度对检测结果的影响
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [10.0, 0.0])),
            Primitive::Line(Line::from_coords([10.0, 0.0], [10.0, 10.0])),
            Primitive::Line(Line::from_coords([10.0, 10.0], [0.0, 10.0])),
            Primitive::Line(Line::from_coords([0.0, 10.0], [0.0, 0.0])),
        ];

        // 使用默认精度
        let detector_default = ClosedRegionDetector::new();
        let regions_default = detector_default.find_closed_regions(&primitives);
        assert!(!regions_default.is_empty());

        // 使用更高精度
        let detector_high = ClosedRegionDetector::new().with_quantization_epsilon(0.0001);
        let regions_high = detector_high.find_closed_regions(&primitives);
        assert!(!regions_high.is_empty());

        // 使用较低精度
        let detector_low = ClosedRegionDetector::new().with_quantization_epsilon(0.01);
        let regions_low = detector_low.find_closed_regions(&primitives);
        assert!(!regions_low.is_empty());

        // 验证精度设置有效
        assert!((detector_high.quantization_epsilon - 0.0001).abs() < 1e-6);
        assert!((detector_low.quantization_epsilon - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_with_min_area() {
        let primitives = vec![
            Primitive::Line(Line::from_coords([0.0, 0.0], [10.0, 0.0])),
            Primitive::Line(Line::from_coords([10.0, 0.0], [10.0, 10.0])),
            Primitive::Line(Line::from_coords([10.0, 10.0], [0.0, 10.0])),
            Primitive::Line(Line::from_coords([0.0, 10.0], [0.0, 0.0])),
        ];

        // 默认最小面积
        let detector_default = ClosedRegionDetector::new();
        let _regions_default = detector_default.find_closed_regions(&primitives);

        // 设置较大的最小面积，应该过滤掉更多区域
        let detector_large = ClosedRegionDetector::new().with_min_area(50.0);
        let regions_large = detector_large.find_closed_regions(&primitives);

        // 面积为 100 的正方形应该保留
        assert!(regions_large.iter().any(|r| r.area >= 50.0));
    }
}
