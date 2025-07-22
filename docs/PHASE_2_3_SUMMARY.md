# GPU Charts - Phase 2 & 3 Quick Reference

## Phase 2: Core Infrastructure (5.5 weeks)

### Week 1-2: DataManager
- [ ] Handle-based API with zero-copy buffers
- [ ] LRU cache with configurable limits
- [ ] SIMD optimizations for 2-3x speedup
- [ ] Chunked parsing for large datasets

### Week 3-4: GPU Optimization
- [ ] Compute shader vertex generation
- [ ] Indirect draw calls
- [ ] Multi-resolution rendering
- [ ] Vertex compression (<8 bytes)

### Week 5-5.5: Networking
- [ ] HTTP/2 with connection pooling
- [ ] Compression (Gzip, Brotli)
- [ ] WebSocket real-time streaming
- [ ] Request batching & cancellation

**Goal**: 1B points @ 60 FPS

---

## Phase 3: Features & Production (6.5 weeks)

### Week 1: Configuration
- [ ] Hot-reloadable config system
- [ ] Performance auto-tuning
- [ ] 20+ built-in presets
- [ ] A/B testing support

### Week 2-3: Integration
- [ ] DataManager ↔ Renderer connection
- [ ] Unified TypeScript API
- [ ] React component library
- [ ] Performance dashboard

### Week 4-5: Advanced Features
- [ ] Scatter plots & heatmaps
- [ ] 3D charts with WebGPU
- [ ] 10+ technical indicators
- [ ] Annotation system

### Week 6-6.5: Infrastructure
- [ ] GPU test suite
- [ ] Visual regression testing
- [ ] DevTools extension
- [ ] Production deployment

**Goal**: Production-ready with 5+ chart types

---

## Key Metrics

| Metric | Current | Phase 2 | Phase 3 |
|--------|---------|---------|---------|
| Max Points | 1M | 1B | 1B |
| FPS | 180 | 60 | 60 |
| Chart Types | 1 | 1 | 5+ |
| Indicators | 0 | 0 | 10+ |
| Test Coverage | 70% | 85% | 95% |

---

## Critical Path

```
DataManager → GPU Compute → Networking → Integration → Features → Production
    2w           2w           1.5w         1.5w         2w         1.5w
```

---

## Team Requirements

- **Phase 2**: 2 engineers (GPU + Systems)
- **Phase 3**: 3 engineers (+ Frontend)
- **Total Duration**: 12 weeks
- **Budget**: $150-200K

---

## Risk Summary

### High Risk
- GPU memory limits → Implement tiling
- 1B point target → Have 500M fallback

### Medium Risk
- SIMD complexity → Platform fallbacks
- WebGPU support → WebGL 2.0 path

### Low Risk
- Integration issues → Continuous testing
- Schedule slip → Buffer time included