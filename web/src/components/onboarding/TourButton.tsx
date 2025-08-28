import { useState } from 'react';
import { Play, BookOpen, RefreshCw, ChevronDown } from 'lucide-react';
import { useTour, TOURS } from './TourManager';

interface TourButtonProps {
  variant?: 'floating' | 'inline' | 'menu';
  className?: string;
}

export function TourButton({ variant = 'floating', className = '' }: TourButtonProps) {
  const { startTour, hasCompletedTour, resetTour, isTourActive } = useTour();
  const [showMenu, setShowMenu] = useState(false);

  if (variant === 'floating') {
    return (
      <div className={`fixed bottom-4 right-4 z-50 ${className}`}>
        <button
          onClick={() => setShowMenu(!showMenu)}
          disabled={isTourActive}
          className="bg-blue-500 hover:bg-blue-600 text-white p-3 rounded-full shadow-lg transition-all duration-200 hover:shadow-xl disabled:opacity-50 group"
          aria-label="Open tour menu"
        >
          <div className="flex items-center gap-2">
            <BookOpen size={20} className="group-hover:scale-110 transition-transform" />
            <ChevronDown 
              size={16} 
              className={`transition-transform ${showMenu ? 'rotate-180' : ''}`}
            />
          </div>
        </button>

        {showMenu && (
          <div className="absolute bottom-full right-0 mb-2 bg-white rounded-lg shadow-2xl border min-w-64 py-2">
            <div className="px-4 py-2 border-b text-sm font-semibold text-gray-700">
              Interactive Tours
            </div>
            
            {Object.entries(TOURS).map(([tourId, tour]) => (
              <div key={tourId} className="px-4 py-2">
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <div className="font-medium text-gray-900 text-sm">
                      {tour.name}
                    </div>
                    <div className="text-xs text-gray-500">
                      {hasCompletedTour(tourId) ? '✅ Completed' : 'New'}
                    </div>
                  </div>
                  
                  <div className="flex gap-1">
                    <button
                      onClick={() => {
                        startTour(tourId);
                        setShowMenu(false);
                      }}
                      className="bg-blue-500 hover:bg-blue-600 text-white px-3 py-1 rounded text-xs flex items-center gap-1 transition-colors"
                    >
                      <Play size={12} />
                      {hasCompletedTour(tourId) ? 'Replay' : 'Start'}
                    </button>
                    
                    {hasCompletedTour(tourId) && (
                      <button
                        onClick={() => resetTour(tourId)}
                        className="text-gray-500 hover:text-gray-700 p-1 rounded transition-colors"
                        title="Reset tour progress"
                      >
                        <RefreshCw size={12} />
                      </button>
                    )}
                  </div>
                </div>
              </div>
            ))}
            
            <div className="px-4 py-2 border-t">
              <div className="text-xs text-gray-500">
                💡 Tours help you discover features and best practices
              </div>
            </div>
          </div>
        )}

        {/* Click outside to close menu */}
        {showMenu && (
          <div 
            className="fixed inset-0 z-[-1]"
            onClick={() => setShowMenu(false)}
          />
        )}
      </div>
    );
  }

  if (variant === 'inline') {
    return (
      <div className={`space-y-2 ${className}`}>
        <div className="text-sm font-medium text-gray-700 mb-3">
          Interactive Tours
        </div>
        
        {Object.entries(TOURS).map(([tourId, tour]) => (
          <div key={tourId} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
            <div>
              <div className="font-medium text-gray-900 text-sm">
                {tour.name}
              </div>
              <div className="text-xs text-gray-600 mt-1">
                {tour.steps.length} steps • {hasCompletedTour(tourId) ? 'Completed' : 'Not started'}
              </div>
            </div>
            
            <div className="flex gap-2">
              <button
                onClick={() => startTour(tourId)}
                disabled={isTourActive}
                className="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded text-sm flex items-center gap-2 transition-colors disabled:opacity-50"
              >
                <Play size={14} />
                {hasCompletedTour(tourId) ? 'Replay' : 'Start'}
              </button>
              
              {hasCompletedTour(tourId) && (
                <button
                  onClick={() => resetTour(tourId)}
                  className="text-gray-500 hover:text-gray-700 p-2 rounded transition-colors"
                  title="Reset progress"
                >
                  <RefreshCw size={14} />
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
    );
  }

  // Menu variant - simple buttons for integration into existing menus
  return (
    <div className={className}>
      {Object.entries(TOURS).map(([tourId, tour]) => (
        <button
          key={tourId}
          onClick={() => startTour(tourId)}
          disabled={isTourActive}
          className="w-full flex items-center gap-3 px-4 py-3 text-left hover:bg-gray-100 transition-colors disabled:opacity-50"
        >
          <Play size={16} className="text-blue-500" />
          <div>
            <div className="font-medium text-gray-900 text-sm">
              {tour.name}
            </div>
            {hasCompletedTour(tourId) && (
              <div className="text-xs text-green-600">✅ Completed</div>
            )}
          </div>
        </button>
      ))}
    </div>
  );
}

// Quick start button for new users
export function QuickStartButton({ className = '' }: { className?: string }) {
  const { startTour, hasCompletedTour } = useTour();

  if (hasCompletedTour('first-time')) {
    return null; // Hide for returning users
  }

  return (
    <button
      onClick={() => startTour('first-time')}
      className={`bg-gradient-to-r from-blue-500 to-purple-600 hover:from-blue-600 hover:to-purple-700 text-white px-6 py-3 rounded-lg font-semibold flex items-center gap-2 shadow-lg hover:shadow-xl transition-all duration-200 ${className}`}
    >
      <Play size={20} />
      Take the Tour
      <span className="text-xs bg-white bg-opacity-20 px-2 py-1 rounded ml-2">
        2 min
      </span>
    </button>
  );
}