import { Link } from 'react-router-dom';
import { Check, ArrowLeft } from 'lucide-react';

export default function PricingPage() {
  const plans = [
    {
      name: 'Free',
      price: '$0',
      period: 'forever',
      description: 'Perfect for getting started',
      features: [
        'Real-time market data',
        'Basic chart types',
        '5 saved layouts',
        'Email support',
        'WebGPU acceleration'
      ],
      cta: 'Get Started Free',
      popular: false
    },
    {
      name: 'Pro',
      price: '$29',
      period: 'per month',
      description: 'For active traders',
      features: [
        'All Free features',
        'Advanced indicators',
        'Unlimited layouts',
        'Real-time alerts',
        'Priority support',
        'Historical data export',
        'Multi-exchange view'
      ],
      cta: 'Start Free Trial',
      popular: true
    },
    {
      name: 'Enterprise',
      price: '$99',
      period: 'per month',
      description: 'For professional teams',
      features: [
        'All Pro features',
        'Team collaboration',
        'Custom indicators',
        'API access',
        '24/7 phone support',
        'Advanced analytics',
        'White-label options'
      ],
      cta: 'Contact Sales',
      popular: false
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

      {/* Pricing Section */}
      <section className="px-8 py-24">
        <div className="max-w-6xl mx-auto">
          <div className="text-center mb-16">
            <h1 className="text-4xl font-bold text-text-primary mb-4">
              Simple, Transparent Pricing
            </h1>
            <p className="text-xl text-text-secondary">
              Choose the plan that fits your trading needs
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
            {plans.map((plan) => (
              <div 
                key={plan.name}
                className={`card p-8 relative ${
                  plan.popular 
                    ? 'border-accent-blue ring-2 ring-accent-blue/20' 
                    : ''
                }`}
              >
                {plan.popular && (
                  <div className="absolute -top-4 left-1/2 transform -translate-x-1/2">
                    <span className="bg-accent-blue text-white px-4 py-1 text-sm font-medium rounded-full">
                      Most Popular
                    </span>
                  </div>
                )}

                <div className="text-center mb-8">
                  <h3 className="text-2xl font-bold text-text-primary mb-2">
                    {plan.name}
                  </h3>
                  <div className="mb-4">
                    <span className="text-4xl font-bold text-text-primary">
                      {plan.price}
                    </span>
                    <span className="text-text-secondary ml-2">
                      {plan.period}
                    </span>
                  </div>
                  <p className="text-text-secondary">
                    {plan.description}
                  </p>
                </div>

                <div className="space-y-4 mb-8">
                  {plan.features.map((feature) => (
                    <div key={feature} className="flex items-center gap-3">
                      <Check className="text-accent-green flex-shrink-0" size={20} />
                      <span className="text-text-secondary">{feature}</span>
                    </div>
                  ))}
                </div>

                <button className={`w-full py-3 px-4 rounded-lg font-medium transition-colors ${
                  plan.popular
                    ? 'bg-accent-blue text-white hover:bg-accent-blue/90'
                    : 'bg-bg-secondary border border-border text-text-primary hover:bg-bg-tertiary'
                }`}>
                  {plan.cta}
                </button>
              </div>
            ))}
          </div>

          <div className="text-center mt-16">
            <p className="text-text-secondary mb-4">
              All plans include 14-day free trial • No credit card required
            </p>
            <div className="text-sm text-text-tertiary">
              Questions? <Link to="/about" className="text-accent-blue hover:underline">Contact us</Link>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}