import { useState, useEffect } from 'react';
import { useAppStore } from '../../store/useAppStore';
import { formatExchangeName, getExchangeColor } from '../../services/symbolApi';
import { Eye, EyeOff, TrendingUp, TrendingDown, Minus } from 'lucide-react';
import clsx from 'clsx';
import { ExchangeIndicator, PriceChangeIndicator } from '../common/AccessibleIndicator';

interface ExchangeData {
  exchange: string;
  symbol: string;
  currentPrice?: number;
  change24h?: number;
  visible: boolean;
}

interface ChartLegendProps {
  className?: string;
}

export default function ChartLegend({ className }: ChartLegendProps) {
  const { comparisonMode, selectedExchanges, baseSymbol } = useAppStore();
  const [exchangeData, setExchangeData] = useState<ExchangeData[]>([]);
  
  // Initialize exchange data when selections change
  useEffect(() => {
    if (comparisonMode && selectedExchanges && baseSymbol) {
      const newData = selectedExchanges.map(exchange => ({
        exchange,
        symbol: `${exchange}:${baseSymbol}`,
        visible: true,
        // These will be populated when we fetch real data
        currentPrice: undefined,
        change24h: undefined,
      }));
      setExchangeData(newData);
    } else {
      setExchangeData([]);
    }
  }, [comparisonMode, selectedExchanges, baseSymbol]);
  
  // Only show legend in comparison mode with multiple exchanges
  if (!comparisonMode || !selectedExchanges || selectedExchanges.length < 2) {
    return null;
  }
  
  const toggleVisibility = (exchange: string) => {
    setExchangeData(prev => 
      prev.map(data => 
        data.exchange === exchange 
          ? { ...data, visible: !data.visible }
          : data
      )
    );
  };
  
  const formatPrice = (price?: number) => {
    if (!price) return '---';
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(price);
  };
  
  const formatChange = (change?: number) => {
    if (!change) return '0.00%';
    const sign = change >= 0 ? '+' : '';
    return `${sign}${change.toFixed(2)}%`;
  };
  
  return (
    <div className={clsx(
      "bg-gray-800 border border-gray-600 rounded-lg p-3",
      className
    )}>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-white text-sm font-semibold">Exchange Comparison</h3>
        <span className="text-xs text-gray-400">{baseSymbol}</span>
      </div>
      
      <div className="space-y-2">
        {exchangeData.map((data) => {
          const color = getExchangeColor(data.exchange);
          const isVisible = data.visible;
          
          return (
            <div
              key={data.exchange}
              className={clsx(
                "flex items-center justify-between p-2 rounded transition-all",
                isVisible ? "bg-gray-700" : "bg-gray-900 opacity-60"
              )}
            >
              <div className="flex items-center gap-3">
                <ExchangeIndicator 
                  exchange={data.exchange}
                  active={isVisible}
                  size="sm"
                />
                
                {/* Exchange details */}
                <div className="flex flex-col">
                  <span className={clsx(
                    "text-sm font-medium",
                    isVisible ? "text-white" : "text-gray-400"
                  )}>
                    {formatExchangeName(data.exchange)}
                  </span>
                  {data.currentPrice && (
                    <span className="text-xs text-gray-400">
                      {formatPrice(data.currentPrice)}
                    </span>
                  )}
                </div>
              </div>
              
              <div className="flex items-center gap-2">
                {/* Price change with accessible indicators */}
                {data.change24h !== undefined && (
                  <div className="flex items-center gap-1">
                    {data.change24h > 0 ? (
                      <TrendingUp className="w-3 h-3 text-green-400" aria-hidden="true" />
                    ) : data.change24h < 0 ? (
                      <TrendingDown className="w-3 h-3 text-red-400" aria-hidden="true" />
                    ) : (
                      <Minus className="w-3 h-3 text-gray-400" aria-hidden="true" />
                    )}
                    <span className={clsx(
                      "text-xs font-medium",
                      data.change24h >= 0 ? "text-green-400" : "text-red-400"
                    )}>
                      {formatChange(data.change24h)}
                    </span>
                  </div>
                )}
                
                {/* Visibility toggle */}
                <button
                  onClick={() => toggleVisibility(data.exchange)}
                  className="p-1 hover:bg-gray-600 rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500"
                  aria-label={`${isVisible ? 'Hide' : 'Show'} ${formatExchangeName(data.exchange)} data`}
                >
                  {isVisible ? (
                    <Eye className="w-4 h-4 text-gray-400" />
                  ) : (
                    <EyeOff className="w-4 h-4 text-gray-500" />
                  )}
                  <span className="sr-only">
                    {isVisible ? 'Hide' : 'Show'} {formatExchangeName(data.exchange)} data
                  </span>
                </button>
              </div>
            </div>
          );
        })}
      </div>
      
      {/* Price spread indicator */}
      {exchangeData.length === 2 && 
       exchangeData[0].currentPrice && 
       exchangeData[1].currentPrice && (
        <div className="mt-3 pt-3 border-t border-gray-700">
          <div className="flex items-center justify-between text-xs">
            <span className="text-gray-400">Spread</span>
            <span className="text-white font-medium">
              {formatPrice(Math.abs(exchangeData[0].currentPrice - exchangeData[1].currentPrice))}
              <span className="text-gray-400 ml-1">
                ({((Math.abs(exchangeData[0].currentPrice - exchangeData[1].currentPrice) / 
                   Math.min(exchangeData[0].currentPrice, exchangeData[1].currentPrice)) * 100).toFixed(3)}%)
              </span>
            </span>
          </div>
        </div>
      )}
    </div>
  );
}