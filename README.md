# 3d-bevy-template

A Bevy template for 3rd-person games: adventure, squad mechanics and strategy.
Desktop only (Windows/macOS/Linux). Levels are authored in Blender and imported
as glTF; characters and animations come from glTF-native asset packs
(see [docs/CHARACTERS.md](docs/CHARACTERS.md)). FBX-only sources such as
Mixamo are not supported — convert them to glTF in Blender yourself if you
need them.

See [ROADMAP.md](ROADMAP.md) for the build plan and current status.

## Structure

- `game/` — thin binary; composes plugins into a runnable demo game
- `template_core/` — library of Bevy plugins, one per concern

## Running

```sh
cargo run                                      # dev build (dynamic linking, fast rebuilds)
cargo build --release --no-default-features    # shippable build
```
