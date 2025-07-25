# Mid Line Compute Shader Implementation Plan

## Overview
Implement a GPU-based compute shader to calculate the mid price from best_bid and best_ask data. This will serve as a foundation for more complex GPU-based calculations in the future.

## Architecture

### 1. Compute Shader Pipeline
```
Input Buffers (bid, ask) → Compute Shader → Output Buffer (mid) → Line Renderer
```

### 2. Components

#### A. Compute Shader (`mid_price_compute.wgsl`)
- Input: Two storage buffers (best_bid, best_ask)
- Output: One storage buffer (mid_price)
- Calculation: `mid = (bid + ask) / 2.0`
- Workgroup size: 256 (optimal for most GPUs)

#### B. ComputeProcessor (`compute_processor.rs`)
- Generic compute shader processor
- Manages compute pipelines and bind groups
- Reusable for future calculations

#### C. MidPriceCalculator (`mid_price_calculator.rs`)
- Specific implementation for mid price
- Creates and manages compute pipeline
- Handles buffer creation and binding

#### D. Integration
- Hook into preset system
- Calculate on-demand when bid/ask data changes
- Cache results to avoid recalculation

## Implementation Steps

### Phase 1: Core Compute Infrastructure
1. Create generic ComputeProcessor trait
2. Implement compute shader loading and compilation
3. Add buffer management for compute operations

### Phase 2: Mid Price Implementation
1. Write mid_price_compute.wgsl shader
2. Create MidPriceCalculator
3. Integrate with data loading pipeline

### Phase 3: Renderer Integration
1. Add computed data support to PlotRenderer
2. Update preset to use computed mid price
3. Handle dynamic updates

### Phase 4: Optimization
1. Add caching for computed results
2. Implement dirty tracking
3. Batch multiple calculations

## Benefits
1. **Performance**: GPU-parallel calculation for large datasets
2. **Extensibility**: Foundation for complex indicators
3. **Reusability**: Generic compute infrastructure
4. **Real-time**: Fast enough for live data updates

## Future Calculations
- Moving averages (SMA, EMA)
- Bollinger Bands
- RSI
- VWAP
- Custom indicators