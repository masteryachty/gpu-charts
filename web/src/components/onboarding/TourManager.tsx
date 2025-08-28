import { createContext, useContext, useState, useCallback, ReactNode, useEffect } from 'react';
import { TourOverlay, TourStep } from './TourOverlay';

interface TourContextValue {
  startTour: (tourId: string) => void;
  skipTour: (tourId: string) => void;
  isTourActive: boolean;
  currentTour: string | null;
  hasCompletedTour: (tourId: string) => boolean;
  resetTour: (tourId: string) => void;
}

const TourContext = createContext<TourContextValue | undefined>(undefined);

// Tour configurations
export const TOURS = {
  firstTime: {
    id: 'first-time',
    name: 'First Time User Tour',
    steps: [
      {
        id: 'welcome',
        title: '🎉 Welcome to GPU Charts!',
        content: `
          <p>Welcome to the most advanced financial charting platform powered by GPU acceleration!</p>
          <p>This quick tour will show you the key features to get you started.</p>
        `,
        target: 'body',
        placement: 'center' as const,
        showSkip: true,
      },
      {
        id: 'chart-area',
        title: '📈 Interactive Chart',
        content: `
          <p>This is your main chart area where all the magic happens:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li><strong>Mouse wheel:</strong> Zoom in/out</li>
            <li><strong>Click & drag:</strong> Pan around</li>
            <li><strong>Hover:</strong> See price details</li>
          </ul>
          <p class="mt-2">Try hovering over the chart to see the new tooltip system!</p>
        `,
        target: '#webgpu-canvas',
        placement: 'right' as const,
        action: {
          type: 'hover' as const,
          description: 'Hover over the chart to see the tooltip',
        },
      },
      {
        id: 'symbol-selector',
        title: '🪙 Symbol Selection',
        content: `
          <p>Choose which cryptocurrency or trading pair to analyze:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li>Popular pairs like BTC-USD, ETH-USD</li>
            <li>Real-time data from major exchanges</li>
            <li>Search for any supported symbol</li>
          </ul>
        `,
        target: '[data-testid="symbol-selector"]',
        placement: 'bottom' as const,
      },
      {
        id: 'time-range',
        title: '⏰ Time Range Controls',
        content: `
          <p>Control the time period you want to analyze:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li><strong>1 Hour:</strong> For day trading</li>
            <li><strong>1 Day:</strong> Recent trends</li>
            <li><strong>1 Week:</strong> Broader perspective</li>
          </ul>
          <p class="mt-2">Try clicking on a different time range!</p>
        `,
        target: '[aria-label*="time range"]',
        placement: 'bottom' as const,
        action: {
          type: 'click' as const,
          description: 'Click on a time range button',
        },
      },
      {
        id: 'comparison-mode',
        title: '🔄 Exchange Comparison',
        content: `
          <p>Compare prices across multiple exchanges:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li>Toggle comparison mode</li>
            <li>Select multiple exchanges</li>
            <li>See price differences in real-time</li>
          </ul>
        `,
        target: '[data-testid="comparison-toggle"]',
        placement: 'bottom' as const,
      },
      {
        id: 'sidebar',
        title: '🛠️ Tools & Indicators',
        content: `
          <p>Access powerful analysis tools:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li><strong>Indicators:</strong> RSI, MACD, Moving Averages</li>
            <li><strong>Drawing Tools:</strong> Trend lines, Fibonacci</li>
            <li><strong>Settings:</strong> Customize your experience</li>
          </ul>
        `,
        target: '#sidebar',
        placement: 'right' as const,
      },
      {
        id: 'keyboard-shortcuts',
        title: '⌨️ Keyboard Shortcuts',
        content: `
          <p>Master these shortcuts for efficient trading:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li><strong>Ctrl + R:</strong> Reset chart view</li>
            <li><strong>Ctrl + =/-:</strong> Zoom in/out</li>
            <li><strong>Alt:</strong> Show tooltip at center</li>
            <li><strong>Arrow keys:</strong> Navigate interface</li>
          </ul>
          <p class="mt-2">Press <kbd class="bg-gray-100 px-1 rounded">?</kbd> anytime for help!</p>
        `,
        target: 'body',
        placement: 'center' as const,
      },
      {
        id: 'performance',
        title: '⚡ GPU-Powered Performance',
        content: `
          <p>This platform leverages your GPU for incredible performance:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li><strong>Real-time rendering:</strong> 60+ FPS smooth charts</li>
            <li><strong>Million data points:</strong> No lag or stuttering</li>
            <li><strong>WebGPU technology:</strong> Next-generation graphics</li>
          </ul>
          <p class="mt-2">Experience the difference of hardware acceleration!</p>
        `,
        target: 'body',
        placement: 'center' as const,
      },
      {
        id: 'complete',
        title: '🎯 You\'re Ready to Trade!',
        content: `
          <p>Congratulations! You now know the basics of GPU Charts.</p>
          <p class="mt-2">Here are some next steps:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li>Try different symbols and time ranges</li>
            <li>Experiment with indicators and drawing tools</li>
            <li>Enable comparison mode for arbitrage opportunities</li>
          </ul>
          <p class="mt-3 text-sm text-gray-600">
            💡 Tip: You can restart this tour anytime from the settings menu.
          </p>
        `,
        target: 'body',
        placement: 'center' as const,
      },
    ] as TourStep[],
  },
  
  features: {
    id: 'advanced-features',
    name: 'Advanced Features Tour',
    steps: [
      {
        id: 'indicators-intro',
        title: '📊 Technical Indicators',
        content: `
          <p>Add powerful technical analysis indicators to your charts:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li><strong>Moving Averages:</strong> Trend direction</li>
            <li><strong>RSI:</strong> Overbought/oversold conditions</li>
            <li><strong>MACD:</strong> Momentum analysis</li>
            <li><strong>Bollinger Bands:</strong> Volatility bands</li>
          </ul>
        `,
        target: '[aria-label="Indicators"]',
        placement: 'right' as const,
      },
      {
        id: 'drawing-tools',
        title: '✏️ Drawing Tools',
        content: `
          <p>Annotate your charts with professional drawing tools:</p>
          <ul class="list-disc list-inside mt-2 space-y-1">
            <li><strong>Trend Lines:</strong> Connect highs and lows</li>
            <li><strong>Fibonacci Retracements:</strong> Key levels</li>
            <li><strong>Rectangles:</strong> Support/resistance zones</li>
          </ul>
        `,
        target: '[aria-label="Drawing Tools"]',
        placement: 'right' as const,
      },
    ] as TourStep[],
  }
};

interface TourProviderProps {
  children: ReactNode;
}

export function TourProvider({ children }: TourProviderProps) {
  const [activeTour, setActiveTour] = useState<string | null>(null);
  const [completedTours, setCompletedTours] = useState<Set<string>>(new Set());

  // Load completed tours from localStorage
  useEffect(() => {
    const stored = localStorage.getItem('gpu-charts-completed-tours');
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        setCompletedTours(new Set(parsed));
      } catch (error) {
        console.warn('Failed to parse stored tour data:', error);
      }
    }
  }, []);

  const startTour = useCallback((tourId: string) => {
    setActiveTour(tourId);
  }, []);

  const completeTour = useCallback((tourId: string) => {
    setActiveTour(null);
    const newCompleted = new Set([...completedTours, tourId]);
    setCompletedTours(newCompleted);
    
    // Persist to localStorage
    localStorage.setItem(
      'gpu-charts-completed-tours',
      JSON.stringify([...newCompleted])
    );
  }, [completedTours]);

  const skipTour = useCallback((tourId: string) => {
    setActiveTour(null);
    // Don't mark as completed when skipped
  }, []);

  const closeTour = useCallback(() => {
    setActiveTour(null);
  }, []);

  const hasCompletedTour = useCallback((tourId: string) => {
    return completedTours.has(tourId);
  }, [completedTours]);

  const resetTour = useCallback((tourId: string) => {
    const newCompleted = new Set(completedTours);
    newCompleted.delete(tourId);
    setCompletedTours(newCompleted);
    
    // Update localStorage
    localStorage.setItem(
      'gpu-charts-completed-tours',
      JSON.stringify([...newCompleted])
    );
  }, [completedTours]);

  const currentTourData = activeTour ? TOURS[activeTour as keyof typeof TOURS] : null;

  return (
    <TourContext.Provider
      value={{
        startTour,
        skipTour,
        isTourActive: !!activeTour,
        currentTour: activeTour,
        hasCompletedTour,
        resetTour,
      }}
    >
      {children}
      
      {activeTour && currentTourData && (
        <TourOverlay
          steps={currentTourData.steps}
          isActive={!!activeTour}
          onComplete={() => completeTour(activeTour)}
          onSkip={() => skipTour(activeTour)}
          onClose={closeTour}
        />
      )}
    </TourContext.Provider>
  );
}

export function useTour() {
  const context = useContext(TourContext);
  if (context === undefined) {
    throw new Error('useTour must be used within a TourProvider');
  }
  return context;
}

// Auto-start first-time tour for new users
export function FirstTimeUserDetector() {
  const { startTour, hasCompletedTour } = useTour();

  useEffect(() => {
    // Check if this is a first-time user
    const hasVisited = localStorage.getItem('gpu-charts-has-visited');
    
    if (!hasVisited && !hasCompletedTour('first-time')) {
      // Small delay to let the app initialize
      const timer = setTimeout(() => {
        startTour('first-time');
        localStorage.setItem('gpu-charts-has-visited', 'true');
      }, 2000);
      
      return () => clearTimeout(timer);
    }
  }, [startTour, hasCompletedTour]);

  return null;
}