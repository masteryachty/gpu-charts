import { Routes, Route } from 'react-router-dom';
import HomePage from './pages/HomePage';
import TradingApp from './pages/TradingApp';

function App() {
  return (
    <div className="min-h-screen bg-bg-primary">
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/app/*" element={<TradingApp />} />
        <Route path="/pricing" element={<div>Pricing Page</div>} />
        <Route path="/about" element={<div>About Page</div>} />
        <Route path="/docs" element={<div>Documentation</div>} />
      </Routes>
    </div>
  );
}

export default App;