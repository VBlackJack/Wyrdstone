[English version](README.md)

# Wyrdstone

**Wyrdstone** est un jeu Match-3 et de reconstruction de village nordique écrit en **Rust** avec **Bevy 0.15**, conçu pour **Windows**, **Android** et le **Web (WASM32 / WebGL2)**.

## Jouer dans le navigateur (PC & Smartphone)

Lancez le jeu directement sans installation sur ordinateur ou mobile (zone de jeu maximisée en Portrait et Paysage mobile y compris grands smartphones, mode plein écran Portrait ou Paysage avec synchronisation `visualViewport`, tiroirs Match-3 défilables sur smartphone, gestes tactiles, manifeste PWA et sauvegarde locale automatique dans `localStorage`) :

- **GitHub Pages (Version Web de test)** : <https://vblackjack.github.io/Wyrdstone/>

## Fonctionnalités

- **Match-3, Fureur Divine & Surcharge Ragnarok** : 6 runes nordiques, Éclairs, Bombes de Surtr, Éclats du Bifrost (super-combos avec aperçu en direct des zones de fusion), Glace, Chaînes de Fenrir, Caisses de givre, Reliques enfouies, Portails du Bifrost, **Brouillard de Niflheim**, **Runes Maudites**, aperçu AoE (souris & tactile), buffer d'entrée, indices intelligents, coaching FTUE guidé, **1 Annulation gratuite**, **Animations Rapides (1,5x)**, navigation clavier, HUD adaptatif **DesktopWide** / **Paysage mobile** à panneaux latéraux + tiroirs **Portrait**, **Bonus Valhalla**, paliers d'**Aide de Valkyrie** en cas de défaite, **badges daltonisme**, **Runes Favorites du Patron** (+50% Fureur), **Autels de Dévotion (5 paliers)** par Dieu Patron, et **Surcharge Ragnarok** (Score x2, +50% dégâts Boss, immunité au givre).
- **Raids Boss, Gantelet du Ragnarok, Défi de la Semaine & Valhalla Blitz** : Affrontez **Fenrir**, **Jormungandr**, **Surtr** et **Hel, Reine de Helheim** (2 phases, interruption **Étourdissement & Vulnérabilité**), tentez le **Gantelet du Ragnarok** (Boss Rush x4 à coups partagés et Fureur/Surcharge conservées), mesurez-vous au **Défi de la Semaine** déterministe (médailles Bronze/Argent/Or/Légende), sprintez en **Valhalla Blitz (75s)** (multiplicateur x1-x8, gains de temps, gel du chrono), ou résolvez les **6 Épreuves d'Yggdrasil** (0 RNG, Sceaux Par et redémarrage en 1 clic).
- **Expédition 2.0, Endgame & Fjord Vivant** : Enchaînez l'**Expédition Infinie 2.0** (**6 Reliques**, **3 Serments du Scalde**, 6 schémas de plateau, **Boss toutes les 5 vagues**) ; entretenez la **Flamme du Valhalla**, utilisez le **Dernier Souffle**, récoltez le **Grenier Hors-Ligne** et la barre de **Réclamation rapide**, remplissez vos **Contrats du Jour** sur le **Sentier du Skalde cyclique** (cycles infinis avec +15% de récompenses par cycle jusqu'à +150% et XP excédentaire conservée), montez vos 6 bâtiments jusqu'à la **Maîtrise 5 étoiles** avec **Doubles Amulettes** et **5 Synergies Divines**, brassez des **Breuvages de Guerre**, enchaînez les **Décrets sans fin du Jarl** après la Saga (outil bonus tous les 5 décrets), rechargez en boucle les **5 Collections**, érigez la **Merveille d'Yggdrasil (10 paliers)**, accomplissez l'**Ascension des Neuf Royaumes** (Midgard, Jotunheim, Nidavellir, Vanaheim, Asgard) avec atmosphère propre et conservation des déblocages permanents, et exportez/importez vos sauvegardes JSON.

## Architecture

- `crates/wyrdstone_core` : logique pure en Rust (plateau Match-3, gravité, portails, givre rampant, cascades, boosters, Dieux Patrons, Surcharge Ragnarok, Raids Boss, Gantelet, Valhalla Blitz, Épreuves d'Yggdrasil, Expédition 2.0, Sentier du Skalde cyclique, Décrets sans fin, Merveille d'Yggdrasil, Ascension des Neuf Royaumes, Doubles Amulettes & Synergies, i18n EN/FR, sauvegarde JSON atomique).
- `crates/wyrdstone_game` : couche graphique Bevy 2D avec illustrations nordiques peintes, modules `city_ui/` et `match3_scene/`, défilement molette + tactile, audio WAV procédural (avec BGM dédiée Boss Raid), contrôles souris et tactile, intégration Android `NativeActivity` (`cargo-apk`), et cible Web WASM32 WebGL2 avec persistance `localStorage`.
- `web/` : enveloppe navigateur auto-localisée FR/EN (`web/index.html`) et manifeste PWA (`web/manifest.webmanifest`) adaptés aux écrans PC larges et smartphones avec plein écran Portrait/Paysage, synchronisation `visualViewport` / `:fullscreen`, zones sûres et verrouillage du scroll tactile.
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

### Web (WASM32 / Navigateur)

Prérequis : cible Rust `wasm32-unknown-unknown` et `wasm-bindgen-cli` aligné sur la version `wasm-bindgen` de `Cargo.lock`.

```powershell
rustup target add wasm32-unknown-unknown
cargo build --profile wasm-release --locked --target wasm32-unknown-unknown --bin wyrdstone
wasm-bindgen --out-dir dist --out-name wyrdstone --target web --no-typescript target/wasm32-unknown-unknown/wasm-release/wyrdstone.wasm
Copy-Item web/index.html dist/index.html
Copy-Item web/manifest.webmanifest dist/manifest.webmanifest
Copy-Item assets/sprites/app_icon.png dist/app_icon.png
```

Servez `dist/` via un serveur HTTP local (par exemple `python -m http.server -d dist 8080`) et ouvrez `http://localhost:8080`. Le déploiement continu vers GitHub Pages est assuré par `.github/workflows/pages.yml` à chaque push sur `main`.

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

Copyright 2026 Julien Bombled - Sous licence [Apache License 2.0](LICENSE).
