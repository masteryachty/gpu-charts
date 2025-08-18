describe.skip('Visual Regression - Different Viewports', () => {
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
      cy.visit('/app?topic=BTC-USD');
      cy.waitForChartRender();

      // Take full page screenshot
      cy.screenshot(`viewport-${viewport.name}-full`, { capture: 'viewport' });

      // Take canvas only screenshot
      cy.get('canvas#webgpu-canvas').screenshot(`viewport-${viewport.name}-canvas`);
    });
  });

  it('should handle viewport resize', () => {
    // Start with desktop
    cy.viewport(1920, 1080);
    cy.visit('/app?topic=BTC-USD');
    cy.waitForChartRender();
    cy.screenshot('resize-desktop-initial', { capture: 'viewport' });

    // Resize to laptop
    cy.viewport(1366, 768);
    cy.wait(2000);
    cy.screenshot('resize-to-laptop', { capture: 'viewport' });

    // Resize to tablet
    cy.viewport(1024, 768);
    cy.wait(2000);
    cy.screenshot('resize-to-tablet', { capture: 'viewport' });

    // Back to desktop
    cy.viewport(1920, 1080);
    cy.wait(2000);
    cy.screenshot('resize-back-to-desktop', { capture: 'viewport' });
  });

  it('should maintain preset across viewport changes', () => {
    // Start with desktop and set preset
    cy.viewport(1920, 1080);
    cy.visit('/app?topic=BTC-USD');
    cy.waitForChartRender();
    cy.selectPreset('Candlestick');
    cy.wait(3000);
    cy.screenshot('candlestick-desktop', { capture: 'viewport' });

    // Change to laptop
    cy.viewport(1366, 768);
    cy.wait(2000);
    cy.screenshot('candlestick-laptop', { capture: 'viewport' });

    // Change to tablet
    cy.viewport(1024, 768);
    cy.wait(2000);
    cy.screenshot('candlestick-tablet', { capture: 'viewport' });
  });
});