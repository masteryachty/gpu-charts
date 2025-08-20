import { createTestUrl } from '../support/test-constants';

describe('Visual Regression - Chart Presets', () => {
  beforeEach(() => {
    // Visit the app with BTC-USD data and fixed time range
    cy.visit(createTestUrl('BTC-USD'));
    
    // Wait for chart to be ready
    cy.waitForChartRender();
  });

  it('should render default chart view', () => {
    // Verify chart is visible
    cy.get('canvas#webgpu-canvas').should('be.visible');
    
    // Compare with baseline screenshot
    cy.compareSnapshot('default-chart-view');
    
    // Also capture just the canvas
    cy.get('canvas#webgpu-canvas').compareSnapshot('default-canvas-only');
  });

  it('should render Market Data preset', () => {
    // Select Market Data preset
    cy.selectPreset('Market Data');
    
    // Wait for rendering
    cy.wait(5000);
    
    // Compare screenshots
    cy.compareSnapshot('market-data-preset');
    cy.get('canvas#webgpu-canvas').compareSnapshot('market-data-canvas');
  });

  it('should render Candlestick preset', () => {
    // Select Candlestick preset
    cy.selectPreset('Candlestick');
    
    // Wait for rendering
    cy.wait(5000);
    
    // Compare screenshots
    cy.compareSnapshot('candlestick-preset');
    cy.get('canvas#webgpu-canvas').compareSnapshot('candlestick-canvas');
  });

  it('should handle preset switching', () => {
    // Start with Market Data
    cy.selectPreset('Market Data');
    cy.wait(3000);
    cy.compareSnapshot('preset-market-data-initial');
    
    // Switch to Candlestick
    cy.selectPreset('Candlestick');
    cy.wait(3000);
    cy.compareSnapshot('preset-switched-to-candlestick');
    
    // Switch back to Market Data
    cy.selectPreset('Market Data');
    cy.wait(3000);
    cy.compareSnapshot('preset-switched-back-to-market-data');
  });

  it('should show all available presets', () => {
    // Get all preset options
    cy.get('select:has(option:contains("Select a Preset")) option').then(($options) => {
      const presets = Array.from($options)
        .map(opt => opt.textContent)
        .filter(text => text && text !== 'Select a Preset');
      
      cy.log('Found presets:', presets.join(', '));
      
      // Test each preset
      presets.forEach((preset) => {
        if (preset) {
          // Select preset (includes dynamic waiting)
          cy.selectPreset(preset);
          
          // Compare screenshot for each preset
          const snapshotName = `preset-${preset.toLowerCase().replace(/\s+/g, '-')}`;
          cy.compareSnapshot(snapshotName);
          cy.get('canvas#webgpu-canvas').compareSnapshot(`${snapshotName}-canvas`);
        }
      });
    });
  });
});