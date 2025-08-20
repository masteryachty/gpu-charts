import { createTestUrl } from '../support/test-constants';

describe.skip('Visual Regression - Chart Interactions', () => {
  beforeEach(() => {
    cy.visit(createTestUrl('BTC-USD'));
    cy.waitForChartRender();
  });

  it('should handle zoom interactions', () => {
    // Initial state
    cy.compareSnapshot('interaction-initial-state');
    
    // Zoom in
    cy.zoomChart('in', 300);
    cy.compareSnapshot('interaction-zoomed-in');
    
    // Zoom out
    cy.zoomChart('out', 600);
    cy.compareSnapshot('interaction-zoomed-out');
    
    // Return to normal
    cy.zoomChart('in', 300);
    cy.compareSnapshot('interaction-zoom-reset');
  });

  it('should handle pan interactions', () => {
    // Initial state
    cy.get('canvas#webgpu-canvas').compareSnapshot('pan-initial');
    
    // Pan right
    cy.panChart(200, 0);
    cy.get('canvas#webgpu-canvas').compareSnapshot('pan-right');
    
    // Pan left
    cy.panChart(-400, 0);
    cy.get('canvas#webgpu-canvas').compareSnapshot('pan-left');
    
    // Pan back to center
    cy.panChart(200, 0);
    cy.get('canvas#webgpu-canvas').compareSnapshot('pan-center');
  });

  it('should handle combined interactions', () => {
    // Zoom and pan
    cy.zoomChart('in', 200);
    cy.panChart(100, 0);
    cy.compareSnapshot('interaction-zoom-and-pan');
    
    // More zoom
    cy.zoomChart('in', 200);
    cy.compareSnapshot('interaction-deep-zoom');
    
    // Pan while zoomed
    cy.panChart(-200, 0);
    cy.compareSnapshot('interaction-deep-zoom-panned');
  });

  it('should maintain chart state during interactions', () => {
    // Select a preset first
    cy.selectPreset('Market Data');
    cy.wait(3000);
    
    // Initial preset state
    cy.compareSnapshot('market-data-before-interaction');
    
    // Interact with the chart
    cy.zoomChart('in', 250);
    cy.panChart(150, 0);
    
    // Chart should still show Market Data preset
    cy.compareSnapshot('market-data-after-interaction');
  });

  it('should handle rapid interactions', () => {
    // Perform multiple rapid interactions
    cy.zoomChart('in', 100);
    cy.zoomChart('in', 100);
    cy.zoomChart('in', 100);
    cy.wait(1000);
    cy.compareSnapshot('rapid-zoom-in');
    
    cy.panChart(50, 0);
    cy.panChart(50, 0);
    cy.panChart(50, 0);
    cy.wait(1000);
    cy.compareSnapshot('rapid-pan');
    
    // Rapid zoom out
    cy.zoomChart('out', 100);
    cy.zoomChart('out', 100);
    cy.zoomChart('out', 100);
    cy.wait(1000);
    cy.compareSnapshot('rapid-zoom-out');
  });
});