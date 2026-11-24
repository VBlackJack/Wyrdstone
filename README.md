[Version française](README.fr.md)

# Wyrdstone

**Wyrdstone** is a Norse mythology Match-3 and fjord village reconstruction game written in **Rust** with **Bevy 0.15**, targeting **Windows** and **Android**.

## Features

- **Match-3, Divine Fury & Ragnarok Overdrive**: 6 Norse elemental runes, Line Bolts, Surtr Bombs, Bifrost Shards (`Bifrost + Line/Bomb` super-combos), Ice Plates, Fenrir Chains, Frost Crates, Buried Relics, Bifrost Portals, spreading Niflheim Frost, **Niflheim Fog of War**, countdown **Cursed Runes**, live AoE tool previews (mouse & touch), late swap buffering, booster-combo smart hints, **1 Free Undo** (`Backspace`), **Fast Animations (1.5x)**, keyboard grid navigation (Arrows + `Enter`), **Valhalla Bonus** victory lightning cascade, **Colorblind badges**, **Patron Favored Runes** (+50% Fury), **Divine Fury** per Patron God (Thor, Freya, Loki, Odin), and **Ragnarok Overdrive** (2x score, +50% boss damage, frost immunity).
- **Mythological Boss Raids, Ragnarok Gauntlet & Valhalla Blitz**: Challenge **Fenrir**, **Jormungandr**, **Surtr**, and **Hel, Queen of Helheim** (2-phase encounters with **Stagger & Exposed Wards**), enter the 4-Boss **Ragnarok Gauntlet** rush with shared moves and carried Fury/Overdrive, sprint through **75s Valhalla Blitz** arcade runs (x1-x8 momentum, time extensions, Valkyrie medals), or tackle **6 zero-RNG Yggdrasil Trials** with Par Seals and instant 1-click resets.
- **Drakkar Expedition 2.0 & Living Fjord**: Push roguelite **Drakkar Expeditions** with **6 Expedition Relics**, **3 Skald's Oaths**, rotating board layouts, and **Wave-5 Bosses**; rebuild 6 upgradeable structures, unlock **Dual-Slot Amulets** with **5 Divine Synergies**, brew single-raid **War Draughts**, complete 12 Skalds' Feats and 18 Saga quests, collect 5 Relic realm crests, chase 7-day Daily Trials, and export/import atomic JSON backups.

## Architecture

- `crates/wyrdstone_core`: Pure Rust game logic (Match-3 board, gravity, portals, frost spread, cascades, boosters, Patron Gods, Ragnarok Overdrive, Boss Raids, Ragnarok Gauntlet, Valhalla Blitz, Yggdrasil Trials, Endless Expedition 2.0, Dual Amulets & Synergies, i18n EN/FR, atomic JSON save).
- `crates/wyrdstone_game`: Bevy 2D presentation layer with painted Norse artwork, modular `city_ui/` and `match3_scene/` scenes, mouse-wheel + touch scrolling, procedural WAV audio (with dedicated Boss Raid BGM), responsive mouse + touch input, and Android `NativeActivity` integration (packaged with `cargo-apk`).
- `assets/locales/`: English (`en.json`) and French (`fr.json`) localization tables.
- `assets/data/`: Externalized RON data (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`, `amulets.ron`, `collections.ron`, `tool_shop.ron`, `bosses.ron`, `achievements.ron`, `trials.ron`).

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
cargo install cargo-apk --version 0.10.0 --locked
cargo apk build -p wyrdstone_game --lib --release
```

### Release signing (Android)

The release workflow signs the APK with a keystore stored in two GitHub repository secrets:

- `ANDROID_KEYSTORE_BASE64`: the keystore file, base64 encoded.
- `ANDROID_KEYSTORE_PASSWORD`: the keystore password.

Create the keystore once and keep it safe (losing it prevents updating installed apps):

```powershell
keytool -genkeypair -v -keystore release.keystore -alias wyrdstone -keyalg RSA -keysize 2048 -validity 10000
[Convert]::ToBase64String([IO.File]::ReadAllBytes("release.keystore")) | Set-Clipboard
```

Paste the clipboard content into `ANDROID_KEYSTORE_BASE64`. The job fails with a clear error if either secret is missing. Locally, set `CARGO_APK_RELEASE_KEYSTORE` and `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` before `cargo apk build -p wyrdstone_game --lib --release`.

## License

Copyright 2026 Julien Bombled - Licensed under the [Apache License 2.0](LICENSE).
