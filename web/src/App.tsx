import { Routes, Route } from 'react-router-dom';
import HomePage from './pages/HomePage';
import TradingApp from './pages/TradingApp';
// import TestNewArchitecture from './pages/TestNewArchitecture';
// import { ErrorBoundary } from './index';
import { useAppStore } from './store/useAppStore';

// Initialize the integration system - temporarily disabled for testing
// initializeIntegrationSystem({
//   enableErrorReporting: true,
//   enablePerformanceMonitoring: true,
//   debugMode: process.env.NODE_ENV === 'development'
// });

function App() {
  // Make store available globally for testing
  const store = useAppStore;
  if (typeof window !== 'undefined') {
    (window as any).__zustandStore = store;
  }

  return (
    <div className="min-h-screen bg-bg-primary">
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/app/*" element={<TradingApp />} />
        {/* <Route path="/test-new" element={<TestNewArchitecture />} /> */}
        <Route path="/pricing" element={<div>Pricing Page</div>} />
        <Route path="/about" element={<div>About Page</div>} />
        <Route path="/docs" element={<div>Documentation</div>} />
      </Routes>
    </div>
  );
}

export default App;