use antares::domain::conditions::{ConditionDefinition, ConditionEffect};
use std::collections::HashMap;

/// Conditions RON used for tests (loaded from data/conditions.ron)
const CONDITIONS_RON: &str = include_str!("../../../data/conditions.ron");

#[test]
fn test_conditions_examples_parse_success() {
    // Round-trip parse test to ensure the RON sample parses into domain types
    let parsed: Vec<ConditionDefinition> =
        ron::from_str(CONDITIONS_RON).expect("Failed to parse conditions RON");

    // Basic sanity checks
    assert!(
        parsed.len() >= 12,
        "Expected at least 12 conditions in test data"
    );
    assert!(parsed.iter().any(|c| c.id == "blind"));
    assert!(parsed.iter().any(|c| c.id == "poison"));
    assert!(parsed.iter().any(|c| c.id == "regeneration"));
    assert!(parsed.iter().any(|c| c.id == "empty_effects"));

    // Verify DOT: poison has a DamageOverTime effect with element "poison" and sides=4
    let poison_def = parsed
        .iter()
        .find(|c| c.id == "poison")
        .expect("poison condition not found");
    let poison_dot = poison_def
        .effects
        .iter()
        .find(|e| matches!(e, ConditionEffect::DamageOverTime { .. }))
        .expect("poison should have a DamageOverTime effect");
    if let ConditionEffect::DamageOverTime { damage, element } = poison_dot {
        assert_eq!(damage.sides, 4);
        assert_eq!(element.as_str(), "poison");
    } else {
        panic!("expected DamageOverTime effect for poison");
    }

    // Verify HOT: regeneration has HealOverTime with sides=4
    let regen_def = parsed
        .iter()
        .find(|c| c.id == "regeneration")
        .expect("regeneration condition not found");
    let regen_hot = regen_def
        .effects
        .iter()
        .find(|e| matches!(e, ConditionEffect::HealOverTime { .. }))
        .expect("regeneration should have a HealOverTime effect");
    if let ConditionEffect::HealOverTime { amount } = regen_hot {
        assert_eq!(amount.sides, 4);
    } else {
        panic!("expected HealOverTime effect for regeneration");
    }
}

#[test]
fn test_conditions_examples_detect_duplicates() {
    let parsed: Vec<ConditionDefinition> = ron::from_str(CONDITIONS_RON).unwrap();

    let mut counts: HashMap<String, usize> = HashMap::new();
    for c in parsed.iter() {
        *counts.entry(c.id.clone()).or_insert(0) += 1;
    }

    // dup1 should be present more than once in the test data
    assert!(
        *counts.get("dup1").unwrap_or(&0) >= 2,
        "Expected duplicate ID 'dup1' in test data"
    );
}

#[test]
fn test_empty_effects_and_max_value() {
    let parsed: Vec<ConditionDefinition> = ron::from_str(CONDITIONS_RON).unwrap();

    // Empty effects verify parse correctly with empty vector
    let empty_def = parsed
        .iter()
        .find(|c| c.id == "empty_effects")
        .expect("empty_effects not found");
    assert!(
        empty_def.effects.is_empty(),
        "empty_effects should have an empty effects list"
    );

    // Max integer value parsing
    let max_def = parsed
        .iter()
        .find(|c| c.id == "max_strength_value")
        .expect("max_strength_value not found");
    let mut found_val: Option<i16> = None;
    for e in max_def.effects.iter() {
        if let ConditionEffect::AttributeModifier {
            attribute: _,
            value,
        } = e
        {
            found_val = Some(*value);
            break;
        }
    }
    assert!(
        found_val.is_some(),
        "max_strength_value should contain an AttributeModifier effect"
    );
    assert_eq!(
        found_val.unwrap(),
        32767,
        "Expected attribute modifier value to be 32767"
    );
}
