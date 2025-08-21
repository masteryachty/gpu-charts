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
    bubbles: true,
    force: true
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
      .trigger('mousedown', centerX, centerY, { button: 0, force: true })
      .trigger('mousemove', centerX + deltaX, centerY + deltaY, { force: true })
      .trigger('mouseup', centerX + deltaX, centerY + deltaY, { force: true });
  });
  
  // Wait for render
  cy.wait(500);
});

// Command to wait for chart to render
Cypress.Commands.add('waitForChartRender', () => {
  // Wait for canvas to be visible
  cy.get('canvas').should('be.visible');
  
  // Wait for initial render to complete
  cy.wait(2000);
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

// Command to activate tooltip
Cypress.Commands.add('activateTooltip', (x?: number, y?: number) => {
  cy.get('canvas#webgpu-canvas').then(($canvas) => {
    const canvas = $canvas[0];
    const rect = canvas.getBoundingClientRect();
    const targetX = x !== undefined ? x : rect.left + rect.width / 2;
    const targetY = y !== undefined ? y : rect.top + rect.height / 2;
    
    cy.get('canvas#webgpu-canvas')
      .trigger('mousedown', targetX, targetY, { button: 2, force: true });
  });
  
  // Wait for tooltip to render
  cy.wait(100);
});

// Command to deactivate tooltip
Cypress.Commands.add('deactivateTooltip', (x?: number, y?: number) => {
  cy.get('canvas#webgpu-canvas').then(($canvas) => {
    const canvas = $canvas[0];
    const rect = canvas.getBoundingClientRect();
    const targetX = x !== undefined ? x : rect.left + rect.width / 2;
    const targetY = y !== undefined ? y : rect.top + rect.height / 2;
    
    cy.get('canvas#webgpu-canvas')
      .trigger('mouseup', targetX, targetY, { button: 2, force: true });
  });
  
  // Wait for tooltip to disappear
  cy.wait(100);
});

// Command to move tooltip
Cypress.Commands.add('moveTooltip', (x: number, y: number) => {
  cy.get('canvas#webgpu-canvas')
    .trigger('mousemove', x, y, { force: true });
  
  // Wait for tooltip to update
  cy.wait(50);
});

// Command to test tooltip at specific positions
Cypress.Commands.add('testTooltipAtPosition', (position: 'left' | 'center' | 'right' | 'quarter' | 'three-quarter', snapshotName: string) => {
  cy.get('canvas#webgpu-canvas').then(($canvas) => {
    const canvas = $canvas[0];
    const rect = canvas.getBoundingClientRect();
    const centerY = rect.top + rect.height / 2;
    
    let targetX: number;
    switch (position) {
      case 'left':
        targetX = rect.left + rect.width * 0.1;
        break;
      case 'quarter':
        targetX = rect.left + rect.width * 0.25;
        break;
      case 'center':
        targetX = rect.left + rect.width * 0.5;
        break;
      case 'three-quarter':
        targetX = rect.left + rect.width * 0.75;
        break;
      case 'right':
        targetX = rect.left + rect.width * 0.9;
        break;
    }
    
    cy.activateTooltip(targetX, centerY);
    cy.get('canvas#webgpu-canvas').compareSnapshot(snapshotName);
    cy.deactivateTooltip(targetX, centerY);
  });
});

// Command to check tooltip performance
Cypress.Commands.add('checkTooltipPerformance', (testName: string) => {
  let frameCount = 0;
  let startTime = 0;
  let rafId: number;

  cy.window().then((win) => {
    const measureFPS = () => {
      if (startTime === 0) {
        startTime = performance.now();
      }
      frameCount++;
      rafId = win.requestAnimationFrame(measureFPS);
    };

    // Start measuring FPS
    measureFPS();

    cy.get('canvas#webgpu-canvas').then(($canvas) => {
      const canvas = $canvas[0];
      const rect = canvas.getBoundingClientRect();
      const startX = rect.left + rect.width * 0.1;
      const endX = rect.left + rect.width * 0.9;
      const centerY = rect.top + rect.height / 2;

      // Activate tooltip and perform movements
      cy.activateTooltip(startX, centerY);

      // Perform smooth movement
      const steps = 30;
      for (let i = 0; i <= steps; i++) {
        const x = startX + ((endX - startX) * i) / steps;
        cy.moveTooltip(x, centerY);
      }

      cy.wait(1000).then(() => {
        // Stop measuring and check FPS
        win.cancelAnimationFrame(rafId);
        const endTime = performance.now();
        const duration = (endTime - startTime) / 1000;
        const fps = frameCount / duration;

        // Log performance results
        cy.log(`${testName} FPS: ${fps.toFixed(2)}`);
        
        // FPS should be acceptable (at least 45 FPS for smooth interaction)
        expect(fps).to.be.greaterThan(45, `${testName} should maintain at least 45 FPS`);
      });

      cy.deactivateTooltip(endX, centerY);
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
      waitForChartRender(): Chainable<void>;
      compareSnapshot(name: string, options?: any): Chainable<void>;
      activateTooltip(x?: number, y?: number): Chainable<void>;
      deactivateTooltip(x?: number, y?: number): Chainable<void>;
      moveTooltip(x: number, y: number): Chainable<void>;
      testTooltipAtPosition(position: 'left' | 'center' | 'right' | 'quarter' | 'three-quarter', snapshotName: string): Chainable<void>;
      checkTooltipPerformance(testName: string): Chainable<void>;
    }
  }
}

export {};