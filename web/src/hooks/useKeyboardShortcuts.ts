import { useEffect, useCallback } from 'react';

export type KeyboardShortcut = {
  key: string;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
  callback: (event: KeyboardEvent) => void;
  description?: string;
};

/**
 * Hook for managing keyboard shortcuts
 * Provides easy registration and cleanup of keyboard shortcuts
 */
export function useKeyboardShortcuts(shortcuts: KeyboardShortcut[]) {
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    for (const shortcut of shortcuts) {
      const {
        key,
        ctrlKey = false,
        shiftKey = false,
        altKey = false,
        metaKey = false,
        callback
      } = shortcut;

      // Check if the key combination matches
      if (
        event.key.toLowerCase() === key.toLowerCase() &&
        event.ctrlKey === ctrlKey &&
        event.shiftKey === shiftKey &&
        event.altKey === altKey &&
        event.metaKey === metaKey
      ) {
        event.preventDefault();
        callback(event);
        break;
      }
    }
  }, [shortcuts]);

  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [handleKeyDown]);
}

/**
 * Pre-defined common shortcuts for charts
 */
export function useChartKeyboardShortcuts(callbacks: {
  onReset?: () => void;
  onZoomIn?: () => void;
  onZoomOut?: () => void;
  onToggleGrid?: () => void;
  onToggleTooltips?: () => void;
  onSavePreset?: () => void;
}) {
  const shortcuts: KeyboardShortcut[] = [
    {
      key: 'r',
      ctrlKey: true,
      callback: () => callbacks.onReset?.(),
      description: 'Reset chart'
    },
    {
      key: '=',
      ctrlKey: true,
      callback: () => callbacks.onZoomIn?.(),
      description: 'Zoom in'
    },
    {
      key: '-',
      ctrlKey: true,
      callback: () => callbacks.onZoomOut?.(),
      description: 'Zoom out'
    },
    {
      key: 'g',
      ctrlKey: true,
      callback: () => callbacks.onToggleGrid?.(),
      description: 'Toggle grid'
    },
    {
      key: 't',
      ctrlKey: true,
      callback: () => callbacks.onToggleTooltips?.(),
      description: 'Toggle tooltips'
    },
    {
      key: 's',
      ctrlKey: true,
      shiftKey: true,
      callback: () => callbacks.onSavePreset?.(),
      description: 'Save preset'
    },
  ].filter(shortcut => shortcut.callback !== undefined);

  useKeyboardShortcuts(shortcuts);

  return shortcuts; // Return shortcuts for display in UI
}