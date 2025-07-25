//! Example usage of TriangleRenderer
//! This file demonstrates how to integrate the TriangleRenderer with the MultiRenderer system

use crate::charts::TriangleRenderer;
use crate::multi_renderer::{MultiRenderer, MultiRendererBuilder, RenderOrder};
use crate::{PlotRenderer, XAxisRenderer, YAxisRenderer, CandlestickRenderer};
use shared_types::{TradeData, TradeSide};
use std::rc::Rc;

/// Example: Create a multi-renderer with candlesticks and trade markers
pub fn create_candles_with_trades_renderer(
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    trades: Vec<TradeData>,
) -> MultiRenderer {
    let mut builder = MultiRendererBuilder::new(device.clone(), queue.clone(), format)
        .with_render_order(RenderOrder::BackgroundToForeground);

    // Build the multi-renderer
    let mut multi_renderer = builder.build();

    // Add candlestick renderer (background)
    let candle_renderer = CandlestickRenderer::new(
        device.clone(),
        queue.clone(),
        format,
    );
    multi_renderer.add_renderer(Box::new(candle_renderer));

    // Add triangle renderer for trades (foreground)
    let mut triangle_renderer = TriangleRenderer::new(
        device.clone(),
        queue.clone(),
        format,
    );
    
    // Set trade data
    triangle_renderer.update_trades(&trades);
    
    // Optionally customize triangle size
    triangle_renderer.set_triangle_size(10.0); // 10 pixels
    
    multi_renderer.add_renderer(Box::new(triangle_renderer));

    // Add axes on top
    let x_axis = XAxisRenderer::new(device.clone(), queue.clone(), format, width, height);
    multi_renderer.add_renderer(Box::new(x_axis));

    let y_axis = YAxisRenderer::new(device.clone(), queue.clone(), format, width, height);
    multi_renderer.add_renderer(Box::new(y_axis));

    multi_renderer
}

/// Example: Create a line chart with trade markers
pub fn create_line_with_trades_renderer(
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    trades: Vec<TradeData>,
) -> MultiRenderer {
    let mut multi_renderer = MultiRendererBuilder::new(device.clone(), queue.clone(), format)
        .with_render_order(RenderOrder::Sequential)
        .build();

    // Add plot renderer for lines
    let plot_renderer = PlotRenderer::new(device.clone(), queue.clone(), format);
    multi_renderer.add_renderer(Box::new(plot_renderer));

    // Add triangle renderer for trades
    let mut triangle_renderer = TriangleRenderer::new(
        device.clone(),
        queue.clone(),
        format,
    );
    triangle_renderer.update_trades(&trades);
    multi_renderer.add_renderer(Box::new(triangle_renderer));

    // Add axes
    let x_axis = XAxisRenderer::new(device.clone(), queue.clone(), format, width, height);
    multi_renderer.add_renderer(Box::new(x_axis));

    let y_axis = YAxisRenderer::new(device.clone(), queue.clone(), format, width, height);
    multi_renderer.add_renderer(Box::new(y_axis));

    multi_renderer
}

/// Example trade data for testing
pub fn generate_sample_trades() -> Vec<TradeData> {
    vec![
        TradeData {
            timestamp: 1234567890,
            price: 100.5,
            volume: 1.5,
            side: TradeSide::Buy,
        },
        TradeData {
            timestamp: 1234567900,
            price: 99.8,
            volume: 2.0,
            side: TradeSide::Sell,
        },
        TradeData {
            timestamp: 1234567910,
            price: 101.2,
            volume: 0.5,
            side: TradeSide::Buy,
        },
        TradeData {
            timestamp: 1234567920,
            price: 100.0,
            volume: 3.0,
            side: TradeSide::Sell,
        },
    ]
}