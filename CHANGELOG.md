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
- **Externalized RON Data**: Moved Skalds' Feats (`achievements.ron`), Clan Rank thresholds (`balance.ron`), and per-building passive harvest yields (`buildings.ron`) out of Rust code.

### Fixed
- **Fog of War**: Prevented invisible matches inside fog, fog-targeted Bifrost Prisms, hidden starting/Fury boosters, and shuffle deadlocks on fogged tiles.
- **Free Tools & Divine Fury**: Stopped free tools and Patron Fury from ticking Boss Riposte counters or triggering Niflheim Frost spread.
- **Eye of Odin Amulet (Tiers 1-3)**: Scaled starting boosters from 1 to 3 across upgrade tiers and updated the Amulets modal to show live scaled effects.
- **Boss & Daily Rewards**: Applied reward multipliers (Freya's Fury, Amulets, Longship Dock, Clan Rank, remaining moves) to Boss Raids and displayed streak-scaled rewards on Daily Trial victory.
- **UI & State Polish**: Cleared `active_expedition` on Retreat and Expedition Boons close/Escape, timed Boss portrait hit-flash reset, prevented nested Harvest buttons from triggering parent pin selection, synced global `M` mute shortcut with Title/City labels, removed unused `city.panel.building.cost` locale key, and avoided full board rebuilds on invalid adjacent clicks.
