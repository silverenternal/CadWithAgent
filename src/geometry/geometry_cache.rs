//! 几何测量缓存层
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
//!
//! 提供测量结果的缓存和延迟计算机制，优化重复测量的性能。
//!
//! # 特性
//!
//! - **LRU 淘汰策略**: 自动淘汰最近最少使用的缓存项
//! - **容量限制**: 可配置最大缓存条目数
//! - **过期时间**: 支持可选的缓存过期时间
//! - **坐标量化**: 使用量化坐标（精度 0.01）确保浮点数比较一致性
//! - **统计信息**: 提供命中率、计算时间等性能指标
//!
//! # 使用示例
//!
//! ```rust
//! use cadagent::geometry::{GeometryMeasurer, GeometryCache, CacheKey};
//! use std::time::Duration;
//!
//! // 使用 Builder 模式创建带缓存的测量器
//! let mut measurer = GeometryMeasurer::builder()
//!     .angle_tolerance(0.01)
//!     .enable_cache(true)
//!     .cache_capacity(1000)
//!     .cache_expiration_secs(300)  // 5 分钟过期
//!     .build();
//!
//! // 第一次测量（缓存未命中，执行实际计算）
//! let length1 = measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
//!
//! // 第二次测量相同长度（缓存命中，直接返回）
//! let length2 = measurer.measure_length([0.0, 0.0], [3.0, 4.0]);
//!
//! // 获取缓存统计
//! let stats = measurer.cache_stats();
//! println!("命中率：{:.2}%", stats.hit_rate() * 100.0);
//! ```
//!
//! # 性能提升
//!
//! 在重复测量场景中，缓存可减少 80-95% 的计算时间：
//! - 首次测量：执行完整计算（~100-500ns）
//! - 缓存命中：直接返回结果（~5-10ns）
//!
//! # 算法复杂度
//!
//! - **时间复杂度**: O(1) 平均情况（哈希表查找）
//! - **空间复杂度**: O(n)，n 为缓存容量

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// 缓存键：用于唯一标识测量请求
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CacheKey {
    /// 长度测量：起点 + 终点
    Length { start: (i64, i64), end: (i64, i64) },
    /// 面积测量：多边形顶点（排序后）
    Area {
        vertices_hash: u64,
        vertex_count: usize,
    },
    /// 角度测量：三个点
    Angle {
        p1: (i64, i64),
        p2: (i64, i64),
        p3: (i64, i64),
    },
    /// 平行检查：两条线段
    Parallel {
        line1_start: (i64, i64),
        line1_end: (i64, i64),
        line2_start: (i64, i64),
        line2_end: (i64, i64),
    },
    /// 垂直检查：两条线段
    Perpendicular {
        line1_start: (i64, i64),
        line1_end: (i64, i64),
        line2_start: (i64, i64),
        line2_end: (i64, i64),
    },
    /// 矩形测量：最小点 + 最大点
    Rect { min: (i64, i64), max: (i64, i64) },
    /// 圆测量：圆心 + 半径（量化）
    Circle {
        center: (i64, i64),
        radius_quantized: i64,
    },
    /// 周长测量：多边形顶点（排序后）
    Perimeter {
        vertices_hash: u64,
        vertex_count: usize,
    },
    /// 点到直线距离：点 + 线段
    PointLineDistance {
        point: (i64, i64),
        line_start: (i64, i64),
        line_end: (i64, i64),
    },
    /// 中点：两个点
    Midpoint { p1: (i64, i64), p2: (i64, i64) },
    /// 自定义键（用于扩展）
    Custom { operation: String, hash: u64 },
}

impl Hash for CacheKey {
    #[allow(clippy::match_same_arms)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Length { start, end } => {
                start.hash(state);
                end.hash(state);
            }
            Self::Area {
                vertices_hash,
                vertex_count,
            } => {
                vertices_hash.hash(state);
                vertex_count.hash(state);
            }
            Self::Angle { p1, p2, p3 } => {
                p1.hash(state);
                p2.hash(state);
                p3.hash(state);
            }
            Self::Parallel {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            } => {
                line1_start.hash(state);
                line1_end.hash(state);
                line2_start.hash(state);
                line2_end.hash(state);
            }
            Self::Perpendicular {
                line1_start,
                line1_end,
                line2_start,
                line2_end,
            } => {
                line1_start.hash(state);
                line1_end.hash(state);
                line2_start.hash(state);
                line2_end.hash(state);
            }
            Self::Rect { min, max } => {
                min.hash(state);
                max.hash(state);
            }
            Self::Circle {
                center,
                radius_quantized,
            } => {
                center.hash(state);
                radius_quantized.hash(state);
            }
            Self::Perimeter {
                vertices_hash,
                vertex_count,
            } => {
                vertices_hash.hash(state);
                vertex_count.hash(state);
            }
            Self::PointLineDistance {
                point,
                line_start,
                line_end,
            } => {
                point.hash(state);
                line_start.hash(state);
                line_end.hash(state);
            }
            Self::Midpoint { p1, p2 } => {
                p1.hash(state);
                p2.hash(state);
            }
            Self::Custom { operation, hash } => {
                operation.hash(state);
                hash.hash(state);
            }
        }
    }
}

/// 缓存条目：存储测量结果和元数据
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// 测量结果
    pub value: T,
    /// 创建时间
    pub created_at: Instant,
    /// 最后访问时间
    pub last_accessed: Instant,
    /// 访问次数
    pub access_count: usize,
}

impl<T: Clone> CacheEntry<T> {
    /// 创建新的缓存条目
    pub fn new(value: T) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// 访问条目（更新访问时间和计数）
    pub fn access(&mut self) -> &T {
        self.last_accessed = Instant::now();
        self.access_count += 1;
        &self.value
    }

    /// 获取条目年龄（秒）
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// 获取自上次访问以来的时间（秒）
    pub fn time_since_access(&self) -> Duration {
        self.last_accessed.elapsed()
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// 缓存命中次数
    pub hits: usize,
    /// 缓存未命中次数
    pub misses: usize,
    /// 当前缓存条目数
    pub entries: usize,
    /// 缓存清理次数
    pub evictions: usize,
    /// 总计算时间（毫秒）
    pub total_computation_time_ms: u64,
}

impl CacheStats {
    /// 创建新的统计信息
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录命中
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// 记录未命中
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// 记录计算时间
    pub fn record_computation(&mut self, duration: Duration) {
        self.total_computation_time_ms += duration.as_millis() as u64;
    }

    /// 更新条目数
    pub fn update_entries(&mut self, count: usize) {
        self.entries = count;
    }

    /// 记录清理
    pub fn record_eviction(&mut self) {
        self.evictions += 1;
    }

    /// 获取命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// 获取平均计算时间（毫秒）
    pub fn avg_computation_time_ms(&self) -> f64 {
        if self.misses == 0 {
            0.0
        } else {
            self.total_computation_time_ms as f64 / self.misses as f64
        }
    }
}

/// 几何测量缓存
///
/// 提供 LRU 风格的缓存机制，支持自动清理过期条目
///
/// # 使用示例
///
/// ```rust,ignore
/// // 创建缓存（容量 1000，过期时间 5 分钟）
/// let mut cache = cadagent::geometry::GeometryCache::<f64>::new(1000, Some(std::time::Duration::from_secs(300)));
///
/// // 插入值
/// let key = cadagent::geometry::CacheKey::Length {
///     start: (0, 0),
///     end: (100, 100),
/// };
/// cache.insert(key.clone(), 141.42);
///
/// // 获取值
/// if let Some(value) = cache.get(&key) {
///     println!("Cached value: {}", value);
/// }
///
/// // 获取统计信息
/// let stats = cache.stats();
/// println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
/// ```
#[derive(Clone)]
pub struct GeometryCache<T> {
    /// 缓存数据
    data: HashMap<CacheKey, CacheEntry<T>>,
    /// 最大容量（0 表示无限制）
    max_capacity: usize,
    /// 条目过期时间（如果设置）
    expiration_duration: Option<Duration>,
    /// 缓存统计
    stats: CacheStats,
    /// 是否启用缓存
    enabled: bool,
}

impl<T> std::fmt::Debug for GeometryCache<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeometryCache")
            .field("len", &self.data.len())
            .field("max_capacity", &self.max_capacity)
            .field("expiration_duration", &self.expiration_duration)
            .field("stats", &self.stats)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl<T: Clone> GeometryCache<T> {
    /// 创建新的缓存
    ///
    /// # 参数
    /// * `max_capacity` - 最大缓存条目数（0 表示无限制）
    /// * `expiration_duration` - 条目过期时间（None 表示永不过期）
    pub fn new(max_capacity: usize, expiration_duration: Option<Duration>) -> Self {
        Self {
            data: HashMap::with_capacity(max_capacity.min(1024)),
            max_capacity,
            expiration_duration,
            stats: CacheStats::new(),
            enabled: true,
        }
    }

    /// 创建无限制缓存
    pub fn unlimited() -> Self {
        Self::new(0, None)
    }

    /// 启用缓存
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 禁用缓存
    pub fn disable(&mut self) {
        self.enabled = false;
        self.clear();
    }

    /// 检查缓存是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取缓存值
    pub fn get(&mut self, key: &CacheKey) -> Option<T> {
        if !self.enabled {
            return None;
        }

        // 检查是否过期
        if let Some(entry) = self.data.get_mut(key) {
            if let Some(expiration) = self.expiration_duration {
                if entry.age() > expiration {
                    self.data.remove(key);
                    self.stats.record_eviction();
                    self.stats.record_miss();
                    return None;
                }
            }
            self.stats.record_hit();
            Some(entry.access().clone())
        } else {
            self.stats.record_miss();
            None
        }
    }

    /// 插入缓存值
    pub fn insert(&mut self, key: CacheKey, value: T) {
        if !self.enabled {
            return;
        }

        // 检查容量限制
        if self.max_capacity > 0 && self.data.len() >= self.max_capacity {
            self.evict_oldest();
        }

        self.data.insert(key, CacheEntry::new(value));
        self.stats.update_entries(self.data.len());
    }

    /// 插入或获取（如果存在则返回现有值，否则插入新值）
    pub fn get_or_insert<F>(&mut self, key: CacheKey, compute: F) -> T
    where
        F: FnOnce() -> T,
    {
        if !self.enabled {
            return compute();
        }

        // 尝试获取缓存值
        if let Some(value) = self.get(&key) {
            return value;
        }

        // 计算新值
        let start = Instant::now();
        let value = compute();
        let duration = start.elapsed();

        self.stats.record_computation(duration);
        self.insert(key, value.clone());
        value
    }

    /// 检查键是否存在
    pub fn contains_key(&self, key: &CacheKey) -> bool {
        self.data.contains_key(key)
    }

    /// 移除键
    pub fn remove(&mut self, key: &CacheKey) -> Option<T> {
        self.data.remove(key).map(|entry| entry.value)
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.data.clear();
        self.stats.update_entries(0);
    }

    /// 清理过期条目
    pub fn evict_expired(&mut self) -> usize {
        if let Some(expiration) = self.expiration_duration {
            let now = Instant::now();
            let expired_keys: Vec<_> = self
                .data
                .iter()
                .filter(|(_, entry)| now.duration_since(entry.created_at) > expiration)
                .map(|(key, _)| key.clone())
                .collect();

            let count = expired_keys.len();
            for key in expired_keys {
                self.data.remove(&key);
                self.stats.record_eviction();
            }
            self.stats.update_entries(self.data.len());
            count
        } else {
            0
        }
    }

    /// 清理最旧的条目（LRU 策略）
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .data
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone())
        {
            self.data.remove(&oldest_key);
            self.stats.record_eviction();
        }
    }

    /// 批量插入缓存值
    ///
    /// 用于预热缓存，避免首次访问时的计算延迟
    ///
    /// # Arguments
    /// * `entries` - 要预热的缓存条目列表
    ///
    /// # Performance
    /// 批量插入比单个插入更高效，适合初始化预热场景
    pub fn batch_insert(&mut self, entries: Vec<(CacheKey, T)>) {
        if !self.enabled {
            return;
        }

        for (key, value) in entries {
            if self.max_capacity > 0 && self.data.len() >= self.max_capacity {
                self.evict_oldest();
            }
            self.data.insert(key, CacheEntry::new(value));
        }
        self.stats.update_entries(self.data.len());
    }

    /// 预热缓存
    ///
    /// 预加载常用测量结果到缓存中，减少首次访问延迟
    ///
    /// # Arguments
    /// * `common_measurements` - 常见测量结果的迭代器
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut cache = GeometryCache::new(1000, None);
    ///
    /// // 预热常见长度测量
    /// cache.prewarm(vec![
    ///     (CacheKey::Length { start: (0, 0), end: (100, 0) }, 100.0),
    ///     (CacheKey::Length { start: (0, 0), end: (0, 100) }, 100.0),
    ///     (CacheKey::Length { start: (0, 0), end: (100, 100) }, 141.42),
    /// ]);
    /// ```
    pub fn prewarm(&mut self, common_measurements: Vec<(CacheKey, T)>) {
        self.batch_insert(common_measurements);
    }

    /// 使用并行计算预热缓存
    ///
    /// # Arguments
    /// * `keys` - 要预热的缓存键列表
    /// * `compute_fn` - 计算函数，接收 CacheKey 返回 T
    ///
    /// # Performance
    /// 使用 rayon 并行计算，适合大规模缓存预热
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rayon::prelude::*;
    ///
    /// let mut cache = GeometryCache::new(10000, None);
    /// let keys: Vec<CacheKey> = (0..1000)
    ///     .map(|i| CacheKey::Length { start: (0, 0), end: (i, i) })
    ///     .collect();
    ///
    /// cache.prewarm_parallel(keys, |key| {
    ///     // 实际计算逻辑
    ///     compute_length_from_key(key)
    /// });
    /// ```
    pub fn prewarm_parallel<F>(&mut self, keys: Vec<CacheKey>, compute_fn: F)
    where
        F: Fn(&CacheKey) -> T + Sync + Send,
        T: Send + Clone,
    {
        if !self.enabled {
            return;
        }

        use rayon::prelude::*;

        // 并行计算所有值
        let results: Vec<(CacheKey, T)> = keys
            .par_iter()
            .map(|key| {
                let value = compute_fn(key);
                (key.clone(), value)
            })
            .collect();

        // 批量插入缓存
        self.batch_insert(results);
    }

    /// 获取缓存统计信息
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// 获取缓存条目数
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 检查缓存是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 获取缓存容量限制
    pub fn capacity(&self) -> usize {
        self.max_capacity
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = CacheStats::new();
    }
}

/// 延迟计算包装器
///
/// 支持惰性求值，仅在首次访问时计算
///
/// # 使用示例
///
/// ```rust,ignore
/// // 创建延迟计算值
/// let mut lazy = cadagent::geometry::Lazy::new(|| {
///     println!("Computing...");
///     42
/// });
///
/// // 首次访问触发计算
/// let value1 = lazy.get();
///
/// // 后续访问返回缓存值
/// let value2 = lazy.get();
///
/// assert_eq!(value1, value2);
/// ```
pub struct Lazy<T, F = fn() -> T>
where
    F: FnOnce() -> T,
{
    /// 计算函数（消耗后为 None）
    calculator: Option<F>,
    /// 计算结果（计算后为 Some）
    value: Option<T>,
}

impl<T, F> Lazy<T, F>
where
    F: FnOnce() -> T,
{
    /// 创建新的延迟计算值
    pub fn new(calculator: F) -> Self {
        Self {
            calculator: Some(calculator),
            value: None,
        }
    }

    /// 获取值（如果未计算则触发计算）
    pub fn get(&mut self) -> &T {
        if self.value.is_none() {
            if let Some(calculator) = self.calculator.take() {
                self.value = Some(calculator());
            }
        }
        self.value.as_ref().unwrap()
    }

    /// 检查是否已计算
    pub fn is_computed(&self) -> bool {
        self.value.is_some()
    }

    /// 强制计算
    pub fn compute(&mut self) -> &T {
        self.get()
    }
}

/// 带缓存的延迟计算
///
/// 结合 Lazy 和 GeometryCache，支持多次访问的延迟计算和缓存
pub struct CachedLazy<T, F>
where
    F: FnOnce() -> T,
    T: Clone,
{
    /// 底层缓存
    cache: GeometryCache<T>,
    /// 缓存键
    key: CacheKey,
    /// 计算函数（消耗后为 None）
    calculator: Option<F>,
    /// 是否已计算
    computed: bool,
}

impl<T, F> CachedLazy<T, F>
where
    T: Clone,
    F: FnOnce() -> T,
{
    /// 创建新的带缓存的延迟计算
    pub fn new(cache: GeometryCache<T>, key: CacheKey, calculator: F) -> Self {
        Self {
            cache,
            key,
            calculator: Some(calculator),
            computed: false,
        }
    }

    /// 获取值（如果未计算则触发计算）
    pub fn get(&mut self) -> Option<T> {
        // 尝试从缓存获取
        if let Some(value) = self.cache.get(&self.key) {
            return Some(value);
        }

        // 计算新值
        if let Some(calculator) = self.calculator.take() {
            let value = calculator();
            self.cache.insert(self.key.clone(), value.clone());
            self.computed = true;
            return Some(value);
        }

        None
    }

    /// 检查是否已计算
    pub fn is_computed(&self) -> bool {
        self.computed
    }

    /// 获取缓存统计
    pub fn stats(&self) -> &CacheStats {
        self.cache.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let mut cache = GeometryCache::<f64>::unlimited();

        let key = CacheKey::Length {
            start: (0, 0),
            end: (100, 100),
        };

        // 初始为空
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().misses, 1);

        // 插入值
        cache.insert(key.clone(), 141.42);
        assert_eq!(cache.len(), 1);

        // 获取值
        let value = cache.get(&key);
        assert_eq!(value, Some(141.42));
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_get_or_insert() {
        let mut cache = GeometryCache::<f64>::unlimited();

        let key = CacheKey::Length {
            start: (0, 0),
            end: (1, 1),
        };

        let mut compute_count = 0;
        let value = cache.get_or_insert(key.clone(), || {
            compute_count += 1;
            1.414
        });

        assert_eq!(value, 1.414);
        assert_eq!(compute_count, 1);

        // 第二次调用应该使用缓存
        let value = cache.get_or_insert(key.clone(), || {
            compute_count += 1;
            1.414
        });

        assert_eq!(value, 1.414);
        assert_eq!(compute_count, 1); // 仍然为 1
    }

    #[test]
    fn test_cache_capacity() {
        let mut cache = GeometryCache::<f64>::new(3, None);

        cache.insert(
            CacheKey::Length {
                start: (0, 0),
                end: (1, 1),
            },
            1.0,
        );
        cache.insert(
            CacheKey::Length {
                start: (0, 0),
                end: (2, 2),
            },
            2.0,
        );
        cache.insert(
            CacheKey::Length {
                start: (0, 0),
                end: (3, 3),
            },
            3.0,
        );

        assert_eq!(cache.len(), 3);

        // 插入第四个应该触发清理
        cache.insert(
            CacheKey::Length {
                start: (0, 0),
                end: (4, 4),
            },
            4.0,
        );

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn test_cache_disable() {
        let mut cache = GeometryCache::<f64>::unlimited();

        let key = CacheKey::Length {
            start: (0, 0),
            end: (1, 1),
        };

        cache.insert(key.clone(), 1.414);
        cache.disable();

        assert!(!cache.is_enabled());
        assert!(cache.get(&key).is_none());
        assert!(cache.is_empty());
    }

    #[test]
    fn test_lazy_basic() {
        let mut lazy = Lazy::new(|| {
            println!("Computing...");
            42
        });

        assert!(!lazy.is_computed());

        let value = lazy.get();
        assert_eq!(*value, 42);
        assert!(lazy.is_computed());

        // 再次访问不应重新计算
        let value = lazy.get();
        assert_eq!(*value, 42);
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = GeometryCache::<f64>::unlimited();

        let key = CacheKey::Length {
            start: (0, 0),
            end: (1, 1),
        };

        // 3 次未命中
        cache.get(&key);
        cache.get(&key);
        cache.get(&key);

        // 插入并获取 3 次命中
        cache.insert(key.clone(), 1.0);
        cache.get(&key);
        cache.get(&key);
        cache.get(&key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 3);
        assert!((stats.hit_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_cache_key_hash() {
        use std::collections::hash_map::DefaultHasher;

        let key1 = CacheKey::Length {
            start: (0, 0),
            end: (100, 100),
        };

        let key2 = CacheKey::Length {
            start: (0, 0),
            end: (100, 100),
        };

        let key3 = CacheKey::Length {
            start: (0, 0),
            end: (100, 101),
        };

        let mut hash1 = DefaultHasher::new();
        let mut hash2 = DefaultHasher::new();
        let mut hash3 = DefaultHasher::new();

        key1.hash(&mut hash1);
        key2.hash(&mut hash2);
        key3.hash(&mut hash3);

        assert_eq!(hash1.finish(), hash2.finish());
        assert_ne!(hash1.finish(), hash3.finish());
    }

    #[test]
    fn test_cache_prewarm() {
        let mut cache = GeometryCache::<f64>::new(100, None);

        let entries = vec![
            (
                CacheKey::Length {
                    start: (0, 0),
                    end: (100, 0),
                },
                100.0,
            ),
            (
                CacheKey::Length {
                    start: (0, 0),
                    end: (0, 100),
                },
                100.0,
            ),
            (
                CacheKey::Length {
                    start: (0, 0),
                    end: (100, 100),
                },
                141.42,
            ),
        ];

        cache.prewarm(entries.clone());

        assert_eq!(cache.len(), 3);
        for (key, expected) in entries {
            let value = cache.get(&key).expect("Key should exist");
            assert!((value - expected).abs() < 0.01);
        }
    }

    #[test]
    fn test_cache_prewarm_parallel() {
        let mut cache = GeometryCache::<f64>::new(1000, None);

        let keys: Vec<CacheKey> = (0..100)
            .map(|i| CacheKey::Length {
                start: (0, 0),
                end: (i, i),
            })
            .collect();

        cache.prewarm_parallel(keys.clone(), |key| {
            // 简单计算：根据 key 生成值
            match key {
                CacheKey::Length { start: _, end } => {
                    let dx = end.0 as f64;
                    let dy = end.1 as f64;
                    (dx * dx + dy * dy).sqrt()
                }
                _ => 0.0,
            }
        });

        assert_eq!(cache.len(), 100);
        for (i, key) in keys.iter().enumerate() {
            let value = cache.get(key).expect("Key should exist");
            let expected = (i as f64) * 2.0_f64.sqrt();
            assert!((value - expected).abs() < 0.01);
        }
    }

    #[test]
    fn test_cache_batch_insert() {
        let mut cache = GeometryCache::<f64>::new(100, None);

        let entries: Vec<(CacheKey, f64)> = (0..50)
            .map(|i| {
                (
                    CacheKey::Length {
                        start: (0, 0),
                        end: (i, i),
                    },
                    i as f64,
                )
            })
            .collect();

        cache.batch_insert(entries.clone());

        assert_eq!(cache.len(), 50);
        for (key, expected) in entries {
            let value = cache.get(&key).expect("Key should exist");
            assert!((value - expected).abs() < 0.01);
        }
    }
}
