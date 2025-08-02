#!/bin/bash

# Reset all changes first
git reset HEAD

# Stage only files that have pure log removals (React/TypeScript)
# These files appear to have only log removals based on analysis:
git add web/src/pages/TradingApp.tsx
git add web/src/components/chart/WasmCanvas.tsx
git add web/src/store/useAppStore.ts

# For Rust files with only log removals:
git add crates/wasm-bridge/src/controls/canvas_controller.rs
git add crates/renderer/src/drawables/plot.rs
git add crates/renderer/src/drawables/y_axis.rs
git add crates/renderer/src/lib.rs

# Show what was staged
echo "Staged files (log cleanup only):"
git diff --cached --name-only

echo ""
echo "Files with mixed changes (not staged):"
echo "- web/src/hooks/useWasmChart.ts (has useEffect and render loop additions)"
echo "- crates/wasm-bridge/src/lib.rs (has new methods: needs_render, set_time_range)"
echo "- crates/wasm-bridge/src/chart_engine.rs (has new methods)"
echo "- crates/data-manager/src/lib.rs (has sample debugging code)"
echo "- crates/data-manager/src/data_store.rs (has new y_to_screen_position method)"
echo "- crates/data-manager/src/data_retriever.rs (has buffer usage changes)"
echo "- crates/renderer/src/calcables/min_max.rs (complete rewrite for all data groups)"
echo "- crates/renderer/src/calcables/*.wgsl (shader changes)"
echo "- crates/renderer/src/charts/triangle_renderer.rs (triangle size change and new logic)"
echo "- crates/renderer/src/charts/triangle.wgsl (Y-axis inversion fix)"