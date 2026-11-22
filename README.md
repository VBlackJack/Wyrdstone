[Version française](README.fr.md)

# Wyrdstone

**Wyrdstone** is a Norse mythology Match-3 and fjord village reconstruction game written in **Rust** with **Bevy 0.15**, targeting **Windows** and **Android**.

## Architecture

- `crates/wyrdstone_core`: Pure Rust game logic (Match-3 board, gravity, cascades, boosters, ice plates, Fenrir chains, active tools, fjord buildings, Saga quests, i18n EN/FR, JSON save).
- `crates/wyrdstone_game`: Bevy 2D presentation layer with procedural Elder Futhark rune textures, responsive mouse + touch input, and Android `GameActivity` integration.
- `assets/locales/`: English (`en.json`) and French (`fr.json`) localization tables.
- `assets/data/`: Externalized RON data (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`).

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
