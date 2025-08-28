import { Link } from 'react-router-dom';
import { ArrowLeft, Github, Twitter, Mail } from 'lucide-react';

export default function AboutPage() {
  return (
    <div className="min-h-screen bg-bg-primary">
      {/* Navigation */}
      <nav className="flex items-center justify-between px-8 py-6 border-b border-border">
        <Link to="/" className="text-2xl font-bold text-gradient">
          GRAPH
        </Link>
        <Link 
          to="/" 
          className="inline-flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors"
        >
          <ArrowLeft size={20} />
          Back to Home
        </Link>
      </nav>

      {/* About Section */}
      <section className="px-8 py-24">
        <div className="max-w-4xl mx-auto">
          <div className="text-center mb-16">
            <h1 className="text-4xl font-bold text-text-primary mb-4">
              About Graph
            </h1>
            <p className="text-xl text-text-secondary">
              Revolutionizing financial data visualization with cutting-edge technology
            </p>
          </div>

          <div className="prose prose-invert max-w-none">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-12 mb-16">
              <div>
                <h2 className="text-2xl font-bold text-text-primary mb-4">Our Mission</h2>
                <p className="text-text-secondary leading-relaxed">
                  We believe that every millisecond matters in trading. Traditional charting platforms 
                  are built on outdated technology that introduces unnecessary delays. Graph leverages 
                  modern web technologies like WebGPU and WebAssembly to deliver the fastest, most 
                  responsive trading charts ever built.
                </p>
              </div>

              <div>
                <h2 className="text-2xl font-bold text-text-primary mb-4">Technology</h2>
                <p className="text-text-secondary leading-relaxed">
                  Built from the ground up in Rust and compiled to WebAssembly, Graph delivers 
                  native-level performance directly in your browser. WebGPU acceleration ensures 
                  smooth 120fps rendering even with millions of data points, while our optimized 
                  data pipeline achieves sub-16ms latency.
                </p>
              </div>
            </div>

            <div className="bg-bg-secondary border border-border rounded-lg p-8 mb-16">
              <h2 className="text-2xl font-bold text-text-primary mb-6">Key Features</h2>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <div>
                  <h3 className="font-semibold text-text-primary mb-2">⚡ Ultra-Low Latency</h3>
                  <p className="text-text-secondary">Sub-16ms data-to-pixel rendering</p>
                </div>
                <div>
                  <h3 className="font-semibold text-text-primary mb-2">🚀 WebGPU Acceleration</h3>
                  <p className="text-text-secondary">Hardware-accelerated graphics</p>
                </div>
                <div>
                  <h3 className="font-semibold text-text-primary mb-2">📊 Advanced Analytics</h3>
                  <p className="text-text-secondary">Professional trading indicators</p>
                </div>
                <div>
                  <h3 className="font-semibold text-text-primary mb-2">🔒 Privacy First</h3>
                  <p className="text-text-secondary">Your data stays in your browser</p>
                </div>
              </div>
            </div>

            <div className="text-center">
              <h2 className="text-2xl font-bold text-text-primary mb-6">Get in Touch</h2>
              <p className="text-text-secondary mb-8">
                Have questions or feedback? We'd love to hear from you.
              </p>
              
              <div className="flex justify-center gap-6">
                <a 
                  href="mailto:hello@graph.com" 
                  className="inline-flex items-center gap-2 bg-bg-secondary border border-border px-4 py-2 rounded-lg text-text-primary hover:bg-bg-tertiary transition-colors"
                >
                  <Mail size={20} />
                  Email
                </a>
                <a 
                  href="https://github.com/graph" 
                  className="inline-flex items-center gap-2 bg-bg-secondary border border-border px-4 py-2 rounded-lg text-text-primary hover:bg-bg-tertiary transition-colors"
                >
                  <Github size={20} />
                  GitHub
                </a>
                <a 
                  href="https://twitter.com/graph" 
                  className="inline-flex items-center gap-2 bg-bg-secondary border border-border px-4 py-2 rounded-lg text-text-primary hover:bg-bg-tertiary transition-colors"
                >
                  <Twitter size={20} />
                  Twitter
                </a>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}