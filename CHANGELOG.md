# Changelog

All notable changes to Wyrdstone are documented in this file.

## [Unreleased]

### Added
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
