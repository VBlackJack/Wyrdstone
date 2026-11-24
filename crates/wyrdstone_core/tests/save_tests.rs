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

//! Recovery-aware save loading, backup rotation and atomic write behavior.

use rand::SeedableRng;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use wyrdstone_core::save::{LoadOutcome, ReadOnlyResolution, SAVE_SCHEMA_VERSION};
use wyrdstone_core::{GameDataBundle, Language, SaveData};

static DIR_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Temporary directory removed on drop, unique per test.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let id = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "wyrdstone_save_tests_{label}_{}_{id}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn save_path(&self) -> PathBuf {
        self.0.join("wyrdstone_save.json")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sample_save(tutorial_step: u8, language: Language) -> SaveData {
    let data = GameDataBundle::load_bundled(language).unwrap();
    let mut save = SaveData::new_game(&data.balance, language);
    save.tutorial_step = tutorial_step;
    save
}

fn corrupt_copies(path: &Path) -> Vec<PathBuf> {
    let prefix = format!("{}.corrupt-", path.file_name().unwrap().to_string_lossy());
    std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&prefix))
        })
        .collect()
}

#[test]
fn missing_file_reports_not_found() {
    let dir = TempDir::new("missing");
    assert!(matches!(
        SaveData::load_with_recovery(&dir.save_path()),
        LoadOutcome::NotFound
    ));
}

#[test]
fn readable_file_reports_loaded() {
    let dir = TempDir::new("loaded");
    let path = dir.save_path();
    sample_save(2, Language::En).save_to_path(&path).unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::Loaded(save) => assert_eq!(save.tutorial_step, 2),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn corrupt_file_without_backup_is_preserved_and_reports_fresh() {
    let dir = TempDir::new("fresh");
    let path = dir.save_path();
    std::fs::write(&path, "{ invalid json").unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::FreshAfterBackup(copied) => {
            assert_eq!(std::fs::read_to_string(&copied).unwrap(), "{ invalid json");
            assert!(copied
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains(".corrupt-"));
        }
        other => panic!("expected FreshAfterBackup, got {other:?}"),
    }
    // The unreadable original is left in place; only a later save replaces it.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ invalid json");
    assert_eq!(corrupt_copies(&path).len(), 1);
}

#[test]
fn corrupt_file_with_good_backup_reports_recovered() {
    let dir = TempDir::new("recovered");
    let path = dir.save_path();
    sample_save(3, Language::Fr)
        .save_to_path(&SaveData::backup_path(&path))
        .unwrap();
    std::fs::write(&path, "not json at all").unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::RecoveredFromBackup(save) => {
            assert_eq!(save.tutorial_step, 3);
            assert_eq!(save.language, Language::Fr);
        }
        other => panic!("expected RecoveredFromBackup, got {other:?}"),
    }
    assert_eq!(corrupt_copies(&path).len(), 1);
}

#[test]
fn corrupt_file_with_corrupt_backup_reports_fresh() {
    let dir = TempDir::new("badbak");
    let path = dir.save_path();
    std::fs::write(SaveData::backup_path(&path), "{").unwrap();
    std::fs::write(&path, "{").unwrap();
    assert!(matches!(
        SaveData::load_with_recovery(&path),
        LoadOutcome::FreshAfterBackup(_)
    ));
}

#[test]
fn unreadable_and_uncopyable_file_reports_read_only() {
    let dir = TempDir::new("readonly");
    let path = dir.save_path();
    // A directory at the save path exists but can neither be read nor copied as a file.
    std::fs::create_dir_all(&path).unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::ReadOnly { reason, backup } => {
            assert!(!reason.is_empty());
            assert!(backup.is_none());
        }
        other => panic!("expected ReadOnly, got {other:?}"),
    }
    assert!(path.is_dir(), "the unreadable entry must not be touched");
    assert!(SaveData::backup_unreadable(&path).is_err());
}

#[test]
fn read_only_with_readable_backup_provides_the_backup() {
    let dir = TempDir::new("readonly_bak");
    let path = dir.save_path();
    sample_save(4, Language::En)
        .save_to_path(&SaveData::backup_path(&path))
        .unwrap();
    std::fs::create_dir_all(&path).unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::ReadOnly { backup, .. } => {
            assert_eq!(backup.expect("backup must be provided").tutorial_step, 4);
        }
        other => panic!("expected ReadOnly, got {other:?}"),
    }
    assert!(path.is_dir(), "the unreadable entry must not be touched");
}

#[test]
fn lock_then_unlock_keeps_session_read_only_and_never_overwrites() {
    let dir = TempDir::new("lock_unlock");
    let path = dir.save_path();
    // Locked: a directory at the save path cannot be read or copied.
    std::fs::create_dir_all(&path).unwrap();
    assert_eq!(
        SaveData::resolve_read_only(&path),
        ReadOnlyResolution::StillBlocked
    );
    // Unlocked: the real save is readable again.
    std::fs::remove_dir_all(&path).unwrap();
    sample_save(7, Language::En).save_to_path(&path).unwrap();
    assert_eq!(
        SaveData::resolve_read_only(&path),
        ReadOnlyResolution::KeepReadOnly
    );
    assert!(corrupt_copies(&path).is_empty());
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::Loaded(save) => assert_eq!(save.tutorial_step, 7),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn genuinely_corrupt_primary_is_copied_then_lifted() {
    let dir = TempDir::new("lift");
    let path = dir.save_path();
    std::fs::write(&path, "{ invalid json").unwrap();
    assert_eq!(SaveData::resolve_read_only(&path), ReadOnlyResolution::Lift);
    assert_eq!(corrupt_copies(&path).len(), 1);
}

#[test]
fn backup_unreadable_reports_none_for_missing_file() {
    let dir = TempDir::new("nobackup");
    assert!(SaveData::backup_unreadable(&dir.save_path())
        .unwrap()
        .is_none());
}

#[test]
fn rotation_copies_previous_good_file_to_bak() {
    let dir = TempDir::new("rotate");
    let path = dir.save_path();
    sample_save(1, Language::En)
        .save_with_options(&path, true)
        .unwrap();
    assert!(
        !SaveData::backup_path(&path).exists(),
        "nothing to rotate on the first write"
    );
    let report = sample_save(2, Language::En)
        .save_with_options(&path, true)
        .unwrap();
    assert!(report.backup_rotation_error.is_none());

    let bak = SaveData::from_json(&std::fs::read_to_string(SaveData::backup_path(&path)).unwrap())
        .unwrap();
    assert_eq!(bak.tutorial_step, 1);
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::Loaded(save) => assert_eq!(save.tutorial_step, 2),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn backup_is_not_overwritten_when_rotation_is_disabled_over_unreadable_main() {
    let dir = TempDir::new("keepbak");
    let path = dir.save_path();
    let bak_path = SaveData::backup_path(&path);
    sample_save(7, Language::En)
        .save_to_path(&bak_path)
        .unwrap();
    std::fs::write(&path, "{ corrupt").unwrap();

    // The caller has not verified the main file this session, so it disables rotation.
    sample_save(9, Language::En)
        .save_with_options(&path, false)
        .unwrap();

    let bak = SaveData::from_json(&std::fs::read_to_string(&bak_path).unwrap()).unwrap();
    assert_eq!(bak.tutorial_step, 7, "the good backup must survive");
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::Loaded(save) => assert_eq!(save.tutorial_step, 9),
        other => panic!("expected Loaded, got {other:?}"),
    }
}

#[test]
fn plain_save_to_path_never_touches_bak() {
    let dir = TempDir::new("export");
    let path = dir.0.join("wyrdstone_backup.json");
    sample_save(1, Language::En).save_to_path(&path).unwrap();
    sample_save(2, Language::En).save_to_path(&path).unwrap();
    assert!(!SaveData::backup_path(&path).exists());
    assert!(!path.with_extension("json.tmp").exists());
}

#[test]
fn legacy_save_without_anchor_field_defaults_to_zero() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let save = SaveData::new_game(&data.balance, Language::En);
    let mut value: serde_json::Value = serde_json::from_str(&save.to_json().unwrap()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("energy_regen_anchor_unix_secs");
    let loaded = SaveData::from_json(&value.to_string()).unwrap();
    assert_eq!(loaded.energy_regen_anchor_unix_secs, 0);
}

#[test]
fn tampered_save_is_clamped_on_load_and_cannot_overflow() {
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut save = SaveData::new_game(&data.balance, Language::En);
    save.city.resources.wood = u32::MAX;
    save.city.resources.gold = u32::MAX;
    save.city.glory = u32::MAX;
    save.city.energy = u32::MAX;
    save.city.login_streak_day = 200;
    save.city.last_login_claim_day = Some(u64::MAX);
    save.city.stats.daily_streak = 4_000_000_000;
    save.city.stats.last_daily_day_index = u64::MAX;
    save.city.stats.total_victories = u32::MAX;
    save.city.tools.mjolnir = u32::MAX;
    save.city
        .building_tiers
        .insert("great_hall".to_string(), 4_000_000_000);
    save.city.building_tiers.insert("forge".to_string(), 99);
    save.city.amulet_tiers.insert("odin_eye".to_string(), 250);
    save.city
        .level_best_stars
        .insert("great_hall_1".to_string(), 99);

    let dir = TempDir::new("tampered");
    let path = dir.save_path();
    save.save_to_path(&path).unwrap();
    let loaded = SaveData::load_from_path(&path).unwrap().unwrap();

    let great_hall = data.building_by_id("great_hall").unwrap();
    let forge = data.building_by_id("forge").unwrap();
    assert_eq!(loaded.city.building_tier("great_hall"), great_hall.max_tier);
    assert_eq!(loaded.city.building_tier("forge"), forge.max_tier);
    assert_eq!(
        loaded.city.amulet_tiers.get("odin_eye").copied(),
        Some(wyrdstone_core::city::MAX_AMULET_TIER)
    );
    assert_eq!(loaded.city.level_stars("great_hall_1"), 3);
    assert_eq!(loaded.city.login_streak_day, 7);
    assert_eq!(
        loaded.city.stats.daily_streak,
        wyrdstone_core::city::MAX_DAILY_STREAK_CYCLE
    );
    let mut city = loaded.city;
    assert_eq!(city.energy, city.max_energy(&data.balance));

    // None of the follow-up operations may panic on the saturated state.
    let reward = wyrdstone_core::ResourceBundle {
        wood: 5,
        stone: 5,
        gold: 5,
        mead: 5,
    };
    let _ = city.try_claim_login_tribute(1);
    city.record_daily_victory(2, &reward, &data.balance);
    city.record_level_victory("great_hall", "great_hall_1", 3, &reward, &data.balance);
    city.apply_offline_energy_regen(u64::MAX, &data.balance);
    assert_eq!(city.resources.wood, u32::MAX);
    assert_eq!(city.glory, u32::MAX);
}

/// JSON of a save claiming `schema_version`, with a field the current build does not know.
fn versioned_json(tutorial_step: u8, schema_version: u32, unknown_field: bool) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(&sample_save(tutorial_step, Language::En).to_json().unwrap()).unwrap();
    value["schema_version"] = serde_json::json!(schema_version);
    if unknown_field {
        value["field_from_the_future"] = serde_json::json!({ "x": 1 });
    }
    serde_json::to_string_pretty(&value).unwrap()
}

fn newer_json(tutorial_step: u8) -> String {
    versioned_json(tutorial_step, SAVE_SCHEMA_VERSION + 1, true)
}

fn dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
}

#[test]
fn newer_primary_reports_newer_version_and_stays_untouched() {
    let dir = TempDir::new("newer_primary");
    let path = dir.save_path();
    let raw = newer_json(2);
    std::fs::write(&path, &raw).unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::NewerVersion { found, backup } => {
            assert_eq!(found, SAVE_SCHEMA_VERSION + 1);
            assert!(backup.is_none());
        }
        other => panic!("expected NewerVersion, got {other:?}"),
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    assert!(corrupt_copies(&path).is_empty());
    assert!(!path.with_extension("json.corrupt").exists());
    assert_eq!(dir_entries(&dir.0), vec!["wyrdstone_save.json".to_string()]);
}

#[test]
fn newer_primary_with_current_backup_provides_the_backup() {
    let dir = TempDir::new("newer_with_bak");
    let path = dir.save_path();
    std::fs::write(&path, newer_json(2)).unwrap();
    sample_save(4, Language::En)
        .save_to_path(&SaveData::backup_path(&path))
        .unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::NewerVersion { backup, .. } => {
            assert_eq!(backup.expect("current backup is usable").tutorial_step, 4);
        }
        other => panic!("expected NewerVersion, got {other:?}"),
    }
    assert!(corrupt_copies(&path).is_empty());
}

#[test]
fn newer_primary_with_newer_backup_provides_no_backup() {
    let dir = TempDir::new("newer_with_newer_bak");
    let path = dir.save_path();
    std::fs::write(&path, newer_json(2)).unwrap();
    std::fs::write(SaveData::backup_path(&path), newer_json(3)).unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::NewerVersion { backup, .. } => assert!(backup.is_none()),
        other => panic!("expected NewerVersion, got {other:?}"),
    }
    assert!(corrupt_copies(&path).is_empty());
}

#[test]
fn load_from_path_rejects_newer_file_without_corrupt_copy() {
    let dir = TempDir::new("newer_load_from_path");
    let path = dir.save_path();
    let raw = newer_json(2);
    std::fs::write(&path, &raw).unwrap();
    let err = SaveData::load_from_path(&path).expect_err("newer file must be refused");
    assert!(err.contains("newer version"), "unexpected error: {err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    assert!(!path.with_extension("json.corrupt").exists());
    assert!(corrupt_copies(&path).is_empty());
}

#[test]
fn resolve_read_only_keeps_newer_primary_blocked_without_new_files() {
    let dir = TempDir::new("newer_resolve");
    let path = dir.save_path();
    std::fs::write(&path, newer_json(2)).unwrap();
    let before = dir_entries(&dir.0);
    assert_eq!(
        SaveData::resolve_read_only(&path),
        ReadOnlyResolution::StillBlocked
    );
    assert_eq!(dir_entries(&dir.0), before);
}

#[test]
fn corrupt_json_is_still_preserved_by_load_from_path() {
    let dir = TempDir::new("corrupt_load_from_path");
    let path = dir.save_path();
    std::fs::write(&path, "{ invalid json").unwrap();
    assert!(SaveData::load_from_path(&path).is_err());
    let copy = path.with_extension("json.corrupt");
    assert_eq!(std::fs::read_to_string(copy).unwrap(), "{ invalid json");
}

#[test]
fn older_schema_save_loads_at_the_current_version() {
    let dir = TempDir::new("older_schema");
    let path = dir.save_path();
    std::fs::write(&path, versioned_json(3, SAVE_SCHEMA_VERSION - 1, false)).unwrap();
    match SaveData::load_with_recovery(&path) {
        LoadOutcome::Loaded(save) => {
            assert_eq!(save.schema_version, SAVE_SCHEMA_VERSION);
            assert_eq!(save.tutorial_step, 3);
        }
        other => panic!("expected Loaded, got {other:?}"),
    }
    let loaded = SaveData::load_from_path(&path).unwrap().unwrap();
    assert_eq!(loaded.schema_version, SAVE_SCHEMA_VERSION);
}

#[test]
fn preserve_copy_writes_a_tagged_sibling_and_keeps_the_source() {
    let dir = TempDir::new("preserve_copy");
    let path = dir.save_path();
    let raw = newer_json(2);
    std::fs::write(&path, &raw).unwrap();
    let copy = SaveData::preserve_copy(&path, "newer").unwrap();
    assert_eq!(copy.parent(), path.parent());
    assert!(copy
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("wyrdstone_save.json.newer-"));
    assert_eq!(std::fs::read_to_string(&copy).unwrap(), raw);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
    assert!(corrupt_copies(&path).is_empty());
}

#[test]
fn preserve_copy_fails_for_a_missing_source_and_creates_nothing() {
    let dir = TempDir::new("preserve_copy_missing");
    let path = dir.save_path();
    assert!(SaveData::preserve_copy(&path, "newer").is_err());
    assert!(dir_entries(&dir.0).is_empty());
}

/// Files next to the save whose name starts with `<save file name>.<tag>-`.
fn tagged_copies(path: &Path, tag: &str) -> Vec<PathBuf> {
    let prefix = format!("{}.{tag}-", path.file_name().unwrap().to_string_lossy());
    std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&prefix))
        })
        .collect()
}

#[test]
fn prepare_replace_copies_newer_primary_before_replacement() {
    let dir = TempDir::new("prep_primary");
    let path = dir.save_path();
    let raw = newer_json(4);
    std::fs::write(&path, &raw).unwrap();
    let copies = SaveData::prepare_replace(&path, true).unwrap();
    assert_eq!(copies.len(), 1);
    assert_eq!(tagged_copies(&path, "newer"), copies);
    assert_eq!(std::fs::read_to_string(&copies[0]).unwrap(), raw);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
}

#[test]
fn prepare_replace_copies_newer_backup_with_bak_newer_tag() {
    let dir = TempDir::new("prep_newer_bak");
    let path = dir.save_path();
    let bak = SaveData::backup_path(&path);
    let raw = newer_json(6);
    std::fs::write(&bak, &raw).unwrap();
    std::fs::write(&path, "{ not json").unwrap();
    let copies = SaveData::prepare_replace(&path, false).unwrap();
    assert_eq!(copies.len(), 1);
    assert_eq!(tagged_copies(&path, "bak-newer"), copies);
    assert!(tagged_copies(&path, "newer").is_empty());
    assert_eq!(std::fs::read_to_string(&copies[0]).unwrap(), raw);
    assert_eq!(std::fs::read_to_string(&bak).unwrap(), raw);
}

#[test]
fn prepare_replace_copies_both_newer_primary_and_newer_backup() {
    let dir = TempDir::new("prep_both");
    let path = dir.save_path();
    std::fs::write(&path, newer_json(1)).unwrap();
    std::fs::write(SaveData::backup_path(&path), newer_json(2)).unwrap();
    let copies = SaveData::prepare_replace(&path, true).unwrap();
    assert_eq!(copies.len(), 2);
    assert_eq!(tagged_copies(&path, "newer").len(), 1);
    assert_eq!(tagged_copies(&path, "bak-newer").len(), 1);
}

#[test]
fn prepare_replace_ignores_current_version_backup() {
    let dir = TempDir::new("prep_current_bak");
    let path = dir.save_path();
    std::fs::write(&path, "{ not json").unwrap();
    std::fs::write(
        SaveData::backup_path(&path),
        versioned_json(3, SAVE_SCHEMA_VERSION, false),
    )
    .unwrap();
    let before = dir_entries(&dir.0);
    assert!(SaveData::prepare_replace(&path, false).unwrap().is_empty());
    assert_eq!(dir_entries(&dir.0), before);
}

#[test]
fn prepare_replace_aborts_without_new_files_when_copy_fails() {
    let dir = TempDir::new("prep_fail");
    let path = dir.save_path();
    // A directory cannot be copied as a file: the required copy fails.
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(SaveData::backup_path(&path), newer_json(2)).unwrap();
    let before = dir_entries(&dir.0);
    assert!(SaveData::prepare_replace(&path, true).is_err());
    assert_eq!(
        dir_entries(&dir.0),
        before,
        "an aborted replacement leaves no copies"
    );
    assert!(path.is_dir());
}

#[test]
fn prepare_replace_with_missing_primary_returns_empty() {
    let dir = TempDir::new("prep_missing");
    let path = dir.save_path();
    assert!(SaveData::prepare_replace(&path, true).unwrap().is_empty());
    assert!(dir_entries(&dir.0).is_empty());
}

#[test]
fn is_newer_version_distinguishes_newer_current_and_unreadable() {
    let dir = TempDir::new("is_newer");
    let path = dir.save_path();
    assert!(!SaveData::is_newer_version(&path));
    std::fs::write(&path, newer_json(1)).unwrap();
    assert!(SaveData::is_newer_version(&path));
    std::fs::write(&path, versioned_json(1, SAVE_SCHEMA_VERSION, false)).unwrap();
    assert!(!SaveData::is_newer_version(&path));
    std::fs::write(&path, "{ not json").unwrap();
    assert!(!SaveData::is_newer_version(&path));
}

#[test]
fn resolve_read_only_detailed_reports_the_corrupt_copy() {
    let dir = TempDir::new("detailed");
    let path = dir.save_path();
    std::fs::write(&path, "{ not json").unwrap();
    assert_eq!(
        SaveData::resolve_read_only_detailed(&path),
        (ReadOnlyResolution::Lift, true)
    );
    let gone = TempDir::new("detailed_gone");
    assert_eq!(
        SaveData::resolve_read_only_detailed(&gone.save_path()),
        (ReadOnlyResolution::Lift, false)
    );
}

#[test]
fn must_preserve_backup_only_for_a_newer_backup() {
    let dir = TempDir::new("must_preserve");
    let path = dir.save_path();
    let bak = SaveData::backup_path(&path);
    assert!(!SaveData::must_preserve_backup(&path));
    std::fs::write(&bak, versioned_json(1, SAVE_SCHEMA_VERSION, false)).unwrap();
    assert!(!SaveData::must_preserve_backup(&path));
    std::fs::write(&bak, "{ not json").unwrap();
    assert!(!SaveData::must_preserve_backup(&path));
    std::fs::write(&bak, newer_json(1)).unwrap();
    assert!(SaveData::must_preserve_backup(&path));
}

#[test]
fn prepare_replace_removes_the_first_copy_when_the_second_fails() {
    let dir = TempDir::new("prep_partial");
    let path = dir.save_path();
    std::fs::write(&path, newer_json(1)).unwrap();
    std::fs::write(SaveData::backup_path(&path), newer_json(2)).unwrap();
    // Directories squat on every `.bak-newer-<secs>` name the second copy could pick (the
    // clock may tick during the call), so the primary copy succeeds and the backup copy fails.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    for offset in 0..5 {
        let blocker = path.with_extension(format!("json.bak-newer-{}", now + offset));
        std::fs::create_dir_all(&blocker).unwrap();
    }
    let before = dir_entries(&dir.0);
    assert!(SaveData::prepare_replace(&path, true).is_err());
    assert!(
        tagged_copies(&path, "newer").is_empty(),
        "the first copy is rolled back"
    );
    assert_eq!(dir_entries(&dir.0), before);
}

#[test]
fn tampered_expedition_save_is_sanitized_and_does_not_freeze_or_overflow() {
    let dir = TempDir::new("tampered_expedition");
    let path = dir.save_path();
    let data = GameDataBundle::load_bundled(Language::En).unwrap();
    let mut save = SaveData::new_game(&data.balance, Language::En);
    save.active_expedition = Some(wyrdstone_core::ExpeditionRun {
        wave: 0,
        extra_moves: u32::MAX,
        score_bonus_pct: u32::MAX,
        starting_boosters: u32::MAX,
        fury_headstart: u32::MAX,
        pending_boons: vec![wyrdstone_core::ExpeditionBoon::StartingBooster; 20],
        ..Default::default()
    });
    save.save_to_path(&path).unwrap();

    let loaded = match SaveData::load_with_recovery(&path) {
        LoadOutcome::Loaded(s) => s,
        other => panic!("expected Loaded, got {other:?}"),
    };
    let mut run = loaded.active_expedition.expect("expedition preserved");
    assert_eq!(run.wave, 1);
    assert!(run.starting_boosters <= 64);
    assert!(run.extra_moves <= data.balance.expedition.moves_cap);
    assert!(run.score_bonus_pct <= wyrdstone_core::expedition::MAX_EXPEDITION_SCORE_BONUS_PCT);
    assert!(run.fury_headstart <= wyrdstone_core::FURY_CHARGE_MAX);
    assert!(run.pending_boons.len() <= data.balance.expedition.boon_choices);

    let mut rng = rand::rngs::StdRng::seed_from_u64(9);
    let session = wyrdstone_core::LevelSession::start_expedition_wave_with(
        &run,
        &data.balance.expedition,
        None,
        wyrdstone_core::CityBonuses::default(),
        wyrdstone_core::PatronGod::Thor,
        &mut rng,
    );
    assert_eq!(session.level.id, "expedition_w1");
    run.wave = u32::MAX;
    run.apply_boon(
        wyrdstone_core::ExpeditionBoon::StartingBooster,
        &mut save.city,
    );
    assert_eq!(run.wave, u32::MAX);
}

#[test]
fn preserve_copy_within_same_second_does_not_overwrite() {
    let dir = TempDir::new("preserve_same_sec");
    let path = dir.save_path();
    std::fs::write(&path, newer_json(1)).unwrap();
    let first = SaveData::preserve_copy(&path, "newer").unwrap();
    std::fs::write(&path, newer_json(2)).unwrap();
    let second = SaveData::preserve_copy(&path, "newer").unwrap();
    assert_ne!(first, second);
    assert_eq!(std::fs::read_to_string(&first).unwrap(), newer_json(1));
    assert_eq!(std::fs::read_to_string(&second).unwrap(), newer_json(2));
}
