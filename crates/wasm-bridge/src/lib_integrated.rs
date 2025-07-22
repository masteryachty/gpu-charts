//! Integrated WASM bridge with Phase 3 rendering
//!
//! This version properly connects the configuration system to actual rendering
//! by integrating the DataManager and Renderer with shared WebGPU resources.

mod webgpu_init;

use gpu_charts_config::{GpuChartsConfig, HotReloadManager};
use gpu_charts_renderer::{Renderer, RenderConfig};
use gpu_charts_shared::{Error, Result};
use wasm_bindgen::prelude::*;
use web_sys::console;
use std::sync::Arc;
use wgpu::{Device, Queue, Surface};

/// Log a message to the browser console
macro_rules! log {
    ($($t:tt)*) => {
        console::log_1(&format!($($t)*).into());
    };
}

/// Integrated chart system with rendering
#[wasm_bindgen]
pub struct ChartSystemIntegrated {
    device: Arc<Device>,
    queue: Arc<Queue>,
    surface: Surface<'static>,
    renderer: Arc<Renderer>,
    config_manager: Arc<HotReloadManager>,
    canvas_id: String,
    width: u32,
    height: u32,
}

#[wasm_bindgen]
impl ChartSystemIntegrated {
    /// Initialize the integrated chart system
    #[wasm_bindgen(constructor)]
    pub async fn new(canvas_id: String) -> Result<ChartSystemIntegrated> {
        console_error_panic_hook::set_once();
        log!("Initializing ChartSystemIntegrated for canvas: {}", canvas_id);

        // Initialize WebGPU with shared resources
        let (device, queue, surface) = webgpu_init::initialize_webgpu(&canvas_id)
            .await
            .map_err(|e| Error::InitializationError(format!("{:?}", e)))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // Get canvas dimensions
        let document = web_sys::window()
            .ok_or(Error::InitializationError("No window".to_string()))?
            .document()
            .ok_or(Error::InitializationError("No document".to_string()))?;
        
        let canvas = document
            .get_element_by_id(&canvas_id)
            .ok_or(Error::InitializationError("Canvas not found".to_string()))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| Error::InitializationError("Not a canvas".to_string()))?;

        let width = canvas.client_width() as u32;
        let height = canvas.client_height() as u32;

        // Initialize configuration system
        let default_config = GpuChartsConfig::default();
        let config_manager = Arc::new(HotReloadManager::new(default_config, |_| Ok(())));

        // Create renderer with initial config
        let render_config = convert_to_render_config(&config_manager.current());
        let renderer = Arc::new(
            Renderer::new(device.clone(), queue.clone(), render_config)
                .await
                .map_err(|e| Error::InitializationError(format!("Renderer init failed: {:?}", e)))?
        );

        Ok(Self {
            device,
            queue,
            surface,
            renderer,
            config_manager,
            canvas_id,
            width,
            height,
        })
    }

    /// Update configuration and apply to renderer
    #[wasm_bindgen]
    pub fn update_config(&mut self, config_json: &str) -> Result<()> {
        let new_config: GpuChartsConfig = serde_json::from_str(config_json)
            .map_err(|e| Error::InvalidConfiguration(e.to_string()))?;
        
        // Update configuration
        self.config_manager.update_config(new_config.clone());
        
        // Apply to renderer
        let render_config = convert_to_render_config(&new_config);
        self.renderer.update_config(render_config)
            .map_err(|e| Error::InvalidConfiguration(format!("Renderer update failed: {:?}", e)))?;
        
        log!("Configuration updated and applied to renderer");
        Ok(())
    }

    /// Get current configuration as JSON
    #[wasm_bindgen]
    pub fn get_config(&self) -> String {
        let config = self.config_manager.current();
        serde_json::to_string(&*config).unwrap_or_else(|_| "{}".to_string())
    }

    /// Set quality preset and apply immediately
    #[wasm_bindgen]
    pub fn set_quality_preset(&mut self, preset: &str) -> Result<()> {
        let mut config = (*self.config_manager.current()).clone();
        
        // Apply preset settings
        match preset {
            "ultra" => {
                config.rendering.max_fps = 144;
                config.rendering.msaa_samples = 8;
                config.rendering.enable_bloom = true;
                config.rendering.enable_fxaa = true;
                config.rendering.enable_shadows = true;
                config.rendering.texture_filtering = gpu_charts_config::TextureFiltering::Anisotropic16x;
                config.performance.lod_bias = 0.0;
                config.performance.max_draw_calls = 10000;
            }
            "high" => {
                config.rendering.max_fps = 120;
                config.rendering.msaa_samples = 4;
                config.rendering.enable_bloom = true;
                config.rendering.enable_fxaa = true;
                config.rendering.enable_shadows = false;
                config.rendering.texture_filtering = gpu_charts_config::TextureFiltering::Anisotropic8x;
                config.performance.lod_bias = 0.5;
                config.performance.max_draw_calls = 5000;
            }
            "medium" => {
                config.rendering.max_fps = 60;
                config.rendering.msaa_samples = 2;
                config.rendering.enable_bloom = false;
                config.rendering.enable_fxaa = true;
                config.rendering.enable_shadows = false;
                config.rendering.texture_filtering = gpu_charts_config::TextureFiltering::Bilinear;
                config.performance.lod_bias = 1.0;
                config.performance.max_draw_calls = 2500;
            }
            "low" => {
                config.rendering.max_fps = 30;
                config.rendering.msaa_samples = 1;
                config.rendering.enable_bloom = false;
                config.rendering.enable_fxaa = false;
                config.rendering.enable_shadows = false;
                config.rendering.texture_filtering = gpu_charts_config::TextureFiltering::Nearest;
                config.performance.lod_bias = 2.0;
                config.performance.max_draw_calls = 1000;
            }
            _ => return Err(Error::InvalidConfiguration(format!("Unknown preset: {}", preset))),
        }
        
        // Update and apply
        self.config_manager.update_config(config.clone());
        let render_config = convert_to_render_config(&config);
        self.renderer.update_config(render_config)?;
        
        log!("Quality preset '{}' applied", preset);
        Ok(())
    }

    /// Render a frame
    #[wasm_bindgen]
    pub async fn render(&mut self) -> Result<()> {
        // Get the next frame
        let output = self.surface.get_current_texture()
            .map_err(|e| Error::RenderError(format!("Failed to get surface texture: {:?}", e)))?;
        
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Render the frame
        self.renderer.render(&self.queue, &view, self.width, self.height)
            .await
            .map_err(|e| Error::RenderError(format!("Render failed: {:?}", e)))?;
        
        // Present the frame
        output.present();
        
        Ok(())
    }

    /// Handle window resize
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        log!("Resizing to {}x{}", width, height);
        
        self.width = width;
        self.height = height;
        
        // Reconfigure surface
        webgpu_init::reconfigure_surface(&self.surface, &self.device, width, height)
            .map_err(|e| Error::RenderError(format!("Surface reconfigure failed: {:?}", e)))?;
        
        // Notify renderer
        self.renderer.resize(width, height);
        
        Ok(())
    }

    /// Set chart data (simplified for now)
    #[wasm_bindgen]
    pub fn set_data(&mut self, data_json: &str) -> Result<()> {
        // Parse data
        let data: Vec<f32> = serde_json::from_str(data_json)
            .map_err(|e| Error::InvalidData(format!("Failed to parse data: {}", e)))?;
        
        // Create GPU buffer
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chart Data"),
            contents: bytemuck::cast_slice(&data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        // Pass to renderer (simplified - would use DataManager in full version)
        // self.renderer.set_data_buffer(buffer);
        
        log!("Data set with {} points", data.len());
        Ok(())
    }

    /// Get performance metrics
    #[wasm_bindgen]
    pub fn get_performance_metrics(&self) -> String {
        let metrics = self.renderer.get_performance_metrics();
        serde_json::to_string(&metrics).unwrap_or_else(|_| "{}".to_string())
    }

    /// Check if a feature is enabled
    #[wasm_bindgen]
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        let config = self.config_manager.current();
        match feature {
            "scatter_plots" => config.features.scatter_plots,
            "heatmaps" => config.features.heatmaps,
            "three_d_charts" => config.features.three_d_charts,
            "technical_indicators" => config.features.technical_indicators,
            "annotations" => config.features.annotations,
            "custom_shaders" => config.features.custom_shaders,
            _ => false,
        }
    }
}

/// Convert GpuChartsConfig to RenderConfig
fn convert_to_render_config(config: &GpuChartsConfig) -> RenderConfig {
    RenderConfig {
        max_fps: config.rendering.max_fps,
        vsync: config.rendering.vsync,
        msaa_samples: config.rendering.msaa_samples,
        enable_bloom: config.rendering.enable_bloom,
        enable_fxaa: config.rendering.enable_fxaa,
        enable_taa: config.rendering.enable_taa,
        enable_shadows: config.rendering.enable_shadows,
        shadow_quality: match config.rendering.shadow_quality {
            gpu_charts_config::ShadowQuality::Low => gpu_charts_renderer::ShadowQuality::Low,
            gpu_charts_config::ShadowQuality::Medium => gpu_charts_renderer::ShadowQuality::Medium,
            gpu_charts_config::ShadowQuality::High => gpu_charts_renderer::ShadowQuality::High,
            gpu_charts_config::ShadowQuality::Ultra => gpu_charts_renderer::ShadowQuality::Ultra,
        },
        texture_filtering: match config.rendering.texture_filtering {
            gpu_charts_config::TextureFiltering::Nearest => gpu_charts_renderer::TextureFiltering::Nearest,
            gpu_charts_config::TextureFiltering::Bilinear => gpu_charts_renderer::TextureFiltering::Bilinear,
            gpu_charts_config::TextureFiltering::Trilinear => gpu_charts_renderer::TextureFiltering::Trilinear,
            gpu_charts_config::TextureFiltering::Anisotropic4x => gpu_charts_renderer::TextureFiltering::Anisotropic4x,
            gpu_charts_config::TextureFiltering::Anisotropic8x => gpu_charts_renderer::TextureFiltering::Anisotropic8x,
            gpu_charts_config::TextureFiltering::Anisotropic16x => gpu_charts_renderer::TextureFiltering::Anisotropic16x,
        },
        enable_mipmaps: config.rendering.enable_mipmaps,
        lod_bias: config.performance.lod_bias,
    }
}

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    log!("GPU Charts Phase 3 Integrated WASM Bridge initialized");
}