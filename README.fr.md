[English version](README.md)

# Wyrdstone

**Wyrdstone** est un jeu Match-3 et de reconstruction de village nordique écrit en **Rust** avec **Bevy 0.15**, conçu pour **Windows** et **Android**.

## Architecture

- `crates/wyrdstone_core` : logique pure en Rust (plateau Match-3, gravité, cascades, boosters, dalles de glace, chaînes de Fenrir, outils actifs, bâtiments du fjord, quêtes de la Saga, i18n EN/FR, sauvegarde JSON).
- `crates/wyrdstone_game` : couche graphique Bevy 2D avec textures procédurales de runes nordiques, contrôles souris et tactile Android, et point d'entrée Android `GameActivity`.
- `assets/locales/` : fichiers de traduction anglais (`en.json`) et français (`fr.json`).
- `assets/data/` : données de jeu externalisées au format RON (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`).

## Compilation et lancement (PowerShell)

### Tests

```powershell
cargo test --workspace
```

### Windows

```powershell
cargo run --bin wyrdstone
```

### Android (APK ARM64)

Prérequis : Android SDK + NDK (`ANDROID_HOME`, `ANDROID_NDK_HOME`), cible Rust `aarch64-linux-android` et `cargo-apk`.

```powershell
rustup target add aarch64-linux-android
cargo install cargo-apk
cargo apk build -p wyrdstone_game --release
```

## Licence

Copyright 2026 Julien Bombled - Sous licence [Apache 2.0](LICENSE).
