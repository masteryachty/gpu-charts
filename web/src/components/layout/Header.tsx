import { Search, ChevronDown, Settings, User } from 'lucide-react';
import { useAppStore } from '../../store/useAppStore';

export default function Header() {
  const { currentSymbol, setCurrentSymbol, isConnected } = useAppStore();

  return (
    <header className="h-16 bg-bg-primary border-b border-border flex items-center px-6">
      {/* Logo */}
      <div className="text-xl font-bold text-gradient mr-8">
        GRAPH
      </div>

      {/* Search / Symbol Selector */}
      <div className="flex-1 max-w-md">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-text-tertiary" size={16} />
          <input
            type="text"
            value={currentSymbol}
            onChange={(e) => setCurrentSymbol(e.target.value.toUpperCase())}
            className="input-primary pl-10 pr-4 w-full"
            placeholder="Search symbols..."
          />
          <ChevronDown className="absolute right-3 top-1/2 transform -translate-y-1/2 text-text-tertiary" size={16} />
        </div>
      </div>

      {/* Watchlist */}
      <div className="mx-8">
        <button className="flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors">
          <span>Watchlist</span>
          <ChevronDown size={16} />
        </button>
      </div>

      {/* Connection Status */}
      <div className="flex items-center gap-2 mr-6">
        <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-accent-green' : 'bg-accent-red'}`} />
        <span className="text-xs text-text-tertiary">
          {isConnected ? 'Connected' : 'Disconnected'}
        </span>
      </div>

      {/* User Menu */}
      <div className="flex items-center gap-4">
        <button className="p-2 hover:bg-bg-secondary transition-colors">
          <Settings size={20} className="text-text-secondary" />
        </button>
        <button className="flex items-center gap-2 hover:bg-bg-secondary px-3 py-2 transition-colors">
          <User size={20} className="text-text-secondary" />
          <ChevronDown size={16} className="text-text-tertiary" />
        </button>
      </div>
    </header>
  );
}