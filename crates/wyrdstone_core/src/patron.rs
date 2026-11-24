/*
 * Copyright 2026 Julien Bombled
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Patron Norse Gods and the Divine Fury gauge mechanic.

use serde::{Deserialize, Serialize};

/// Charge required to unleash a Patron God's Divine Fury power.
pub const FURY_CHARGE_MAX: u32 = 100;
/// Fury gained per cleared rune tile.
pub const FURY_PER_TILE: u32 = 5;
/// Bonus fury gained per cascade combo step beyond x1.
pub const FURY_PER_COMBO_STEP: u32 = 8;
/// Bonus fury gained per broken obstacle (ice, chain, crate).
pub const FURY_PER_OBSTACLE: u32 = 10;

/// Selectable Norse Patron deity determining the player's active Divine Fury ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PatronGod {
    /// Blasts up to 4 priority obstacle/ice/chained cells with lightning.
    #[default]
    Thor,
    /// Clears all runes of the most frequent color and grants +25% spoils for this board.
    Freya,
    /// Spawns 2 random Line/Bomb boosters on free rune cells.
    Loki,
    /// Grants +2 extra moves and places a Surtr Bomb on the board.
    Odin,
}

impl PatronGod {
    pub const ALL: [PatronGod; 4] = [
        PatronGod::Thor,
        PatronGod::Freya,
        PatronGod::Loki,
        PatronGod::Odin,
    ];

    pub const fn name_key(self) -> &'static str {
        match self {
            PatronGod::Thor => "patron.thor.name",
            PatronGod::Freya => "patron.freya.name",
            PatronGod::Loki => "patron.loki.name",
            PatronGod::Odin => "patron.odin.name",
        }
    }

    pub const fn desc_key(self) -> &'static str {
        match self {
            PatronGod::Thor => "patron.thor.desc",
            PatronGod::Freya => "patron.freya.desc",
            PatronGod::Loki => "patron.loki.desc",
            PatronGod::Odin => "patron.odin.desc",
        }
    }

    /// Elemental rune attuned to this Patron God, granting +50% bonus Divine Fury charge when cleared.
    pub const fn favored_rune(self) -> crate::board::RuneType {
        match self {
            PatronGod::Thor => crate::board::RuneType::Fire,
            PatronGod::Freya => crate::board::RuneType::Gold,
            PatronGod::Loki => crate::board::RuneType::Water,
            PatronGod::Odin => crate::board::RuneType::Stone,
        }
    }
}
