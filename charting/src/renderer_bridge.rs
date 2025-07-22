//! Bridge between the new modular renderer architecture and the existing web integration

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlCanvasElement;

use gpu_charts_shared::{ChartConfiguration, ChartType, VisualConfig, DataHandle, TimeRange, DataMetadata};
use gpu_charts_renderer::{Renderer, Viewport, GpuBufferSet};
use gpu_charts_data::{DataManager, DataRequest};

use crate::renderer::data_retriever::fetch_data;
use crate::controls::canvas_controller::CanvasController;

/// Bridge structure that connects the new renderer with web integration
pub struct RendererBridge {
    renderer: Rc<RefCell<Renderer>>,
    data_manager: Rc<RefCell<DataManager>>,
    canvas_controller: Option<CanvasController>,
    current_config: Option<ChartConfiguration>,
}

impl RendererBridge {
    /// Create a new renderer bridge
    pub async fn new(canvas: HtmlCanvasElement, width: u32, height: u32) -> Result<Self, JsValue> {
        // Create WebGPU instance and adapter
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| JsValue::from_str("Failed to get WebGPU adapter"))?;
            
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Renderer Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;
            
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        
        // Create surface from canvas
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {:?}", e)))?;
            
        // Create renderer
        let renderer = Renderer::new_with_device(
            device.clone(),
            queue.clone(),
            surface,
            width,
            height,
        ).map_err(|e| JsValue::from_str(&format!("Failed to create renderer: {:?}", e)))?;
        
        // Create data manager
        let data_manager = DataManager::new_with_device(device, queue)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create data manager: {:?}", e)))?;
            
        Ok(Self {
            renderer: Rc::new(RefCell::new(renderer)),
            data_manager: Rc::new(RefCell::new(data_manager)),
            canvas_controller: None,
            current_config: None,
        })
    }
    
    /// Initialize with configuration
    pub async fn init_with_config(&mut self, config: ChartConfiguration) -> Result<(), JsValue> {
        // Validate configuration
        gpu_charts_renderer::config::ConfigValidator::validate(&config)
            .map_err(|e| JsValue::from_str(&format!("Invalid configuration: {:?}", e)))?;
            
        // Update renderer configuration
        self.renderer.borrow_mut().update_config(config.clone())
            .map_err(|e| JsValue::from_str(&format!("Failed to update config: {:?}", e)))?;
            
        // Create canvas controller for user interactions
        let data_manager = self.data_manager.clone();
        let renderer = self.renderer.clone();
        
        let controller = CanvasController::new(move |request| {
            let data_manager = data_manager.clone();
            let renderer = renderer.clone();
            
            spawn_local(async move {
                // Request data from data manager
                match data_manager.borrow_mut().request_data(request).await {
                    Ok(handle) => {
                        // Get GPU buffers from data manager
                        if let Some(buffers) = data_manager.borrow().get_gpu_buffers(&handle) {
                            // Register buffers with renderer
                            renderer.borrow_mut().register_buffer_set(handle, buffers);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to fetch data: {:?}", e);
                    }
                }
            });
        });
        
        self.canvas_controller = Some(controller);
        self.current_config = Some(config);
        
        Ok(())
    }
    
    /// Load initial data based on time range
    pub async fn load_data(&mut self, symbol: String, time_range: TimeRange) -> Result<(), JsValue> {
        let request = DataRequest {
            symbol: symbol.clone(),
            columns: vec!["time".to_string(), "price".to_string()],
            time_range,
            aggregation: None,
        };
        
        // Request data from data manager
        let handle = self.data_manager.borrow_mut()
            .request_data(request)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to request data: {:?}", e)))?;
            
        // Get GPU buffers
        let buffers = self.data_manager.borrow()
            .get_gpu_buffers(&handle)
            .ok_or_else(|| JsValue::from_str("Failed to get GPU buffers"))?;
            
        // Register with renderer
        self.renderer.borrow_mut().register_buffer_set(handle, buffers);
        
        Ok(())
    }
    
    /// Render a frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        self.renderer.borrow_mut()
            .render()
            .map_err(|e| JsValue::from_str(&format!("Render failed: {:?}", e)))
    }
    
    /// Handle resize
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.borrow_mut().resize(width, height);
    }
    
    /// Update viewport (for pan/zoom)
    pub fn update_viewport(&mut self, viewport: Viewport) {
        self.renderer.borrow_mut().update_viewport(viewport);
    }
    
    /// Handle mouse wheel event
    pub async fn handle_mouse_wheel(&mut self, delta: f64, x: f64, y: f64) -> Result<(), JsValue> {
        if let Some(controller) = &mut self.canvas_controller {
            controller.handle_mouse_wheel(delta, x, y).await?;
        }
        Ok(())
    }
    
    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> String {
        let metrics = self.renderer.borrow().get_performance_metrics();
        serde_json::to_string(&metrics).unwrap_or_default()
    }
    
    /// Get detailed stats
    pub fn get_stats(&self) -> String {
        let stats = self.renderer.borrow().get_stats();
        stats.to_string()
    }
}

/// Helper to create default chart configuration
pub fn create_default_config(chart_type: ChartType) -> ChartConfiguration {
    ChartConfiguration {
        chart_type,
        visual_config: VisualConfig::default(),
        overlays: vec![],
        data_handles: vec![],
    }
}