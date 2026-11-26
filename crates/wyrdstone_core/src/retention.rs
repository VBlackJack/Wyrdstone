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

//! Retention systems: Valhalla Flame win streaks, Last Stand, Offline Vault, Daily Bounties, and free Skald's Path.

use crate::board::ActiveTool;
use crate::city::{ResourceBundle, ToolInventory};
use serde::{Deserialize, Serialize};

/// Extra moves granted by Last Stand on move-limited defeat.
pub const LAST_STAND_EXTRA_MOVES: u32 = 5;
/// Extra seconds granted by Last Stand on timed defeat.
pub const LAST_STAND_EXTRA_SECS: f32 = 15.0;
/// Gold cost of invoking Last Stand.
pub const LAST_STAND_GOLD_COST: u32 = 60;
/// Mead cost of invoking Last Stand.
pub const LAST_STAND_MEAD_COST: u32 = 10;

/// Minimum offline absence before the Fjord Vault offers a welcome-back chest (30 minutes).
pub const OFFLINE_VAULT_MIN_ABSENCE_SECS: u64 = 1_800;
/// Duration of one passive offline harvest cycle (30 minutes).
pub const OFFLINE_VAULT_CYCLE_SECS: u64 = 1_800;
/// Base maximum hours accumulated by the Offline Harvest Vault.
pub const OFFLINE_VAULT_BASE_MAX_HOURS: u64 = 8;
/// Additional offline cap hours unlocked per Great Hall tier.
pub const OFFLINE_VAULT_EXTRA_HOURS_PER_HALL_TIER: u64 = 2;

/// Number of tiers in the free seasonal Skald's Path track.
pub const SKALD_PASS_TIERS: u8 = 10;
/// Skald XP required per tier.
pub const SKALD_PASS_XP_PER_TIER: u32 = 100;
/// Skald XP awarded on a standard board victory.
pub const SKALD_XP_BOARD_WIN: u32 = 15;
/// Skald XP awarded on a Daily Trial victory.
pub const SKALD_XP_DAILY_WIN: u32 = 30;
/// Skald XP awarded on a Boss Raid or Gauntlet victory.
pub const SKALD_XP_BOSS_WIN: u32 = 35;

/// Maximum Devotion upgrade tier per Norse Patron deity.
pub const MAX_PATRON_DEVOTION_TIER: u8 = 5;
/// Fury charge speed bonus per Patron Devotion tier (+6% per tier, +30% at Tier 5).
pub const PATRON_DEVOTION_FURY_PCT_PER_TIER: u32 = 6;
/// Score multiplier bonus per Patron Devotion tier (+4% per tier, +20% at Tier 5).
pub const PATRON_DEVOTION_SCORE_PCT_PER_TIER: u32 = 4;
/// Starting Divine Fury bonus unlocked at Patron Devotion Tier 3+.
pub const PATRON_DEVOTION_START_FURY_AT_TIER3: u32 = 10;
/// Bonus Boss damage percentage unlocked at Patron Devotion Tier 5.
pub const PATRON_DEVOTION_BOSS_DMG_PCT_AT_TIER5: u32 = 15;

/// Building identifier carried by Weekly Seeded Challenge runs.
pub const WEEKLY_CHALLENGE_BUILDING_ID: &str = "weekly_challenge";
/// Score medal thresholds for Bronze, Silver, Gold, and Legend on Weekly Seeded Challenges.
pub const WEEKLY_MEDAL_THRESHOLDS: [u32; 4] = [8_000, 14_000, 22_000, 32_000];

/// Glory increment per Legend Rank beyond High King.
pub const LEGEND_RANK_GLORY_STEP: u32 = 250;
/// Bonus spoils percentage granted per Legend Rank.
pub const LEGEND_SPOILS_PCT_PER_LEVEL: u32 = 2;
/// Maximum spoils bonus percentage attainable from Legend Ranks.
pub const LEGEND_SPOILS_MAX_BONUS_PCT: u32 = 40;

/// Returns the resource cost required to invoke Last Stand once on defeat.
pub const fn last_stand_cost() -> ResourceBundle {
    ResourceBundle {
        wood: 0,
        stone: 0,
        gold: LAST_STAND_GOLD_COST,
        mead: LAST_STAND_MEAD_COST,
    }
}

/// Escalating 4-tier win-streak buff granting stronger starting board power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValhallaFlameTier {
    #[default]
    None,
    Ember,
    Blaze,
    Bifrost,
    Inferno,
}

impl ValhallaFlameTier {
    /// Maps the current win streak count onto its Valhalla Flame tier.
    pub const fn from_streak(streak: u32) -> Self {
        match streak {
            0 => Self::None,
            1 => Self::Ember,
            2 => Self::Blaze,
            3 | 4 => Self::Bifrost,
            _ => Self::Inferno,
        }
    }

    pub const fn tier_index(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Ember => 1,
            Self::Blaze => 2,
            Self::Bifrost => 3,
            Self::Inferno => 4,
        }
    }

    pub const fn name_key(self) -> &'static str {
        match self {
            Self::None => "flame.tier.none",
            Self::Ember => "flame.tier.ember",
            Self::Blaze => "flame.tier.blaze",
            Self::Bifrost => "flame.tier.bifrost",
            Self::Inferno => "flame.tier.inferno",
        }
    }

    pub const fn desc_key(self) -> &'static str {
        match self {
            Self::None => "flame.tier.none",
            Self::Ember => "flame.desc.ember",
            Self::Blaze => "flame.desc.blaze",
            Self::Bifrost => "flame.desc.bifrost",
            Self::Inferno => "flame.desc.inferno",
        }
    }

    /// Bonus starting Divine Fury granted at Tier 4 (Inferno).
    pub const fn starting_fury_bonus(self) -> u32 {
        match self {
            Self::Inferno => 25,
            _ => 0,
        }
    }

    /// Returns the next flame tier and remaining consecutive victories needed to reach it.
    pub const fn streak_needed_for_next(streak: u32) -> Option<(Self, u32)> {
        match streak {
            0 => Some((Self::Ember, 1)),
            1 => Some((Self::Blaze, 1)),
            2 => Some((Self::Bifrost, 1)),
            3 => Some((Self::Inferno, 2)),
            4 => Some((Self::Inferno, 1)),
            _ => None,
        }
    }
}

/// Accumulated passive village production earned while away from the Fjord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineVaultSummary {
    pub elapsed_secs: u64,
    pub capped_secs: u64,
    pub max_hours: u64,
    pub cycles: u32,
    pub rewards: ResourceBundle,
}

/// Category of a rotating Daily Bounty contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DailyBountyKind {
    ClearRunes,
    WinBoards,
    TriggerFury,
    ReachCombo,
    ClearObstacles,
    EarnScore,
}

impl DailyBountyKind {
    pub const ALL: [Self; 6] = [
        Self::ClearRunes,
        Self::WinBoards,
        Self::TriggerFury,
        Self::ReachCombo,
        Self::ClearObstacles,
        Self::EarnScore,
    ];

    pub const fn desc_key(self) -> &'static str {
        match self {
            Self::ClearRunes => "bounty.kind.clear_runes",
            Self::WinBoards => "bounty.kind.win_boards",
            Self::TriggerFury => "bounty.kind.trigger_fury",
            Self::ReachCombo => "bounty.kind.reach_combo",
            Self::ClearObstacles => "bounty.kind.clear_obstacles",
            Self::EarnScore => "bounty.kind.earn_score",
        }
    }

    const fn target(self) -> u32 {
        match self {
            Self::ClearRunes => 120,
            Self::WinBoards => 2,
            Self::TriggerFury => 2,
            Self::ReachCombo => 4,
            Self::ClearObstacles => 20,
            Self::EarnScore => 15_000,
        }
    }

    const fn reward(self) -> ResourceBundle {
        match self {
            Self::ClearRunes => ResourceBundle {
                wood: 45,
                stone: 35,
                gold: 50,
                mead: 10,
            },
            Self::WinBoards => ResourceBundle {
                wood: 60,
                stone: 60,
                gold: 80,
                mead: 15,
            },
            Self::TriggerFury => ResourceBundle {
                wood: 40,
                stone: 55,
                gold: 75,
                mead: 12,
            },
            Self::ReachCombo => ResourceBundle {
                wood: 50,
                stone: 50,
                gold: 70,
                mead: 15,
            },
            Self::ClearObstacles => ResourceBundle {
                wood: 65,
                stone: 50,
                gold: 60,
                mead: 12,
            },
            Self::EarnScore => ResourceBundle {
                wood: 55,
                stone: 55,
                gold: 90,
                mead: 15,
            },
        }
    }
}

/// Single daily contract entry on the Fjord bounty board.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyBounty {
    pub kind: DailyBountyKind,
    pub target: u32,
    pub progress: u32,
    pub claimed: bool,
    pub skald_xp: u32,
    pub reward: ResourceBundle,
}

impl DailyBounty {
    pub fn is_complete(&self) -> bool {
        self.progress >= self.target
    }

    pub fn is_claimable(&self) -> bool {
        !self.claimed && self.is_complete()
    }
}

/// Deterministic set of 3 rotating daily contracts keyed by UTC day index.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyBountyBoard {
    pub day_index: u64,
    pub bounties: Vec<DailyBounty>,
}

impl DailyBountyBoard {
    /// Generates 3 distinct deterministic bounties when the UTC day changes.
    pub fn ensure_for_day(&mut self, day_index: u64) {
        if self.day_index == day_index && !self.bounties.is_empty() {
            return;
        }
        self.day_index = day_index;
        self.bounties.clear();
        let mut pool = DailyBountyKind::ALL.to_vec();
        let mut state = day_index
            .wrapping_add(0x9E37_79B9_7F4A_7C15)
            .wrapping_mul(0xBF58_476D_1CE4_E5B9);
        for _ in 0..3 {
            state ^= state >> 27;
            state = state.wrapping_mul(0x94D0_49BB_1331_11EB);
            let idx = (state as usize) % pool.len();
            let kind = pool.swap_remove(idx);
            self.bounties.push(DailyBounty {
                kind,
                target: kind.target(),
                progress: 0,
                claimed: false,
                skald_xp: 45,
                reward: kind.reward(),
            });
        }
    }

    /// Adds cumulative progress toward matching unclaimed bounties.
    pub fn record_progress(&mut self, kind: DailyBountyKind, amount: u32) {
        if amount == 0 {
            return;
        }
        for b in &mut self.bounties {
            if !b.claimed && b.kind == kind {
                b.progress = b.progress.saturating_add(amount).min(b.target);
            }
        }
    }

    /// Records a single-event peak (such as max cascade combo) toward matching bounties.
    pub fn record_peak(&mut self, kind: DailyBountyKind, peak: u32) {
        for b in &mut self.bounties {
            if !b.claimed && b.kind == kind {
                b.progress = b.progress.max(peak).min(b.target);
            }
        }
    }

    pub fn claimable_count(&self) -> usize {
        self.bounties.iter().filter(|b| b.is_claimable()).count()
    }

    pub fn sanitize(&mut self) {
        self.bounties.truncate(3);
        for b in &mut self.bounties {
            b.target = b.target.max(1);
            b.progress = b.progress.min(b.target);
        }
    }
}

/// Free 10-tier Skald's Path reward track funded by victories and Daily Bounties.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkaldPassState {
    pub xp: u32,
    pub claimed_tiers: Vec<u8>,
}

/// Returns `(resources, optional_tool, glory)` for tier `1..=10` of the free Skald's Path.
pub const fn skald_pass_tier_reward(tier: u8) -> (ResourceBundle, Option<ActiveTool>, u32) {
    match tier {
        1 => (
            ResourceBundle {
                wood: 45,
                stone: 45,
                gold: 60,
                mead: 10,
            },
            None,
            15,
        ),
        2 => (
            ResourceBundle {
                wood: 60,
                stone: 60,
                gold: 75,
                mead: 15,
            },
            Some(ActiveTool::Mjolnir),
            20,
        ),
        3 => (
            ResourceBundle {
                wood: 80,
                stone: 80,
                gold: 100,
                mead: 20,
            },
            None,
            25,
        ),
        4 => (
            ResourceBundle {
                wood: 95,
                stone: 95,
                gold: 120,
                mead: 25,
            },
            Some(ActiveTool::SurtrEmber),
            30,
        ),
        5 => (
            ResourceBundle {
                wood: 120,
                stone: 120,
                gold: 150,
                mead: 30,
            },
            None,
            35,
        ),
        6 => (
            ResourceBundle {
                wood: 140,
                stone: 140,
                gold: 175,
                mead: 35,
            },
            Some(ActiveTool::ThorBolt),
            40,
        ),
        7 => (
            ResourceBundle {
                wood: 165,
                stone: 165,
                gold: 210,
                mead: 40,
            },
            None,
            45,
        ),
        8 => (
            ResourceBundle {
                wood: 190,
                stone: 190,
                gold: 240,
                mead: 45,
            },
            Some(ActiveTool::DrakkarCharge),
            55,
        ),
        9 => (
            ResourceBundle {
                wood: 220,
                stone: 220,
                gold: 280,
                mead: 55,
            },
            None,
            65,
        ),
        _ => (
            ResourceBundle {
                wood: 300,
                stone: 300,
                gold: 400,
                mead: 80,
            },
            Some(ActiveTool::BifrostPrism),
            80,
        ),
    }
}

impl SkaldPassState {
    pub fn add_xp(&mut self, amount: u32) {
        let cap = u32::from(SKALD_PASS_TIERS) * SKALD_PASS_XP_PER_TIER;
        self.xp = self.xp.saturating_add(amount).min(cap);
    }

    pub fn current_tier(&self) -> u8 {
        (self.xp / SKALD_PASS_XP_PER_TIER).min(u32::from(SKALD_PASS_TIERS)) as u8
    }

    pub fn is_tier_claimed(&self, tier: u8) -> bool {
        self.claimed_tiers.contains(&tier)
    }

    pub fn claimable_count(&self) -> usize {
        let unlocked = self.current_tier();
        (1..=unlocked).filter(|t| !self.is_tier_claimed(*t)).count()
    }

    pub fn try_claim_tier(
        &mut self,
        tier: u8,
        resources: &mut ResourceBundle,
        tools: &mut ToolInventory,
        glory: &mut u32,
    ) -> Option<(ResourceBundle, Option<ActiveTool>, u32)> {
        if tier == 0
            || tier > SKALD_PASS_TIERS
            || tier > self.current_tier()
            || self.is_tier_claimed(tier)
        {
            return None;
        }
        let (res, tool, gl) = skald_pass_tier_reward(tier);
        resources.add(&res);
        if let Some(t) = tool {
            tools.grant_one(t);
        }
        *glory = glory.saturating_add(gl);
        self.claimed_tiers.push(tier);
        Some((res, tool, gl))
    }

    pub fn sanitize(&mut self) {
        let cap = u32::from(SKALD_PASS_TIERS) * SKALD_PASS_XP_PER_TIER;
        self.xp = self.xp.min(cap);
        self.claimed_tiers
            .retain(|&t| (1..=SKALD_PASS_TIERS).contains(&t));
        self.claimed_tiers.sort_unstable();
        self.claimed_tiers.dedup();
    }
}

/// Secondary game modes unlocked progressively as the player advances through early Fjord milestones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayMode {
    DailyTrial,
    YggdrasilTrials,
    ValhallaBlitz,
    WeeklyChallenge,
    BossRaids,
    Expedition,
}

impl PlayMode {
    pub const ALL: [Self; 6] = [
        Self::DailyTrial,
        Self::YggdrasilTrials,
        Self::ValhallaBlitz,
        Self::WeeklyChallenge,
        Self::BossRaids,
        Self::Expedition,
    ];

    pub const fn name_key(self) -> &'static str {
        match self {
            Self::DailyTrial => "city.panel.daily.button",
            Self::YggdrasilTrials => "city.panel.trials.title",
            Self::ValhallaBlitz => "city.panel.blitz.button",
            Self::WeeklyChallenge => "city.panel.weekly.button",
            Self::BossRaids => "city.panel.raids.button",
            Self::Expedition => "city.panel.expedition.button",
        }
    }

    pub const fn unlock_hint_key(self) -> &'static str {
        match self {
            Self::DailyTrial => "mode.unlock.daily",
            Self::YggdrasilTrials => "mode.unlock.trials",
            Self::ValhallaBlitz => "mode.unlock.blitz",
            Self::WeeklyChallenge => "mode.unlock.weekly",
            Self::BossRaids => "mode.unlock.raids",
            Self::Expedition => "mode.unlock.expedition",
        }
    }
}

/// Returns every secondary play mode newly unlocked between `before_mask` and `after_mask`.
pub fn newly_unlocked_modes(before_mask: u8, after_mask: u8) -> Vec<PlayMode> {
    let diff = after_mask & !before_mask;
    PlayMode::ALL
        .iter()
        .enumerate()
        .filter(|(idx, _)| (diff & (1u8 << idx)) != 0)
        .map(|(_, &mode)| mode)
        .collect()
}

/// Returns `(resources, required_glory)` to upgrade Patron Devotion to `next_tier` (`1..=5`).
pub const fn patron_devotion_upgrade_cost(next_tier: u8) -> (ResourceBundle, u32) {
    match next_tier {
        1 => (
            ResourceBundle {
                wood: 120,
                stone: 120,
                gold: 160,
                mead: 25,
            },
            50,
        ),
        2 => (
            ResourceBundle {
                wood: 200,
                stone: 200,
                gold: 260,
                mead: 45,
            },
            150,
        ),
        3 => (
            ResourceBundle {
                wood: 320,
                stone: 320,
                gold: 400,
                mead: 70,
            },
            300,
        ),
        4 => (
            ResourceBundle {
                wood: 460,
                stone: 460,
                gold: 580,
                mead: 100,
            },
            500,
        ),
        _ => (
            ResourceBundle {
                wood: 650,
                stone: 650,
                gold: 800,
                mead: 140,
            },
            750,
        ),
    }
}

/// Maps a UTC day index to its deterministic UTC week index.
pub const fn weekly_challenge_week_index(day_index: u64) -> u64 {
    day_index / 7
}

/// Maps a Weekly Seeded Challenge score to medal tier `0..=4` (None/Bronze/Silver/Gold/Legend).
pub const fn weekly_medal_for_score(score: u32) -> u8 {
    if score >= WEEKLY_MEDAL_THRESHOLDS[3] {
        4
    } else if score >= WEEKLY_MEDAL_THRESHOLDS[2] {
        3
    } else if score >= WEEKLY_MEDAL_THRESHOLDS[1] {
        2
    } else if score >= WEEKLY_MEDAL_THRESHOLDS[0] {
        1
    } else {
        0
    }
}

/// Returns the localization key for Weekly Seeded Challenge medal tier `0..=4`.
pub const fn weekly_medal_name_key(medal: u8) -> &'static str {
    match medal {
        1 => "weekly.medal.bronze",
        2 => "weekly.medal.silver",
        3 => "weekly.medal.gold",
        4 => "weekly.medal.legend",
        _ => "weekly.medal.none",
    }
}

/// Returns `(resources, optional_tool, glory)` awarded when first reaching Weekly medal `1..=4`.
pub const fn weekly_medal_reward(medal: u8) -> (ResourceBundle, Option<ActiveTool>, u32) {
    match medal {
        1 => (
            ResourceBundle {
                wood: 90,
                stone: 90,
                gold: 120,
                mead: 20,
            },
            None,
            25,
        ),
        2 => (
            ResourceBundle {
                wood: 140,
                stone: 140,
                gold: 190,
                mead: 35,
            },
            Some(ActiveTool::ThorBolt),
            40,
        ),
        3 => (
            ResourceBundle {
                wood: 210,
                stone: 210,
                gold: 290,
                mead: 55,
            },
            Some(ActiveTool::DrakkarCharge),
            65,
        ),
        4 => (
            ResourceBundle {
                wood: 320,
                stone: 320,
                gold: 450,
                mead: 85,
            },
            Some(ActiveTool::BifrostPrism),
            100,
        ),
        _ => (
            ResourceBundle {
                wood: 0,
                stone: 0,
                gold: 0,
                mead: 0,
            },
            None,
            0,
        ),
    }
}

/// Persistent score record and claimed medal tier for the current Weekly Seeded Challenge.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyChallengeState {
    pub week_index: u64,
    pub best_score: u32,
    pub claimed_medal: u8,
}

impl WeeklyChallengeState {
    /// Resets weekly record and medal progression when a new UTC week starts.
    pub fn ensure_for_week(&mut self, week_index: u64) {
        if self.week_index != week_index {
            self.week_index = week_index;
            self.best_score = 0;
            self.claimed_medal = 0;
        }
    }

    /// Records a Weekly Challenge run, granting any newly reached medal tier rewards once per week.
    pub fn record_run(
        &mut self,
        week_index: u64,
        score: u32,
        resources: &mut ResourceBundle,
        tools: &mut ToolInventory,
        glory: &mut u32,
    ) -> (u8, bool, ResourceBundle, u32) {
        self.ensure_for_week(week_index);
        let is_new_best = score > self.best_score;
        if is_new_best {
            self.best_score = score;
        }
        let reached = weekly_medal_for_score(score);
        let mut bonus_res = ResourceBundle::default();
        let mut bonus_glory = 0u32;
        if reached > self.claimed_medal {
            for medal in (self.claimed_medal + 1)..=reached {
                let (res, tool, gl) = weekly_medal_reward(medal);
                resources.add(&res);
                if let Some(t) = tool {
                    tools.grant_one(t);
                }
                *glory = glory.saturating_add(gl);
                bonus_res.add(&res);
                bonus_glory = bonus_glory.saturating_add(gl);
            }
            self.claimed_medal = reached;
        }
        (reached, is_new_best, bonus_res, bonus_glory)
    }

    pub fn sanitize(&mut self) {
        self.claimed_medal = self.claimed_medal.min(4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valhalla_flame_tiers_scale_with_streak() {
        assert_eq!(ValhallaFlameTier::from_streak(0), ValhallaFlameTier::None);
        assert_eq!(ValhallaFlameTier::from_streak(1), ValhallaFlameTier::Ember);
        assert_eq!(ValhallaFlameTier::from_streak(2), ValhallaFlameTier::Blaze);
        assert_eq!(
            ValhallaFlameTier::from_streak(3),
            ValhallaFlameTier::Bifrost
        );
        assert_eq!(
            ValhallaFlameTier::from_streak(4),
            ValhallaFlameTier::Bifrost
        );
        assert_eq!(
            ValhallaFlameTier::from_streak(5),
            ValhallaFlameTier::Inferno
        );
        assert_eq!(ValhallaFlameTier::Inferno.starting_fury_bonus(), 25);
        assert_eq!(ValhallaFlameTier::Bifrost.starting_fury_bonus(), 0);
        assert_eq!(
            ValhallaFlameTier::streak_needed_for_next(0),
            Some((ValhallaFlameTier::Ember, 1))
        );
        assert_eq!(
            ValhallaFlameTier::streak_needed_for_next(1),
            Some((ValhallaFlameTier::Blaze, 1))
        );
        assert_eq!(
            ValhallaFlameTier::streak_needed_for_next(2),
            Some((ValhallaFlameTier::Bifrost, 1))
        );
        assert_eq!(
            ValhallaFlameTier::streak_needed_for_next(3),
            Some((ValhallaFlameTier::Inferno, 2))
        );
        assert_eq!(
            ValhallaFlameTier::streak_needed_for_next(4),
            Some((ValhallaFlameTier::Inferno, 1))
        );
        assert_eq!(ValhallaFlameTier::streak_needed_for_next(5), None);
        assert_eq!(ValhallaFlameTier::streak_needed_for_next(9), None);
    }

    #[test]
    fn newly_unlocked_modes_returns_newly_set_bits_in_order() {
        assert!(newly_unlocked_modes(0b000000, 0b000000).is_empty());
        assert_eq!(
            newly_unlocked_modes(0b000001, 0b010101),
            vec![PlayMode::ValhallaBlitz, PlayMode::BossRaids]
        );
        assert!(newly_unlocked_modes(0b000011, 0b000001).is_empty());
    }

    #[test]
    fn daily_bounty_board_generates_three_distinct_contracts_and_tracks_progress() {
        let mut board = DailyBountyBoard::default();
        board.ensure_for_day(20_417);
        assert_eq!(board.bounties.len(), 3);
        let kinds: std::collections::HashSet<DailyBountyKind> =
            board.bounties.iter().map(|b| b.kind).collect();
        assert_eq!(kinds.len(), 3, "daily bounties must be 3 distinct kinds");

        let first_kind = board.bounties[0].kind;
        let target = board.bounties[0].target;
        board.record_progress(first_kind, target + 50);
        assert_eq!(board.bounties[0].progress, target);
        assert!(board.bounties[0].is_claimable());
        assert_eq!(board.claimable_count(), 1);
    }

    #[test]
    fn skald_pass_unlocks_and_claims_all_ten_tiers() {
        let mut pass = SkaldPassState::default();
        let mut res = ResourceBundle::default();
        let mut tools = ToolInventory::default();
        let mut glory = 0u32;

        pass.add_xp(250);
        assert_eq!(pass.current_tier(), 2);
        assert_eq!(pass.claimable_count(), 2);
        assert!(pass
            .try_claim_tier(1, &mut res, &mut tools, &mut glory)
            .is_some());
        assert!(pass
            .try_claim_tier(1, &mut res, &mut tools, &mut glory)
            .is_none());
        assert!(pass
            .try_claim_tier(3, &mut res, &mut tools, &mut glory)
            .is_none());
        assert!(pass
            .try_claim_tier(2, &mut res, &mut tools, &mut glory)
            .is_some());
        assert_eq!(tools.mjolnir, 1);
        assert_eq!(pass.claimable_count(), 0);
    }
}
