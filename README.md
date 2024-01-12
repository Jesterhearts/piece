# Piece
Piece is an **unofficial** rules engine for MTG. It is currently very much a work-in-progress with
no AI.

Future plans:
- A large number of cards supported.
- An AI capable of making interesting matches.

# Adding Cards
Cards are written in `yaml` format. You can see examples in the [cards directory](cards). The proto
schema is defined in the [protos directory](piece-lib/src/protos). In particular, check out the
[card](piece-lib/src/protos/card.proto#L12), [effect](piece-lib/src/protos/effects.proto#L13), and
[restriction](piece-lib/src/protos/targets.proto#L11) definitions for all of the fields available.
See also the [counter](piece-lib/src/protos/counters.proto#L5), and
[keyword](piece-lib/src/protos/keywords.proto#L5) implementations for their definitions.

## Text-to-enum conversions
- Counters accepts the text +1/+1 and -1/-1 for P1P1 and M1M1 counters.
- Enums accept any format of Title Case, UpperCamelCase, lower case, etc. The only thing to be
  careful of is types, where the typeline needs compound types to not be space-separated. E.g.
  PowerPlant needs to be in UpperCamelCase, snake_case, SCREAMING_SNAKE, etc. Otherwise it is
  ambiguous if the types are Power and Plant or just Power Plant.

## Quirks to be aware of
- Assume most effects are pulling targets from anywhere (battlefield, graveyard, exile, etc) and use
  restrictions to narrow down the target list appropriately.
- Type, subtype, keyword, and color list fields must be written as a comma separated list of values.
  The parser automatically converts these to a list of the appropriate type. This means that you
  write:
  ```yaml
  types: Artifact, Creature
  colors: Blue
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

# Why Protos?
- The author is familiar with their usage.
- They provide a convenient one-stop location for the card schema.
- They load very fast (less than 30ms for ~24k cards from a scryfall dump on the author's laptop).
  - The two-stage build process allows the binary proto files to be exported at build time, while
    allowing changes to the yaml & proto card definitions (including changing proto tag numbers -
    normally a nono).


---
Mana symbols are sourced from: https://github.com/andrewgioia/Mana