describe('Visual Regression - Chart Presets', () => {
  beforeEach(() => {
    // Visit the app with BTC-USD data
    cy.visit('/app?topic=BTC-USD');
    
    // Wait for chart to be ready
    cy.waitForChartRender();
  });

  it('should render default chart view', () => {
    // Verify chart is visible
    cy.get('canvas#webgpu-canvas').should('be.visible');
    
    // Take baseline screenshot
    cy.screenshot('default-chart-view', { capture: 'viewport' });
    
    // Also capture just the canvas
    cy.get('canvas#webgpu-canvas').screenshot('default-canvas-only');
  });

  it('should render Market Data preset', () => {
    // Select Market Data preset
    cy.selectPreset('Market Data');
    
    // Wait for rendering
    cy.wait(5000);
    
    // Take screenshots
    cy.screenshot('market-data-preset', { capture: 'viewport' });
    cy.get('canvas#webgpu-canvas').screenshot('market-data-canvas');
  });

  it('should render Candlestick preset', () => {
    // Select Candlestick preset
    cy.selectPreset('Candlestick');
    
    // Wait for rendering
    cy.wait(5000);
    
    // Take screenshots
    cy.screenshot('candlestick-preset', { capture: 'viewport' });
    cy.get('canvas#webgpu-canvas').screenshot('candlestick-canvas');
  });

  it('should handle preset switching', () => {
    // Start with Market Data
    cy.selectPreset('Market Data');
    cy.wait(3000);
    cy.screenshot('preset-market-data-initial', { capture: 'viewport' });
    
    // Switch to Candlestick
    cy.selectPreset('Candlestick');
    cy.wait(3000);
    cy.screenshot('preset-switched-to-candlestick', { capture: 'viewport' });
    
    // Switch back to Market Data
    cy.selectPreset('Market Data');
    cy.wait(3000);
    cy.screenshot('preset-switched-back-to-market-data', { capture: 'viewport' });
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
          cy.selectPreset(preset);
          cy.wait(5000);
          
          // Take screenshot for each preset
          const snapshotName = `preset-${preset.toLowerCase().replace(/\s+/g, '-')}`;
          cy.screenshot(snapshotName, { capture: 'viewport' });
          cy.get('canvas#webgpu-canvas').screenshot(`${snapshotName}-canvas`);
        }
      });
    });
  });
});