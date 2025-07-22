//! Unified API for GPU Charts system

use crate::{
    bridge::{DataManagerBridge, RendererBridge},
    lifecycle::LifecycleCoordinator,
    IntegrationError, Result,
};
use gpu_charts_data::{BufferMetadata, DataSource};
use gpu_charts_renderer::{PerformanceMetrics as RendererMetrics, Viewport};
use gpu_charts_shared::{ChartConfiguration, ChartType};
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

/// Main unified API for GPU Charts
#[derive(Clone)]
pub struct UnifiedApi {
    /// Data manager bridge
    data_bridge: DataManagerBridge,

    /// Renderer bridge
    renderer_bridge: RendererBridge,

    /// Lifecycle coordinator
    lifecycle: LifecycleCoordinator,

    /// Active charts
    charts: Arc<RwLock<std::collections::HashMap<Uuid, ChartHandle>>>,

    /// API version
    version: ApiVersion,
}

/// API version information
#[derive(Debug, Clone)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Default for ApiVersion {
    fn default() -> Self {
        Self {
            major: 1,
            minor: 0,
            patch: 0,
        }
    }
}

/// Handle to an active chart
pub struct ChartHandle {
    pub id: Uuid,
    pub config: ChartConfiguration,
    pub data_handles: Vec<Uuid>,
    pub viewport: Viewport,
}

impl UnifiedApi {
    /// Create a new unified API
    pub fn new(
        data_bridge: DataManagerBridge,
        renderer_bridge: RendererBridge,
        lifecycle: LifecycleCoordinator,
    ) -> Self {
        Self {
            data_bridge,
            renderer_bridge,
            lifecycle,
            charts: Arc::new(RwLock::new(std::collections::HashMap::new())),
            version: ApiVersion::default(),
        }
    }

    /// Get API version
    pub fn version(&self) -> &ApiVersion {
        &self.version
    }

    /// Initialize the system
    pub async fn initialize(&self) -> Result<()> {
        self.lifecycle.initialize().await?;
        Ok(())
    }

    /// Create a new chart
    pub async fn create_chart(&self, config: ChartConfiguration) -> Result<Uuid> {
        // Ensure system is ready
        if self.lifecycle.get_state() != crate::lifecycle::LifecycleState::Ready
            && self.lifecycle.get_state() != crate::lifecycle::LifecycleState::Running
        {
            return Err(IntegrationError::Lifecycle(
                "System not ready for chart creation".to_string(),
            ));
        }

        let chart_id = Uuid::new_v4();
        let chart_handle = ChartHandle {
            id: chart_id,
            config,
            data_handles: Vec::new(),
            viewport: Viewport::default(),
        };

        self.charts.write().insert(chart_id, chart_handle);

        // Register resource
        self.lifecycle.register_resource(
            crate::lifecycle::ResourceType::RenderPipeline,
            chart_id.to_string(),
        );

        Ok(chart_id)
    }

    /// Load data for a chart
    pub async fn load_chart_data(
        &self,
        chart_id: Uuid,
        source: DataSource,
        metadata: BufferMetadata,
    ) -> Result<Uuid> {
        let data_handle_id = self.data_bridge.load_data(source, metadata).await?;

        // Associate with chart
        if let Some(chart) = self.charts.write().get_mut(&chart_id) {
            chart.data_handles.push(data_handle_id);
        } else {
            return Err(IntegrationError::Bridge("Chart not found".to_string()));
        }

        Ok(data_handle_id)
    }

    /// Update chart viewport
    pub fn update_viewport(&self, chart_id: Uuid, viewport: Viewport) -> Result<()> {
        if let Some(chart) = self.charts.write().get_mut(&chart_id) {
            chart.viewport = viewport;

            // Trigger prefetching
            let _ = self.data_bridge.prefetch_viewport_data(&viewport, 1.0);

            Ok(())
        } else {
            Err(IntegrationError::Bridge("Chart not found".to_string()))
        }
    }

    /// Render a chart
    pub fn render_chart(
        &self,
        chart_id: Uuid,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        metrics: &RendererMetrics,
    ) -> Result<()> {
        let chart = self
            .charts
            .read()
            .get(&chart_id)
            .ok_or_else(|| IntegrationError::Bridge("Chart not found".to_string()))?
            .clone();

        // Get data handles
        let mut buffer_handles = Vec::new();
        for handle_id in &chart.data_handles {
            if let Some(handle) = self.data_bridge.get_handle(handle_id) {
                buffer_handles.push(handle);
            }
        }

        // Render
        self.renderer_bridge.render(
            encoder,
            surface_view,
            &buffer_handles,
            &chart.viewport,
            metrics,
        )?;

        Ok(())
    }

    /// Delete a chart
    pub async fn delete_chart(&self, chart_id: Uuid) -> Result<()> {
        if let Some(chart) = self.charts.write().remove(&chart_id) {
            // Release data handles
            for handle_id in chart.data_handles {
                self.data_bridge.release_handle(&handle_id);
            }

            // Unregister resource
            self.lifecycle.unregister_resource(
                crate::lifecycle::ResourceType::RenderPipeline,
                chart_id.to_string(),
            );

            Ok(())
        } else {
            Err(IntegrationError::Bridge("Chart not found".to_string()))
        }
    }

    /// Get chart information
    pub fn get_chart_info(&self, chart_id: Uuid) -> Option<ChartInfo> {
        self.charts.read().get(&chart_id).map(|chart| ChartInfo {
            id: chart.id,
            chart_type: chart.config.chart_type,
            data_count: chart.data_handles.len(),
            viewport: chart.viewport.clone(),
        })
    }

    /// List all charts
    pub fn list_charts(&self) -> Vec<ChartInfo> {
        self.charts
            .read()
            .values()
            .map(|chart| ChartInfo {
                id: chart.id,
                chart_type: chart.config.chart_type,
                data_count: chart.data_handles.len(),
                viewport: chart.viewport.clone(),
            })
            .collect()
    }

    /// Handle window resize
    pub fn handle_resize(&self, new_size: (u32, u32)) -> Result<()> {
        self.renderer_bridge.handle_resize(new_size)?;
        Ok(())
    }

    /// Shutdown the system
    pub async fn shutdown(&self) -> Result<()> {
        // Delete all charts
        let chart_ids: Vec<_> = self.charts.read().keys().cloned().collect();
        for chart_id in chart_ids {
            let _ = self.delete_chart(chart_id).await;
        }

        // Shutdown lifecycle
        self.lifecycle.shutdown().await?;

        Ok(())
    }
}

/// Chart information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChartInfo {
    pub id: Uuid,
    pub chart_type: ChartType,
    pub data_count: usize,
    pub viewport: Viewport,
}

/// Fluent API builder for chart creation
pub struct ChartBuilder {
    config: ChartConfiguration,
    data_sources: Vec<(DataSource, BufferMetadata)>,
    viewport: Option<Viewport>,
}

impl ChartBuilder {
    /// Create a new chart builder
    pub fn new(chart_type: ChartType) -> Self {
        Self {
            config: ChartConfiguration {
                chart_type,
                visual_config: Default::default(),
                data_handles: Vec::new(),
                overlays: Vec::new(),
            },
            data_sources: Vec::new(),
            viewport: None,
        }
    }

    /// Set visual configuration
    pub fn with_visual_config(mut self, config: gpu_charts_shared::VisualConfig) -> Self {
        self.config.visual_config = config;
        self
    }

    /// Add a data source
    pub fn add_data(mut self, source: DataSource, metadata: BufferMetadata) -> Self {
        self.data_sources.push((source, metadata));
        self
    }

    /// Set viewport
    pub fn with_viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = Some(viewport);
        self
    }

    /// Build and create the chart
    pub async fn build(self, api: &UnifiedApi) -> Result<Uuid> {
        // Create chart
        let chart_id = api.create_chart(self.config).await?;

        // Load data
        for (source, metadata) in self.data_sources {
            api.load_chart_data(chart_id, source, metadata).await?;
        }

        // Set viewport if provided
        if let Some(viewport) = self.viewport {
            api.update_viewport(chart_id, viewport)?;
        }

        Ok(chart_id)
    }
}

/// TypeScript definitions generator
pub struct TypeScriptGenerator;

impl TypeScriptGenerator {
    /// Generate TypeScript definitions for the API
    pub fn generate() -> String {
        r#"// Auto-generated TypeScript definitions for GPU Charts API

export interface GpuChartsApi {
    version(): ApiVersion;
    initialize(): Promise<void>;
    createChart(config: ChartConfiguration): Promise<string>;
    loadChartData(chartId: string, source: DataSource, metadata: BufferMetadata): Promise<string>;
    updateViewport(chartId: string, viewport: Viewport): void;
    renderChart(chartId: string, canvas: HTMLCanvasElement): void;
    deleteChart(chartId: string): Promise<void>;
    getChartInfo(chartId: string): ChartInfo | null;
    listCharts(): ChartInfo[];
    handleResize(width: number, height: number): void;
    shutdown(): Promise<void>;
}

export interface ApiVersion {
    major: number;
    minor: number;
    patch: number;
}

export interface ChartConfiguration {
    chartType: ChartType;
    visualConfig: VisualConfig;
    overlays: OverlayConfig[];
}

export enum ChartType {
    Line = "Line",
    Scatter = "Scatter",
    Heatmap = "Heatmap",
    ThreeD = "ThreeD",
}

export interface VisualConfig {
    backgroundColor: [number, number, number, number];
    gridColor: [number, number, number, number];
    textColor: [number, number, number, number];
    marginPercent: number;
    showGrid: boolean;
    showAxes: boolean;
}

export interface OverlayConfig {
    overlayType: string;
    renderLocation: RenderLocation;
}

export enum RenderLocation {
    AbovePlot = "AbovePlot",
    BelowPlot = "BelowPlot",
}

export interface DataSource {
    type: "Http" | "WebSocket" | "File";
    url?: string;
    path?: string;
}

export interface BufferMetadata {
    rowCount: number;
    columnCount: number;
    timeRange: [number, number];
    valueRange: [number, number];
}

export interface Viewport {
    xMin: number;
    xMax: number;
    yMin: number;
    yMax: number;
}

export interface ChartInfo {
    id: string;
    chartType: ChartType;
    dataCount: number;
    viewport: Viewport;
}

export interface PerformanceMetrics {
    fps: number;
    frameTime: number;
    gpuTime: number;
    cpuTime: number;
}
"#
        .to_string()
    }
}
