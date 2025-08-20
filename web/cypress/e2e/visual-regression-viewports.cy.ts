import { createTestUrl } from '../support/test-constants';

describe('Visual Regression - Different Viewports', () => {
  const viewports = [
    { name: 'desktop-4k', width: 3840, height: 2160 },
    { name: 'desktop-full-hd', width: 1920, height: 1080 },
    { name: 'laptop', width: 1366, height: 768 },
    { name: 'tablet-landscape', width: 1024, height: 768 },
    { name: 'tablet-portrait', width: 768, height: 1024 },
    { name: 'mobile-landscape', width: 667, height: 375 },
  ];

  viewports.forEach((viewport) => {
    it(`should render correctly at ${viewport.name} (${viewport.width}x${viewport.height})`, () => {
      // Set viewport
      cy.viewport(viewport.width, viewport.height);

      // Visit the app
      cy.visit(createTestUrl('BTC-USD'));
      cy.waitForChartRender();

      // Compare full page screenshot
      cy.compareSnapshot(`viewport-${viewport.name}-full`);

      // Compare canvas only screenshot
      cy.get('canvas#webgpu-canvas').compareSnapshot(`viewport-${viewport.name}-canvas`);
    });
  });

  it('should handle viewport resize', () => {
    // Start with desktop
    cy.viewport(1920, 1080);
    cy.visit(createTestUrl('BTC-USD'));
    cy.waitForChartRender();
    cy.compareSnapshot('resize-desktop-initial');

    // Resize to laptop
    cy.viewport(1366, 768);
    cy.wait(2000);
    cy.compareSnapshot('resize-to-laptop');

    // Resize to tablet
    cy.viewport(1024, 768);
    cy.wait(2000);
    cy.compareSnapshot('resize-to-tablet');

    // Back to desktop
    cy.viewport(1920, 1080);
    cy.wait(2000);
    cy.compareSnapshot('resize-back-to-desktop');
  });

  it('should maintain preset across viewport changes', () => {
    // Start with desktop and set preset
    cy.viewport(1920, 1080);
    cy.visit(createTestUrl('BTC-USD'));
    cy.waitForChartRender();
    cy.selectPreset('Candlestick');
    cy.wait(3000);
    cy.compareSnapshot('candlestick-desktop');

    // Change to laptop
    cy.viewport(1366, 768);
    cy.wait(2000);
    cy.compareSnapshot('candlestick-laptop');

    // Change to tablet
    cy.viewport(1024, 768);
    cy.wait(2000);
    cy.compareSnapshot('candlestick-tablet');
  });
});