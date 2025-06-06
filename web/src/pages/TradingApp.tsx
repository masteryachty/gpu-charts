import { Routes, Route } from 'react-router-dom';
import Header from '../components/layout/Header';
import Sidebar from '../components/layout/Sidebar';
import StatusBar from '../components/layout/StatusBar';
import WasmCanvas from '../components/chart/WasmCanvas';

function ChartView() {
  return (
    <div className="flex-1 flex">
      <Sidebar />
      <main className="flex-1 flex flex-col">
        <WasmCanvas />
        <StatusBar />
      </main>
    </div>
  );
}

export default function TradingApp() {
  return (
    <div className="h-screen bg-bg-primary flex flex-col">
      <Header />
      <Routes>
        <Route path="/" element={<ChartView />} />
        <Route path="/chart/:symbol" element={<ChartView />} />
      </Routes>
    </div>
  );
}