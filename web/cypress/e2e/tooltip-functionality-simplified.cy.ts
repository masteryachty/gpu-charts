import { createTestUrl } from '../support/test-constants';

describe('Tooltip Functionality - Core Tests', () => {
  beforeEach(() => {
    cy.visit(createTestUrl('BTC-USD'));
    cy.waitForChartRender();
  });

  describe('Basic Tooltip Behavior', () => {
    it('should activate and deactivate tooltip correctly', () => {
      // Test activation
      cy.activateTooltip();
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-basic-activated');
      
      // Test deactivation
      cy.deactivateTooltip();
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-basic-deactivated');
    });

    it('should not activate on left-click', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Left-click should not activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 0, force: true })
          .trigger('mouseup', centerX, centerY, { button: 0, force: true });

        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-no-left-click');
      });
    });
  });

  describe('Tooltip Positioning', () => {
    it('should show tooltip at different chart positions', () => {
      cy.testTooltipAtPosition('left', 'tooltip-position-left');
      cy.testTooltipAtPosition('quarter', 'tooltip-position-quarter');
      cy.testTooltipAtPosition('center', 'tooltip-position-center');
      cy.testTooltipAtPosition('three-quarter', 'tooltip-position-three-quarter');
      cy.testTooltipAtPosition('right', 'tooltip-position-right');
    });

    it('should update position when mouse moves', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const startX = rect.left + rect.width / 4;
        const endX = rect.left + (3 * rect.width) / 4;
        const centerY = rect.top + rect.height / 2;

        // Activate at start position
        cy.activateTooltip(startX, centerY);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-move-start');

        // Move to end position
        cy.moveTooltip(endX, centerY);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-move-end');

        cy.deactivateTooltip(endX, centerY);
      });
    });
  });

  describe('Tooltip with Different Presets', () => {
    it('should work with Market Data preset', () => {
      cy.selectPreset('Market Data');
      cy.waitForPresetChange();
      
      cy.activateTooltip();
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-market-data-preset');
      cy.deactivateTooltip();
    });

    it('should work with Candlestick preset', () => {
      cy.selectPreset('Candlestick');
      cy.waitForPresetChange();
      
      cy.activateTooltip();
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-candlestick-preset');
      cy.deactivateTooltip();
    });
  });

  describe('Tooltip Performance', () => {
    it('should maintain good performance during movement', () => {
      cy.checkTooltipPerformance('Tooltip Movement Performance');
    });

    it('should handle rapid activation/deactivation', () => {
      // Rapidly toggle tooltip multiple times
      for (let i = 0; i < 5; i++) {
        cy.activateTooltip();
        cy.wait(50);
        cy.deactivateTooltip();
        cy.wait(50);
      }

      // Final state should be clean
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-rapid-toggle-clean');
    });
  });

  describe('Tooltip Edge Cases', () => {
    it('should handle tooltip at chart edges', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerY = rect.top + rect.height / 2;

        // Test left edge
        cy.activateTooltip(rect.left + 5, centerY);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-edge-left');
        cy.deactivateTooltip(rect.left + 5, centerY);

        cy.wait(100);

        // Test right edge
        cy.activateTooltip(rect.right - 5, centerY);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-edge-right');
        cy.deactivateTooltip(rect.right - 5, centerY);
      });
    });

    it('should work correctly with zoomed chart', () => {
      // Zoom in
      cy.zoomChart('in', 300);
      cy.wait(500);

      cy.activateTooltip();
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-zoomed-in');
      cy.deactivateTooltip();

      // Zoom out
      cy.zoomChart('out', 600);
      cy.wait(500);

      cy.activateTooltip();
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-zoomed-out');
      cy.deactivateTooltip();
    });

    it('should handle tooltip during other interactions', () => {
      cy.activateTooltip();
      
      // Try to zoom while tooltip is active (should not interfere)
      cy.zoomChart('in', 200);
      cy.wait(200);
      
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-during-zoom');
      cy.deactivateTooltip();
    });
  });

  describe('Tooltip Visual Quality', () => {
    it('should render with consistent styling', () => {
      const presets = ['Market Data', 'Candlestick'];

      presets.forEach((preset) => {
        cy.selectPreset(preset);
        cy.waitForPresetChange();

        cy.activateTooltip();
        cy.get('canvas#webgpu-canvas').compareSnapshot(`tooltip-style-${preset.toLowerCase().replace(' ', '-')}`);
        cy.deactivateTooltip();
      });
    });

    it('should show clear vertical line and labels', () => {
      cy.selectPreset('Market Data');
      cy.waitForPresetChange();

      cy.activateTooltip();
      cy.wait(200); // Extra wait for labels to stabilize
      cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-visual-quality');
      cy.deactivateTooltip();
    });
  });
});