// SPDX-FileCopyrightText: 2025 Brett Smith <xbcsmith@gmail.com>
// SPDX-License-Identifier: Apache-2.0

//! Animation state machine for managing animation states and transitions
//!
//! This module provides a finite state machine (FSM) for controlling animation
//! playback based on game state. The state machine handles transitions between
//! states based on conditions and parameters.
//!
//! # Overview
//!
//! The animation state machine supports:
//!
//! - Multiple animation states with blend trees
//! - Conditional transitions between states
//! - Parameter-based transition evaluation
//! - Transition blending with configurable duration
//! - Default state initialization
//!
//! # Examples
//!
//! ```
//! use antares::domain::visual::animation_state_machine::{
//!     AnimationStateMachine, AnimationState, Transition, TransitionCondition
//! };
//! use antares::domain::visual::blend_tree::BlendNode;
//! use std::collections::HashMap;
//!
//! // Create a simple state machine for locomotion
//! let mut state_machine = AnimationStateMachine::new("Locomotion".to_string());
//!
//! // Add idle state
//! state_machine.add_state(AnimationState {
//!     name: "Idle".to_string(),
//!     blend_tree: BlendNode::clip("IdleAnimation".to_string(), 1.0),
//! });
//!
//! // Add walk state
//! state_machine.add_state(AnimationState {
//!     name: "Walk".to_string(),
//!     blend_tree: BlendNode::clip("WalkAnimation".to_string(), 1.0),
//! });
//!
//! // Add transition from Idle to Walk when speed > 0.1
//! state_machine.add_transition(Transition {
//!     from: "Idle".to_string(),
//!     to: "Walk".to_string(),
//!     condition: TransitionCondition::GreaterThan {
//!         parameter: "speed".to_string(),
//!         threshold: 0.1,
//!     },
//!     duration: 0.3,
//! });
//!
//! state_machine.set_current_state("Idle".to_string());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::domain::visual::blend_tree::BlendNode;

/// Condition for triggering a state transition
///
/// Conditions are evaluated using runtime parameters to determine when
/// to transition between animation states.
///
/// # Examples
///
/// ```
/// use antares::domain::visual::animation_state_machine::TransitionCondition;
///
/// // Transition when speed exceeds threshold
/// let condition = TransitionCondition::GreaterThan {
///     parameter: "speed".to_string(),
///     threshold: 2.0,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransitionCondition {
    /// Always transition (immediate)
    Always,

    /// Transition when parameter > threshold
    GreaterThan {
        /// Parameter name to check
        parameter: String,

        /// Threshold value
        threshold: f32,
    },

    /// Transition when parameter < threshold
    LessThan {
        /// Parameter name to check
        parameter: String,

        /// Threshold value
        threshold: f32,
    },

    /// Transition when parameter == value (with small epsilon)
    Equal {
        /// Parameter name to check
        parameter: String,

        /// Expected value
        value: f32,
    },

    /// Transition when parameter is in range [min, max]
    InRange {
        /// Parameter name to check
        parameter: String,

        /// Minimum value (inclusive)
        min: f32,

        /// Maximum value (inclusive)
        max: f32,
    },

    /// Logical AND of multiple conditions
    And(Vec<TransitionCondition>),

    /// Logical OR of multiple conditions
    Or(Vec<TransitionCondition>),

    /// Logical NOT of a condition
    Not(Box<TransitionCondition>),
}

impl TransitionCondition {
    /// Evaluates the condition against runtime parameters
    ///
    /// # Arguments
    ///
    /// * `parameters` - Current parameter values
    ///
    /// # Returns
    ///
    /// Returns `true` if the condition is satisfied
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::TransitionCondition;
    /// use std::collections::HashMap;
    ///
    /// let condition = TransitionCondition::GreaterThan {
    ///     parameter: "speed".to_string(),
    ///     threshold: 1.0,
    /// };
    ///
    /// let mut params = HashMap::new();
    /// params.insert("speed".to_string(), 2.0);
    ///
    /// assert!(condition.evaluate(&params));
    /// ```
    pub fn evaluate(&self, parameters: &HashMap<String, f32>) -> bool {
        match self {
            TransitionCondition::Always => true,
            TransitionCondition::GreaterThan {
                parameter,
                threshold,
            } => parameters.get(parameter).is_some_and(|v| v > threshold),
            TransitionCondition::LessThan {
                parameter,
                threshold,
            } => parameters.get(parameter).is_some_and(|v| v < threshold),
            TransitionCondition::Equal { parameter, value } => parameters
                .get(parameter)
                .is_some_and(|v| (v - value).abs() < 0.001),
            TransitionCondition::InRange {
                parameter,
                min,
                max,
            } => parameters
                .get(parameter)
                .is_some_and(|v| v >= min && v <= max),
            TransitionCondition::And(conditions) => {
                conditions.iter().all(|c| c.evaluate(parameters))
            }
            TransitionCondition::Or(conditions) => {
                conditions.iter().any(|c| c.evaluate(parameters))
            }
            TransitionCondition::Not(condition) => !condition.evaluate(parameters),
        }
    }
}

/// A transition between two animation states
///
/// Defines when and how to transition from one state to another.
///
/// # Fields
///
/// * `from` - Source state name
/// * `to` - Destination state name
/// * `condition` - Condition that triggers the transition
/// * `duration` - Blend duration in seconds
///
/// # Examples
///
/// ```
/// use antares::domain::visual::animation_state_machine::{Transition, TransitionCondition};
///
/// let transition = Transition {
///     from: "Idle".to_string(),
///     to: "Walk".to_string(),
///     condition: TransitionCondition::GreaterThan {
///         parameter: "speed".to_string(),
///         threshold: 0.1,
///     },
///     duration: 0.3,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    /// Source state name
    pub from: String,

    /// Destination state name
    pub to: String,

    /// Condition for triggering this transition
    pub condition: TransitionCondition,

    /// Blend duration in seconds
    pub duration: f32,
}

impl Transition {
    /// Creates a new transition
    ///
    /// # Arguments
    ///
    /// * `from` - Source state name
    /// * `to` - Destination state name
    /// * `condition` - Transition condition
    /// * `duration` - Blend duration
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::{Transition, TransitionCondition};
    ///
    /// let transition = Transition::new(
    ///     "Idle".to_string(),
    ///     "Walk".to_string(),
    ///     TransitionCondition::Always,
    ///     0.2,
    /// );
    /// ```
    pub fn new(from: String, to: String, condition: TransitionCondition, duration: f32) -> Self {
        Self {
            from,
            to,
            condition,
            duration,
        }
    }
}

/// A single animation state
///
/// Represents a state in the animation state machine. Each state has a name
/// and an associated blend tree for animation playback.
///
/// # Fields
///
/// * `name` - State name (must be unique within the state machine)
/// * `blend_tree` - Blend tree defining animation for this state
///
/// # Examples
///
/// ```
/// use antares::domain::visual::animation_state_machine::AnimationState;
/// use antares::domain::visual::blend_tree::BlendNode;
///
/// let idle_state = AnimationState {
///     name: "Idle".to_string(),
///     blend_tree: BlendNode::clip("IdleAnimation".to_string(), 1.0),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationState {
    /// State name
    pub name: String,

    /// Blend tree for this state
    pub blend_tree: BlendNode,
}

/// Animation state machine
///
/// Manages animation states and transitions. The state machine evaluates
/// transition conditions each frame and smoothly blends between states.
///
/// # Fields
///
/// * `name` - State machine name (for debugging)
/// * `states` - All available states
/// * `transitions` - All transitions between states
/// * `current_state` - Name of the currently active state
/// * `parameters` - Runtime parameters for transition evaluation
///
/// # Examples
///
/// ```
/// use antares::domain::visual::animation_state_machine::AnimationStateMachine;
///
/// let mut state_machine = AnimationStateMachine::new("PlayerLocomotion".to_string());
/// state_machine.set_parameter("speed".to_string(), 0.0);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationStateMachine {
    /// State machine name
    pub name: String,

    /// All animation states
    pub states: HashMap<String, AnimationState>,

    /// All transitions
    pub transitions: Vec<Transition>,

    /// Current active state name
    pub current_state: String,

    /// Runtime parameters for condition evaluation
    pub parameters: HashMap<String, f32>,
}

impl AnimationStateMachine {
    /// Creates a new animation state machine
    ///
    /// # Arguments
    ///
    /// * `name` - State machine name
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::AnimationStateMachine;
    ///
    /// let state_machine = AnimationStateMachine::new("Combat".to_string());
    /// assert_eq!(state_machine.name, "Combat");
    /// ```
    pub fn new(name: String) -> Self {
        Self {
            name,
            states: HashMap::new(),
            transitions: Vec::new(),
            current_state: String::new(),
            parameters: HashMap::new(),
        }
    }

    /// Adds a state to the state machine
    ///
    /// # Arguments
    ///
    /// * `state` - State to add
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::{AnimationStateMachine, AnimationState};
    /// use antares::domain::visual::blend_tree::BlendNode;
    ///
    /// let mut state_machine = AnimationStateMachine::new("Test".to_string());
    /// state_machine.add_state(AnimationState {
    ///     name: "Idle".to_string(),
    ///     blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
    /// });
    ///
    /// assert!(state_machine.states.contains_key("Idle"));
    /// ```
    pub fn add_state(&mut self, state: AnimationState) {
        self.states.insert(state.name.clone(), state);
    }

    /// Adds a transition to the state machine
    ///
    /// # Arguments
    ///
    /// * `transition` - Transition to add
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::{AnimationStateMachine, Transition, TransitionCondition};
    ///
    /// let mut state_machine = AnimationStateMachine::new("Test".to_string());
    /// state_machine.add_transition(Transition::new(
    ///     "Idle".to_string(),
    ///     "Walk".to_string(),
    ///     TransitionCondition::Always,
    ///     0.3,
    /// ));
    ///
    /// assert_eq!(state_machine.transitions.len(), 1);
    /// ```
    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    /// Sets a parameter value
    ///
    /// # Arguments
    ///
    /// * `name` - Parameter name
    /// * `value` - Parameter value
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::AnimationStateMachine;
    ///
    /// let mut state_machine = AnimationStateMachine::new("Test".to_string());
    /// state_machine.set_parameter("speed".to_string(), 2.5);
    ///
    /// assert_eq!(state_machine.parameters.get("speed"), Some(&2.5));
    /// ```
    pub fn set_parameter(&mut self, name: String, value: f32) {
        self.parameters.insert(name, value);
    }

    /// Sets the current state
    ///
    /// # Arguments
    ///
    /// * `state_name` - Name of the state to activate
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::AnimationStateMachine;
    ///
    /// let mut state_machine = AnimationStateMachine::new("Test".to_string());
    /// state_machine.set_current_state("Idle".to_string());
    ///
    /// assert_eq!(state_machine.current_state, "Idle");
    /// ```
    pub fn set_current_state(&mut self, state_name: String) {
        self.current_state = state_name;
    }

    /// Updates the state machine and checks for transitions
    ///
    /// # Returns
    ///
    /// Returns `Some(transition)` if a transition was triggered, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::{
    ///     AnimationStateMachine, AnimationState, Transition, TransitionCondition
    /// };
    /// use antares::domain::visual::blend_tree::BlendNode;
    ///
    /// let mut state_machine = AnimationStateMachine::new("Test".to_string());
    /// state_machine.add_state(AnimationState {
    ///     name: "Idle".to_string(),
    ///     blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
    /// });
    /// state_machine.add_state(AnimationState {
    ///     name: "Walk".to_string(),
    ///     blend_tree: BlendNode::clip("Walk".to_string(), 1.0),
    /// });
    /// state_machine.add_transition(Transition::new(
    ///     "Idle".to_string(),
    ///     "Walk".to_string(),
    ///     TransitionCondition::GreaterThan {
    ///         parameter: "speed".to_string(),
    ///         threshold: 0.1,
    ///     },
    ///     0.3,
    /// ));
    ///
    /// state_machine.set_current_state("Idle".to_string());
    /// state_machine.set_parameter("speed".to_string(), 2.0);
    ///
    /// let transition = state_machine.update();
    /// assert!(transition.is_some());
    /// ```
    pub fn update(&mut self) -> Option<Transition> {
        // Find first valid transition from current state
        for transition in &self.transitions {
            if transition.from == self.current_state
                && transition.condition.evaluate(&self.parameters)
            {
                self.current_state = transition.to.clone();
                return Some(transition.clone());
            }
        }
        None
    }

    /// Validates the state machine
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, or `Err(String)` with error description
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - State machine has no states
    /// - Current state doesn't exist
    /// - Transition references non-existent states
    ///
    /// # Examples
    ///
    /// ```
    /// use antares::domain::visual::animation_state_machine::{AnimationStateMachine, AnimationState};
    /// use antares::domain::visual::blend_tree::BlendNode;
    ///
    /// let mut state_machine = AnimationStateMachine::new("Test".to_string());
    /// state_machine.add_state(AnimationState {
    ///     name: "Idle".to_string(),
    ///     blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
    /// });
    /// state_machine.set_current_state("Idle".to_string());
    ///
    /// assert!(state_machine.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.states.is_empty() {
            return Err("State machine has no states".to_string());
        }

        if !self.current_state.is_empty() && !self.states.contains_key(&self.current_state) {
            return Err(format!(
                "Current state '{}' does not exist",
                self.current_state
            ));
        }

        // Validate all transitions reference existing states
        for transition in &self.transitions {
            if !self.states.contains_key(&transition.from) {
                return Err(format!(
                    "Transition references non-existent state: '{}'",
                    transition.from
                ));
            }
            if !self.states.contains_key(&transition.to) {
                return Err(format!(
                    "Transition references non-existent state: '{}'",
                    transition.to
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::visual::blend_tree::BlendNode;

    #[test]
    fn test_transition_condition_always() {
        let condition = TransitionCondition::Always;
        let params = HashMap::new();
        assert!(condition.evaluate(&params));
    }

    #[test]
    fn test_transition_condition_greater_than() {
        let condition = TransitionCondition::GreaterThan {
            parameter: "speed".to_string(),
            threshold: 1.0,
        };

        let mut params = HashMap::new();
        params.insert("speed".to_string(), 2.0);
        assert!(condition.evaluate(&params));

        params.insert("speed".to_string(), 0.5);
        assert!(!condition.evaluate(&params));
    }

    #[test]
    fn test_transition_condition_less_than() {
        let condition = TransitionCondition::LessThan {
            parameter: "health".to_string(),
            threshold: 10.0,
        };

        let mut params = HashMap::new();
        params.insert("health".to_string(), 5.0);
        assert!(condition.evaluate(&params));

        params.insert("health".to_string(), 15.0);
        assert!(!condition.evaluate(&params));
    }

    #[test]
    fn test_transition_condition_in_range() {
        let condition = TransitionCondition::InRange {
            parameter: "angle".to_string(),
            min: -90.0,
            max: 90.0,
        };

        let mut params = HashMap::new();
        params.insert("angle".to_string(), 0.0);
        assert!(condition.evaluate(&params));

        params.insert("angle".to_string(), 100.0);
        assert!(!condition.evaluate(&params));
    }

    #[test]
    fn test_transition_condition_and() {
        let condition = TransitionCondition::And(vec![
            TransitionCondition::GreaterThan {
                parameter: "speed".to_string(),
                threshold: 1.0,
            },
            TransitionCondition::LessThan {
                parameter: "speed".to_string(),
                threshold: 5.0,
            },
        ]);

        let mut params = HashMap::new();
        params.insert("speed".to_string(), 3.0);
        assert!(condition.evaluate(&params));

        params.insert("speed".to_string(), 6.0);
        assert!(!condition.evaluate(&params));
    }

    #[test]
    fn test_transition_new() {
        let transition = Transition::new(
            "Idle".to_string(),
            "Walk".to_string(),
            TransitionCondition::Always,
            0.3,
        );

        assert_eq!(transition.from, "Idle");
        assert_eq!(transition.to, "Walk");
        assert_eq!(transition.duration, 0.3);
    }

    #[test]
    fn test_animation_state_machine_new() {
        let state_machine = AnimationStateMachine::new("Test".to_string());
        assert_eq!(state_machine.name, "Test");
        assert!(state_machine.states.is_empty());
        assert!(state_machine.transitions.is_empty());
    }

    #[test]
    fn test_animation_state_machine_add_state() {
        let mut state_machine = AnimationStateMachine::new("Test".to_string());
        state_machine.add_state(AnimationState {
            name: "Idle".to_string(),
            blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
        });

        assert_eq!(state_machine.states.len(), 1);
        assert!(state_machine.states.contains_key("Idle"));
    }

    #[test]
    fn test_animation_state_machine_set_parameter() {
        let mut state_machine = AnimationStateMachine::new("Test".to_string());
        state_machine.set_parameter("speed".to_string(), 2.5);

        assert_eq!(state_machine.parameters.get("speed"), Some(&2.5));
    }

    #[test]
    fn test_animation_state_machine_update_with_transition() {
        let mut state_machine = AnimationStateMachine::new("Test".to_string());
        state_machine.add_state(AnimationState {
            name: "Idle".to_string(),
            blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
        });
        state_machine.add_state(AnimationState {
            name: "Walk".to_string(),
            blend_tree: BlendNode::clip("Walk".to_string(), 1.0),
        });
        state_machine.add_transition(Transition::new(
            "Idle".to_string(),
            "Walk".to_string(),
            TransitionCondition::GreaterThan {
                parameter: "speed".to_string(),
                threshold: 0.1,
            },
            0.3,
        ));

        state_machine.set_current_state("Idle".to_string());
        state_machine.set_parameter("speed".to_string(), 2.0);

        let transition = state_machine.update();
        assert!(transition.is_some());
        assert_eq!(state_machine.current_state, "Walk");
    }

    #[test]
    fn test_animation_state_machine_update_no_transition() {
        let mut state_machine = AnimationStateMachine::new("Test".to_string());
        state_machine.add_state(AnimationState {
            name: "Idle".to_string(),
            blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
        });
        state_machine.set_current_state("Idle".to_string());
        state_machine.set_parameter("speed".to_string(), 0.0);

        let transition = state_machine.update();
        assert!(transition.is_none());
        assert_eq!(state_machine.current_state, "Idle");
    }

    #[test]
    fn test_animation_state_machine_validate_success() {
        let mut state_machine = AnimationStateMachine::new("Test".to_string());
        state_machine.add_state(AnimationState {
            name: "Idle".to_string(),
            blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
        });
        state_machine.set_current_state("Idle".to_string());

        assert!(state_machine.validate().is_ok());
    }

    #[test]
    fn test_animation_state_machine_validate_no_states() {
        let state_machine = AnimationStateMachine::new("Test".to_string());
        let result = state_machine.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no states"));
    }

    #[test]
    fn test_animation_state_machine_validate_invalid_current_state() {
        let mut state_machine = AnimationStateMachine::new("Test".to_string());
        state_machine.add_state(AnimationState {
            name: "Idle".to_string(),
            blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
        });
        state_machine.set_current_state("NonExistent".to_string());

        let result = state_machine.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_animation_state_machine_serialization() {
        let mut state_machine = AnimationStateMachine::new("Test".to_string());
        state_machine.add_state(AnimationState {
            name: "Idle".to_string(),
            blend_tree: BlendNode::clip("Idle".to_string(), 1.0),
        });

        let serialized = ron::to_string(&state_machine).unwrap();
        let deserialized: AnimationStateMachine = ron::from_str(&serialized).unwrap();
        assert_eq!(state_machine, deserialized);
    }
}
