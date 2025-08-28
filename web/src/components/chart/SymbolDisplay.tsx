import { formatExchangeName, getExchangeColor } from '../../services/symbolApi';
import { ExchangeIndicator } from '../common/AccessibleIndicator';

interface SymbolDisplayProps {
  baseSymbol: string;
  exchange: string;
}

export default function SymbolDisplay({ baseSymbol, exchange }: SymbolDisplayProps) {
  return (
    <div className="space-y-2">
      <label className="text-gray-300 text-sm font-medium">Current Symbol</label>
      <div
        data-testid="current-symbol"
        className="w-full bg-gray-700 border border-gray-600 text-white rounded px-3 py-2 text-sm"
        role="status"
        aria-label={`Currently viewing ${baseSymbol} on ${formatExchangeName(exchange)}`}
      >
        <div className="flex items-center justify-between">
          <span className="font-mono font-bold">{baseSymbol}</span>
          <ExchangeIndicator 
            exchange={exchange}
            active={true}
            size="sm"
          />
        </div>
      </div>
    </div>
  );
}