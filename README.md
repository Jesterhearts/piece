# Piece
Piece is a rules engine for MTG. It is currently very much a work-in-progress with no AI.

Future plans:
- A large number of cards supported.
- An AI capable of making interesting matches.

# Adding Cards
Cards are written in `textproto` format. You can see examples in the [cards directory](cards). Protos are defined in the [protos directory](src/protos).

## Formatting
There is a tool for formatting protos which can be invoked with `cargo run --bin format`. This will also perform validation on the protos to make sure they can be loaded by the engine. Please use it before attempting to submit any changes to the textprotos.

## Quirks to be aware of
- When adding restrictions, individual restrictions are AND'd together. So
  ```textproto
  restrictions {
    of_type { types{ artifact {} } }
  }
  restrictions {
    of_type { types { creature {} } }
  }
  ```
  will match anything that is _both_ an artifact and a creature. This is different from subfields in restrictions, which are OR'd together. So
  ```textproto
  restrictions {
    of_type {
      types { artifact {} }
      types { creature {} }
    }
  }
  ```
  will matching anything that is either an artifact or a creature (or both).