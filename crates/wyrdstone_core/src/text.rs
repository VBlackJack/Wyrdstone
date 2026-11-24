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

//! Dynamic text formatters binding i18n templates to live balance and content values.

use crate::achievements::{AchievementCondition, AchievementDef, ClanRank};
use crate::balance::{BalanceConfig, BuildingBonusKind, SessionTuning};
use crate::boss::{BossDef, BossRiposteKind};
use crate::city::BuildingDef;
use crate::expedition::ExpeditionBoon;
use crate::i18n::I18nCatalog;
use crate::level::DailyBlessing;
use crate::patron::PatronGod;

/// Formats a Clan Rank passive perk string using values from `balance`.
pub fn format_rank_perk(i18n: &I18nCatalog, rank: ClanRank, balance: &BalanceConfig) -> String {
    let key = rank.perk_key();
    match rank {
        ClanRank::Scout => i18n.t(key).to_string(),
        ClanRank::Huscarl | ClanRank::Jarl | ClanRank::HighKing => i18n.t_fmt(
            key,
            &[
                ("res", rank.resource_bonus_pct_with(balance).to_string()),
                ("fury", rank.starting_fury_with(balance).to_string()),
            ],
        ),
    }
}

/// Formats today's Fjord Blessing description using values from `tuning`.
pub fn format_blessing_desc(
    i18n: &I18nCatalog,
    blessing: DailyBlessing,
    tuning: &SessionTuning,
) -> String {
    let val = match blessing {
        DailyBlessing::OdinFavor => tuning.blessing_odin_fury_pct,
        DailyBlessing::NjordBreeze => tuning.blessing_njord_resource_pct,
        DailyBlessing::FenrirEclipse => tuning.blessing_fenrir_boss_damage_pct,
        DailyBlessing::IdunnGrace => tuning.blessing_idunn_extra_moves,
    };
    i18n.t_fmt(blessing.desc_key(), &[("val", val.to_string())])
}

/// Formats a Patron God's Divine Fury description using values from `tuning`.
pub fn format_patron_desc(i18n: &I18nCatalog, patron: PatronGod, tuning: &SessionTuning) -> String {
    let key = patron.desc_key();
    match patron {
        PatronGod::Thor => i18n.t_fmt(key, &[("cells", tuning.thor_fury_strike_cells.to_string())]),
        PatronGod::Freya => i18n.t_fmt(key, &[("pct", tuning.freya_fury_resource_pct.to_string())]),
        PatronGod::Loki => i18n.t_fmt(key, &[("count", tuning.loki_fury_boosters.to_string())]),
        PatronGod::Odin => i18n.t_fmt(key, &[("moves", tuning.odin_fury_extra_moves.to_string())]),
    }
}

/// Formats an Expedition boon description using its carried numeric magnitude.
pub fn format_boon_desc(i18n: &I18nCatalog, boon: ExpeditionBoon) -> String {
    let key = boon.desc_key();
    match boon {
        ExpeditionBoon::ExtraMoves(v)
        | ExpeditionBoon::ScoreBoostPct(v)
        | ExpeditionBoon::GrantMjolnir(v)
        | ExpeditionBoon::FuryHeadstart(v) => i18n.t_fmt(key, &[("val", v.to_string())]),
        ExpeditionBoon::StartingBooster | ExpeditionBoon::Relic(_) => i18n.t(key).to_string(),
    }
}

/// Formats a village building's passive bonus description from `balance`.
pub fn format_building_bonus(
    i18n: &I18nCatalog,
    building: &BuildingDef,
    balance: &BalanceConfig,
) -> String {
    let kind = balance
        .building_bonuses
        .iter()
        .find(|binding| binding.building_id == building.id)
        .map(|binding| binding.kind);
    let val = match kind {
        Some(BuildingBonusKind::ExtraMaxEnergy) => balance.great_hall_energy_per_tier,
        Some(BuildingBonusKind::ScoreBonusPct) => balance.forge_score_pct_per_tier,
        Some(BuildingBonusKind::ResourceBonusPct) => balance.dock_resource_pct_per_tier,
        Some(BuildingBonusKind::ExtraMoves) => balance.shrine_moves_per_tier,
        Some(BuildingBonusKind::EnergyRegenSpeedPct) => balance.meadery_regen_pct_per_tier,
        Some(BuildingBonusKind::FuryChargeBonusPct) => balance.watchtower_fury_pct_per_tier,
        None => 0,
    };
    i18n.t_fmt(&building.bonus_key, &[("val", val.to_string())])
}

const fn riposte_target_count(kind: BossRiposteKind) -> usize {
    match kind {
        BossRiposteKind::ChainRunes(n)
        | BossRiposteKind::FreezeCells(n)
        | BossRiposteKind::DropCrates(n)
        | BossRiposteKind::CurseRunes(n) => n,
    }
}

/// Formats a Mythological Boss Raid description from `boss` riposte settings.
pub fn format_boss_desc(i18n: &I18nCatalog, boss: &BossDef) -> String {
    let count = riposte_target_count(boss.riposte);
    let p2_count = riposte_target_count(boss.phase2_riposte.unwrap_or(boss.riposte));
    i18n.t_fmt(
        &boss.desc_key,
        &[
            ("turns", boss.riposte_interval.to_string()),
            ("count", count.to_string()),
            ("p2_count", p2_count.to_string()),
        ],
    )
}

/// Formats a Skald's Feat description from its condition threshold when present.
pub fn format_achievement_desc(i18n: &I18nCatalog, ach: &AchievementDef) -> String {
    if let Some(val) = ach.condition.and_then(AchievementCondition::threshold) {
        i18n.t_fmt(&ach.desc_key, &[("val", val.to_string())])
    } else {
        i18n.t(&ach.desc_key).to_string()
    }
}
