import { Phase3ConfigDemo } from '../components/Phase3ConfigDemo';
import { Phase3RenderingDemo } from '../components/Phase3RenderingDemo';

export default function Phase3Demo() {
  return (
    <div className="min-h-screen bg-dark-900 text-white">
      {/* Header */}
      <div className="bg-dark-800 border-b border-dark-600 px-6 py-4">
        <h1 className="text-2xl font-bold">GPU Charts - Phase 3 Integration Demo</h1>
        <p className="text-gray-400 mt-1">
          Demonstrating configuration system and rendering integration
        </p>
      </div>

      {/* Main Content */}
      <div className="p-6">
        <div className="max-w-7xl mx-auto space-y-8">
          {/* Rendering Demo */}
          <section>
            <h2 className="text-xl font-semibold mb-4">
              Configuration → Rendering Integration
            </h2>
            <Phase3RenderingDemo />
          </section>

          {/* Configuration System */}
          <section>
            <h2 className="text-xl font-semibold mb-4">
              Configuration System Controls
            </h2>
            <Phase3ConfigDemo />
          </section>

          {/* Architecture Overview */}
          <section className="bg-dark-800 rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Architecture Overview</h2>
            
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <div>
                <h3 className="text-lg font-medium mb-3 text-blue-400">Current State</h3>
                <ul className="space-y-2 text-sm">
                  <li className="flex items-start">
                    <span className="text-green-400 mr-2">✓</span>
                    Phase 3 configuration system working
                  </li>
                  <li className="flex items-start">
                    <span className="text-green-400 mr-2">✓</span>
                    Configuration changes propagate to settings
                  </li>
                  <li className="flex items-start">
                    <span className="text-green-400 mr-2">✓</span>
                    Legacy renderer still handles actual rendering
                  </li>
                  <li className="flex items-start">
                    <span className="text-yellow-400 mr-2">⚠</span>
                    Basic config → render connection demonstrated
                  </li>
                </ul>
              </div>

              <div>
                <h3 className="text-lg font-medium mb-3 text-purple-400">Next Steps</h3>
                <ul className="space-y-2 text-sm">
                  <li className="flex items-start">
                    <span className="text-gray-400 mr-2">○</span>
                    Fix WASM dependencies for full renderer
                  </li>
                  <li className="flex items-start">
                    <span className="text-gray-400 mr-2">○</span>
                    Implement data manager integration
                  </li>
                  <li className="flex items-start">
                    <span className="text-gray-400 mr-2">○</span>
                    Port interaction handlers (zoom/pan)
                  </li>
                  <li className="flex items-start">
                    <span className="text-gray-400 mr-2">○</span>
                    Complete migration from legacy system
                  </li>
                </ul>
              </div>
            </div>

            <div className="mt-6 p-4 bg-dark-700 rounded">
              <h3 className="text-sm font-medium text-gray-300 mb-2">Migration Progress</h3>
              <div className="w-full bg-dark-600 rounded-full h-2">
                <div className="bg-gradient-to-r from-blue-500 to-purple-500 h-2 rounded-full" style={{ width: '40%' }}></div>
              </div>
              <p className="text-xs text-gray-400 mt-2">
                40% Complete - Configuration system integrated, rendering connection demonstrated
              </p>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}