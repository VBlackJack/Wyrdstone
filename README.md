[Version française](README.fr.md)

# Wyrdstone

**Wyrdstone** is a Norse mythology Match-3 and fjord village reconstruction game written in **Rust** with **Bevy 0.15**, targeting **Windows** and **Android**.

## Features

- **Match-3, Divine Fury & Super-Combos**: 6 Norse elemental runes, Line Bolts, Surtr Bombs, Bifrost Shards (`Bifrost + Line/Bomb` super-combos), Ice Plates, Fenrir Chains, Frost Crates, Buried Relics, Bifrost Portals, spreading Niflheim Frost, **Niflheim Fog of War**, countdown **Cursed Runes**, live AoE tool previews, **1 Free Undo** (`Backspace`), **Fast Animations (1.5x)**, drag-target highlight, keyboard grid navigation (Arrows + `Enter`), **Valhalla Bonus** victory lightning cascade, optional **Colorblind geometric badges**, and the **Divine Fury** gauge powered by your chosen **Patron God** (Thor, Freya, Loki, or Odin) with painted portraits and signature VFX.
- **Mythological Boss Raids & Endless Expedition**: Challenge **Fenrir**, **Jormungandr**, **Surtr**, and **Hel, Queen of Helheim** (2-phase encounters with painted boss portraits, elemental weaknesses, ripostes, and damage hit-flash), or push your limits in the roguelite **Endless Drakkar Expedition** with 3-choice blessings between waves.
- **Living Fjord Reconstruction**: 6 upgradeable structures (including **Heimdall's Watchtower**) under animated Aurora Borealis and rising warm embers, **Daily Fjord Blessings**, 3-step guided tutorial by Jarl Halvar, permanent clan bonuses, periodic passive tributes (+ Collect All), **Clan Ranks with passive perks**, 12 Skalds' Feats with live progress, 18 Saga quests, **5 upgradeable Nordic Amulets (Tier 1-3)** with painted medallions, 5 Relic Collections with painted realm crests, Runic Tool Forge, procedural Daily Trials with 7-day streaks, persisted Endless Expeditions, Win Streak starter boosters, and atomic JSON saves with Backup Export/Import.

## Architecture

- `crates/wyrdstone_core`: Pure Rust game logic (Match-3 board, gravity, portals, frost spread, cascades, boosters, Patron Gods, Boss Raids, Endless Expedition, Clan Ranks & Feats, i18n EN/FR, atomic JSON save).
- `crates/wyrdstone_game`: Bevy 2D presentation layer with painted Norse artwork, modular `city_ui/` and `match3_scene/` scenes, mouse-wheel + touch scrolling, procedural WAV audio (with dedicated Boss Raid BGM), responsive mouse + touch input, and Android `GameActivity` integration.
- `assets/locales/`: English (`en.json`) and French (`fr.json`) localization tables.
- `assets/data/`: Externalized RON data (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`, `amulets.ron`, `collections.ron`, `tool_shop.ron`, `bosses.ron`, `achievements.ron`).

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
