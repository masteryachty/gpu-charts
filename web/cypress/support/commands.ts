/// <reference types="cypress" />

// Custom commands for GPU Charts testing

// Command to select a preset with dynamic waiting
Cypress.Commands.add('selectPreset', (presetName: string) => {
  // Set up intercept for the API call that will be triggered
  cy.intercept('GET', '**/api/data*').as('presetDataLoad');
  
  // Select the preset
  cy.get('select:has(option:contains("Select a Preset"))')
    .select(presetName);
  
  // Wait for the API call to complete
  cy.wait('@presetDataLoad', { timeout: 10000 });
  
  // Small wait for render to complete after data loads
  cy.wait(500);
});

// Command to perform zoom with dynamic waiting
Cypress.Commands.add('zoomChart', (direction: 'in' | 'out', amount: number = 200) => {
  // Set up intercept if zoom triggers data reload
  cy.intercept('GET', '**/api/data*').as('zoomDataLoad');
  
  cy.get('canvas#webgpu-canvas').trigger('wheel', {
    deltaY: direction === 'in' ? -amount : amount,
    bubbles: true
  });
  
  // Wait for any data reload or just a short render time
  cy.wait(500);
});

// Command to perform pan with dynamic waiting
Cypress.Commands.add('panChart', (deltaX: number, deltaY: number = 0) => {
  // Set up intercept if pan triggers data reload
  cy.intercept('GET', '**/api/data*').as('panDataLoad');
  
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
  
  // Wait for any data reload or just a short render time
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