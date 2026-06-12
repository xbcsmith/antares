# Phase 5: Wind Animation (Optional Enhancement)

**Goal**: Add subtle wind effects for visual dynamism

## 5.1 Implement Wind System

**Add procedural wind animation**:

**Files to Create**:

- `src/game/systems/advanced_grass.rs::wind` module

**New Components**:

```rust
#[derive(Resource)]
pub struct WindConfig {
    pub strength: f32,
    pub frequency: f32,
    pub direction: Vec2,
}

#[derive(Component)]
pub struct WindAffected {
    pub phase_offset: f32,
}
```

**Animation System**:

```rust
fn grass_wind_system(
    time: Res<Time>,
    wind_config: Res<WindConfig>,
    mut grass_query: Query<(&mut Transform, &WindAffected), With<GrassBlade>>,
) {
    // Apply sinusoidal motion to blade tips
}
```

## 5.2 Shader-Based Wind (Advanced)

**For smoother performance, implement wind in vertex shader**:

**Files to Create**:

- `assets/shaders/grass.wgsl`

**Note**: Requires custom material and render pipeline setup

**Reference**: `bevy_procedural_grass/src/assets/shaders/grass.wgsl`

## 5.3 Testing Requirements

**Visual Tests**:

- [ ] Wind motion visible and smooth
- [ ] Wind direction configurable
- [ ] Wind strength affects motion amplitude
- [ ] No performance degradation

## 5.4 Deliverables

- [ ] CPU-based wind system (simpler approach)
- [ ] OR shader-based wind (advanced approach)
- [ ] Wind configuration in `GrassConfig`
- [ ] Documentation and examples

## 5.5 Success Criteria

- [ ] Wind animation visible and natural-looking
- [ ] Configurable wind parameters
- [ ] No performance impact from wind system
- [ ] Can be disabled for performance
