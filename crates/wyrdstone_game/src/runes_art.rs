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

use crate::theme::{rune_base_rgb, RUNE_TEX_SIZE};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;
use wyrdstone_core::{Booster, RuneType};

/// Handles to all procedurally generated 2D board textures.
#[derive(Resource)]
pub struct RuneTextures {
    pub runes: HashMap<RuneType, Handle<Image>>,
    pub cell_bg: Handle<Image>,
    pub ice_layer1: Handle<Image>,
    pub ice_layer2: Handle<Image>,
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

/// Startup system that builds all procedural rune and board textures.
pub fn init_rune_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut runes = HashMap::new();
    for rune in RuneType::ALL {
        let img = build_rune_stone_image(rune);
        runes.insert(rune, images.add(img));
    }

    let textures = RuneTextures {
        runes,
        cell_bg: images.add(build_cell_bg_image()),
        ice_layer1: images.add(build_ice_image(1)),
        ice_layer2: images.add(build_ice_image(2)),
        chain_overlay: images.add(build_chain_image()),
        booster_h: images.add(build_line_booster_image(true)),
        booster_v: images.add(build_line_booster_image(false)),
        booster_bomb: images.add(build_bomb_booster_image()),
        booster_bifrost: images.add(build_bifrost_image()),
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

fn build_rune_stone_image(rune: RuneType) -> Image {
    let mut px = new_canvas();
    let [br, bg, bb] = rune_base_rgb(rune);
    let s = RUNE_TEX_SIZE as i32;

    // Rounded carved stone tablet with subtle bevel lighting.
    for y in 4..(s - 4) {
        for x in 4..(s - 4) {
            let dx = (x - 4).min(s - 5 - x);
            let dy = (y - 4).min(s - 5 - y);
            if dx + dy < 7 {
                continue;
            }
            let edge = dx.min(dy);
            let shade = if edge <= 2 {
                if x < s / 2 || y < s / 2 { 1.22 } else { 0.68 }
            } else {
                let grad = 1.08 - (y as f32 / s as f32) * 0.22;
                grad
            };
            let r = (br as f32 * shade).clamp(0.0, 255.0) as u8;
            let g = (bg as f32 * shade).clamp(0.0, 255.0) as u8;
            let b = (bb as f32 * shade).clamp(0.0, 255.0) as u8;
            set_px(&mut px, x, y, [r, g, b, 255]);
        }
    }

    // Carved Elder Futhark rune symbol in luminous ivory.
    let glyph = [248, 245, 232, 255];
    match rune {
        // Fehu (Fire)
        RuneType::Fire => {
            draw_thick_line(&mut px, 24, 15, 24, 49, 2, glyph);
            draw_thick_line(&mut px, 24, 24, 42, 14, 2, glyph);
            draw_thick_line(&mut px, 24, 34, 40, 24, 2, glyph);
        }
        // Isa + crystal cross (Ice)
        RuneType::Ice => {
            draw_thick_line(&mut px, 32, 13, 32, 51, 2, glyph);
            draw_thick_line(&mut px, 19, 23, 45, 41, 2, glyph);
            draw_thick_line(&mut px, 45, 23, 19, 41, 2, glyph);
        }
        // Algiz (Wood / Yggdrasil)
        RuneType::Wood => {
            draw_thick_line(&mut px, 32, 14, 32, 50, 2, glyph);
            draw_thick_line(&mut px, 32, 32, 18, 16, 2, glyph);
            draw_thick_line(&mut px, 32, 32, 46, 16, 2, glyph);
        }
        // Othala (Stone / Heritage)
        RuneType::Stone => {
            draw_thick_line(&mut px, 32, 14, 44, 27, 2, glyph);
            draw_thick_line(&mut px, 44, 27, 32, 40, 2, glyph);
            draw_thick_line(&mut px, 32, 40, 20, 27, 2, glyph);
            draw_thick_line(&mut px, 20, 27, 32, 14, 2, glyph);
            draw_thick_line(&mut px, 27, 35, 19, 49, 2, glyph);
            draw_thick_line(&mut px, 37, 35, 45, 49, 2, glyph);
        }
        // Sowilo (Gold / Sun)
        RuneType::Gold => {
            draw_thick_line(&mut px, 42, 14, 22, 30, 2, glyph);
            draw_thick_line(&mut px, 22, 30, 42, 34, 2, glyph);
            draw_thick_line(&mut px, 42, 34, 22, 50, 2, glyph);
        }
        // Laguz (Water / Fjord)
        RuneType::Water => {
            draw_thick_line(&mut px, 24, 14, 24, 50, 2, glyph);
            draw_thick_line(&mut px, 24, 14, 43, 28, 2, glyph);
            draw_thick_line(&mut px, 20, 46, 44, 46, 1, glyph);
        }
    }

    finish_image(px)
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

fn build_ice_image(layers: u8) -> Image {
    let mut px = new_canvas();
    let s = RUNE_TEX_SIZE as i32;
    let alpha = if layers >= 2 { 200 } else { 115 };
    for y in 2..(s - 2) {
        for x in 2..(s - 2) {
            let border = x <= 5 || y <= 5 || x >= s - 6 || y >= s - 6;
            let rgba = if border {
                [190, 240, 255, alpha]
            } else {
                [120, 210, 245, alpha / 2]
            };
            set_px(&mut px, x, y, rgba);
        }
    }
    if layers >= 2 {
        draw_thick_line(&mut px, 10, 10, 54, 54, 1, [235, 250, 255, 210]);
        draw_thick_line(&mut px, 54, 10, 10, 54, 1, [235, 250, 255, 210]);
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

fn build_bifrost_image() -> Image {
    let mut px = new_canvas();
    let s = RUNE_TEX_SIZE as i32;
    let c = s / 2;
    let palette: [[u8; 3]; 6] = [
        [240, 75, 75],
        [248, 178, 52],
        [92, 214, 122],
        [72, 206, 242],
        [108, 128, 248],
        [218, 108, 238],
    ];
    for y in 6..(s - 6) {
        for x in 6..(s - 6) {
            let d2 = (x - c) * (x - c) + (y - c) * (y - c);
            if d2 <= 24 * 24 {
                let band = (((x + y) / 7) as usize) % palette.len();
                let [r, g, b] = palette[band];
                let core = d2 <= 8 * 8;
                let rgba = if core {
                    [255, 255, 255, 255]
                } else {
                    [r, g, b, 250]
                };
                set_px(&mut px, x, y, rgba);
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
