declare module '@pkg/gpu_charts_wasm.js' {
  export default function init(input?: any): Promise<any>;
  
  export class ChartSystem {
    constructor(canvas_id: string, base_url: string);
    
    update_chart(
      chart_type: string,
      symbol: string,
      start_time: bigint,
      end_time: bigint
    ): Promise<void>;
    
    render(): void;
    update_config(config_json: string): void;
    get_config(): string;
    resize(width: number, height: number): void;
    get_stats(): string;
    destroy(): void;
    
    // Mouse event handlers
    handle_mouse_wheel(delta_y: number, x: number, y: number): void;
    handle_mouse_move(x: number, y: number): void;
    handle_mouse_click(x: number, y: number, pressed: boolean): void;
    needs_render(): boolean;
  }
  
  export function version(): string;
}

declare module '@pkg/gpu_charts_wasm' {
  export * from '@pkg/gpu_charts_wasm.js';
}