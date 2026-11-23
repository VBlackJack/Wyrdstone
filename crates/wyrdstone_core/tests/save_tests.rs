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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use wyrdstone_core::save::LoadOutcome;
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
        LoadOutcome::ReadOnly { reason } => assert!(!reason.is_empty()),
        other => panic!("expected ReadOnly, got {other:?}"),
    }
    assert!(path.is_dir(), "the unreadable entry must not be touched");
    assert!(SaveData::backup_unreadable(&path).is_err());
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
