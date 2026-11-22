[Version française](README.fr.md)

# Wyrdstone

**Wyrdstone** is a Norse mythology Match-3 and fjord village reconstruction game written in **Rust** with **Bevy 0.15**, targeting **Windows** and **Android**.

## Features

- **Match-3 & Super-Combos**: 6 Norse elemental runes, Line Bolts, Surtr Bombs, Bifrost Shards (including `Bifrost + Line/Bomb` super-combos), Ice Plates, Fenrir Chains, Frost Crates, Buried Relics, Bifrost Portals, and spreading Niflheim Frost on Legendary boards.
- **Fjord Reconstruction**: 5 upgradeable structures with permanent clan bonuses, periodic passive tributes directly on the map, and selectable Standard / Mastery / Legendary boards with 1-3 star ratings.
- **Progression & Replayability**: 15 Saga quests (Chapters 1 & 2), 5 Nordic Amulets, 5 Nine Realms Relic Collections linked to fjord locations, Runic Tool Forge, procedural Daily Trials with 7-day streaks, and atomic JSON saves with persisted audio settings.

## Architecture

- `crates/wyrdstone_core`: Pure Rust game logic (Match-3 board, gravity, portals, frost spread, cascades, boosters, active tools, fjord buildings, passive tributes, Saga quests, i18n EN/FR, atomic JSON save).
- `crates/wyrdstone_game`: Bevy 2D presentation layer with painted Norse artwork, modular `city_ui/` and `match3_scene/` scenes, procedural WAV audio, responsive mouse + touch input, and Android `GameActivity` integration.
- `assets/locales/`: English (`en.json`) and French (`fr.json`) localization tables.
- `assets/data/`: Externalized RON data (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`, `amulets.ron`, `collections.ron`, `tool_shop.ron`).

## Build and Run (PowerShell)

### Tests

```powershell
cargo test --workspace
```

### Windows

```powershell
cargo run --bin wyrdstone
```

### Android (ARM64 APK)

Prerequisites: Android SDK + NDK (`ANDROID_HOME`, `ANDROID_NDK_HOME`), Rust target `aarch64-linux-android`, and `cargo-apk`.

```powershell
rustup target add aarch64-linux-android
cargo install cargo-apk
cargo apk build -p wyrdstone_game --release
```

## License

Copyright 2026 Julien Bombled - Licensed under the [Apache License 2.0](LICENSE).
