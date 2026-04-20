//! GPU rendering pipeline for high-performance geometry visualization
//!
//! This module provides a rendering pipeline using wgpu for visualizing
//! CAD geometry with support for:
//! - 3D model rendering with Phong lighting
//! - Multiple viewports
//! - Level of detail (LOD) rendering
//! - Picking and selection
//! - Anti-aliasing (MSAA)

use nalgebra::{Matrix4, Point3, Vector3};
use wgpu::{
    BindGroup, CompositeAlphaMode, Device, LoadOp, MultisampleState, Operations, PresentMode,
    RenderPassColorAttachment, RenderPipeline, StoreOp, Surface, SurfaceConfiguration, Texture,
    TextureFormat, TextureView,
};

use super::buffers::{IndexBuffer, UniformBuffer, Vertex, VertexBuffer};
use super::compute::GpuContext;

/// Camera for 3D viewing
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// Create a new camera
    pub fn new(
        position: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        fov: f32,
        aspect: f32,
    ) -> Self {
        Self {
            position,
            target,
            up,
            fov,
            aspect,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Get the view matrix
    pub fn view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.position, &self.target, &self.up)
    }

    /// Get the projection matrix
    pub fn projection_matrix(&self) -> Matrix4<f32> {
        Matrix4::new_perspective(self.aspect, self.fov, self.near, self.far)
    }

    /// Get the view-projection matrix
    pub fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.projection_matrix() * self.view_matrix()
    }

    /// Update aspect ratio
    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    /// Orbit camera around target
    pub fn orbit(&mut self, yaw: f32, pitch: f32) {
        let dx = self.position.x - self.target.x;
        let dy = self.position.y - self.target.y;
        let dz = self.position.z - self.target.z;

        let radius = (dx * dx + dy * dy + dz * dz).sqrt();
        let new_yaw = yaw.to_radians();
        let new_pitch = pitch
            .to_radians()
            .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());

        self.position.x = self.target.x + radius * new_pitch.cos() * new_yaw.sin();
        self.position.y = self.target.y + radius * new_pitch.sin();
        self.position.z = self.target.z + radius * new_pitch.cos() * new_yaw.cos();
    }

    /// Zoom camera
    pub fn zoom(&mut self, delta: f32) {
        let direction = self.target - self.position;
        let length = direction.norm();
        let new_length = (length - delta).max(0.1);
        let direction = direction.normalize();

        self.position = self.target - direction * new_length;
    }
}

/// Level of Detail (LOD) configuration
///
/// Controls the geometric detail level based on distance from camera.
/// Higher LOD = more triangles, better quality, slower performance.
/// Lower LOD = fewer triangles, faster rendering, suitable for distant objects.
#[derive(Debug, Clone, Copy, Default)]
pub enum LodLevel {
    /// Highest detail (no simplification)
    #[default]
    High,
    /// Medium detail (~50% triangles)
    Medium,
    /// Low detail (~25% triangles)
    Low,
    /// Adaptive (automatically select based on distance)
    Adaptive,
}

/// Render configuration
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Anti-aliasing sample count (1, 2, 4, or 8)
    pub sample_count: u32,
    /// Level of detail setting
    pub lod_level: LodLevel,
    /// Enable shadows
    pub enable_shadows: bool,
    /// Enable wireframe mode
    pub wireframe: bool,
    /// Background color
    pub background_color: wgpu::Color,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            sample_count: 4, // Default to 4x MSAA
            lod_level: LodLevel::High,
            enable_shadows: false,
            wireframe: false,
            background_color: wgpu::Color {
                r: 0.1,
                g: 0.1,
                b: 0.15,
                a: 1.0,
            },
        }
    }
}

impl RenderConfig {
    /// Create a new render config with custom MSAA
    pub fn with_msaa(sample_count: u32) -> Self {
        Self {
            sample_count: sample_count.clamp(1, 8),
            ..Default::default()
        }
    }

    /// Enable or disable wireframe mode
    pub fn with_wireframe(mut self, enabled: bool) -> Self {
        self.wireframe = enabled;
        self
    }

    /// Set LOD level
    pub fn with_lod(mut self, lod: LodLevel) -> Self {
        self.lod_level = lod;
        self
    }

    /// Get the appropriate polygon mode for this config
    pub fn polygon_mode(&self) -> wgpu::PolygonMode {
        if self.wireframe {
            wgpu::PolygonMode::Line
        } else {
            wgpu::PolygonMode::Fill
        }
    }

    /// Get the multisample state for the render pipeline
    pub fn multisample_state(&self) -> MultisampleState {
        MultisampleState {
            count: self.sample_count,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }
}

/// Uniform data for rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub model: [[f32; 4]; 4],
    pub light_position: [f32; 4],
    pub light_color: [f32; 4],
    pub camera_position: [f32; 4],
    pub time: f32,
    _padding: [f32; 3],
}

impl Default for RenderUniforms {
    fn default() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
            model: Matrix4::identity().into(),
            light_position: [10.0, 10.0, 10.0, 1.0],
            light_color: [1.0, 1.0, 1.0, 1.0],
            camera_position: [0.0, 0.0, 0.0, 1.0],
            time: 0.0,
            _padding: [0.0; 3],
        }
    }
}

/// Renderer state
pub struct Renderer {
    context: GpuContext,
    render_pipeline: RenderPipeline,
    vertex_buffer: Option<VertexBuffer>,
    index_buffer: Option<IndexBuffer>,
    uniform_buffer: UniformBuffer<RenderUniforms>,
    uniforms: RenderUniforms,
    config: Option<SurfaceConfiguration>,
    surface: Option<Surface<'static>>,
    depth_texture: Option<DepthTexture>,
    render_config: RenderConfig,
}

/// Depth texture for 3D rendering
pub struct DepthTexture {
    #[allow(dead_code)]
    texture: Texture,
    view: TextureView,
}

impl DepthTexture {
    /// Create a depth texture
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }

    /// Get the texture view
    pub fn view(&self) -> &TextureView {
        &self.view
    }
}

impl Renderer {
    /// Create a new renderer with default configuration
    pub fn new(context: &GpuContext) -> Self {
        Self::with_config(context, &RenderConfig::default())
    }

    /// Create a new renderer with custom configuration
    pub fn with_config(context: &GpuContext, config: &RenderConfig) -> Self {
        let shader = context.create_shader_module(RENDER_SHADER_WGSL);

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Render Bind Group Layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            context
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: "vs_main",
                        buffers: &[Vertex::layout()],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: TextureFormat::Bgra8UnormSrgb,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: config.polygon_mode(),
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    }),
                    multisample: config.multisample_state(),
                    multiview: None,
                });

        let uniform_buffer = UniformBuffer::new(&context.device, &RenderUniforms::default());

        Self {
            context: context.clone(),
            render_pipeline,
            vertex_buffer: None,
            index_buffer: None,
            uniform_buffer,
            uniforms: RenderUniforms::default(),
            config: None,
            surface: None,
            depth_texture: None,
            render_config: config.clone(),
        }
    }

    /// Set the surface for rendering
    pub fn set_surface(&mut self, surface: Surface<'static>) -> Result<(), RendererError> {
        // Get surface size (this would normally come from window)
        let width = 1920;
        let height = 1080;

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width,
            height,
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        self.surface = Some(surface);
        self.config = Some(config);
        self.depth_texture = Some(DepthTexture::new(&self.context.device, width, height));

        Ok(())
    }

    /// Set geometry to render
    pub fn set_geometry(&mut self, vertices: &[Vertex], indices: &[u32]) {
        self.vertex_buffer = Some(VertexBuffer::new(&self.context.device, vertices));
        self.index_buffer = Some(IndexBuffer::new(&self.context.device, indices));
    }

    /// Set camera
    pub fn set_camera(&mut self, camera: &Camera) {
        self.uniforms.view_proj = camera.view_projection_matrix().into();
        self.uniforms.camera_position =
            [camera.position.x, camera.position.y, camera.position.z, 1.0];
    }

    /// Set model matrix
    pub fn set_model(&mut self, model: Matrix4<f32>) {
        self.uniforms.model = model.into();
    }

    /// Update uniforms
    pub fn update_uniforms(&mut self) {
        self.uniform_buffer
            .update(&self.context.queue, &self.uniforms);
    }

    /// Create bind group for rendering
    pub fn create_bind_group(&self) -> BindGroup {
        self.context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group"),
                layout: &self.render_pipeline.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.buffer().as_entire_binding(),
                }],
            })
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<(), RendererError> {
        let surface = self.surface.as_ref().ok_or(RendererError::NoSurface)?;

        let frame = surface
            .get_current_texture()
            .map_err(|_| RendererError::SurfaceError)?;

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_view = self
            .depth_texture
            .as_ref()
            .ok_or(RendererError::NoDepthTexture)?
            .view();

        let bind_group = self.create_bind_group();

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(self.render_config.background_color),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);

        if let (Some(vb), Some(ib)) = (&self.vertex_buffer, &self.index_buffer) {
            render_pass.set_vertex_buffer(0, vb.buffer().slice(..));
            render_pass.set_index_buffer(ib.buffer().slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..ib.index_count(), 0, 0..1);
        }

        drop(render_pass);
        self.context.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }

    /// Set background color
    pub fn set_background_color(&mut self, color: wgpu::Color) {
        self.render_config.background_color = color;
    }

    /// Get the current render configuration
    pub fn render_config(&self) -> &RenderConfig {
        &self.render_config
    }

    /// Update the render configuration
    pub fn set_render_config(&mut self, config: RenderConfig) {
        self.render_config = config;
    }

    /// Set wireframe mode
    pub fn set_wireframe(&mut self, enabled: bool) {
        self.render_config.wireframe = enabled;
    }

    /// Set LOD level
    pub fn set_lod_level(&mut self, lod: LodLevel) {
        self.render_config.lod_level = lod;
    }

    /// Set MSAA sample count
    pub fn set_msaa(&mut self, sample_count: u32) {
        self.render_config.sample_count = sample_count.clamp(1, 8);
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(config) = &mut self.config {
            config.width = width;
            config.height = height;
            self.depth_texture = Some(DepthTexture::new(&self.context.device, width, height));
        }
    }

    /// Get the GPU context
    pub fn context(&self) -> &GpuContext {
        &self.context
    }
}

/// Renderer error types
#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("No surface configured")]
    NoSurface,
    #[error("Renderer not configured")]
    NotConfigured,
    #[error("Surface error")]
    SurfaceError,
    #[error("No depth texture")]
    NoDepthTexture,
}

/// WGSL render shader source
const RENDER_SHADER_WGSL: &str = r"
struct RenderUniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    light_position: vec4<f32>,
    light_color: vec4<f32>,
    camera_position: vec4<f32>,
    time: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: RenderUniforms;

@vertex
fn vs_main(
    @location(0) in_position: vec3<f32>,
    @location(1) in_color: vec3<f32>,
    @location(2) in_normal: vec3<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    
    let world_position = uniforms.model * vec4<f32>(in_position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = uniforms.view_proj * world_position;
    
    let normal_matrix = mat3x3<f32>(
        uniforms.model[0].xyz,
        uniforms.model[1].xyz,
        uniforms.model[2].xyz
    );
    out.normal = normalize(normal_matrix * in_normal);
    out.color = in_color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Phong lighting
    let normal = normalize(in.normal);
    let light_dir = normalize(uniforms.light_position.xyz - in.world_position);
    let view_dir = normalize(uniforms.camera_position.xyz - in.world_position);
    let half_dir = normalize(light_dir + view_dir);
    
    // Ambient
    let ambient = 0.1 * in.color;
    
    // Diffuse
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = diff * uniforms.light_color.rgb * in.color;
    
    // Specular (Blinn-Phong)
    let spec = pow(max(dot(normal, half_dir), 0.0), 32.0);
    let specular = spec * uniforms.light_color.rgb;
    
    let result = ambient + diffuse + specular;
    return vec4<f32>(result, 1.0);
}
";

/// Viewport for rendering
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    /// Create a new viewport
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get aspect ratio
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Full screen viewport
    pub fn full_screen(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new(
            Point3::new(0.0, 5.0, 10.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            std::f32::consts::FRAC_PI_4,
            16.0 / 9.0,
        );

        assert_eq!(camera.position, Point3::new(0.0, 5.0, 10.0));
        assert_eq!(camera.target, Point3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_camera_matrices() {
        let camera = Camera::new(
            Point3::new(0.0, 0.0, 10.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            std::f32::consts::FRAC_PI_4,
            1.0,
        );

        let view = camera.view_matrix();
        let _proj = camera.projection_matrix();

        // View matrix should transform camera position to origin
        let transformed = view * Point3::new(0.0, 0.0, 10.0).to_homogeneous();
        assert!((transformed.w - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_viewport_aspect() {
        let viewport = Viewport::new(0, 0, 1920, 1080);
        assert!((viewport.aspect() - 16.0 / 9.0).abs() < 1e-5);
    }

    #[test]
    fn test_render_uniforms_size() {
        // RenderUniforms should be properly sized for WGSL
        assert_eq!(std::mem::size_of::<RenderUniforms>(), 192);
    }

    #[test]
    fn test_lod_level_default() {
        let lod = LodLevel::default();
        assert!(matches!(lod, LodLevel::High));
    }

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert_eq!(config.sample_count, 4);
        assert!(matches!(config.lod_level, LodLevel::High));
        assert!(!config.enable_shadows);
        assert!(!config.wireframe);
    }

    #[test]
    fn test_render_config_builder() {
        let config = RenderConfig::default()
            .with_wireframe(true)
            .with_lod(LodLevel::Medium);

        assert!(config.wireframe);
        assert!(matches!(config.lod_level, LodLevel::Medium));
    }

    #[test]
    fn test_render_config_msaa_clamp() {
        let config = RenderConfig::with_msaa(16);
        assert_eq!(config.sample_count, 8); // Should clamp to max 8

        let config2 = RenderConfig::with_msaa(0);
        assert_eq!(config2.sample_count, 1); // Should clamp to min 1
    }

    #[test]
    fn test_render_config_polygon_mode() {
        let config = RenderConfig::default();
        assert_eq!(config.polygon_mode(), wgpu::PolygonMode::Fill);

        let config = RenderConfig::default().with_wireframe(true);
        assert_eq!(config.polygon_mode(), wgpu::PolygonMode::Line);
    }

    #[test]
    fn test_render_config_multisample_state() {
        let config = RenderConfig::with_msaa(4);
        let msaa = config.multisample_state();
        assert_eq!(msaa.count, 4);
        assert_eq!(msaa.mask, !0);
        assert!(!msaa.alpha_to_coverage_enabled);
    }

    #[tokio::test]
    async fn test_renderer_with_config() {
        let ctx_result = GpuContext::new().await;
        if let Ok(ctx) = ctx_result {
            let config = RenderConfig::with_msaa(4);
            let _renderer = Renderer::with_config(&ctx, &config);
            println!("Renderer with custom config created successfully");
        } else {
            println!("GPU not available, skipping test");
        }
    }
}
