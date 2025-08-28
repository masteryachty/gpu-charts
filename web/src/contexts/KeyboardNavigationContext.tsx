import React, { createContext, useContext, useCallback, useState, useRef, useEffect } from 'react';
import { useKeyboardNavigation, useSkipLinks, announceToScreenReader } from '../hooks/useKeyboardNavigation';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';
import { useAppStore } from '../store/useAppStore';

interface KeyboardNavigationContextType {
  // Navigation state
  isNavigationMode: boolean;
  focusedIndex: number;
  
  // Navigation actions
  navigate: (direction: 'up' | 'down' | 'left' | 'right' | 'next' | 'previous') => void;
  navigateToIndex: (index: number) => void;
  activateCurrentElement: () => void;
  
  // Skip links
  addSkipLink: (id: string, label: string, target: string) => void;
  navigateToSkipTarget: (target: string) => void;
  skipLinks: Array<{ id: string; label: string; target: string }>;
  
  // Help system
  showKeyboardHelp: () => void;
  hideKeyboardHelp: () => void;
  isHelpVisible: boolean;
  
  // Accessibility announcements
  announce: (message: string) => void;
}

const KeyboardNavigationContext = createContext<KeyboardNavigationContextType | null>(null);

export function useKeyboardNavigationContext() {
  const context = useContext(KeyboardNavigationContext);
  if (!context) {
    throw new Error('useKeyboardNavigationContext must be used within a KeyboardNavigationProvider');
  }
  return context;
}

interface KeyboardNavigationProviderProps {
  children: React.ReactNode;
}

export function KeyboardNavigationProvider({ children }: KeyboardNavigationProviderProps) {
  const [isHelpVisible, setIsHelpVisible] = useState(false);
  const helpDialogRef = useRef<HTMLDivElement>(null);
  
  // App store for chart interactions
  const { 
    resetToDefaults,
    setTimeRange,
    setCurrentSymbol,
    comparisonMode,
    setComparisonMode 
  } = useAppStore();

  // Skip links management
  const { skipLinks, addSkipLink, navigateToSkipTarget } = useSkipLinks();
  
  // Main keyboard navigation
  const {
    containerRef,
    focusedIndex,
    isNavigationMode,
    navigate,
    navigateToIndex,
    activateCurrentElement
  } = useKeyboardNavigation({
    enableArrowKeys: true,
    enableTabNavigation: true,
    enableActivation: true,
    enableEscape: true,
    focusTrap: {
      enabled: isHelpVisible,
      initialFocus: helpDialogRef.current?.querySelector('button') as HTMLElement,
      allowEscape: true
    }
  });

  // Helper function for screen reader announcements
  const announce = useCallback((message: string) => {
    announceToScreenReader(message);
  }, []);

  // Chart-specific keyboard shortcuts
  const handleChartReset = useCallback(() => {
    resetToDefaults();
    announce('Chart reset to default settings');
  }, [resetToDefaults, announce]);

  const handleZoomIn = useCallback(() => {
    // This would integrate with the WASM chart instance
    announce('Zooming in');
  }, [announce]);

  const handleZoomOut = useCallback(() => {
    // This would integrate with the WASM chart instance
    announce('Zooming out');
  }, [announce]);

  const handleToggleComparisonMode = useCallback(() => {
    setComparisonMode(!comparisonMode);
    announce(comparisonMode ? 'Comparison mode disabled' : 'Comparison mode enabled');
  }, [comparisonMode, setComparisonMode, announce]);

  const handleQuickTimeRangeChange = useCallback((range: string) => {
    const now = Math.floor(Date.now() / 1000);
    let startTime: number;
    
    switch (range) {
      case '1h':
        startTime = now - 3600;
        break;
      case '4h':
        startTime = now - 14400;
        break;
      case '1d':
        startTime = now - 86400;
        break;
      case '1w':
        startTime = now - 604800;
        break;
      default:
        return;
    }
    
    setTimeRange(startTime, now);
    announce(`Time range changed to ${range}`);
  }, [setTimeRange]);

  const handleSymbolQuickSelect = useCallback((symbolNum: string) => {
    const symbols = ['BTC-USD', 'ETH-USD', 'SOL-USD', 'ADA-USD', 'DOT-USD'];
    const index = parseInt(symbolNum) - 1;
    
    if (index >= 0 && index < symbols.length) {
      setCurrentSymbol(`coinbase:${symbols[index]}`);
      announce(`Symbol changed to ${symbols[index]}`);
    }
  }, [setCurrentSymbol]);

  // Global keyboard shortcuts
  useKeyboardShortcuts([
    // Help system
    {
      key: 'F1',
      callback: () => setIsHelpVisible(!isHelpVisible),
      description: 'Toggle keyboard help'
    },
    {
      key: '?',
      callback: () => setIsHelpVisible(!isHelpVisible),
      description: 'Show keyboard help'
    },
    
    // Chart shortcuts
    {
      key: 'r',
      ctrlKey: true,
      callback: handleChartReset,
      description: 'Reset chart'
    },
    {
      key: '=',
      ctrlKey: true,
      callback: handleZoomIn,
      description: 'Zoom in'
    },
    {
      key: '-',
      ctrlKey: true,
      callback: handleZoomOut,
      description: 'Zoom out'
    },
    {
      key: 'c',
      ctrlKey: true,
      callback: handleToggleComparisonMode,
      description: 'Toggle comparison mode'
    },
    
    // Quick time range shortcuts
    {
      key: '1',
      altKey: true,
      callback: () => handleQuickTimeRangeChange('1h'),
      description: 'Set 1 hour time range'
    },
    {
      key: '4',
      altKey: true,
      callback: () => handleQuickTimeRangeChange('4h'),
      description: 'Set 4 hour time range'
    },
    {
      key: 'd',
      altKey: true,
      callback: () => handleQuickTimeRangeChange('1d'),
      description: 'Set 1 day time range'
    },
    {
      key: 'w',
      altKey: true,
      callback: () => handleQuickTimeRangeChange('1w'),
      description: 'Set 1 week time range'
    },
    
    // Quick symbol selection
    {
      key: '1',
      shiftKey: true,
      callback: () => handleSymbolQuickSelect('1'),
      description: 'Select BTC-USD'
    },
    {
      key: '2',
      shiftKey: true,
      callback: () => handleSymbolQuickSelect('2'),
      description: 'Select ETH-USD'
    },
    {
      key: '3',
      shiftKey: true,
      callback: () => handleSymbolQuickSelect('3'),
      description: 'Select SOL-USD'
    },
    {
      key: '4',
      shiftKey: true,
      callback: () => handleSymbolQuickSelect('4'),
      description: 'Select ADA-USD'
    },
    {
      key: '5',
      shiftKey: true,
      callback: () => handleSymbolQuickSelect('5'),
      description: 'Select DOT-USD'
    },
    
    // Navigation shortcuts
    {
      key: 's',
      altKey: true,
      callback: () => navigateToSkipTarget('sidebar'),
      description: 'Go to sidebar'
    },
    {
      key: 'c',
      altKey: true,
      callback: () => navigateToSkipTarget('chart'),
      description: 'Go to chart'
    },
    {
      key: 'h',
      altKey: true,
      callback: () => navigateToSkipTarget('header'),
      description: 'Go to header'
    }
  ]);

  // Set up skip links
  useEffect(() => {
    addSkipLink('skip-to-content', 'Skip to main content', 'main-content');
    addSkipLink('skip-to-sidebar', 'Skip to sidebar', 'sidebar');
    addSkipLink('skip-to-chart', 'Skip to chart', 'chart');
    addSkipLink('skip-to-controls', 'Skip to chart controls', 'chart-controls');
  }, [addSkipLink]);

  // Helper functions
  const showKeyboardHelp = useCallback(() => {
    setIsHelpVisible(true);
    announce('Keyboard help opened');
  }, [announce]);

  const hideKeyboardHelp = useCallback(() => {
    setIsHelpVisible(false);
    announce('Keyboard help closed');
  }, [announce]);

  // Handle escape key in help dialog
  useEffect(() => {
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && isHelpVisible) {
        hideKeyboardHelp();
      }
    };

    if (isHelpVisible) {
      document.addEventListener('keydown', handleEscape);
      return () => document.removeEventListener('keydown', handleEscape);
    }
  }, [isHelpVisible, hideKeyboardHelp]);

  const contextValue: KeyboardNavigationContextType = {
    isNavigationMode,
    focusedIndex,
    navigate,
    navigateToIndex,
    activateCurrentElement,
    addSkipLink,
    navigateToSkipTarget,
    skipLinks,
    showKeyboardHelp,
    hideKeyboardHelp,
    isHelpVisible,
    announce
  };

  return (
    <KeyboardNavigationContext.Provider value={contextValue}>
      <div ref={containerRef}>
        {children}
        
        {/* Keyboard Help Dialog */}
        {isHelpVisible && (
          <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div
              ref={helpDialogRef}
              className="bg-gray-800 border border-gray-600 rounded-lg p-6 max-w-2xl max-h-[80vh] overflow-y-auto"
              role="dialog"
              aria-modal="true"
              aria-labelledby="keyboard-help-title"
            >
              <div className="flex justify-between items-center mb-4">
                <h2 id="keyboard-help-title" className="text-xl font-bold text-white">
                  Keyboard Shortcuts
                </h2>
                <button
                  onClick={hideKeyboardHelp}
                  className="text-gray-400 hover:text-white p-1"
                  aria-label="Close keyboard help"
                >
                  ✕
                </button>
              </div>
              
              <div className="space-y-4">
                <div>
                  <h3 className="text-lg font-semibold text-white mb-2">General Navigation</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-sm">
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Tab</kbd> - Navigate forward
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Shift+Tab</kbd> - Navigate backward
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Enter</kbd> - Activate element
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Space</kbd> - Activate button/checkbox
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Arrow Keys</kbd> - Navigate in grids
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Escape</kbd> - Close dialogs/menus
                    </div>
                  </div>
                </div>
                
                <div>
                  <h3 className="text-lg font-semibold text-white mb-2">Chart Controls</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-sm">
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Ctrl+R</kbd> - Reset chart
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Ctrl+=</kbd> - Zoom in
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Ctrl+-</kbd> - Zoom out
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Ctrl+C</kbd> - Toggle comparison
                    </div>
                  </div>
                </div>
                
                <div>
                  <h3 className="text-lg font-semibold text-white mb-2">Quick Actions</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-sm">
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Alt+1</kbd> - 1 hour range
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Alt+4</kbd> - 4 hour range
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Alt+D</kbd> - 1 day range
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Alt+W</kbd> - 1 week range
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Shift+1-5</kbd> - Select symbols
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">F1 or ?</kbd> - Show this help
                    </div>
                  </div>
                </div>
                
                <div>
                  <h3 className="text-lg font-semibold text-white mb-2">Skip Links</h3>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-sm">
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Alt+S</kbd> - Go to sidebar
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Alt+C</kbd> - Go to chart
                    </div>
                    <div className="text-gray-300">
                      <kbd className="bg-gray-700 px-2 py-1 rounded">Alt+H</kbd> - Go to header
                    </div>
                  </div>
                </div>
              </div>
              
              <div className="mt-6 flex justify-end">
                <button
                  onClick={hideKeyboardHelp}
                  className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 transition-colors"
                >
                  Close
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </KeyboardNavigationContext.Provider>
  );
}

export default KeyboardNavigationProvider;