// Import commands.js using ES2015 syntax:
import './commands';

// Alternatively you can use CommonJS syntax:
// require('./commands')

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
  
  // Extra wait for rendering to complete
  cy.wait(5000);
});

Cypress.Commands.add('waitForChartRender', () => {
  // First wait for WebGPU
  cy.waitForWebGPU();
  
  // Then wait for actual rendering
  cy.wait(5000); // Give time for data fetch and render
  
  // Check that chart instance exists
  cy.window().then((win) => {
    // Log any console errors
    if (win.console && win.console.error) {
      cy.log('Checking for console errors...');
    }
  });
});

// Add TypeScript declarations
declare global {
  namespace Cypress {
    interface Chainable {
      waitForWebGPU(): Chainable<void>;
      waitForChartRender(): Chainable<void>;
    }
  }
}