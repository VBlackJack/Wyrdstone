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

use crate::theme::RUNE_TEX_SIZE;
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

fn draw_thick_line(pixels: &mut [u8], x0: i32, y0: i32, x1: i32, y1: i32, radius: i32, rgba: [u8; 4]) {
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

fn build_cell_bg_image() -> Image {
    let mut px = new_canvas();
    let s = RUNE_TEX_SIZE as i32;
    for y in 1..(s - 1) {
        for x in 1..(s - 1) {
            let border = x <= 2 || y <= 2 || x >= s - 3 || y >= s - 3;
            let rgba = if border {
                [56, 72, 94, 220]
            } else {
                [26, 36, 52, 235]
            };
            set_px(&mut px, x, y, rgba);
        }
    }
    finish_image(px)
}

/// Translucent glacial ice cube drawn OVER the runestone so the tile looks genuinely encased in ice.
fn build_ice_top_frost_image(layers: u8) -> Image {
    let mut px = new_canvas();
    let s = RUNE_TEX_SIZE as i32;

    // 1. Translucent frozen glass interior glaze (thicker & milkier for 2-layer glacial ice).
    let base_alpha: f32 = if layers >= 2 { 138.0 } else { 88.0 };
    let wall_w: i32 = if layers >= 2 { 6 } else { 4 };

    for y in 1..(s - 1) {
        for x in 1..(s - 1) {
            let left = x;
            let right = s - 1 - x;
            let top = y;
            let bottom = s - 1 - y;
            let edge = left.min(right).min(top).min(bottom);

            // Outer dark frost rim for crisp silhouette against any tile.
            if edge == 1 {
                set_px(&mut px, x, y, [78, 168, 214, 240]);
                continue;
            }

            // Thick 3D beveled ice cube walls: bright white-cyan on top/left, deep arctic blue on bottom/right.
            if edge <= wall_w {
                let top_left = left <= wall_w || top <= wall_w;
                let rgba = if top_left {
                    if layers >= 2 {
                        [236, 252, 255, 242]
                    } else {
                        [218, 248, 255, 215]
                    }
                } else if layers >= 2 {
                    [102, 198, 238, 232]
                } else {
                    [92, 186, 228, 195]
                };
                set_px(&mut px, x, y, rgba);
                continue;
            }

            // Subtle frost gradient stronger near corners & top-left specular sheen.
            let corner_dist = (left.min(right) + top.min(bottom)) as f32;
            let corner_frost = ((24.0 - corner_dist).max(0.0) / 24.0) * if layers >= 2 { 92.0 } else { 62.0 };

            // Glossy diagonal ice-glass reflection bands across the frozen face.
            let diag = x + y;
            let band1 = (diag - 36).abs() <= 4;
            let band2 = (diag - 48).abs() <= 2;
            let glare = if band1 {
                68.0
            } else if band2 {
                45.0
            } else {
                0.0
            };

            let alpha = (base_alpha + corner_frost + glare).clamp(0.0, 235.0) as u8;
            let r = (165.0 + glare * 1.2 + corner_frost * 0.7).clamp(0.0, 255.0) as u8;
            let g = (232.0 + glare * 0.4 + corner_frost * 0.3).clamp(0.0, 255.0) as u8;
            let b = 255u8;
            set_px(&mut px, x, y, [r, g, b, alpha]);
        }
    }

    // 2. Crisp specular highlight glint along top-left inner bevel.
    draw_thick_line(&mut px, 8, 8, 28, 8, 1, [255, 255, 255, 235]);
    draw_thick_line(&mut px, 8, 8, 8, 24, 1, [255, 255, 255, 235]);

    // 3. Layer 2 (Thick Glacial Ice): jagged internal fracture lines & frozen rime crystals.
    if layers >= 2 {
        let crack_hi = [245, 253, 255, 235];
        let crack_lo = [85, 178, 224, 210];
        // Jagged glacial crack from top-left to center-right
        draw_thick_line(&mut px, 10, 12, 26, 24, 1, crack_lo);
        draw_thick_line(&mut px, 26, 24, 38, 20, 1, crack_lo);
        draw_thick_line(&mut px, 38, 20, 54, 34, 1, crack_lo);
        draw_thick_line(&mut px, 10, 11, 26, 23, 0, crack_hi);
        draw_thick_line(&mut px, 26, 23, 38, 19, 0, crack_hi);
        draw_thick_line(&mut px, 38, 19, 54, 33, 0, crack_hi);
        // Secondary crack from bottom-left to center
        draw_thick_line(&mut px, 12, 52, 28, 38, 1, crack_lo);
        draw_thick_line(&mut px, 28, 38, 46, 50, 1, crack_lo);
        draw_thick_line(&mut px, 12, 51, 28, 37, 0, crack_hi);
        draw_thick_line(&mut px, 28, 37, 46, 49, 0, crack_hi);
    }

    finish_image(px)
}

fn build_chain_image() -> Image {
    let mut px = new_canvas();
    let iron = [185, 194, 208, 235];
    let shadow = [62, 68, 78, 235];
    draw_thick_line(&mut px, 8, 8, 56, 56, 3, shadow);
    draw_thick_line(&mut px, 56, 8, 8, 56, 3, shadow);
    draw_thick_line(&mut px, 8, 8, 56, 56, 2, iron);
    draw_thick_line(&mut px, 56, 8, 8, 56, 2, iron);
    finish_image(px)
}

fn build_line_booster_image(horizontal: bool) -> Image {
    let mut px = new_canvas();
    let glow = [255, 244, 150, 245];
    if horizontal {
        draw_thick_line(&mut px, 6, 32, 58, 32, 3, glow);
        draw_thick_line(&mut px, 14, 24, 6, 32, 2, glow);
        draw_thick_line(&mut px, 14, 40, 6, 32, 2, glow);
        draw_thick_line(&mut px, 50, 24, 58, 32, 2, glow);
        draw_thick_line(&mut px, 50, 40, 58, 32, 2, glow);
    } else {
        draw_thick_line(&mut px, 32, 6, 32, 58, 3, glow);
        draw_thick_line(&mut px, 24, 14, 32, 6, 2, glow);
        draw_thick_line(&mut px, 40, 14, 32, 6, 2, glow);
        draw_thick_line(&mut px, 24, 50, 32, 58, 2, glow);
        draw_thick_line(&mut px, 40, 50, 32, 58, 2, glow);
    }
    finish_image(px)
}

fn build_bomb_booster_image() -> Image {
    let mut px = new_canvas();
    let s = RUNE_TEX_SIZE as i32;
    let c = s / 2;
    for y in 0..s {
        for x in 0..s {
            let d2 = (x - c) * (x - c) + (y - c) * (y - c);
            if (17 * 17..=23 * 23).contains(&d2) {
                set_px(&mut px, x, y, [255, 165, 48, 245]);
            }
        }
    }
    finish_image(px)
}

fn build_selection_ring_image() -> Image {
    let mut px = new_canvas();
    let s = RUNE_TEX_SIZE as i32;
    for y in 1..(s - 1) {
        for x in 1..(s - 1) {
            let border = x <= 4 || y <= 4 || x >= s - 5 || y >= s - 5;
            if border {
                set_px(&mut px, x, y, [255, 224, 102, 255]);
            }
        }
    }
    finish_image(px)
}
