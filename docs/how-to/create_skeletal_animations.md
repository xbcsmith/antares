# How to Create Skeletal Animations

This guide walks you through creating skeletal animations for creatures in Antares, from defining the skeleton to authoring animations and setting up state machines.

## Prerequisites

- Basic understanding of 3D transformations
- Familiarity with RON data format
- Text editor for editing `.ron` files

## Step 1: Define a Skeleton

Create a skeleton definition file in `data/skeletons/`.

### Example: Simple Humanoid Skeleton

```ron
// data/skeletons/simple_humanoid.ron
Skeleton(
    bones: [
        // Bone 0: Root
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
        
        // Bone 1: Pelvis
        Bone(
            id: 1,
            name: "pelvis",
            parent: Some(0),
            rest_transform: MeshTransform(
                translation: [0.0, 1.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            ),
            inverse_bind_pose: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, -1.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
        
        // Bone 2: Spine
        Bone(
            id: 2,
            name: "spine",
            parent: Some(1),
            rest_transform: MeshTransform(
                translation: [0.0, 0.5, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            ),
            inverse_bind_pose: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, -1.5],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
        
        // Bone 3: Head
        Bone(
            id: 3,
            name: "head",
            parent: Some(2),
            rest_transform: MeshTransform(
                translation: [0.0, 0.8, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            ),
            inverse_bind_pose: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, -2.3],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
        
        // Bone 4: Left Upper Arm
        Bone(
            id: 4,
            name: "left_upper_arm",
            parent: Some(2),
            rest_transform: MeshTransform(
                translation: [-0.5, 0.5, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            ),
            inverse_bind_pose: [
                [1.0, 0.0, 0.0, 0.5],
                [0.0, 1.0, 0.0, -2.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
        
        // Bone 5: Left Forearm
        Bone(
            id: 5,
            name: "left_forearm",
            parent: Some(4),
            rest_transform: MeshTransform(
                translation: [0.0, -0.6, 0.0],
                rotation: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            ),
            inverse_bind_pose: [
                [1.0, 0.0, 0.0, 0.5],
                [0.0, 1.0, 0.0, -1.4],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ),
    ],
    root_bone: 0,
)
```

### Skeleton Design Tips

1. **Start with root bone** (ID 0, no parent)
2. **Number bones sequentially** (IDs match array indices)
3. **Use descriptive names** (`left_forearm`, not `bone_5`)
4. **Parent bones before children** (easier to read)
5. **Calculate inverse bind pose** from rest pose position

### Quick Inverse Bind Pose Calculation

For a bone at world position `[x, y, z]`, the inverse bind pose translation column is `[-x, -y, -z]`:

```
Bone at [0, 2.3, 0] → inverse_bind_pose translation: [0.0, -2.3, 0.0]
```

## Step 2: Create Skeletal Animations

Create animation files in `data/animations/`.

### Example: Idle Animation

```ron
// data/animations/humanoid/idle.ron
SkeletalAnimation(
    name: "Idle",
    duration: 2.0,
    bone_tracks: {
        // Spine gentle sway
        2: [
            BoneKeyframe(
                time: 0.0,
                position: [0.0, 0.5, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],  // Identity quaternion
                scale: [1.0, 1.0, 1.0],
            ),
            BoneKeyframe(
                time: 1.0,
                position: [0.0, 0.5, 0.0],
                rotation: [0.0, 0.02, 0.0, 0.9998],  // Slight rotation
                scale: [1.0, 1.0, 1.0],
            ),
            BoneKeyframe(
                time: 2.0,
                position: [0.0, 0.5, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            ),
        ],
        // Head slight bob
        3: [
            BoneKeyframe(
                time: 0.0,
                position: [0.0, 0.8, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            ),
            BoneKeyframe(
                time: 1.0,
                position: [0.0, 0.78, 0.0],  // Slightly lower
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            ),
            BoneKeyframe(
                time: 2.0,
                position: [0.0, 0.8, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            ),
        ],
    },
    looping: true,
)
```

### Example: Walk Animation

```ron
// data/animations/humanoid/walk.ron
SkeletalAnimation(
    name: "Walk",
    duration: 1.0,
    bone_tracks: {
        // Left arm swing
        4: [
            BoneKeyframe(
                time: 0.0,
                position: [-0.5, 0.5, 0.0],
                rotation: [0.17, 0.0, 0.0, 0.985],  // Swing forward
                scale: [1.0, 1.0, 1.0],
            ),
            BoneKeyframe(
                time: 0.5,
                position: [-0.5, 0.5, 0.0],
                rotation: [-0.17, 0.0, 0.0, 0.985],  // Swing back
                scale: [1.0, 1.0, 1.0],
            ),
            BoneKeyframe(
                time: 1.0,
                position: [-0.5, 0.5, 0.0],
                rotation: [0.17, 0.0, 0.0, 0.985],
                scale: [1.0, 1.0, 1.0],
            ),
        ],
    },
    looping: true,
)
```

### Animation Authoring Tips

1. **Only keyframe bones that move** (omit static bones)
2. **First and last keyframes should match** for looping animations
3. **Use quaternions for rotation** (SLERP interpolation)
4. **Keep keyframe count minimal** (only key changes)
5. **Test at different speeds** (use `speed` multiplier)

### Quaternion Cheat Sheet

```
Identity (no rotation):     [0.0, 0.0, 0.0, 1.0]
90° around Y axis:          [0.0, 0.707, 0.0, 0.707]
180° around Y axis:         [0.0, 1.0, 0.0, 0.0]
45° around X axis:          [0.383, 0.0, 0.0, 0.924]
```

## Step 3: Create Blend Trees (Optional)

Blend trees combine multiple animations for smooth transitions.

### Example: Locomotion Blend Tree

```ron
// data/blend_trees/locomotion.ron
Blend2D(
    x_param: "speed",
    y_param: "strafe",
    samples: [
        // Idle at origin
        BlendSample(
            position: Vec2(x: 0.0, y: 0.0),
            animation: AnimationClip(
                animation_name: "Idle",
                speed: 1.0,
            ),
        ),
        // Walk forward
        BlendSample(
            position: Vec2(x: 1.5, y: 0.0),
            animation: AnimationClip(
                animation_name: "Walk",
                speed: 1.0,
            ),
        ),
        // Run forward
        BlendSample(
            position: Vec2(x: 4.0, y: 0.0),
            animation: AnimationClip(
                animation_name: "Run",
                speed: 1.0,
            ),
        ),
        // Strafe left
        BlendSample(
            position: Vec2(x: 1.5, y: -1.0),
            animation: AnimationClip(
                animation_name: "StrafeLeft",
                speed: 1.0,
            ),
        ),
        // Strafe right
        BlendSample(
            position: Vec2(x: 1.5, y: 1.0),
            animation: AnimationClip(
                animation_name: "StrafeRight",
                speed: 1.0,
            ),
        ),
    ],
)
```

## Step 4: Create Animation State Machine

State machines manage transitions between animations based on game state.

### Example: Character Locomotion State Machine

```ron
// data/state_machines/character_locomotion.ron
AnimationStateMachine(
    name: "CharacterLocomotion",
    states: {
        "Idle": AnimationState(
            name: "Idle",
            blend_tree: Clip(AnimationClip(
                animation_name: "Idle",
                speed: 1.0,
            )),
        ),
        "Walk": AnimationState(
            name: "Walk",
            blend_tree: Clip(AnimationClip(
                animation_name: "Walk",
                speed: 1.0,
            )),
        ),
        "Run": AnimationState(
            name: "Run",
            blend_tree: Clip(AnimationClip(
                animation_name: "Run",
                speed: 1.0,
            )),
        ),
        "Jump": AnimationState(
            name: "Jump",
            blend_tree: Clip(AnimationClip(
                animation_name: "Jump",
                speed: 1.0,
            )),
        ),
    },
    transitions: [
        // Idle → Walk when moving
        Transition(
            from: "Idle",
            to: "Walk",
            condition: GreaterThan(
                parameter: "speed",
                threshold: 0.1,
            ),
            duration: 0.3,
        ),
        // Walk → Idle when stopped
        Transition(
            from: "Walk",
            to: "Idle",
            condition: LessThan(
                parameter: "speed",
                threshold: 0.1,
            ),
            duration: 0.3,
        ),
        // Walk → Run when sprinting
        Transition(
            from: "Walk",
            to: "Run",
            condition: GreaterThan(
                parameter: "speed",
                threshold: 3.0,
            ),
            duration: 0.2,
        ),
        // Run → Walk when slowing down
        Transition(
            from: "Run",
            to: "Walk",
            condition: LessThan(
                parameter: "speed",
                threshold: 3.0,
            ),
            duration: 0.2,
        ),
        // Any → Jump when jump pressed
        Transition(
            from: "Idle",
            to: "Jump",
            condition: GreaterThan(
                parameter: "jump_pressed",
                threshold: 0.5,
            ),
            duration: 0.1,
        ),
        Transition(
            from: "Walk",
            to: "Jump",
            condition: GreaterThan(
                parameter: "jump_pressed",
                threshold: 0.5,
            ),
            duration: 0.1,
        ),
    ],
    current_state: "Idle",
    parameters: {},
)
```

## Step 5: Use in Game Code

### Load Skeleton and Animations

```rust
use antares::domain::visual::skeleton::Skeleton;
use antares::domain::visual::skeletal_animation::SkeletalAnimation;

// Load skeleton
let skeleton_data = std::fs::read_to_string("data/skeletons/simple_humanoid.ron")?;
let skeleton: Skeleton = ron::from_str(&skeleton_data)?;

// Validate skeleton
skeleton.validate()?;

// Load animation
let anim_data = std::fs::read_to_string("data/animations/humanoid/walk.ron")?;
let walk_animation: SkeletalAnimation = ron::from_str(&anim_data)?;

// Validate animation
walk_animation.validate()?;
```

### Sample Animation at Runtime

```rust
// Sample animation at current time
let current_time = 0.5; // 0.5 seconds into animation
let bone_id = 4; // Left upper arm

if let Some(keyframe) = walk_animation.sample_bone(bone_id, current_time) {
    println!("Bone {} at time {}: pos={:?}, rot={:?}",
             bone_id, current_time, keyframe.position, keyframe.rotation);
}
```

### Use State Machine

```rust
use antares::domain::visual::animation_state_machine::AnimationStateMachine;

// Load state machine
let sm_data = std::fs::read_to_string("data/state_machines/character_locomotion.ron")?;
let mut state_machine: AnimationStateMachine = ron::from_str(&sm_data)?;

// Set initial state
state_machine.set_current_state("Idle".to_string());

// Update parameters from game state
let character_velocity = 2.5; // Units per second
state_machine.set_parameter("speed".to_string(), character_velocity);

// Check for transitions
if let Some(transition) = state_machine.update() {
    println!("Transitioning from {} to {} over {} seconds",
             transition.from, transition.to, transition.duration);
}
```

## Step 6: Add Inverse Kinematics (Optional)

IK allows procedural adjustments to animations for foot placement, hand reaching, etc.

### Define IK Chain

```rust
use antares::game::systems::ik::{IkChain, Vec3};

// Left leg IK chain (thigh + shin)
let left_leg_chain = IkChain {
    bones: [6, 7], // Thigh bone ID, shin bone ID
    target: Vec3::new(0.0, 0.0, 0.0), // Ground position under foot
    pole_target: Some(Vec3::new(0.0, 0.0, 1.0)), // Knee points forward
};
```

### Solve IK

```rust
use antares::game::systems::ik::solve_two_bone_ik;

// Get bone positions from skeleton
let thigh_pos = Vec3::new(-0.3, 1.0, 0.0);
let shin_pos = Vec3::new(-0.3, 0.5, 0.0);
let foot_pos = Vec3::new(-0.3, 0.0, 0.0);

// Solve IK to place foot on ground at different height
let ground_target = Vec3::new(-0.3, -0.1, 0.0); // 0.1 units below normal

let rotations = solve_two_bone_ik(
    thigh_pos,
    shin_pos,
    foot_pos,
    ground_target,
    left_leg_chain.pole_target,
);

// Apply rotations[0] to thigh bone
// Apply rotations[1] to shin bone
```

## Common Tasks

### Adding a New Animation

1. Create `.ron` file in `data/animations/<skeleton_type>/`
2. Define bone tracks for bones that move
3. Set duration and looping
4. Validate with `animation.validate()`
5. Test in game or preview tool

### Creating a Looping Animation

```ron
SkeletalAnimation(
    name: "IdleBreathe",
    duration: 3.0,
    bone_tracks: {
        1: [ // Torso
            // First keyframe
            BoneKeyframe(time: 0.0, position: [0.0, 1.0, 0.0], ...),
            // Middle keyframe (inhale)
            BoneKeyframe(time: 1.5, position: [0.0, 1.02, 0.0], ...),
            // Last keyframe (MUST match first for smooth loop)
            BoneKeyframe(time: 3.0, position: [0.0, 1.0, 0.0], ...),
        ],
    },
    looping: true, // Enable looping
)
```

**Important**: First and last keyframes MUST have identical transforms for seamless looping.

### Debugging Animations

```rust
// Print all bone tracks
for (bone_id, keyframes) in &animation.bone_tracks {
    println!("Bone {}: {} keyframes", bone_id, keyframes.len());
    for kf in keyframes {
        println!("  t={}: pos={:?}", kf.time, kf.position);
    }
}

// Check validation
match animation.validate() {
    Ok(()) => println!("Animation valid"),
    Err(e) => println!("Validation error: {}", e),
}

// Sample at multiple times
for t in 0..=10 {
    let time = t as f32 * 0.1;
    if let Some(kf) = animation.sample_bone(0, time) {
        println!("t={}: {:?}", time, kf.position);
    }
}
```

## Best Practices

1. **Keep skeletons simple**: 20-50 bones for humanoids
2. **Use consistent naming**: left/right prefixes, descriptive names
3. **Minimize keyframes**: Only keyframe changes, not every frame
4. **Test looping**: Ensure first/last keyframes match
5. **Validate everything**: Run `validate()` on all data
6. **Use state machines**: Better than manual animation switching
7. **Layer animations**: Use blend trees for complex movement
8. **Apply IK last**: IK adjusts after animation sampling

## Troubleshooting

### Animation doesn't play

- Check state machine is in correct state
- Verify animation duration > 0
- Ensure keyframes are sorted by time
- Confirm bone IDs exist in skeleton

### Jerky rotations

- Use quaternions, not Euler angles
- Ensure quaternions are normalized
- Add more keyframes for rapid rotations
- Check SLERP is being used (not LERP)

### IK not working

- Verify target is within reach
- Check bone positions are correct
- Ensure pole vector is valid
- Test with simple straight-line targets first

### State machine stuck

- Print parameter values
- Check condition thresholds
- Verify transition exists from current state
- Look for parameter name typos

## Next Steps

- Read [`docs/explanation/skeletal_animation.md`](../explanation/skeletal_animation.md) for system architecture
- See example animations in `data/animations/`
- Explore blend trees for complex blending
- Try procedural animation generation
- Build visual editor tools for easier authoring

## Resources

- Quaternion calculator: https://quaternions.online/
- SLERP visualization: https://www.euclideanspace.com/maths/algebra/realNormedAlgebra/quaternions/slerp/
- IK tutorial: https://www.alanzucconi.com/2017/04/10/inverse-kinematics/
