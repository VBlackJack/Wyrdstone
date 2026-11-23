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

//! High-resolution procedural SDF textures for Match-3 VFX, sanctuary ambience, and Fjord weather.

use crate::theme::{VFX_LARGE_TEX_SIZE, VFX_TEX_SIZE};
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

const AURORA_TEX_W: usize = 256;
const AURORA_TEX_H: usize = 64;
const ICON_TEX_SIZE: usize = 64;
const SLICE_TEX_SIZE: usize = 128;

/// Handles to all procedural VFX, ambient sprite, 9-slice UI frame, and resource icon textures.
#[derive(Resource)]
pub struct VfxTextures {
    pub spark_glow: Handle<Image>,
    pub soft_orb: Handle<Image>,
    pub shockwave_ring: Handle<Image>,
    pub lightning_beam: Handle<Image>,
    pub runic_vortex: Handle<Image>,
    pub aurora_ribbon: Handle<Image>,
    pub board_frame: Handle<Image>,
    pub shimmer_band: Handle<Image>,
    pub water_glint: Handle<Image>,
    pub panel_frame_9slice: Handle<Image>,
    pub button_gold_9slice: Handle<Image>,
    pub icon_wood: Handle<Image>,
    pub icon_stone: Handle<Image>,
    pub icon_gold: Handle<Image>,
    pub icon_mead: Handle<Image>,
    pub icon_energy: Handle<Image>,
    pub icon_star: Handle<Image>,
    pub ruin_cairn: Handle<Image>,
}

/// PreStartup system generating all smooth SDF VFX, 9-slice UI, and icon textures once at boot.
pub fn init_vfx_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let textures = VfxTextures {
        spark_glow: images.add(build_spark_glow_image(VFX_TEX_SIZE)),
        soft_orb: images.add(build_soft_orb_image(VFX_TEX_SIZE)),
        shockwave_ring: images.add(build_shockwave_ring_image(VFX_LARGE_TEX_SIZE)),
        lightning_beam: images.add(build_lightning_beam_image(VFX_TEX_SIZE, VFX_LARGE_TEX_SIZE)),
        runic_vortex: images.add(build_runic_vortex_image(VFX_LARGE_TEX_SIZE)),
        aurora_ribbon: images.add(build_aurora_ribbon_image(AURORA_TEX_W, AURORA_TEX_H)),
        board_frame: images.add(build_board_frame_image(VFX_LARGE_TEX_SIZE)),
        shimmer_band: images.add(build_shimmer_band_image(VFX_TEX_SIZE)),
        water_glint: images.add(build_water_glint_image(AURORA_TEX_W, AURORA_TEX_H)),
        panel_frame_9slice: images.add(build_panel_9slice_image(SLICE_TEX_SIZE)),
        button_gold_9slice: images.add(build_gold_button_9slice_image(SLICE_TEX_SIZE)),
        icon_wood: images.add(build_wood_icon_image(ICON_TEX_SIZE)),
        icon_stone: images.add(build_stone_icon_image(ICON_TEX_SIZE)),
        icon_gold: images.add(build_gold_icon_image(ICON_TEX_SIZE)),
        icon_mead: images.add(build_mead_icon_image(ICON_TEX_SIZE)),
        icon_energy: images.add(build_energy_icon_image(ICON_TEX_SIZE)),
        icon_star: images.add(build_star_icon_image(ICON_TEX_SIZE)),
        ruin_cairn: images.add(build_ruin_cairn_image(128, 84)),
    };
    commands.insert_resource(textures);
}

fn finish_linear_image(width: usize, height: usize, pixels: Vec<u8>) -> Image {
    let mut img = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::linear();
    img
}

/// Four-pointed radiant star with a soft Gaussian glow halo for crisp burst particles.
fn build_spark_glow_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let half = (size as f32 - 1.0) * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 - half) / half;
            let ny = (y as f32 - half) / half;
            let dist = (nx * nx + ny * ny).sqrt();
            let halo = (-dist * dist * 4.2).exp();
            let core = (-dist * dist * 24.0).exp();
            let cross_h = (-ny.abs() * 14.0).exp() * (-nx.abs() * 2.4).exp();
            let cross_v = (-nx.abs() * 14.0).exp() * (-ny.abs() * 2.4).exp();
            let rays = (cross_h + cross_v) * 0.78;
            let edge_fade = (1.0 - ((dist - 0.82) / 0.18).clamp(0.0, 1.0)).powf(1.5);
            let intensity = ((halo * 0.62 + core * 0.95 + rays) * edge_fade).clamp(0.0, 1.0);
            let whiteness = (core * 0.85 + rays * 0.35).clamp(0.0, 1.0);
            let c = (210.0 + whiteness * 45.0).clamp(0.0, 255.0) as u8;
            let a = (intensity * 255.0) as u8;
            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[c, c, c.saturating_add(10), a]);
        }
    }
    finish_linear_image(size, size, px)
}

/// Smooth radial orb for torchlight halos, snow, embers, and subtle tile flashes.
fn build_soft_orb_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let half = (size as f32 - 1.0) * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 - half) / half;
            let ny = (y as f32 - half) / half;
            let dist = (nx * nx + ny * ny).sqrt();
            let falloff = (1.0 - dist).clamp(0.0, 1.0);
            let glow = falloff * falloff * (3.0 - 2.0 * falloff);
            let hot_core = (-dist * dist * 10.0).exp() * 0.45;
            let alpha =
                ((glow * 0.82 + hot_core) * (1.0 - dist.clamp(0.0, 1.0)).signum()).clamp(0.0, 1.0);
            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    finish_linear_image(size, size, px)
}

/// Expanding circular shockwave ring with bright rim and soft inner/outer aura.
fn build_shockwave_ring_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let half = (size as f32 - 1.0) * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 - half) / half;
            let ny = (y as f32 - half) / half;
            let dist = (nx * nx + ny * ny).sqrt();
            let ring_dist = (dist - 0.72).abs();
            let outer_ring = (-ring_dist * ring_dist * 165.0).exp();
            let soft_halo = (-ring_dist * ring_dist * 24.0).exp() * 0.55;
            let inner_echo = (-(dist - 0.50).powi(2) * 190.0).exp() * 0.35;
            let edge_fade = (1.0 - ((dist - 0.90) / 0.10).clamp(0.0, 1.0)).powf(1.5);
            let alpha = ((outer_ring + soft_halo + inner_echo) * edge_fade).clamp(0.0, 1.0);
            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    finish_linear_image(size, size, px)
}

/// Jagged vertical lightning plasma beam with bright white filament and wide electric glow.
fn build_lightning_beam_image(width: usize, height: usize) -> Image {
    let mut px = vec![0u8; width * height * 4];
    let half_w = (width as f32 - 1.0) * 0.5;
    for y in 0..height {
        let v = y as f32 / (height as f32 - 1.0);
        // End-cap fade so beams blend cleanly into the board edges.
        let end_fade = (v * 8.0).min(1.0) * ((1.0 - v) * 8.0).min(1.0);
        // Layered deterministic zig-zag spine for natural lightning crackle.
        let spine_main =
            (v * 23.0).sin() * 0.12 + (v * 57.0 + 1.3).sin() * 0.06 + (v * 113.0).cos() * 0.025;
        let spine_branch = (v * 31.0 + 2.1).cos() * 0.18 - (v * 71.0).sin() * 0.05;
        for x in 0..width {
            let nx = (x as f32 - half_w) / half_w;
            let d_main = (nx - spine_main).abs();
            let d_branch = (nx - spine_branch).abs();
            let core = (-d_main * d_main * 170.0).exp();
            let halo = (-d_main * d_main * 11.0).exp() * 0.68;
            let branch = (-d_branch * d_branch * 240.0).exp() * 0.52;
            let edge_x = (1.0 - ((nx.abs() - 0.78) / 0.22).clamp(0.0, 1.0)).powf(1.4);
            let alpha = ((core + halo + branch) * end_fade * edge_x).clamp(0.0, 1.0);
            let idx = (y * width + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    finish_linear_image(width, height, px)
}

/// Concentric Norse runic mandala with 8 radiant rays for Patron Fury activations.
fn build_runic_vortex_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let half = (size as f32 - 1.0) * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 - half) / half;
            let ny = (y as f32 - half) / half;
            let dist = (nx * nx + ny * ny).sqrt();
            let angle = ny.atan2(nx);

            let outer_ring = (-(dist - 0.80).powi(2) * 280.0).exp();
            let inner_ring = (-(dist - 0.46).powi(2) * 320.0).exp() * 0.85;
            let rune_ticks = if (0.50..=0.76).contains(&dist) {
                let spoke = (angle * 12.0).cos().abs().powf(14.0);
                spoke * 0.65
            } else {
                0.0
            };
            let rays =
                (angle * 8.0 + dist * 2.2).cos().max(0.0).powf(6.0) * (-dist * 1.4).exp() * 0.72;
            let center_glow = (-dist * dist * 9.0).exp() * 0.58;
            let edge_fade = (1.0 - ((dist - 0.88) / 0.12).clamp(0.0, 1.0)).powf(1.5);
            let alpha = ((outer_ring + inner_ring + rune_ticks + rays + center_glow) * edge_fade)
                .clamp(0.0, 1.0);
            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    finish_linear_image(size, size, px)
}

/// Soft multi-curtain Aurora Borealis ribbon with feathered alpha edges for Fjord and Title skies.
fn build_aurora_ribbon_image(width: usize, height: usize) -> Image {
    let mut px = vec![0u8; width * height * 4];
    for y in 0..height {
        let v = y as f32 / (height as f32 - 1.0);
        for x in 0..width {
            let u = x as f32 / (width as f32 - 1.0);
            // Smooth horizontal taper at both ends.
            let x_env = (u * std::f32::consts::PI).sin().powf(1.4);
            // Undulating wave center along Y.
            let wave_y = 0.52
                + (u * std::f32::consts::TAU * 1.5).sin() * 0.14
                + (u * std::f32::consts::TAU * 3.0 + 1.1).cos() * 0.05;
            let dy = v - wave_y;
            // Brighter bottom curtain fading softly upward.
            let curtain = if dy < 0.0 {
                (-dy * dy * 10.5).exp()
            } else {
                (-dy * dy * 26.0).exp()
            };
            // Vertical shimmer streaks like real polar auroras.
            let streaks = 0.72
                + 0.28 * (u * 38.0 + v * 3.0).sin().powi(2)
                + 0.15 * (u * 71.0 - v * 2.0).cos().powi(2);
            let alpha = (x_env * curtain * streaks * 0.92).clamp(0.0, 1.0);
            let idx = (y * width + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    finish_linear_image(width, height, px)
}

/// Sculpted dark slate board backdrop with beveled gold-bronze border and corner rivets.
fn build_board_frame_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let border_outer = 6.0;
    let border_gold = 10.0;
    let border_inner = 15.0;
    let corner_r = 16.0;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32;
            let fy = y as f32;
            // Rounded rectangle signed distance from outer edge.
            let half = s * 0.5;
            let qx = (fx - half).abs() - (half - corner_r);
            let qy = (fy - half).abs() - (half - corner_r);
            let outside = qx.max(0.0).hypot(qy.max(0.0));
            let inside = qx.max(qy).min(0.0);
            let sdf = outside + inside - corner_r;

            if sdf > 1.0 {
                continue;
            }
            let edge_aa = (1.0 - sdf.clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let depth = -sdf;
            let top_light = (1.0 - fy / s) * 0.35 + (1.0 - fx / s) * 0.15;

            let (r, g, b, a) = if depth < border_outer {
                // Dark carved outer stone rim
                (
                    20.0 + top_light * 22.0,
                    28.0 + top_light * 26.0,
                    40.0 + top_light * 30.0,
                    245.0,
                )
            } else if depth < border_gold {
                // Metallic bronze-gold trim
                let t = ((depth - border_outer) / (border_gold - border_outer) - 0.5).abs() * 2.0;
                let sheen = (1.0 - t) * 48.0 + top_light * 44.0;
                (
                    165.0 + sheen,
                    128.0 + sheen * 0.85,
                    58.0 + sheen * 0.45,
                    250.0,
                )
            } else if depth < border_inner {
                // Recessed inner bevel shadow
                (14.0, 20.0, 30.0, 242.0)
            } else {
                // Deep translucent slate interior with subtle radial vignette
                let nx = (fx - half) / half;
                let ny = (fy - half) / half;
                let vig = (nx * nx + ny * ny).sqrt().clamp(0.0, 1.0);
                (
                    13.0 + (1.0 - vig) * 10.0,
                    20.0 + (1.0 - vig) * 13.0,
                    31.0 + (1.0 - vig) * 18.0,
                    224.0 + vig * 18.0,
                )
            };

            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[
                r.clamp(0.0, 255.0) as u8,
                g.clamp(0.0, 255.0) as u8,
                b.clamp(0.0, 255.0) as u8,
                (a * edge_aa).clamp(0.0, 255.0) as u8,
            ]);
        }
    }
    finish_linear_image(size, size, px)
}

/// Tilted diagonal specular sheen band that sweeps smoothly across boosters and special runes.
fn build_shimmer_band_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let half = (size as f32 - 1.0) * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 - half) / half;
            let ny = (y as f32 - half) / half;
            // Diagonal band centered along nx + ny = 0, soft radial mask so corners stay clear.
            let diag = (nx * 0.78 + ny * 0.62).abs();
            let band = (-diag * diag * 28.0).exp();
            let core = (-diag * diag * 110.0).exp() * 0.65;
            let radial = (1.0 - (nx * nx + ny * ny).sqrt().clamp(0.0, 1.0)).powf(0.8);
            let alpha = ((band * 0.65 + core) * radial).clamp(0.0, 1.0);
            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[255, 252, 232, (alpha * 255.0) as u8]);
        }
    }
    finish_linear_image(size, size, px)
}

/// Soft horizontal water ripple highlight for Fjord bay aurora reflections.
fn build_water_glint_image(width: usize, height: usize) -> Image {
    let mut px = vec![0u8; width * height * 4];
    for y in 0..height {
        let v = y as f32 / (height as f32 - 1.0);
        let y_env = (v * std::f32::consts::PI).sin().powf(1.8);
        let ripple_row = (v * std::f32::consts::TAU * 6.0).sin().max(0.0).powf(2.2);
        for x in 0..width {
            let u = x as f32 / (width as f32 - 1.0);
            let x_env = (u * std::f32::consts::PI).sin().powf(1.5);
            let wave = 0.55 + 0.45 * (u * 26.0 + v * 9.0).sin().powi(2);
            let alpha = (x_env * y_env * (0.25 + ripple_row * 0.75) * wave).clamp(0.0, 1.0);
            let idx = (y * width + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[255, 255, 255, (alpha * 255.0) as u8]);
        }
    }
    finish_linear_image(width, height, px)
}

/// Sculpted 9-slice Nordic obsidian-slate UI panel with beveled bronze-gold trim and corner studs.
fn build_panel_9slice_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let half = s * 0.5;
    let corner_r = 12.0;

    let studs = [
        (11.0_f32, 11.0_f32),
        (s - 11.0, 11.0),
        (11.0, s - 11.0),
        (s - 11.0, s - 11.0),
    ];

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let qx = (fx - half).abs() - (half - corner_r);
            let qy = (fy - half).abs() - (half - corner_r);
            let sdf = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - corner_r;
            if sdf > 1.0 {
                continue;
            }
            let aa = (1.0 - sdf.clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let depth = -sdf;
            let top_sheen = (1.0 - fy / s).powf(1.6);

            let (mut r, mut g, mut b, a) = if depth < 2.2 {
                // Dark carved outer shadow rim
                (14.0, 20.0, 30.0, 248.0)
            } else if depth < 4.8 {
                // Warm antique bronze-gold filigree border
                let ridge = 1.0 - ((depth - 3.5).abs() / 1.3).clamp(0.0, 1.0);
                (
                    138.0 + ridge * 68.0 + top_sheen * 24.0,
                    112.0 + ridge * 54.0 + top_sheen * 18.0,
                    64.0 + ridge * 26.0,
                    250.0,
                )
            } else if depth < 7.2 {
                // Inner bevel recess
                (18.0, 26.0, 39.0, 244.0)
            } else {
                // Subtle slate vertical gradient
                (
                    22.0 + top_sheen * 14.0,
                    31.0 + top_sheen * 18.0,
                    46.0 + top_sheen * 22.0,
                    236.0,
                )
            };

            // Embossed golden corner rivets
            for &(sx, sy) in &studs {
                let d = (fx - sx).hypot(fy - sy);
                if d < 3.4 {
                    let h = (1.0 - d / 3.4).powf(0.8);
                    r = r * (1.0 - h) + 232.0 * h;
                    g = g * (1.0 - h) + 190.0 * h;
                    b = b * (1.0 - h) + 92.0 * h;
                }
            }

            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[
                r.clamp(0.0, 255.0) as u8,
                g.clamp(0.0, 255.0) as u8,
                b.clamp(0.0, 255.0) as u8,
                (a * aa).clamp(0.0, 255.0) as u8,
            ]);
        }
    }
    finish_linear_image(size, size, px)
}

/// Sculpted 9-slice forged gold primary button with warm bevel highlight.
fn build_gold_button_9slice_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let half = s * 0.5;
    let corner_r = 12.0;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let qx = (fx - half).abs() - (half - corner_r);
            let qy = (fy - half).abs() - (half - corner_r);
            let sdf = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - corner_r;
            if sdf > 1.0 {
                continue;
            }
            let aa = (1.0 - sdf.clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let depth = -sdf;
            let top_grad = 1.0 - (fy / s);

            let (r, g, b, a) = if depth < 2.2 {
                (92.0, 62.0, 18.0, 252.0)
            } else if depth < 5.0 {
                (252.0, 224.0, 134.0, 255.0)
            } else {
                (
                    214.0 + top_grad * 36.0,
                    158.0 + top_grad * 46.0,
                    48.0 + top_grad * 34.0,
                    255.0,
                )
            };

            let idx = (y * size + x) * 4;
            px[idx..idx + 4].copy_from_slice(&[
                r.clamp(0.0, 255.0) as u8,
                g.clamp(0.0, 255.0) as u8,
                b.clamp(0.0, 255.0) as u8,
                (a * aa).clamp(0.0, 255.0) as u8,
            ]);
        }
    }
    finish_linear_image(size, size, px)
}

fn blend_icon_px(px: &mut [u8], size: usize, x: usize, y: usize, rgba: [u8; 4]) {
    if x >= size || y >= size {
        return;
    }
    let idx = (y * size + x) * 4;
    let sa = rgba[3] as f32 / 255.0;
    let da = px[idx + 3] as f32 / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.001 {
        return;
    }
    for c in 0..3 {
        px[idx + c] = ((rgba[c] as f32 * sa + px[idx + c] as f32 * da * (1.0 - sa)) / oa)
            .clamp(0.0, 255.0) as u8;
    }
    px[idx + 3] = (oa * 255.0).clamp(0.0, 255.0) as u8;
}

/// Painted-style Nordic timber log icon for Wood.
fn build_wood_icon_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let c = s * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 + 0.5 - c) / (s * 0.44);
            let ny = (y as f32 + 0.5 - c) / (s * 0.44);
            // Tilted timber cylinder body + warm cut end-cap
            let rx = nx * 0.82 - ny * 0.57;
            let ry = nx * 0.57 + ny * 0.82;
            let body_sdf = (rx.abs() - 0.72).max(ry.abs() - 0.36);
            if body_sdf < 0.08 {
                let aa = ((0.08 - body_sdf) / 0.10).clamp(0.0, 1.0);
                let grain = (rx * 18.0 + (ry * 6.0).sin() * 2.0).sin() * 12.0;
                let shade = (1.0 - (ry + 0.36) / 0.72).clamp(0.0, 1.0);
                let r = (108.0 + shade * 68.0 + grain).clamp(0.0, 255.0) as u8;
                let g = (66.0 + shade * 46.0 + grain * 0.6).clamp(0.0, 255.0) as u8;
                let b = (36.0 + shade * 24.0).clamp(0.0, 255.0) as u8;
                blend_icon_px(&mut px, size, x, y, [r, g, b, (255.0 * aa) as u8]);
            }
            // Bright circular wood rings on top-right cut face
            let ex = (rx - 0.66) / 0.24;
            let ey = ry / 0.36;
            let ed = ex.hypot(ey);
            if ed < 1.06 {
                let aa = ((1.06 - ed) / 0.12).clamp(0.0, 1.0);
                let ring = (ed * 14.0).cos() * 14.0;
                let r = (205.0 + ring).clamp(0.0, 255.0) as u8;
                let g = (158.0 + ring * 0.8).clamp(0.0, 255.0) as u8;
                let b = (94.0 + ring * 0.5).clamp(0.0, 255.0) as u8;
                blend_icon_px(&mut px, size, x, y, [r, g, b, (255.0 * aa) as u8]);
            }
            // Small emerald leaf sprout on the log
            let lx = (nx + 0.15) / 0.32;
            let ly = (ny + 0.52) / 0.20;
            let ld = lx.hypot(ly);
            if ld < 1.0 {
                let aa = ((1.0 - ld) / 0.18).clamp(0.0, 1.0);
                blend_icon_px(&mut px, size, x, y, [78, 198, 108, (255.0 * aa) as u8]);
            }
        }
    }
    finish_linear_image(size, size, px)
}

/// Faceted carved slate-granite boulder icon for Stone.
fn build_stone_icon_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let c = s * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 + 0.5 - c) / (s * 0.42);
            let ny = (y as f32 + 0.5 - c) / (s * 0.42);
            // Hexagonal carved stone silhouette
            let ax = nx.abs();
            let ay = ny.abs();
            let hex = (ax * 0.866 + ay * 0.5).max(ay) - 0.88;
            if hex < 0.08 {
                let aa = ((0.08 - hex) / 0.10).clamp(0.0, 1.0);
                let top_facet: f32 = if ny < -0.15 && nx < 0.25 {
                    1.22
                } else if nx > 0.15 && ny > 0.0 {
                    0.72
                } else {
                    0.96
                };
                let rim: f32 = if hex > -0.10 { 0.65 } else { 1.0 };
                let r = (138.0 * top_facet * rim).clamp(0.0, 255.0) as u8;
                let g = (146.0 * top_facet * rim).clamp(0.0, 255.0) as u8;
                let b = (176.0 * top_facet * rim).clamp(0.0, 255.0) as u8;
                blend_icon_px(&mut px, size, x, y, [r, g, b, (255.0 * aa) as u8]);
            }
        }
    }
    finish_linear_image(size, size, px)
}

/// Embossed Nordic solar gold coin icon for Gold.
fn build_gold_icon_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let c = s * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 + 0.5 - c) / (s * 0.42);
            let ny = (y as f32 + 0.5 - c) / (s * 0.42);
            let d = nx.hypot(ny);
            if d < 1.04 {
                let aa = ((1.04 - d) / 0.10).clamp(0.0, 1.0);
                let rim = (d - 0.78).abs() < 0.12;
                let sheen = ((1.0 - (nx + ny) * 0.55) * 0.55).clamp(0.0, 1.0);
                let diamond = (nx.abs() + ny.abs()) < 0.42;
                let boost = if rim || diamond { 1.18 } else { 0.94 };
                let r = (224.0 * boost + sheen * 34.0).clamp(0.0, 255.0) as u8;
                let g = (168.0 * boost + sheen * 52.0).clamp(0.0, 255.0) as u8;
                let b = (36.0 * boost + sheen * 44.0).clamp(0.0, 255.0) as u8;
                blend_icon_px(&mut px, size, x, y, [r, g, b, (255.0 * aa) as u8]);
            }
        }
    }
    finish_linear_image(size, size, px)
}

/// Amber Viking mead tankard/horn icon with frothy white crest for Mead.
fn build_mead_icon_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let c = s * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 + 0.5 - c) / (s * 0.42);
            let ny = (y as f32 + 0.5 - c) / (s * 0.42);
            // Tapered wooden tankard body
            let half_w = 0.56 - (ny + 0.6) * 0.08;
            let mug_sdf = (nx.abs() - half_w).max((ny - 0.12).abs() - 0.62);
            if mug_sdf < 0.06 {
                let aa = ((0.06 - mug_sdf) / 0.08).clamp(0.0, 1.0);
                let band = (ny + 0.25).abs() < 0.09 || (ny - 0.48).abs() < 0.09;
                let shade = 1.0 - (nx + 0.5) * 0.35;
                let (r, g, b) = if band {
                    (192.0 * shade, 166.0 * shade, 118.0 * shade)
                } else {
                    (226.0 * shade, 144.0 * shade, 42.0 * shade)
                };
                blend_icon_px(
                    &mut px,
                    size,
                    x,
                    y,
                    [
                        r.clamp(0.0, 255.0) as u8,
                        g.clamp(0.0, 255.0) as u8,
                        b.clamp(0.0, 255.0) as u8,
                        (255.0 * aa) as u8,
                    ],
                );
            }
            // Frothy mead foam cap on top
            let f1 = (nx + 0.22).hypot(ny + 0.55);
            let f2 = (nx - 0.18).hypot(ny + 0.58);
            let f3 = nx.hypot(ny + 0.64);
            let foam_d = f1.min(f2).min(f3);
            if foam_d < 0.34 {
                let aa = ((0.34 - foam_d) / 0.08).clamp(0.0, 1.0);
                blend_icon_px(&mut px, size, x, y, [252, 246, 228, (255.0 * aa) as u8]);
            }
        }
    }
    finish_linear_image(size, size, px)
}

/// Luminous cyan-blue Thor lightning bolt icon for Endurance/Energy.
fn build_energy_icon_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let c = s * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 + 0.5 - c) / (s * 0.44);
            let ny = (y as f32 + 0.5 - c) / (s * 0.44);
            // Upper-left to center-right bolt + center-left to bottom-right bolt
            let upper = (-0.92..=0.08).contains(&ny)
                && nx >= (-0.34 - ny * 0.42)
                && nx <= (0.28 - ny * 0.12);
            let lower = (-0.08..=0.92).contains(&ny)
                && nx >= (-0.18 - ny * 0.12)
                && nx <= (0.44 - ny * 0.48);
            if upper || lower {
                let glow = 1.0 - (nx * nx + ny * ny).sqrt() * 0.25;
                let r = (108.0 + glow * 85.0).clamp(0.0, 255.0) as u8;
                let g = (224.0 + glow * 31.0).clamp(0.0, 255.0) as u8;
                blend_icon_px(&mut px, size, x, y, [r, g, 255, 255]);
            } else {
                let halo = (-((nx * nx + ny * ny) * 2.8)).exp() * 0.38;
                if halo > 0.04 {
                    blend_icon_px(&mut px, size, x, y, [72, 212, 255, (halo * 255.0) as u8]);
                }
            }
        }
    }
    finish_linear_image(size, size, px)
}

/// Five-pointed sculpted golden Norse mastery star for Victory/Defeat celebrations.
fn build_star_icon_image(size: usize) -> Image {
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let c = s * 0.5;
    for y in 0..size {
        for x in 0..size {
            let nx = (x as f32 + 0.5 - c) / (s * 0.44);
            let ny = (y as f32 + 0.5 - c) / (s * 0.44);
            let r = nx.hypot(ny);
            // Polar 5-pointed star boundary oriented with top point straight up (-PI/2).
            let theta = ny.atan2(nx) + std::f32::consts::FRAC_PI_2;
            let sector = std::f32::consts::TAU / 5.0;
            let local = ((theta.rem_euclid(sector)) - sector * 0.5).abs();
            let k = (local / (sector * 0.5)).clamp(0.0, 1.0);
            let boundary = 0.42 + (1.0 - k) * 0.54;
            let d = r - boundary;
            if d < 0.08 {
                let aa = ((0.08 - d) / 0.10).clamp(0.0, 1.0);
                let ridge = (1.0 - (local / (sector * 0.5))).clamp(0.0, 1.0);
                let top_lit = (1.0 - ny * 0.45).clamp(0.5, 1.35);
                let cr = (232.0 * top_lit + ridge * 24.0).clamp(0.0, 255.0) as u8;
                let cg = (180.0 * top_lit + ridge * 52.0).clamp(0.0, 255.0) as u8;
                let cb = (42.0 + ridge * 78.0).clamp(0.0, 255.0) as u8;
                blend_icon_px(&mut px, size, x, y, [cr, cg, cb, (255.0 * aa) as u8]);
            }
        }
    }
    finish_linear_image(size, size, px)
}

/// Low mossy Viking stone foundation / ruined cairn shown on unbuilt (Tier 0) Fjord plots
/// so constructing Tier 1 visibly transforms the landscape.
fn build_ruin_cairn_image(width: usize, height: usize) -> Image {
    let mut px = vec![0u8; width * height * 4];
    let stones: &[(f32, f32, f32, f32, f32)] = &[
        (0.28, 0.72, 0.16, 0.12, 0.92),
        (0.46, 0.76, 0.20, 0.13, 1.05),
        (0.68, 0.72, 0.17, 0.12, 0.88),
        (0.36, 0.58, 0.15, 0.11, 1.08),
        (0.56, 0.60, 0.16, 0.12, 0.96),
        (0.47, 0.44, 0.11, 0.14, 1.12),
    ];

    for y in 0..height {
        let ny = y as f32 / height as f32;
        for x in 0..width {
            let nx = x as f32 / width as f32;
            // Soft elliptical dirt/shadow patch under the ruined stones.
            let gx = (nx - 0.5) / 0.44;
            let gy = (ny - 0.76) / 0.20;
            let gd = gx.hypot(gy);
            if gd < 1.0 {
                let ga = (1.0 - gd * gd).powf(1.5) * 145.0;
                let idx = (y * width + x) * 4;
                px[idx..idx + 4].copy_from_slice(&[20, 28, 36, ga as u8]);
            }

            for &(cx, cy, rx, ry, shade) in stones {
                let dx = (nx - cx) / rx;
                let dy = (ny - cy) / ry;
                let d = dx.hypot(dy);
                if d < 1.05 {
                    let aa = ((1.05 - d) / 0.14).clamp(0.0, 1.0);
                    let top_lit = (1.0 - dy * 0.45).clamp(0.55, 1.35);
                    let snow_cap = dy < -0.35 && d < 0.92;
                    let (r, g, b) = if snow_cap {
                        (188.0, 212.0, 232.0)
                    } else {
                        (
                            92.0 * shade * top_lit,
                            104.0 * shade * top_lit,
                            120.0 * shade * top_lit,
                        )
                    };
                    let idx = (y * width + x) * 4;
                    let sa = aa;
                    let da = px[idx + 3] as f32 / 255.0;
                    let oa = sa + da * (1.0 - sa);
                    if oa > 0.001 {
                        px[idx] = ((r * sa + px[idx] as f32 * da * (1.0 - sa)) / oa)
                            .clamp(0.0, 255.0) as u8;
                        px[idx + 1] = ((g * sa + px[idx + 1] as f32 * da * (1.0 - sa)) / oa)
                            .clamp(0.0, 255.0) as u8;
                        px[idx + 2] = ((b * sa + px[idx + 2] as f32 * da * (1.0 - sa)) / oa)
                            .clamp(0.0, 255.0) as u8;
                        px[idx + 3] = (oa * 255.0).clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    finish_linear_image(width, height, px)
}
