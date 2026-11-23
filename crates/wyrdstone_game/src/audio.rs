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

//! Procedural Nordic WAV synthesizer for SFX and ambient fjord music.

use crate::state::GameContext;
use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use bevy::window::AppLifecycle;
use std::f32::consts::TAU;

const SAMPLE_RATE: u32 = 22_050;

/// D-minor pentatonic frequencies (Hz) for escalating combo chimes.
const COMBO_NOTES_HZ: [f32; 6] = [293.66, 349.23, 392.00, 440.00, 523.25, 587.33];

/// Named sound effects available across scenes.
#[derive(Debug, Clone, Copy)]
pub enum SfxKind {
    Swap,
    Invalid,
    MatchCombo(u32),
    ToolBlast,
    FuryUnleash,
    BossRiposte,
    RelicDrop,
    Upgrade,
    Victory,
    Defeat,
}

/// Marker component for the looping background music entity.
#[derive(Component)]
pub struct BgmTrack;

/// Holds synthesized in-memory WAV audio handles, mute state, and persisted volumes.
#[derive(Resource)]
pub struct SoundBank {
    pub muted: bool,
    pub bgm_volume: f32,
    pub sfx_volume: f32,
    pub swap: Handle<AudioSource>,
    pub invalid: Handle<AudioSource>,
    pub combo_chimes: Vec<Handle<AudioSource>>,
    pub tool_blast: Handle<AudioSource>,
    pub fury_unleash: Handle<AudioSource>,
    pub boss_riposte: Handle<AudioSource>,
    pub relic_drop: Handle<AudioSource>,
    pub upgrade: Handle<AudioSource>,
    pub victory: Handle<AudioSource>,
    pub defeat: Handle<AudioSource>,
    pub bgm_fjord: Handle<AudioSource>,
    pub bgm_match3: Handle<AudioSource>,
    pub bgm_boss: Handle<AudioSource>,
}

/// Synthesizes all Nordic SFX and ambient music at startup and begins the BGM loop.
pub fn init_sound_bank(
    mut commands: Commands,
    ctx: Res<GameContext>,
    mut audio_assets: ResMut<Assets<AudioSource>>,
) {
    let swap = add_wav(&mut audio_assets, synth_stone_slide(0.11, 210.0, 320.0));
    let invalid = add_wav(&mut audio_assets, synth_stone_slide(0.14, 150.0, 95.0));

    let combo_chimes = COMBO_NOTES_HZ
        .iter()
        .map(|&freq| {
            add_wav(
                &mut audio_assets,
                synth_harp_chord(0.32, &[freq, freq * 1.5]),
            )
        })
        .collect();

    let tool_blast = add_wav(&mut audio_assets, synth_thunder_strike(0.38));
    let fury_unleash = add_wav(&mut audio_assets, synth_fury_horn(0.62));
    let boss_riposte = add_wav(&mut audio_assets, synth_boss_roar(0.46));
    let relic_drop = add_wav(
        &mut audio_assets,
        synth_arpeggio(0.45, &[440.0, 523.25, 659.25, 880.0]),
    );
    let upgrade = add_wav(
        &mut audio_assets,
        synth_arpeggio(0.48, &[293.66, 440.0, 587.33]),
    );
    let victory = add_wav(
        &mut audio_assets,
        synth_arpeggio(0.75, &[293.66, 349.23, 440.0, 587.33]),
    );
    let defeat = add_wav(
        &mut audio_assets,
        synth_arpeggio(0.65, &[293.66, 261.63, 220.0, 174.61]),
    );
    let bgm_fjord = add_wav(&mut audio_assets, synth_nordic_ambient_loop(8.0));
    let bgm_match3 = add_wav(&mut audio_assets, synth_nordic_battle_loop(6.4));
    let bgm_boss = add_wav(&mut audio_assets, synth_nordic_boss_loop(6.4));

    let initial_vol = if ctx.audio_muted { 0.0 } else { ctx.bgm_volume };
    commands.spawn((
        BgmTrack,
        AudioPlayer::new(bgm_fjord.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::new(initial_vol),
            ..default()
        },
    ));

    commands.insert_resource(SoundBank {
        muted: ctx.audio_muted,
        bgm_volume: ctx.bgm_volume,
        sfx_volume: ctx.sfx_volume,
        swap,
        invalid,
        combo_chimes,
        tool_blast,
        fury_unleash,
        boss_riposte,
        relic_drop,
        upgrade,
        victory,
        defeat,
        bgm_fjord,
        bgm_match3,
        bgm_boss,
    });
}

/// Switches background music to the battle or Boss Raid track when entering Match-3.
pub fn switch_bgm_to_match3(
    mut commands: Commands,
    ctx: Res<GameContext>,
    bank: Res<SoundBank>,
    tracks: Query<Entity, With<BgmTrack>>,
) {
    let is_boss = ctx
        .active_session
        .as_ref()
        .and_then(|s| s.boss.as_ref())
        .is_some();
    let track = if is_boss {
        bank.bgm_boss.clone()
    } else {
        bank.bgm_match3.clone()
    };
    replace_bgm(&mut commands, &tracks, track, bank.muted, bank.bgm_volume);
}

/// Restores the calm Fjord ambient loop when leaving Match-3.
pub fn switch_bgm_to_fjord(
    mut commands: Commands,
    bank: Res<SoundBank>,
    tracks: Query<Entity, With<BgmTrack>>,
) {
    replace_bgm(
        &mut commands,
        &tracks,
        bank.bgm_fjord.clone(),
        bank.muted,
        bank.bgm_volume,
    );
}

fn replace_bgm(
    commands: &mut Commands,
    tracks: &Query<Entity, With<BgmTrack>>,
    handle: Handle<AudioSource>,
    muted: bool,
    bgm_volume: f32,
) {
    for entity in tracks {
        commands.entity(entity).despawn_recursive();
    }
    commands.spawn((
        BgmTrack,
        AudioPlayer::new(handle),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::new(if muted { 0.0 } else { bgm_volume }),
            ..default()
        },
    ));
}

/// Plays a one-shot sound effect if audio is not muted.
pub fn play_sfx(commands: &mut Commands, bank: &SoundBank, kind: SfxKind) {
    if bank.muted || bank.sfx_volume <= 0.001 {
        return;
    }
    let handle = match kind {
        SfxKind::Swap => bank.swap.clone(),
        SfxKind::Invalid => bank.invalid.clone(),
        SfxKind::MatchCombo(combo) => {
            let idx = combo
                .saturating_sub(1)
                .min((bank.combo_chimes.len() - 1) as u32) as usize;
            bank.combo_chimes[idx].clone()
        }
        SfxKind::ToolBlast => bank.tool_blast.clone(),
        SfxKind::FuryUnleash => bank.fury_unleash.clone(),
        SfxKind::BossRiposte => bank.boss_riposte.clone(),
        SfxKind::RelicDrop => bank.relic_drop.clone(),
        SfxKind::Upgrade => bank.upgrade.clone(),
        SfxKind::Victory => bank.victory.clone(),
        SfxKind::Defeat => bank.defeat.clone(),
    };

    commands.spawn((
        AudioPlayer::new(handle),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::new(bank.sfx_volume),
            ..default()
        },
    ));
}

/// Toggles global mute, updates the BGM sink, and persists the preference.
pub fn toggle_mute(
    ctx: &mut GameContext,
    bank: &mut SoundBank,
    bgm_sinks: &mut Query<&mut AudioSink, With<BgmTrack>>,
) {
    bank.muted = !bank.muted;
    ctx.audio_muted = bank.muted;
    let target_vol = if bank.muted { 0.0 } else { bank.bgm_volume };
    for sink in bgm_sinks.iter_mut() {
        sink.set_volume(target_vol);
    }
    ctx.persist();
}

/// Selectable BGM volumes, quietest first; the last step is the BGM maximum (shown as 100%).
pub const BGM_VOLUME_STEPS: [f32; 5] = [0.0, 0.12, 0.22, 0.35, 0.50];
/// Selectable SFX volumes, quietest first.
pub const SFX_VOLUME_STEPS: [f32; 5] = [0.0, 0.25, 0.55, 0.80, 1.0];
/// Tolerance when matching the current volume against a step.
const VOLUME_STEP_EPSILON: f32 = 0.02;
/// Percentage scale used when displaying a volume.
const PERCENT_SCALE: f32 = 100.0;

/// BGM volume as a percentage of the BGM maximum (the last step), clamped to 100.
pub fn bgm_volume_percent(v: f32) -> u32 {
    let max = BGM_VOLUME_STEPS[BGM_VOLUME_STEPS.len() - 1];
    ((v / max * PERCENT_SCALE).round() as u32).min(PERCENT_SCALE as u32)
}

/// Cycles BGM volume through `BGM_VOLUME_STEPS` and persists.
pub fn cycle_bgm_volume(
    ctx: &mut GameContext,
    bank: &mut SoundBank,
    bgm_sinks: &mut Query<&mut AudioSink, With<BgmTrack>>,
) {
    let steps = BGM_VOLUME_STEPS;
    let next = steps
        .iter()
        .copied()
        .find(|&v| v > bank.bgm_volume + VOLUME_STEP_EPSILON)
        .unwrap_or(steps[0]);
    bank.bgm_volume = next;
    ctx.bgm_volume = next;
    let target_vol = if bank.muted { 0.0 } else { bank.bgm_volume };
    for sink in bgm_sinks.iter_mut() {
        sink.set_volume(target_vol);
    }
    ctx.persist();
}

/// Cycles SFX volume in steps and persists.
pub fn cycle_sfx_volume(ctx: &mut GameContext, bank: &mut SoundBank) {
    let steps = SFX_VOLUME_STEPS;
    let next = steps
        .iter()
        .copied()
        .find(|&v| v > bank.sfx_volume + VOLUME_STEP_EPSILON)
        .unwrap_or(steps[0]);
    bank.sfx_volume = next;
    ctx.sfx_volume = next;
    ctx.persist();
}

fn add_wav(assets: &mut Assets<AudioSource>, samples: Vec<f32>) -> Handle<AudioSource> {
    let bytes = encode_pcm16_wav(&samples, SAMPLE_RATE);
    assets.add(AudioSource {
        bytes: bytes.into(),
    })
}

fn synth_stone_slide(duration_secs: f32, start_hz: f32, end_hz: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let freq = start_hz + (end_hz - start_hz) * t;
        phase += TAU * freq / SAMPLE_RATE as f32;
        let env = (1.0 - t).powf(1.6) * (t * 14.0).min(1.0);
        let sample = (phase.sin() * 0.7 + (phase * 2.1).sin() * 0.2) * env * 0.45;
        out.push(sample);
    }
    out
}

fn synth_harp_chord(duration_secs: f32, freqs: &[f32]) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let norm = i as f32 / n as f32;
        let env = (1.0 - norm).powf(2.0) * (norm * 25.0).min(1.0);
        let mut sum = 0.0f32;
        for &f in freqs {
            sum += (TAU * f * t).sin() * 0.65 + (TAU * f * 2.0 * t).sin() * 0.25;
        }
        out.push((sum / freqs.len().max(1) as f32) * env * 0.55);
    }
    out
}

fn synth_thunder_strike(duration_secs: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let mut seed: u32 = 0x5EED_1234;
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let norm = i as f32 / n as f32;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((seed >> 16) as f32 / 32768.0) - 1.0;
        let boom = (TAU * (92.0 - 45.0 * norm) * t).sin();
        let ring = (TAU * 520.0 * t).sin() * (1.0 - norm).powf(4.0);
        let env = (1.0 - norm).powf(1.8);
        out.push((boom * 0.55 + noise * 0.30 + ring * 0.25) * env * 0.65);
    }
    out
}

fn synth_arpeggio(duration_secs: f32, notes: &[f32]) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = vec![0.0f32; n];
    let step = n / notes.len().max(1);
    for (idx, &freq) in notes.iter().enumerate() {
        let start = idx * step;
        let note_len = (n - start).min(step * 2);
        for j in 0..note_len {
            let t = j as f32 / SAMPLE_RATE as f32;
            let norm = j as f32 / note_len as f32;
            let env = (1.0 - norm).powf(1.7) * (norm * 24.0).min(1.0);
            let s =
                ((TAU * freq * t).sin() * 0.7 + (TAU * freq * 2.0 * t).sin() * 0.2) * env * 0.45;
            out[start + j] = (out[start + j] + s).clamp(-0.95, 0.95);
        }
    }
    out
}

fn synth_nordic_ambient_loop(duration_secs: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let harp_events: [(f32, f32); 6] = [
        (0.5, 293.66),
        (1.8, 440.00),
        (3.2, 349.23),
        (4.6, 523.25),
        (6.0, 392.00),
        (7.1, 440.00),
    ];

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let loop_fade = {
            let edge = 0.25;
            if t < edge {
                t / edge
            } else if t > duration_secs - edge {
                (duration_secs - t) / edge
            } else {
                1.0
            }
        };
        let swell = 0.75 + 0.25 * (TAU * t / duration_secs).sin();
        let drone = ((TAU * 73.42 * t).sin() * 0.45
            + (TAU * 110.0 * t).sin() * 0.30
            + (TAU * 146.83 * t).sin() * 0.18)
            * swell
            * 0.32;

        let mut harp = 0.0f32;
        for &(start_t, freq) in &harp_events {
            let dt = t - start_t;
            if (0.0..1.4).contains(&dt) {
                let env = (1.0 - dt / 1.4).powf(2.4) * (dt * 30.0).min(1.0);
                harp += ((TAU * freq * dt).sin() * 0.7 + (TAU * freq * 2.0 * dt).sin() * 0.2)
                    * env
                    * 0.26;
            }
        }
        out.push(((drone + harp) * loop_fade).clamp(-0.9, 0.9));
    }
    out
}

fn synth_nordic_battle_loop(duration_secs: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let beat_secs = 0.4f32;
    let melody: [f32; 16] = [
        293.66, 349.23, 440.00, 392.00, 293.66, 440.00, 523.25, 440.00, 349.23, 392.00, 440.00,
        523.25, 587.33, 523.25, 440.00, 392.00,
    ];

    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let beat_idx = (t / beat_secs) as usize;
        let beat_t = t - (beat_idx as f32) * beat_secs;

        let strong = beat_idx.is_multiple_of(2);
        let drum_freq = if strong {
            88.0 - 38.0 * (beat_t / beat_secs).min(1.0)
        } else {
            135.0 - 45.0 * (beat_t / beat_secs).min(1.0)
        };
        let drum_env = (1.0 - (beat_t / 0.22).min(1.0)).powf(2.2);
        let drum = (TAU * drum_freq * beat_t).sin() * drum_env * if strong { 0.42 } else { 0.22 };

        let note_hz = melody[beat_idx % melody.len()];
        let harp_env = (1.0 - (beat_t / beat_secs).min(1.0)).powf(1.8) * (beat_t * 35.0).min(1.0);
        let harp = ((TAU * note_hz * beat_t).sin() * 0.65
            + (TAU * note_hz * 2.0 * beat_t).sin() * 0.20)
            * harp_env
            * 0.28;

        let bass = (TAU * 73.42 * t).sin() * 0.14;

        let edge = 0.08;
        let fade = if t < edge {
            t / edge
        } else if t > duration_secs - edge {
            (duration_secs - t) / edge
        } else {
            1.0
        };
        out.push(((drum + harp + bass) * fade).clamp(-0.9, 0.9));
    }
    out
}

fn synth_fury_horn(duration_secs: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let norm = i as f32 / n as f32;
        let env = (norm * 10.0).min(1.0) * (1.0 - norm).powf(1.25);
        let f1 = 146.83 + 22.0 * (norm * 2.0).min(1.0); // Rising D3 brass war-horn
        let f2 = f1 * 1.5; // Perfect fifth
        let f3 = f1 * 2.0; // Octave shimmer
        let s =
            (TAU * f1 * t).sin() * 0.50 + (TAU * f2 * t).sin() * 0.30 + (TAU * f3 * t).sin() * 0.20;
        out.push((s * env * 0.72).clamp(-0.95, 0.95));
    }
    out
}

fn synth_boss_roar(duration_secs: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let mut seed: u32 = 0xB055_7777;
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let norm = i as f32 / n as f32;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((seed >> 16) as f32 / 32768.0) - 1.0;
        let growl = (TAU * (82.0 - 34.0 * norm) * t).sin() * 0.55
            + (TAU * (116.5 - 40.0 * norm) * t).sin() * 0.30;
        let env = (norm * 18.0).min(1.0) * (1.0 - norm).powf(1.5);
        out.push(((growl + noise * 0.28) * env * 0.70).clamp(-0.95, 0.95));
    }
    out
}

fn synth_nordic_boss_loop(duration_secs: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    let beat_secs = 0.2f32; // Double-time war drums
    let melody: [f32; 16] = [
        293.66, 311.13, 293.66, 220.00, 349.23, 311.13, 293.66, 261.63, 440.00, 415.30, 349.23,
        311.13, 293.66, 261.63, 220.00, 293.66,
    ];
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let beat_idx = (t / beat_secs) as usize;
        let beat_t = t - (beat_idx as f32) * beat_secs;

        let heavy = beat_idx.is_multiple_of(4);
        let drum_freq = if heavy {
            76.0 - 32.0 * (beat_t / beat_secs).min(1.0)
        } else {
            118.0 - 40.0 * (beat_t / beat_secs).min(1.0)
        };
        let drum_env = (1.0 - (beat_t / 0.16).min(1.0)).powf(2.0);
        let drum = (TAU * drum_freq * beat_t).sin() * drum_env * if heavy { 0.48 } else { 0.26 };

        let note_hz = melody[(beat_idx / 2) % melody.len()];
        let horn_env = (1.0 - (beat_t / beat_secs).min(1.0)).powf(1.3);
        let horn = ((TAU * note_hz * t).sin() * 0.55 + (TAU * note_hz * 0.5 * t).sin() * 0.35)
            * horn_env
            * 0.28;
        let sub = (TAU * 55.0 * t).sin() * 0.18;

        let edge = 0.08;
        let fade = if t < edge {
            t / edge
        } else if t > duration_secs - edge {
            (duration_secs - t) / edge
        } else {
            1.0
        };
        out.push(((drum + horn + sub) * fade).clamp(-0.9, 0.9));
    }
    out
}

fn encode_pcm16_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_bytes = (samples.len() * 2) as u32;
    let file_size = 36 + data_bytes;
    let mut buf = Vec::with_capacity(44 + data_bytes as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes());
    buf.extend_from_slice(&16u16.to_le_bytes());
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes.to_le_bytes());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        buf.extend_from_slice(&pcm.to_le_bytes());
    }
    buf
}

/// Pauses background music and saves when the app is suspended, and resumes the music when the
/// app comes back. A muted track keeps a zero volume, so resuming it is always safe and keeps a
/// later unmute audible.
///
/// Known limitation: an in-progress Match-3 board is not persisted, only the city state.
pub fn handle_app_lifecycle(
    mut events: EventReader<AppLifecycle>,
    mut ctx: ResMut<GameContext>,
    bgm_sinks: Query<&AudioSink, With<BgmTrack>>,
) {
    for event in events.read() {
        match event {
            AppLifecycle::Suspended => {
                for sink in &bgm_sinks {
                    sink.pause();
                }
                ctx.persist();
            }
            AppLifecycle::WillResume | AppLifecycle::Running => {
                for sink in &bgm_sinks {
                    sink.play();
                }
            }
            AppLifecycle::Idle | AppLifecycle::WillSuspend => {}
        }
    }
}

#[cfg(test)]
mod volume_tests {
    use super::*;

    #[test]
    fn bgm_percent_spans_zero_to_hundred() {
        assert_eq!(bgm_volume_percent(0.0), 0);
        assert_eq!(bgm_volume_percent(0.25), 50);
        assert_eq!(bgm_volume_percent(0.50), 100);
    }

    #[test]
    fn bgm_percent_never_exceeds_hundred() {
        assert_eq!(bgm_volume_percent(2.0), 100);
    }

    #[test]
    fn every_bgm_step_has_distinct_percent() {
        let pcts: Vec<u32> = BGM_VOLUME_STEPS
            .iter()
            .map(|&v| bgm_volume_percent(v))
            .collect();
        assert_eq!(pcts, vec![0, 24, 44, 70, 100]);
    }
}
