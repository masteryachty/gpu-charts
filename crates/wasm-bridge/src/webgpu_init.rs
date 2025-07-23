//! WebGPU initialization for WASM environment
//!
//! This module provides unified WebGPU initialization that can be shared
//! between the DataManager and Renderer components.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wgpu::{Device, PresentMode, Queue, Surface, SurfaceConfiguration, TextureUsages};

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
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        flags: wgpu::InstanceFlags::default(),
        backend_options: Default::default(),
    });

    // Create surface from canvas
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;

    // Request adapter
    web_sys::console::log_1(&"[WebGPU Init] Requesting adapter...".into());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to find adapter: {:?}", e)))?;

    // Log adapter info
    let adapter_info = adapter.get_info();
    web_sys::console::log_1(
        &format!("[WebGPU Init] Adapter found: {:?}", adapter_info.name).into(),
    );
    web_sys::console::log_1(
        &format!("[WebGPU Init] Adapter backend: {:?}", adapter_info.backend).into(),
    );

    // Use browser-compatible limits to avoid errors
    let limits = wgpu::Limits::downlevel_webgl2_defaults();
    web_sys::console::log_1(
        &format!(
            "[WebGPU Init] Using browser-compatible limits (downlevel_webgl2_defaults): {:?}",
            limits
        )
        .into(),
    );

    // Request device and queue
    web_sys::console::log_1(&"[WebGPU Init] Requesting device with default limits...".into());
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("GPU Charts Device"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            memory_hints: Default::default(),
            trace: Default::default(),
        })
        .await
        .map_err(|e| {
            web_sys::console::error_1(
                &format!("[WebGPU Init] Device request failed: {:?}", e).into(),
            );
            JsValue::from_str(&format!("Failed to create device: {:?}", e))
        })?;

    web_sys::console::log_1(&"[WebGPU Init] Device created successfully!".into());

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

// Reconfigure surface is not needed - the renderer handles this internally
