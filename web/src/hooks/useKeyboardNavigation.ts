import { useCallback, useEffect, useRef, useState } from 'react';

/**
 * Navigation direction for focus movement
 */
export type NavigationDirection = 'up' | 'down' | 'left' | 'right' | 'next' | 'previous';

/**
 * Focus trap configuration
 */
export interface FocusTrapOptions {
  enabled: boolean;
  initialFocus?: HTMLElement | null;
  restoreFocus?: HTMLElement | null;
  allowEscape?: boolean;
}

/**
 * Keyboard navigation configuration
 */
export interface KeyboardNavigationOptions {
  /** Enable arrow key navigation */
  enableArrowKeys?: boolean;
  
  /** Enable Tab navigation */
  enableTabNavigation?: boolean;
  
  /** Enable Enter/Space activation */
  enableActivation?: boolean;
  
  /** Enable Escape handling */
  enableEscape?: boolean;
  
  /** Focus trap configuration */
  focusTrap?: FocusTrapOptions;
  
  /** Custom key handlers */
  keyHandlers?: Record<string, (event: KeyboardEvent) => void>;
  
  /** Navigation scope selector */
  scope?: string;
}

/**
 * Hook for comprehensive keyboard navigation support
 */
export function useKeyboardNavigation(options: KeyboardNavigationOptions = {}) {
  const {
    enableArrowKeys = true,
    enableTabNavigation = true,
    enableActivation = true,
    enableEscape = true,
    focusTrap,
    keyHandlers = {},
    scope
  } = options;

  const containerRef = useRef<HTMLElement>(null);
  const [focusedIndex, setFocusedIndex] = useState<number>(-1);
  const [isNavigationMode, setIsNavigationMode] = useState<boolean>(false);
  
  // Get all focusable elements within the scope
  const getFocusableElements = useCallback((): HTMLElement[] => {
    const container = containerRef.current || document;
    const selector = scope || '[tabindex], button, [href], input, select, textarea, [contenteditable="true"]';
    
    const elements = Array.from(container.querySelectorAll(selector)) as HTMLElement[];
    
    return elements.filter(element => {
      // Filter out disabled and hidden elements
      if (element.hasAttribute('disabled')) return false;
      if (element.hasAttribute('aria-hidden') && element.getAttribute('aria-hidden') === 'true') return false;
      if (element.tabIndex === -1 && !element.hasAttribute('tabindex')) return false;
      
      // Check if element is visible
      const style = getComputedStyle(element);
      if (style.display === 'none' || style.visibility === 'hidden') return false;
      
      return true;
    });
  }, [scope]);

  // Navigate to specific element by index
  const navigateToIndex = useCallback((index: number) => {
    const elements = getFocusableElements();
    if (index >= 0 && index < elements.length) {
      const element = elements[index];
      element.focus();
      setFocusedIndex(index);
      setIsNavigationMode(true);
      
      // Announce to screen readers
      if (element.getAttribute('aria-label') || element.textContent) {
        const announcement = element.getAttribute('aria-label') || element.textContent || 'Navigation item';
        announceToScreenReader(`Focused: ${announcement.trim()}`);
      }
    }
  }, [getFocusableElements]);

  // Navigate in a specific direction
  const navigate = useCallback((direction: NavigationDirection) => {
    const elements = getFocusableElements();
    if (elements.length === 0) return;

    const currentIndex = focusedIndex >= 0 ? focusedIndex : 0;
    let nextIndex = currentIndex;

    switch (direction) {
      case 'up':
      case 'previous':
        nextIndex = currentIndex > 0 ? currentIndex - 1 : elements.length - 1;
        break;
      case 'down':
      case 'next':
        nextIndex = currentIndex < elements.length - 1 ? currentIndex + 1 : 0;
        break;
      case 'left':
        // For grid-like navigation (if applicable)
        nextIndex = Math.max(0, currentIndex - 1);
        break;
      case 'right':
        // For grid-like navigation (if applicable)
        nextIndex = Math.min(elements.length - 1, currentIndex + 1);
        break;
    }

    navigateToIndex(nextIndex);
  }, [focusedIndex, getFocusableElements, navigateToIndex]);

  // Activate current focused element
  const activateCurrentElement = useCallback(() => {
    const elements = getFocusableElements();
    if (focusedIndex >= 0 && focusedIndex < elements.length) {
      const element = elements[focusedIndex];
      
      // Trigger appropriate event based on element type
      if (element.tagName === 'BUTTON' || element.tagName === 'A') {
        element.click();
      } else if (element.tagName === 'INPUT') {
        const input = element as HTMLInputElement;
        if (input.type === 'checkbox' || input.type === 'radio') {
          input.click();
        } else {
          input.focus();
        }
      } else if (element.hasAttribute('role') && element.getAttribute('role') === 'button') {
        element.click();
      }
    }
  }, [focusedIndex, getFocusableElements]);

  // Handle keyboard events
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    // Handle custom key handlers first
    if (keyHandlers[event.key] || keyHandlers[`${event.key}+${event.ctrlKey ? 'ctrl' : ''}+${event.shiftKey ? 'shift' : ''}+${event.altKey ? 'alt' : ''}`]) {
      const handler = keyHandlers[event.key] || keyHandlers[`${event.key}+${event.ctrlKey ? 'ctrl' : ''}+${event.shiftKey ? 'shift' : ''}+${event.altKey ? 'alt' : ''}`];
      handler(event);
      return;
    }

    // Skip if target is an input field (unless specifically enabled)
    const target = event.target as HTMLElement;
    const isInputField = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.contentEditable === 'true';
    
    if (isInputField && !target.hasAttribute('data-keyboard-nav')) {
      return;
    }

    let handled = false;

    // Arrow key navigation
    if (enableArrowKeys) {
      switch (event.key) {
        case 'ArrowUp':
          navigate('up');
          handled = true;
          break;
        case 'ArrowDown':
          navigate('down');
          handled = true;
          break;
        case 'ArrowLeft':
          navigate('left');
          handled = true;
          break;
        case 'ArrowRight':
          navigate('right');
          handled = true;
          break;
      }
    }

    // Tab navigation
    if (enableTabNavigation) {
      switch (event.key) {
        case 'Tab':
          if (event.shiftKey) {
            navigate('previous');
          } else {
            navigate('next');
          }
          handled = true;
          break;
      }
    }

    // Activation keys
    if (enableActivation) {
      switch (event.key) {
        case 'Enter':
        case ' ':
          if (isNavigationMode) {
            activateCurrentElement();
            handled = true;
          }
          break;
      }
    }

    // Escape handling
    if (enableEscape && event.key === 'Escape') {
      setIsNavigationMode(false);
      setFocusedIndex(-1);
      
      if (focusTrap?.restoreFocus) {
        focusTrap.restoreFocus.focus();
      }
      handled = true;
    }

    if (handled) {
      event.preventDefault();
      event.stopPropagation();
    }
  }, [
    keyHandlers,
    enableArrowKeys,
    enableTabNavigation,
    enableActivation,
    enableEscape,
    navigate,
    activateCurrentElement,
    isNavigationMode,
    focusTrap
  ]);

  // Handle focus events to track current position
  const handleFocus = useCallback((event: FocusEvent) => {
    const elements = getFocusableElements();
    const focusedElement = event.target as HTMLElement;
    const index = elements.indexOf(focusedElement);
    
    if (index >= 0) {
      setFocusedIndex(index);
      setIsNavigationMode(true);
    }
  }, [getFocusableElements]);

  // Handle focus trap
  useEffect(() => {
    if (focusTrap?.enabled) {
      const elements = getFocusableElements();
      if (elements.length > 0) {
        const firstElement = focusTrap.initialFocus || elements[0];
        firstElement.focus();
        navigateToIndex(0);
      }
    }
  }, [focusTrap, getFocusableElements, navigateToIndex]);

  // Set up event listeners
  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('focusin', handleFocus);
    
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('focusin', handleFocus);
    };
  }, [handleKeyDown, handleFocus]);

  return {
    containerRef,
    focusedIndex,
    isNavigationMode,
    navigate,
    navigateToIndex,
    activateCurrentElement,
    getFocusableElements,
    setNavigationMode: setIsNavigationMode
  };
}

/**
 * Hook for skip links navigation
 */
export function useSkipLinks() {
  const skipLinks = useRef<Array<{ id: string; label: string; target: string }>>([]);
  
  const addSkipLink = useCallback((id: string, label: string, target: string) => {
    skipLinks.current.push({ id, label, target });
  }, []);
  
  const navigateToSkipTarget = useCallback((target: string) => {
    const element = document.getElementById(target) || document.querySelector(`[data-skip-target="${target}"]`);
    if (element) {
      element.focus();
      element.scrollIntoView({ behavior: 'smooth' });
      announceToScreenReader(`Navigated to ${target}`);
    }
  }, []);
  
  return {
    skipLinks: skipLinks.current,
    addSkipLink,
    navigateToSkipTarget
  };
}

/**
 * Hook for roving tabindex pattern
 */
export function useRovingTabindex(initialIndex: number = 0) {
  const [activeIndex, setActiveIndex] = useState(initialIndex);
  const itemsRef = useRef<HTMLElement[]>([]);
  
  const registerItem = useCallback((element: HTMLElement | null, index: number) => {
    if (element) {
      itemsRef.current[index] = element;
      element.tabIndex = index === activeIndex ? 0 : -1;
    }
  }, [activeIndex]);
  
  const setActiveItem = useCallback((index: number) => {
    itemsRef.current.forEach((item, i) => {
      if (item) {
        item.tabIndex = i === index ? 0 : -1;
      }
    });
    setActiveIndex(index);
    
    if (itemsRef.current[index]) {
      itemsRef.current[index].focus();
    }
  }, []);
  
  const handleKeyDown = useCallback((event: KeyboardEvent, currentIndex: number) => {
    let nextIndex = currentIndex;
    
    switch (event.key) {
      case 'ArrowRight':
      case 'ArrowDown':
        event.preventDefault();
        nextIndex = Math.min(currentIndex + 1, itemsRef.current.length - 1);
        break;
      case 'ArrowLeft':
      case 'ArrowUp':
        event.preventDefault();
        nextIndex = Math.max(currentIndex - 1, 0);
        break;
      case 'Home':
        event.preventDefault();
        nextIndex = 0;
        break;
      case 'End':
        event.preventDefault();
        nextIndex = itemsRef.current.length - 1;
        break;
    }
    
    if (nextIndex !== currentIndex) {
      setActiveItem(nextIndex);
    }
  }, [setActiveItem]);
  
  return {
    activeIndex,
    registerItem,
    setActiveItem,
    handleKeyDown
  };
}

/**
 * Utility function to announce to screen readers
 */
export function announceToScreenReader(message: string) {
  const announcement = document.createElement('div');
  announcement.setAttribute('aria-live', 'polite');
  announcement.setAttribute('aria-atomic', 'true');
  announcement.setAttribute('class', 'sr-only');
  announcement.style.position = 'absolute';
  announcement.style.left = '-10000px';
  announcement.style.width = '1px';
  announcement.style.height = '1px';
  announcement.style.overflow = 'hidden';
  
  announcement.textContent = message;
  document.body.appendChild(announcement);
  
  // Remove after announcement
  setTimeout(() => {
    document.body.removeChild(announcement);
  }, 1000);
}

/**
 * Hook for managing focus restoration
 */
export function useFocusRestore() {
  const lastFocusedElement = useRef<HTMLElement | null>(null);
  
  const saveFocus = useCallback(() => {
    lastFocusedElement.current = document.activeElement as HTMLElement;
  }, []);
  
  const restoreFocus = useCallback(() => {
    if (lastFocusedElement.current && document.contains(lastFocusedElement.current)) {
      lastFocusedElement.current.focus();
    }
  }, []);
  
  return { saveFocus, restoreFocus };
}

export default useKeyboardNavigation;