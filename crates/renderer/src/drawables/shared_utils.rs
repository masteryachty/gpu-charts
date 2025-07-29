//! Shared utilities for drawable components

use wgpu::{BindGroupLayout, Device};

/// Minimum pixel spacing between axis labels/grid lines
pub const MIN_AXIS_LABEL_SPACING: f32 = 100.0;

/// Creates the shared bind group layout used by axis renderers and DataStore
/// This layout contains:
/// - Binding 0: X range buffer (vec2<u32> for min/max timestamps)
/// - Binding 1: Y range buffer (vec2<f32> for min/max values)
pub fn create_shared_range_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("shared_range_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    })
}
