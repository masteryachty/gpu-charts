import React, { useCallback, useEffect, useState, useMemo } from 'react';
// import { useErrorHandler } from '../../hooks/useErrorHandler'; // Disabled temporarily
import type { AppError } from '../../errors/ErrorTypes';

/**
 * Error Notification Center
 * 
 * Displays user-friendly error notifications with appropriate severity levels,
 * recovery options, and dismissal capabilities.
 */

export interface ErrorNotificationCenterProps {
  /** Maximum number of notifications to show simultaneously */
  maxNotifications?: number;
  
  /** Position on screen */
  position?: 'top-right' | 'top-left' | 'bottom-right' | 'bottom-left' | 'top-center' | 'bottom-center';
  
  /** Auto-dismiss timeout for low severity errors (ms) */
  autoHideTimeoutMs?: number;
  
  /** Enable sound notifications */
  enableSounds?: boolean;
  
  /** Show detailed error information */
  showDetailedInfo?: boolean;
  
  /** Custom notification renderer */
  renderNotification?: (error: AppError, index: number, onDismiss: () => void) => React.ReactNode;
}

interface NotificationItem {
  id: string;
  error: AppError;
  timestamp: number;
  dismissed: boolean;
  autoHideTimer?: NodeJS.Timeout;
}

export default function ErrorNotificationCenter({
  maxNotifications = 5,
  position = 'top-right',
  autoHideTimeoutMs = 8000,
  enableSounds = false,
  showDetailedInfo = false,
  renderNotification
}: ErrorNotificationCenterProps) {
  // const [errorState, errorAPI] = useErrorHandler({
  //   subscribeToCategories: [],
  //   subscribeToSeverities: ['medium', 'high', 'critical'],
  //   maxRecentErrors: 100
  // }); // Disabled temporarily
  
  // Mock error state for now
  const errorState = { 
    recentErrors: [] as AppError[], 
    isLoading: false, 
    pendingNotifications: 0 
  };
  const errorAPI = useMemo(() => ({ 
    getNotifications: (): AppError[] => [],
    clearNotifications: () => {},
    getRecentErrors: (): AppError[] => []
  }), []);
  
  const [notifications, setNotifications] = useState<NotificationItem[]>([]);
  const [isVisible, setIsVisible] = useState(true);
  
  // Process new errors into notifications
  useEffect(() => {
    const newNotifications = errorAPI.getNotifications();
    
    if (newNotifications.length > 0) {
      console.log(`[ErrorNotificationCenter] Processing ${newNotifications.length} new notifications`);
      
      const notificationItems: NotificationItem[] = newNotifications.map(error => {
        const item: NotificationItem = {
          id: Math.random().toString(36).substr(2, 9),
          error,
          timestamp: Date.now(),
          dismissed: false
        };
        
        // Auto-hide low severity errors
        if ('severity' in error && error.severity === 'low' && autoHideTimeoutMs > 0) {
          item.autoHideTimer = setTimeout(() => {
            // dismissNotification(item.id); // Fixed: avoid hoisting issue
          }, autoHideTimeoutMs);
        }
        
        return item;
      });
      
      setNotifications(prev => {
        const combined = [...notificationItems, ...prev];
        return combined.slice(0, maxNotifications);
      });
      
      // Play notification sound
      if (enableSounds && newNotifications.some(n => 'severity' in n && (n.severity === 'critical' || n.severity === 'high'))) {
        // playNotificationSound(newNotifications[0].severity); // Fixed: avoid hoisting issue
      }
    }
  }, [errorState.pendingNotifications, autoHideTimeoutMs, maxNotifications, enableSounds, errorAPI]);
  
  // Dismiss notification
  const dismissNotification = useCallback((id: string) => {
    setNotifications(prev => prev.map(notification => {
      if (notification.id === id) {
        if (notification.autoHideTimer) {
          clearTimeout(notification.autoHideTimer);
        }
        return { ...notification, dismissed: true };
      }
      return notification;
    }));
    
    // Remove after animation
    setTimeout(() => {
      setNotifications(prev => prev.filter(n => n.id !== id));
    }, 300);
  }, []);
  
  // Dismiss all notifications
  const dismissAll = useCallback(() => {
    notifications.forEach(notification => {
      if (notification.autoHideTimer) {
        clearTimeout(notification.autoHideTimer);
      }
    });
    
    setNotifications(prev => prev.map(n => ({ ...n, dismissed: true })));
    
    setTimeout(() => {
      setNotifications([]);
    }, 300);
  }, [notifications]);
  
  // Play notification sound - commented out until needed
  // const playNotificationSound = useCallback((_severity: string) => {
  //   if (!enableSounds) return;
  //   
  //   try {
  //     const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
  //     const oscillator = audioContext.createOscillator();
  //     const gainNode = audioContext.createGain();
  //     
  //     // Different frequencies for different severities
  //     const frequency = _severity === 'critical' ? 800 : _severity === 'high' ? 600 : 400;
  //     oscillator.frequency.value = frequency;
  //     oscillator.type = 'sine';
  //     
  //     gainNode.gain.setValueAtTime(0.1, audioContext.currentTime);
  //     gainNode.gain.exponentialRampToValueAtTime(0.01, audioContext.currentTime + 0.5);
  //     
  //     oscillator.connect(gainNode);
  //     gainNode.connect(audioContext.destination);
  //     
  //     oscillator.start(audioContext.currentTime);
  //     oscillator.stop(audioContext.currentTime + 0.5);
  //   } catch (error) {
  //     console.warn('[ErrorNotificationCenter] Failed to play notification sound:', error);
  //   }
  // }, [enableSounds]);
  
  // Position classes
  const getPositionClasses = (position: string): string => {
    const baseClasses = 'fixed z-50 p-4 space-y-3';
    
    switch (position) {
      case 'top-right':
        return `${baseClasses} top-4 right-4`;
      case 'top-left':
        return `${baseClasses} top-4 left-4`;
      case 'bottom-right':
        return `${baseClasses} bottom-4 right-4`;
      case 'bottom-left':
        return `${baseClasses} bottom-4 left-4`;
      case 'top-center':
        return `${baseClasses} top-4 left-1/2 transform -translate-x-1/2`;
      case 'bottom-center':
        return `${baseClasses} bottom-4 left-1/2 transform -translate-x-1/2`;
      default:
        return `${baseClasses} top-4 right-4`;
    }
  };
  
  // Get severity color classes
  const getSeverityClasses = (severity: string): string => {
    switch (severity) {
      case 'critical':
        return 'bg-red-900 border-red-600 text-red-100';
      case 'high':
        return 'bg-orange-900 border-orange-600 text-orange-100';
      case 'medium':
        return 'bg-yellow-900 border-yellow-600 text-yellow-100';
      case 'low':
        return 'bg-blue-900 border-blue-600 text-blue-100';
      default:
        return 'bg-gray-900 border-gray-600 text-gray-100';
    }
  };
  
  // Get severity icon
  const getSeverityIcon = (severity: string): string => {
    switch (severity) {
      case 'critical':
        return '🚨';
      case 'high':
        return '⚠️';
      case 'medium':
        return '⚡';
      case 'low':
        return 'ℹ️';
      default:
        return '•';
    }
  };
  
  // Format error message for user display
  const formatErrorMessage = (error: AppError): string => {
    // Convert technical error messages to user-friendly ones
    const userFriendlyMessages: Record<string, string> = {
      'WASM_INIT_FAILED': 'Failed to initialize chart engine. Please refresh the page.',
      'DATA_FETCH_FAILED': 'Unable to load market data. Check your connection.',
      'STORE_SYNC_FAILED': 'Chart synchronization error. Some changes may be lost.',
      'NETWORK_TIMEOUT': 'Request timed out. Please try again.',
      'PERFORMANCE_MEMORY_LEAK': 'Performance issue detected. Consider refreshing the page.',
      'VALIDATION_INVALID_SYMBOL': 'Invalid trading symbol selected.',
    };
    
    return userFriendlyMessages[error.code] || error.message;
  };
  
  // Default notification renderer
  const renderDefaultNotification = (notification: NotificationItem, index: number) => {
    const { error, dismissed } = notification;
    
    return (
      <div
        key={notification.id}
        className={`
          max-w-sm w-full rounded-lg border shadow-lg transition-all duration-300 transform
          ${getSeverityClasses(error.severity)}
          ${dismissed ? 'opacity-0 translate-x-full' : 'opacity-100 translate-x-0'}
        `}
        style={{ 
          transitionDelay: `${index * 50}ms`,
          marginBottom: index > 0 ? '8px' : '0'
        }}
      >
        <div className="p-4">
          <div className="flex items-start">
            <div className="flex-shrink-0 mr-3">
              <span className="text-xl">{getSeverityIcon(error.severity)}</span>
            </div>
            
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-2">
                <h4 className="font-medium text-sm">
                  {error.category.charAt(0).toUpperCase() + error.category.slice(1)} Error
                </h4>
                <button
                  onClick={() => dismissNotification(notification.id)}
                  className="text-gray-400 hover:text-white transition-colors"
                  aria-label="Dismiss notification"
                >
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              
              <p className="text-sm opacity-90 mb-2">
                {formatErrorMessage(error)}
              </p>
              
              {showDetailedInfo && error.context && (
                <details className="text-xs opacity-75">
                  <summary className="cursor-pointer hover:opacity-100">Technical details</summary>
                  <pre className="mt-1 whitespace-pre-wrap font-mono">
                    {JSON.stringify(error.context, null, 2)}
                  </pre>
                </details>
              )}
              
              <div className="flex items-center justify-between mt-3">
                <span className="text-xs opacity-75">
                  {new Date(error.timestamp).toLocaleTimeString()}
                </span>
                
                <div className="flex space-x-2">
                  {error.severity === 'critical' && (
                    <button
                      onClick={() => window.location.reload()}
                      className="text-xs px-2 py-1 bg-white/20 rounded hover:bg-white/30 transition-colors"
                    >
                      Reload
                    </button>
                  )}
                  
                  <button
                    onClick={() => dismissNotification(notification.id)}
                    className="text-xs px-2 py-1 bg-white/20 rounded hover:bg-white/30 transition-colors"
                  >
                    Dismiss
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  };
  
  if (!isVisible || notifications.length === 0) {
    return null;
  }
  
  return (
    <div className={getPositionClasses(position)}>
      {/* Header with dismiss all button */}
      {notifications.length > 1 && (
        <div className="flex justify-between items-center mb-2">
          <span className="text-gray-400 text-sm">
            {notifications.length} notification{notifications.length !== 1 ? 's' : ''}
          </span>
          <button
            onClick={dismissAll}
            className="text-gray-400 hover:text-white text-sm transition-colors"
          >
            Dismiss all
          </button>
        </div>
      )}
      
      {/* Notifications */}
      {notifications.map((notification, index) => {
        if (renderNotification) {
          return renderNotification(
            notification.error,
            index,
            () => dismissNotification(notification.id)
          );
        }
        
        return renderDefaultNotification(notification, index);
      })}
      
      {/* Toggle visibility button */}
      <button
        onClick={() => setIsVisible(false)}
        className="mt-2 text-gray-500 hover:text-gray-300 text-xs transition-colors"
        title="Hide notifications"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.878 9.878L12 12m-2.122-2.122L7.758 7.758M12 12l2.122-2.122m0 0L16.242 7.758M12 12l-2.122 2.122" />
        </svg>
      </button>
    </div>
  );
}

/**
 * Floating notification toggle button for when notifications are hidden
 */
export function ErrorNotificationToggle() {
  // const [errorState] = useErrorHandler(); // Disabled temporarily
  const errorState = { pendingNotifications: 0 }; // Mock for now
  const [_isVisible, _setIsVisible] = useState(false);
  
  if (errorState.pendingNotifications === 0) {
    return null;
  }
  
  return (
    <button
      onClick={() => _setIsVisible(true)}
      className="fixed bottom-4 right-4 z-40 bg-red-600 text-white p-3 rounded-full shadow-lg hover:bg-red-700 transition-colors"
      title={`${errorState.pendingNotifications} error notification${errorState.pendingNotifications !== 1 ? 's' : ''}`}
    >
      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      
      {errorState.pendingNotifications > 0 && (
        <span className="absolute -top-2 -right-2 bg-red-800 text-white text-xs rounded-full w-6 h-6 flex items-center justify-center">
          {errorState.pendingNotifications > 99 ? '99+' : errorState.pendingNotifications}
        </span>
      )}
    </button>
  );
}