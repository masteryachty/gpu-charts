# GPU-Charts Architecture Diagram

## High-Level Architecture

```mermaid
graph TB
    subgraph "Frontend (TypeScript/React)"
        RC[React Component<br/>WasmCanvas.tsx]
        ZS[Zustand Store]
        HOOK[useWasmChart Hook]
    end
    
    subgraph "WASM Bridge Layer (Rust/WASM)"
        CHART[Chart Instance]
        LG[LineGraph]
        CC[CanvasController]
        IM[InstanceManager]
    end
    
    subgraph "Core Libraries (Rust)"
        DM[DataManager<br/>- Fetching<br/>- Caching<br/>- GPU Buffers]
        DS[DataStore<br/>- State<br/>- Transforms<br/>- Dirty Flag]
        CS[ConfigSystem<br/>- Presets<br/>- Quality]
        PM[PresetManager]
    end
    
    subgraph "Renderer (Rust/WebGPU)"
        REND[Renderer<br/>- Surface<br/>- Device/Queue]
        MR[MultiRenderer<br/>- Plot<br/>- X-Axis<br/>- Y-Axis]
        CE[ComputeEngine<br/>- GPU Compute<br/>- Min/Max<br/>- Averages]
    end
    
    subgraph "External"
        API[Data Server API<br/>api.rednax.io]
        GPU[WebGPU<br/>Hardware]
    end
    
    RC --> HOOK
    HOOK --> CHART
    ZS -.->|State Sync| CHART
    CHART --> LG
    CHART --> CC
    CHART --> IM
    LG --> DM
    LG --> DS
    LG --> REND
    LG --> PM
    PM --> CS
    CC --> DS
    DM --> DS
    DM --> API
    REND --> MR
    REND --> CE
    CE --> DS
    MR --> GPU
    CE --> GPU
```

## First Render Method Call Sequence

```mermaid
sequenceDiagram
    participant React as React Component
    participant Hook as useWasmChart
    participant WASM as WASM Module
    participant Chart as Chart Instance
    participant LG as LineGraph
    participant DM as DataManager
    participant DS as DataStore
    participant Rend as Renderer
    participant MR as MultiRenderer
    participant GPU as WebGPU

    React->>Hook: Component Mount
    Hook->>Hook: Initialize State
    Hook->>WASM: Dynamic Import
    WASM->>WASM: Initialize Module
    Hook->>Chart: new Chart()
    Hook->>Chart: chart.init(canvas_id, width, height)
    
    Chart->>LG: LineGraph::new()
    LG->>DS: DataStore::new(width, height)
    LG->>GPU: Create Instance
    LG->>GPU: Request Adapter
    LG->>GPU: Request Device & Queue
    LG->>DM: DataManager::new()
    LG->>Rend: Renderer::new()
    Rend->>GPU: Configure Surface
    Rend->>GPU: Create ComputeEngine
    
    LG->>MR: Create MultiRenderer
    MR->>MR: Add PlotRenderer
    MR->>MR: Add XAxisRenderer
    MR->>MR: Add YAxisRenderer
    
    Chart->>Chart: render()
    Chart->>LG: LineGraph::render()
    LG->>Rend: Renderer::render()
    Rend->>DS: Check is_dirty()
    Rend->>GPU: Get Current Texture
    Rend->>GPU: Create Command Encoder
    Rend->>MR: MultiRenderer::render()
    MR->>GPU: Draw Calls
    Rend->>GPU: Submit & Present
    Rend->>DS: Mark Clean
```

## Data Flow Diagram

```mermaid
graph LR
    subgraph "Data Sources"
        API[Data API]
        CACHE[Cache]
    end
    
    subgraph "Data Processing"
        DM[DataManager]
        PARSE[Binary Parser]
        BUF[GPU Buffer Creator]
        DS[DataStore]
    end
    
    subgraph "GPU Compute"
        CE[ComputeEngine]
        COMP[Compute Shaders<br/>- Min/Max<br/>- Averages]
        RESULT[Computed Results]
    end
    
    subgraph "Rendering"
        MR[MultiRenderer]
        PLOT[Plot Renderer]
        AXES[Axis Renderers]
        FRAME[Frame Buffer]
    end
    
    API -->|Binary Data| DM
    CACHE -->|Cached Data| DM
    DM --> PARSE
    PARSE --> BUF
    BUF --> DS
    DS --> CE
    CE --> COMP
    COMP --> RESULT
    RESULT --> DS
    DS --> MR
    MR --> PLOT
    MR --> AXES
    PLOT --> FRAME
    AXES --> FRAME
```

## Event Handling Flow

```mermaid
graph TB
    subgraph "User Input"
        WHEEL[Mouse Wheel]
        DRAG[Mouse Drag]
        CLICK[Mouse Click]
        KEY[Keyboard]
    end
    
    subgraph "React Events"
        RW[onWheel]
        RD[onMouseMove]
        RC[onMouseDown/Up]
        RK[onKeyDown]
    end
    
    subgraph "WASM Bridge"
        HW[handle_mouse_wheel]
        HM[handle_mouse_motion]
        HC[handle_mouse_click]
        HK[handle_key_event]
    end
    
    subgraph "Canvas Controller"
        CC[CanvasController]
        ZOOM[Apply Zoom]
        PAN[Apply Pan]
        SEL[Apply Selection]
    end
    
    subgraph "State Update"
        DS[DataStore]
        DIRTY[Mark Dirty]
        RENDER[Trigger Render]
    end
    
    WHEEL --> RW --> HW --> CC
    DRAG --> RD --> HM --> CC
    CLICK --> RC --> HC --> CC
    KEY --> RK --> HK --> CC
    
    CC --> ZOOM --> DS
    CC --> PAN --> DS
    CC --> SEL --> DS
    
    DS --> DIRTY --> RENDER
```

## Performance-Critical Paths

```mermaid
graph TD
    subgraph "Hot Path 1: Render Loop"
        RAF[requestAnimationFrame]
        CHECK[Check needs_render]
        RENDER[Execute render]
        GPU[GPU Draw]
    end
    
    subgraph "Hot Path 2: Data Updates"
        FETCH[Fetch Data]
        GPUBUF[Create GPU Buffers]
        COMPUTE[GPU Compute]
        UPDATE[Update State]
    end
    
    subgraph "Hot Path 3: User Interaction"
        EVENT[User Event]
        TRANSFORM[Transform Coords]
        STATE[Update State]
        RERENDER[Re-render]
    end
    
    RAF --> CHECK
    CHECK -->|dirty| RENDER
    CHECK -->|clean| RAF
    RENDER --> GPU
    GPU --> RAF
    
    FETCH --> GPUBUF
    GPUBUF --> COMPUTE
    COMPUTE --> UPDATE
    
    EVENT --> TRANSFORM
    TRANSFORM --> STATE
    STATE --> RERENDER
```

## Memory Management

```mermaid
graph LR
    subgraph "CPU Memory"
        WASM[WASM Linear Memory]
        CACHE[Data Cache]
        STATE[Application State]
    end
    
    subgraph "GPU Memory"
        VB[Vertex Buffers]
        IB[Index Buffers]
        UB[Uniform Buffers]
        TB[Texture Memory]
        CB[Compute Buffers]
    end
    
    subgraph "Shared Resources"
        BG[Bind Groups]
        PL[Pipelines]
        SH[Shaders]
    end
    
    WASM --> VB
    WASM --> IB
    WASM --> UB
    CACHE --> CB
    STATE --> UB
    
    VB --> BG
    IB --> BG
    UB --> BG
    CB --> BG
    
    BG --> PL
    SH --> PL
```

## Key Performance Optimizations

1. **Zero-Copy Data Path**: Memory-mapped files → GPU buffers
2. **GPU Compute**: All heavy calculations on GPU
3. **Smart Dirty Tracking**: Only re-render when needed
4. **Efficient Buffer Management**: Reuse GPU resources
5. **Async Pipeline**: Non-blocking render loop
6. **Cached API Calls**: Minimize network requests
7. **Batched Updates**: Group state changes
8. **Optimized Shaders**: Minimal GPU instructions