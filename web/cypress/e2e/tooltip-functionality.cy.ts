import { createTestUrl } from '../support/test-constants';

describe('Tooltip Functionality', () => {
  beforeEach(() => {
    cy.visit(createTestUrl('BTC-USD'));
    cy.waitForChartRender();
  });

  describe('Tooltip Activation and Deactivation', () => {
    it('should activate tooltip on right-click and hold', () => {
      // Get canvas for interactions
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Right-click and hold to activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        // Wait for tooltip to render
        cy.wait(100);

        // Check that tooltip state is active (will be verified via visual regression)
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-activated');
      });
    });

    it('should deactivate tooltip on right-click release', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-active-before-release');

        // Release right mouse button
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', centerX, centerY, { button: 2 });

        cy.wait(100);

        // Tooltip should be deactivated
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-deactivated');
      });
    });

    it('should not activate tooltip on left-click', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Left-click should not activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 0 })
          .trigger('mouseup', centerX, centerY, { button: 0 });

        cy.wait(100);

        // No tooltip should be visible
        cy.get('canvas#webgpu-canvas').compareSnapshot('no-tooltip-on-left-click');
      });
    });
  });

  describe('Tooltip Line and Positioning', () => {
    it('should show vertical line at mouse position', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const quarterX = rect.left + rect.width / 4;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip at quarter position
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', quarterX, centerY, { button: 2 });

        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-line-quarter-position');
      });
    });

    it('should update line position when mouse moves while tooltip is active', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const startX = rect.left + rect.width / 4;
        const endX = rect.left + (3 * rect.width) / 4;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', startX, centerY, { button: 2 });

        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-line-start-position');

        // Move mouse while tooltip is active
        cy.get('canvas#webgpu-canvas')
          .trigger('mousemove', endX, centerY);

        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-line-moved-position');
      });
    });

    it('should show tooltip line across different chart sections', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const positions = [
          { x: rect.left + rect.width * 0.1, name: 'left-edge' },
          { x: rect.left + rect.width * 0.5, name: 'center' },
          { x: rect.left + rect.width * 0.9, name: 'right-edge' }
        ];
        const centerY = rect.top + rect.height / 2;

        positions.forEach((pos, index) => {
          // Activate tooltip at position
          cy.get('canvas#webgpu-canvas')
            .trigger('mousedown', pos.x, centerY, { button: 2 });

          cy.wait(100);
          cy.get('canvas#webgpu-canvas').compareSnapshot(`tooltip-line-${pos.name}`);

          // Deactivate for next test
          cy.get('canvas#webgpu-canvas')
            .trigger('mouseup', pos.x, centerY, { button: 2 });

          cy.wait(50);
        });
      });
    });
  });

  describe('Tooltip Labels and Values', () => {
    it('should display labels with correct values for market data', () => {
      // Select Market Data preset to ensure consistent data
      cy.selectPreset('Market Data');
      cy.waitForPresetChange();

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(200);

        // Check labels are visible and properly positioned
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-labels-market-data');
      });
    });

    it('should display labels with correct values for candlestick data', () => {
      // Select Candlestick preset
      cy.selectPreset('Candlestick');
      cy.waitForPresetChange();

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(200);

        // Check candlestick labels (OHLC values)
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-labels-candlestick');
      });
    });

    it('should update label values when tooltip moves', () => {
      cy.selectPreset('Market Data');
      cy.waitForPresetChange();

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const leftX = rect.left + rect.width / 4;
        const rightX = rect.left + (3 * rect.width) / 4;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip at left position
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', leftX, centerY, { button: 2 });

        cy.wait(200);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-labels-left-position');

        // Move to right position
        cy.get('canvas#webgpu-canvas')
          .trigger('mousemove', rightX, centerY);

        cy.wait(200);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-labels-right-position');
      });
    });
  });

  describe('Tooltip Visual Styling', () => {
    it('should render tooltip line with correct styling', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(100);

        // Check line styling (semi-transparent white)
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-line-styling');
      });
    });

    it('should render label backgrounds with proper opacity', () => {
      cy.selectPreset('Market Data');
      cy.waitForPresetChange();

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(200);

        // Check label background styling
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-label-backgrounds');
      });
    });

    it('should maintain consistent styling across different presets', () => {
      const presets = ['Market Data', 'Candlestick'];

      presets.forEach((preset) => {
        cy.selectPreset(preset);
        cy.waitForPresetChange();

        cy.get('canvas#webgpu-canvas').then(($canvas) => {
          const canvas = $canvas[0];
          const rect = canvas.getBoundingClientRect();
          const centerX = rect.left + rect.width / 2;
          const centerY = rect.top + rect.height / 2;

          // Activate tooltip
          cy.get('canvas#webgpu-canvas')
            .trigger('mousedown', centerX, centerY, { button: 2 });

          cy.wait(200);

          // Check styling consistency
          cy.get('canvas#webgpu-canvas').compareSnapshot(`tooltip-styling-${preset.toLowerCase().replace(' ', '-')}`);

          // Deactivate tooltip
          cy.get('canvas#webgpu-canvas')
            .trigger('mouseup', centerX, centerY, { button: 2 });

          cy.wait(100);
        });
      });
    });
  });

  describe('Tooltip Color Coding', () => {
    it('should use appropriate colors for different data series', () => {
      cy.selectPreset('Market Data');
      cy.waitForPresetChange();

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(200);

        // Check that different series use different colors
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-color-coding-market-data');
      });
    });

    it('should use correct colors for candlestick OHLC values', () => {
      cy.selectPreset('Candlestick');
      cy.waitForPresetChange();

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(200);

        // Check OHLC color coding
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-color-coding-candlestick');
      });
    });
  });

  describe('Tooltip Performance', () => {
    it('should maintain 60 FPS during tooltip interactions', () => {
      let frameCount = 0;
      let startTime = 0;
      let rafId: number;

      cy.window().then((win) => {
        const measureFPS = () => {
          if (startTime === 0) {
            startTime = performance.now();
          }
          frameCount++;
          rafId = requestAnimationFrame(measureFPS);
        };

        // Start measuring FPS
        measureFPS();

        cy.get('canvas#webgpu-canvas').then(($canvas) => {
          const canvas = $canvas[0];
          const rect = canvas.getBoundingClientRect();
          const centerX = rect.left + rect.width / 2;
          const centerY = rect.top + rect.height / 2;

          // Activate tooltip
          cy.get('canvas#webgpu-canvas')
            .trigger('mousedown', centerX, centerY, { button: 2 });

          // Perform rapid mouse movements
          for (let i = 0; i < 10; i++) {
            const x = rect.left + (rect.width / 10) * i;
            cy.get('canvas#webgpu-canvas')
              .trigger('mousemove', x, centerY);
            cy.wait(16); // ~60 FPS timing
          }

          cy.wait(1000).then(() => {
            // Stop measuring and check FPS
            cancelAnimationFrame(rafId);
            const endTime = performance.now();
            const duration = (endTime - startTime) / 1000;
            const fps = frameCount / duration;

            // FPS should be close to 60
            expect(fps).to.be.greaterThan(50);
            cy.log(`Measured FPS: ${fps.toFixed(2)}`);
          });

          // Deactivate tooltip
          cy.get('canvas#webgpu-canvas')
            .trigger('mouseup', centerX, centerY, { button: 2 });
        });
      });
    });

    it('should update tooltip smoothly without frame drops', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const startX = rect.left + rect.width * 0.1;
        const endX = rect.left + rect.width * 0.9;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', startX, centerY, { button: 2 });

        cy.wait(100);

        // Perform smooth movement across the chart
        const steps = 20;
        for (let i = 0; i <= steps; i++) {
          const x = startX + ((endX - startX) * i) / steps;
          cy.get('canvas#webgpu-canvas')
            .trigger('mousemove', x, centerY);
          cy.wait(16); // 60 FPS timing
        }

        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-smooth-movement-end');

        // Deactivate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', endX, centerY, { button: 2 });
      });
    });

    it('should handle rapid tooltip activation/deactivation', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Rapidly activate and deactivate tooltip
        for (let i = 0; i < 5; i++) {
          cy.get('canvas#webgpu-canvas')
            .trigger('mousedown', centerX, centerY, { button: 2 });
          cy.wait(50);
          cy.get('canvas#webgpu-canvas')
            .trigger('mouseup', centerX, centerY, { button: 2 });
          cy.wait(50);
        }

        // Final state should be clean (no tooltip)
        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-rapid-toggle-final');
      });
    });
  });

  describe('Tooltip Edge Cases', () => {
    it('should handle tooltip at chart edges', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerY = rect.top + rect.height / 2;

        // Test near left edge
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', rect.left + 10, centerY, { button: 2 });
        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-left-edge');
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', rect.left + 10, centerY, { button: 2 });

        cy.wait(100);

        // Test near right edge
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', rect.right - 10, centerY, { button: 2 });
        cy.wait(100);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-right-edge');
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', rect.right - 10, centerY, { button: 2 });
      });
    });

    it('should handle tooltip during zoom operations', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });
        cy.wait(100);

        // Zoom while tooltip is active
        cy.zoomChart('in', 200);
        cy.wait(100);

        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-during-zoom');

        // Deactivate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', centerX, centerY, { button: 2 });
      });
    });

    it('should handle tooltip during pan operations', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });
        cy.wait(100);

        // Note: Pan uses left mouse button, so tooltip should remain active
        // We'll just test that tooltip remains visible during other interactions
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-before-other-interaction');

        // Move mouse (should update tooltip position)
        cy.get('canvas#webgpu-canvas')
          .trigger('mousemove', centerX + 100, centerY);
        cy.wait(100);

        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-after-mouse-move');

        // Deactivate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', centerX + 100, centerY, { button: 2 });
      });
    });

    it('should handle multiple simultaneous right-clicks', () => {
      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Simulate multiple rapid right-clicks
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 })
          .trigger('mouseup', centerX, centerY, { button: 2 })
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(100);

        // Should handle gracefully and show tooltip
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-multiple-right-clicks');

        // Clean up
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', centerX, centerY, { button: 2 });
      });
    });
  });

  describe('Tooltip with Different Data Ranges', () => {
    it('should work correctly with zoomed-in data', () => {
      // Zoom in significantly
      cy.zoomChart('in', 500);
      cy.wait(500);

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip on zoomed chart
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(200);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-zoomed-in-chart');

        // Deactivate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', centerX, centerY, { button: 2 });
      });
    });

    it('should work correctly with zoomed-out data', () => {
      // Zoom out significantly
      cy.zoomChart('out', 800);
      cy.wait(500);

      cy.get('canvas#webgpu-canvas').then(($canvas) => {
        const canvas = $canvas[0];
        const rect = canvas.getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        // Activate tooltip on zoomed-out chart
        cy.get('canvas#webgpu-canvas')
          .trigger('mousedown', centerX, centerY, { button: 2 });

        cy.wait(200);
        cy.get('canvas#webgpu-canvas').compareSnapshot('tooltip-zoomed-out-chart');

        // Deactivate tooltip
        cy.get('canvas#webgpu-canvas')
          .trigger('mouseup', centerX, centerY, { button: 2 });
      });
    });
  });
});