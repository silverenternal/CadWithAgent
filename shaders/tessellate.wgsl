// Tessellation shader for GPU-accelerated B-Rep mesh generation
// Used by: src/gpu/compute.rs - ComputePipeline

struct TessellationParams {
    subdivision_level: u32,
    triangle_count: u32,
    output_stride: f32,
    padding: f32,
    reserved1: f32,
    reserved2: f32,
    reserved3: f32,
    reserved4: f32,
    reserved5: f32,
    reserved6: f32,
    reserved7: f32,
    reserved8: f32,
};

struct Triangle {
    v0: vec3<f32>,
    pad0: f32,
    v1: vec3<f32>,
    pad1: f32,
    v2: vec3<f32>,
    pad2: f32,
};

@group(0) @binding(0)
var<storage, read> input_triangles: array<Triangle>;

@group(0) @binding(1)
var<storage, read_write> output_vertices: array<vec3<f32>>;

@group(0) @binding(2)
var<uniform> params: TessellationParams;

// Linear interpolation
fn lerp(a: vec3<f32>, b: vec3<f32>, t: f32) -> vec3<f32> {
    return a + (b - a) * t;
}

// Subdivide a triangle using Loop subdivision (simplified)
@compute @workgroup_size(64)
fn tessellate_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let tri_idx = global_id.x;
    if (tri_idx >= params.triangle_count) {
        return;
    }

    let tri = input_triangles[tri_idx];

    // Compute edge midpoints
    let m0 = lerp(tri.v0, tri.v1, 0.5);
    let m1 = lerp(tri.v1, tri.v2, 0.5);
    let m2 = lerp(tri.v2, tri.v0, 0.5);

    // Output 4 subdivided triangles (12 vertices)
    let base_idx = tri_idx * 12u;

    // Triangle 1: original v0, m0, m2
    output_vertices[base_idx + 0u] = tri.v0;
    output_vertices[base_idx + 1u] = m0;
    output_vertices[base_idx + 2u] = m2;

    // Triangle 2: m0, original v1, m1
    output_vertices[base_idx + 3u] = m0;
    output_vertices[base_idx + 4u] = tri.v1;
    output_vertices[base_idx + 5u] = m1;

    // Triangle 3: m2, m1, original v2
    output_vertices[base_idx + 6u] = m2;
    output_vertices[base_idx + 7u] = m1;
    output_vertices[base_idx + 8u] = tri.v2;

    // Triangle 4: m0, m1, m2 (center)
    output_vertices[base_idx + 9u] = m0;
    output_vertices[base_idx + 10u] = m1;
    output_vertices[base_idx + 11u] = m2;
}
