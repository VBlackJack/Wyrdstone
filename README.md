[Version française](README.fr.md)

# Wyrdstone

**Wyrdstone** is a Norse mythology Match-3 and fjord village reconstruction game written in **Rust** with **Bevy 0.15**, targeting **Windows**, **Android**, and **Web (WASM32 / WebGL2)**.

## Play in Browser (PC & Smartphone)

Play directly without installing anything on desktop or mobile (maximized Portrait and PhoneLandscape board layouts including wide smartphones, selectable Portrait/Landscape fullscreen mode with `visualViewport` sync, collapsible scrollable mobile Match-3 drawers, touch gestures, standalone PWA support, and automatic browser `localStorage` progression):

- **GitHub Pages (Live Test Build)**: <https://vblackjack.github.io/Wyrdstone/>

## Features

- **Match-3, Divine Fury & Ragnarok Overdrive**: 6 Norse elemental runes, Line Bolts, Surtr Bombs, Bifrost Shards (`Bifrost + Line/Bomb` super-combos with live drag fusion blast previews), Ice Plates, Fenrir Chains, Frost Crates, Buried Relics, Bifrost Portals, spreading Niflheim Frost, **Niflheim Fog of War**, countdown **Cursed Runes**, live AoE tool previews (mouse & touch), late swap buffering, booster-combo smart hints, guided FTUE onboarding, **1 Free Undo** (`Backspace`), **Fast Animations (1.5x)**, keyboard grid navigation (Arrows + `Enter`), adaptive **DesktopWide** / **PhoneLandscape** side-rails + **Portrait** drawer HUD, **Valhalla Bonus** victory lightning cascade, **Valkyrie's Aid** recovery tiers on defeat, **Colorblind badges**, **Patron Favored Runes** (+50% Fury), **Divine Fury** per Patron God (Thor, Freya, Loki, Odin) with **5-Tier Patron Devotion Altars**, and **Ragnarok Overdrive** (2x score, +50% boss damage, frost immunity).
- **Mythological Boss Raids, Ragnarok Gauntlet, Weekly Challenge & Valhalla Blitz**: Challenge **Fenrir**, **Jormungandr**, **Surtr**, and **Hel, Queen of Helheim** (2-phase encounters with **Stagger & Exposed Wards**), enter the 4-Boss **Ragnarok Gauntlet** rush with shared moves and carried Fury/Overdrive, compete on deterministic **Weekly Seeded Challenges** for Bronze/Silver/Gold/Legend medals, sprint through **75s Valhalla Blitz** arcade runs (x1-x8 momentum, time extensions, Valkyrie medals), or tackle **6 zero-RNG Yggdrasil Trials** with Par Seals and instant 1-click resets.
- **Drakkar Expedition 2.0, Retention Loops & Living Fjord**: Push roguelite **Drakkar Expeditions** with **6 Expedition Relics**, **3 Skald's Oaths**, rotating board layouts, and **Wave-5 Bosses**; build **Valhalla Flame** streaks, invoke **Last Stand**, claim the **Offline Harvest Vault** and **Quick-Claim** Fjord rewards bar, complete rotating **Daily Bounties** on the infinite **Cyclic Skald's Path** (+15% reward scaling per cycle up to +150% with preserved overflow XP), rebuild 6 structures up to **5-Star Building Mastery** with **Dual-Slot Amulets** and **5 Divine Synergies**, brew **War Draughts**, complete 12 Skalds' Feats and 18 Saga quests followed by infinite **Endless Jarl's Decrees** (bonus tool every 5th decree), recharge **5 Repeatable Realm Collections**, grow the **10-Stage Yggdrasil World-Tree Wonder** (+harvest & Boss damage), ascend across the **Nine Realms** (Midgard, Jotunheim, Nidavellir, Vanaheim, Asgard) with distinct realm atmospheres and preserved permanent unlocks, and export/import JSON backups.

## Architecture

- `crates/wyrdstone_core`: Pure Rust game logic (Match-3 board, gravity, portals, frost spread, cascades, boosters, Patron Gods, Ragnarok Overdrive, Boss Raids, Ragnarok Gauntlet, Valhalla Blitz, Yggdrasil Trials, Endless Expedition 2.0, Cyclic Skald Pass, Endless Decrees, Yggdrasil Wonder, Nine Realms Ascension, Dual Amulets & Synergies, i18n EN/FR, atomic JSON save).
- `crates/wyrdstone_game`: Bevy 2D presentation layer with painted Norse artwork, modular `city_ui/` and `match3_scene/` scenes, mouse-wheel + touch scrolling, procedural WAV audio (with dedicated Boss Raid BGM), responsive mouse + touch input, Android `NativeActivity` integration (`cargo-apk`), and WASM32 WebGL2 browser support with `localStorage` persistence.
- `web/`: EN/FR auto-localized browser shell (`web/index.html`) and PWA manifest (`web/manifest.webmanifest`) supporting PC widescreen and mobile Portrait/Landscape fullscreen orientation lock with `visualViewport` / `:fullscreen` sync, safe-area padding, and touch-scroll lock.
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

### Web (WASM32 / Browser)

Prerequisites: Rust target `wasm32-unknown-unknown` and `wasm-bindgen-cli` matching the `wasm-bindgen` version in `Cargo.lock`.

```powershell
rustup target add wasm32-unknown-unknown
cargo build --profile wasm-release --locked --target wasm32-unknown-unknown --bin wyrdstone
wasm-bindgen --out-dir dist --out-name wyrdstone --target web --no-typescript target/wasm32-unknown-unknown/wasm-release/wyrdstone.wasm
Copy-Item web/index.html dist/index.html
Copy-Item web/manifest.webmanifest dist/manifest.webmanifest
Copy-Item assets/sprites/app_icon.png dist/app_icon.png
```

Serve `dist/` over any local HTTP server (for example `python -m http.server -d dist 8080`) and open `http://localhost:8080`. Automated deployments to GitHub Pages run via `.github/workflows/pages.yml` on pushes to `main`.

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
