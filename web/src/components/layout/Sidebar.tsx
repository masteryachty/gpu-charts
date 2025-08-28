import { useState, useEffect } from 'react';
import { 
  Settings, 
  BarChart3, 
  TrendingUp, 
  Bell,
  ChevronLeft,
  ChevronRight,
  Plus,
  X,
  Menu
} from 'lucide-react';
import { useBreakpoint } from '../../hooks/useResizeObserver';
import { useLoading } from '../../contexts/LoadingContext';
import { SidebarLoadingSkeleton } from '../loading/LoadingSkeleton';

const sidebarItems = [
  { icon: Settings, label: 'Settings', id: 'settings' },
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
  const { loading } = useLoading();
  const [isCollapsed, setIsCollapsed] = useState(true);
  const [activePanel, setActivePanel] = useState<string | null>('indicators');
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);
  
  // Get current breakpoint for responsive behavior
  const { ref: breakpointRef, breakpoint } = useBreakpoint();

  // Determine layout based on breakpoint
  const isMobile = ['xs', 'sm'].includes(breakpoint);
  const isTablet = breakpoint === 'md';
  const isDesktop = ['lg', 'xl', '2xl'].includes(breakpoint);

  // Auto-collapse on mobile/tablet
  useEffect(() => {
    if (isMobile || isTablet) {
      setIsCollapsed(true);
      setIsMobileMenuOpen(false);
    }
  }, [isMobile, isTablet]);

  // Close mobile menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (isMobile && isMobileMenuOpen) {
        const target = event.target as Element;
        if (!target.closest('[data-sidebar-container]') && !target.closest('[data-mobile-menu-trigger]')) {
          setIsMobileMenuOpen(false);
        }
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isMobile, isMobileMenuOpen]);

  // Mobile bottom navigation
  if (isMobile) {
    return (
      <>
        {/* Mobile menu trigger button */}
        <button
          data-mobile-menu-trigger
          onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
          className="lg:hidden fixed top-4 left-4 z-50 p-2 bg-gray-800 rounded-lg border border-gray-600 shadow-lg"
          aria-label="Toggle navigation menu"
        >
          {isMobileMenuOpen ? (
            <X className="h-5 w-5 text-white" />
          ) : (
            <Menu className="h-5 w-5 text-white" />
          )}
        </button>

        {/* Mobile bottom navigation */}
        <div className="fixed bottom-0 left-0 right-0 z-40 bg-gray-800 border-t border-gray-600">
          <div className="flex items-center justify-around py-2">
            {sidebarItems.map((item) => (
              <button
                key={item.id}
                onClick={() => {
                  setActivePanel(activePanel === item.id ? null : item.id);
                  setIsMobileMenuOpen(true);
                }}
                className={`flex flex-col items-center gap-1 px-3 py-2 transition-colors ${
                  activePanel === item.id ? 'text-blue-400' : 'text-gray-400 hover:text-gray-200'
                }`}
                aria-label={item.label}
              >
                <item.icon size={20} />
                <span className="text-xs">{item.label}</span>
              </button>
            ))}
          </div>
        </div>

        {/* Mobile menu overlay */}
        {isMobileMenuOpen && (
          <div className="fixed inset-0 z-30 bg-black bg-opacity-50 lg:hidden">
            <div 
              data-sidebar-container
              className="fixed bottom-16 left-4 right-4 bg-gray-800 border border-gray-600 rounded-lg max-h-96 overflow-y-auto"
            >
              {activePanel && (
                <div className="p-4">
                  {renderPanelContent(activePanel)}
                </div>
              )}
            </div>
          </div>
        )}
      </>
    );
  }

  // Tablet modal overlay
  if (isTablet) {
    return (
      <>
        {/* Tablet menu trigger */}
        <button
          onClick={() => setIsMobileMenuOpen(!isMobileMenuOpen)}
          className="fixed top-4 left-4 z-50 p-2 bg-gray-800 rounded-lg border border-gray-600 shadow-lg"
          aria-label="Toggle sidebar"
        >
          <Menu className="h-5 w-5 text-white" />
        </button>

        {/* Tablet modal sidebar */}
        {isMobileMenuOpen && (
          <div className="fixed inset-0 z-40 flex">
            <div className="fixed inset-0 bg-black bg-opacity-50" onClick={() => setIsMobileMenuOpen(false)} />
            <div 
              data-sidebar-container
              className="relative w-64 bg-gray-800 border-r border-gray-600 shadow-xl"
            >
              <DesktopSidebar 
                isCollapsed={false}
                activePanel={activePanel}
                setActivePanel={setActivePanel}
                setIsCollapsed={setIsCollapsed}
                onClose={() => setIsMobileMenuOpen(false)}
              />
            </div>
          </div>
        )}
      </>
    );
  }

  // Show loading skeleton while initializing
  if (loading.wasm || loading.initialization || loading.webgpu) {
    return <SidebarLoadingSkeleton />;
  }

  // Desktop sidebar
  return (
    <div ref={breakpointRef}>
      <DesktopSidebar 
        isCollapsed={isCollapsed}
        activePanel={activePanel}
        setActivePanel={setActivePanel}
        setIsCollapsed={setIsCollapsed}
      />
    </div>
  );
}

// Desktop sidebar component
function DesktopSidebar({
  isCollapsed,
  activePanel,
  setActivePanel,
  setIsCollapsed,
  onClose
}: {
  isCollapsed: boolean;
  activePanel: string | null;
  setActivePanel: (panel: string | null) => void;
  setIsCollapsed: (collapsed: boolean) => void;
  onClose?: () => void;
}) {
  return (
    <div className={`bg-bg-secondary border-r border-border transition-all duration-300 ${
      isCollapsed ? 'w-16' : 'w-64'
    }`}>
      {/* Toggle Button */}
      <div className="h-16 flex items-center justify-between px-4 border-b border-border">
        {!isCollapsed && <span className="text-text-secondary text-sm font-medium">Tools</span>}
        <div className="flex items-center gap-2">
          {onClose && (
            <button
              onClick={onClose}
              className="p-1 hover:bg-bg-tertiary transition-colors md:hidden"
              aria-label="Close sidebar"
            >
              <X size={16} className="text-text-secondary" />
            </button>
          )}
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            className="p-1 hover:bg-bg-tertiary transition-colors"
            aria-label={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          >
            {isCollapsed ? (
              <ChevronRight size={16} className="text-text-secondary" />
            ) : (
              <ChevronLeft size={16} className="text-text-secondary" />
            )}
          </button>
        </div>
      </div>

      {/* Navigation Icons */}
      <div className="py-4">
        {sidebarItems.map((item) => (
          <button
            key={item.id}
            onClick={() => setActivePanel(activePanel === item.id ? null : item.id)}
            className={`w-full flex items-center gap-3 px-4 py-3 hover:bg-bg-tertiary transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-inset ${
              activePanel === item.id ? 'bg-bg-tertiary border-r-2 border-accent-blue' : ''
            }`}
            title={isCollapsed ? item.label : undefined}
            aria-label={item.label}
            aria-pressed={activePanel === item.id}
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
        <div className="flex-1 p-4 border-t border-border overflow-y-auto">
          {renderPanelContent(activePanel)}
        </div>
      )}
    </div>
  );
}

// Panel content renderer
function renderPanelContent(activePanel: string) {
  switch (activePanel) {
    case 'indicators':
      return (
        <div>
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-text-primary font-medium">Indicators</h3>
            <button 
              className="p-1 hover:bg-bg-tertiary transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
              aria-label="Add indicator"
            >
              <Plus size={16} className="text-text-secondary" />
            </button>
          </div>
          <div className="space-y-2">
            {indicators.map((indicator) => (
              <button
                key={indicator}
                className="w-full text-left px-3 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-bg-tertiary transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 rounded"
                aria-label={`Add ${indicator} indicator`}
              >
                {indicator}
              </button>
            ))}
          </div>
        </div>
      );

    case 'drawing':
      return (
        <div>
          <h3 className="text-text-primary font-medium mb-4">Drawing Tools</h3>
          <div className="space-y-2">
            {['Trend Line', 'Rectangle', 'Fibonacci'].map((tool) => (
              <button 
                key={tool}
                className="w-full text-left px-3 py-2 text-sm text-text-secondary hover:text-text-primary hover:bg-bg-tertiary transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 rounded"
                aria-label={`Select ${tool} drawing tool`}
              >
                {tool}
              </button>
            ))}
          </div>
        </div>
      );

    case 'alerts':
      return (
        <div>
          <h3 className="text-text-primary font-medium mb-4">Alerts</h3>
          <div className="text-text-tertiary text-sm" role="status">
            No active alerts
          </div>
        </div>
      );

    case 'settings':
      return (
        <div>
          <h3 className="text-text-primary font-medium mb-4">Settings</h3>
          <div className="space-y-4">
            <div>
              <label htmlFor="theme-select" className="block text-text-secondary text-sm mb-2">
                Theme
              </label>
              <select id="theme-select" className="input-primary w-full">
                <option>Dark (Default)</option>
                <option>High Contrast</option>
              </select>
            </div>
            <div>
              <label htmlFor="refresh-rate-select" className="block text-text-secondary text-sm mb-2">
                Refresh Rate
              </label>
              <select id="refresh-rate-select" className="input-primary w-full">
                <option>120 FPS</option>
                <option>60 FPS</option>
                <option>30 FPS</option>
              </select>
            </div>
          </div>
        </div>
      );

    default:
      return null;
  }
}