//! Rendering-focused WASM bridge
//!
//! This version focuses on getting basic rendering working with the Phase 3 architecture

use gpu_charts_config::{GpuChartsConfig, HotReloadManager};
use gpu_charts_shared::{ChartConfiguration, ChartType, DataHandle, VisualConfig, Error, Result};
use wasm_bindgen::prelude::*;
use web_sys::console;
use std::sync::Arc;

/// Log a message to the browser console
macro_rules! log {
    ($($t:tt)*) => {
        console::log_1(&format!($($t)*).into());
    };
}

/// Chart system with basic rendering
#[wasm_bindgen]
pub struct ChartSystemWithRendering {
    config_manager: Arc<HotReloadManager>,
    canvas_id: String,
    device: Option<Arc<wgpu::Device>>,
    queue: Option<Arc<wgpu::Queue>>,
    surface: Option<wgpu::Surface<'static>>,
    renderer: Option<gpu_charts_renderer::Renderer>,
    width: u32,
    height: u32,
}

#[wasm_bindgen]
impl ChartSystemWithRendering {
    /// Create a new chart system
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: String) -> Result<ChartSystemWithRendering> {
        console_error_panic_hook::set_once();
        log!("Initializing ChartSystemWithRendering for canvas: {}", canvas_id);

        // Initialize configuration
        let default_config = GpuChartsConfig::default();
        let config_manager = Arc::new(HotReloadManager::new(default_config, |_| Ok(())));

        // Get canvas dimensions
        let (width, height) = get_canvas_dimensions(&canvas_id)?;

        Ok(Self {
            config_manager,
            canvas_id,
            device: None,
            queue: None,
            surface: None,
            renderer: None,
            width,
            height,
        })
    }

    /// Initialize WebGPU and renderer
    #[wasm_bindgen]
    pub async fn initialize_rendering(&mut self) -> Result<()> {
        log!("Initializing WebGPU rendering...");

        // Initialize WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
        });

        // Get canvas
        let canvas = get_canvas(&self.canvas_id)?;
        
        // Create surface
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| Error::InitializationError(format!("Failed to create surface: {:?}", e)))?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(Error::InitializationError("Failed to find adapter".to_string()))?;

        // Request device
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
            .map_err(|e| Error::InitializationError(format!("Failed to create device: {:?}", e)))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Configure surface
        configure_surface(&surface, &adapter, &device, self.width, self.height);

        // Create renderer
        let renderer = gpu_charts_renderer::Renderer::new_with_device(
            device.clone(),
            queue.clone(),
            surface,
            self.width,
            self.height,
        ).map_err(|e| Error::InitializationError(format!("Renderer creation failed: {:?}", e)))?;

        self.device = Some(device);
        self.queue = Some(queue);
        self.renderer = Some(renderer);

        log!("WebGPU rendering initialized successfully");
        Ok(())
    }

    /// Update configuration
    #[wasm_bindgen]
    pub fn update_config(&mut self, config_json: &str) -> Result<()> {
        let new_config: GpuChartsConfig = serde_json::from_str(config_json)
            .map_err(|e| Error::InvalidConfiguration(e.to_string()))?;
        
        self.config_manager.update_config(new_config);
        
        // Apply to renderer if initialized
        if let Some(renderer) = &mut self.renderer {
            let chart_config = create_chart_configuration(&new_config);
            renderer.update_config(chart_config)?;
        }
        
        log!("Configuration updated");
        Ok(())
    }

    /// Set quality preset
    #[wasm_bindgen]
    pub fn set_quality_preset(&mut self, preset: &str) -> Result<()> {
        let mut config = (*self.config_manager.current()).clone();
        
        match preset {
            "ultra" => {
                config.rendering.max_fps = 144;
                config.rendering.msaa_samples = 8;
                config.rendering.enable_bloom = true;
                config.rendering.enable_fxaa = true;
            }
            "high" => {
                config.rendering.max_fps = 120;
                config.rendering.msaa_samples = 4;
                config.rendering.enable_bloom = true;
                config.rendering.enable_fxaa = true;
            }
            "medium" => {
                config.rendering.max_fps = 60;
                config.rendering.msaa_samples = 2;
                config.rendering.enable_bloom = false;
                config.rendering.enable_fxaa = true;
            }
            "low" => {
                config.rendering.max_fps = 30;
                config.rendering.msaa_samples = 1;
                config.rendering.enable_bloom = false;
                config.rendering.enable_fxaa = false;
            }
            _ => return Err(Error::InvalidConfiguration(format!("Unknown preset: {}", preset))),
        }
        
        self.config_manager.update_config(config.clone());
        
        // Apply to renderer
        if let Some(renderer) = &mut self.renderer {
            let chart_config = create_chart_configuration(&config);
            renderer.update_config(chart_config)?;
        }
        
        log!("Quality preset '{}' applied", preset);
        Ok(())
    }

    /// Render a frame
    #[wasm_bindgen]
    pub async fn render(&mut self) -> Result<()> {
        if self.renderer.is_none() {
            return Err(Error::NotInitialized("Renderer not initialized".to_string()));
        }

        let renderer = self.renderer.as_mut().unwrap();
        let queue = self.queue.as_ref().unwrap();
        
        renderer.render(queue).await?;
        
        Ok(())
    }

    /// Handle resize
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.width = width;
        self.height = height;
        
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(width, height)?;
        }
        
        Ok(())
    }

    /// Get performance metrics
    #[wasm_bindgen]
    pub fn get_performance_metrics(&self) -> String {
        if let Some(renderer) = &self.renderer {
            let metrics = renderer.get_performance_metrics();
            serde_json::json!({
                "fps": 1000.0 / metrics.frame_time_ms,
                "frame_time_ms": metrics.frame_time_ms,
                "gpu_time_ms": metrics.gpu_time_ms,
                "cpu_time_ms": metrics.cpu_time_ms,
                "draw_calls": metrics.draw_calls,
                "vertices": metrics.vertices_rendered,
                "triangles": metrics.triangles_rendered,
            }).to_string()
        } else {
            "{}".to_string()
        }
    }

    /// Check if rendering is initialized
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        self.renderer.is_some()
    }

    /// Get current configuration
    #[wasm_bindgen]
    pub fn get_config(&self) -> String {
        let config = self.config_manager.current();
        serde_json::to_string(&*config).unwrap_or_else(|_| "{}".to_string())
    }
}

// Helper functions

fn get_canvas_dimensions(canvas_id: &str) -> Result<(u32, u32)> {
    let canvas = get_canvas(canvas_id)?;
    Ok((canvas.client_width() as u32, canvas.client_height() as u32))
}

fn get_canvas(canvas_id: &str) -> Result<web_sys::HtmlCanvasElement> {
    let document = web_sys::window()
        .ok_or(Error::InitializationError("No window".to_string()))?
        .document()
        .ok_or(Error::InitializationError("No document".to_string()))?;
    
    document
        .get_element_by_id(canvas_id)
        .ok_or(Error::InitializationError("Canvas not found".to_string()))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| Error::InitializationError("Not a canvas".to_string()))
}

fn configure_surface(
    surface: &wgpu::Surface,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    width: u32,
    height: u32,
) {
    let surface_caps = surface.get_capabilities(adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);
    
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    
    surface.configure(device, &config);
}

fn create_chart_configuration(config: &GpuChartsConfig) -> ChartConfiguration {
    ChartConfiguration {
        chart_type: ChartType::Line, // Default for now
        data_handles: vec![], // Will be populated when data is set
        visual_config: VisualConfig {
            background_color: [0.1, 0.1, 0.1, 1.0],
            grid_color: [0.3, 0.3, 0.3, 0.5],
            text_color: [0.9, 0.9, 0.9, 1.0],
            line_colors: vec![
                [0.0, 0.7, 1.0, 1.0], // Blue
                [1.0, 0.5, 0.0, 1.0], // Orange
                [0.0, 1.0, 0.5, 1.0], // Green
                [1.0, 0.0, 0.5, 1.0], // Pink
            ],
            line_width: 2.0,
            point_size: 4.0,
            margin_percent: 0.05,
            show_grid: true,
            show_axes: true,
            animation_speed: 1.0,
        },
        overlays: vec![],
        interactions: gpu_charts_shared::InteractionConfig {
            enable_zoom: true,
            enable_pan: true,
            enable_hover: true,
            enable_selection: false,
        },
        time_range: None,
        y_range: None,
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    log!("GPU Charts Rendering WASM Bridge initialized");
}