import { useEffect, useRef, useCallback, useState } from 'react';

export interface ResizeObserverEntry {
  contentRect: {
    width: number;
    height: number;
    top: number;
    left: number;
  };
  target: Element;
}

/**
 * Hook for observing element resize events
 * Provides modern ResizeObserver API with fallback
 */
export function useResizeObserver<T extends Element = HTMLDivElement>(
  callback?: (entry: ResizeObserverEntry) => void
) {
  const elementRef = useRef<T>(null);
  const observerRef = useRef<ResizeObserver | null>(null);
  const callbackRef = useRef(callback);

  // Update callback ref when callback changes
  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  const observe = useCallback((element: T | null) => {
    if (!element) return;

    // Disconnect previous observer
    if (observerRef.current) {
      observerRef.current.disconnect();
    }

    // Create new observer if ResizeObserver is supported
    if ('ResizeObserver' in window) {
      observerRef.current = new ResizeObserver((entries) => {
        if (callbackRef.current && entries.length > 0) {
          const entry = entries[0];
          callbackRef.current({
            contentRect: {
              width: entry.contentRect.width,
              height: entry.contentRect.height,
              top: entry.contentRect.top,
              left: entry.contentRect.left,
            },
            target: entry.target,
          });
        }
      });

      observerRef.current.observe(element);
    } else {
      // Fallback for browsers without ResizeObserver
      console.warn('ResizeObserver not supported, using window resize fallback');
      
      const handleResize = () => {
        if (callbackRef.current && element) {
          const rect = element.getBoundingClientRect();
          callbackRef.current({
            contentRect: {
              width: rect.width,
              height: rect.height,
              top: rect.top,
              left: rect.left,
            },
            target: element,
          });
        }
      };

      window.addEventListener('resize', handleResize);
      
      // Initial measurement
      handleResize();

      // Store cleanup function
      observerRef.current = {
        disconnect: () => window.removeEventListener('resize', handleResize),
        observe: () => {},
        unobserve: () => {},
      } as ResizeObserver;
    }
  }, []);

  useEffect(() => {
    if (elementRef.current) {
      observe(elementRef.current);
    }

    return () => {
      if (observerRef.current) {
        observerRef.current.disconnect();
      }
    };
  }, [observe]);

  return elementRef;
}

/**
 * Hook for tracking element size
 * Returns current dimensions and provides ref to attach to element
 */
export function useElementSize<T extends Element = HTMLDivElement>() {
  const [size, setSize] = useState({ width: 0, height: 0 });
  
  const ref = useResizeObserver<T>(({ contentRect }) => {
    setSize({
      width: contentRect.width,
      height: contentRect.height,
    });
  });

  return { ref, width: size.width, height: size.height };
}

/**
 * Hook for responsive breakpoints
 * Provides current breakpoint based on element width
 */
export function useBreakpoint<T extends Element = HTMLDivElement>(
  breakpoints: { [key: string]: number } = {
    xs: 0,
    sm: 640,
    md: 768,
    lg: 1024,
    xl: 1280,
    '2xl': 1536,
  }
) {
  const [currentBreakpoint, setCurrentBreakpoint] = useState('xs');
  
  const ref = useResizeObserver<T>(({ contentRect }) => {
    const width = contentRect.width;
    
    // Find the largest breakpoint that fits
    let matchedBreakpoint = 'xs';
    Object.entries(breakpoints).forEach(([name, size]) => {
      if (width >= size) {
        matchedBreakpoint = name;
      }
    });
    
    setCurrentBreakpoint(matchedBreakpoint);
  });

  return { ref, breakpoint: currentBreakpoint };
}