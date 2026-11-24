# Changelog

All notable changes to Wyrdstone are documented in this file.

## [Unreleased]

### Added
- **4th Mythological Boss (Hel), Daily Fjord Blessings & Collection Crests**: Added **Hel, Queen of Helheim** (`boss_hel`, Fog of War + `CurseRunes` riposte + painted portrait), deterministic UTC **Daily Fjord Blessings** (Odin's Favor, Njord's Breeze, Fenrir's Eclipse, Idunn's Grace) shown in Fjord City and applied to every board, and 5 painted Realm Collection emblems in the Collections modal.
- **Match-3 Comfort & Accessibility**: Added **Fast Animations (1.5x)** toggle in Chronicles and Match-3 Pause (persisted in `SaveData`), **1 Free Undo** per board (`Backspace` or HUD button), live drag-target cell highlight, full **keyboard grid navigation** (Arrow keys + `Enter`), and separated static (`BoardStaticEntity`) vs dynamic Match-3 sprite layers to eliminate per-step board flicker.
- **Patron God Signature VFX, Amulet Artwork & Living Fjord Embers**: Distinct Divine Fury animations per Patron God (Thor lightning bolts, Freya golden sunburst, Loki spinning violet vortex, Odin runic ring), 5 painted Nordic Amulet medallion icons, and rising warm embers on rebuilt Fjord structures.
- **Win Streak, 2-Phase Mythological Bosses & Cursed Runes**: Consecutive victories (`win_streak >= 2`) award a free starting booster; Fenrir, Jormungandr, and Surtr enrage below 50% HP with a new weakness and stronger riposte; Legendary boards and late Expedition waves feature countdown Cursed Runes.
- **Portrait Ergonomics & Targeted Fjord Updates**: Compact icon-and-count Match-3 bottom bar on narrow portrait screens (`< 760px`) preserving board size, plus in-place Endurance text updates on Fjord City avoiding full UI rebuilds.
- **Release Automation & Save Backup**: Automated `.github/workflows/release.yml` building Windows `.exe` and Android ARM64 `.apk` on `v*` tags, plus one-click JSON Backup Export/Import (`wyrdstone_backup.json`) in Chronicles.
- **Heimdall's Watchtower (6th Fjord Building)**: New cliffside beacon building (`watchtower`) with painted artwork, 3 Match-3 boards (`watchtower_1..3`), passive tributes, and +15% Divine Fury charge per tier.
- **Saga Chapter 2 Quests (`q16`-`q18`)**: New quests tied to Mythological Boss Raids, Endless Drakkar Expeditions, and Heimdall's Watchtower.
- **Clan Rank Passive Perks & Live Feat Progress**: Clan Ranks (Scout -> Huscarl -> Jarl -> High King) now grant passive spoils (+5%/+10%/+15%) and starting Divine Fury (+10/+20/+30); locked Skalds' Feats display live numeric progress (`done/target`).
- **Persisted Endless Expedition**: Active runs are saved in `SaveData` (schema v5) and can be resumed directly from Fjord City without re-spending Endurance.
- **Dedicated Boss Raid Audio & VFX**: Intense Boss Raid BGM track, distinct Divine Fury war-horn and Boss Riposte roar SFX, 2-stage `Bifrost + Line/Bomb` super-combo charge animation, and golden halo highlights on adjacent booster pairs.
- **Match-3 Pause Quick Settings**: Mute, BGM/SFX volume, and Colorblind Badges toggles directly inside the Match-3 Guide/Pause overlay.
- **UI Scrolling & Typography Guard**: Full mouse-wheel and Android touch-drag scrolling with HiDPI clamping and scroll-offset memory across all modals and the Building Inspector; automated recursive Typography Guard test (`TEST-09`).
- **Data-Driven Gameplay Tuning**: Login tribute table, daily streak scaling and tools, win-streak boosters, clan rank perks, building bonuses and upgrade perks, amulet tier steps, mastery ladder, star thresholds, daily blessings, patron fury values, Daily Trial generation, Endless Expedition pacing and boons, and boss damage/phase/curse tuning now live in `balance.ron` and the content RON files; achievements declare their unlock condition in `achievements.ron`. Bundled data is validated at startup (cross-references, ranges, reserved ids), and golden tests pin the previous numbers so the move changed no value.
- **Externalized RON Data**: Moved Skalds' Feats (`achievements.ron`), Clan Rank thresholds (`balance.ron`), and per-building passive harvest yields (`buildings.ron`) out of Rust code.

### Changed
- **Single Turn Engine**: Match-3 turn resolution (swaps, tools, Divine Fury, cascades, relic drops, end-of-turn hooks, victory boosters) now runs through one step machine in `wyrdstone_core`; the animation layer only plays its steps. A differential fuzz against the previous behaviour proves the move changed no rule. Clear order is now deterministic.

### Fixed
- **Fjord Map Buildings**: Buildings fill their sprite box again and stay anchored to their terrain spot on every window size; rebuilt buildings show their tier stars and the chapter dialogue opens after the rise animation.
- **Board Clearance**: More space is reserved under the Match-3 board so the tool bar no longer overlaps it.
- **Unwinnable Content**: `meadery_1` now targets a rune that actually spawns; Expedition wave 1 only targets runes inside its pool; Daily Trial relics never land on blocked cells. Data-feasibility tests cover every bundled level, a full year of dailies, and Expedition waves.
- **Board Dead-Ends**: The shuffle fallback is verified, swaps must form a match involving a swapped cell (hints agree), and unshufflable boards are regenerated instead of soft-locking.
- **Save Safety**: An unreadable save is never overwritten: it is copied aside, the `.bak` is tried, and the session stays read-only (with a persistent warning) while the original could still load. Saves are fsynced and a rotating `.bak` is kept.
- **Newer-Version Saves**: A save written by a newer Wyrdstone is never treated as corrupt nor overwritten: the session is read-only with a dedicated warning, and an explicit New Saga or Import first preserves the newer files (`.newer-*`, `.bak-newer-*`) and aborts if that copy fails. Loaded saves are sanitized (tiers, stars, streaks, energy clamped; resource arithmetic saturates).
- **Login Tribute**: Rolling the clock back can no longer re-claim the daily tribute.
- **Match-3 Polish**: Stale hint glows reset, Undo restores the stats of the undone move, the countdown pauses during cascades, mute refreshes the HUD, HUD texts update in place during cascades, and embers/overlays follow window resizes.
- **UI Polish**: Scrollbars only appear when content overflows, BGM volume shows real percentages, Continue is offered only when real progress exists, Reduced Motion covers city build animations, and button enabled state is explicit.
- **Android Platform**: Save path from the app's internal data dir, BGM paused and progress saved on suspend, Back button mapped to Escape (City -> Title -> quit), packaging switched to NativeActivity with full `configChanges`, signed release APK via repository secrets, and CI now builds the Android APK.
- **Endurance Regen**: Driven by the wall clock, so time spent in the background or asleep is credited.
- **Touch & UI**: Buttons fire on release so touch scrolling never buys or picks anything; notices show as a toast on the hub and Title; portrait pins no longer overlap; every chapter speaker has a portrait; Undo state refreshes; HUD clicks no longer reach the board; text on accent buttons meets 4.5:1 contrast; tappable controls are at least 44 px; composite UI strings come from locale templates; the Windows build no longer opens a console.
- **Fog of War**: Prevented invisible matches inside fog, fog-targeted Bifrost Prisms, hidden starting/Fury boosters, and shuffle deadlocks on fogged tiles.
- **Free Tools & Divine Fury**: Stopped free tools and Patron Fury from ticking Boss Riposte counters or triggering Niflheim Frost spread.
- **Eye of Odin Amulet (Tiers 1-3)**: Scaled starting boosters from 1 to 3 across upgrade tiers and updated the Amulets modal to show live scaled effects.
- **Boss & Daily Rewards**: Applied reward multipliers (Freya's Fury, Amulets, Longship Dock, Clan Rank, remaining moves) to Boss Raids and displayed streak-scaled rewards on Daily Trial victory.
- **UI & State Polish**: Cleared `active_expedition` on Retreat and Expedition Boons close/Escape, timed Boss portrait hit-flash reset, prevented nested Harvest buttons from triggering parent pin selection, synced global `M` mute shortcut with Title/City labels, removed unused `city.panel.building.cost` locale key, and avoided full board rebuilds on invalid adjacent clicks.
