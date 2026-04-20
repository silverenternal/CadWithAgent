//! GPU buffer management for efficient geometry data transfer
//!
//! This module provides abstractions for managing GPU buffers used in compute
//! and rendering operations. It handles memory allocation, data transfer,
//! and buffer lifecycle management.

use bytemuck::{Pod as BytemuckPod, Zeroable};
use std::sync::Arc;
use wgpu::{Buffer, BufferAddress, BufferAsyncError, BufferUsages, Device, MapMode, Queue};

/// GPU buffer wrapper with automatic memory management
#[derive(Debug)]
pub struct GpuBuffer<T: BytemuckPod + Send + Sync> {
    buffer: Arc<Buffer>,
    size: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BytemuckPod + Send + Sync> GpuBuffer<T> {
    /// Create a new GPU buffer from a slice of data
    pub fn new(device: &Device, data: &[T], usage: BufferUsages) -> Self {
        let size = data.len();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuBuffer"),
            size: std::mem::size_of_val(data) as BufferAddress,
            usage: usage | BufferUsages::COPY_DST,
            mapped_at_creation: true, // Enable mapped_at_creation for initialization
        });

        // Write data to the mapped buffer
        {
            let slice = buffer.slice(..);
            let mapped = slice.get_mapped_range();
            // Use ptr::copy_nonoverlapping to copy data to mapped memory
            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr() as *const u8,
                    mapped.as_ptr() as *mut u8,
                    std::mem::size_of_val(data),
                );
            }
            // mapped is dropped here, which should unmap the buffer
        }
        // Explicitly unmap the buffer to ensure it's ready for GPU operations
        buffer.unmap();

        Self {
            buffer: Arc::new(buffer),
            size,
            _marker: std::marker::PhantomData,
        }
    }

    /// Create a new GPU buffer with uninitialized data (for compute output)
    pub fn uninitialized(device: &Device, size: usize, usage: BufferUsages) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GpuBuffer"),
            size: (size * std::mem::size_of::<T>()) as BufferAddress,
            usage: usage | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self {
            buffer: Arc::new(buffer),
            size,
            _marker: std::marker::PhantomData,
        }
    }

    /// Write data to the buffer
    pub fn write(&self, queue: &Queue, data: &[T]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(data));
    }

    /// Read data from the buffer (async, requires mapping)
    pub async fn read(&self, device: &Device, _queue: &Queue) -> Result<Vec<T>, BufferAsyncError> {
        let buffer_slice = self.buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();

        buffer_slice.map_async(MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        device.poll(wgpu::Maintain::Wait);

        let _ = rx.receive().await.ok_or(BufferAsyncError)?;

        let data = buffer_slice.get_mapped_range();
        let result = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        self.buffer.unmap();

        Ok(result)
    }

    /// Get the underlying wgpu buffer
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get the number of elements
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> usize {
        self.size * std::mem::size_of::<T>()
    }
}

impl<T: BytemuckPod + Send + Sync> Clone for GpuBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            buffer: Arc::clone(&self.buffer),
            size: self.size,
            _marker: std::marker::PhantomData,
        }
    }
}

/// Buffer builder for fluent GPU buffer creation
pub struct GpuBufferBuilder<T: BytemuckPod + Send + Sync> {
    usage: BufferUsages,
    _marker: std::marker::PhantomData<T>,
}

impl<T: BytemuckPod + Send + Sync> GpuBufferBuilder<T> {
    /// Create a new buffer builder
    pub fn new() -> Self {
        Self {
            usage: BufferUsages::empty(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Set buffer usage flags
    pub fn with_usage(mut self, usage: BufferUsages) -> Self {
        self.usage = usage;
        self
    }

    /// Add vertex buffer usage
    pub fn vertex(mut self) -> Self {
        self.usage |= BufferUsages::VERTEX;
        self
    }

    /// Add index buffer usage
    pub fn index(mut self) -> Self {
        self.usage |= BufferUsages::INDEX;
        self
    }

    /// Add uniform buffer usage
    pub fn uniform(mut self) -> Self {
        self.usage |= BufferUsages::UNIFORM;
        self
    }

    /// Add storage buffer usage
    pub fn storage(mut self) -> Self {
        self.usage |= BufferUsages::STORAGE;
        self
    }

    /// Add indirect buffer usage
    pub fn indirect(mut self) -> Self {
        self.usage |= BufferUsages::INDIRECT;
        self
    }

    /// Build the buffer from data
    pub fn build(self, device: &Device, data: &[T]) -> GpuBuffer<T> {
        GpuBuffer::new(device, data, self.usage)
    }

    /// Build an uninitialized buffer
    pub fn build_uninitialized(self, device: &Device, size: usize) -> GpuBuffer<T> {
        GpuBuffer::uninitialized(device, size, self.usage)
    }
}

impl<T: BytemuckPod + Send + Sync> Default for GpuBufferBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Vertex buffer for rendering
#[derive(Debug)]
pub struct VertexBuffer {
    buffer: GpuBuffer<Vertex>,
    vertex_count: u32,
}

/// Vertex structure for rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, BytemuckPod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    _padding: f32,
}

impl Vertex {
    /// Create a new vertex
    pub fn new(
        position: nalgebra::Vector3<f32>,
        color: nalgebra::Vector3<f32>,
        normal: nalgebra::Vector3<f32>,
    ) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            color: [color.x, color.y, color.z],
            normal: [normal.x, normal.y, normal.z],
            _padding: 0.0,
        }
    }

    /// Get the vertex buffer layout
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12, // 3 * 4 bytes for position
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 24, // 6 * 4 bytes for position + color
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

impl VertexBuffer {
    /// Create a new vertex buffer
    pub fn new(device: &Device, vertices: &[Vertex]) -> Self {
        let vertex_count = vertices.len() as u32;
        let buffer = GpuBufferBuilder::new().vertex().build(device, vertices);

        Self {
            buffer,
            vertex_count,
        }
    }

    /// Get the underlying buffer
    pub fn buffer(&self) -> &Buffer {
        self.buffer.buffer()
    }

    /// Get the vertex count
    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }
}

/// Index buffer for rendering
#[derive(Debug)]
pub struct IndexBuffer {
    buffer: GpuBuffer<u32>,
    index_count: u32,
}

impl IndexBuffer {
    /// Create a new index buffer
    pub fn new(device: &Device, indices: &[u32]) -> Self {
        let index_count = indices.len() as u32;
        let buffer = GpuBufferBuilder::new().index().build(device, indices);

        Self {
            buffer,
            index_count,
        }
    }

    /// Get the underlying buffer
    pub fn buffer(&self) -> &Buffer {
        self.buffer.buffer()
    }

    /// Get the index count
    pub fn index_count(&self) -> u32 {
        self.index_count
    }
}

/// Uniform buffer for passing data to shaders
#[derive(Debug)]
pub struct UniformBuffer<T: BytemuckPod + Send + Sync> {
    buffer: GpuBuffer<T>,
}

impl<T: BytemuckPod + Send + Sync> UniformBuffer<T> {
    /// Create a new uniform buffer
    pub fn new(device: &Device, data: &T) -> Self {
        let buffer = GpuBufferBuilder::new()
            .uniform()
            .build(device, std::slice::from_ref(data));

        Self { buffer }
    }

    /// Update the uniform data
    pub fn update(&self, queue: &Queue, data: &T) {
        self.buffer.write(queue, std::slice::from_ref(data));
    }

    /// Get the underlying buffer
    pub fn buffer(&self) -> &Buffer {
        self.buffer.buffer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_layout() {
        let layout = Vertex::layout();
        assert_eq!(
            layout.array_stride,
            std::mem::size_of::<Vertex>() as BufferAddress
        );
        assert_eq!(layout.attributes.len(), 3);
    }

    #[test]
    fn test_vertex_size() {
        // Vertex should be 40 bytes: 3 * f32 * 3 vectors + 1 f32 padding
        assert_eq!(std::mem::size_of::<Vertex>(), 40);
    }
}
