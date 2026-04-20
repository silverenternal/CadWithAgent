// Normal calculation shader for GPU-accelerated mesh processing
// Used by: src/gpu/compute.rs - ComputePipeline

struct VertexData {
    position: vec3<f32>,
    normal: vec3<f32>,
    padding: f32,
};

@group(0) @binding(0)
var<storage, read> input_vertices: array<vec3<f32>>;

@group(0) @binding(1)
var<storage, read_write> output_normals: array<vec3<f32>>;

@group(0) @binding(2)
var<storage, read> indices: array<u32>;

@compute @workgroup_size(64)
fn normal_calc_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let vertex_idx = global_id.x;
    if (vertex_idx >= arrayLength(&input_vertices)) {
        return;
    }

    // Compute normal using adjacent vertices (simplified)
    // In production, would use triangle adjacency
    var normal = vec3<f32>(0.0, 0.0, 0.0);
    let pos = input_vertices[vertex_idx];

    // Sample neighboring vertices for normal estimation
    let count = min(3u, arrayLength(&input_vertices) - vertex_idx);
    for (var i = 1u; i <= count; i = i + 1u) {
        let neighbor = input_vertices[vertex_idx + i];
        let edge = neighbor - pos;
        normal = normal + edge;
    }

    // Normalize
    let len = length(normal);
    if (len > 0.0001) {
        output_normals[vertex_idx] = normalize(normal);
    } else {
        output_normals[vertex_idx] = vec3<f32>(0.0, 1.0, 0.0);
    }
}
