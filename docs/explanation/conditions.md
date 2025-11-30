# Conditions

We need to add conditions to Spells. For example Blind should add condition blind to a monster and Sleep would add sleep as a condition, etc... We need a set of conditions that can be used by things other than monsters. For example a pool could Heal a character or put them to Sleep. A Flaming Sword could inflict a condition of burning that inflicts 2 HP damage per round. Conditions would need durations. Light would set an area to light for a period of time. Conditions also need an area of effect. For example Cone of Cold should effect a set of tiles resembling a cone in front of the caster. Monsters in the cone would have to save vs cold damage. We also need to update the SDK UI to support the new data.

Store conditions data in a conditions.ron file so that users can easily edit and add new conditions.
