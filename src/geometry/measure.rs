//! 几何测量工具
#![allow(clippy::cast_possible_truncation)]
//!
//! 提供长度、面积、角度等测量功能，支持缓存优化

#![allow(clippy::cast_precision_loss)]

use super::geometry_cache::{CacheKey, CacheStats, GeometryCache};
use crate::error::GeometryConfig;
use crate::geometry::{Circle, Line, Point, Polygon, Rect};
use std::time::Duration;

/// 宏：对所有缓存执行 clear 操作
///
/// 用于简化重复的 `if let Some(ref mut cache)` 模式
macro_rules! clear_all_caches {
    ($self:ident) => {
        if let Some(ref mut cache) = $self.length_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.area_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.angle_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.parallel_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.perpendicular_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.rect_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.circle_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.perimeter_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.point_line_dist_cache {
            cache.clear();
        }
        if let Some(ref mut cache) = $self.midpoint_cache {
            cache.clear();
        }
    };
}

/// 宏：对所有缓存执行操作并累加结果
///
/// 用于简化重复的 `if let Some(ref cache) { stats += cache.stats(); }` 模式
macro_rules! for_each_cache_accumulate {
    ($self:ident, $stats:ident, $field:ident) => {
        if let Some(ref cache) = $self.length_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.area_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.angle_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.parallel_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.perpendicular_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.rect_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.circle_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.perimeter_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.point_line_dist_cache {
            $stats.$field += cache.stats().$field;
        }
        if let Some(ref cache) = $self.midpoint_cache {
            $stats.$field += cache.stats().$field;
        }
    };
}

/// 几何测量工具集
///
/// # 使用示例
///
/// ```rust
/// use cadagent::geometry::GeometryMeasurer;
///
/// // 使用默认配置
/// let measurer = GeometryMeasurer::new();
///
/// // 使用自定义配置（Builder 模式）
/// let measurer = GeometryMeasurer::builder()
///     .angle_tolerance(0.01)  // 弧度
///     .distance_tolerance(0.01)
///     .build();
///
/// // 启用缓存
/// let mut measurer = GeometryMeasurer::builder()
///     .angle_tolerance(0.01)
///     .enable_cache(true)
///     .cache_capacity(1000)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct GeometryMeasurer {
    /// 几何配置（包含角度和距离容差）
    config: GeometryConfig,
    /// 长度测量缓存
    length_cache: Option<GeometryCache<f64>>,
    /// 面积测量缓存
    area_cache: Option<GeometryCache<f64>>,
    /// 角度测量缓存
    angle_cache: Option<GeometryCache<f64>>,
    /// 平行检查缓存
    parallel_cache: Option<GeometryCache<ParallelResult>>,
    /// 垂直检查缓存
    perpendicular_cache: Option<GeometryCache<PerpendicularResult>>,
    /// 矩形测量缓存
    rect_cache: Option<GeometryCache<RectDimensions>>,
    /// 圆测量缓存
    circle_cache: Option<GeometryCache<CircleDimensions>>,
    /// 周长测量缓存
    perimeter_cache: Option<GeometryCache<f64>>,
    /// 点到直线距离缓存
    point_line_dist_cache: Option<GeometryCache<f64>>,
    /// 中点缓存
    midpoint_cache: Option<GeometryCache<[f64; 2]>>,
    /// 是否启用缓存
    cache_enabled: bool,
}

/// `GeometryMeasurer` 的 Builder 模式实现
///
/// # 使用示例
///
/// ```rust
/// use cadagent::geometry::GeometryMeasurer;
///
/// let measurer = GeometryMeasurer::builder()
///     .angle_tolerance(0.01)
///     .distance_tolerance(0.01)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct GeometryMeasurerBuilder {
    angle_tolerance: Option<f64>,
    distance_tolerance: Option<f64>,
    min_confidence: Option<f64>,
    normalize_range: Option<[f64; 2]>,
    enable_normalization: Option<bool>,
    /// 缓存配置
    enable_cache: Option<bool>,
    cache_capacity: Option<usize>,
    cache_expiration: Option<Duration>,
}

impl GeometryMeasurerBuilder {
    /// 设置角度容差（弧度）
    pub fn angle_tolerance(mut self, tolerance: f64) -> Self {
        self.angle_tolerance = Some(tolerance);
        self
    }

    /// 设置距离容差
    pub fn distance_tolerance(mut self, tolerance: f64) -> Self {
        self.distance_tolerance = Some(tolerance);
        self
    }

    /// 设置最小置信度
    pub fn min_confidence(mut self, confidence: f64) -> Self {
        self.min_confidence = Some(confidence);
        self
    }

    /// 设置归一化范围
    pub fn normalize_range(mut self, range: [f64; 2]) -> Self {
        self.normalize_range = Some(range);
        self
    }

    /// 设置是否启用归一化
    pub fn enable_normalization(mut self, enable: bool) -> Self {
        self.enable_normalization = Some(enable);
        self
    }

    /// 启用或禁用缓存
    pub fn enable_cache(mut self, enable: bool) -> Self {
        self.enable_cache = Some(enable);
        self
    }

    /// 设置缓存容量（0 表示无限制）
    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = Some(capacity);
        self
    }

    /// 设置缓存过期时间（秒）
    pub fn cache_expiration_secs(mut self, secs: u64) -> Self {
        self.cache_expiration = Some(Duration::from_secs(secs));
        self
    }

    /// 设置缓存过期时间
    pub fn cache_expiration(mut self, duration: Duration) -> Self {
        self.cache_expiration = Some(duration);
        self
    }

    /// 构建 `GeometryMeasurer`
    pub fn build(self) -> GeometryMeasurer {
        let mut config = GeometryConfig::default();

        if let Some(tolerance) = self.angle_tolerance {
            config.angle_tolerance = tolerance;
        }
        if let Some(tolerance) = self.distance_tolerance {
            config.distance_tolerance = tolerance;
        }
        if let Some(confidence) = self.min_confidence {
            config.min_confidence = confidence;
        }
        if let Some(range) = self.normalize_range {
            config.normalize_range = range;
        }
        if let Some(enable) = self.enable_normalization {
            config.enable_normalization = enable;
        }

        let cache_enabled = self.enable_cache.unwrap_or(false);
        let cache_capacity = self.cache_capacity.unwrap_or(1000);
        let cache_expiration = self.cache_expiration;

        GeometryMeasurer {
            config,
            length_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            area_cache: cache_enabled.then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            angle_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            parallel_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            perpendicular_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            rect_cache: cache_enabled.then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            circle_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            perimeter_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            point_line_dist_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            midpoint_cache: cache_enabled
                .then(|| GeometryCache::new(cache_capacity, cache_expiration)),
            cache_enabled,
        }
    }
}

impl GeometryMeasurer {
    /// 使用默认配置创建测量器（缓存禁用）
    pub fn new() -> Self {
        Self {
            config: GeometryConfig::default(),
            length_cache: None,
            area_cache: None,
            angle_cache: None,
            parallel_cache: None,
            perpendicular_cache: None,
            rect_cache: None,
            circle_cache: None,
            perimeter_cache: None,
            point_line_dist_cache: None,
            midpoint_cache: None,
            cache_enabled: false,
        }
    }

    /// 使用自定义配置创建测量器（缓存禁用）
    ///
    /// # 建议
    /// 对于链式调用，建议使用 `builder()` 方法
    pub fn with_config(config: GeometryConfig) -> Self {
        Self {
            config,
            length_cache: None,
            area_cache: None,
            angle_cache: None,
            parallel_cache: None,
            perpendicular_cache: None,
            rect_cache: None,
            circle_cache: None,
            perimeter_cache: None,
            point_line_dist_cache: None,
            midpoint_cache: None,
            cache_enabled: false,
        }
    }

    /// 创建 Builder 用于链式配置
    ///
    /// # 使用示例
    ///
    /// ```rust
    /// use cadagent::geometry::GeometryMeasurer;
    ///
    /// let measurer = GeometryMeasurer::builder()
    ///     .angle_tolerance(0.01)
    ///     .distance_tolerance(0.01)
    ///     .enable_cache(true)
    ///     .cache_capacity(1000)
    ///     .build();
    /// ```
    pub fn builder() -> GeometryMeasurerBuilder {
        GeometryMeasurerBuilder::default()
    }

    /// 启用缓存
    pub fn enable_cache(&mut self, capacity: usize, expiration: Option<Duration>) {
        self.cache_enabled = true;
        self.length_cache = Some(GeometryCache::new(capacity, expiration));
        self.area_cache = Some(GeometryCache::new(capacity, expiration));
        self.angle_cache = Some(GeometryCache::new(capacity, expiration));
        self.parallel_cache = Some(GeometryCache::new(capacity, expiration));
        self.perpendicular_cache = Some(GeometryCache::new(capacity, expiration));
        self.rect_cache = Some(GeometryCache::new(capacity, expiration));
        self.circle_cache = Some(GeometryCache::new(capacity, expiration));
        self.perimeter_cache = Some(GeometryCache::new(capacity, expiration));
        self.point_line_dist_cache = Some(GeometryCache::new(capacity, expiration));
        self.midpoint_cache = Some(GeometryCache::new(capacity, expiration));
    }

    /// 禁用缓存并清空
    pub fn disable_cache(&mut self) {
        self.cache_enabled = false;
        self.length_cache = None;
        self.area_cache = None;
        self.angle_cache = None;
        self.parallel_cache = None;
        self.perpendicular_cache = None;
        self.rect_cache = None;
        self.circle_cache = None;
        self.perimeter_cache = None;
        self.point_line_dist_cache = None;
        self.midpoint_cache = None;
    }

    /// 检查缓存是否启用
    pub fn is_cache_enabled(&self) -> bool {
        self.cache_enabled
    }

    /// 清空所有缓存
    pub fn clear_cache(&mut self) {
        clear_all_caches!(self);
    }

    /// 获取缓存统计信息
    pub fn cache_stats(&self) -> CacheStats {
        let mut total_stats = CacheStats::new();

        // 使用宏累加所有缓存的统计信息
        for_each_cache_accumulate!(self, total_stats, hits);
        for_each_cache_accumulate!(self, total_stats, misses);
        for_each_cache_accumulate!(self, total_stats, evictions);
        for_each_cache_accumulate!(self, total_stats, total_computation_time_ms);

        total_stats
    }

    /// 重置缓存统计
    pub fn reset_cache_stats(&mut self) {
        if let Some(ref mut cache) = self.length_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.area_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.angle_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.parallel_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.perpendicular_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.rect_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.circle_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.perimeter_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.point_line_dist_cache {
            cache.reset_stats();
        }
        if let Some(ref mut cache) = self.midpoint_cache {
            cache.reset_stats();
        }
    }

    /// 获取角度容差（度）
    #[inline]
    fn angle_tolerance_degrees(&self) -> f64 {
        self.config.angle_tolerance.to_degrees()
    }

    /// 获取角度容差（弧度）
    #[inline]
    #[allow(dead_code)]
    fn angle_tolerance_radians(&self) -> f64 {
        self.config.angle_tolerance
    }

    /// 获取距离容差
    #[inline]
    #[allow(dead_code)]
    fn distance_tolerance(&self) -> f64 {
        self.config.distance_tolerance
    }

    /// 测量两点之间的线段长度
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn measure_length(&mut self, start: [f64; 2], end: [f64; 2]) -> f64 {
        // 量化坐标用于缓存键（精度 0.01）
        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::Length {
            start: (quantize(start[0]), quantize(start[1])),
            end: (quantize(end[0]), quantize(end[1])),
        };

        if let Some(ref mut cache) = self.length_cache {
            return cache.get_or_insert(key, || {
                let p1 = Point::from_array(start);
                let p2 = Point::from_array(end);
                p1.distance(&p2)
            });
        }

        // 无缓存时直接计算
        let p1 = Point::from_array(start);
        let p2 = Point::from_array(end);
        p1.distance(&p2)
    }

    /// 计算多边形面积（使用鞋带公式）
    ///
    /// # 缓存支持
    /// 缓存基于顶点哈希，相同顶点的多边形将返回缓存结果
    pub fn measure_area(&mut self, vertices: Vec<[f64; 2]>) -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // 计算顶点哈希
        let mut hasher = DefaultHasher::new();
        for v in &vertices {
            let quantized = ((v[0] * 100.0).round() as i64, (v[1] * 100.0).round() as i64);
            quantized.hash(&mut hasher);
        }
        let vertices_hash = hasher.finish();

        let key = CacheKey::Area {
            vertices_hash,
            vertex_count: vertices.len(),
        };

        if let Some(ref mut cache) = self.area_cache {
            return cache.get_or_insert(key, || {
                let polygon = Polygon::from_coords(vertices);
                polygon.area()
            });
        }

        let polygon = Polygon::from_coords(vertices);
        polygon.area()
    }
}

impl Default for GeometryMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryMeasurer {
    /// 测量三点形成的角度（以度为单位）
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn measure_angle(&mut self, p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]) -> f64 {
        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::Angle {
            p1: (quantize(p1[0]), quantize(p1[1])),
            p2: (quantize(p2[0]), quantize(p2[1])),
            p3: (quantize(p3[0]), quantize(p3[1])),
        };

        if let Some(ref mut cache) = self.angle_cache {
            return cache.get_or_insert(key, || Self::compute_angle(p1, p2, p3));
        }

        Self::compute_angle(p1, p2, p3)
    }

    /// 计算角度的内部实现
    fn compute_angle(p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]) -> f64 {
        let v1 = (p1[0] - p2[0], p1[1] - p2[1]);
        let v2 = (p3[0] - p2[0], p3[1] - p2[1]);

        let dot = v1.0 * v2.0 + v1.1 * v2.1;
        let mag1 = (v1.0.powi(2) + v1.1.powi(2)).sqrt();
        let mag2 = (v2.0.powi(2) + v2.1.powi(2)).sqrt();

        if mag1 == 0.0 || mag2 == 0.0 {
            return 0.0;
        }

        let cos_angle = dot / (mag1 * mag2);
        let cos_angle = cos_angle.clamp(-1.0, 1.0);
        let angle_rad = cos_angle.acos();
        angle_rad.to_degrees()
    }

    /// 检查两条线段是否平行
    ///
    /// 使用配置中的角度容差（`GeometryConfig.angle_tolerance`）
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn check_parallel(
        &mut self,
        line1_start: [f64; 2],
        line1_end: [f64; 2],
        line2_start: [f64; 2],
        line2_end: [f64; 2],
    ) -> ParallelResult {
        // 验证输入坐标是否有效（非 NaN/Infinity）
        if !is_valid_coordinate(line1_start)
            || !is_valid_coordinate(line1_end)
            || !is_valid_coordinate(line2_start)
            || !is_valid_coordinate(line2_end)
        {
            return ParallelResult {
                is_parallel: false,
                angle_diff: f64::NAN,
            };
        }

        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::Parallel {
            line1_start: (quantize(line1_start[0]), quantize(line1_start[1])),
            line1_end: (quantize(line1_end[0]), quantize(line1_end[1])),
            line2_start: (quantize(line2_start[0]), quantize(line2_start[1])),
            line2_end: (quantize(line2_end[0]), quantize(line2_end[1])),
        };

        // 提前获取容差值，避免借用冲突
        let angle_tolerance_deg = self.angle_tolerance_degrees();

        if let Some(ref mut cache) = self.parallel_cache {
            return cache.get_or_insert(key, || {
                Self::compute_parallel(
                    line1_start,
                    line1_end,
                    line2_start,
                    line2_end,
                    angle_tolerance_deg,
                )
            });
        }

        Self::compute_parallel(
            line1_start,
            line1_end,
            line2_start,
            line2_end,
            angle_tolerance_deg,
        )
    }

    /// 计算平行检查的内部实现
    fn compute_parallel(
        line1_start: [f64; 2],
        line1_end: [f64; 2],
        line2_start: [f64; 2],
        line2_end: [f64; 2],
        angle_tolerance_deg: f64,
    ) -> ParallelResult {
        let line1 = Line::from_coords(line1_start, line1_end);
        let line2 = Line::from_coords(line2_start, line2_end);

        let dir1 = line1.direction();
        let dir2 = line2.direction();

        // 计算方向向量的叉积
        let cross = dir1.x * dir2.y - dir1.y * dir2.x;

        //  clamp 到 [-1, 1] 避免 asin 域错误
        let cross_clamped = cross.clamp(-1.0, 1.0);
        let angle_diff = cross_clamped.abs().asin().to_degrees();

        ParallelResult {
            is_parallel: angle_diff < angle_tolerance_deg,
            angle_diff,
        }
    }

    /// 检查两条线段是否垂直
    ///
    /// 使用配置中的角度容差（`GeometryConfig.angle_tolerance`）
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn check_perpendicular(
        &mut self,
        line1_start: [f64; 2],
        line1_end: [f64; 2],
        line2_start: [f64; 2],
        line2_end: [f64; 2],
    ) -> PerpendicularResult {
        // 验证输入坐标是否有效（非 NaN/Infinity）
        if !is_valid_coordinate(line1_start)
            || !is_valid_coordinate(line1_end)
            || !is_valid_coordinate(line2_start)
            || !is_valid_coordinate(line2_end)
        {
            return PerpendicularResult {
                is_perpendicular: false,
                angle_diff: f64::NAN,
            };
        }

        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::Perpendicular {
            line1_start: (quantize(line1_start[0]), quantize(line1_start[1])),
            line1_end: (quantize(line1_end[0]), quantize(line1_end[1])),
            line2_start: (quantize(line2_start[0]), quantize(line2_start[1])),
            line2_end: (quantize(line2_end[0]), quantize(line2_end[1])),
        };

        // 提前获取容差值，避免借用冲突
        let angle_tolerance_deg = self.angle_tolerance_degrees();

        if let Some(ref mut cache) = self.perpendicular_cache {
            return cache.get_or_insert(key, || {
                Self::compute_perpendicular(
                    line1_start,
                    line1_end,
                    line2_start,
                    line2_end,
                    angle_tolerance_deg,
                )
            });
        }

        Self::compute_perpendicular(
            line1_start,
            line1_end,
            line2_start,
            line2_end,
            angle_tolerance_deg,
        )
    }

    /// 计算垂直检查的内部实现
    fn compute_perpendicular(
        line1_start: [f64; 2],
        line1_end: [f64; 2],
        line2_start: [f64; 2],
        line2_end: [f64; 2],
        angle_tolerance_deg: f64,
    ) -> PerpendicularResult {
        let line1 = Line::from_coords(line1_start, line1_end);
        let line2 = Line::from_coords(line2_start, line2_end);

        let dir1 = line1.direction();
        let dir2 = line2.direction();

        // 计算方向向量的点积
        let dot = dir1.x * dir2.x + dir1.y * dir2.y;
        let mag1 = (dir1.x.powi(2) + dir1.y.powi(2)).sqrt();
        let mag2 = (dir2.x.powi(2) + dir2.y.powi(2)).sqrt();

        // 避免除以零
        if mag1 == 0.0 || mag2 == 0.0 {
            return PerpendicularResult {
                is_perpendicular: false,
                angle_diff: f64::NAN,
            };
        }

        // 计算归一化点积并 clamp 到 [-1, 1] 避免 acos 域错误
        let cos_angle = (dot / (mag1 * mag2)).clamp(-1.0, 1.0);
        let angle_diff = (90.0 - cos_angle.abs().acos().to_degrees()).abs();

        PerpendicularResult {
            is_perpendicular: angle_diff < angle_tolerance_deg,
            angle_diff,
        }
    }

    /// 计算矩形的宽度和高度
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn measure_rect(&mut self, min: [f64; 2], max: [f64; 2]) -> RectDimensions {
        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::Rect {
            min: (quantize(min[0]), quantize(min[1])),
            max: (quantize(max[0]), quantize(max[1])),
        };

        if let Some(ref mut cache) = self.rect_cache {
            return cache.get_or_insert(key, || {
                let rect = Rect::from_coords(min, max);
                RectDimensions {
                    width: rect.width(),
                    height: rect.height(),
                    area: rect.area(),
                    center: rect.center().to_array(),
                }
            });
        }

        let rect = Rect::from_coords(min, max);
        RectDimensions {
            width: rect.width(),
            height: rect.height(),
            area: rect.area(),
            center: rect.center().to_array(),
        }
    }

    /// 计算圆的面积和周长
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn measure_circle(&mut self, center: [f64; 2], radius: f64) -> CircleDimensions {
        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::Circle {
            center: (quantize(center[0]), quantize(center[1])),
            radius_quantized: quantize(radius),
        };

        if let Some(ref mut cache) = self.circle_cache {
            return cache.get_or_insert(key, || {
                let circle = Circle::from_coords(center, radius);
                CircleDimensions {
                    radius: circle.radius,
                    diameter: circle.diameter(),
                    area: circle.area(),
                    circumference: circle.circumference(),
                }
            });
        }

        let circle = Circle::from_coords(center, radius);
        CircleDimensions {
            radius: circle.radius,
            diameter: circle.diameter(),
            area: circle.area(),
            circumference: circle.circumference(),
        }
    }

    /// 计算多边形的周长
    ///
    /// # 缓存支持
    /// 缓存基于顶点哈希，相同顶点的多边形将返回缓存结果
    pub fn measure_perimeter(&mut self, vertices: Vec<[f64; 2]>) -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // 计算顶点哈希
        let mut hasher = DefaultHasher::new();
        for v in &vertices {
            let quantized = ((v[0] * 100.0).round() as i64, (v[1] * 100.0).round() as i64);
            quantized.hash(&mut hasher);
        }
        let vertices_hash = hasher.finish();

        let key = CacheKey::Perimeter {
            vertices_hash,
            vertex_count: vertices.len(),
        };

        if let Some(ref mut cache) = self.perimeter_cache {
            return cache.get_or_insert(key, || {
                let polygon = Polygon::from_coords(vertices);
                polygon.perimeter()
            });
        }

        let polygon = Polygon::from_coords(vertices);
        polygon.perimeter()
    }

    /// 计算点到直线的距离
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn point_to_line_distance(
        &mut self,
        point: [f64; 2],
        line_start: [f64; 2],
        line_end: [f64; 2],
    ) -> f64 {
        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::PointLineDistance {
            point: (quantize(point[0]), quantize(point[1])),
            line_start: (quantize(line_start[0]), quantize(line_start[1])),
            line_end: (quantize(line_end[0]), quantize(line_end[1])),
        };

        if let Some(ref mut cache) = self.point_line_dist_cache {
            return cache.get_or_insert(key, || {
                Self::compute_point_line_distance(point, line_start, line_end)
            });
        }

        Self::compute_point_line_distance(point, line_start, line_end)
    }

    /// 计算点到直线距离的内部实现
    fn compute_point_line_distance(
        point: [f64; 2],
        line_start: [f64; 2],
        line_end: [f64; 2],
    ) -> f64 {
        let p = Point::from_array(point);
        // 使用 unchecked 版本以支持零长度线段的边缘情况处理
        let line = Line::from_coords_unchecked(line_start, line_end);

        let a = (p.x - line.start.x) * (line.end.y - line.start.y)
            - (p.y - line.start.y) * (line.end.x - line.start.x);
        let b = line.length();

        if b == 0.0 {
            return p.distance(&line.start);
        }

        a.abs() / b
    }

    /// 计算两个点之间的中点
    ///
    /// # 缓存支持
    /// 如果启用缓存，重复的测量请求将直接返回缓存结果
    pub fn midpoint(&mut self, p1: [f64; 2], p2: [f64; 2]) -> [f64; 2] {
        let quantize = |v: f64| (v * 100.0).round() as i64;
        let key = CacheKey::Midpoint {
            p1: (quantize(p1[0]), quantize(p1[1])),
            p2: (quantize(p2[0]), quantize(p2[1])),
        };

        if let Some(ref mut cache) = self.midpoint_cache {
            return cache.get_or_insert(key, || Self::compute_midpoint(p1, p2));
        }

        Self::compute_midpoint(p1, p2)
    }

    /// 计算中点的内部实现
    fn compute_midpoint(p1: [f64; 2], p2: [f64; 2]) -> [f64; 2] {
        // 使用 unchecked 版本以支持重合点的边缘情况
        let line = Line::from_coords_unchecked(p1, p2);
        line.midpoint().to_array()
    }
}

/// 平行检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParallelResult {
    pub is_parallel: bool,
    pub angle_diff: f64,
}

/// 垂直检查结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerpendicularResult {
    pub is_perpendicular: bool,
    pub angle_diff: f64,
}

/// 矩形尺寸
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RectDimensions {
    pub width: f64,
    pub height: f64,
    pub area: f64,
    pub center: [f64; 2],
}

/// 圆尺寸
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CircleDimensions {
    pub radius: f64,
    pub diameter: f64,
    pub area: f64,
    pub circumference: f64,
}

// 重新导出便捷类型
pub use CircleDimensions as MeasureCircleResult;
pub use ParallelResult as MeasureParallelResult;
pub use PerpendicularResult as MeasurePerpendicularResult;
pub use RectDimensions as MeasureRectResult;

/// 检查坐标是否有效（非 NaN/Infinity）
fn is_valid_coordinate(coord: [f64; 2]) -> bool {
    coord[0].is_finite() && coord[1].is_finite()
}

// ============================================================================
// 高级几何算法
// ============================================================================

/// 线段交点结果
#[derive(Debug, Clone, PartialEq)]
pub enum LineIntersection {
    /// 无交点（平行或不相交）
    None,
    /// 单个交点
    Single([f64; 2]),
    /// 重合（无限多个交点）
    Coincident,
}

/// 计算两条线段的交点
///
/// # 参数
///
/// * `p1`, `p2` - 第一条线段的端点
/// * `p3`, `p4` - 第二条线段的端点
///
/// # 返回
///
/// * `LineIntersection::None` - 无交点（平行或线段不相交）
/// * `LineIntersection::Single([x, y])` - 单个交点坐标
/// * `LineIntersection::Coincident` - 线段重合
///
/// # 算法
///
/// 使用参数方程求解：
/// - 线段 1: P = p1 + t * (p2 - p1), t ∈ [0, 1]
/// - 线段 2: P = p3 + u * (p4 - p3), u ∈ [0, 1]
///
/// # 示例
///
/// ```rust,ignore
/// use cadagent::geometry::{line_intersection, LineIntersection};
///
/// // 相交线段
/// let result = line_intersection(
///     [0.0, 0.0], [2.0, 2.0],
///     [0.0, 2.0], [2.0, 0.0]
/// );
/// assert!(matches!(result, LineIntersection::Single([1.0, 1.0])));
///
/// // 平行线段
/// let result = line_intersection(
///     [0.0, 0.0], [1.0, 0.0],
///     [0.0, 1.0], [1.0, 1.0]
/// );
/// assert!(matches!(result, LineIntersection::None));
/// ```
pub fn line_intersection(
    p1: [f64; 2],
    p2: [f64; 2],
    p3: [f64; 2],
    p4: [f64; 2],
) -> LineIntersection {
    let x1 = p1[0];
    let y1 = p1[1];
    let x2 = p2[0];
    let y2 = p2[1];
    let x3 = p3[0];
    let y3 = p3[1];
    let x4 = p4[0];
    let y4 = p4[1];

    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);

    // 检查是否平行
    if denom.abs() < 1e-10 {
        // 检查是否重合：检查 p3 是否在直线 p1-p2 上
        let cross = (x3 - x1) * (y2 - y1) - (y3 - y1) * (x2 - x1);
        if cross.abs() < 1e-10 {
            return LineIntersection::Coincident;
        }
        return LineIntersection::None;
    }

    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
    let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / denom;

    // 检查交点是否在线段上
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        let x = x1 + t * (x2 - x1);
        let y = y1 + t * (y2 - y1);
        LineIntersection::Single([x, y])
    } else {
        LineIntersection::None
    }
}

/// 计算点到线段的最近点
///
/// # 参数
///
/// * `point` - 目标点
/// * `line_start`, `line_end` - 线段的两个端点
///
/// # 返回
///
/// 返回线段上距离目标点最近的点的坐标
///
/// # 算法
///
/// 使用向量投影：
/// 1. 计算从 `line_start` 到 point 的向量
/// 2. 投影到线段方向向量上
/// 3. 限制参数 t 在 [0, 1] 范围内
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::closest_point_on_segment;
///
/// // 点在线段延长线上
/// let closest = closest_point_on_segment([5.0, 0.0], [0.0, 0.0], [10.0, 0.0]);
/// assert_eq!(closest, [5.0, 0.0]);
///
/// // 点在线段外
/// let closest = closest_point_on_segment([5.0, 5.0], [0.0, 0.0], [10.0, 0.0]);
/// assert_eq!(closest, [5.0, 0.0]);
/// ```
pub fn closest_point_on_segment(
    point: [f64; 2],
    line_start: [f64; 2],
    line_end: [f64; 2],
) -> [f64; 2] {
    let dx = line_end[0] - line_start[0];
    let dy = line_end[1] - line_start[1];

    // 线段长度为 0，返回起点
    if dx == 0.0 && dy == 0.0 {
        return line_start;
    }

    // 计算投影参数 t
    let t =
        ((point[0] - line_start[0]) * dx + (point[1] - line_start[1]) * dy) / (dx * dx + dy * dy);

    // 限制 t 在 [0, 1] 范围内
    let t = t.clamp(0.0, 1.0);

    [line_start[0] + t * dx, line_start[1] + t * dy]
}

/// 计算点到线段的距离
///
/// # 参数
///
/// * `point` - 目标点
/// * `line_start`, `line_end` - 线段的两个端点
///
/// # 返回
///
/// 返回点到线段的最短距离
///
/// # 示例
///
/// ```rust,ignore
/// use cadagent::geometry::point_to_segment_distance;
///
/// let dist = point_to_segment_distance([0.0, 3.0], [0.0, 0.0], [4.0, 0.0]);
/// assert!((dist - 3.0).abs() < 1e-6);
/// ```
pub fn point_to_segment_distance(point: [f64; 2], line_start: [f64; 2], line_end: [f64; 2]) -> f64 {
    let closest = closest_point_on_segment(point, line_start, line_end);
    let dx = point[0] - closest[0];
    let dy = point[1] - closest[1];
    dx * dx + dy * dy
}

/// 计算多边形的质心
///
/// # 参数
///
/// * `vertices` - 多边形的顶点列表（按顺时针或逆时针顺序）
///
/// # 返回
///
/// * `Some([cx, cy])` - 质心坐标
/// * `None` - 顶点数不足 3 个或面积为 0
///
/// # 算法
///
/// 使用多边形质心公式：
/// - Cx = (1/6A) * Σ((xi + xi+1)(xi*yi+1 - xi+1*yi))
/// - Cy = (1/6A) * Σ((yi + yi+1)(xi*yi+1 - xi+1*yi))
///
/// 其中 A 为多边形面积
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::polygon_centroid;
///
/// // 正方形质心
/// let centroid = polygon_centroid(&[
///     [0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]
/// ]);
/// assert_eq!(centroid, Some([5.0, 5.0]));
///
/// // 三角形质心
/// let centroid = polygon_centroid(&[
///     [0.0, 0.0], [3.0, 0.0], [0.0, 4.0]
/// ]);
/// assert!(centroid.is_some());
/// ```
pub fn polygon_centroid(vertices: &[[f64; 2]]) -> Option<[f64; 2]> {
    let n = vertices.len();
    if n < 3 {
        return None;
    }

    let mut area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;

    for i in 0..n {
        let j = (i + 1) % n;
        let cross = vertices[i][0] * vertices[j][1] - vertices[j][0] * vertices[i][1];
        area += cross;
        cx += (vertices[i][0] + vertices[j][0]) * cross;
        cy += (vertices[i][1] + vertices[j][1]) * cross;
    }

    area *= 0.5;
    if area.abs() < 1e-10 {
        return None;
    }

    cx /= 6.0 * area;
    cy /= 6.0 * area;

    Some([cx, cy])
}

/// 计算点集的凸包
///
/// # 参数
///
/// * `points` - 输入点集
///
/// # 返回
///
/// 返回凸包顶点（按逆时针顺序，不重复起点）
///
/// # 算法
///
/// 使用 Andrew's monotone chain 算法：
/// 1. 按 x 坐标（x 相同时按 y）排序点
/// 2. 构建下凸包
/// 3. 构建上凸包
/// 4. 合并（移除重复的端点）
///
/// 时间复杂度：O(n log n)
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::convex_hull;
///
/// let points = vec![
///     [0.0, 0.0], [1.0, 1.0], [2.0, 0.0],
///     [1.0, 0.5], [0.5, 0.5]
/// ];
/// let hull = convex_hull(&points);
/// assert_eq!(hull.len(), 3); // 三角形
/// ```
pub fn convex_hull(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let n = points.len();
    if n <= 1 {
        return points.to_vec();
    }

    // 排序：先按 x，再按 y
    let mut sorted: Vec<_> = points.to_vec();
    sorted.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal))
    });

    // 叉积辅助函数
    fn cross(o: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    }

    // 构建下凸包
    let mut lower = Vec::new();
    for p in &sorted {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }

    // 构建上凸包
    let mut upper = Vec::new();
    for p in sorted.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }

    // 合并（移除重复的端点）
    lower.pop();
    upper.pop();
    lower.extend(upper);

    lower
}

/// 判断点是否在多边形内
///
/// # 参数
///
/// * `point` - 待判断的点
/// * `polygon` - 多边形顶点（按顺时针或逆时针顺序）
///
/// # 返回
///
/// * `true` - 点在多边形内
/// * `false` - 点在多边形外
///
/// # 算法
///
/// 使用射线法（ray casting algorithm）：
/// 从点向右发射射线，计算与多边形边的交点数
/// 奇数个交点表示在内部，偶数个表示在外部
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::point_in_polygon;
///
/// let square = [
///     [0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]
/// ];
/// assert!(point_in_polygon([5.0, 5.0], &square));
/// assert!(!point_in_polygon([15.0, 5.0], &square));
/// ```
pub fn point_in_polygon(point: [f64; 2], polygon: &[[f64; 2]]) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }

    let (px, py) = (point[0], point[1]);
    let mut inside = false;

    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (polygon[i][0], polygon[i][1]);
        let (xj, yj) = (polygon[j][0], polygon[j][1]);

        // 检查射线是否与边相交
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }

        j = i;
    }

    inside
}

/// 计算两个向量的夹角（弧度）
///
/// # 参数
///
/// * `v1` - 第一个向量 [dx, dy]
/// * `v2` - 第二个向量 [dx, dy]
///
/// # 返回
///
/// 返回夹角（范围 [0, π]）
///
/// # 示例
///
/// ```rust
/// use cadagent::geometry::vector_angle;
/// use std::f64::consts::PI;
///
/// // 垂直向量
/// let angle = vector_angle([1.0, 0.0], [0.0, 1.0]);
/// assert!((angle - PI / 2.0).abs() < 1e-6);
///
/// // 平行向量
/// let angle = vector_angle([1.0, 0.0], [1.0, 0.0]);
/// assert!(angle.abs() < 1e-6);
/// ```
pub fn vector_angle(v1: [f64; 2], v2: [f64; 2]) -> f64 {
    let dot = v1[0] * v2[0] + v1[1] * v2[1];
    let len1 = (v1[0] * v1[0] + v1[1] * v1[1]).sqrt();
    let len2 = (v2[0] * v2[0] + v2[1] * v2[1]).sqrt();

    if len1 < 1e-10 || len2 < 1e-10 {
        return 0.0;
    }

    let cos_angle = dot / (len1 * len2);
    // 限制在 [-1, 1] 范围内，避免数值误差
    let cos_angle = cos_angle.clamp(-1.0, 1.0);
    cos_angle.acos()
}

#[cfg(test)]
mod advanced_tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_line_intersection_single() {
        let result = line_intersection([0.0, 0.0], [2.0, 2.0], [0.0, 2.0], [2.0, 0.0]);
        assert!(matches!(result, LineIntersection::Single([1.0, 1.0])));
    }

    #[test]
    fn test_line_intersection_none() {
        // 平行线
        let result = line_intersection([0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]);
        assert!(matches!(result, LineIntersection::None));
    }

    #[test]
    fn test_closest_point_on_segment() {
        let closest = closest_point_on_segment([5.0, 5.0], [0.0, 0.0], [10.0, 0.0]);
        assert_eq!(closest, [5.0, 0.0]);
    }

    #[test]
    fn test_polygon_centroid() {
        let centroid = polygon_centroid(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]);
        assert_eq!(centroid, Some([5.0, 5.0]));
    }

    #[test]
    fn test_convex_hull() {
        // 正方形凸包
        let points = vec![
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
            [0.5, 0.5], // 内部点
        ];
        let hull = convex_hull(&points);
        // 凸包应该是正方形，包含 4 个顶点
        assert_eq!(hull.len(), 4);
        // 验证凸包包含正方形的四个顶点
        assert!(hull.contains(&[0.0, 0.0]));
        assert!(hull.contains(&[1.0, 0.0]));
        assert!(hull.contains(&[1.0, 1.0]));
        assert!(hull.contains(&[0.0, 1.0]));
    }

    #[test]
    fn test_point_in_polygon() {
        let square = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        assert!(point_in_polygon([5.0, 5.0], &square));
        assert!(!point_in_polygon([15.0, 5.0], &square));
    }

    #[test]
    fn test_vector_angle() {
        let angle = vector_angle([1.0, 0.0], [0.0, 1.0]);
        assert!((angle - PI / 2.0).abs() < 1e-6);
    }
}
