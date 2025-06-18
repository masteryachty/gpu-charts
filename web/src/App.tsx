import { Routes, Route } from 'react-router-dom';
import HomePage from './pages/HomePage';
import TradingApp from './pages/TradingApp';
import { ErrorBoundary } from './index';
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
    <ErrorBoundary
      enableAutoRecovery={true}
      maxRetryAttempts={3}
      enableReporting={true}
      componentName="App"
    >
      <div className="min-h-screen bg-bg-primary">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/app/*" element={<TradingApp />} />
          <Route path="/pricing" element={<div>Pricing Page</div>} />
          <Route path="/about" element={<div>About Page</div>} />
          <Route path="/docs" element={<div>Documentation</div>} />
        </Routes>
        
        {/* Global error notification system - temporarily disabled for testing */}
        {/* <ErrorNotificationCenter
          position="top-right"
          maxNotifications={5}
          autoHideTimeoutMs={8000}
          enableSounds={false}
          showDetailedInfo={false}
        /> */}
      </div>
    </ErrorBoundary>
  );
}

export default App;