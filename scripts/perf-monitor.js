#!/usr/bin/env node
/**
 * Performance monitoring script for GPU Charts
 * Tracks memory usage, frame times, and other metrics
 */

const PERFORMANCE_TARGETS = {
    frameTime: 16.67, // 60 FPS
    memoryUsage: 2.0, // 2x data size
    cacheHitRate: 0.8, // 80%
    gpuMemory: 2048, // 2GB max
};

class PerformanceMonitor {
    constructor() {
        this.metrics = {
            frameCount: 0,
            frameTimes: [],
            memorySnapshots: [],
            cacheStats: null,
            gpuStats: null,
        };
        this.startTime = Date.now();
    }

    recordFrame(frameTime) {
        this.metrics.frameCount++;
        this.metrics.frameTimes.push(frameTime);
        
        // Keep only last 1000 frame times
        if (this.metrics.frameTimes.length > 1000) {
            this.metrics.frameTimes.shift();
        }
    }

    recordMemory(heapUsed, heapTotal, external) {
        this.metrics.memorySnapshots.push({
            timestamp: Date.now(),
            heapUsed,
            heapTotal,
            external,
        });
    }

    updateCacheStats(hitRate, entries, sizeMB) {
        this.metrics.cacheStats = { hitRate, entries, sizeMB };
    }

    updateGPUStats(memoryMB, bufferCount) {
        this.metrics.gpuStats = { memoryMB, bufferCount };
    }

    getReport() {
        const frameTimes = this.metrics.frameTimes;
        const avgFrameTime = frameTimes.reduce((a, b) => a + b, 0) / frameTimes.length;
        const p95FrameTime = frameTimes.sort((a, b) => a - b)[Math.floor(frameTimes.length * 0.95)];
        const p99FrameTime = frameTimes.sort((a, b) => a - b)[Math.floor(frameTimes.length * 0.99)];

        const report = {
            duration: (Date.now() - this.startTime) / 1000,
            frameCount: this.metrics.frameCount,
            fps: {
                average: 1000 / avgFrameTime,
                p95: 1000 / p95FrameTime,
                p99: 1000 / p99FrameTime,
            },
            frameTimes: {
                average: avgFrameTime,
                p95: p95FrameTime,
                p99: p99FrameTime,
            },
            memory: this.getMemoryStats(),
            cache: this.metrics.cacheStats,
            gpu: this.metrics.gpuStats,
            targets: this.checkTargets(),
        };

        return report;
    }

    getMemoryStats() {
        if (this.metrics.memorySnapshots.length === 0) return null;

        const latest = this.metrics.memorySnapshots[this.metrics.memorySnapshots.length - 1];
        const heapUsedMB = latest.heapUsed / 1024 / 1024;
        const heapTotalMB = latest.heapTotal / 1024 / 1024;

        return {
            heapUsedMB,
            heapTotalMB,
            externalMB: latest.external / 1024 / 1024,
        };
    }

    checkTargets() {
        const avgFrameTime = this.metrics.frameTimes.reduce((a, b) => a + b, 0) / this.metrics.frameTimes.length;
        
        return {
            frameTime: avgFrameTime <= PERFORMANCE_TARGETS.frameTime,
            cacheHitRate: this.metrics.cacheStats?.hitRate >= PERFORMANCE_TARGETS.cacheHitRate,
            gpuMemory: this.metrics.gpuStats?.memoryMB <= PERFORMANCE_TARGETS.gpuMemory,
        };
    }

    printReport() {
        const report = this.getReport();
        
        console.log('\n=== Performance Report ===');
        console.log(`Duration: ${report.duration.toFixed(1)}s`);
        console.log(`Frames: ${report.frameCount}`);
        
        console.log('\nFrame Performance:');
        console.log(`  Average FPS: ${report.fps.average.toFixed(1)}`);
        console.log(`  P95 FPS: ${report.fps.p95.toFixed(1)}`);
        console.log(`  P99 FPS: ${report.fps.p99.toFixed(1)}`);
        
        if (report.memory) {
            console.log('\nMemory Usage:');
            console.log(`  Heap Used: ${report.memory.heapUsedMB.toFixed(1)} MB`);
            console.log(`  Heap Total: ${report.memory.heapTotalMB.toFixed(1)} MB`);
        }
        
        if (report.cache) {
            console.log('\nCache Performance:');
            console.log(`  Hit Rate: ${(report.cache.hitRate * 100).toFixed(1)}%`);
            console.log(`  Entries: ${report.cache.entries}`);
            console.log(`  Size: ${report.cache.sizeMB.toFixed(1)} MB`);
        }
        
        if (report.gpu) {
            console.log('\nGPU Resources:');
            console.log(`  Memory: ${report.gpu.memoryMB.toFixed(1)} MB`);
            console.log(`  Buffers: ${report.gpu.bufferCount}`);
        }
        
        console.log('\nPerformance Targets:');
        Object.entries(report.targets).forEach(([key, passed]) => {
            console.log(`  ${key}: ${passed ? '✓ PASS' : '✗ FAIL'}`);
        });
    }
}

// Export for use in tests
if (typeof module !== 'undefined' && module.exports) {
    module.exports = PerformanceMonitor;
}

// CLI usage
if (require.main === module) {
    console.log('GPU Charts Performance Monitor');
    console.log('This would connect to a running instance and monitor performance.');
    console.log('Usage: node perf-monitor.js [--duration=60] [--output=report.json]');
}