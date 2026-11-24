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

//! The synchronous wrappers (`LevelSession::play_swap` / `play_tool`) against the step machine,
//! and the board-only helpers (`Board::try_swap` / `use_active_tool`) against pinned digests.

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::sync::OnceLock;
use wyrdstone_core::{
    ActiveTool, Board, Booster, Cell, CellPos, CityBonuses, GameDataBundle, Language, LevelDef,
    LevelOutcome, LevelSession, MatchStepOutcome, Objective, PatronGod, Turn, TurnStep,
};

/// Seeds exercised by the wrapper equivalence tests.
const WRAPPER_SEEDS: u64 = 12;
/// Maximum consecutive turns played per seed.
const WRAPPER_MAX_TURNS: usize = 6;
/// Seeds used for the pinned board-only scenarios.
const BOARD_SEEDS: [u64; 4] = [1, 7, 23, 101];
/// Tool targets used by the tool scenarios, as `(col, row)`.
const TOOL_TARGETS: [(usize, usize); 2] = [(3, 3), (1, 5)];
/// Side of the playable window of the frost world.
const FROST_WINDOW: usize = 8;
/// Boss raid used by the boss wrapper scenarios.
const BOSS_ID: &str = "boss_fenrir";
/// Tools that take a target.
const TARGETED_TOOLS: [ActiveTool; 5] = [
    ActiveTool::Mjolnir,
    ActiveTool::SurtrEmber,
    ActiveTool::ThorBolt,
    ActiveTool::BifrostPrism,
    ActiveTool::DrakkarCharge,
];

type WorldMaker = fn(u64) -> World;

fn forge_world(seed: u64) -> World {
    level_world("forge_1", seed, |_| {})
}

/// A level session with the random stream that started it.
#[derive(Clone)]
struct World {
    session: LevelSession,
    rng: StdRng,
}

/// Bundled game data, parsed once per process.
fn bundled() -> &'static GameDataBundle {
    static DATA: OnceLock<GameDataBundle> = OnceLock::new();
    DATA.get_or_init(|| GameDataBundle::load_bundled(Language::En).expect("bundled data"))
}

fn level_world(level_id: &str, seed: u64, tweak: impl FnOnce(&mut LevelDef)) -> World {
    let mut level = bundled().level_by_id(level_id).expect("level").clone();
    tweak(&mut level);
    let mut rng = StdRng::seed_from_u64(seed);
    let session = LevelSession::start(level, &mut rng);
    World { session, rng }
}

fn boss_world(seed: u64) -> World {
    let data = bundled();
    let def = data
        .bosses
        .iter()
        .find(|b| b.id == BOSS_ID)
        .expect("boss")
        .clone();
    let mut rng = StdRng::seed_from_u64(seed);
    let session = LevelSession::start_boss_with(
        def,
        None,
        CityBonuses::default(),
        PatronGod::Thor,
        &data.balance,
        &mut rng,
    );
    World { session, rng }
}

/// Relic world: a buried relic one row above the bottom of column 3.
fn s3_world(seed: u64) -> World {
    level_world("forge_1", seed, |level| {
        level.objectives = vec![
            Objective::DropRelics { target: 1 },
            Objective::ClearIce { target: 12 },
        ];
        level.relic_cells = vec![(3, 6)];
    })
}

/// Frost level cut down to a small window so a turn can leave no move.
fn frost_world(seed: u64) -> World {
    let mut w = level_world("great_hall_3", seed, |level| {
        level.relic_cells.clear();
        level.crate_cells.clear();
    });
    let cols = w.session.board.cols();
    let rows = w.session.board.rows();
    for row in 0..rows {
        for col in 0..cols {
            if col >= FROST_WINDOW || row >= FROST_WINDOW {
                if let Some(cell) = w.session.board.get_mut(CellPos::new(col, row)) {
                    *cell = Cell::blocked_cell();
                }
            }
        }
    }
    w
}

/// Curse level: cursed runes tick on swaps only.
fn s6_world(seed: u64) -> World {
    level_world("shrine_3", seed, |_| {})
}

/// Boss raid on a frost level that would riposte on the very next move-consuming turn.
fn s8_world(seed: u64) -> World {
    let mut w = boss_world(seed);
    w.session.level.frost_spread = true;
    if let Some(boss) = w.session.boss.as_mut() {
        boss.turns_until_riposte = 1;
    }
    w
}

/// A rune-less Bifrost Shard next to a horizontal line booster.
fn s10_world(seed: u64) -> World {
    let mut w = level_world("forge_1", seed, |level| {
        level.ice_cells.clear();
        level.chained_cells.clear();
    });
    if let Some(cell) = w.session.board.get_mut(CellPos::new(0, 0)) {
        cell.rune = None;
        cell.booster = Booster::BifrostShard;
    }
    if let Some(cell) = w.session.board.get_mut(CellPos::new(1, 0)) {
        cell.booster = Booster::LineHorizontal;
    }
    w
}

fn board_worlds() -> Vec<(&'static str, WorldMaker)> {
    vec![
        ("forge_1", forge_world),
        ("relic", s3_world),
        ("bifrost", s10_world),
        ("curse", s6_world),
    ]
}

fn wrapper_worlds() -> Vec<(&'static str, WorldMaker)> {
    let mut worlds = board_worlds();
    worlds.push(("frost", frost_world));
    worlds.push(("boss", s8_world));
    worlds
}

fn fnv(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn outcome_text(o: &MatchStepOutcome) -> String {
    let mut runes: Vec<String> = o
        .cleared_runes
        .iter()
        .map(|(r, c)| format!("{r:?}={c}"))
        .collect();
    runes.sort();
    format!(
        "valid={} score={} combo={} ice={} chains={} crates={} fog={} relics={} runes=[{}] pos={:?} shuffled={}",
        o.valid,
        o.score_gained,
        o.max_combo,
        o.ice_broken,
        o.chains_broken,
        o.crates_broken,
        o.fog_cleared,
        o.relics_dropped,
        runes.join(","),
        o.cleared_positions,
        o.shuffled
    )
}

/// Digest of a board-only helper result: outcome, final board and the next random draw.
fn board_digest(board: &Board, outcome: &MatchStepOutcome, rng: &mut StdRng) -> u64 {
    let text = format!("{}\n{:?}\n{}", outcome_text(outcome), board, rng.next_u64());
    fnv(&text)
}

/// Digests of `Board::try_swap` (hint move) and `Board::use_active_tool` on seeded boards.
fn board_helper_digests() -> Vec<(String, u64)> {
    let balance = &bundled().balance;
    let mut out = Vec::new();
    for (name, make) in board_worlds() {
        for seed in BOARD_SEEDS {
            let w = make(seed);
            let mut board = w.session.board.clone();
            let mut rng = w.rng.clone();
            if let Some((a, b)) = board.find_hint_move() {
                let step = board.try_swap(a, b, balance, &mut rng);
                out.push((
                    format!("try_swap {name} {seed}"),
                    board_digest(&board, &step, &mut rng),
                ));
            }
            for tool in TARGETED_TOOLS {
                for (col, row) in TOOL_TARGETS {
                    let mut board = w.session.board.clone();
                    let mut rng = w.rng.clone();
                    let target = Some(CellPos::new(col, row));
                    let step = board.use_active_tool(tool, target, balance, &mut rng);
                    out.push((
                        format!("tool {tool:?} {name} {seed} {col},{row}"),
                        board_digest(&board, &step, &mut rng),
                    ));
                }
            }
            let mut board = w.session.board.clone();
            let mut rng = w.rng.clone();
            let step = board.use_active_tool(ActiveTool::LokiShuffle, None, balance, &mut rng);
            out.push((
                format!("loki {name} {seed}"),
                board_digest(&board, &step, &mut rng),
            ));
        }
    }
    out
}

fn drain(session: &mut LevelSession, mut turn: Turn, rng: &mut StdRng) -> MatchStepOutcome {
    let balance = &bundled().balance;
    let mut total = MatchStepOutcome::default();
    while let Some(step) = turn.advance(session, balance, rng) {
        match step {
            TurnStep::Cleared(report) => merge(&mut total, &report.step),
            TurnStep::RelicsDropped(report) => merge(&mut total, &report.step),
            TurnStep::Ended(end) => total.shuffled |= end.shuffled,
            TurnStep::ClearPending(_) => {}
        }
    }
    total
}

/// Manual drive of a swap through the step machine, merging per-step outcomes like the wrapper.
fn manual_swap(
    session: &mut LevelSession,
    a: CellPos,
    b: CellPos,
    rng: &mut StdRng,
) -> MatchStepOutcome {
    match session.begin_swap(a, b) {
        Ok((turn, _)) => drain(session, turn, rng),
        Err(_) => MatchStepOutcome::default(),
    }
}

fn manual_tool(
    session: &mut LevelSession,
    tool: ActiveTool,
    target: Option<CellPos>,
    rng: &mut StdRng,
) -> MatchStepOutcome {
    let balance = &bundled().balance;
    if tool == ActiveTool::LokiShuffle {
        return session.loki_shuffle(balance, rng);
    }
    let Some(target) = target else {
        return MatchStepOutcome::default();
    };
    match session.begin_tool(tool, target) {
        Ok((turn, _)) => drain(session, turn, rng),
        Err(_) => MatchStepOutcome::default(),
    }
}

fn merge(total: &mut MatchStepOutcome, step: &MatchStepOutcome) {
    total.valid |= step.valid;
    total.score_gained += step.score_gained;
    total.max_combo = total.max_combo.max(step.max_combo);
    total.ice_broken += step.ice_broken;
    total.chains_broken += step.chains_broken;
    total.crates_broken += step.crates_broken;
    total.fog_cleared += step.fog_cleared;
    total.relics_dropped += step.relics_dropped;
    for (&rune, &count) in &step.cleared_runes {
        *total.cleared_runes.entry(rune).or_insert(0) += count;
    }
    total.cleared_positions.extend(&step.cleared_positions);
}

fn assert_same(
    label: &str,
    left: &World,
    right: &World,
    lo: &MatchStepOutcome,
    ro: &MatchStepOutcome,
) {
    assert_eq!(
        format!("{:?}", left.session),
        format!("{:?}", right.session),
        "{label}: session"
    );
    assert_eq!(left.session.board, right.session.board, "{label}: board");
    assert_eq!(
        left.rng.clone().next_u64(),
        right.rng.clone().next_u64(),
        "{label}: rng"
    );
    assert_eq!(outcome_text(lo), outcome_text(ro), "{label}: outcome");
}

#[test]
fn play_swap_equals_manual_step_machine() {
    let balance = &bundled().balance;
    for (name, make) in wrapper_worlds() {
        for seed in 0..WRAPPER_SEEDS {
            let mut wrapped = make(seed);
            let mut manual = make(seed);
            for turn in 0..WRAPPER_MAX_TURNS {
                let Some((a, b)) = wrapped.session.board.find_hint_move() else {
                    break;
                };
                let lo = wrapped.session.play_swap(a, b, balance, &mut wrapped.rng);
                let ro = manual_swap(&mut manual.session, a, b, &mut manual.rng);
                let label = format!("swap {name} {seed} turn {turn}");
                assert_same(&label, &wrapped, &manual, &lo, &ro);
                if wrapped.session.outcome != LevelOutcome::InProgress {
                    break;
                }
            }
        }
    }
}

#[test]
fn play_swap_rejects_like_the_step_machine() {
    let balance = &bundled().balance;
    let mut wrapped = forge_world(3);
    let mut manual = wrapped.clone();
    // Non-adjacent cells never start a turn.
    let (a, b) = (CellPos::new(0, 0), CellPos::new(2, 2));
    let lo = wrapped.session.play_swap(a, b, balance, &mut wrapped.rng);
    let ro = manual_swap(&mut manual.session, a, b, &mut manual.rng);
    assert!(!lo.valid);
    assert_same("rejected swap", &wrapped, &manual, &lo, &ro);
}

#[test]
fn play_tool_equals_manual_step_machine() {
    let balance = &bundled().balance;
    for (name, make) in wrapper_worlds() {
        for seed in 0..WRAPPER_SEEDS {
            for tool in TARGETED_TOOLS {
                for (col, row) in TOOL_TARGETS {
                    let target = Some(CellPos::new(col, row));
                    let mut wrapped = make(seed);
                    let mut manual = make(seed);
                    let lo = wrapped
                        .session
                        .play_tool(tool, target, balance, &mut wrapped.rng);
                    let ro = manual_tool(&mut manual.session, tool, target, &mut manual.rng);
                    let label = format!("tool {tool:?} {name} {seed} {col},{row}");
                    assert_same(&label, &wrapped, &manual, &lo, &ro);
                }
            }
            let mut wrapped = make(seed);
            let mut manual = make(seed);
            let lo =
                wrapped
                    .session
                    .play_tool(ActiveTool::LokiShuffle, None, balance, &mut wrapped.rng);
            let ro = manual_tool(
                &mut manual.session,
                ActiveTool::LokiShuffle,
                None,
                &mut manual.rng,
            );
            assert_same(&format!("loki {name} {seed}"), &wrapped, &manual, &lo, &ro);
        }
    }
}

#[test]
fn play_tool_without_target_is_a_no_op() {
    let balance = &bundled().balance;
    let mut w = forge_world(5);
    let before = format!("{:?}", w.session);
    let out = w
        .session
        .play_tool(ActiveTool::Mjolnir, None, balance, &mut w.rng);
    assert!(!out.valid);
    assert_eq!(before, format!("{:?}", w.session));
}

/// Digests captured from `Board::try_swap` / `use_active_tool` updated for BifrostShard
/// double-trigger fix and `cleared_positions` deduplication.
const BOARD_HELPER_DIGESTS: &[u64] = &[
    0xe93f89556f354212,
    0xaf2641be91fad0d1,
    0xe113cd23fcce7adc,
    0xe376db3a897ddf93,
    0xc7f9471b1a24ccf5,
    0x158d06b115b08056,
    0x896e6cb22a0fc645,
    0x472c5b16a5535484,
    0x472c5b16a5535484,
    0x27eaccddf05968ed,
    0xf45c4a13505d2354,
    0x8a8b7ddcc963fa17,
    0x76fba7c642eb2220,
    0x5440067805960c0d,
    0x554d2a5d0ee5bad3,
    0x8d6d34c6fe8044bf,
    0x542e021e22efa138,
    0x62b2896c66d1a7de,
    0xbefd03a95b7d9c04,
    0x2dac8b2238602d38,
    0xe991b82841d96180,
    0x118f402e9e5e0bd6,
    0x075ad2cf49679438,
    0xa4df093b34e9e92c,
    0x12b19e1b529a14f8,
    0x668e904515d6e818,
    0xdd2254a9a7040147,
    0x6ef0ef2e962b23e1,
    0x2a075abc18fbc33c,
    0xe881a4bc78b6adb9,
    0xc41f9ca0d2707f40,
    0x9251ff67f4534857,
    0x8bd785c972ea3d05,
    0x396af732a60b1119,
    0x54b0b53987035294,
    0x5c7b9917ab77ac25,
    0xe8c7c52b6cfb5490,
    0xe0ac922d9bc1a8b4,
    0xc9d9e778d37c3885,
    0xc826152b779be006,
    0xe48b16af5f368863,
    0xcd8e8dcf73783c8b,
    0xdec90a94b258fb91,
    0xd4287729acbc5c8e,
    0x8e8aec27d99e1dc8,
    0xac9bb8ef99130cce,
    0xa15a0b0ae1954cf2,
    0x42fc6b4b5b6d5347,
    0x6b456f781ae7d1bb,
    0xc970274ead36a226,
    0x22ebdc6beb7279ed,
    0x94ac08117fd6262a,
    0xb3dee6d24133d7f4,
    0x1ea2abdbadbb8eab,
    0xa640917a082cc7c2,
    0x33b6b92e617f1c91,
    0x33b6b92e617f1c91,
    0x5be2afb1d9f3c4b7,
    0x1ad59cf427095cdf,
    0xedd2ecbb94a3cae9,
    0x94ac075d52283bd4,
    0xa48413676512a6f7,
    0x204eab717d910ef1,
    0xea7ef6d259bb6a13,
    0x73deef4dc96cb86c,
    0x3e4af0fa0549fbb2,
    0x1644c10cf041558e,
    0x22431fca119c1122,
    0x9c78aaa1412f33e4,
    0x819c9381b8eff550,
    0xe1b91e6c2a842c48,
    0x21039969e364b037,
    0xc3f5749a5b40057a,
    0x162f79ff36655ecc,
    0xd5dec72a2eecba1f,
    0xffc9efbd10258f19,
    0xca5d148a6356e806,
    0xf5e79f3b2edba55b,
    0xe6da2373b7ae7d18,
    0x007dc5f3054e71ed,
    0xfdac5f0f65bf7849,
    0x550a08ec7e10ed3d,
    0x2b18cd40dd8b3a78,
    0xe226cd88ec6c0c29,
    0x0d65a5940e5c1780,
    0xf60a7299b54f8fc0,
    0xf8b67ac417f77837,
    0xe92e4e1dde5c19b6,
    0x2360dcabc602ea99,
    0xb605d2c86f156eb7,
    0x9a5b30f9c4c0ccb1,
    0xc70682d6d9256421,
    0x4cbdfd3f339250bc,
    0x659a6af61fadfbba,
    0x0f30583b4f549b78,
    0xe04952760dd44ee8,
    0x915dff4fa77a10e4,
    0x5bd16987f7247f86,
    0xf7826e5361473d68,
    0xe808099f438f657b,
    0x25d2092576d25e32,
    0x827708119df7d154,
    0x5a5f333ea2a19eab,
    0xd0153d170f10176b,
    0xd0153d170f10176b,
    0x02333adeaa675083,
    0xa669c2da4fc0f8f7,
    0xaeb22b8416ff0c88,
    0xc39160d196a7b816,
    0xa142fc339d8cf40d,
    0x1a36a8516bd0b18c,
    0xfac74c439289653e,
    0x38c1cc92ab131fc7,
    0x6e56048d76274d61,
    0x6f688565070f4623,
    0xbca1f49c2faade50,
    0xa1fb3e784a23c3a4,
    0x876dc8e3058d7e89,
    0xd493de8addf9d09c,
    0x579e105d9caef149,
    0xaacaf3096a34105a,
    0x0ea11fd7baab71f6,
    0x50b0f32c73e2e424,
    0x25140e372e1bd7c5,
    0xcf042c06e43ae73e,
    0x4dfc6a9b93357624,
    0x305a8f63629f6eab,
    0x1054d4585efadd2f,
    0x814cf6991668fe49,
    0x8ad54937c8b0dc79,
    0x430eabd80ede7bae,
    0xa900e414cbadfc1a,
    0xdaf1e88d2b8fcc41,
    0x21a38015bf6fa096,
    0x0e858b081979f424,
    0x832d85b95ec24925,
    0x40007e81bd4e8f28,
    0xe7c9d0e64fcb52fe,
    0x1f8c1647ec23836a,
    0x4684f3660ede7b0d,
    0x1d3e36279bf90786,
    0x6fe50ae2de64c03f,
    0x9654de23127e2886,
    0x0537c5b25cc881c5,
    0xb0452687f0a354ab,
    0xa15b443f162dc92e,
    0x1333a365a7bca9dd,
    0xc11e883fa534404c,
    0x9413db3243a44f45,
    0x90cc7a0db12bdfe3,
    0x8dc707eb5d0d58a9,
    0x1edb5e7bb79b4b25,
    0xa6c53b8be5fe89f1,
    0xefba27c35537477b,
    0x02dd080580a473e1,
    0x77e7fd848aa88f2e,
    0x97748b867c13c73c,
    0xc8c586027b4353ea,
    0xd38f664c52bd97bd,
    0xa307b16245490a45,
    0xc79f7a4562bf9a8f,
    0x02ac9a0b91224505,
    0x409a635965739c0a,
    0x9c984de03690e134,
    0xe2bb678f21306608,
    0xbea7bf07e82064a2,
    0x7d1993f395f761a0,
    0x7fa58dd0a6ebf407,
    0xa74c1b32d5ea84aa,
    0xa878a837203a7ef4,
    0x8071c728f98860b9,
    0x1e462a9a6f98797d,
    0x123e258a49a9ab5e,
    0x2659b572f1bc6ced,
    0x854c12a51a000943,
    0x2803445f2218a174,
    0xa333336534cbfb39,
    0x525f87b9f4ead2e2,
    0x100d4bdde3725ddc,
    0xe1a73ba5c7bdaac9,
    0x1d7696b6c9f8d089,
    0xc3b3238b1be9d2e3,
    0xc46e547579ba0a5e,
    0x0fb41db0fcb3796e,
    0x4eec7ec1dddcbbdf,
    0x25f115ffeeb6ec5e,
    0xbe2b66823b6b3f75,
    0xfc8689cbcf2a90b1,
    0x8836efbb93cb2f8b,
    0x3669beed5614c9c9,
    0x6c5ae6af3fe0f6e6,
    0x1285658a3323f4fb,
];

#[test]
fn board_helpers_are_unchanged() {
    let digests = board_helper_digests();
    let got: Vec<u64> = digests.iter().map(|(_, d)| *d).collect();
    let listing: String = digests
        .iter()
        .map(|(label, d)| {
            format!(
                "{label} {d:#018x}
"
            )
        })
        .collect();
    assert_eq!(
        got.as_slice(),
        BOARD_HELPER_DIGESTS,
        "board helper digests changed; current values:
{listing}"
    );
}
