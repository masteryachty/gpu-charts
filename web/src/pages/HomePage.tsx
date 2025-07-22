import { Link } from 'react-router-dom';
import { ArrowRight, Zap, Monitor, Gauge, TrendingUp } from 'lucide-react';

export default function HomePage() {
  return (
    <div className="min-h-screen bg-bg-primary">
      {/* Navigation */}
      <nav className="flex items-center justify-between px-8 py-6 border-b border-border">
        <div className="text-2xl font-bold text-gradient">
          GRAPH
        </div>
        <div className="flex items-center gap-6">
          <Link to="/pricing" className="text-text-secondary hover:text-text-primary transition-colors">
            Pricing
          </Link>
          <Link to="/docs" className="text-text-secondary hover:text-text-primary transition-colors">
            Docs
          </Link>
          <Link to="/app" className="text-text-secondary hover:text-text-primary transition-colors">
            Login
          </Link>
          <Link to="/phase3" className="text-text-secondary hover:text-text-primary transition-colors">
            Phase 3 Demo
          </Link>
          <Link to="/culling-test" className="text-text-secondary hover:text-text-primary transition-colors">
            Culling Test
          </Link>
          <Link to="/app" className="btn-primary">
            Start Trading
          </Link>
        </div>
      </nav>

      {/* Hero Section */}
      <section className="px-8 py-24">
        <div className="max-w-6xl mx-auto text-center">
          <div className="mb-8">
            <div className="inline-block text-6xl font-bold mb-6">
              <span className="text-gradient">Ultra-Fast</span>
              <br />
              <span className="text-text-primary">Market Visualization</span>
            </div>
            <p className="text-xl text-text-secondary max-w-2xl mx-auto leading-relaxed">
              Professional trading charts that render at 120fps. 
              WebGPU-accelerated performance for serious traders.
            </p>
          </div>

          <div className="flex items-center justify-center gap-4 mb-16">
            <Link to="/app" className="btn-primary inline-flex items-center gap-2">
              Start Free Trial
              <ArrowRight size={20} />
            </Link>
            <Link to="/pricing" className="btn-secondary">
              View Pricing
            </Link>
          </div>

          {/* Demo Area - Placeholder for GIF */}
          <div className="card p-8 max-w-4xl mx-auto">
            <div className="aspect-video bg-bg-tertiary border border-border flex items-center justify-center">
              <div className="text-center">
                <div className="text-4xl mb-4">📈</div>
                <div className="text-text-secondary">
                  High-speed chart demo GIF will go here
                </div>
                <div className="text-sm text-text-tertiary mt-2">
                  Showing real-time rendering at 120fps
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Features Section */}
      <section className="px-8 py-24 border-t border-border">
        <div className="max-w-6xl mx-auto">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-8">
            <div className="text-center">
              <div className="w-16 h-16 bg-accent-green/10 border border-accent-green/20 flex items-center justify-center mx-auto mb-4">
                <Zap className="text-accent-green" size={32} />
              </div>
              <h3 className="text-xl font-semibold mb-2">Sub-16ms Latency</h3>
              <p className="text-text-secondary">
                Faster than any competitor. Built for split-second trading decisions.
              </p>
            </div>

            <div className="text-center">
              <div className="w-16 h-16 bg-accent-blue/10 border border-accent-blue/20 flex items-center justify-center mx-auto mb-4">
                <Monitor className="text-accent-blue" size={32} />
              </div>
              <h3 className="text-xl font-semibold mb-2">Desktop Optimized</h3>
              <p className="text-text-secondary">
                Multi-monitor ready. No mobile compromises.
              </p>
            </div>

            <div className="text-center">
              <div className="w-16 h-16 bg-accent-purple/10 border border-accent-purple/20 flex items-center justify-center mx-auto mb-4">
                <Gauge className="text-accent-purple" size={32} />
              </div>
              <h3 className="text-xl font-semibold mb-2">WebGPU Powered</h3>
              <p className="text-text-secondary">
                Hardware acceleration for smooth 120fps rendering.
              </p>
            </div>

            <div className="text-center">
              <div className="w-16 h-16 bg-accent-yellow/10 border border-accent-yellow/20 flex items-center justify-center mx-auto mb-4">
                <TrendingUp className="text-accent-yellow" size={32} />
              </div>
              <h3 className="text-xl font-semibold mb-2">Professional Tools</h3>
              <p className="text-text-secondary">
                Advanced indicators and drawing tools for technical analysis.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="px-8 py-12 border-t border-border">
        <div className="max-w-6xl mx-auto text-center">
          <div className="text-text-gradient text-2xl font-bold mb-4">GRAPH</div>
          <p className="text-text-tertiary">
            © 2024 Graph. Built for professional traders.
          </p>
        </div>
      </footer>
    </div>
  );
}