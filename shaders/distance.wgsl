// Distance computation shader for GPU-accelerated collision detection
// Used by: src/gpu/compute.rs - ComputePipeline

struct DistanceParams {
    point_a: vec3<f32>,
    pad1: f32,
    point_b: vec3<f32>,
    pad2: f32,
    threshold: f32,
    compute_all_pairs: u32,
    point_count: u32,
    pad3: f32,
};

struct Point3D {
    pos: vec3<f32>,
    distance: f32,
};

@group(0) @binding(0)
var<storage, read> input_points: array<Point3D>;

@group(0) @binding(1)
var<storage, read_write> output_distances: array<f32>;

@group(0) @binding(2)
var<uniform> params: DistanceParams;

// Compute distance from each point to a reference point
@compute @workgroup_size(64)
fn distance_to_point_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.point_count) {
        return;
    }

    let point = input_points[index].pos;
    let dx = point.x - params.point_a.x;
    let dy = point.y - params.point_a.y;
    let dz = point.z - params.point_a.z;

    let distance = sqrt(dx * dx + dy * dy + dz * dz);
    output_distances[index] = distance;
}

// Compute all pairwise distances (O(n^2) - use for small sets only)
@compute @workgroup_size(64)
fn all_pairs_distance_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    let j = global_id.y;

    if (i >= params.point_count || j >= params.point_count || i >= j) {
        return;
    }

    let pi = input_points[i].pos;
    let pj = input_points[j].pos;

    let dx = pi.x - pj.x;
    let dy = pi.y - pj.y;
    let dz = pi.z - pj.z;

    let distance = sqrt(dx * dx + dy * dy + dz * dz);

    // Store in upper triangular matrix layout
    let idx = i * params.point_count + j;
    output_distances[idx] = distance;
}

// Collision detection: mark points within threshold
@compute @workgroup_size(64)
fn collision_detect_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= params.point_count) {
        return;
    }

    let point = input_points[index].pos;
    let dx = point.x - params.point_a.x;
    let dy = point.y - params.point_a.y;
    let dz = point.z - params.point_a.z;

    let dist_sq = dx * dx + dy * dy + dz * dz;
    let threshold_sq = params.threshold * params.threshold;

    // Output 1.0 if collision, 0.0 otherwise
    if (dist_sq < threshold_sq) {
        output_distances[index] = 1.0;
    } else {
        output_distances[index] = 0.0;
    }
}
