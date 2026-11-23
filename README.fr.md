[English version](README.md)

# Wyrdstone

**Wyrdstone** est un jeu Match-3 et de reconstruction de village nordique écrit en **Rust** avec **Bevy 0.15**, conçu pour **Windows** et **Android**.

## Fonctionnalités

- **Match-3, Fureur Divine & Super-Combos** : 6 runes élémentaires nordiques, Éclairs de ligne, Bombes de Surtr, Éclats du Bifrost (super-combos `Bifrost + Éclair/Bombe`), Dalles de glace, Chaînes de Fenrir, Caisses de givre, Reliques enfouies, Portails du Bifrost, Givre rampant, **Brouillard de Niflheim** et **Runes Maudites** à compte à rebours, aperçu AoE au survol, **1 Annulation gratuite** (`Backspace`), **Animations Rapides (1,5x)**, surbrillance au glisser, navigation clavier (Flèches + `Entrée`), éclairs **Bonus Valhalla** en fin de victoire, **badges daltonisme** optionnels et jauge de **Fureur Divine** liée à votre **Dieu Patron** (Thor, Freya, Loki ou Odin) avec portraits peints et effets dédiés.
- **Raids de Boss Mythologiques & Expédition Infinie** : Affrontez **Fenrir**, **Jormungandr**, **Surtr** et **Hel, Reine de Helheim** (combats en 2 phases avec portraits peints, barre de PV, faiblesses élémentaires, ripostes et flash de dégâts), ou lancez-vous dans l'**Expédition Infinie** roguelite avec 3 bénédictions au choix entre chaque vague.
- **Fjord Vivant & Progression** : 6 bâtiments améliorables (dont la **Tour de Guet d'Heimdall**) sous des aurores boréales et braises animées, **Bénédictions Quotidiennes du Fjord**, tutoriel guidé en 3 étapes par Jarl Halvar, tributs passifs (+ Tout récolter), **Rangs de Clan à bonus passifs**, 12 Exploits du Scalde avec progression en direct, 18 quêtes de la Saga, **5 Amulettes Nordiques améliorables (Niv. 1-3)** à médaillons peints, 5 collections de Reliques avec blasons peints, Forge d'Outils Runiques, Défis du Jour procéduraux (streak 7 jours), Expéditions Infinies sauvegardées, bonus de Série de Victoires et sauvegarde JSON atomique avec Export/Import.

## Architecture

- `crates/wyrdstone_core` : logique pure en Rust (plateau Match-3, gravité, portails, givre rampant, cascades, boosters, Dieux Patrons, Raids Boss, Expédition Infinie, Rangs & Exploits, i18n EN/FR, sauvegarde JSON atomique).
- `crates/wyrdstone_game` : couche graphique Bevy 2D avec illustrations nordiques peintes, modules `city_ui/` et `match3_scene/`, défilement molette + tactile, audio WAV procédural (avec BGM dédiée Boss Raid), contrôles souris et tactile Android, et point d'entrée Android `GameActivity`.
- `assets/locales/` : fichiers de traduction anglais (`en.json`) et français (`fr.json`).
- `assets/data/` : données de jeu externalisées au format RON (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`, `amulets.ron`, `collections.ron`, `tool_shop.ron`, `bosses.ron`, `achievements.ron`).

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
