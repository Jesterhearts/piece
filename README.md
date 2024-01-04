# Piece
Piece is an **unofficial** rules engine for MTG. It is currently very much a work-in-progress with
no AI.

Future plans:
- A large number of cards supported.
- An AI capable of making interesting matches.

# Adding Cards
Cards are written in `yaml` format. You can see examples in the [cards directory](cards). The proto
schema is defined in the [protos directory](src/protos). In particular, check out the
[card](src/protos/card.proto), [effect](src/protos/effects.proto), and
[restriction](src/protos/targets.proto) definitions for all of the fields available. See the
[counter](src/counters.rs), and [keyword](src/card.rs) implementations for their definitions.

## Validation
Added cards can be validated by running `cargo run --bin validate`. This will validate that all
cards in the DB can be loaded successfully and should print helpful error messages if validation
fails.

## Quirks to be aware of
- Type (and subtype) list fields must be written as a comma separated list of types. The parser
  automatically converts these to a list of the appropriate type. This means that instead of
  writing:
  ```yaml
  types:
    - type_: !Artifact {}
    - type_: !Creature {}
  ```
  instead you write:
  ```yaml
  types: Artifact, Creature
  ```
  The same is true of mana gain abilities, which are written with the standard {W}, {U}, {B}, {R},
  {G}, {C} notation for White, Blue, Black, Red, Green, and Colorless respectively:
  ```yaml
  gain: !Specific
    # This is not comma separated, it's just a list of mana to gain.
    gain: '{W}{U}'
  gain: !Choice
    # This separates each choice with a comma
    choices: '{W}{U}, {U}{B}'
  ```
- When adding restrictions, individual restrictions are AND'd together. So
  ```yaml
  - restriction: !OfType
      types: Artifact
  - restriction: !OfType
      types: Creature
  ```
  will match anything that is _both_ an artifact and a creature. This is different from subfields in
  restrictions, which are OR'd together. So
  ```yaml
  - restriction: !OfType
      types: Artifact, Creature
  ```
  will matching anything that is either an artifact or a creature (or both).
- The yaml tags are UpperCamelCase versions of the snake_case field names for oneofs in the proto
  definitions. E.g. `battlefield_modifier` is `!BattlefieldModifier` and `modify_target` is
  `!ModifyTarget` for the `effect` oneof field in the `Effects` proto.


---
Mana symbols are sourced from: https://github.com/andrewgioia/Mana