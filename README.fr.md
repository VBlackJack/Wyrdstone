[English version](README.md)

# Wyrdstone

**Wyrdstone** est un jeu Match-3 et de reconstruction de village nordique écrit en **Rust** avec **Bevy 0.15**, conçu pour **Windows** et **Android**.

## Fonctionnalités

- **Match-3 & Super-Combos** : 6 runes élémentaires nordiques, Éclairs de ligne, Bombes de Surtr, Éclats du Bifrost (incluant les super-combos `Bifrost + Éclair/Bombe`), Dalles de glace, Chaînes de Fenrir, Caisses de givre, Reliques enfouies, Portails du Bifrost et Givre rampant de Niflheim sur les plateaux Légendaires.
- **Reconstruction du Fjord** : 5 bâtiments améliorables offrant des bonus permanents, des tributs passifs à récolter sur la carte, et un sélecteur Standard / Maîtrise / Légendaire avec notes 1 à 3 étoiles.
- **Progression & Rejouabilité** : 15 quêtes de la Saga (Chapitres 1 et 2), 5 Amulettes Nordiques, 5 collections de Reliques des Neuf Mondes liées aux lieux du fjord, Forge d'Outils Runiques, Défis du Jour procéduraux avec séries de 7 jours, et sauvegarde JSON atomique avec réglages audio persistés.

## Architecture

- `crates/wyrdstone_core` : logique pure en Rust (plateau Match-3, gravité, portails, givre rampant, cascades, boosters, outils actifs, bâtiments du fjord, tributs passifs, quêtes de la Saga, i18n EN/FR, sauvegarde JSON atomique).
- `crates/wyrdstone_game` : couche graphique Bevy 2D avec illustrations nordiques peintes, modules `city_ui/` et `match3_scene/`, audio WAV procédural, contrôles souris et tactile Android, et point d'entrée Android `GameActivity`.
- `assets/locales/` : fichiers de traduction anglais (`en.json`) et français (`fr.json`).
- `assets/data/` : données de jeu externalisées au format RON (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`, `amulets.ron`, `collections.ron`, `tool_shop.ron`).

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
