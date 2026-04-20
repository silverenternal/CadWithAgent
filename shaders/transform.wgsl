// Transform shader for GPU point transformations
// Used by: src/gpu/compute.rs - TransformPipeline

struct TransformParams {
    matrix: mat4x4<f32>,
    use_projection: u32,
    viewport_width: f32,
    viewport_height: f32,
    padding1: f32,
};

struct Point3D {
    pos: vec3<f32>,
    w: f32,
};

@group(0) @binding(0)
var<storage, read> input_points: array<Point3D>;

@group(0) @binding(1)
var<storage, read_write> output_points: array<Point3D>;

@group(0) @binding(2)
var<uniform> params: TransformParams;

@compute @workgroup_size(64)
fn transform_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&input_points)) {
        return;
    }

    let input = input_points[index];
    let input_vec = vec4<f32>(input.pos, input.w);

    // Apply 4x4 matrix transformation
    var output_vec = params.matrix * input_vec;

    // Perspective division if projection is enabled
    if (params.use_projection == 1 && output_vec.w != 0.0) {
        output_vec = output_vec / output_vec.w;
    }

    // Viewport transformation for 2D projection
    if (params.use_projection == 1) {
        output_vec.x = (output_vec.x + 1.0) * 0.5 * params.viewport_width;
        output_vec.y = (1.0 - output_vec.y) * 0.5 * params.viewport_height;
    }

    output_points[index] = Point3D(output_vec.xyz, output_vec.w);
}
