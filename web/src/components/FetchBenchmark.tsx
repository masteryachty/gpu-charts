import React, { useState } from 'react';

interface BenchmarkResult {
  oldMethodTime: number;
  newMethodTime: number;
  comparison: string;
  memoryUsage?: string;
}

const FetchBenchmark: React.FC = () => {
  const [isRunning, setIsRunning] = useState(false);
  const [results, setResults] = useState<BenchmarkResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [iterations, setIterations] = useState(10);
  const [testUrl, setTestUrl] = useState('https://api.rednax.io/api/data?symbol=BTC-USD&type=MD&start=1734567890&end=1734567900&columns=time,best_bid');

  const runBenchmark = async () => {
    setIsRunning(true);
    setError(null);
    setResults(null);

    try {
      // Import and initialize WASM module
      const wasmModule = await import('@pkg/GPU_charting');
      await wasmModule.default();
      
      // Create benchmark instance
      const benchmark = new wasmModule.FetchBenchmark(testUrl);
      
      // Run the comparison
      const comparisonResult = await benchmark.run_comparison(iterations);
      
      // Parse the result string to extract times
      const oldMatch = comparisonResult.match(/Old method average: ([\d.]+) ms/);
      const newMatch = comparisonResult.match(/New method average: ([\d.]+) ms/);
      
      if (oldMatch && newMatch) {
        setResults({
          oldMethodTime: parseFloat(oldMatch[1]),
          newMethodTime: parseFloat(newMatch[1]),
          comparison: comparisonResult
        });
      }
      
      // Run memory usage test with fewer iterations
      if (iterations <= 20) {
        const memoryResult = await benchmark.benchmark_memory_usage(5);
        setResults(prev => prev ? { ...prev, memoryUsage: memoryResult } : null);
      }
      
    } catch (err) {
      console.error('Benchmark failed:', err);
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setIsRunning(false);
    }
  };

  const getSpeedupClass = (oldTime: number, newTime: number) => {
    const speedup = oldTime / newTime;
    if (speedup > 1.2) return 'text-green-600 font-bold';
    if (speedup > 1.05) return 'text-green-500';
    if (speedup < 0.95) return 'text-red-500';
    if (speedup < 0.8) return 'text-red-600 font-bold';
    return 'text-gray-600';
  };

  return (
    <div className="p-6 max-w-4xl mx-auto">
      <h2 className="text-2xl font-bold mb-4">Fetch Method Benchmark</h2>
      
      <div className="bg-gray-100 p-4 rounded mb-6">
        <p className="text-sm text-gray-600 mb-4">
          This benchmark compares two fetch methods for retrieving binary data:
        </p>
        <ul className="list-disc list-inside text-sm space-y-1 mb-4">
          <li><strong>Old Method:</strong> Request.get() → array_buffer() → JsFuture → ArrayBuffer</li>
          <li><strong>New Method:</strong> FetchClient wrapper → Vec&lt;u8&gt; → Uint8Array → ArrayBuffer</li>
        </ul>
      </div>

      <div className="space-y-4 mb-6">
        <div>
          <label className="block text-sm font-medium mb-1">Test URL:</label>
          <input
            type="text"
            value={testUrl}
            onChange={(e) => setTestUrl(e.target.value)}
            className="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            disabled={isRunning}
          />
        </div>
        
        <div>
          <label className="block text-sm font-medium mb-1">Iterations:</label>
          <input
            type="number"
            value={iterations}
            onChange={(e) => setIterations(Math.max(1, parseInt(e.target.value) || 1))}
            min="1"
            max="100"
            className="w-32 px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
            disabled={isRunning}
          />
          <span className="ml-2 text-sm text-gray-500">
            (Memory test runs only if ≤20 iterations)
          </span>
        </div>
      </div>

      <button
        onClick={runBenchmark}
        disabled={isRunning || !testUrl}
        className={`px-6 py-2 rounded font-medium transition-colors ${
          isRunning || !testUrl
            ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
            : 'bg-blue-500 text-white hover:bg-blue-600'
        }`}
      >
        {isRunning ? 'Running Benchmark...' : 'Run Benchmark'}
      </button>

      {error && (
        <div className="mt-6 p-4 bg-red-50 text-red-800 rounded">
          <h3 className="font-bold mb-1">Error</h3>
          <p className="text-sm">{error}</p>
        </div>
      )}

      {results && (
        <div className="mt-6 space-y-4">
          <div className="bg-white p-4 rounded border border-gray-200">
            <h3 className="font-bold mb-3">Performance Results</h3>
            
            <div className="grid grid-cols-2 gap-4 mb-4">
              <div>
                <h4 className="font-medium text-gray-600">Old Method (Request API)</h4>
                <p className="text-2xl font-bold">{results.oldMethodTime.toFixed(2)} ms</p>
              </div>
              <div>
                <h4 className="font-medium text-gray-600">New Method (FetchClient)</h4>
                <p className="text-2xl font-bold">{results.newMethodTime.toFixed(2)} ms</p>
              </div>
            </div>
            
            <div className="border-t pt-3">
              <p className={`text-lg ${getSpeedupClass(results.oldMethodTime, results.newMethodTime)}`}>
                Speedup: {(results.oldMethodTime / results.newMethodTime).toFixed(2)}x
                {results.newMethodTime < results.oldMethodTime ? ' faster' : ' slower'}
              </p>
              <p className="text-sm text-gray-600 mt-1">
                Difference: {Math.abs(results.oldMethodTime - results.newMethodTime).toFixed(2)} ms
              </p>
            </div>
          </div>

          <div className="bg-gray-50 p-4 rounded">
            <h3 className="font-bold mb-2">Detailed Comparison</h3>
            <pre className="text-xs font-mono whitespace-pre-wrap">{results.comparison}</pre>
          </div>

          {results.memoryUsage && (
            <div className="bg-blue-50 p-4 rounded">
              <h3 className="font-bold mb-2">Memory Usage Analysis</h3>
              <pre className="text-xs font-mono whitespace-pre-wrap">{results.memoryUsage}</pre>
            </div>
          )}
        </div>
      )}

      <div className="mt-8 text-sm text-gray-500">
        <h4 className="font-medium mb-2">Notes:</h4>
        <ul className="list-disc list-inside space-y-1">
          <li>Results may vary based on network latency, file size, and browser performance</li>
          <li>The benchmark includes a warm-up run to avoid cold start bias</li>
          <li>Memory usage requires performance.memory API (Chrome/Edge only)</li>
          <li>Lower times are better - indicates faster execution</li>
        </ul>
      </div>
    </div>
  );
};

export default FetchBenchmark;