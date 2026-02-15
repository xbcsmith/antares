# Skeletal Animation System

**Status**: Implemented (Phase 10)
**Date**: 2025-02-14

## Overview

The skeletal animation system provides advanced character animation capabilities beyond simple keyframe mesh transforms. It implements a hierarchical bone structure with per-bone animation tracks, quaternion-based smooth interpolation, animation blending, inverse kinematics, and state machine control.

## Architecture

### Core Components

The skeletal animation system consists of five main modules:

1. **Skeleton** (`src/domain/visual/skeleton.rs`) - Bone hierarchy and structure
2. **Skeletal Animation** (`src/domain/visual/skeletal_animation.rs`) - Per-bone animation tracks
3. **Blend Trees** (`src/domain/visual/blend_tree.rs`) - Animation blending system
4. **Animation State Machine** (`src/domain/visual/animation_state_machine.rs`) - State-based animation control
5. **Inverse Kinematics** (`src/game/systems/ik.rs`) - Procedural bone positioning

### Data Flow

```
Skeleton Definition (RON)
    ↓
Bone Hierarchy
    ↓
Skeletal Animations (per-bone keyframes)
    ↓
Blend Trees (combine animations)
    ↓
State Machine (manage transitions)
    ↓
IK Solver (procedural adjustments)
    ↓
Final Bone Transforms
    ↓
Skinned Mesh Rendering
```

## 1. Skeletal Hierarchy

### Bone Structure

Each bone in a skeleton has:

- **Unique ID**: Index-based identifier
- **Name**: Human-readable name for reference
- **Parent**: Optional parent bone (None for root bones)
- **Rest Transform**: Default position/rotation/scale relative to parent
- **Inverse Bind Pose**: 4x4 matrix for skinning calculations

### Skeleton Organization

```rust
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub root_bone: BoneId,
}
```

A skeleton maintains:

- Complete bone hierarchy
- Single primary root bone
- Parent-child relationships
- Traversal utilities

### Example: Humanoid Skeleton

```ron
Skeleton(
    bones: [
        Bone(
            id: 0,
            name: "root",
            parent: None,
            rest_transform: MeshTransform(
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            ),
            inverse_bind_pose: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
        Bone(
            id: 1,
            name: "spine",
            parent: Some(0),
            rest_transform: MeshTransform(
                translation: [0.0, 1.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            ),
            inverse_bind_pose: [...],
        ),
        Bone(id: 2, name: "head", parent: Some(1), ...),
        Bone(id: 3, name: "left_shoulder", parent: Some(1), ...),
        Bone(id: 4, name: "left_upper_arm", parent: Some(3), ...),
        Bone(id: 5, name: "left_forearm", parent: Some(4), ...),
        Bone(id: 6, name: "left_hand", parent: Some(5), ...),
    ],
    root_bone: 0,
)
```

### Validation

The skeleton system validates:

- No circular parent references
- All parent IDs exist and are valid
- Root bone has no parent
- Bone IDs match their array indices
- No duplicate bone IDs

## 2. Skeletal Animation

### Per-Bone Animation Tracks

Unlike simple keyframe animations that transform entire meshes, skeletal animations store separate keyframe tracks for each bone:

```rust
pub struct SkeletalAnimation {
    pub name: String,
    pub duration: f32,
    pub bone_tracks: HashMap<BoneId, Vec<BoneKeyframe>>,
    pub looping: bool,
}
```

### Bone Keyframes

Each keyframe stores full transform data:

```rust
pub struct BoneKeyframe {
    pub time: f32,
    pub position: [f32; 3],      // Translation
    pub rotation: [f32; 4],      // Quaternion [x, y, z, w]
    pub scale: [f32; 3],         // Scale
}
```

### Quaternion Rotations

**Why Quaternions?**

- No gimbal lock (unlike Euler angles)
- Smooth interpolation via SLERP
- Compact representation (4 floats)
- Efficient composition

**SLERP (Spherical Linear Interpolation)**:

```rust
fn slerp_quat(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    // Normalize quaternions
    // Calculate angle between them
    // Interpolate along great circle arc
    // Returns smoothly interpolated rotation
}
```

### Animation Sampling

To get bone transform at time `t`:

1. Find keyframes surrounding time `t`
2. Calculate interpolation factor
3. LERP position and scale
4. SLERP rotation
5. Return interpolated keyframe

```rust
let sample = animation.sample_bone(bone_id, 0.5);
// Returns interpolated transform at t=0.5 seconds
```

### Example: Walk Cycle Animation

```ron
SkeletalAnimation(
    name: "Walk",
    duration: 2.0,
    bone_tracks: {
        // Left leg
        3: [
            BoneKeyframe(time: 0.0, position: [0.0, 0.0, 0.5], ...),
            BoneKeyframe(time: 1.0, position: [0.0, 0.0, -0.5], ...),
            BoneKeyframe(time: 2.0, position: [0.0, 0.0, 0.5], ...),
        ],
        // Right leg (opposite phase)
        4: [
            BoneKeyframe(time: 0.0, position: [0.0, 0.0, -0.5], ...),
            BoneKeyframe(time: 1.0, position: [0.0, 0.0, 0.5], ...),
            BoneKeyframe(time: 2.0, position: [0.0, 0.0, -0.5], ...),
        ],
        // Arms swing counter to legs
        5: [...],
        6: [...],
    },
    looping: true,
)
```

## 3. Animation Blend Trees

### Blend Node Types

Blend trees combine multiple animations:

```rust
pub enum BlendNode {
    Clip(AnimationClip),                    // Single animation
    Blend2D { ... },                        // 2D blend space
    Additive { base, additive, weight },    // Additive layer
    LayeredBlend { layers },                // Multiple layers
}
```

### Simple Clip

Play a single animation:

```rust
BlendNode::Clip(AnimationClip {
    animation_name: "Idle".to_string(),
    speed: 1.0,
})
```

### 2D Blend Space

Blend animations based on two parameters (e.g., speed and direction):

```rust
BlendNode::Blend2D {
    x_param: "speed".to_string(),
    y_param: "direction".to_string(),
    samples: vec![
        BlendSample {
            position: Vec2::new(0.0, 0.0),   // Idle at origin
            animation: AnimationClip::new("Idle".to_string(), 1.0),
        },
        BlendSample {
            position: Vec2::new(1.0, 0.0),   // Walk at speed=1
            animation: AnimationClip::new("Walk".to_string(), 1.0),
        },
        BlendSample {
            position: Vec2::new(3.0, 0.0),   // Run at speed=3
            animation: AnimationClip::new("Run".to_string(), 1.0),
        },
        BlendSample {
            position: Vec2::new(1.0, 45.0),  // Strafe walk
            animation: AnimationClip::new("StrafeWalk".to_string(), 1.0),
        },
    ],
}
```

### Additive Blending

Add an animation on top of a base (e.g., hit reaction while running):

```rust
BlendNode::Additive {
    base: Box::new(BlendNode::clip("Run".to_string(), 1.0)),
    additive: Box::new(BlendNode::clip("HitReaction".to_string(), 1.0)),
    weight: 0.75,  // 75% of hit reaction added
}
```

### Layered Blending

Combine multiple animations (e.g., upper body shooting, lower body walking):

```rust
BlendNode::LayeredBlend {
    layers: vec![
        (Box::new(BlendNode::clip("UpperBodyShoot".to_string(), 1.0)), 1.0),
        (Box::new(BlendNode::clip("LowerBodyWalk".to_string(), 1.0)), 1.0),
    ],
}
```

## 4. Animation State Machine

### State Machine Structure

Manages animation states and transitions:

```rust
pub struct AnimationStateMachine {
    pub name: String,
    pub states: HashMap<String, AnimationState>,
    pub transitions: Vec<Transition>,
    pub current_state: String,
    pub parameters: HashMap<String, f32>,
}
```

### Animation States

Each state has a blend tree:

```rust
pub struct AnimationState {
    pub name: String,
    pub blend_tree: BlendNode,
}
```

### Transitions

Conditional transitions between states:

```rust
pub struct Transition {
    pub from: String,
    pub to: String,
    pub condition: TransitionCondition,
    pub duration: f32,  // Blend duration
}
```

### Transition Conditions

```rust
pub enum TransitionCondition {
    Always,
    GreaterThan { parameter: String, threshold: f32 },
    LessThan { parameter: String, threshold: f32 },
    Equal { parameter: String, value: f32 },
    InRange { parameter: String, min: f32, max: f32 },
    And(Vec<TransitionCondition>),
    Or(Vec<TransitionCondition>),
    Not(Box<TransitionCondition>),
}
```

### Example: Character Locomotion

```ron
AnimationStateMachine(
    name: "CharacterLocomotion",
    states: {
        "Idle": AnimationState(
            name: "Idle",
            blend_tree: Clip(AnimationClip(
                animation_name: "IdleAnimation",
                speed: 1.0,
            )),
        ),
        "Walk": AnimationState(
            name: "Walk",
            blend_tree: Clip(AnimationClip(
                animation_name: "WalkAnimation",
                speed: 1.0,
            )),
        ),
        "Run": AnimationState(
            name: "Run",
            blend_tree: Clip(AnimationClip(
                animation_name: "RunAnimation",
                speed: 1.0,
            )),
        ),
        "Jump": AnimationState(
            name: "Jump",
            blend_tree: Clip(AnimationClip(
                animation_name: "JumpAnimation",
                speed: 1.0,
            )),
        ),
    },
    transitions: [
        Transition(
            from: "Idle",
            to: "Walk",
            condition: GreaterThan(parameter: "speed", threshold: 0.1),
            duration: 0.3,
        ),
        Transition(
            from: "Walk",
            to: "Run",
            condition: GreaterThan(parameter: "speed", threshold: 3.0),
            duration: 0.2,
        ),
        Transition(
            from: "Walk",
            to: "Idle",
            condition: LessThan(parameter: "speed", threshold: 0.1),
            duration: 0.3,
        ),
        Transition(
            from: "Idle",
            to: "Jump",
            condition: GreaterThan(parameter: "jump_pressed", threshold: 0.5),
            duration: 0.1,
        ),
    ],
    current_state: "Idle",
    parameters: {
        "speed": 0.0,
        "jump_pressed": 0.0,
    },
)
```

### Runtime Usage

```rust
// Update parameters based on game state
state_machine.set_parameter("speed".to_string(), character_velocity.length());
state_machine.set_parameter("jump_pressed".to_string(), if jump_pressed { 1.0 } else { 0.0 });

// Check for transitions
if let Some(transition) = state_machine.update() {
    println!("Transitioning from {} to {} over {} seconds",
             transition.from, transition.to, transition.duration);
}
```

## 5. Inverse Kinematics (IK)

### Two-Bone IK Solver

Calculates bone rotations to position an end effector at a target:

```rust
pub struct IkChain {
    pub bones: [BoneId; 2],      // [upper_bone, lower_bone]
    pub target: Vec3,             // Target position
    pub pole_target: Option<Vec3>, // Controls joint bend direction
}

pub fn solve_two_bone_ik(
    root_pos: Vec3,
    mid_pos: Vec3,
    end_pos: Vec3,
    target: Vec3,
    pole_target: Option<Vec3>,
) -> [Quat; 2]  // Returns [root_rotation, mid_rotation]
```

### Algorithm

1. Calculate bone lengths
2. Calculate distance to target
3. Clamp to reachable range (min/max reach)
4. Use law of cosines to find joint angles
5. Calculate rotation axis from pole vector
6. Generate quaternions for both joints

### Use Cases

**Foot Placement on Terrain**:

```rust
let left_foot_chain = IkChain {
    bones: [thigh_bone_id, shin_bone_id],
    target: ground_position_under_foot,
    pole_target: Some(knee_direction),
};

let rotations = solve_two_bone_ik(...);
// Apply rotations to thigh and shin bones
```

**Hand Reaching for Object**:

```rust
let right_arm_chain = IkChain {
    bones: [upper_arm_id, forearm_id],
    target: object_position,
    pole_target: Some(elbow_direction),
};
```

**Look-At Target**:

```rust
let neck_chain = IkChain {
    bones: [neck_id, head_id],
    target: look_at_position,
    pole_target: None,
};
```

### Pole Vector

The pole vector controls the direction the middle joint bends (elbow/knee direction):

```
Without pole vector: Joint bends in unpredictable direction
With pole vector: Joint bends toward pole target
```

Example:

```rust
// Knee bends forward (toward positive Z)
pole_target: Some(Vec3::new(0.0, 0.0, 1.0))

// Elbow bends upward (toward positive Y)
pole_target: Some(Vec3::new(0.0, 1.0, 0.0))
```

## Integration with Game Engine

### Bevy ECS Components

```rust
// Skeletal creature component
#[derive(Component)]
pub struct SkeletalCreature {
    pub skeleton: Skeleton,
    pub current_animation: SkeletalAnimation,
    pub state_machine: AnimationStateMachine,
}

// Bone entity component
#[derive(Component)]
pub struct BoneTransform {
    pub bone_id: BoneId,
    pub local_transform: Transform,
    pub world_transform: Transform,
}

// IK target component
#[derive(Component)]
pub struct IkTarget {
    pub chain: IkChain,
    pub enabled: bool,
}
```

### Animation Playback System

```rust
fn skeletal_animation_system(
    time: Res<Time>,
    mut query: Query<(&mut SkeletalCreature, &Children)>,
    mut bone_query: Query<(&BoneTransform, &mut Transform)>,
) {
    for (mut creature, children) in query.iter_mut() {
        // Update state machine
        creature.state_machine.update();

        // Get current animation from active state
        let current_time = time.elapsed_secs();

        // Sample animation for each bone
        for bone_id in 0..creature.skeleton.bones.len() {
            if let Some(keyframe) = creature.current_animation.sample_bone(bone_id, current_time) {
                // Find bone entity and update transform
                if let Some(bone_entity) = children.get(bone_id) {
                    if let Ok((_, mut transform)) = bone_query.get_mut(*bone_entity) {
                        transform.translation = Vec3::from(keyframe.position);
                        transform.rotation = Quat::from_array(keyframe.rotation);
                        transform.scale = Vec3::from(keyframe.scale);
                    }
                }
            }
        }
    }
}
```

### IK System

```rust
fn ik_solver_system(
    query: Query<(&SkeletalCreature, &IkTarget, &Children)>,
    mut bone_query: Query<(&BoneTransform, &mut Transform)>,
) {
    for (creature, ik_target, children) in query.iter() {
        if !ik_target.enabled {
            continue;
        }

        let chain = &ik_target.chain;

        // Get bone positions
        let root_pos = get_bone_world_position(&creature.skeleton, chain.bones[0]);
        let mid_pos = get_bone_world_position(&creature.skeleton, chain.bones[1]);
        let end_pos = calculate_end_effector_position(...);

        // Solve IK
        let rotations = solve_two_bone_ik(
            root_pos,
            mid_pos,
            end_pos,
            chain.target,
            chain.pole_target,
        );

        // Apply rotations to bones
        apply_bone_rotation(chain.bones[0], rotations[0], ...);
        apply_bone_rotation(chain.bones[1], rotations[1], ...);
    }
}
```

## Performance Considerations

### Optimization Strategies

**Animation Sampling**:
- O(log n) keyframe lookup via binary search
- Cache last accessed keyframe for sequential access
- Pre-sort keyframes during load

**SLERP Optimization**:
- Use linear interpolation when quaternions are close (dot > 0.9995)
- Normalize quaternions only when needed
- Consider lookup tables for common angles

**State Machine**:
- Evaluate conditions lazily
- Cache frequently accessed parameters
- Limit transitions per frame

**IK Solver**:
- Run IK at lower frequency than rendering (30Hz vs 60Hz)
- Use IK only when necessary (flag-based enable/disable)
- Consider analytical solutions over iterative methods

### Expected Performance

- **Skeletal Animation Sampling**: <0.1ms per creature
- **SLERP**: ~10 nanoseconds per quaternion
- **IK Solve**: <0.1ms per chain
- **State Machine Update**: <0.01ms per creature
- **Target**: 50+ skeletal creatures at 60 FPS

## Data File Organization

```
data/
├── skeletons/
│   ├── humanoid.ron
│   ├── quadruped.ron
│   └── dragon.ron
├── animations/
│   ├── humanoid/
│   │   ├── idle.ron
│   │   ├── walk.ron
│   │   ├── run.ron
│   │   └── jump.ron
│   └── quadruped/
│       ├── idle.ron
│       └── walk.ron
└── state_machines/
    ├── character_locomotion.ron
    └── enemy_behavior.ron
```

## Best Practices

### Skeleton Design

1. **Keep bone count reasonable**: 20-50 bones for humanoids
2. **Use meaningful bone names**: "left_forearm" not "bone_23"
3. **Maintain consistent hierarchy**: Root → Spine → Limbs
4. **Validate after changes**: Run `skeleton.validate()` after edits

### Animation Authoring

1. **Use consistent keyframe timing**: Align keyframes across bones
2. **Minimize keyframe count**: Only keyframe changes, not every frame
3. **Test looping animations**: Ensure first/last keyframes match
4. **Normalize quaternions**: Prevent drift in long animations

### Blend Trees

1. **Start simple**: Use single clips, add blending later
2. **Test parameter ranges**: Verify blend space samples cover expected values
3. **Keep blend trees shallow**: Deep trees are harder to debug
4. **Use additive for overlays**: Hit reactions, aiming offsets

### State Machines

1. **One state machine per behavior**: Locomotion, Combat, Interaction
2. **Use clear state names**: "Idle", "Walking", "Sprinting"
3. **Test all transitions**: Ensure no dead-end states
4. **Limit transition count**: Too many = hard to maintain

### IK Usage

1. **Apply after animation**: IK adjusts animated pose
2. **Use pole vectors**: Control joint bend direction
3. **Clamp targets**: Keep within reachable range
4. **Blend IK influence**: Smooth enable/disable with 0-1 weight

## Troubleshooting

### Animation Not Playing

- Check state machine is in correct state
- Verify animation duration > 0
- Ensure keyframes are sorted by time
- Confirm bone IDs match skeleton

### Jerky Rotations

- Use SLERP, not LERP for quaternions
- Normalize quaternions after interpolation
- Check for flipped quaternions (dot < 0)
- Increase keyframe density for rapid rotations

### IK Not Reaching Target

- Verify target is within reach (length1 + length2)
- Check bone positions are correct
- Ensure pole vector points in valid direction
- Test with simpler target positions first

### State Machine Not Transitioning

- Print current parameter values
- Check transition conditions match parameter names
- Verify condition thresholds are correct
- Look for competing transitions (first match wins)

## Future Enhancements

### Planned Features

- **Animation Compression**: Reduce keyframe data size by 60%+
- **Procedural Animation**: Generate walk cycles, idle breathing
- **Multi-Bone IK**: 3+ bone chains with constraints
- **Animation Retargeting**: Apply animations to different skeletons
- **Motion Matching**: Data-driven animation selection
- **Ragdoll Physics**: Skeletal death animations

### Editor Features

- **Visual Skeleton Editor**: Drag-and-drop bone creation
- **Animation Timeline**: Keyframe editing with preview
- **Blend Tree Editor**: Visual node graph
- **State Machine Editor**: Visual state diagram
- **IK Gizmos**: Interactive target manipulation

## References

- **Quaternions**: Ken Shoemake, "Animating Rotation with Quaternion Curves"
- **SLERP**: https://en.wikipedia.org/wiki/Slerp
- **IK**: Andreas Aristidou, "Inverse Kinematics: A Review of Existing Techniques"
- **Animation Blending**: Lucas Kovar, "Motion Graphs"
- **State Machines**: Ian Millington, "Game AI Programming by Example"

## Conclusion

The skeletal animation system provides a robust foundation for complex character animations in Antares. By combining hierarchical bone structures, smooth quaternion interpolation, flexible blend trees, state-based control, and procedural IK adjustments, it enables high-quality character animation while maintaining data-driven flexibility and performance.
