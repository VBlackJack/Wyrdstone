[English version](README.md)

# Wyrdstone

**Wyrdstone** est un jeu Match-3 et de reconstruction de village nordique écrit en **Rust** avec **Bevy 0.15**, conçu pour **Windows** et **Android**.

## Fonctionnalités

- **Match-3, Fureur Divine & Surcharge Ragnarok** : 6 runes nordiques, Éclairs, Bombes de Surtr, Éclats du Bifrost (super-combos avec aperçu en direct des zones de fusion), Glace, Chaînes de Fenrir, Caisses de givre, Reliques enfouies, Portails du Bifrost, **Brouillard de Niflheim**, **Runes Maudites**, aperçu AoE (souris & tactile), buffer d'entrée, indices intelligents, coaching FTUE guidé, **1 Annulation gratuite**, **Animations Rapides (1,5x)**, navigation clavier, HUD adaptatif **DesktopWide** à doubles panneaux + portrait mobile, **Bonus Valhalla**, **badges daltonisme**, **Runes Favorites du Patron** (+50% Fureur), **Autels de Dévotion (5 paliers)** par Dieu Patron, et **Surcharge Ragnarok** (Score x2, +50% dégâts Boss, immunité au givre).
- **Raids Boss, Gantelet du Ragnarok, Défi de la Semaine & Valhalla Blitz** : Affrontez **Fenrir**, **Jormungandr**, **Surtr** et **Hel, Reine de Helheim** (2 phases, interruption **Étourdissement & Vulnérabilité**), tentez le **Gantelet du Ragnarok** (Boss Rush x4 à coups partagés et Fureur/Surcharge conservées), mesurez-vous au **Défi de la Semaine** déterministe (médailles Bronze/Argent/Or/Légende), sprintez en **Valhalla Blitz (75s)** (multiplicateur x1-x8, gains de temps, gel du chrono), ou résolvez les **6 Épreuves d'Yggdrasil** (0 RNG, Sceaux Par et redémarrage en 1 clic).
- **Expédition 2.0, Rétention & Fjord Vivant** : Enchaînez l'**Expédition Infinie 2.0** (**6 Reliques**, **3 Serments du Scalde**, 6 schémas de plateau, **Boss toutes les 5 vagues**) ; entretenez la **Flamme du Valhalla**, utilisez le **Dernier Souffle**, récoltez le **Grenier Hors-Ligne**, remplissez vos **Contrats du Jour** sur le **Sentier du Skalde** (10 paliers gratuits), progressez en **Rangs de Légende** au-delà de Haut Roi, rebâtissez 6 bâtiments avec **Doubles Amulettes** et **5 Synergies Divines**, brassez des **Breuvages de Guerre**, accomplissez 12 Exploits, 18 quêtes et 5 collections de Reliques, avec sauvegarde JSON atomique et Export/Import.

## Architecture

- `crates/wyrdstone_core` : logique pure en Rust (plateau Match-3, gravité, portails, givre rampant, cascades, boosters, Dieux Patrons, Surcharge Ragnarok, Raids Boss, Gantelet, Valhalla Blitz, Épreuves d'Yggdrasil, Expédition 2.0, Doubles Amulettes & Synergies, i18n EN/FR, sauvegarde JSON atomique).
- `crates/wyrdstone_game` : couche graphique Bevy 2D avec illustrations nordiques peintes, modules `city_ui/` et `match3_scene/`, défilement molette + tactile, audio WAV procédural (avec BGM dédiée Boss Raid), contrôles souris et tactile Android, et point d'entrée Android `NativeActivity` (empaqueté avec `cargo-apk`).
- `assets/locales/` : fichiers de traduction anglais (`en.json`) et français (`fr.json`).
- `assets/data/` : données de jeu externalisées au format RON (`balance.ron`, `buildings.ron`, `levels.ron`, `quests.ron`, `amulets.ron`, `collections.ron`, `tool_shop.ron`, `bosses.ron`, `achievements.ron`, `trials.ron`).

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
cargo install cargo-apk --version 0.10.0 --locked
cargo apk build -p wyrdstone_game --lib --release
```

### Signature de release (Android)

Le workflow de release signe l'APK avec un keystore stocké dans deux secrets du dépôt GitHub :

- `ANDROID_KEYSTORE_BASE64` : le fichier keystore, encodé en base64.
- `ANDROID_KEYSTORE_PASSWORD` : le mot de passe du keystore.

Créer le keystore une seule fois et le conserver précieusement (le perdre empêche de mettre à jour les applications installées) :

```powershell
keytool -genkeypair -v -keystore release.keystore -alias wyrdstone -keyalg RSA -keysize 2048 -validity 10000
[Convert]::ToBase64String([IO.File]::ReadAllBytes("release.keystore")) | Set-Clipboard
```

Coller le contenu du presse-papiers dans `ANDROID_KEYSTORE_BASE64`. Le job échoue avec un message clair si l'un des secrets est absent. En local, définir `CARGO_APK_RELEASE_KEYSTORE` et `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` avant `cargo apk build -p wyrdstone_game --lib --release`.

## Licence

Copyright 2026 Julien Bombled - Sous licence [Apache 2.0](LICENSE).
