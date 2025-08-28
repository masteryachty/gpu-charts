import { Link } from 'react-router-dom';
import { ArrowLeft, BookOpen, Code, Zap, Settings } from 'lucide-react';

export default function DocsPage() {
  const sections = [
    {
      title: 'Getting Started',
      icon: BookOpen,
      description: 'Learn the basics of Graph trading platform',
      items: [
        'Quick Start Guide',
        'Setting up your workspace',
        'Understanding the interface',
        'Your first chart'
      ]
    },
    {
      title: 'Chart Features',
      icon: Zap,
      description: 'Master advanced charting capabilities',
      items: [
        'Chart types and timeframes',
        'Technical indicators',
        'Drawing tools',
        'Multi-exchange comparison',
        'Real-time data feeds'
      ]
    },
    {
      title: 'API Reference',
      icon: Code,
      description: 'Integrate Graph with your applications',
      items: [
        'REST API endpoints',
        'WebSocket connections',
        'Authentication',
        'Rate limits',
        'SDK documentation'
      ]
    },
    {
      title: 'Configuration',
      icon: Settings,
      description: 'Customize Graph to your needs',
      items: [
        'Quality presets',
        'Keyboard shortcuts',
        'Theme customization',
        'Layout management',
        'Performance tuning'
      ]
    }
  ];

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

      {/* Documentation Section */}
      <section className="px-8 py-24">
        <div className="max-w-6xl mx-auto">
          <div className="text-center mb-16">
            <h1 className="text-4xl font-bold text-text-primary mb-4">
              Documentation
            </h1>
            <p className="text-xl text-text-secondary">
              Everything you need to master Graph trading platform
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            {sections.map((section) => {
              const Icon = section.icon;
              return (
                <div key={section.title} className="card p-8">
                  <div className="flex items-center gap-4 mb-4">
                    <div className="w-12 h-12 bg-accent-blue/10 border border-accent-blue/20 flex items-center justify-center rounded-lg">
                      <Icon className="text-accent-blue" size={24} />
                    </div>
                    <div>
                      <h2 className="text-xl font-bold text-text-primary">
                        {section.title}
                      </h2>
                    </div>
                  </div>
                  
                  <p className="text-text-secondary mb-6">
                    {section.description}
                  </p>

                  <ul className="space-y-3">
                    {section.items.map((item) => (
                      <li key={item}>
                        <a 
                          href="#" 
                          className="text-text-secondary hover:text-accent-blue transition-colors border-b border-transparent hover:border-accent-blue/30"
                        >
                          {item}
                        </a>
                      </li>
                    ))}
                  </ul>
                </div>
              );
            })}
          </div>

          <div className="mt-16 text-center">
            <div className="bg-bg-secondary border border-border rounded-lg p-8 max-w-2xl mx-auto">
              <h2 className="text-2xl font-bold text-text-primary mb-4">
                Need Help?
              </h2>
              <p className="text-text-secondary mb-6">
                Can't find what you're looking for? Our support team is here to help.
              </p>
              <div className="flex justify-center gap-4">
                <Link 
                  to="/about" 
                  className="btn-secondary"
                >
                  Contact Support
                </Link>
                <Link 
                  to="/app" 
                  className="btn-primary"
                >
                  Try Graph Now
                </Link>
              </div>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}