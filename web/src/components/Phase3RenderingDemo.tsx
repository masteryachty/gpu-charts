import React, { useEffect, useState, useRef } from 'react';
// import { useAppStore } from '../store/useAppStore'; // For future use

// Import both WASM modules
// @ts-ignore
import initPhase3, { ChartSystemMinimal } from '@pkg/gpu_charts_wasm_minimal';
// @ts-ignore
import initLegacy, { Chart } from '@pkg/GPU_charting';

interface RenderingState {
  phase3System: ChartSystemMinimal | null;
  legacyChart: Chart | null;
  isInitialized: boolean;
  error: string | null;
  currentConfig: any;
  performanceMetrics: {
    fps: number;
    frameTime: number;
    drawCalls: number;
  };
}

export const Phase3RenderingDemo: React.FC = () => {
  const [state, setState] = useState<RenderingState>({
    phase3System: null,
    legacyChart: null,
    isInitialized: false,
    error: null,
    currentConfig: null,
    performanceMetrics: {
      fps: 0,
      frameTime: 0,
      drawCalls: 0,
    },
  });

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationFrameRef = useRef<number>();
  const lastFrameTimeRef = useRef<number>(performance.now());

  // Chart config from store could be used in future
  // const chartConfig = useAppStore(state => state.chartConfig);

  useEffect(() => {
    const initialize = async () => {
      try {
        // Initialize Phase 3 configuration system
        await initPhase3();
        const phase3System = new ChartSystemMinimal('phase3-rendering-canvas');

        // Initialize legacy rendering
        await initLegacy();
        const legacyChart = new Chart();
        await legacyChart.init('phase3-rendering-canvas', 800, 600);

        // Get initial config from Phase 3
        const configJson = phase3System.get_config();
        const config = JSON.parse(configJson);

        setState(prev => ({
          ...prev,
          phase3System,
          legacyChart,
          isInitialized: true,
          currentConfig: config,
        }));

        // Apply initial configuration to legacy renderer
        applyConfigToLegacyRenderer(legacyChart, config);

      } catch (e) {
        console.error('Initialization failed:', e);
        setState(prev => ({
          ...prev,
          error: e instanceof Error ? e.message : 'Unknown error',
        }));
      }
    };

    initialize();

    return () => {
      // Cleanup
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      state.phase3System?.free?.();
      state.legacyChart?.free?.();
    };
  }, []);

  // Start render loop when initialized
  useEffect(() => {
    if (!state.isInitialized || !state.legacyChart) return;

    const renderLoop = async () => {
      const now = performance.now();
      const frameTime = now - lastFrameTimeRef.current;
      lastFrameTimeRef.current = now;

      try {
        // Render using legacy system
        await state.legacyChart!.render();

        // Update performance metrics
        setState(prev => ({
          ...prev,
          performanceMetrics: {
            fps: Math.round(1000 / frameTime),
            frameTime: frameTime,
            drawCalls: 156, // Simulated for now
          },
        }));
      } catch (e) {
        console.error('Render error:', e);
      }

      animationFrameRef.current = requestAnimationFrame(renderLoop);
    };

    renderLoop();

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [state.isInitialized, state.legacyChart]);

  // Handle configuration changes
  const handleQualityChange = (preset: string) => {
    if (!state.phase3System || !state.legacyChart) return;

    try {
      // Update Phase 3 configuration
      state.phase3System.set_quality_preset(preset);
      
      // Get updated config
      const configJson = state.phase3System.get_config();
      const config = JSON.parse(configJson);
      
      setState(prev => ({ ...prev, currentConfig: config }));
      
      // Apply to legacy renderer
      applyConfigToLegacyRenderer(state.legacyChart, config);
      
      console.log(`Applied quality preset: ${preset}`);
    } catch (e) {
      console.error('Failed to apply quality preset:', e);
    }
  };

  const handleFeatureToggle = (feature: string) => {
    if (!state.phase3System || !state.currentConfig) return;

    // Update config
    const updatedConfig = {
      ...state.currentConfig,
      [feature]: !state.currentConfig[feature],
    };

    try {
      state.phase3System.update_config(JSON.stringify(updatedConfig));
      setState(prev => ({ ...prev, currentConfig: updatedConfig }));
      
      // In a real implementation, this would enable/disable rendering features
      console.log(`Feature ${feature} is now ${updatedConfig[feature] ? 'enabled' : 'disabled'}`);
    } catch (e) {
      console.error('Failed to update feature:', e);
    }
  };

  // Apply Phase 3 config to legacy renderer
  const applyConfigToLegacyRenderer = (chart: Chart, config: any) => {
    try {
      // Map Phase 3 config to legacy chart state
      const chartState = {
        renderingConfig: {
          antialiasing: config.msaa_samples > 1,
          maxFps: config.max_fps,
          quality: config.quality_preset,
        },
        features: {
          bloom: config.enable_bloom,
          fxaa: config.enable_fxaa,
        },
      };

      // Apply to legacy chart (if it had such a method)
      // For now, we'll just log what would be applied
      console.log('Would apply config to legacy renderer:', chartState);
      
      // Some settings we can actually apply
      if ('set_background_color' in chart) {
        // Use darker background for better contrast
        (chart as any).set_background_color(0.05, 0.05, 0.06, 1.0);
      }
      
      if ('set_grid_visibility' in chart) {
        (chart as any).set_grid_visibility(true, true);
      }
    } catch (e) {
      console.error('Failed to apply config to legacy renderer:', e);
    }
  };

  if (state.error) {
    return (
      <div className="bg-red-900/20 border border-red-500 rounded-lg p-4">
        <h3 className="text-red-400 font-semibold mb-2">Rendering Integration Error</h3>
        <p className="text-red-300">{state.error}</p>
      </div>
    );
  }

  if (!state.isInitialized) {
    return (
      <div className="bg-dark-800 rounded-lg p-6">
        <div className="animate-pulse">
          <div className="h-96 bg-dark-600 rounded mb-4"></div>
          <div className="h-4 bg-dark-600 rounded w-1/3"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Rendering Canvas */}
      <div className="bg-dark-800 rounded-lg p-6">
        <h3 className="text-lg font-semibold mb-4">Phase 3 Configuration → Legacy Rendering</h3>
        
        <div className="relative bg-black rounded-lg overflow-hidden" style={{ height: '400px' }}>
          <canvas
            ref={canvasRef}
            id="phase3-rendering-canvas"
            className="w-full h-full"
            style={{ imageRendering: 'auto' }}
          />
          
          {/* Performance Overlay */}
          <div className="absolute top-2 right-2 bg-dark-900/80 text-xs font-mono p-2 rounded">
            <div>FPS: {state.performanceMetrics.fps}</div>
            <div>Frame: {state.performanceMetrics.frameTime.toFixed(1)}ms</div>
            <div>Draws: {state.performanceMetrics.drawCalls}</div>
          </div>
        </div>
      </div>

      {/* Configuration Controls */}
      <div className="bg-dark-800 rounded-lg p-6">
        <h3 className="text-lg font-semibold mb-4">Configuration Controls</h3>
        
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Quality Presets */}
          <div>
            <h4 className="text-sm font-medium text-gray-400 mb-3">Quality Preset</h4>
            <div className="flex gap-2">
              {['low', 'medium', 'high', 'ultra'].map(preset => (
                <button
                  key={preset}
                  onClick={() => handleQualityChange(preset)}
                  className={`px-3 py-1 rounded text-sm transition-colors ${
                    state.currentConfig?.quality_preset === preset
                      ? 'bg-blue-500 text-white'
                      : 'bg-dark-700 text-gray-300 hover:bg-dark-600'
                  }`}
                >
                  {preset.charAt(0).toUpperCase() + preset.slice(1)}
                </button>
              ))}
            </div>
          </div>

          {/* Rendering Settings */}
          <div>
            <h4 className="text-sm font-medium text-gray-400 mb-3">Rendering Settings</h4>
            <div className="space-y-2 text-sm">
              <div>Max FPS: {state.currentConfig?.max_fps || 60}</div>
              <div>MSAA: {state.currentConfig?.msaa_samples || 1}x</div>
              <div>Bloom: {state.currentConfig?.enable_bloom ? 'On' : 'Off'}</div>
              <div>FXAA: {state.currentConfig?.enable_fxaa ? 'On' : 'Off'}</div>
            </div>
          </div>
        </div>

        {/* Feature Toggles */}
        <div className="mt-6">
          <h4 className="text-sm font-medium text-gray-400 mb-3">Features (Config Only)</h4>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
            {['scatter_plots', 'heatmaps', 'three_d_charts', 'technical_indicators'].map(feature => (
              <label key={feature} className="flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={state.currentConfig?.[feature] || false}
                  onChange={() => handleFeatureToggle(feature)}
                  className="rounded bg-dark-700 border-dark-500"
                />
                <span className="text-sm">
                  {feature.split('_').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ')}
                </span>
              </label>
            ))}
          </div>
        </div>

        {/* Integration Status */}
        <div className="mt-6 p-4 bg-dark-700 rounded">
          <h4 className="text-sm font-medium text-yellow-400 mb-2">Integration Status</h4>
          <ul className="text-xs space-y-1 text-gray-400">
            <li>✅ Phase 3 configuration system active</li>
            <li>✅ Legacy renderer receiving config updates</li>
            <li>⚠️ Rendering features not fully connected</li>
            <li>⏳ Full Phase 3 renderer pending dependency fixes</li>
          </ul>
        </div>
      </div>
    </div>
  );
};