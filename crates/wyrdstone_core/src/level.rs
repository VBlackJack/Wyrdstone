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

//! Level definitions, objective tracking, timed mode, frost spread, amulet modifiers, and Match-3 session state.

use crate::balance::{
    add_pct_u32, BalanceConfig, BlitzTuning, BossTuning, DailyTuning, ExpeditionTuning,
    SessionTuning,
};
use crate::board::{
    ActiveTool, Board, Booster, CellPos, MatchStepOutcome, PreparedClear, RuneType,
};
use crate::boss::{BossDef, BossState, GauntletRun, GAUNTLET_BUILDING_ID, GAUNTLET_HP_SCALE_PCT};
use crate::city::{AmuletEffect, CityBonuses, DivineSynergy, ResourceBundle, WarDraught};
use crate::expedition::{ExpeditionOath, ExpeditionRelic, ExpeditionRun, EXPEDITION_BUILDING_ID};
use crate::patron::{
    PatronGod, FURY_CHARGE_MAX, FURY_PER_COMBO_STEP, FURY_PER_OBSTACLE, FURY_PER_TILE,
};
use crate::retention::{
    ValhallaFlameTier, LAST_STAND_EXTRA_MOVES, LAST_STAND_EXTRA_SECS, MAX_PATRON_DEVOTION_TIER,
    PATRON_DEVOTION_BOSS_DMG_PCT_AT_TIER5, PATRON_DEVOTION_FURY_PCT_PER_TIER,
    PATRON_DEVOTION_SCORE_PCT_PER_TIER, PATRON_DEVOTION_START_FURY_AT_TIER3,
    WEEKLY_CHALLENGE_BUILDING_ID, WEEKLY_MEDAL_THRESHOLDS,
};
use crate::trials::TrialDef;
use crate::turn::{Turn, TurnStep};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// Building identifier carried by procedurally generated Daily Trial levels.
pub const DAILY_TRIAL_BUILDING_ID: &str = "daily_trial";
/// Building / mode identifier carried by Valhalla Blitz arcade score-attack runs.
pub const BLITZ_BUILDING_ID: &str = "valhalla_blitz";
const DAILY_SEED_SALT: u64 = 0x5759_5244_5354_4F4E;

/// Score threshold medal tier earned in Valhalla Blitz.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub enum BlitzMedal {
    #[default]
    None,
    Bronze,
    Silver,
    Gold,
    Valkyrie,
}

impl BlitzMedal {
    pub fn from_score(score: u32, tuning: &BlitzTuning) -> Self {
        if score >= tuning.valkyrie_score {
            Self::Valkyrie
        } else if score >= tuning.gold_score {
            Self::Gold
        } else if score >= tuning.silver_score {
            Self::Silver
        } else if score >= tuning.bronze_score {
            Self::Bronze
        } else {
            Self::None
        }
    }

    pub const fn name_key(self) -> &'static str {
        match self {
            Self::None => "blitz.medal.none",
            Self::Bronze => "blitz.medal.bronze",
            Self::Silver => "blitz.medal.silver",
            Self::Gold => "blitz.medal.gold",
            Self::Valkyrie => "blitz.medal.valkyrie",
        }
    }

    pub fn next_target(self, tuning: &BlitzTuning) -> Option<(Self, u32)> {
        match self {
            Self::None => Some((Self::Bronze, tuning.bronze_score)),
            Self::Bronze => Some((Self::Silver, tuning.silver_score)),
            Self::Silver => Some((Self::Gold, tuning.gold_score)),
            Self::Gold => Some((Self::Valkyrie, tuning.valkyrie_score)),
            Self::Valkyrie => None,
        }
    }

    pub const fn reward_bonus_pct(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Bronze => 50,
            Self::Silver => 100,
            Self::Gold => 175,
            Self::Valkyrie => 250,
        }
    }

    pub const fn glory_reward(self, tuning: &BlitzTuning) -> u32 {
        match self {
            Self::None => 0,
            Self::Bronze => tuning.glory_bronze,
            Self::Silver => tuning.glory_silver,
            Self::Gold => tuning.glory_gold,
            Self::Valkyrie => tuning.glory_valkyrie,
        }
    }
}
/// Preferred top-row relic columns for the relic excavation daily (2 and 3 relic variants).
const DAILY_RELIC_COLUMNS_TWO: [usize; 2] = [2, 5];
const DAILY_RELIC_COLUMNS_THREE: [usize; 3] = [1, 4, 6];
/// Extra top-row columns used when a preferred relic column is blocked.
const DAILY_RELIC_FALLBACK_COLUMNS: [usize; 6] = [1, 2, 3, 4, 5, 6];

/// Daily deterministic Fjord Blessing active for all Match-3 boards on a given UTC day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DailyBlessing {
    /// +20% Divine Fury charge rate.
    OdinFavor,
    /// +15% resource spoils on victory.
    NjordBreeze,
    /// +25% damage dealt to Mythological Bosses.
    FenrirEclipse,
    /// +1 extra move on every board.
    IdunnGrace,
}

impl DailyBlessing {
    pub const ALL: [DailyBlessing; 4] = [
        DailyBlessing::OdinFavor,
        DailyBlessing::NjordBreeze,
        DailyBlessing::FenrirEclipse,
        DailyBlessing::IdunnGrace,
    ];

    /// Deterministically picks today's Fjord Blessing from the UTC day index.
    pub const fn for_day(day_index: u64) -> Self {
        Self::ALL[(day_index as usize) % Self::ALL.len()]
    }

    pub const fn name_key(self) -> &'static str {
        match self {
            DailyBlessing::OdinFavor => "blessing.odin.name",
            DailyBlessing::NjordBreeze => "blessing.njord.name",
            DailyBlessing::FenrirEclipse => "blessing.fenrir.name",
            DailyBlessing::IdunnGrace => "blessing.idunn.name",
        }
    }

    pub const fn desc_key(self) -> &'static str {
        match self {
            DailyBlessing::OdinFavor => "blessing.odin.desc",
            DailyBlessing::NjordBreeze => "blessing.njord.desc",
            DailyBlessing::FenrirEclipse => "blessing.fenrir.desc",
            DailyBlessing::IdunnGrace => "blessing.idunn.desc",
        }
    }
}

/// Win condition attached to a Match-3 level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Objective {
    ClearIce { target: u32 },
    CollectRune { rune: RuneType, target: u32 },
    ReachScore { target: u32 },
    DropRelics { target: u32 },
}

/// Live progress snapshot for a single objective.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveProgress {
    pub objective: Objective,
    pub done: u32,
    pub target: u32,
}

impl ObjectiveProgress {
    pub fn is_complete(&self) -> bool {
        self.done >= self.target
    }
}

/// Static level definition loaded from `assets/data/levels.ron`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelDef {
    pub id: String,
    pub building_id: String,
    pub cols: usize,
    pub rows: usize,
    pub max_moves: u32,
    #[serde(default)]
    pub time_limit_seconds: Option<u32>,
    pub rune_types_count: usize,
    pub objectives: Vec<Objective>,
    pub ice_cells: Vec<(usize, usize, u8)>,
    pub chained_cells: Vec<(usize, usize)>,
    #[serde(default)]
    pub crate_cells: Vec<(usize, usize, u8)>,
    #[serde(default)]
    pub relic_cells: Vec<(usize, usize)>,
    #[serde(default)]
    pub portals: Vec<((usize, usize), (usize, usize))>,
    #[serde(default)]
    pub frost_spread: bool,
    #[serde(default)]
    pub fog_cells: Vec<(usize, usize)>,
    #[serde(default)]
    pub cursed_cells: Vec<(usize, usize, u8)>,
    pub blocked_cells: Vec<(usize, usize)>,
    pub base_reward: ResourceBundle,
}

impl LevelDef {
    /// Deterministically generates today's procedural Daily Trial board from the UTC day index.
    pub fn daily_challenge(day_index: u64) -> Self {
        Self::daily_challenge_with(day_index, &DailyTuning::default())
    }

    /// Same as [`LevelDef::daily_challenge`] but driven by the given Daily Trial tuning.
    pub fn daily_challenge_with(day_index: u64, tuning: &DailyTuning) -> Self {
        let mut rng = StdRng::seed_from_u64(day_index ^ DAILY_SEED_SALT);
        let archetype = rng.gen_range(0..4usize);
        let target_rune = RuneType::ALL[rng.gen_range(0..RuneType::ALL.len())];

        let blocked_cells = match rng.gen_range(0..3usize) {
            0 => vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            1 => vec![(0, 3), (0, 4), (7, 3), (7, 4)],
            _ => vec![(3, 0), (4, 0), (3, 7), (4, 7)],
        };

        let mut ice_cells = Vec::new();
        let mut chained_cells = Vec::new();
        let mut crate_cells = Vec::new();
        let mut relic_cells = Vec::new();
        let mut portals = Vec::new();
        let mut time_limit_seconds = None;
        let mut frost_spread = false;

        let objectives = match archetype {
            0 => {
                // Glacial fortress: clear ice + collect rune
                let ring_rows = [2usize, 5usize];
                for &r in &ring_rows {
                    for c in 2..=5 {
                        let layers = if c == 2 || c == 5 { 2 } else { 1 };
                        ice_cells.push((c, r, layers));
                    }
                }
                for r in 3..=4 {
                    ice_cells.push((2, r, 1));
                    ice_cells.push((5, r, 1));
                }
                chained_cells.push((3, 3));
                chained_cells.push((4, 4));
                crate_cells.push((1, 3, 1));
                crate_cells.push((6, 4, 1));
                frost_spread = rng.gen_bool(tuning.glacial_frost_chance);
                vec![
                    Objective::ClearIce {
                        target: ice_cells.len() as u32,
                    },
                    Objective::CollectRune {
                        rune: target_rune,
                        target: rng.gen_range(
                            tuning.glacial_collect_range.0..=tuning.glacial_collect_range.1,
                        ),
                    },
                ]
            }
            1 => {
                // Relic excavation: drop 2-3 relics through crates & ice
                let relic_count =
                    rng.gen_range(tuning.relic_count_range.0..=tuning.relic_count_range.1);
                let preferred: &[usize] = if relic_count == 2 {
                    &DAILY_RELIC_COLUMNS_TWO
                } else {
                    &DAILY_RELIC_COLUMNS_THREE
                };
                // Only free top-row cells can hold a relic; fall back to other free columns.
                let is_free = |c: usize| !blocked_cells.contains(&(c, 0));
                let mut relic_columns: Vec<usize> =
                    preferred.iter().copied().filter(|&c| is_free(c)).collect();
                for c in DAILY_RELIC_FALLBACK_COLUMNS {
                    if relic_columns.len() >= relic_count {
                        break;
                    }
                    if is_free(c) && !relic_columns.contains(&c) {
                        relic_columns.push(c);
                    }
                }
                relic_columns.truncate(relic_count);
                for c in relic_columns {
                    relic_cells.push((c, 0));
                }
                let relic_count = relic_cells.len();
                for c in 2..=5 {
                    ice_cells.push((c, 4, if c == 2 || c == 5 { 2 } else { 1 }));
                    ice_cells.push((c, 6, 1));
                }
                crate_cells.push((3, 5, 2));
                crate_cells.push((4, 5, 2));
                chained_cells.push((2, 2));
                chained_cells.push((5, 2));
                vec![
                    Objective::DropRelics {
                        target: relic_count as u32,
                    },
                    Objective::ClearIce { target: 8 },
                ]
            }
            2 => {
                // Timed Skald's Blitz: score + rune collection
                time_limit_seconds =
                    Some(rng.gen_range(tuning.blitz_time_range.0..=tuning.blitz_time_range.1));
                for &(c, r) in &[
                    (1, 2),
                    (2, 2),
                    (5, 2),
                    (6, 2),
                    (1, 5),
                    (2, 5),
                    (5, 5),
                    (6, 5),
                ] {
                    ice_cells.push((c, r, if c == 2 || c == 5 { 2 } else { 1 }));
                }
                chained_cells.extend([(3, 3), (4, 3), (3, 4), (4, 4)]);
                vec![
                    Objective::ReachScore {
                        target: rng
                            .gen_range(tuning.blitz_score_range.0..=tuning.blitz_score_range.1),
                    },
                    Objective::CollectRune {
                        rune: target_rune,
                        target: rng
                            .gen_range(tuning.blitz_collect_range.0..=tuning.blitz_collect_range.1),
                    },
                ]
            }
            _ => {
                // Bifrost Portal Trials: diagonal teleporters + score & ice
                portals.push(((1, 1), (6, 6)));
                portals.push(((6, 1), (1, 6)));
                for c in 2..=5 {
                    ice_cells.push((c, 2, 1));
                    ice_cells.push((c, 5, 2));
                }
                crate_cells.push((3, 3, 1));
                crate_cells.push((4, 4, 1));
                chained_cells.push((3, 4));
                chained_cells.push((4, 3));
                vec![
                    Objective::ClearIce { target: 8 },
                    Objective::ReachScore {
                        target: tuning.portal_score_target,
                    },
                ]
            }
        };

        Self {
            id: format!("daily_{day_index}"),
            building_id: DAILY_TRIAL_BUILDING_ID.to_string(),
            cols: 8,
            rows: 8,
            max_moves: rng.gen_range(tuning.max_moves_range.0..=tuning.max_moves_range.1),
            time_limit_seconds,
            rune_types_count: 6,
            objectives,
            ice_cells,
            chained_cells,
            crate_cells,
            relic_cells,
            portals,
            frost_spread,
            fog_cells: if archetype == 3 {
                vec![(2, 3), (3, 3), (4, 3), (5, 3), (2, 4), (5, 4)]
            } else {
                vec![]
            },
            cursed_cells: vec![],
            blocked_cells,
            base_reward: tuning.base_reward,
        }
    }

    /// Generates the 75-second Valhalla Blitz arcade score-attack board.
    pub fn valhalla_blitz_with(tuning: &BlitzTuning) -> Self {
        Self {
            id: BLITZ_BUILDING_ID.to_string(),
            building_id: BLITZ_BUILDING_ID.to_string(),
            cols: 8,
            rows: 8,
            max_moves: 999,
            time_limit_seconds: Some(tuning.duration_secs),
            rune_types_count: RuneType::ALL.len(),
            objectives: vec![Objective::ReachScore {
                target: tuning.valkyrie_score,
            }],
            ice_cells: vec![],
            chained_cells: vec![],
            crate_cells: vec![],
            relic_cells: vec![],
            portals: vec![],
            frost_spread: false,
            fog_cells: vec![],
            cursed_cells: vec![],
            blocked_cells: vec![(0, 0), (7, 0), (0, 7), (7, 7)],
            base_reward: tuning.base_reward,
        }
    }

    /// Deterministically generates the Weekly Seeded Challenge 8x8 board for a UTC week index.
    pub fn weekly_challenge(week_index: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(week_index ^ 0x5759_5244_5745_454B);
        let layout = rng.gen_range(0..3usize);
        let mut ice_cells = Vec::new();
        let mut chained_cells = Vec::new();
        let mut crate_cells = Vec::new();
        let mut portals = Vec::new();
        let blocked_cells = match layout {
            0 => {
                for r in 2..=5 {
                    ice_cells.push((r, r, if r == 3 || r == 4 { 2 } else { 1 }));
                    ice_cells.push((7 - r, r, 1));
                }
                chained_cells.extend([(3, 2), (4, 5)]);
                vec![(0, 0), (7, 0), (0, 7), (7, 7)]
            }
            1 => {
                for c in 2..=5 {
                    ice_cells.push((c, 2, 1));
                    ice_cells.push((c, 5, 2));
                }
                crate_cells.extend([(2, 3, 1), (5, 3, 1), (2, 4, 1), (5, 4, 1)]);
                chained_cells.extend([(3, 3), (4, 4)]);
                vec![(0, 3), (0, 4), (7, 3), (7, 4)]
            }
            _ => {
                portals.push(((1, 7), (6, 0)));
                portals.push(((6, 7), (1, 0)));
                for c in 2..=5 {
                    ice_cells.push((c, 3, 1));
                    ice_cells.push((c, 4, 1));
                }
                crate_cells.extend([(3, 2, 1), (4, 5, 1)]);
                vec![(3, 0), (4, 0), (3, 7), (4, 7)]
            }
        };
        Self {
            id: format!("weekly_{week_index}"),
            building_id: WEEKLY_CHALLENGE_BUILDING_ID.to_string(),
            cols: 8,
            rows: 8,
            max_moves: 28,
            time_limit_seconds: None,
            rune_types_count: 6,
            objectives: vec![Objective::ReachScore {
                target: WEEKLY_MEDAL_THRESHOLDS[0],
            }],
            ice_cells,
            chained_cells,
            crate_cells,
            relic_cells: vec![],
            portals,
            frost_spread: false,
            fog_cells: vec![],
            cursed_cells: vec![],
            blocked_cells,
            base_reward: ResourceBundle {
                wood: 110,
                stone: 110,
                gold: 150,
                mead: 25,
            },
        }
    }
}

/// Terminal state of a Match-3 level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelOutcome {
    InProgress,
    Victory,
    Defeat,
}

/// Playable Match-3 level session wrapping a board, moves/timer, score, amulet buffs, and objectives.
#[derive(Debug, Clone)]
pub struct LevelSession {
    pub level: LevelDef,
    pub board: Board,
    pub initial_moves: u32,
    pub moves_left: u32,
    pub time_left_secs: Option<f32>,
    pub score: u32,
    pub score_bonus_pct: u32,
    pub resource_bonus_pct: u32,
    pub objectives: Vec<ObjectiveProgress>,
    pub outcome: LevelOutcome,
    pub last_combo: u32,
    pub last_shuffled: bool,
    pub turn_ice_broken: u32,
    pub last_frost_spread: Option<CellPos>,
    pub patron: PatronGod,
    pub fury: u32,
    pub fury_max: u32,
    pub fury_charge_bonus_pct: u32,
    pub fury_resource_bonus_pct: u32,
    pub boss_damage_bonus_pct: u32,
    pub overdrive_charge: u32,
    pub overdrive_max: u32,
    pub overdrive_turns_left: u32,
    pub just_triggered_overdrive: bool,
    pub overdrive_ignited_this_turn: bool,
    pub session_tuning: SessionTuning,
    pub is_blitz: bool,
    pub blitz_multiplier: u32,
    pub blitz_chain_timer_secs: f32,
    pub blitz_freeze_secs: f32,
    pub blitz_bonus_time_earned: f32,
    pub blitz_tuning: BlitzTuning,
    pub is_trial: bool,
    pub trial_par_moves: u32,
    pub expedition_relics: Vec<ExpeditionRelic>,
    pub daily_blessing: Option<DailyBlessing>,
    pub boss: Option<BossState>,
    pub last_stand_used: bool,
}

impl LevelSession {
    /// Starts a new playable session without an amulet.
    pub fn start<R: Rng>(level: LevelDef, rng: &mut R) -> Self {
        Self::start_with_amulet(level, None, rng)
    }

    /// Starts a new playable session applying an optional equipped Nordic Amulet.
    pub fn start_with_amulet<R: Rng>(
        level: LevelDef,
        amulet: Option<AmuletEffect>,
        rng: &mut R,
    ) -> Self {
        Self::start_with_bonuses(level, amulet, CityBonuses::default(), rng)
    }

    /// Starts a new playable session combining equipped Amulet and rebuilt village bonuses.
    pub fn start_with_bonuses<R: Rng>(
        level: LevelDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        rng: &mut R,
    ) -> Self {
        Self::start_full(level, amulet, city_bonuses, PatronGod::default(), None, rng)
    }

    /// Starts a Mythological Boss Raid encounter.
    pub fn start_boss<R: Rng>(
        boss: BossDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        rng: &mut R,
    ) -> Self {
        Self::start_boss_tuned(
            boss,
            amulet,
            city_bonuses,
            patron,
            &BossTuning::default(),
            rng,
        )
    }

    /// Starts a Mythological Boss Raid encounter driven by the balance boss tuning.
    pub fn start_boss_with<R: Rng>(
        boss: BossDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> Self {
        Self::start_boss_tuned(boss, amulet, city_bonuses, patron, &balance.boss, rng)
    }

    fn start_boss_tuned<R: Rng>(
        boss: BossDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        tuning: &BossTuning,
        rng: &mut R,
    ) -> Self {
        let level = boss.to_level_def();
        Self::start_full(
            level,
            amulet,
            city_bonuses,
            patron,
            Some(BossState::new_with(boss, tuning)),
            rng,
        )
    }

    /// Starts an Endless Drakkar Expedition wave applying accumulated roguelite boons.
    pub fn start_expedition_wave<R: Rng>(
        run: &ExpeditionRun,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        rng: &mut R,
    ) -> Self {
        Self::start_expedition_wave_with(
            run,
            &ExpeditionTuning::default(),
            amulet,
            city_bonuses,
            patron,
            rng,
        )
    }

    /// Starts an Endless Drakkar Expedition wave driven by the given expedition tuning.
    pub fn start_expedition_wave_with<R: Rng>(
        run: &ExpeditionRun,
        tuning: &ExpeditionTuning,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        rng: &mut R,
    ) -> Self {
        let level = run.generate_wave_level_with(tuning, rng);
        let mut session = Self::start_full(level, amulet, city_bonuses, patron, None, rng);
        session.score_bonus_pct = session.score_bonus_pct.saturating_add(run.score_bonus_pct);
        session.fury = session
            .fury
            .saturating_add(run.fury_headstart)
            .min(FURY_CHARGE_MAX);
        for _ in 0..run.starting_boosters {
            if session.board.spawn_random_starting_booster(rng).is_none() {
                break;
            }
        }
        session
    }

    /// Starts an Endless Drakkar Expedition wave with Relics, Skald's Oaths, and Wave-5 Boss Raids.
    pub fn start_expedition_wave_full<R: Rng>(
        run: &ExpeditionRun,
        balance: &BalanceConfig,
        bosses: &[BossDef],
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        rng: &mut R,
    ) -> Self {
        let mut session = if run.is_boss_wave() && !bosses.is_empty() {
            let boss_idx = ((run.wave / 5).saturating_sub(1) as usize) % bosses.len();
            let mut boss = bosses[boss_idx].clone();
            let hp_pct = 60u64.saturating_add(u64::from(run.wave).saturating_mul(8));
            boss.max_hp = ((u64::from(boss.max_hp) * hp_pct) / 100).max(1_200) as u32;
            let base_lvl = run.generate_wave_level_v2(&balance.expedition, rng);
            boss.max_moves = base_lvl.max_moves;
            let mut level = boss.to_level_def();
            level.id = format!("expedition_w{}", run.wave);
            level.building_id = EXPEDITION_BUILDING_ID.to_string();
            level.base_reward = base_lvl.base_reward;
            level.frost_spread = base_lvl.frost_spread;
            if run.active_oath == Some(ExpeditionOath::BerserkerRage) {
                level.cursed_cells.extend([
                    (2, 5, balance.boss.curse_turns),
                    (5, 2, balance.boss.curse_turns),
                ]);
            }
            Self::start_full(
                level,
                amulet,
                city_bonuses,
                patron,
                Some(BossState::new_with(boss, &balance.boss)),
                rng,
            )
        } else {
            let level = run.generate_wave_level_v2(&balance.expedition, rng);
            Self::start_full(level, amulet, city_bonuses, patron, None, rng)
        };
        session.apply_session_tuning(&balance.session);
        session.score_bonus_pct = session.score_bonus_pct.saturating_add(run.score_bonus_pct);
        session.fury = session
            .fury
            .saturating_add(run.fury_headstart)
            .min(FURY_CHARGE_MAX);
        for _ in 0..run.starting_boosters {
            if session.board.spawn_random_starting_booster(rng).is_none() {
                break;
            }
        }
        session.expedition_relics = run.relics.clone();
        let mut extra_boosters = 0u32;
        for &relic in &run.relics {
            match relic {
                ExpeditionRelic::ValkyrieFeathers => {
                    session.fury_charge_bonus_pct =
                        session.fury_charge_bonus_pct.saturating_add(25);
                }
                ExpeditionRelic::SurtrCoal => {
                    let half = session.overdrive_max / 2;
                    session.add_overdrive_charge(half);
                }
                ExpeditionRelic::MimirScroll => {
                    session.resource_bonus_pct = session.resource_bonus_pct.saturating_add(15);
                    extra_boosters = extra_boosters.saturating_add(1);
                }
                ExpeditionRelic::Gjallarhorn => {
                    session.boss_damage_bonus_pct =
                        session.boss_damage_bonus_pct.saturating_add(35);
                }
                ExpeditionRelic::FrostbreakerAxe | ExpeditionRelic::BifrostCompass => {}
            }
        }
        if let Some(oath) = run.active_oath {
            match oath {
                ExpeditionOath::IronVow => {
                    session.resource_bonus_pct = session.resource_bonus_pct.saturating_add(60);
                }
                ExpeditionOath::Frostbound => {
                    session.resource_bonus_pct = session.resource_bonus_pct.saturating_add(50);
                    extra_boosters = extra_boosters.saturating_add(1);
                }
                ExpeditionOath::BerserkerRage => {
                    session.score_bonus_pct = session.score_bonus_pct.saturating_add(40);
                    session.fury_charge_bonus_pct =
                        session.fury_charge_bonus_pct.saturating_add(30);
                }
            }
        }
        for _ in 0..extra_boosters {
            if session.board.spawn_random_starting_booster(rng).is_none() {
                break;
            }
        }
        session
    }

    /// Full session constructor supporting Patron God, optional Boss Raid, amulet, and city bonuses.
    pub fn start_full<R: Rng>(
        level: LevelDef,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        boss: Option<BossState>,
        rng: &mut R,
    ) -> Self {
        let mut board = Board::new_for_level(&level, rng);
        let mut total_starting_boosters = city_bonuses.starting_boosters;
        if let Some(AmuletEffect::StartingBoosters(count)) = amulet {
            total_starting_boosters = total_starting_boosters.saturating_add(count);
        }
        for _ in 0..total_starting_boosters {
            if board.spawn_random_starting_booster(rng).is_none() {
                break;
            }
        }

        let objectives = level
            .objectives
            .iter()
            .map(|obj| {
                let target = match obj {
                    Objective::ClearIce { target }
                    | Objective::CollectRune { target, .. }
                    | Objective::ReachScore { target }
                    | Objective::DropRelics { target } => *target,
                };
                ObjectiveProgress {
                    objective: obj.clone(),
                    done: 0,
                    target,
                }
            })
            .collect();

        let amulet_moves = match amulet {
            Some(AmuletEffect::ExtraMoves(m)) => m,
            _ => 0,
        };
        let amulet_score = match amulet {
            Some(AmuletEffect::ScoreBonusPct(p)) => p,
            _ => 0,
        };
        let amulet_resources = match amulet {
            Some(AmuletEffect::ResourceBonusPct(p)) => p,
            _ => 0,
        };
        let amulet_time_pct = match amulet {
            Some(AmuletEffect::ExtraTimePct(p)) => p,
            _ => 0,
        };
        let time_left_secs = level
            .time_limit_seconds
            .map(|s| (s as f32) * (1.0 + amulet_time_pct as f32 / 100.0));
        let total_moves = level
            .max_moves
            .saturating_add(amulet_moves)
            .saturating_add(city_bonuses.extra_moves);

        Self {
            initial_moves: total_moves,
            moves_left: total_moves,
            time_left_secs,
            level,
            board,
            score: 0,
            score_bonus_pct: amulet_score.saturating_add(city_bonuses.score_bonus_pct),
            resource_bonus_pct: amulet_resources.saturating_add(city_bonuses.resource_bonus_pct),
            objectives,
            outcome: LevelOutcome::InProgress,
            last_combo: 0,
            last_shuffled: false,
            turn_ice_broken: 0,
            last_frost_spread: None,
            patron,
            fury: city_bonuses.starting_fury.min(FURY_CHARGE_MAX),
            fury_max: FURY_CHARGE_MAX,
            fury_charge_bonus_pct: city_bonuses.fury_charge_bonus_pct,
            fury_resource_bonus_pct: 0,
            boss_damage_bonus_pct: 0,
            overdrive_charge: 0,
            overdrive_max: SessionTuning::default().overdrive_charge_max.max(1),
            overdrive_turns_left: 0,
            just_triggered_overdrive: false,
            overdrive_ignited_this_turn: false,
            // Legacy/characterization sessions omit `apply_session_tuning`; live game sessions
            // attach `&balance.session` explicitly on start.
            session_tuning: SessionTuning {
                patron_favored_rune_fury_pct: 0,
                overdrive_turns: 0,
                ..SessionTuning::default()
            },
            is_blitz: false,
            blitz_multiplier: 1,
            blitz_chain_timer_secs: 0.0,
            blitz_freeze_secs: 0.0,
            blitz_bonus_time_earned: 0.0,
            blitz_tuning: BlitzTuning::default(),
            is_trial: false,
            trial_par_moves: 0,
            expedition_relics: Vec::new(),
            daily_blessing: None,
            boss,
            last_stand_used: false,
        }
    }

    /// Starts a 75-second Valhalla Blitz arcade score-attack session.
    pub fn start_valhalla_blitz_with<R: Rng>(
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> Self {
        let level = LevelDef::valhalla_blitz_with(&balance.blitz);
        let mut session = Self::start_full(level, amulet, city_bonuses, patron, None, rng);
        session.is_blitz = true;
        session.blitz_tuning = balance.blitz;
        session.apply_session_tuning(&balance.session);
        session
    }

    /// Starts a deterministic zero-RNG Yggdrasil Trial puzzle session.
    pub fn start_trial(trial: &TrialDef, balance: &BalanceConfig) -> Result<Self, String> {
        let board = trial.build_board()?;
        let level = trial.to_level_def();
        let objectives = level
            .objectives
            .iter()
            .map(|obj| {
                let target = match obj {
                    Objective::ClearIce { target }
                    | Objective::CollectRune { target, .. }
                    | Objective::ReachScore { target }
                    | Objective::DropRelics { target } => *target,
                };
                ObjectiveProgress {
                    objective: obj.clone(),
                    done: 0,
                    target,
                }
            })
            .collect();
        Ok(Self {
            initial_moves: trial.max_moves,
            moves_left: trial.max_moves,
            time_left_secs: None,
            level,
            board,
            score: 0,
            score_bonus_pct: 0,
            resource_bonus_pct: 0,
            objectives,
            outcome: LevelOutcome::InProgress,
            last_combo: 0,
            last_shuffled: false,
            turn_ice_broken: 0,
            last_frost_spread: None,
            patron: PatronGod::default(),
            fury: 0,
            fury_max: FURY_CHARGE_MAX,
            fury_charge_bonus_pct: 0,
            fury_resource_bonus_pct: 0,
            boss_damage_bonus_pct: 0,
            overdrive_charge: 0,
            overdrive_max: balance.session.overdrive_charge_max.max(1),
            overdrive_turns_left: 0,
            just_triggered_overdrive: false,
            overdrive_ignited_this_turn: false,
            session_tuning: SessionTuning {
                patron_favored_rune_fury_pct: 0,
                overdrive_turns: 0,
                ..balance.session
            },
            is_blitz: false,
            blitz_multiplier: 1,
            blitz_chain_timer_secs: 0.0,
            blitz_freeze_secs: 0.0,
            blitz_bonus_time_earned: 0.0,
            blitz_tuning: BlitzTuning::default(),
            is_trial: true,
            trial_par_moves: trial.par_moves,
            expedition_relics: Vec::new(),
            daily_blessing: None,
            boss: None,
            last_stand_used: false,
        })
    }

    /// Applies escalating Valhalla Flame win-streak starting power to a non-trial session.
    pub fn apply_valhalla_flame<R: Rng>(&mut self, tier: ValhallaFlameTier, rng: &mut R) {
        if self.is_trial || tier == ValhallaFlameTier::None {
            return;
        }
        let booster = match tier {
            ValhallaFlameTier::None => return,
            ValhallaFlameTier::Ember => {
                if rng.gen_bool(0.5) {
                    Booster::LineHorizontal
                } else {
                    Booster::LineVertical
                }
            }
            ValhallaFlameTier::Blaze => Booster::Bomb,
            ValhallaFlameTier::Bifrost | ValhallaFlameTier::Inferno => Booster::BifrostShard,
        };
        let _ = self.board.spawn_specific_starting_booster(booster, rng);
        let fury_bonus = tier.starting_fury_bonus();
        if fury_bonus > 0 {
            self.fury = self.fury.saturating_add(fury_bonus).min(self.fury_max);
        }
    }

    /// Applies permanent Patron Devotion Altar bonuses (`tier` 1..=5) to a non-trial session.
    pub fn apply_patron_devotion(&mut self, tier: u8) {
        if self.is_trial || tier == 0 {
            return;
        }
        let t = u32::from(tier.min(MAX_PATRON_DEVOTION_TIER));
        self.fury_charge_bonus_pct = self
            .fury_charge_bonus_pct
            .saturating_add(t.saturating_mul(PATRON_DEVOTION_FURY_PCT_PER_TIER));
        self.score_bonus_pct = self
            .score_bonus_pct
            .saturating_add(t.saturating_mul(PATRON_DEVOTION_SCORE_PCT_PER_TIER));
        if tier >= 3 {
            self.fury = self
                .fury
                .saturating_add(PATRON_DEVOTION_START_FURY_AT_TIER3)
                .min(self.fury_max);
        }
        if tier >= 5 {
            self.boss_damage_bonus_pct = self
                .boss_damage_bonus_pct
                .saturating_add(PATRON_DEVOTION_BOSS_DMG_PCT_AT_TIER5);
        }
    }

    /// Applies endgame Legend Rank spoils bonus percentage to a non-trial session.
    pub fn apply_legend_bonus(&mut self, extra_resource_pct: u32) {
        if !self.is_trial && extra_resource_pct > 0 {
            self.resource_bonus_pct = self.resource_bonus_pct.saturating_add(extra_resource_pct);
        }
    }

    /// Applies permanent Building Mastery spoils bonus percentage to a non-trial building session.
    pub fn apply_building_mastery_bonus(&mut self, extra_resource_pct: u32) {
        if !self.is_trial && extra_resource_pct > 0 {
            self.resource_bonus_pct = self.resource_bonus_pct.saturating_add(extra_resource_pct);
        }
    }

    /// True when the defeated player can invoke Last Stand once to continue this run.
    pub fn can_use_last_stand(&self) -> bool {
        self.outcome == LevelOutcome::Defeat
            && !self.last_stand_used
            && !self.is_trial
            && !self.is_blitz
    }

    /// Revives a defeated session once with bonus moves or time.
    pub fn apply_last_stand(&mut self, out_of_time: bool) -> bool {
        if !self.can_use_last_stand() {
            return false;
        }
        self.last_stand_used = true;
        self.outcome = LevelOutcome::InProgress;
        if out_of_time {
            if let Some(secs) = self.time_left_secs.as_mut() {
                *secs += LAST_STAND_EXTRA_SECS;
            }
            self.moves_left = self.moves_left.max(1);
        } else {
            self.moves_left = self.moves_left.saturating_add(LAST_STAND_EXTRA_MOVES);
        }
        true
    }

    /// Number of moves consumed so far in this session.
    pub fn trial_moves_used(&self) -> u32 {
        self.initial_moves.saturating_sub(self.moves_left)
    }

    /// Starts one stage of a 4-boss Ragnarok Gauntlet rush, preserving move pool, Fury, and Overdrive.
    pub fn start_gauntlet_stage<R: Rng>(
        run: &GauntletRun,
        boss_def: &BossDef,
        balance: &BalanceConfig,
        amulet: Option<AmuletEffect>,
        city_bonuses: CityBonuses,
        patron: PatronGod,
        rng: &mut R,
    ) -> Self {
        let mut scaled_boss = boss_def.clone();
        scaled_boss.max_hp =
            ((u64::from(boss_def.max_hp) * u64::from(GAUNTLET_HP_SCALE_PCT)) / 100).max(100) as u32;
        scaled_boss.max_moves = run.moves_left;
        let mut level = scaled_boss.to_level_def();
        level.id = format!("gauntlet_{}_{}", run.stage_index + 1, boss_def.id);
        level.building_id = GAUNTLET_BUILDING_ID.to_string();
        let mut session = Self::start_full(
            level,
            amulet,
            city_bonuses,
            patron,
            Some(BossState::new_with(scaled_boss, &balance.boss)),
            rng,
        );
        session.apply_session_tuning(&balance.session);
        if run.stage_index > 0 {
            session.initial_moves = run.moves_left;
            session.moves_left = run.moves_left;
            session.fury = run.carried_fury.min(session.fury_max);
            session.overdrive_turns_left = run.carried_overdrive_turns;
        }
        session
    }

    /// Captures runtime session tuning (from BalanceConfig) and aligns Overdrive / Stagger limits.
    pub fn apply_session_tuning(&mut self, tuning: &SessionTuning) {
        self.session_tuning = *tuning;
        self.overdrive_max = tuning.overdrive_charge_max.max(1);
        if let Some(boss) = self.boss.as_mut() {
            boss.stagger_enabled = true;
        }
    }

    /// Adds charge toward Ragnarok Overdrive, igniting Overdrive when reaching `overdrive_max`.
    pub fn add_overdrive_charge(&mut self, amount: u32) {
        if amount == 0
            || self.session_tuning.overdrive_turns == 0
            || self.overdrive_turns_left > 0
            || self.outcome != LevelOutcome::InProgress
        {
            return;
        }
        self.overdrive_charge = self.overdrive_charge.saturating_add(amount);
        if self.overdrive_charge >= self.overdrive_max {
            self.overdrive_charge = 0;
            self.overdrive_turns_left = self.session_tuning.overdrive_turns;
            self.just_triggered_overdrive = true;
            self.overdrive_ignited_this_turn = true;
        }
    }

    /// Decrements Ragnarok Overdrive duration at the end of a move-consuming turn.
    pub fn tick_overdrive_if_needed(&mut self) {
        if self.overdrive_ignited_this_turn {
            self.overdrive_ignited_this_turn = false;
        } else if self.overdrive_turns_left > 0 {
            self.overdrive_turns_left -= 1;
        }
    }

    /// Layers a secondary equipped Nordic Amulet effect onto an active session.
    pub fn apply_extra_amulet_effect<R: Rng>(&mut self, effect: AmuletEffect, rng: &mut R) {
        match effect {
            AmuletEffect::ResourceBonusPct(p) => {
                self.resource_bonus_pct = self.resource_bonus_pct.saturating_add(p);
            }
            AmuletEffect::ExtraMoves(m) => {
                self.initial_moves = self.initial_moves.saturating_add(m);
                self.moves_left = self.moves_left.saturating_add(m);
            }
            AmuletEffect::ScoreBonusPct(p) => {
                self.score_bonus_pct = self.score_bonus_pct.saturating_add(p);
            }
            AmuletEffect::StartingBoosters(count) => {
                for _ in 0..count {
                    if self.board.spawn_random_starting_booster(rng).is_none() {
                        break;
                    }
                }
            }
            AmuletEffect::ExtraTimePct(p) => {
                if let Some(ref mut secs) = self.time_left_secs {
                    *secs *= 1.0 + p as f32 / 100.0;
                }
            }
        }
    }

    /// Applies active Patron + Amulet Divine Synergies to an active session.
    pub fn apply_synergies<R: Rng>(&mut self, synergies: &[DivineSynergy], rng: &mut R) {
        for &syn in synergies {
            match syn {
                DivineSynergy::Thunderforge => {
                    self.boss_damage_bonus_pct = self.boss_damage_bonus_pct.saturating_add(20);
                    self.fury_charge_bonus_pct = self.fury_charge_bonus_pct.saturating_add(15);
                }
                DivineSynergy::GoldenHarvest => {
                    self.resource_bonus_pct = self.resource_bonus_pct.saturating_add(20);
                    let _ = self.board.spawn_random_starting_booster(rng);
                }
                DivineSynergy::TricksterTempo => {
                    self.initial_moves = self.initial_moves.saturating_add(2);
                    self.moves_left = self.moves_left.saturating_add(2);
                    self.score_bonus_pct = self.score_bonus_pct.saturating_add(15);
                }
                DivineSynergy::AllfatherSight => {
                    self.fury = self.fury.saturating_add(25).min(self.fury_max);
                    self.score_bonus_pct = self.score_bonus_pct.saturating_add(15);
                }
                DivineSynergy::TwinRunes => {
                    self.score_bonus_pct = self.score_bonus_pct.saturating_add(15);
                    self.resource_bonus_pct = self.resource_bonus_pct.saturating_add(15);
                    self.boss_damage_bonus_pct = self.boss_damage_bonus_pct.saturating_add(10);
                }
            }
        }
    }

    /// Applies a single-raid Runeforge War Draught onto an active session.
    pub fn apply_war_draught<R: Rng>(&mut self, draught: WarDraught, rng: &mut R) {
        match draught {
            WarDraught::MeadOfPoetry => {
                self.fury = self.fury_max;
            }
            WarDraught::ValkyrieElixir => {
                self.initial_moves = self.initial_moves.saturating_add(4);
                self.moves_left = self.moves_left.saturating_add(4);
                for _ in 0..2 {
                    if self.board.spawn_random_starting_booster(rng).is_none() {
                        break;
                    }
                }
            }
            WarDraught::SurtrBrew => {
                self.score_bonus_pct = self.score_bonus_pct.saturating_add(50);
                self.boss_damage_bonus_pct = self.boss_damage_bonus_pct.saturating_add(35);
            }
        }
    }

    /// Applies today's Daily Fjord Blessing modifier onto this session.
    pub fn apply_daily_blessing(&mut self, blessing: DailyBlessing) {
        self.apply_daily_blessing_with(blessing, &SessionTuning::default());
    }

    /// Applies today's Daily Fjord Blessing using the given session tuning.
    pub fn apply_daily_blessing_with(&mut self, blessing: DailyBlessing, tuning: &SessionTuning) {
        self.daily_blessing = Some(blessing);
        match blessing {
            DailyBlessing::OdinFavor => {
                self.fury_charge_bonus_pct = self
                    .fury_charge_bonus_pct
                    .saturating_add(tuning.blessing_odin_fury_pct);
            }
            DailyBlessing::NjordBreeze => {
                self.resource_bonus_pct = self
                    .resource_bonus_pct
                    .saturating_add(tuning.blessing_njord_resource_pct);
            }
            DailyBlessing::FenrirEclipse => {
                self.boss_damage_bonus_pct = self
                    .boss_damage_bonus_pct
                    .saturating_add(tuning.blessing_fenrir_boss_damage_pct);
            }
            DailyBlessing::IdunnGrace => {
                self.initial_moves = self
                    .initial_moves
                    .saturating_add(tuning.blessing_idunn_extra_moves);
                self.moves_left = self
                    .moves_left
                    .saturating_add(tuning.blessing_idunn_extra_moves);
            }
        }
    }

    /// Returns true when the Divine Fury gauge is fully charged and ready to unleash.
    pub fn can_trigger_fury(&self) -> bool {
        !self.is_trial && self.outcome == LevelOutcome::InProgress && self.fury >= self.fury_max
    }

    /// Unleashes the active Patron God's Divine Fury power (free, consumes no move).
    pub fn trigger_patron_fury<R: Rng>(&mut self, rng: &mut R) -> Option<PreparedClear> {
        self.trigger_patron_fury_detailed(rng).0
    }

    /// Unleashes Divine Fury, returning `(optional_clear, spawned_booster_positions)`.
    pub fn trigger_patron_fury_detailed<R: Rng>(
        &mut self,
        rng: &mut R,
    ) -> (Option<PreparedClear>, Vec<CellPos>) {
        self.trigger_patron_fury_detailed_with(&SessionTuning::default(), rng)
    }

    /// Unleashes Divine Fury using the given session tuning.
    pub fn trigger_patron_fury_detailed_with<R: Rng>(
        &mut self,
        tuning: &SessionTuning,
        rng: &mut R,
    ) -> (Option<PreparedClear>, Vec<CellPos>) {
        if !self.can_trigger_fury() {
            return (None, Vec::new());
        }
        let (prepared, spawned) = self.board.apply_patron_fury_detailed(
            self.patron,
            tuning.thor_fury_strike_cells,
            tuning.loki_fury_boosters,
            rng,
        );
        let effective = match self.patron {
            PatronGod::Thor | PatronGod::Freya => prepared.is_some(),
            PatronGod::Loki => !spawned.is_empty(),
            PatronGod::Odin => tuning.odin_fury_extra_moves > 0 || !spawned.is_empty(),
        };
        if !effective {
            return (None, Vec::new());
        }
        self.fury = 0;
        match self.patron {
            PatronGod::Freya => {
                self.fury_resource_bonus_pct = self
                    .fury_resource_bonus_pct
                    .saturating_add(tuning.freya_fury_resource_pct);
            }
            PatronGod::Odin => {
                self.moves_left = self.moves_left.saturating_add(tuning.odin_fury_extra_moves);
            }
            PatronGod::Thor | PatronGod::Loki => {}
        }
        if self.is_blitz {
            self.blitz_freeze_secs = self
                .blitz_freeze_secs
                .max(self.blitz_tuning.fury_freeze_secs);
        }
        self.add_overdrive_charge(self.session_tuning.overdrive_charge_on_fury);
        (prepared, spawned)
    }

    /// Advances the optional countdown timer and handles Valhalla Blitz clock freezes & momentum decay.
    pub fn tick_timer(&mut self, dt_secs: f32) -> bool {
        if self.outcome != LevelOutcome::InProgress {
            return false;
        }
        let mut visual_changed = false;
        if self.is_blitz {
            if self.blitz_chain_timer_secs > 0.0 {
                self.blitz_chain_timer_secs = (self.blitz_chain_timer_secs - dt_secs).max(0.0);
                if self.blitz_chain_timer_secs == 0.0 && self.blitz_multiplier > 1 {
                    self.blitz_multiplier = 1;
                    visual_changed = true;
                }
            }
            if self.blitz_freeze_secs > 0.0 {
                self.blitz_freeze_secs = (self.blitz_freeze_secs - dt_secs).max(0.0);
                if self.blitz_freeze_secs == 0.0 {
                    visual_changed = true;
                }
                return visual_changed;
            }
        }
        if let Some(ref mut remaining) = self.time_left_secs {
            let prev_whole = remaining.ceil() as u32;
            *remaining = (*remaining - dt_secs).max(0.0);
            let new_whole = remaining.ceil() as u32;
            if *remaining <= 0.0 {
                self.outcome = if self.is_blitz {
                    LevelOutcome::Victory
                } else {
                    LevelOutcome::Defeat
                };
                return true;
            }
            return visual_changed || new_whole != prev_whole;
        }
        visual_changed
    }

    /// Attempts a player swap and resolves the whole turn through the step machine (clears,
    /// relics, cascades, end-of-turn hooks, shuffle check, victory/defeat). Returns the merged
    /// outcome of every step, or an empty outcome when the swap is rejected.
    pub fn play_swap<R: Rng>(
        &mut self,
        a: CellPos,
        b: CellPos,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        match self.begin_swap(a, b) {
            Ok((turn, _)) => self.drain_turn(turn, balance, rng),
            Err(_) => MatchStepOutcome::default(),
        }
    }

    /// Applies an active tool without consuming a move, resolving it through the step machine.
    pub fn play_tool<R: Rng>(
        &mut self,
        tool: ActiveTool,
        target: Option<CellPos>,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        if tool == ActiveTool::LokiShuffle {
            return self.loki_shuffle(balance, rng);
        }
        let Some(target) = target else {
            return MatchStepOutcome::default();
        };
        match self.begin_tool(tool, target) {
            Ok((turn, _)) => self.drain_turn(turn, balance, rng),
            Err(_) => MatchStepOutcome::default(),
        }
    }

    /// Advances a turn to its end and merges the per-step outcomes into one.
    fn drain_turn<R: Rng>(
        &mut self,
        mut turn: Turn,
        balance: &BalanceConfig,
        rng: &mut R,
    ) -> MatchStepOutcome {
        let mut total = MatchStepOutcome::default();
        while let Some(step) = turn.advance(self, balance, rng) {
            match step {
                TurnStep::Cleared(report) => total.merge(&report.step),
                TurnStep::RelicsDropped(report) => total.merge(&report.step),
                TurnStep::Ended(end) => total.shuffled |= end.shuffled,
                TurnStep::ClearPending(_) => {}
            }
        }
        total
    }

    /// Decrements remaining moves by one (or bumps Blitz momentum) and resets per-turn trackers.
    pub fn consume_move(&mut self) {
        if self.is_blitz {
            if self.blitz_chain_timer_secs > 0.0 {
                self.blitz_multiplier = self
                    .blitz_multiplier
                    .saturating_add(1)
                    .min(self.blitz_tuning.max_multiplier);
            }
            self.blitz_chain_timer_secs = self.blitz_tuning.chain_window_secs;
        } else {
            self.moves_left = self.moves_left.saturating_sub(1);
        }
        self.turn_ice_broken = 0;
        self.last_frost_spread = None;
        self.just_triggered_overdrive = false;
        self.overdrive_ignited_this_turn = false;
        if let Some(boss) = self.boss.as_mut() {
            boss.reset_turn_stagger_tracking();
        }
    }

    /// Applies score and objective progress from a single animated clear step without ending the turn yet.
    pub fn apply_step_progress(&mut self, step: &MatchStepOutcome) {
        let effective_score_pct = if self.overdrive_turns_left > 0 {
            self.score_bonus_pct
                .saturating_add(self.session_tuning.overdrive_score_bonus_pct)
        } else {
            self.score_bonus_pct
        };
        let boosted_score = add_pct_u32(step.score_gained, effective_score_pct);
        let scored = if self.is_blitz {
            if step.max_combo >= 2 {
                self.blitz_multiplier = self
                    .blitz_multiplier
                    .saturating_add(1)
                    .min(self.blitz_tuning.max_multiplier);
            }
            if step.valid {
                self.blitz_chain_timer_secs = self.blitz_tuning.chain_window_secs;
            }
            if (step.max_combo >= 3 || step.boosters_used > 0)
                && self.blitz_bonus_time_earned < self.blitz_tuning.max_bonus_time_secs
            {
                let bonus = self
                    .blitz_tuning
                    .time_bonus_secs
                    .min(self.blitz_tuning.max_bonus_time_secs - self.blitz_bonus_time_earned);
                self.blitz_bonus_time_earned += bonus;
                if let Some(ref mut remaining) = self.time_left_secs {
                    *remaining += bonus;
                }
            }
            boosted_score.saturating_mul(self.blitz_multiplier.max(1))
        } else {
            boosted_score
        };
        self.score = self.score.saturating_add(scored);
        let is_relic_only_step = step.relics_dropped > 0
            && step.max_combo == 0
            && step.cleared_runes.is_empty()
            && step.ice_broken == 0
            && step.chains_broken == 0
            && step.crates_broken == 0;
        if !is_relic_only_step {
            self.last_combo = step.max_combo;
            self.last_shuffled = step.shuffled;
        }
        self.turn_ice_broken = self.turn_ice_broken.saturating_add(step.ice_broken);

        let tiles_cleared: u32 = step
            .cleared_runes
            .values()
            .copied()
            .fold(0u32, u32::saturating_add);
        let favored_tiles = step
            .cleared_runes
            .get(&self.patron.favored_rune())
            .copied()
            .unwrap_or(0);
        let favored_fury_bonus = favored_tiles
            .saturating_mul(FURY_PER_TILE)
            .saturating_mul(self.session_tuning.patron_favored_rune_fury_pct)
            / 100;
        let obstacles_broken = step
            .ice_broken
            .saturating_add(step.chains_broken)
            .saturating_add(step.crates_broken);
        if self
            .expedition_relics
            .contains(&ExpeditionRelic::FrostbreakerAxe)
            && obstacles_broken > 0
        {
            self.score = self
                .score
                .saturating_add(obstacles_broken.saturating_mul(150));
            self.add_overdrive_charge(obstacles_broken.saturating_mul(10));
        }
        let combo_bonus = step
            .max_combo
            .saturating_sub(1)
            .saturating_mul(FURY_PER_COMBO_STEP);
        let gjallarhorn_fury = if self
            .expedition_relics
            .contains(&ExpeditionRelic::Gjallarhorn)
        {
            step.boosters_used.saturating_mul(10)
        } else {
            0
        };
        let raw_fury = tiles_cleared
            .saturating_mul(FURY_PER_TILE)
            .saturating_add(favored_fury_bonus)
            .saturating_add(obstacles_broken.saturating_mul(FURY_PER_OBSTACLE))
            .saturating_add(combo_bonus)
            .saturating_add(gjallarhorn_fury);
        let fury_gain = add_pct_u32(raw_fury, self.fury_charge_bonus_pct);
        self.fury = self.fury.saturating_add(fury_gain).min(self.fury_max);

        let overdrive_boss_pct = if self.overdrive_turns_left > 0 {
            self.session_tuning.overdrive_boss_damage_pct
        } else {
            0
        };
        if !is_relic_only_step && self.overdrive_turns_left == 0 {
            let combo_charge = step
                .max_combo
                .saturating_sub(1)
                .saturating_mul(self.session_tuning.overdrive_charge_per_combo);
            let booster_charge = step
                .boosters_used
                .saturating_mul(self.session_tuning.overdrive_charge_per_booster);
            self.add_overdrive_charge(combo_charge.saturating_add(booster_charge));
        }

        if !is_relic_only_step {
            if let Some(boss) = self.boss.as_mut() {
                if boss.stagger_enabled
                    && step.boosters_used > 0
                    && !boss.is_defeated()
                    && boss.turns_until_riposte == 1
                    && !boss.turn_stagger_triggered
                {
                    boss.trigger_stagger();
                }
                boss.apply_step_damage(
                    step,
                    self.score_bonus_pct
                        .saturating_add(self.boss_damage_bonus_pct)
                        .saturating_add(overdrive_boss_pct),
                );
            }
        }

        for prog in &mut self.objectives {
            match &prog.objective {
                Objective::ClearIce { .. } => {
                    prog.done = prog.done.saturating_add(step.ice_broken).min(prog.target);
                }
                Objective::CollectRune { rune, .. } => {
                    if let Some(&count) = step.cleared_runes.get(rune) {
                        prog.done = prog.done.saturating_add(count).min(prog.target);
                    }
                }
                Objective::ReachScore { .. } => {
                    if let Some(boss) = &self.boss {
                        prog.done = boss.def.max_hp.saturating_sub(boss.hp).min(prog.target);
                    } else {
                        prog.done = self.score.min(prog.target);
                    }
                }
                Objective::DropRelics { .. } => {
                    prog.done = prog
                        .done
                        .saturating_add(step.relics_dropped)
                        .min(prog.target);
                }
            }
        }
    }

    /// Executes the boss counter-attack countdown at end of turn if a Boss Raid is active.
    pub fn tick_boss_riposte_if_needed<R: Rng>(&mut self, rng: &mut R) -> Vec<CellPos> {
        if let Some(boss) = self.boss.as_mut() {
            boss.tick_turn_and_riposte(&mut self.board, rng)
        } else {
            Vec::new()
        }
    }

    /// Ticks Cursed Rune countdowns at the end of a player turn unless the board is already won.
    pub fn tick_cursed_runes_if_needed(&mut self) -> Vec<CellPos> {
        let already_won = if let Some(boss) = &self.boss {
            boss.is_defeated()
        } else {
            self.objectives.iter().all(ObjectiveProgress::is_complete)
        };
        if already_won {
            return Vec::new();
        }
        self.board.tick_cursed_runes()
    }

    /// On frost-spread levels, freezes 1 adjacent cell if no ice was broken during this turn
    /// and Ragnarok Overdrive is not burning off the frost.
    pub fn apply_frost_spread_if_needed<R: Rng>(&mut self, rng: &mut R) -> Option<CellPos> {
        if !self.level.frost_spread
            || self.overdrive_turns_left > 0
            || self.turn_ice_broken > 0
            || self.objectives.iter().all(ObjectiveProgress::is_complete)
        {
            return None;
        }
        let spread = self.board.spread_frost(rng);
        self.last_frost_spread = spread;
        spread
    }

    /// Evaluates victory or defeat after all cascades of a turn have finished.
    pub fn finalize_turn(&mut self, balance: &BalanceConfig) {
        if let Some(boss) = self.boss.as_mut() {
            boss.just_staggered = false;
        }
        if self.outcome != LevelOutcome::InProgress || self.is_blitz {
            return;
        }
        let won = if let Some(boss) = &self.boss {
            boss.is_defeated()
        } else {
            self.objectives.iter().all(ObjectiveProgress::is_complete)
        };
        if won {
            self.score = self
                .score
                .saturating_add(self.moves_left.saturating_mul(balance.remaining_move_bonus));
            self.outcome = LevelOutcome::Victory;
        } else if self.moves_left == 0 {
            self.outcome = LevelOutcome::Defeat;
        }
    }

    /// Computes a 1..=3 star rating for a completed victory based on remaining moves efficiency.
    pub fn earned_stars(&self) -> u8 {
        self.earned_stars_with(&SessionTuning::default())
    }

    /// Computes the star rating using the given session tuning thresholds (or Blitz medal tier).
    pub fn earned_stars_with(&self, tuning: &SessionTuning) -> u8 {
        if self.outcome != LevelOutcome::Victory {
            return 0;
        }
        if self.is_blitz {
            return match BlitzMedal::from_score(self.score, &self.blitz_tuning) {
                BlitzMedal::Gold | BlitzMedal::Valkyrie => 3,
                BlitzMedal::Silver => 2,
                BlitzMedal::Bronze | BlitzMedal::None => 1,
            };
        }
        let ratio = if self.initial_moves > 0 {
            self.moves_left as f32 / self.initial_moves as f32
        } else {
            0.0
        };
        if ratio >= tuning.three_star_moves_ratio {
            3
        } else if ratio >= tuning.two_star_moves_ratio {
            2
        } else {
            1
        }
    }

    /// Returns `(move_or_medal_bonus_pct, passive_bonus_pct, fury_bonus_pct)` applied to victory rewards.
    pub fn reward_bonus_breakdown(&self, balance: &BalanceConfig) -> (u32, u32, u32) {
        let primary_bonus = if self.is_blitz {
            BlitzMedal::from_score(self.score, &self.blitz_tuning).reward_bonus_pct()
        } else {
            self.moves_left
                .saturating_mul(balance.move_reward_bonus_pct_per_move)
                .min(balance.move_reward_bonus_cap_pct)
        };
        (
            primary_bonus,
            self.resource_bonus_pct,
            self.fury_resource_bonus_pct,
        )
    }

    /// Computes final resource rewards when the level is won (scaled by remaining moves + bonuses).
    pub fn earned_rewards(&self, balance: &BalanceConfig) -> ResourceBundle {
        if self.outcome != LevelOutcome::Victory {
            return ResourceBundle::default();
        }
        let (move_bonus, passive_bonus, fury_bonus) = self.reward_bonus_breakdown(balance);
        let bonus_pct = move_bonus
            .saturating_add(passive_bonus)
            .saturating_add(fury_bonus);
        let scale = |base: u32| add_pct_u32(base, bonus_pct);
        ResourceBundle {
            wood: scale(self.level.base_reward.wood),
            stone: scale(self.level.base_reward.stone),
            gold: scale(self.level.base_reward.gold),
            mead: scale(self.level.base_reward.mead),
        }
    }
}
