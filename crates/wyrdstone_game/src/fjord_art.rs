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

//! Bundled painted Norse fjord map and building illustrations (Windows & Android).

use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::HashMap;

const FJORD_MAP_PNG: &[u8] = include_bytes!("../../../assets/sprites/fjord_map.png");
const BLDG_GREAT_HALL_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_great_hall.png");
const BLDG_FORGE_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_forge.png");
const BLDG_DOCK_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_dock.png");
const BLDG_SHRINE_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_shrine.png");
const BLDG_MEADERY_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_meadery.png");

/// Handles to the painted Fjord village map and all five Viking building illustrations.
#[derive(Resource)]
pub struct FjordTextures {
    pub fjord_map: Handle<Image>,
    pub buildings: HashMap<String, Handle<Image>>,
}

impl FjordTextures {
    /// Returns the painted illustration handle for a given building ID.
    pub fn building_image(&self, building_id: &str) -> Option<Handle<Image>> {
        self.buildings.get(building_id).cloned()
    }
}

/// PreStartup system decoding all bundled painted PNG textures into GPU assets.
pub fn init_fjord_textures(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let fjord_map = images.add(decode_png(FJORD_MAP_PNG));

    let mut buildings = HashMap::new();
    buildings.insert("great_hall".to_string(), images.add(decode_png(BLDG_GREAT_HALL_PNG)));
    buildings.insert("forge".to_string(), images.add(decode_png(BLDG_FORGE_PNG)));
    buildings.insert("longship_dock".to_string(), images.add(decode_png(BLDG_DOCK_PNG)));
    buildings.insert("odin_shrine".to_string(), images.add(decode_png(BLDG_SHRINE_PNG)));
    buildings.insert("meadery".to_string(), images.add(decode_png(BLDG_MEADERY_PNG)));

    commands.insert_resource(FjordTextures {
        fjord_map,
        buildings,
    });
}

fn decode_png(bytes: &[u8]) -> Image {
    Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::linear(),
        RenderAssetUsages::RENDER_WORLD,
    )
    .expect("Bundled PNG sprite must decode cleanly")
}
