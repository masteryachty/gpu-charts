import React from 'react';
import { useKeyboardNavigationContext } from '../../contexts/KeyboardNavigationContext';

/**
 * Skip Links Component
 * Provides keyboard navigation shortcuts to main page areas
 * Only visible when focused for screen reader and keyboard users
 */
export function SkipLinks() {
  const { navigateToSkipTarget, skipLinks } = useKeyboardNavigationContext();

  const handleSkipLinkClick = (event: React.MouseEvent<HTMLAnchorElement>, target: string) => {
    event.preventDefault();
    navigateToSkipTarget(target);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLAnchorElement>, target: string) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      navigateToSkipTarget(target);
    }
  };

  return (
    <nav aria-label="Skip navigation links" className="skip-links">
      <ul className="sr-only focus-within:not-sr-only focus-within:fixed focus-within:top-4 focus-within:left-4 focus-within:z-50 focus-within:bg-blue-600 focus-within:text-white focus-within:p-2 focus-within:rounded focus-within:shadow-lg">
        <li>
          <a
            href="#main-content"
            className="skip-link block px-3 py-2 text-white hover:bg-blue-700 rounded focus:outline-none focus:ring-2 focus:ring-white"
            onClick={(e) => handleSkipLinkClick(e, 'main-content')}
            onKeyDown={(e) => handleKeyDown(e, 'main-content')}
          >
            Skip to main content
          </a>
        </li>
        <li>
          <a
            href="#chart"
            className="skip-link block px-3 py-2 text-white hover:bg-blue-700 rounded focus:outline-none focus:ring-2 focus:ring-white"
            onClick={(e) => handleSkipLinkClick(e, 'chart')}
            onKeyDown={(e) => handleKeyDown(e, 'chart')}
          >
            Skip to chart
          </a>
        </li>
        <li>
          <a
            href="#sidebar"
            className="skip-link block px-3 py-2 text-white hover:bg-blue-700 rounded focus:outline-none focus:ring-2 focus:ring-white"
            onClick={(e) => handleSkipLinkClick(e, 'sidebar')}
            onKeyDown={(e) => handleKeyDown(e, 'sidebar')}
          >
            Skip to sidebar
          </a>
        </li>
        <li>
          <a
            href="#chart-controls"
            className="skip-link block px-3 py-2 text-white hover:bg-blue-700 rounded focus:outline-none focus:ring-2 focus:ring-white"
            onClick={(e) => handleSkipLinkClick(e, 'chart-controls')}
            onKeyDown={(e) => handleKeyDown(e, 'chart-controls')}
          >
            Skip to chart controls
          </a>
        </li>
      </ul>
    </nav>
  );
}

/**
 * Keyboard Help Button
 * Floating button to show keyboard shortcuts
 */
export function KeyboardHelpButton() {
  const { showKeyboardHelp, isHelpVisible } = useKeyboardNavigationContext();

  return (
    <button
      onClick={showKeyboardHelp}
      className="fixed bottom-4 left-4 z-40 bg-blue-600 text-white p-3 rounded-full shadow-lg hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-300"
      title="Show keyboard shortcuts (F1 or ?)"
      aria-label="Show keyboard shortcuts"
      aria-expanded={isHelpVisible}
    >
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    </button>
  );
}

/**
 * Navigation Indicator
 * Shows current navigation state for debugging/development
 */
export function NavigationIndicator() {
  const { isNavigationMode, focusedIndex } = useKeyboardNavigationContext();

  // Only show in development
  if (process.env.NODE_ENV !== 'development') {
    return null;
  }

  return (
    <div className="fixed bottom-4 right-4 z-40 bg-gray-800 text-white p-2 rounded text-xs">
      <div>Nav Mode: {isNavigationMode ? 'ON' : 'OFF'}</div>
      <div>Focus Index: {focusedIndex}</div>
    </div>
  );
}

export default SkipLinks;