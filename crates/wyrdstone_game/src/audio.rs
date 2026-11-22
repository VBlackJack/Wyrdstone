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

use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;
use std::f32::consts::TAU;

const SAMPLE_RATE: u32 = 22_050;
const SFX_VOLUME: f32 = 0.55;
const BGM_VOLUME: f32 = 0.22;

/// D-minor pentatonic frequencies (Hz) for escalating combo chimes.
const COMBO_NOTES_HZ: [f32; 6] = [293.66, 349.23, 392.00, 440.00, 523.25, 587.33];

/// Named sound effects available across scenes.
#[derive(Debug, Clone, Copy)]
pub enum SfxKind {
    Swap,
    Invalid,
    MatchCombo(u32),
    ToolBlast,
    RelicDrop,
    Upgrade,
    Victory,
    Defeat,
}

/// Marker component for the looping background music entity.
#[derive(Component)]
pub struct BgmTrack;

/// Holds synthesized in-memory WAV audio handles and mute state.
#[derive(Resource)]
pub struct SoundBank {
    pub muted: bool,
    pub swap: Handle<AudioSource>,
    pub invalid: Handle<AudioSource>,
    pub combo_chimes: Vec<Handle<AudioSource>>,
    pub tool_blast: Handle<AudioSource>,
    pub relic_drop: Handle<AudioSource>,
    pub upgrade: Handle<AudioSource>,
    pub victory: Handle<AudioSource>,
    pub defeat: Handle<AudioSource>,
}

/// Synthesizes all Nordic SFX and ambient music at startup and begins the BGM loop.
pub fn init_sound_bank(mut commands: Commands, mut audio_assets: ResMut<Assets<AudioSource>>) {
    let swap = add_wav(&mut audio_assets, synth_stone_slide(0.11, 210.0, 320.0));
    let invalid = add_wav(&mut audio_assets, synth_stone_slide(0.14, 150.0, 95.0));

    let combo_chimes = COMBO_NOTES_HZ
        .iter()
        .map(|&freq| add_wav(&mut audio_assets, synth_harp_chord(0.32, &[freq, freq * 1.5])))
        .collect();

    let tool_blast = add_wav(&mut audio_assets, synth_thunder_strike(0.38));
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

    commands.spawn((
        BgmTrack,
        AudioPlayer::new(bgm_fjord.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Loop,
            volume: Volume::new(BGM_VOLUME),
            ..default()
        },
    ));

    commands.insert_resource(SoundBank {
        muted: false,
        swap,
        invalid,
        combo_chimes,
        tool_blast,
        relic_drop,
        upgrade,
        victory,
        defeat,
    });
}

/// Plays a one-shot sound effect if audio is not muted.
pub fn play_sfx(commands: &mut Commands, bank: &SoundBank, kind: SfxKind) {
    if bank.muted {
        return;
    }
    let handle = match kind {
        SfxKind::Swap => bank.swap.clone(),
        SfxKind::Invalid => bank.invalid.clone(),
        SfxKind::MatchCombo(combo) => {
            let idx = combo.saturating_sub(1).min((bank.combo_chimes.len() - 1) as u32) as usize;
            bank.combo_chimes[idx].clone()
        }
        SfxKind::ToolBlast => bank.tool_blast.clone(),
        SfxKind::RelicDrop => bank.relic_drop.clone(),
        SfxKind::Upgrade => bank.upgrade.clone(),
        SfxKind::Victory => bank.victory.clone(),
        SfxKind::Defeat => bank.defeat.clone(),
    };

    commands.spawn((
        AudioPlayer::new(handle),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::new(SFX_VOLUME),
            ..default()
        },
    ));
}

/// Toggles global mute and updates the background music sink volume.
pub fn toggle_mute(bank: &mut SoundBank, bgm_sinks: &mut Query<&mut AudioSink, With<BgmTrack>>) {
    bank.muted = !bank.muted;
    let target_vol = if bank.muted { 0.0 } else { BGM_VOLUME };
    for sink in bgm_sinks.iter_mut() {
        sink.set_volume(target_vol);
    }
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
            let s = ((TAU * freq * t).sin() * 0.7 + (TAU * freq * 2.0 * t).sin() * 0.2) * env * 0.45;
            out[start + j] = (out[start + j] + s).clamp(-0.95, 0.95);
        }
    }
    out
}

fn synth_nordic_ambient_loop(duration_secs: f32) -> Vec<f32> {
    let n = (duration_secs * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(n);
    // Warm D2 + A2 Nordic drone with gentle slow LFO and sparse harp plucks.
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

fn encode_pcm16_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_bytes = (samples.len() * 2) as u32;
    let file_size = 36 + data_bytes;
    let mut buf = Vec::with_capacity(44 + data_bytes as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // PCM chunk size
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    buf.extend_from_slice(&1u16.to_le_bytes()); // Mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // Byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // Block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_bytes.to_le_bytes());
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let pcm = (clamped * i16::MAX as f32) as i16;
        buf.extend_from_slice(&pcm.to_le_bytes());
    }
    buf
}
