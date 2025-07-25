# Preset Implementation Plan - Full Stack

## Overview
Complete implementation of preset-based rendering with uniform data flow, from data fetching through UI to testing.

## Phase 1: Data Flow Implementation (Agent 1)

### Goals
- Implement uniform data flow where all data types go through GPU buffers
- Update renderers to read from buffers consistently
- Wire up preset application with multiple DataGroups

### Tasks
1. **Update DataManager**
   - Add `data_type` parameter to `fetch_data` method
   - Include `type` in API request URL
   - Update cache keys to include data_type

2. **Update TriangleRenderer**
   - Modify to read from GPU buffers instead of TradeData array
   - Update shader to use instance data from buffers
   - Handle side buffer (0=sell/red/down, 1=buy/green/up)

3. **Wire Up Preset Application**
   - Extract data types from preset configuration
   - Create separate DataGroups for each data type
   - Set appropriate active groups
   - Connect renderers to their data groups

4. **Handle Computed Fields**
   - Implement ComputeOp calculations (e.g., mid price)
   - Add computed metrics to DataStore
   - Update renderers to use computed values

## Phase 2: UI Implementation (Agent 2)

### Goals
- Create user-friendly preset selection UI
- Integrate with existing React application
- Handle loading states and errors gracefully

### Tasks
1. **Create PresetSelector Component**
   - Dropdown/select component for preset selection
   - Group presets by category (Market Data, Candles, etc.)
   - Show preset descriptions on hover
   - Handle loading/error states

2. **Update Main App**
   - Add PresetSelector to the chart UI
   - Wire up to WASM methods (list_presets, apply_preset)
   - Update chart on preset selection
   - Show active preset indicator

3. **Data Fetching Integration**
   - Trigger data fetch when preset is applied
   - Show loading spinner during fetch
   - Handle and display errors
   - Update time range controls if needed

4. **Polish**
   - Add transitions/animations
   - Ensure responsive design
   - Add keyboard shortcuts
   - Implement preset persistence (localStorage)

## Phase 3: Integration & Testing

### Outstanding Tasks to Complete
1. **Fix Compilation Issues**
   - Resolve any type mismatches
   - Fix import issues
   - Ensure all crates compile

2. **Complete Data Pipeline**
   - Ensure data flows correctly from API to renderers
   - Test with real market data
   - Verify time alignment works properly

3. **Multi-Renderer Integration**
   - Ensure MultiRenderer properly combines outputs
   - Test render order and layering
   - Verify performance with multiple renderers

4. **Edge Cases**
   - Handle missing data gracefully
   - Test with different time ranges
   - Ensure proper cleanup when switching presets

### Testing Plan with Playwright
1. **Basic Functionality**
   - Navigate to app
   - Select different presets
   - Verify correct data is displayed
   - Check renderer combinations work

2. **Data Verification**
   - Verify bid/ask lines appear correctly
   - Check trade triangles render at correct positions
   - Ensure computed fields (mid price) calculate properly
   - Test with different symbols

3. **UI Testing**
   - Test preset selector interaction
   - Verify loading states
   - Check error handling
   - Test keyboard navigation

4. **Performance Testing**
   - Load large datasets
   - Switch between presets rapidly
   - Verify no memory leaks
   - Check GPU buffer management

## Success Criteria for MR
- [ ] All presets render correctly with appropriate data
- [ ] UI is intuitive and responsive
- [ ] No console errors or warnings
- [ ] Performance is acceptable (60fps)
- [ ] Code is clean and well-documented
- [ ] All tests pass
- [ ] Works with real market data

## Example Test Scenarios
1. **Bid/Ask Lines**: Should show two continuous lines
2. **Trades Overlay**: Should show triangles at trade points
3. **Mid Price**: Should calculate and display correctly
4. **Candles + Volume**: Should show in separate panels
5. **Preset Switching**: Should cleanly transition between presets

## Architecture Benefits Achieved
- Uniform data flow for all renderer types
- Clean separation of concerns
- Extensible for future preset types
- Performant GPU-based rendering
- Intuitive user experience