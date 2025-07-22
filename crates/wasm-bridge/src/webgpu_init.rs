//! WebGPU initialization for WASM environment
//!
//! This module provides unified WebGPU initialization that can be shared
//! between the DataManager and Renderer components.

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlCanvasElement, GpuCanvasContext};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, TextureUsages, PresentMode};

/// Initialize WebGPU with a canvas element
pub async fn initialize_webgpu(
    canvas_id: &str,
) -> Result<(Device, Queue, Surface<'static>), JsValue> {
    // Get the canvas element
    let document = web_sys::window()
        .ok_or("No window found")?
        .document()
        .ok_or("No document found")?;
    
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or("Canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?;
    
    // Get canvas dimensions
    let width = canvas.client_width() as u32;
    let height = canvas.client_height() as u32;
    
    // Create WGPU instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        dx12_shader_compiler: Default::default(),
        flags: wgpu::InstanceFlags::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::default(),
    });
    
    // Create surface from canvas
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;
    
    // Request adapter
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .ok_or("Failed to find adapter")?;
    
    // Request device and queue
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("GPU Charts Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;
    
    // Configure the surface
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);
    
    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    
    surface.configure(&device, &config);
    
    Ok((device, queue, surface))
}

/// Reconfigure surface for new dimensions
pub fn reconfigure_surface(
    surface: &Surface,
    device: &Device,
    width: u32,
    height: u32,
) -> Result<(), JsValue> {
    let surface_caps = surface.get_capabilities(&device.adapter());
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);
    
    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    
    surface.configure(&device, &config);
    Ok(())
}