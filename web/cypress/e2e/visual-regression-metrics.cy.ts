import { createTestUrl } from '../support/test-constants';

describe.skip('Visual Regression - Metric Toggles', () => {
  beforeEach(() => {
    cy.visit(createTestUrl('BTC-USD'));
    cy.waitForChartRender();
  });

  it('should toggle Market Data metrics', () => {
    // Select Market Data preset
    cy.selectPreset('Market Data');
    cy.wait(3000);
    
    // Initial state with all metrics
    cy.compareSnapshot('market-data-all-metrics');
    
    // Find and toggle checkboxes
    cy.get('input[type="checkbox"]').then(($checkboxes) => {
      if ($checkboxes.length > 0) {
        // Uncheck first metric
        cy.wrap($checkboxes[0]).uncheck();
        cy.wait(2000);
        cy.compareSnapshot('market-data-first-metric-off');
        
        // Uncheck second metric if exists
        if ($checkboxes.length > 1) {
          cy.wrap($checkboxes[1]).uncheck();
          cy.wait(2000);
          cy.compareSnapshot('market-data-two-metrics-off');
        }
        
        // Re-check all
        $checkboxes.each((index, checkbox) => {
          cy.wrap(checkbox).check();
        });
        cy.wait(2000);
        cy.compareSnapshot('market-data-all-metrics-restored');
      }
    });
  });

  it('should toggle individual metrics', () => {
    cy.selectPreset('Market Data');
    cy.wait(3000);
    
    // Get all checkbox labels
    cy.get('label:has(input[type="checkbox"])').each(($label, index) => {
      const labelText = $label.text().trim();
      
      // Toggle off
      cy.wrap($label).find('input[type="checkbox"]').uncheck();
      cy.wait(2000);
      cy.compareSnapshot(`metric-${labelText.toLowerCase().replace(/\s+/g, '-')}-off`);
      
      // Toggle back on
      cy.wrap($label).find('input[type="checkbox"]').check();
      cy.wait(2000);
    });
  });

  it('should show only bid/ask metrics', () => {
    cy.selectPreset('Market Data');
    cy.wait(3000);
    
    // Uncheck all non bid/ask metrics
    cy.get('label:has(input[type="checkbox"])').each(($label) => {
      const labelText = $label.text().toLowerCase();
      if (!labelText.includes('bid') && !labelText.includes('ask')) {
        cy.wrap($label).find('input[type="checkbox"]').uncheck();
      }
    });
    
    cy.wait(3000);
    cy.compareSnapshot('market-data-bid-ask-only');
  });

  it('should show chart with no metrics', () => {
    cy.selectPreset('Market Data');
    cy.wait(3000);
    
    // Uncheck all metrics
    cy.get('input[type="checkbox"]').uncheck({ multiple: true });
    cy.wait(3000);
    cy.compareSnapshot('market-data-no-metrics');
  });

  it('should handle Candlestick preset metrics', () => {
    cy.selectPreset('Candlestick');
    cy.wait(3000);
    
    // Initial state
    cy.compareSnapshot('candlestick-initial-metrics');
    
    // Check if there are any toggles
    cy.get('input[type="checkbox"]').then(($checkboxes) => {
      if ($checkboxes.length > 0) {
        // Toggle metrics
        $checkboxes.each((index, checkbox) => {
          cy.wrap(checkbox).uncheck();
          cy.wait(2000);
          cy.compareSnapshot(`candlestick-metric-${index}-off`);
          cy.wrap(checkbox).check();
        });
      } else {
        cy.log('No metric toggles available for Candlestick preset');
      }
    });
  });
});