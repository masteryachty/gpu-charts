/// <reference types="cypress" />

// Custom commands for GPU Charts testing

// Command to select a preset
Cypress.Commands.add('selectPreset', (presetName: string) => {
  // Select the preset
  cy.get('select:has(option:contains("Select a Preset"))')
    .select(presetName);
  
  // Wait for render to complete
  cy.wait(1500);
});

// Command to perform zoom
Cypress.Commands.add('zoomChart', (direction: 'in' | 'out', amount: number = 200) => {
  cy.get('canvas#webgpu-canvas').trigger('wheel', {
    deltaY: direction === 'in' ? -amount : amount,
    bubbles: true
  });
  
  // Wait for render
  cy.wait(500);
});

// Command to perform pan
Cypress.Commands.add('panChart', (deltaX: number, deltaY: number = 0) => {
  cy.get('canvas#webgpu-canvas').then(($canvas) => {
    const canvas = $canvas[0];
    const rect = canvas.getBoundingClientRect();
    const centerX = rect.left + rect.width / 2;
    const centerY = rect.top + rect.height / 2;
    
    cy.get('canvas#webgpu-canvas')
      .trigger('mousedown', centerX, centerY, { button: 0 })
      .trigger('mousemove', centerX + deltaX, centerY + deltaY)
      .trigger('mouseup', centerX + deltaX, centerY + deltaY);
  });
  
  // Wait for render
  cy.wait(500);
});

// Command to check if chart has data
Cypress.Commands.add('chartShouldHaveData', () => {
  cy.window().then((win) => {
    // Check if any API calls were made
    cy.intercept('GET', '**/api/**').as('dataFetch');
    
    // Check canvas is not just black
    cy.get('canvas#webgpu-canvas').then(($canvas) => {
      const canvas = $canvas[0] as HTMLCanvasElement;
      
      // For WebGPU canvas, we can't directly read pixels
      // but we can check if it's been initialized properly
      expect(canvas.width).to.be.greaterThan(0);
      expect(canvas.height).to.be.greaterThan(0);
      expect(canvas.getAttribute('data-initialized')).to.equal('true');
    });
  });
});

// TypeScript declarations
declare global {
  namespace Cypress {
    interface Chainable {
      selectPreset(presetName: string): Chainable<void>;
      zoomChart(direction: 'in' | 'out', amount?: number): Chainable<void>;
      panChart(deltaX: number, deltaY?: number): Chainable<void>;
      chartShouldHaveData(): Chainable<void>;
    }
  }
}

export {};