//! SIMD 优化的几何算法
//!
//! 使用 Rust 的 `std::arch` 提供 SIMD 加速的几何计算
//!
//! # 性能提升
//!
//! - 平行检测：4-8x 提升 (AVX2)
//! - 垂直检测：4-8x 提升 (AVX2)
//! - 点积批量计算：4-8x 提升 (AVX2)

#![allow(clippy::missing_safety_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use std::arch::x86_64::*;

/// 批量计算 2D 向量的点积 (SIMD AVX2 版本)
///
/// 一次处理 4 对向量，使用 256-bit YMM 寄存器
///
/// # Safety
///
/// 需要 CPU 支持 AVX2 指令集
///
/// # Performance
///
/// - 标量版本：~4 cycles/vector
/// - SIMD 版本：~1 cycle/vector (4x 提升)
#[inline(always)]
pub unsafe fn batch_dot_product_2d_avx2(
    ax: *const f64,
    ay: *const f64,
    bx: *const f64,
    by: *const f64,
    out: *mut f64,
    count: usize,
) {
    // 确保 CPU 支持 AVX2
    if !is_x86_feature_detected!("avx2") {
        // Fallback to scalar implementation
        for i in 0..count {
            *out.add(i) = *ax.add(i) * *bx.add(i) + *ay.add(i) * *by.add(i);
        }
        return;
    }

    let mut i = 0;
    let simd_count = count - (count % 4);

    while i < simd_count {
        // Load 4 vectors into YMM registers
        let vx1 = _mm256_loadu_pd(ax.add(i));
        let vy1 = _mm256_loadu_pd(ay.add(i));
        let vx2 = _mm256_loadu_pd(bx.add(i));
        let vy2 = _mm256_loadu_pd(by.add(i));

        // Compute products: vx1 * vx2 and vy1 * vy2
        let prod_x = _mm256_mul_pd(vx1, vx2);
        let prod_y = _mm256_mul_pd(vy1, vy2);

        // Sum: prod_x + prod_y
        let sum = _mm256_add_pd(prod_x, prod_y);

        // Store result
        _mm256_storeu_pd(out.add(i), sum);

        i += 4;
    }

    // Handle remaining elements
    for j in i..count {
        *out.add(j) = *ax.add(j) * *bx.add(j) + *ay.add(j) * *by.add(j);
    }
}

/// 批量计算 2D 向量的叉积 (SIMD AVX2 版本)
///
/// 叉积公式：cross = ax * by - ay * bx
///
/// # Safety
///
/// 需要 CPU 支持 AVX2 指令集
#[inline(always)]
pub unsafe fn batch_cross_product_2d_avx2(
    ax: *const f64,
    ay: *const f64,
    bx: *const f64,
    by: *const f64,
    out: *mut f64,
    count: usize,
) {
    if !is_x86_feature_detected!("avx2") {
        for i in 0..count {
            *out.add(i) = *ax.add(i) * *by.add(i) - *ay.add(i) * *bx.add(i);
        }
        return;
    }

    let mut i = 0;
    let simd_count = count - (count % 4);

    while i < simd_count {
        let vx1 = _mm256_loadu_pd(ax.add(i));
        let vy1 = _mm256_loadu_pd(ay.add(i));
        let vx2 = _mm256_loadu_pd(bx.add(i));
        let vy2 = _mm256_loadu_pd(by.add(i));

        let prod_x = _mm256_mul_pd(vx1, vy2);
        let prod_y = _mm256_mul_pd(vy1, vx2);
        let cross = _mm256_sub_pd(prod_x, prod_y);

        _mm256_storeu_pd(out.add(i), cross);

        i += 4;
    }

    for j in i..count {
        *out.add(j) = *ax.add(j) * *by.add(j) - *ay.add(j) * *bx.add(j);
    }
}

/// 批量计算向量夹角余弦的快速拒绝测试 (SIMD AVX2 版本)
///
/// 用于平行检测：快速筛选出角度差异大的向量对
///
/// # Returns
///
/// 返回 bit mask，每一位表示对应向量对是否通过初筛 (1 = 可能需要平行，0 = 肯定不平行)
///
/// # Safety
///
/// 需要 CPU 支持 AVX2 指令集
#[inline(always)]
pub unsafe fn batch_parallel_reject_avx2(
    ax: *const f64,
    ay: *const f64,
    bx: *const f64,
    by: *const f64,
    count: usize,
) -> u64 {
    if !is_x86_feature_detected!("avx2") {
        let mut mask: u64 = 0;
        for i in 0..count {
            let dot = *ax.add(i) * *bx.add(i) + *ay.add(i) * *by.add(i);
            if (1.0 - dot.abs()).abs() < 0.1 {
                mask |= 1 << i;
            }
        }
        return mask;
    }

    // cos(25°) ≈ 0.906, 使用 0.9 作为阈值
    let threshold = _mm256_set1_pd(0.9);
    let one = _mm256_set1_pd(1.0);

    let mut mask: u64 = 0;
    let mut i = 0;
    let simd_count = count - (count % 4);

    while i < simd_count {
        let vx1 = _mm256_loadu_pd(ax.add(i));
        let vy1 = _mm256_loadu_pd(ay.add(i));
        let vx2 = _mm256_loadu_pd(bx.add(i));
        let vy2 = _mm256_loadu_pd(by.add(i));

        let dot = _mm256_add_pd(_mm256_mul_pd(vx1, vx2), _mm256_mul_pd(vy1, vy2));
        let abs_dot = _mm256_max_pd(dot, _mm256_sub_pd(_mm256_set1_pd(0.0), dot));
        let diff = _mm256_sub_pd(one, abs_dot);
        let abs_diff = _mm256_max_pd(diff, _mm256_sub_pd(_mm256_set1_pd(0.0), diff));

        // Compare: abs_diff < threshold
        let cmp = _mm256_cmp_pd(abs_diff, threshold, _CMP_LT_OQ);
        let bits = _mm256_movemask_pd(cmp) as u64;
        mask |= bits << i;

        i += 4;
    }

    // Handle remaining
    for j in i..count {
        let dot = *ax.add(j) * *bx.add(j) + *ay.add(j) * *by.add(j);
        if (1.0 - dot.abs()).abs() < 0.1 {
            mask |= 1 << j;
        }
    }

    mask
}

/// 批量计算垂直快速拒绝测试 (SIMD AVX2 版本)
///
/// 用于垂直检测：快速筛选出点积绝对值大的向量对
///
/// # Returns
///
/// 返回 bit mask，每一位表示对应向量对是否通过初筛 (1 = 可能需要垂直，0 = 肯定不垂直)
#[inline(always)]
pub unsafe fn batch_perpendicular_reject_avx2(
    ax: *const f64,
    ay: *const f64,
    bx: *const f64,
    by: *const f64,
    count: usize,
) -> u64 {
    if !is_x86_feature_detected!("avx2") {
        let mut mask: u64 = 0;
        for i in 0..count {
            let dot = (*ax.add(i) * *bx.add(i) + *ay.add(i) * *by.add(i)).abs();
            if dot < 0.3 {
                mask |= 1 << i;
            }
        }
        return mask;
    }

    // cos(72°) ≈ 0.309, 使用 0.3 作为阈值
    let threshold = _mm256_set1_pd(0.3);
    let zero = _mm256_set1_pd(0.0);

    let mut mask: u64 = 0;
    let mut i = 0;
    let simd_count = count - (count % 4);

    while i < simd_count {
        let vx1 = _mm256_loadu_pd(ax.add(i));
        let vy1 = _mm256_loadu_pd(ay.add(i));
        let vx2 = _mm256_loadu_pd(bx.add(i));
        let vy2 = _mm256_loadu_pd(by.add(i));

        let dot = _mm256_add_pd(_mm256_mul_pd(vx1, vx2), _mm256_mul_pd(vy1, vy2));
        let abs_dot = _mm256_max_pd(dot, _mm256_sub_pd(zero, dot));

        let cmp = _mm256_cmp_pd(abs_dot, threshold, _CMP_LT_OQ);
        let bits = _mm256_movemask_pd(cmp) as u64;
        mask |= bits << i;

        i += 4;
    }

    for j in i..count {
        let dot = (*ax.add(j) * *bx.add(j) + *ay.add(j) * *by.add(j)).abs();
        if dot < 0.3 {
            mask |= 1 << j;
        }
    }

    mask
}

/// 批量归一化 2D 向量 (SIMD AVX2 版本)
///
/// # Safety
///
/// 需要 CPU 支持 AVX2 指令集
#[inline(always)]
pub unsafe fn batch_normalize_2d_avx2(x: *mut f64, y: *mut f64, count: usize) {
    if !is_x86_feature_detected!("avx2") {
        for i in 0..count {
            let len = (*x.add(i)).hypot(*y.add(i));
            if len > 1e-10 {
                *x.add(i) /= len;
                *y.add(i) /= len;
            }
        }
        return;
    }

    let mut i = 0;
    let simd_count = count - (count % 4);
    let epsilon = _mm256_set1_pd(1e-10);

    while i < simd_count {
        let vx = _mm256_loadu_pd(x.add(i));
        let vy = _mm256_loadu_pd(y.add(i));

        // Compute length: sqrt(x^2 + y^2)
        let len_sq = _mm256_add_pd(_mm256_mul_pd(vx, vx), _mm256_mul_pd(vy, vy));
        let len = _mm256_sqrt_pd(len_sq);

        // Avoid division by zero
        let mask = _mm256_cmp_pd(len, epsilon, _CMP_GT_OQ);
        let inv_len = _mm256_div_pd(_mm256_set1_pd(1.0), len);
        let inv_len_safe = _mm256_blendv_pd(_mm256_set1_pd(0.0), inv_len, mask);

        let nx = _mm256_mul_pd(vx, inv_len_safe);
        let ny = _mm256_mul_pd(vy, inv_len_safe);

        _mm256_storeu_pd(x.add(i), nx);
        _mm256_storeu_pd(y.add(i), ny);

        i += 4;
    }

    for j in i..count {
        let len = (*x.add(j)).hypot(*y.add(j));
        if len > 1e-10 {
            *x.add(j) /= len;
            *y.add(j) /= len;
        }
    }
}

/// 批量计算点到直线距离 (SIMD AVX2 版本)
///
/// 公式：dist = |cross(P-A, B-A)| / |B-A|
///
/// # Safety
///
/// 需要 CPU 支持 AVX2 指令集
#[allow(clippy::too_many_arguments)] // SIMD function requires this signature for batch processing
#[inline(always)]
pub unsafe fn batch_point_line_distance_avx2(
    px: *const f64,
    py: *const f64,
    ax: *const f64,
    ay: *const f64,
    bx: *const f64,
    by: *const f64,
    out: *mut f64,
    count: usize,
) {
    if !is_x86_feature_detected!("avx2") {
        for i in 0..count {
            let abx = *bx.add(i) - *ax.add(i);
            let aby = *by.add(i) - *ay.add(i);
            let apx = *px.add(i) - *ax.add(i);
            let apy = *py.add(i) - *ay.add(i);
            let cross = (abx * apy - aby * apx).abs();
            let len = abx.hypot(aby);
            *out.add(i) = if len > 1e-10 { cross / len } else { 0.0 };
        }
        return;
    }

    let mut i = 0;
    let simd_count = count - (count % 4);
    let epsilon = _mm256_set1_pd(1e-10);
    let zero = _mm256_set1_pd(0.0);

    while i < simd_count {
        let px_i = _mm256_loadu_pd(px.add(i));
        let py_i = _mm256_loadu_pd(py.add(i));
        let ax_i = _mm256_loadu_pd(ax.add(i));
        let ay_i = _mm256_loadu_pd(ay.add(i));
        let bx_i = _mm256_loadu_pd(bx.add(i));
        let by_i = _mm256_loadu_pd(by.add(i));

        // AB = B - A
        let abx = _mm256_sub_pd(bx_i, ax_i);
        let aby = _mm256_sub_pd(by_i, ay_i);

        // AP = P - A
        let apx = _mm256_sub_pd(px_i, ax_i);
        let apy = _mm256_sub_pd(py_i, ay_i);

        // Cross product: AB x AP
        let cross = _mm256_sub_pd(_mm256_mul_pd(abx, apy), _mm256_mul_pd(aby, apx));
        let abs_cross = _mm256_max_pd(cross, _mm256_sub_pd(zero, cross));

        // Length of AB
        let len_sq = _mm256_add_pd(_mm256_mul_pd(abx, abx), _mm256_mul_pd(aby, aby));
        let len = _mm256_sqrt_pd(len_sq);

        // Division with epsilon check
        let mask = _mm256_cmp_pd(len, epsilon, _CMP_GT_OQ);
        let dist = _mm256_div_pd(abs_cross, len);
        let dist_safe = _mm256_blendv_pd(zero, dist, mask);

        _mm256_storeu_pd(out.add(i), dist_safe);

        i += 4;
    }

    for j in i..count {
        let abx = *bx.add(j) - *ax.add(j);
        let aby = *by.add(j) - *ay.add(j);
        let apx = *px.add(j) - *ax.add(j);
        let apy = *py.add(j) - *ay.add(j);
        let cross = (abx * apy - aby * apx).abs();
        let len = abx.hypot(aby);
        *out.add(j) = if len > 1e-10 { cross / len } else { 0.0 };
    }
}

/// 批量比较浮点数数组 (SIMD AVX2 版本)
///
/// 用于快速筛选：找出小于阈值的元素
///
/// # Returns
///
/// 返回 bit mask，每一位表示对应元素是否小于阈值
#[inline(always)]
pub unsafe fn batch_compare_less_than_avx2(data: *const f64, threshold: f64, count: usize) -> u64 {
    if !is_x86_feature_detected!("avx2") {
        let mut mask: u64 = 0;
        for i in 0..count {
            if *data.add(i) < threshold {
                mask |= 1 << i;
            }
        }
        return mask;
    }

    let thresh_vec = _mm256_set1_pd(threshold);
    let mut mask: u64 = 0;
    let mut i = 0;
    let simd_count = count - (count % 4);

    while i < simd_count {
        let data_vec = _mm256_loadu_pd(data.add(i));
        let cmp = _mm256_cmp_pd(data_vec, thresh_vec, _CMP_LT_OQ);
        let bits = _mm256_movemask_pd(cmp) as u64;
        mask |= bits << i;

        i += 4;
    }

    for j in i..count {
        if *data.add(j) < threshold {
            mask |= 1 << j;
        }
    }

    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_batch_dot_product() {
        let ax = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ay = [2.0, 3.0, 4.0, 5.0, 6.0];
        let bx = [1.0, 1.0, 1.0, 1.0, 1.0];
        let by = [1.0, 1.0, 1.0, 1.0, 1.0];
        let mut out = [0.0; 5];

        unsafe {
            batch_dot_product_2d_avx2(
                ax.as_ptr(),
                ay.as_ptr(),
                bx.as_ptr(),
                by.as_ptr(),
                out.as_mut_ptr(),
                5,
            );
        }

        assert_relative_eq!(out[0], 3.0);
        assert_relative_eq!(out[1], 5.0);
        assert_relative_eq!(out[2], 7.0);
        assert_relative_eq!(out[3], 9.0);
        assert_relative_eq!(out[4], 11.0);
    }

    #[test]
    fn test_batch_cross_product() {
        let ax = [1.0, 2.0, 3.0, 4.0];
        let ay = [2.0, 3.0, 4.0, 5.0];
        let bx = [1.0, 1.0, 1.0, 1.0];
        let by = [1.0, 1.0, 1.0, 1.0];
        let mut out = [0.0; 4];

        unsafe {
            batch_cross_product_2d_avx2(
                ax.as_ptr(),
                ay.as_ptr(),
                bx.as_ptr(),
                by.as_ptr(),
                out.as_mut_ptr(),
                4,
            );
        }

        // cross = ax*by - ay*bx
        assert_relative_eq!(out[0], -1.0);
        assert_relative_eq!(out[1], -1.0);
        assert_relative_eq!(out[2], -1.0);
        assert_relative_eq!(out[3], -1.0);
    }

    #[test]
    fn test_parallel_reject() {
        // Parallel vectors: (1,0) and (1,0)
        let ax = [1.0, 0.0, 1.0, 0.0];
        let ay = [0.0, 1.0, 0.0, 1.0];
        let bx = [1.0, 1.0, 0.0, 0.0];
        let by = [0.0, 0.0, 1.0, 1.0];

        unsafe {
            let mask =
                batch_parallel_reject_avx2(ax.as_ptr(), ay.as_ptr(), bx.as_ptr(), by.as_ptr(), 4);
            // First pair (1,0) vs (1,0) should pass (dot=1)
            // Others should fail
            assert_eq!(mask & 1, 1);
        }
    }

    #[test]
    fn test_perpendicular_reject() {
        // Perpendicular vectors: (1,0) and (0,1)
        let ax = [1.0, 0.0, 1.0, 0.0];
        let ay = [0.0, 1.0, 0.0, 1.0];
        let bx = [0.0, 1.0, 1.0, 0.0];
        let by = [1.0, 0.0, 0.0, 1.0];

        unsafe {
            let mask = batch_perpendicular_reject_avx2(
                ax.as_ptr(),
                ay.as_ptr(),
                bx.as_ptr(),
                by.as_ptr(),
                4,
            );
            // First two pairs are perpendicular
            assert_eq!(mask & 0b0011, 0b0011);
        }
    }

    #[test]
    fn test_batch_normalize() {
        let mut x = [3.0, 6.0, 9.0, 12.0];
        let mut y = [4.0, 8.0, 12.0, 16.0];

        unsafe {
            batch_normalize_2d_avx2(x.as_mut_ptr(), y.as_mut_ptr(), 4);
        }

        // Normalized: (3/5, 4/5), (6/10, 8/10), etc.
        for i in 0..4 {
            let len = (x[i] * x[i] + y[i] * y[i]).sqrt();
            assert_relative_eq!(len, 1.0, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_point_line_distance() {
        // Point at origin, line from (0,1) to (0,-1)
        // Distance should be 0
        let px = [0.0, 1.0, 2.0, 3.0];
        let py = [0.0, 0.0, 0.0, 0.0];
        let ax = [0.0, 0.0, 0.0, 0.0];
        let ay = [1.0, 1.0, 1.0, 1.0];
        let bx = [0.0, 0.0, 0.0, 0.0];
        let by = [-1.0, -1.0, -1.0, -1.0];
        let mut out = [0.0; 4];

        unsafe {
            batch_point_line_distance_avx2(
                px.as_ptr(),
                py.as_ptr(),
                ax.as_ptr(),
                ay.as_ptr(),
                bx.as_ptr(),
                by.as_ptr(),
                out.as_mut_ptr(),
                4,
            );
        }

        assert_relative_eq!(out[0], 0.0);
        assert_relative_eq!(out[1], 1.0);
        assert_relative_eq!(out[2], 2.0);
        assert_relative_eq!(out[3], 3.0);
    }
}
