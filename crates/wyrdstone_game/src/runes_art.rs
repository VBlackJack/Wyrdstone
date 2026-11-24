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

//! Procedural Nordic rune stone, obstacle, and booster textures generated at startup.

use crate::theme::{OVERLAY_TEX_SIZE, RUNE_TEX_SIZE};
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use wyrdstone_core::{Booster, RuneType};

const RUNE_FIRE_PNG: &[u8] = include_bytes!("../../../assets/sprites/rune_fire.png");
const RUNE_ICE_PNG: &[u8] = include_bytes!("../../../assets/sprites/rune_ice.png");
const RUNE_WOOD_PNG: &[u8] = include_bytes!("../../../assets/sprites/rune_wood.png");
const RUNE_STONE_PNG: &[u8] = include_bytes!("../../../assets/sprites/rune_stone.png");
const RUNE_GOLD_PNG: &[u8] = include_bytes!("../../../assets/sprites/rune_gold.png");
const RUNE_WATER_PNG: &[u8] = include_bytes!("../../../assets/sprites/rune_water.png");
const CRATE_WOOD_PNG: &[u8] = include_bytes!("../../../assets/sprites/crate_wood.png");
const CRATE_IRON_PNG: &[u8] = include_bytes!("../../../assets/sprites/crate_iron.png");
const RELIC_CHALICE_PNG: &[u8] = include_bytes!("../../../assets/sprites/relic_chalice.png");
const BOOSTER_BIFROST_PNG: &[u8] = include_bytes!("../../../assets/sprites/booster_bifrost.png");

/// Handles to all procedurally generated 2D board textures.
#[derive(Resource)]
pub struct RuneTextures {
    pub runes: HashMap<RuneType, Handle<Image>>,
    pub cell_bg: Handle<Image>,
    pub ice_top_layer1: Handle<Image>,
    pub ice_top_layer2: Handle<Image>,
    pub crate_layer1: Handle<Image>,
    pub crate_layer2: Handle<Image>,
    pub relic_icon: Handle<Image>,
    pub chain_overlay: Handle<Image>,
    pub booster_h: Handle<Image>,
    pub booster_v: Handle<Image>,
    pub booster_bomb: Handle<Image>,
    pub booster_bifrost: Handle<Image>,
    pub selection_ring: Handle<Image>,
    pub portal_ring: Handle<Image>,
    pub fog_overlay: Handle<Image>,
    pub colorblind_badges: HashMap<RuneType, Handle<Image>>,
}

impl RuneTextures {
    pub fn booster_handle(&self, booster: Booster) -> Option<Handle<Image>> {
        match booster {
            Booster::None => None,
            Booster::LineHorizontal => Some(self.booster_h.clone()),
            Booster::LineVertical => Some(self.booster_v.clone()),
            Booster::Bomb => Some(self.booster_bomb.clone()),
            Booster::BifrostShard => Some(self.booster_bifrost.clone()),
        }
    }
}

/// PreStartup system that loads the painted rune gemstones and builds board overlays.
pub fn init_rune_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut runes = HashMap::new();
    runes.insert(RuneType::Fire, images.add(decode_rune_png(RUNE_FIRE_PNG)));
    runes.insert(RuneType::Ice, images.add(decode_rune_png(RUNE_ICE_PNG)));
    runes.insert(RuneType::Wood, images.add(decode_rune_png(RUNE_WOOD_PNG)));
    runes.insert(RuneType::Stone, images.add(decode_rune_png(RUNE_STONE_PNG)));
    runes.insert(RuneType::Gold, images.add(decode_rune_png(RUNE_GOLD_PNG)));
    runes.insert(RuneType::Water, images.add(decode_rune_png(RUNE_WATER_PNG)));

    let mut colorblind_badges = HashMap::new();
    for rune in RuneType::ALL {
        colorblind_badges.insert(rune, images.add(build_colorblind_badge_image(rune)));
    }

    let textures = RuneTextures {
        runes,
        cell_bg: images.add(build_cell_bg_image()),
        ice_top_layer1: images.add(build_ice_top_frost_image(1)),
        ice_top_layer2: images.add(build_ice_top_frost_image(2)),
        crate_layer1: images.add(decode_rune_png(CRATE_WOOD_PNG)),
        crate_layer2: images.add(decode_rune_png(CRATE_IRON_PNG)),
        relic_icon: images.add(decode_rune_png(RELIC_CHALICE_PNG)),
        chain_overlay: images.add(build_chain_image()),
        booster_h: images.add(build_line_booster_image(true)),
        booster_v: images.add(build_line_booster_image(false)),
        booster_bomb: images.add(build_bomb_booster_image()),
        booster_bifrost: images.add(decode_rune_png(BOOSTER_BIFROST_PNG)),
        selection_ring: images.add(build_selection_ring_image()),
        portal_ring: images.add(build_portal_ring_image()),
        fog_overlay: images.add(build_fog_overlay_image()),
        colorblind_badges,
    };

    commands.insert_resource(textures);
}

fn new_canvas() -> Vec<u8> {
    vec![0u8; RUNE_TEX_SIZE * RUNE_TEX_SIZE * 4]
}

fn finish_image(pixels: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: RUNE_TEX_SIZE as u32,
            height: RUNE_TEX_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn set_px(pixels: &mut [u8], x: i32, y: i32, rgba: [u8; 4]) {
    let s = RUNE_TEX_SIZE as i32;
    if x < 0 || y < 0 || x >= s || y >= s {
        return;
    }
    let idx = ((y as usize) * RUNE_TEX_SIZE + (x as usize)) * 4;
    pixels[idx..idx + 4].copy_from_slice(&rgba);
}

fn draw_thick_line(
    pixels: &mut [u8],
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    radius: i32,
    rgba: [u8; 4],
) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = (x0 as f32 + (x1 - x0) as f32 * t).round() as i32;
        let cy = (y0 as f32 + (y1 - y0) as f32 * t).round() as i32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius {
                    set_px(pixels, cx + dx, cy + dy, rgba);
                }
            }
        }
    }
}

fn decode_rune_png(bytes: &[u8]) -> Image {
    Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::linear(),
        RenderAssetUsages::RENDER_WORLD,
    )
    .expect("Bundled rune PNG must decode cleanly")
}

fn new_overlay_canvas() -> Vec<u8> {
    vec![0u8; OVERLAY_TEX_SIZE * OVERLAY_TEX_SIZE * 4]
}

fn finish_overlay_image(pixels: Vec<u8>) -> Image {
    let mut img = Image::new(
        Extent3d {
            width: OVERLAY_TEX_SIZE as u32,
            height: OVERLAY_TEX_SIZE as u32,
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

fn blend_overlay_px(pixels: &mut [u8], x: usize, y: usize, rgba: [u8; 4]) {
    crate::art_util::blend_px(pixels, OVERLAY_TEX_SIZE, OVERLAY_TEX_SIZE, x, y, rgba);
}

type LineSeg = ((f32, f32), (f32, f32));

fn dist_to_segment(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len2 = dx * dx + dy * dy;
    if len2 <= 0.0001 {
        return (px - x0).hypot(py - y0);
    }
    let t = (((px - x0) * dx + (py - y0) * dy) / len2).clamp(0.0, 1.0);
    (px - (x0 + dx * t)).hypot(py - (y0 + dy * t))
}

fn rounded_box_sdf(px: f32, py: f32, half: f32, inset: f32, radius: f32) -> f32 {
    let extent = half - inset - radius;
    let qx = (px - half).abs() - extent;
    let qy = (py - half).abs() - extent;
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
}

fn build_cell_bg_image() -> Image {
    let mut px = new_overlay_canvas();
    let s = OVERLAY_TEX_SIZE as f32;
    let half = s * 0.5;
    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let sdf = rounded_box_sdf(fx, fy, half, 3.0, 10.0);
            if sdf > 1.2 {
                continue;
            }
            let aa = (1.0 - (sdf + 0.2).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let depth = -sdf;
            let light = (1.0 - fy / s) * 0.45 + (1.0 - fx / s) * 0.2;
            let nx = (fx - half) / half;
            let ny = (fy - half) / half;
            let radial = (nx * nx + ny * ny).sqrt().clamp(0.0, 1.0);
            let (r, g, b, a) = if depth < 3.5 {
                // Carved stone bevel rim
                (
                    52.0 + light * 32.0,
                    68.0 + light * 36.0,
                    92.0 + light * 42.0,
                    224.0,
                )
            } else if depth < 6.5 {
                // Subtle recessed groove
                (20.0, 28.0, 42.0, 232.0)
            } else {
                // Smooth dark slate interior with faint center illumination
                (
                    24.0 + (1.0 - radial) * 14.0 + light * 6.0,
                    34.0 + (1.0 - radial) * 18.0 + light * 7.0,
                    50.0 + (1.0 - radial) * 24.0 + light * 9.0,
                    220.0 + radial * 18.0,
                )
            };
            blend_overlay_px(
                &mut px,
                x,
                y,
                [
                    r.clamp(0.0, 255.0) as u8,
                    g.clamp(0.0, 255.0) as u8,
                    b.clamp(0.0, 255.0) as u8,
                    (a * aa).clamp(0.0, 255.0) as u8,
                ],
            );
        }
    }
    finish_overlay_image(px)
}

/// Translucent glacial ice cube drawn OVER the runestone with a clear center window
/// so the trapped rune glyph remains 100% readable while unmistakably encased in ice.
fn build_ice_top_frost_image(layers: u8) -> Image {
    let mut px = new_overlay_canvas();
    let s = OVERLAY_TEX_SIZE as f32;
    let half = s * 0.5;
    let wall_w: f32 = if layers >= 2 { 13.0 } else { 8.5 };
    let clear_radius: f32 = if layers >= 2 { 32.0 } else { 38.0 };
    let outer_radius: f32 = 58.0;

    let cracks: &[LineSeg] = if layers >= 2 {
        &[
            ((14.0, 22.0), (36.0, 34.0)),
            ((36.0, 34.0), (48.0, 18.0)),
            ((114.0, 102.0), (88.0, 88.0)),
            ((88.0, 88.0), (80.0, 112.0)),
            ((112.0, 26.0), (88.0, 44.0)),
            ((18.0, 104.0), (40.0, 84.0)),
        ]
    } else {
        &[]
    };

    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let sdf = rounded_box_sdf(fx, fy, half, 2.0, 12.0);
            if sdf > 1.2 {
                continue;
            }
            let aa = (1.0 - (sdf + 0.2).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            let depth = -sdf;
            let dist = (fx - half).hypot(fy - half);

            if depth < 2.2 {
                blend_overlay_px(&mut px, x, y, [78, 186, 238, (245.0 * aa) as u8]);
                continue;
            }

            if depth <= wall_w {
                let top_left = (fx + fy) < s;
                let bevel = 1.0 - (depth / wall_w);
                let (r, g, b, a) = if top_left {
                    if layers >= 2 {
                        (232.0 + bevel * 23.0, 250.0 + bevel * 5.0, 255.0, 238.0)
                    } else {
                        (212.0 + bevel * 28.0, 245.0 + bevel * 10.0, 255.0, 212.0)
                    }
                } else if layers >= 2 {
                    (88.0 + bevel * 36.0, 194.0 + bevel * 26.0, 244.0, 230.0)
                } else {
                    (82.0 + bevel * 28.0, 184.0 + bevel * 22.0, 234.0, 198.0)
                };
                blend_overlay_px(&mut px, x, y, [r as u8, g as u8, b as u8, (a * aa) as u8]);
                continue;
            }

            let frost_ring =
                ((dist - clear_radius) / (outer_radius - clear_radius)).clamp(0.0, 1.0);
            let center_tint = if layers >= 2 { 24.0 } else { 10.0 };
            let ring_alpha = frost_ring.powf(1.35) * if layers >= 2 { 190.0 } else { 138.0 };

            let diag = (fx + fy) - 70.0;
            let glare1 = (-diag * diag / 48.0).exp();
            let diag2 = (fx + fy) - 102.0;
            let glare2 = (-diag2 * diag2 / 26.0).exp() * 0.7;
            let glare = if dist > clear_radius * 0.72 {
                (glare1 + glare2) * 72.0
            } else {
                0.0
            };

            let alpha = (center_tint + ring_alpha + glare).clamp(0.0, 235.0) * aa;
            let r = (176.0 + frost_ring * 66.0 + glare).clamp(0.0, 255.0) as u8;
            let g = (236.0 + frost_ring * 18.0 + glare * 0.35).clamp(0.0, 255.0) as u8;
            blend_overlay_px(&mut px, x, y, [r, g, 255, alpha as u8]);

            // Specular highlight along top-left inner bevel
            let glint_h = dist_to_segment(fx, fy, 16.0, 16.0, 52.0, 16.0);
            let glint_v = dist_to_segment(fx, fy, 16.0, 16.0, 16.0, 48.0);
            let glint = (-glint_h.min(glint_v).powi(2) / 5.5).exp();
            if glint > 0.05 {
                blend_overlay_px(&mut px, x, y, [255, 255, 255, (glint * 225.0) as u8]);
            }

            for &((x0, y0), (x1, y1)) in cracks {
                let d_lo = dist_to_segment(fx, fy, x0, y0, x1, y1);
                let d_hi = dist_to_segment(fx, fy + 1.5, x0, y0, x1, y1);
                if d_lo < 2.8 {
                    let c_a = (1.0 - d_lo / 2.8).powf(1.5) * 225.0;
                    blend_overlay_px(&mut px, x, y, [68, 168, 224, c_a as u8]);
                }
                if d_hi < 1.6 {
                    let h_a = (1.0 - d_hi / 1.6).powf(1.3) * 245.0;
                    blend_overlay_px(&mut px, x, y, [248, 254, 255, h_a as u8]);
                }
            }
        }
    }

    finish_overlay_image(px)
}

/// Sculpted crossed iron chains with dark drop-shadow and metallic link highlights.
fn build_chain_image() -> Image {
    let mut px = new_overlay_canvas();
    let s = OVERLAY_TEX_SIZE as f32;
    let margin = 12.0;

    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let d1 = dist_to_segment(fx, fy, margin, margin, s - margin, s - margin);
            let d2 = dist_to_segment(fx, fy, s - margin, margin, margin, s - margin);
            let d1_sh = dist_to_segment(fx, fy - 3.0, margin, margin, s - margin, s - margin);
            let d2_sh = dist_to_segment(fx, fy - 3.0, s - margin, margin, margin, s - margin);

            let d_sh = d1_sh.min(d2_sh);
            if d_sh < 8.5 {
                let sh_a = (1.0 - d_sh / 8.5).powf(1.5) * 165.0;
                blend_overlay_px(&mut px, x, y, [10, 14, 20, sh_a as u8]);
            }

            let (d, along) = if d1 < d2 {
                (d1, (fx + fy) * std::f32::consts::FRAC_1_SQRT_2)
            } else {
                (d2, (fx - fy + s) * std::f32::consts::FRAC_1_SQRT_2)
            };

            if d < 6.8 {
                let link_wave = ((along * 0.34).sin()).abs();
                let profile = (1.0 - (d / 6.8).powi(2)).clamp(0.0, 1.0);
                let rim = if d > 4.4 { 0.45 } else { 1.0 };
                let metallic = (0.42 + profile * 0.58 * (0.72 + link_wave * 0.28)) * rim;
                let spec = (-((d - 1.4).powi(2)) / 2.2).exp() * link_wave;
                let r = (68.0 + metallic * 128.0 + spec * 72.0).clamp(0.0, 255.0) as u8;
                let g = (76.0 + metallic * 136.0 + spec * 74.0).clamp(0.0, 255.0) as u8;
                let b = (90.0 + metallic * 144.0 + spec * 80.0).clamp(0.0, 255.0) as u8;
                let aa = ((6.8 - d) / 1.4).clamp(0.0, 1.0);
                blend_overlay_px(&mut px, x, y, [r, g, b, (242.0 * aa) as u8]);
            }
        }
    }
    finish_overlay_image(px)
}

/// Radiant golden energy arrows with soft outer glow for Line Horizontal/Vertical boosters.
fn build_line_booster_image(horizontal: bool) -> Image {
    let mut px = new_overlay_canvas();
    let s = OVERLAY_TEX_SIZE as f32;
    let c = s * 0.5;
    let segments: &[LineSeg] = if horizontal {
        &[
            ((12.0, c), (s - 12.0, c)),
            ((28.0, c - 16.0), (12.0, c)),
            ((28.0, c + 16.0), (12.0, c)),
            ((s - 28.0, c - 16.0), (s - 12.0, c)),
            ((s - 28.0, c + 16.0), (s - 12.0, c)),
        ]
    } else {
        &[
            ((c, 12.0), (c, s - 12.0)),
            ((c - 16.0, 28.0), (c, 12.0)),
            ((c + 16.0, 28.0), (c, 12.0)),
            ((c - 16.0, s - 28.0), (c, s - 12.0)),
            ((c + 16.0, s - 28.0), (c, s - 12.0)),
        ]
    };

    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let mut min_d = f32::MAX;
            for &((x0, y0), (x1, y1)) in segments {
                min_d = min_d.min(dist_to_segment(fx, fy, x0, y0, x1, y1));
            }
            let halo = (-min_d * min_d / 64.0).exp() * 0.72;
            let core = (-min_d * min_d / 11.0).exp();
            let alpha = (halo + core).clamp(0.0, 1.0);
            if alpha > 0.01 {
                let r = (235.0 + core * 20.0).clamp(0.0, 255.0) as u8;
                let g = (176.0 + core * 79.0).clamp(0.0, 255.0) as u8;
                let b = (56.0 + core * 185.0).clamp(0.0, 255.0) as u8;
                blend_overlay_px(&mut px, x, y, [r, g, b, (alpha * 255.0) as u8]);
            }
        }
    }
    finish_overlay_image(px)
}

/// Glowing molten solar ring with fiery aura for Bomb boosters.
fn build_bomb_booster_image() -> Image {
    let mut px = new_overlay_canvas();
    let half = OVERLAY_TEX_SIZE as f32 * 0.5;
    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let dist = (fx - half).hypot(fy - half);
            let ring_d = (dist - 40.0).abs();
            let ring_core = (-ring_d * ring_d / 18.0).exp();
            let ring_halo = (-ring_d * ring_d / 110.0).exp() * 0.65;
            let inner_ember = (-dist * dist / 950.0).exp() * 0.25;
            let alpha = (ring_core + ring_halo + inner_ember).clamp(0.0, 1.0);
            if alpha > 0.01 {
                let r = 255u8;
                let g = (118.0 + ring_core * 124.0).clamp(0.0, 255.0) as u8;
                let b = (24.0 + ring_core * 140.0).clamp(0.0, 255.0) as u8;
                blend_overlay_px(&mut px, x, y, [r, g, b, (alpha * 255.0) as u8]);
            }
        }
    }
    finish_overlay_image(px)
}

/// Smooth rounded golden selection ring with luminous corner brackets.
fn build_selection_ring_image() -> Image {
    let mut px = new_overlay_canvas();
    let half = OVERLAY_TEX_SIZE as f32 * 0.5;
    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let sdf = rounded_box_sdf(fx, fy, half, 5.0, 14.0);
            let band = sdf.abs();
            let core = (-band * band / 7.5).exp();
            let halo = (-band * band / 42.0).exp() * 0.6;
            let alpha = (core + halo).clamp(0.0, 1.0);
            if alpha > 0.01 {
                let r = 255u8;
                let g = (205.0 + core * 48.0).clamp(0.0, 255.0) as u8;
                let b = (78.0 + core * 130.0).clamp(0.0, 255.0) as u8;
                blend_overlay_px(&mut px, x, y, [r, g, b, (alpha * 255.0) as u8]);
            }
        }
    }
    finish_overlay_image(px)
}

/// Smooth dual-ring arcane Bifrost portal vortex.
fn build_portal_ring_image() -> Image {
    let mut px = new_overlay_canvas();
    let half = OVERLAY_TEX_SIZE as f32 * 0.5;
    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let dx = fx - half;
            let dy = fy - half;
            let dist = dx.hypot(dy);
            let angle = dy.atan2(dx);
            let swirl = 0.8 + 0.2 * (angle * 6.0 + dist * 0.12).sin();

            let outer = (-(dist - 52.0).powi(2) / 26.0).exp() * swirl;
            let inner = (-(dist - 43.0).powi(2) / 22.0).exp();
            let aura = (-(dist - 48.0).powi(2) / 140.0).exp() * 0.45;
            let alpha = (outer + inner + aura).clamp(0.0, 1.0);
            if alpha > 0.01 {
                let r = (102.0 + outer * 95.0).clamp(0.0, 255.0) as u8;
                let g = (118.0 + inner * 124.0).clamp(0.0, 255.0) as u8;
                let b = 255u8;
                blend_overlay_px(&mut px, x, y, [r, g, b, (alpha * 245.0) as u8]);
            }
        }
    }
    finish_overlay_image(px)
}

/// Dense swirling Niflheim Fog of War cloud that conceals a tile until an adjacent match dispels it.
fn build_fog_overlay_image() -> Image {
    let mut px = new_overlay_canvas();
    let s = OVERLAY_TEX_SIZE as f32;
    let half = s * 0.5;
    for y in 0..OVERLAY_TEX_SIZE {
        for x in 0..OVERLAY_TEX_SIZE {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let sdf = rounded_box_sdf(fx, fy, half, 2.5, 14.0);
            if sdf > 2.0 {
                continue;
            }
            let edge_aa = (1.0 - ((sdf + 1.0) / 3.0).clamp(0.0, 1.0)).powf(1.3);
            let nx = (fx - half) / half;
            let ny = (fy - half) / half;
            let wave1 = ((fx * 0.085).sin() + (fy * 0.075).cos()) * 0.5;
            let wave2 = ((fx * 0.14 - fy * 0.11).sin()) * 0.5;
            let swirl = (wave1 * 0.6 + wave2 * 0.4) * 28.0;
            let dist = nx.hypot(ny);

            let mist_wisp = (-(ny - 0.18 + (nx * 3.0).sin() * 0.12).powi(2) * 18.0).exp()
                + (-(ny + 0.22 - (nx * 2.5).cos() * 0.10).powi(2) * 18.0).exp();

            let r = (34.0 + swirl * 0.7 + mist_wisp * 68.0).clamp(18.0, 195.0) as u8;
            let g = (48.0 + swirl * 0.9 + mist_wisp * 82.0).clamp(24.0, 215.0) as u8;
            let b = (72.0 + swirl * 1.1 + dist * 16.0 + mist_wisp * 90.0).clamp(40.0, 240.0) as u8;
            let alpha = ((236.0 + mist_wisp * 15.0).clamp(0.0, 250.0) * edge_aa) as u8;
            blend_overlay_px(&mut px, x, y, [r, g, b, alpha]);
        }
    }
    finish_overlay_image(px)
}

/// High-contrast geometric corner badge per rune type for colorblind accessibility.
fn build_colorblind_badge_image(rune: RuneType) -> Image {
    let mut px = new_canvas();
    let dark = [10, 14, 22, 235];
    let white = [255, 255, 255, 255];
    // Crisp dark circular backing in bottom-right corner (42..60, 42..60)
    for y in 40..62 {
        for x in 40..62 {
            let dx = x - 51;
            let dy = y - 51;
            if dx * dx + dy * dy <= 100 {
                set_px(&mut px, x, y, dark);
            }
        }
    }
    match rune {
        RuneType::Fire => {
            // Upward triangle
            draw_thick_line(&mut px, 51, 44, 45, 56, 1, white);
            draw_thick_line(&mut px, 51, 44, 57, 56, 1, white);
            draw_thick_line(&mut px, 45, 56, 57, 56, 1, white);
        }
        RuneType::Ice => {
            // Plus / snow cross
            draw_thick_line(&mut px, 51, 44, 51, 58, 1, white);
            draw_thick_line(&mut px, 44, 51, 58, 51, 1, white);
        }
        RuneType::Wood => {
            // Diamond
            draw_thick_line(&mut px, 51, 44, 58, 51, 1, white);
            draw_thick_line(&mut px, 58, 51, 51, 58, 1, white);
            draw_thick_line(&mut px, 51, 58, 44, 51, 1, white);
            draw_thick_line(&mut px, 44, 51, 51, 44, 1, white);
        }
        RuneType::Stone => {
            // Square
            draw_thick_line(&mut px, 46, 46, 56, 46, 1, white);
            draw_thick_line(&mut px, 56, 46, 56, 56, 1, white);
            draw_thick_line(&mut px, 56, 56, 46, 56, 1, white);
            draw_thick_line(&mut px, 46, 56, 46, 46, 1, white);
        }
        RuneType::Gold => {
            // Hollow circle
            for y in 44..=58 {
                for x in 44..=58 {
                    let d2 = (x - 51) * (x - 51) + (y - 51) * (y - 51);
                    if (20..=42).contains(&d2) {
                        set_px(&mut px, x, y, white);
                    }
                }
            }
        }
        RuneType::Water => {
            // X mark
            draw_thick_line(&mut px, 46, 46, 56, 56, 1, white);
            draw_thick_line(&mut px, 56, 46, 46, 56, 1, white);
        }
    }
    finish_image(px)
}
