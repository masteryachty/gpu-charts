import { useState } from 'react';
import { 
  Settings, 
  BarChart3, 
  TrendingUp, 
  Bell,
  ChevronLeft,
  ChevronRight,
  Plus,
  Zap
} from 'lucide-react';
// import { OptimizationSettings } from '../OptimizationSettings';

const sidebarItems = [
  { icon: Settings, label: 'Settings', id: 'settings' },
  { icon: Zap, label: 'Performance', id: 'performance' },
  { icon: BarChart3, label: 'Indicators', id: 'indicators' },
  { icon: TrendingUp, label: 'Drawing Tools', id: 'drawing' },
  { icon: Bell, label: 'Alerts', id: 'alerts' },
];

const indicators = [
  'Moving Average',
  'RSI',
  'MACD',
  'Bollinger Bands',
  'Volume',
];

export default function Sidebar() {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const [activePanel, setActivePanel] = useState<string | null>('indicators');

  return (
    <div className={`bg-bg-secondary border-r border-border transition-all duration-300 ${
      isCollapsed ? 'w-16' : 'w-64'
    }`}>
      {/* Toggle Button */}
      <div className="h-16 flex items-center justify-between px-4 border-b border-border">
        {!isCollapsed && <span className="text-text-secondary text-sm font-medium">Tools</span>}
        <button
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="p-1 hover:bg-bg-tertiary transition-colors"
        >
          {isCollapsed ? (
            <ChevronRight size={16} className="text-text-secondary" />
          ) : (
            <ChevronLeft size={16} className="text-text-secondary" />
          )}
        </button>
      </div>

      {/* Navigation Icons */}
      <div className="py-4">
        {sidebarItems.map((item) => (
          <button
            key={item.id}
            onClick={() => setActivePanel(activePanel === item.id ? null : item.id)}
            className={`w-full flex items-center gap-3 px-4 py-3 hover:bg-bg-tertiary transition-colors ${
              activePanel === item.id ? 'bg-bg-tertiary border-r-2 border-accent-blue' : ''
            }`}
            title={isCollapsed ? item.label : undefined}
          >
            <item.icon size={20} className="text-text-secondary" />
            {!isCollapsed && (
              <span className="text-text-primary text-sm">{item.label}</span>
            )}
          </button>
        ))}
      </div>

      {/* Panel Content */}
      {!isCollapsed && activePanel && (
        <div className="flex-1 p-4 border-t border-border">
          {activePanel === 'indicators' && (
            <div>
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-text-primary font-medium">Indicators</h3>
                <button className="p-1 hover:bg-bg-tertiary transition-colors">
                  <Plus size={16} className="text-text-secondary" />
                </button>
              </div>
              <div className="space-y-2">
                {indicators.map((indicator) => (
                  <button
                    key={indicator}
                    className="w-full text-left px-3 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-bg-tertiary transition-colors"
                  >
                    {indicator}
                  </button>
                ))}
              </div>
            </div>
          )}

          {activePanel === 'drawing' && (
            <div>
              <h3 className="text-text-primary font-medium mb-4">Drawing Tools</h3>
              <div className="space-y-2">
                <button className="w-full text-left px-3 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-bg-tertiary transition-colors">
                  Trend Line
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-bg-tertiary transition-colors">
                  Rectangle
                </button>
                <button className="w-full text-left px-3 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-bg-tertiary transition-colors">
                  Fibonacci
                </button>
              </div>
            </div>
          )}

          {activePanel === 'alerts' && (
            <div>
              <h3 className="text-text-primary font-medium mb-4">Alerts</h3>
              <div className="text-text-tertiary text-sm">
                No active alerts
              </div>
            </div>
          )}

          {activePanel === 'settings' && (
            <div>
              <h3 className="text-text-primary font-medium mb-4">Settings</h3>
              <div className="space-y-4">
                <div>
                  <label className="block text-text-secondary text-sm mb-2">
                    Theme
                  </label>
                  <select className="input-primary w-full">
                    <option>Dark (Default)</option>
                    <option>High Contrast</option>
                  </select>
                </div>
                <div>
                  <label className="block text-text-secondary text-sm mb-2">
                    Refresh Rate
                  </label>
                  <select className="input-primary w-full">
                    <option>120 FPS</option>
                    <option>60 FPS</option>
                    <option>30 FPS</option>
                  </select>
                </div>
              </div>
            </div>
          )}

          {activePanel === 'performance' && (
            <div>
              <h3 className="text-text-primary font-medium mb-4">Performance</h3>
              <div className="text-text-secondary text-sm">
                Performance settings coming soon...
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}