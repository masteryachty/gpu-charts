import { useState, useEffect, useRef, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { X, ChevronLeft, ChevronRight, Play, SkipForward } from 'lucide-react';

export interface TourStep {
  id: string;
  title: string;
  content: string;
  target: string; // CSS selector or element ID
  placement?: 'top' | 'bottom' | 'left' | 'right' | 'center';
  showSkip?: boolean;
  action?: {
    type: 'click' | 'hover' | 'wait';
    duration?: number;
    description?: string;
  };
  validation?: () => boolean;
}

export interface TourOverlayProps {
  steps: TourStep[];
  isActive: boolean;
  onComplete: () => void;
  onSkip: () => void;
  onClose: () => void;
  className?: string;
}

interface TooltipPosition {
  x: number;
  y: number;
  placement: 'top' | 'bottom' | 'left' | 'right' | 'center';
}

export function TourOverlay({
  steps,
  isActive,
  onComplete,
  onSkip,
  onClose,
  className = ''
}: TourOverlayProps) {
  const [currentStep, setCurrentStep] = useState(0);
  const [tooltipPosition, setTooltipPosition] = useState<TooltipPosition>({
    x: 0,
    y: 0,
    placement: 'center'
  });
  const [targetElement, setTargetElement] = useState<Element | null>(null);
  const [isActionInProgress, setIsActionInProgress] = useState(false);
  
  const overlayRef = useRef<HTMLDivElement>(null);
  const tooltipRef = useRef<HTMLDivElement>(null);

  const currentStepData = steps[currentStep];

  // Update target element and position when step changes
  useEffect(() => {
    if (!isActive || !currentStepData) return;

    const target = document.querySelector(currentStepData.target);
    setTargetElement(target);

    if (target) {
      updateTooltipPosition(target, currentStepData.placement || 'bottom');
      
      // Scroll target into view
      target.scrollIntoView({ 
        behavior: 'smooth', 
        block: 'center',
        inline: 'center' 
      });
    } else {
      // Center position if no target
      setTooltipPosition({
        x: window.innerWidth / 2,
        y: window.innerHeight / 2,
        placement: 'center'
      });
    }
  }, [currentStep, currentStepData, isActive]);

  const updateTooltipPosition = useCallback((target: Element, placement: string) => {
    const rect = target.getBoundingClientRect();
    const tooltip = tooltipRef.current;
    
    if (!tooltip) return;

    const tooltipRect = tooltip.getBoundingClientRect();
    const tooltipWidth = tooltipRect.width || 320;
    const tooltipHeight = tooltipRect.height || 200;
    
    let x = 0;
    let y = 0;
    let finalPlacement = placement;

    switch (placement) {
      case 'top':
        x = rect.left + rect.width / 2 - tooltipWidth / 2;
        y = rect.top - tooltipHeight - 20;
        
        // Check if tooltip fits above
        if (y < 20) {
          finalPlacement = 'bottom';
          y = rect.bottom + 20;
        }
        break;
        
      case 'bottom':
        x = rect.left + rect.width / 2 - tooltipWidth / 2;
        y = rect.bottom + 20;
        
        // Check if tooltip fits below
        if (y + tooltipHeight > window.innerHeight - 20) {
          finalPlacement = 'top';
          y = rect.top - tooltipHeight - 20;
        }
        break;
        
      case 'left':
        x = rect.left - tooltipWidth - 20;
        y = rect.top + rect.height / 2 - tooltipHeight / 2;
        
        // Check if tooltip fits to the left
        if (x < 20) {
          finalPlacement = 'right';
          x = rect.right + 20;
        }
        break;
        
      case 'right':
        x = rect.right + 20;
        y = rect.top + rect.height / 2 - tooltipHeight / 2;
        
        // Check if tooltip fits to the right
        if (x + tooltipWidth > window.innerWidth - 20) {
          finalPlacement = 'left';
          x = rect.left - tooltipWidth - 20;
        }
        break;
        
      default: // center
        x = window.innerWidth / 2 - tooltipWidth / 2;
        y = window.innerHeight / 2 - tooltipHeight / 2;
        finalPlacement = 'center';
    }

    // Final boundary checks
    x = Math.max(20, Math.min(x, window.innerWidth - tooltipWidth - 20));
    y = Math.max(20, Math.min(y, window.innerHeight - tooltipHeight - 20));

    setTooltipPosition({ x, y, placement: finalPlacement as any });
  }, []);

  const handleNext = useCallback(async () => {
    if (currentStepData?.action) {
      setIsActionInProgress(true);
      
      try {
        // Wait for action to complete or timeout
        if (currentStepData.action.type === 'wait') {
          await new Promise(resolve => 
            setTimeout(resolve, currentStepData.action?.duration || 1000)
          );
        }
        
        // Validate step completion if validation provided
        if (currentStepData.validation) {
          const isValid = currentStepData.validation();
          if (!isValid) {
            setIsActionInProgress(false);
            return; // Don't proceed if validation fails
          }
        }
      } catch (error) {
        console.warn('Tour step action failed:', error);
      }
      
      setIsActionInProgress(false);
    }

    if (currentStep < steps.length - 1) {
      setCurrentStep(prev => prev + 1);
    } else {
      onComplete();
    }
  }, [currentStep, currentStepData, steps.length, onComplete]);

  const handlePrevious = useCallback(() => {
    if (currentStep > 0) {
      setCurrentStep(prev => prev - 1);
    }
  }, [currentStep]);

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!isActive) return;

    switch (event.key) {
      case 'Escape':
        onClose();
        break;
      case 'ArrowRight':
      case 'Enter':
      case ' ':
        event.preventDefault();
        handleNext();
        break;
      case 'ArrowLeft':
        event.preventDefault();
        handlePrevious();
        break;
    }
  }, [isActive, handleNext, handlePrevious, onClose]);

  // Keyboard event handling
  useEffect(() => {
    if (isActive) {
      document.addEventListener('keydown', handleKeyDown);
      return () => document.removeEventListener('keydown', handleKeyDown);
    }
  }, [isActive, handleKeyDown]);

  // Handle window resize
  useEffect(() => {
    const handleResize = () => {
      if (targetElement && currentStepData) {
        updateTooltipPosition(targetElement, currentStepData.placement || 'bottom');
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [targetElement, currentStepData, updateTooltipPosition]);

  if (!isActive || !currentStepData) {
    return null;
  }

  const progress = ((currentStep + 1) / steps.length) * 100;

  const overlay = (
    <div
      ref={overlayRef}
      className={`fixed inset-0 z-[10000] bg-black bg-opacity-60 transition-opacity duration-300 ${className}`}
      role="dialog"
      aria-labelledby="tour-title"
      aria-describedby="tour-content"
      aria-modal="true"
    >
      {/* Highlight target element */}
      {targetElement && (
        <div
          className="absolute border-4 border-blue-500 rounded-lg pointer-events-none animate-pulse"
          style={{
            left: targetElement.getBoundingClientRect().left - 4,
            top: targetElement.getBoundingClientRect().top - 4,
            width: targetElement.getBoundingClientRect().width + 8,
            height: targetElement.getBoundingClientRect().height + 8,
          }}
        />
      )}

      {/* Tooltip */}
      <div
        ref={tooltipRef}
        className="absolute bg-white rounded-lg shadow-2xl p-6 max-w-sm min-w-80 transform transition-all duration-300"
        style={{
          left: `${tooltipPosition.x}px`,
          top: `${tooltipPosition.y}px`,
        }}
      >
        {/* Progress bar */}
        <div className="w-full bg-gray-200 rounded-full h-1 mb-4">
          <div
            className="bg-blue-500 h-1 rounded-full transition-all duration-300"
            style={{ width: `${progress}%` }}
          />
        </div>

        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <div className="text-blue-500">
              <Play size={20} />
            </div>
            <h3 id="tour-title" className="text-lg font-semibold text-gray-900">
              {currentStepData.title}
            </h3>
          </div>
          
          <div className="flex items-center gap-2">
            <span className="text-sm text-gray-500">
              {currentStep + 1} of {steps.length}
            </span>
            <button
              onClick={onClose}
              className="p-1 hover:bg-gray-100 rounded transition-colors"
              aria-label="Close tour"
            >
              <X size={16} />
            </button>
          </div>
        </div>

        {/* Content */}
        <div
          id="tour-content"
          className="text-gray-700 mb-6 leading-relaxed"
          dangerouslySetInnerHTML={{ __html: currentStepData.content }}
        />

        {/* Action indicator */}
        {currentStepData.action && (
          <div className="mb-4 p-3 bg-blue-50 rounded-lg border border-blue-200">
            <div className="flex items-center gap-2 text-blue-700 text-sm">
              {isActionInProgress ? (
                <div className="animate-spin rounded-full h-4 w-4 border-2 border-blue-500 border-t-transparent" />
              ) : (
                <Play size={14} />
              )}
              <span>
                {currentStepData.action.description || 
                 `${currentStepData.action.type.charAt(0).toUpperCase() + currentStepData.action.type.slice(1)} to continue`}
              </span>
            </div>
          </div>
        )}

        {/* Navigation */}
        <div className="flex items-center justify-between">
          <div className="flex gap-2">
            {currentStep > 0 && (
              <button
                onClick={handlePrevious}
                className="flex items-center gap-2 px-4 py-2 text-gray-600 hover:text-gray-800 hover:bg-gray-100 rounded transition-colors"
                disabled={isActionInProgress}
              >
                <ChevronLeft size={16} />
                Previous
              </button>
            )}
          </div>

          <div className="flex gap-2">
            {currentStepData.showSkip && (
              <button
                onClick={onSkip}
                className="flex items-center gap-2 px-4 py-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 rounded transition-colors"
                disabled={isActionInProgress}
              >
                <SkipForward size={16} />
                Skip Tour
              </button>
            )}
            
            <button
              onClick={handleNext}
              className="flex items-center gap-2 bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded transition-colors disabled:opacity-50"
              disabled={isActionInProgress}
            >
              {currentStep === steps.length - 1 ? 'Finish' : 'Next'}
              {currentStep < steps.length - 1 && <ChevronRight size={16} />}
            </button>
          </div>
        </div>
      </div>
    </div>
  );

  return createPortal(overlay, document.body);
}