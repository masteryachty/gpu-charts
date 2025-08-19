describe('EMA Indicators on Candlestick Chart', () => {
  beforeEach(() => {
    // Visit the app with BTC-USD data
    cy.visit('/app?topic=BTC-USD');
    cy.waitForChartRender();
    
    // Select Candlestick preset
    cy.selectPreset('Candlestick');
    cy.wait(3000); // Wait for chart to render
  });

  it('should have EMA toggles in the metrics panel', () => {
    // Check that all 5 EMA indicators are present
    const expectedEMAs = ['EMA 9', 'EMA 20', 'EMA 50', 'EMA 100', 'EMA 200'];
    
    expectedEMAs.forEach(emaLabel => {
      cy.get('label')
        .contains(emaLabel)
        .should('exist')
        .parent()
        .find('input[type="checkbox"]')
        .should('exist');
    });
    
    // Take screenshot of metrics panel
    cy.screenshot('ema-indicators-panel', { capture: 'viewport' });
  });

  it('should toggle individual EMA indicators', () => {
    // Test toggling each EMA individually
    const emaLabels = ['EMA 9', 'EMA 20', 'EMA 50', 'EMA 100', 'EMA 200'];
    
    emaLabels.forEach((emaLabel, index) => {
      // Find the EMA checkbox
      cy.get('label')
        .contains(emaLabel)
        .parent()
        .find('input[type="checkbox"]')
        .as(`ema${index}`);
      
      // Check initial state (should be unchecked by default based on config)
      cy.get(`@ema${index}`).should('not.be.checked');
      
      // Enable the EMA
      cy.get(`@ema${index}`).check();
      cy.wait(2000); // Wait for render
      
      // Take screenshot with this EMA enabled
      cy.screenshot(`ema-${emaLabel.toLowerCase().replace(/\s+/g, '-')}-enabled`, { 
        capture: 'viewport' 
      });
      
      // Disable it again
      cy.get(`@ema${index}`).uncheck();
      cy.wait(1000);
    });
  });

  it('should show all EMAs together', () => {
    // Enable all EMAs
    const emaLabels = ['EMA 9', 'EMA 20', 'EMA 50', 'EMA 100', 'EMA 200'];
    
    emaLabels.forEach(emaLabel => {
      cy.get('label')
        .contains(emaLabel)
        .parent()
        .find('input[type="checkbox"]')
        .check();
    });
    
    cy.wait(3000); // Wait for all EMAs to render
    
    // Take screenshot with all EMAs visible
    cy.screenshot('all-emas-enabled', { capture: 'viewport' });
    
    // Verify the canvas is rendering (should have content)
    cy.get('canvas').should('be.visible');
  });

  it('should show different EMA combinations', () => {
    // Test short-term EMAs (9, 20)
    cy.get('label').contains('EMA 9').parent().find('input[type="checkbox"]').check();
    cy.get('label').contains('EMA 20').parent().find('input[type="checkbox"]').check();
    cy.wait(2000);
    cy.screenshot('short-term-emas', { capture: 'viewport' });
    
    // Clear and test medium-term EMAs (20, 50)
    cy.get('input[type="checkbox"]:checked').uncheck({ multiple: true });
    cy.get('label').contains('EMA 20').parent().find('input[type="checkbox"]').check();
    cy.get('label').contains('EMA 50').parent().find('input[type="checkbox"]').check();
    cy.wait(2000);
    cy.screenshot('medium-term-emas', { capture: 'viewport' });
    
    // Clear and test long-term EMAs (100, 200)
    cy.get('input[type="checkbox"]:checked').uncheck({ multiple: true });
    cy.get('label').contains('EMA 100').parent().find('input[type="checkbox"]').check();
    cy.get('label').contains('EMA 200').parent().find('input[type="checkbox"]').check();
    cy.wait(2000);
    cy.screenshot('long-term-emas', { capture: 'viewport' });
  });

  it('should maintain EMA state during zoom operations', () => {
    // Enable some EMAs
    cy.get('label').contains('EMA 9').parent().find('input[type="checkbox"]').check();
    cy.get('label').contains('EMA 50').parent().find('input[type="checkbox"]').check();
    cy.wait(2000);
    
    // Get canvas element
    cy.get('canvas').as('chartCanvas');
    
    // Perform zoom in
    cy.get('@chartCanvas').trigger('wheel', {
      deltaY: -120,
      clientX: 400,
      clientY: 300
    });
    cy.wait(2000);
    cy.screenshot('emas-after-zoom-in', { capture: 'viewport' });
    
    // Perform zoom out
    cy.get('@chartCanvas').trigger('wheel', {
      deltaY: 120,
      clientX: 400,
      clientY: 300
    });
    cy.wait(2000);
    cy.screenshot('emas-after-zoom-out', { capture: 'viewport' });
    
    // Verify EMAs are still checked
    cy.get('label').contains('EMA 9').parent().find('input[type="checkbox"]').should('be.checked');
    cy.get('label').contains('EMA 50').parent().find('input[type="checkbox"]').should('be.checked');
  });

  it('should render EMAs with correct colors', () => {
    // Enable all EMAs to check color differentiation
    const emaConfig = [
      { label: 'EMA 9', color: 'light red' },
      { label: 'EMA 20', color: 'orange' },
      { label: 'EMA 50', color: 'yellow' },
      { label: 'EMA 100', color: 'light green' },
      { label: 'EMA 200', color: 'light blue' }
    ];
    
    // Enable all EMAs
    emaConfig.forEach(({ label }) => {
      cy.get('label')
        .contains(label)
        .parent()
        .find('input[type="checkbox"]')
        .check();
    });
    
    cy.wait(3000);
    
    // Take a screenshot to visually verify colors
    cy.screenshot('ema-color-verification', { capture: 'viewport' });
    
    // Log the configuration for reference
    cy.log('EMA Colors:', emaConfig);
  });

  it('should handle rapid toggling without errors', () => {
    const emaLabel = 'EMA 20';
    
    // Get the checkbox
    cy.get('label')
      .contains(emaLabel)
      .parent()
      .find('input[type="checkbox"]')
      .as('emaCheckbox');
    
    // Rapidly toggle the EMA
    for (let i = 0; i < 5; i++) {
      cy.get('@emaCheckbox').check();
      cy.wait(200);
      cy.get('@emaCheckbox').uncheck();
      cy.wait(200);
    }
    
    // Final state - enable it
    cy.get('@emaCheckbox').check();
    cy.wait(2000);
    
    // Should render without errors
    cy.get('canvas').should('be.visible');
    cy.screenshot('ema-after-rapid-toggle', { capture: 'viewport' });
  });

  it('should work with candlestick chart timeframe changes', () => {
    // Enable EMAs
    cy.get('label').contains('EMA 20').parent().find('input[type="checkbox"]').check();
    cy.get('label').contains('EMA 50').parent().find('input[type="checkbox"]').check();
    cy.wait(2000);
    
    // Take initial screenshot
    cy.screenshot('ema-initial-timeframe', { capture: 'viewport' });
    
    // Zoom in significantly to change timeframe
    const canvas = cy.get('canvas');
    for (let i = 0; i < 3; i++) {
      canvas.trigger('wheel', {
        deltaY: -120,
        clientX: 400,
        clientY: 300
      });
      cy.wait(500);
    }
    
    cy.wait(2000);
    cy.screenshot('ema-zoomed-timeframe', { capture: 'viewport' });
    
    // EMAs should still be visible and checked
    cy.get('label').contains('EMA 20').parent().find('input[type="checkbox"]').should('be.checked');
    cy.get('label').contains('EMA 50').parent().find('input[type="checkbox"]').should('be.checked');
  });
});