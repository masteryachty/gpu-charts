interface LoadingSkeletonProps {
  className?: string;
  variant?: 'pulse' | 'wave';
  width?: string | number;
  height?: string | number;
  rounded?: boolean;
}

export function LoadingSkeleton({
  className = '',
  variant = 'pulse',
  width = '100%',
  height = '1rem',
  rounded = false
}: LoadingSkeletonProps) {
  const baseClasses = 'bg-gray-700 animate-pulse';
  const variantClasses = variant === 'wave' 
    ? 'animate-[shimmer_2s_ease-in-out_infinite] bg-gradient-to-r from-gray-700 via-gray-600 to-gray-700 bg-[length:200%_100%]' 
    : 'animate-pulse';
  const roundedClass = rounded ? 'rounded-full' : 'rounded';

  const style: React.CSSProperties = {
    width: typeof width === 'number' ? `${width}px` : width,
    height: typeof height === 'number' ? `${height}px` : height,
  };

  return (
    <div 
      className={`${baseClasses} ${variantClasses} ${roundedClass} ${className}`}
      style={style}
      aria-hidden="true"
    />
  );
}

interface ChartLoadingSkeletonProps {
  className?: string;
}

export function ChartLoadingSkeleton({ className = '' }: ChartLoadingSkeletonProps) {
  return (
    <div 
      className={`bg-gray-900 p-4 ${className}`}
      role="status"
      aria-label="Chart loading"
    >
      {/* Y-axis labels skeleton */}
      <div className="absolute left-0 top-4 bottom-4 w-16 flex flex-col justify-between">
        {[...Array(8)].map((_, i) => (
          <LoadingSkeleton key={i} width="3rem" height="0.75rem" />
        ))}
      </div>

      {/* Chart area skeleton */}
      <div className="ml-16 mr-4 h-full relative">
        {/* Price lines skeleton */}
        <div className="absolute inset-0 flex flex-col justify-between">
          {[...Array(5)].map((_, i) => (
            <div key={i} className="flex items-center justify-between">
              <LoadingSkeleton width="100%" height="2px" className="opacity-50" />
            </div>
          ))}
        </div>

        {/* Candlestick pattern skeleton */}
        <div className="absolute bottom-0 left-0 right-0 h-64 flex items-end gap-1">
          {[...Array(40)].map((_, i) => (
            <div key={i} className="flex-1 flex flex-col items-center">
              <LoadingSkeleton 
                width="60%" 
                height={`${Math.random() * 60 + 20}%`}
                className="mb-1"
              />
              <LoadingSkeleton width="20%" height="2px" />
            </div>
          ))}
        </div>
      </div>

      {/* X-axis labels skeleton */}
      <div className="absolute bottom-0 left-16 right-4 h-8 flex justify-between items-center">
        {[...Array(6)].map((_, i) => (
          <LoadingSkeleton key={i} width="4rem" height="0.75rem" />
        ))}
      </div>

      {/* Loading indicator in center */}
      <div className="absolute inset-0 flex items-center justify-center">
        <div className="bg-gray-800/90 rounded-lg p-6 text-center">
          <div className="animate-spin text-blue-500 text-3xl mb-3" aria-hidden="true">⚡</div>
          <div className="text-white font-medium mb-1">Initializing Chart Engine</div>
          <div className="text-gray-400 text-sm">Setting up WebGPU rendering...</div>
          <LoadingSkeleton width="12rem" height="4px" className="mt-3" variant="wave" />
        </div>
      </div>
    </div>
  );
}

interface ControlsLoadingSkeletonProps {
  className?: string;
}

export function ControlsLoadingSkeleton({ className = '' }: ControlsLoadingSkeletonProps) {
  return (
    <div 
      className={`space-y-4 ${className}`}
      role="status"
      aria-label="Chart controls loading"
    >
      {/* Symbol selector skeleton */}
      <div className="space-y-2">
        <LoadingSkeleton width="4rem" height="0.875rem" />
        <LoadingSkeleton width="100%" height="2.5rem" rounded />
      </div>

      {/* Time range selector skeleton */}
      <div className="space-y-2">
        <LoadingSkeleton width="5rem" height="0.875rem" />
        <div className="grid grid-cols-2 gap-2">
          {[...Array(4)].map((_, i) => (
            <LoadingSkeleton key={i} width="100%" height="2.25rem" rounded />
          ))}
        </div>
      </div>

      {/* Comparison mode skeleton */}
      <div className="space-y-2">
        <LoadingSkeleton width="8rem" height="0.875rem" />
        <LoadingSkeleton width="100%" height="2.5rem" rounded />
      </div>
    </div>
  );
}

interface SidebarLoadingSkeletonProps {
  className?: string;
}

export function SidebarLoadingSkeleton({ className = '' }: SidebarLoadingSkeletonProps) {
  return (
    <div 
      className={`w-16 bg-gray-800 border-r border-gray-600 ${className}`}
      role="status"
      aria-label="Sidebar loading"
    >
      {/* Header skeleton */}
      <div className="h-16 flex items-center justify-center border-b border-gray-700">
        <LoadingSkeleton width="2rem" height="1rem" />
      </div>

      {/* Navigation items skeleton */}
      <div className="py-4 space-y-1">
        {[...Array(4)].map((_, i) => (
          <div key={i} className="px-4 py-3">
            <LoadingSkeleton width="1.25rem" height="1.25rem" rounded />
          </div>
        ))}
      </div>
    </div>
  );
}

interface DataLoadingSkeletonProps {
  className?: string;
  rows?: number;
}

export function DataLoadingSkeleton({ className = '', rows = 5 }: DataLoadingSkeletonProps) {
  return (
    <div 
      className={`space-y-3 ${className}`}
      role="status"
      aria-label="Data loading"
    >
      {[...Array(rows)].map((_, i) => (
        <div key={i} className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <LoadingSkeleton width="2rem" height="2rem" rounded />
            <div className="space-y-1">
              <LoadingSkeleton width="6rem" height="1rem" />
              <LoadingSkeleton width="4rem" height="0.75rem" />
            </div>
          </div>
          <div className="text-right space-y-1">
            <LoadingSkeleton width="4rem" height="1rem" />
            <LoadingSkeleton width="3rem" height="0.75rem" />
          </div>
        </div>
      ))}
    </div>
  );
}