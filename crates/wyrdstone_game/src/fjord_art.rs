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

//! Procedural Nordic fjord landscape, evolving Viking building illustrations, and falling snow.

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;

const FJORD_W: usize = 560;
const FJORD_H: usize = 200;
const BLDG_W: usize = 120;
const BLDG_H: usize = 80;

/// Handles to the illustrated Fjord banner and building sprites (ruined vs rebuilt).
#[derive(Resource)]
pub struct FjordTextures {
    pub fjord_panorama: Handle<Image>,
    pub buildings_ruined: HashMap<String, Handle<Image>>,
    pub buildings_rebuilt: HashMap<String, Handle<Image>>,
}

impl FjordTextures {
    /// Returns the appropriate building illustration for the given tier (0 = ruined, 1+ = rebuilt).
    pub fn building_image(&self, building_id: &str, tier: u32) -> Option<Handle<Image>> {
        if tier == 0 {
            self.buildings_ruined.get(building_id).cloned()
        } else {
            self.buildings_rebuilt.get(building_id).cloned()
        }
    }
}

/// Startup system generating the Fjord landscape and all 5 building illustrations.
pub fn init_fjord_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let fjord_panorama = images.add(build_fjord_panorama());
    let ids = ["great_hall", "forge", "longship_dock", "odin_shrine", "meadery"];
    let mut buildings_ruined = HashMap::new();
    let mut buildings_rebuilt = HashMap::new();

    for id in ids {
        buildings_ruined.insert(id.to_string(), images.add(build_building_art(id, false)));
        buildings_rebuilt.insert(id.to_string(), images.add(build_building_art(id, true)));
    }

    commands.insert_resource(FjordTextures {
        fjord_panorama,
        buildings_ruined,
        buildings_rebuilt,
    });
}

fn finish_rgba(w: usize, h: usize, px: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        px,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn put_px(px: &mut [u8], w: usize, h: usize, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return;
    }
    let idx = ((y as usize) * w + (x as usize)) * 4;
    px[idx..idx + 4].copy_from_slice(&rgba);
}

fn fill_rect(px: &mut [u8], w: usize, h: usize, x0: i32, y0: i32, rw: i32, rh: i32, rgba: [u8; 4]) {
    for y in y0..(y0 + rh) {
        for x in x0..(x0 + rw) {
            put_px(px, w, h, x, y, rgba);
        }
    }
}

fn build_fjord_panorama() -> Image {
    let mut px = vec![0u8; FJORD_W * FJORD_H * 4];
    let water_y = 132i32;

    for y in 0..FJORD_H as i32 {
        let yf = y as f32 / FJORD_H as f32;
        for x in 0..FJORD_W as i32 {
            let xf = x as f32 / FJORD_W as f32;

            // Night Nordic sky gradient
            let mut r = 10.0 + yf * 14.0;
            let mut g = 18.0 + yf * 24.0;
            let mut b = 36.0 + yf * 34.0;

            // Aurora Borealis ribbons in upper sky
            if y < 105 {
                let wave = (xf * 7.5).sin() * 14.0 + (xf * 16.0).cos() * 6.0 + 38.0;
                let dist = ((y as f32) - wave).abs();
                if dist < 24.0 {
                    let glow = (1.0 - dist / 24.0).powf(1.8);
                    r += glow * 26.0;
                    g += glow * 155.0;
                    b += glow * 118.0;
                }
                let wave2 = (xf * 5.2 + 1.4).sin() * 16.0 + 54.0;
                let dist2 = ((y as f32) - wave2).abs();
                if dist2 < 18.0 {
                    let glow2 = (1.0 - dist2 / 18.0).powf(2.0);
                    r += glow2 * 40.0;
                    g += glow2 * 105.0;
                    b += glow2 * 165.0;
                }
            }

            // Distant mountain range silhouette with snow caps
            let ridge_far = 96.0
                - ((xf * 9.0).sin().abs() * 34.0 + (xf * 21.0 + 1.0).sin().abs() * 14.0);
            let ridge_near = 118.0
                - ((xf * 6.2 + 2.1).sin().abs() * 28.0 + (xf * 14.0).cos().abs() * 11.0);

            if (y as f32) >= ridge_far && y < water_y {
                let snow = (y as f32) - ridge_far < 10.0;
                if snow {
                    r = 188.0;
                    g = 208.0;
                    b = 228.0;
                } else {
                    r = 26.0;
                    g = 40.0;
                    b = 62.0;
                }
            }
            if (y as f32) >= ridge_near && y < water_y {
                let snow = (y as f32) - ridge_near < 7.0;
                if snow {
                    r = 205.0;
                    g = 222.0;
                    b = 238.0;
                } else {
                    r = 18.0;
                    g = 32.0;
                    b = 48.0;
                }
            }

            // Fjord water with shimmering aurora reflection
            if y >= water_y {
                let depth = (y - water_y) as f32 / (FJORD_H as i32 - water_y) as f32;
                let ripple = ((x as f32 * 0.14 + y as f32 * 0.6).sin() * 0.5 + 0.5) * 14.0;
                r = 12.0 + depth * 6.0;
                g = 34.0 + ripple + (1.0 - depth) * 26.0;
                b = 60.0 + ripple + (1.0 - depth) * 28.0;
            }

            // Foreground snowy pine headland on left & right banks
            let bank_left = (FJORD_H as f32 - 34.0) + (xf * 180.0);
            let bank_right = (FJORD_H as f32 - 38.0) + ((1.0 - xf) * 190.0);
            if (y as f32) >= bank_left || (y as f32) >= bank_right {
                r = 32.0;
                g = 52.0;
                b = 58.0;
                if (y as f32) - bank_left.min(bank_right) < 4.0 {
                    r = 184.0;
                    g = 208.0;
                    b = 224.0;
                }
            }

            put_px(
                &mut px,
                FJORD_W,
                FJORD_H,
                x,
                y,
                [
                    r.clamp(0.0, 255.0) as u8,
                    g.clamp(0.0, 255.0) as u8,
                    b.clamp(0.0, 255.0) as u8,
                    255,
                ],
            );
        }
    }

    // Crisp stars in upper sky
    let mut seed: u32 = 0xA5A5_1337;
    for _ in 0..48 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let sx = ((seed >> 16) as usize % FJORD_W) as i32;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let sy = ((seed >> 16) as usize % 68) as i32;
        put_px(&mut px, FJORD_W, FJORD_H, sx, sy, [235, 245, 255, 255]);
    }

    // Glowing moon in top-right sky
    let mx = 485i32;
    let my = 34i32;
    for y in (my - 14)..=(my + 14) {
        for x in (mx - 14)..=(mx + 14) {
            let d2 = (x - mx) * (x - mx) + (y - my) * (y - my);
            if d2 <= 10 * 10 {
                put_px(&mut px, FJORD_W, FJORD_H, x, y, [246, 240, 206, 255]);
            }
        }
    }

    finish_rgba(FJORD_W, FJORD_H, px)
}

fn build_building_art(id: &str, rebuilt: bool) -> Image {
    let mut px = vec![0u8; BLDG_W * BLDG_H * 4];

    // Sky / ground backdrop for each card banner
    for y in 0..BLDG_H as i32 {
        for x in 0..BLDG_W as i32 {
            let rgba = if y < 56 {
                if rebuilt {
                    [20, 34, 54, 255]
                } else {
                    [18, 24, 36, 255]
                }
            } else if id == "longship_dock" {
                [22, 54, 88, 255]
            } else {
                [38, 54, 62, 255]
            };
            put_px(&mut px, BLDG_W, BLDG_H, x, y, rgba);
        }
    }

    let timber = if rebuilt { [136, 86, 48, 255] } else { [74, 66, 60, 255] };
    let roof = if rebuilt { [164, 66, 42, 255] } else { [62, 68, 76, 255] };
    let stone = if rebuilt { [134, 144, 158, 255] } else { [78, 84, 92, 255] };
    let warm_glow = [255, 206, 86, 255];
    let wyrd_glow = [96, 232, 255, 255];

    match id {
        "great_hall" => {
            // Longhouse base
            fill_rect(&mut px, BLDG_W, BLDG_H, 24, 38, 72, 24, timber);
            // Pitched Viking roof
            for row in 0..22 {
                let inset = 22 - row;
                fill_rect(&mut px, BLDG_W, BLDG_H, 16 + inset, 16 + row, 88 - inset * 2, 1, roof);
            }
            if rebuilt {
                // Golden hearth door & windows + dragon ridge horns
                fill_rect(&mut px, BLDG_W, BLDG_H, 54, 46, 12, 16, warm_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 34, 46, 8, 8, warm_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 78, 46, 8, 8, warm_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 36, 10, 4, 8, [238, 192, 66, 255]);
                fill_rect(&mut px, BLDG_W, BLDG_H, 80, 10, 4, 8, [238, 192, 66, 255]);
            }
        }
        "forge" => {
            // Stone smithy & chimney
            fill_rect(&mut px, BLDG_W, BLDG_H, 28, 36, 60, 26, stone);
            fill_rect(&mut px, BLDG_W, BLDG_H, 70, 14, 14, 24, stone);
            fill_rect(&mut px, BLDG_W, BLDG_H, 22, 30, 72, 8, roof);
            if rebuilt {
                // Blazing furnace arch & chimney sparks
                fill_rect(&mut px, BLDG_W, BLDG_H, 42, 44, 20, 18, [242, 114, 34, 255]);
                fill_rect(&mut px, BLDG_W, BLDG_H, 46, 48, 12, 14, warm_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 74, 8, 6, 4, [215, 224, 234, 200]);
            }
        }
        "longship_dock" => {
            // Wooden pier
            fill_rect(&mut px, BLDG_W, BLDG_H, 10, 52, 44, 6, timber);
            fill_rect(&mut px, BLDG_W, BLDG_H, 18, 58, 5, 14, timber);
            fill_rect(&mut px, BLDG_W, BLDG_H, 40, 58, 5, 14, timber);
            // Drakkar hull
            fill_rect(&mut px, BLDG_W, BLDG_H, 48, 50, 56, 10, timber);
            fill_rect(&mut px, BLDG_W, BLDG_H, 98, 40, 6, 12, timber);
            if rebuilt {
                // Striped red/cream square Viking sail
                fill_rect(&mut px, BLDG_W, BLDG_H, 73, 16, 4, 34, [186, 142, 92, 255]);
                for sx in 56..94 {
                    let stripe = ((sx - 56) / 6) % 2 == 0;
                    let col = if stripe { [204, 52, 44, 255] } else { [244, 234, 206, 255] };
                    fill_rect(&mut px, BLDG_W, BLDG_H, sx, 20, 1, 24, col);
                }
            }
        }
        "odin_shrine" => {
            // Three standing runestone monoliths
            fill_rect(&mut px, BLDG_W, BLDG_H, 26, 32, 14, 30, stone);
            fill_rect(&mut px, BLDG_W, BLDG_H, 52, 18, 18, 44, stone);
            fill_rect(&mut px, BLDG_W, BLDG_H, 82, 32, 14, 30, stone);
            if rebuilt {
                // Glowing cyan wyrd rune lines
                fill_rect(&mut px, BLDG_W, BLDG_H, 60, 26, 3, 24, wyrd_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 56, 32, 11, 3, wyrd_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 32, 40, 3, 14, wyrd_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 88, 40, 3, 14, wyrd_glow);
            }
        }
        _ => {
            // Mead Hall with golden roof & barrels
            fill_rect(&mut px, BLDG_W, BLDG_H, 26, 38, 66, 24, timber);
            let thatch = if rebuilt { [214, 168, 58, 255] } else { [82, 78, 68, 255] };
            for row in 0..18 {
                let inset = 18 - row;
                fill_rect(&mut px, BLDG_W, BLDG_H, 20 + inset, 20 + row, 78 - inset * 2, 1, thatch);
            }
            if rebuilt {
                fill_rect(&mut px, BLDG_W, BLDG_H, 42, 46, 10, 10, warm_glow);
                fill_rect(&mut px, BLDG_W, BLDG_H, 66, 46, 10, 10, warm_glow);
                // Oak barrels outside
                fill_rect(&mut px, BLDG_W, BLDG_H, 14, 52, 10, 12, [156, 98, 44, 255]);
            }
        }
    }

    finish_rgba(BLDG_W, BLDG_H, px)
}
