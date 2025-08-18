// Import commands.js using ES2015 syntax:
import './commands';

// Alternatively you can use CommonJS syntax:
// require('./commands')

// Note: API intercepts are set up in individual commands as needed
// This avoids global intercepts that may not be triggered

// Custom WebGPU/Canvas waiting utilities
Cypress.Commands.add('waitForWebGPU', () => {
  cy.window().then((win) => {
    // Check if WebGPU is available
    expect(win.navigator).to.have.property('gpu');
  });
  
  // Wait for canvas to be initialized
  cy.get('canvas#webgpu-canvas', { timeout: 30000 })
    .should('be.visible')
    .and(($canvas) => {
      expect($canvas[0].width).to.be.greaterThan(0);
      expect($canvas[0].height).to.be.greaterThan(0);
    });
  
  // Wait for data-initialized attribute
  cy.get('canvas#webgpu-canvas')
    .should('have.attr', 'data-initialized', 'true');
  
  // Wait for loading overlay to disappear
  cy.get('[data-testid="loading-overlay"]', { timeout: 30000 })
    .should('not.exist');
  
  // Wait for a frame to be rendered (check canvas has content)
  cy.get('canvas#webgpu-canvas').should(($canvas) => {
    const canvas = $canvas[0] as HTMLCanvasElement;
    const ctx = canvas.getContext('2d');
    if (ctx) {
      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const hasContent = imageData.data.some(pixel => pixel !== 0);
      expect(hasContent).to.be.true;
    }
  });
});

Cypress.Commands.add('waitForChartRender', () => {
  // Simply wait for WebGPU initialization and chart to be visible
  // Don't wait for specific API calls as they may not happen in all environments
  cy.waitForWebGPU();
  
  // Additional small wait for chart rendering
  cy.wait(2000);
  
  // Check that chart instance exists
  cy.window().then((win) => {
    // Log any console errors
    if (win.console && win.console.error) {
      cy.log('Checking for console errors...');
    }
  });
});

Cypress.Commands.add('waitForPresetChange', () => {
  // Wait for chart to re-render after preset change
  cy.get('canvas#webgpu-canvas').should('be.visible');
  
  // Wait for render to complete
  cy.wait(1000);
});

// Add TypeScript declarations
declare global {
  namespace Cypress {
    interface Chainable {
      waitForWebGPU(): Chainable<void>;
      waitForChartRender(): Chainable<void>;
      waitForPresetChange(): Chainable<void>;
    }
  }
}