import { useCallback } from 'react';

interface TimeRangePreset {
  id: string;
  label: string;
}

interface TimeRangeSelectorProps {
  onTimeRangeChange: (startTime: number, endTime: number) => void;
  currentStartTime: number;
}

export default function TimeRangeSelector({ onTimeRangeChange, currentStartTime }: TimeRangeSelectorProps) {
  const timePresets: TimeRangePreset[] = [
    { id: '1h', label: '1 Hour' },
    { id: '4h', label: '4 Hours' }, 
    { id: '1d', label: '1 Day' },
    { id: '1w', label: '1 Week' }
  ];

  const handleTimeRangePreset = useCallback((preset: string) => {
    const now = Math.floor(Date.now() / 1000);
    let startTime: number;
    
    switch (preset) {
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
        startTime = now - 86400;
    }
    onTimeRangeChange(startTime, now);
  }, [onTimeRangeChange]);

  return (
    <fieldset className="space-y-2">
      <legend className="text-gray-300 text-sm font-medium">Time Range</legend>
      <div 
        className="grid grid-cols-2 gap-2"
        role="radiogroup"
        aria-labelledby="time-range-legend"
        aria-describedby="time-range-description"
      >
        <div id="time-range-legend" className="sr-only">Select time range for chart data</div>
        <div id="time-range-description" className="sr-only">
          Choose how far back in time to display chart data, from 1 hour to 1 week
        </div>
        {timePresets.map((preset) => {
          const now = Math.floor(Date.now() / 1000);
          let presetStart: number;
          
          switch (preset.id) {
            case '1h': presetStart = now - 3600; break;
            case '4h': presetStart = now - 14400; break;
            case '1d': presetStart = now - 86400; break;
            case '1w': presetStart = now - 604800; break;
            default: presetStart = now - 86400;
          }
          
          const isActive = Math.abs(currentStartTime - presetStart) < 60;
          
          return (
            <button
              key={preset.id}
              onClick={() => handleTimeRangePreset(preset.id)}
              className={`
                px-3 py-2 text-sm font-medium rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500
                ${isActive 
                  ? 'bg-blue-600 text-white ring-2 ring-blue-300 font-bold relative' 
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                }
              `}
              role="radio"
              aria-checked={isActive}
              aria-label={`Set time range to ${preset.label.toLowerCase()}`}
            >
              <div className="flex items-center gap-2">
                {isActive && (
                  <span className="text-xs" aria-hidden="true">●</span>
                )}
                <span>{preset.label}</span>
                {isActive && (
                  <span className="text-xs opacity-75" aria-hidden="true">✓</span>
                )}
              </div>
            </button>
          );
        })}
      </div>
    </fieldset>
  );
}