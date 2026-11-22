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

//! Bundled painted Norse fjord map, Match-3 sanctuary backdrop, buildings, and character portraits.

use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::HashMap;
use wyrdstone_core::ActiveTool;

const FJORD_MAP_PNG: &[u8] = include_bytes!("../../../assets/sprites/fjord_map.png");
const MATCH3_BG_PNG: &[u8] = include_bytes!("../../../assets/sprites/match3_bg.png");
const BLDG_GREAT_HALL_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_great_hall.png");
const BLDG_FORGE_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_forge.png");
const BLDG_DOCK_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_dock.png");
const BLDG_SHRINE_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_shrine.png");
const BLDG_MEADERY_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_meadery.png");
const PORTRAIT_JARL_PNG: &[u8] = include_bytes!("../../../assets/sprites/portrait_jarl.png");
const PORTRAIT_BROKK_PNG: &[u8] = include_bytes!("../../../assets/sprites/portrait_brokk.png");
const PORTRAIT_VOLVA_PNG: &[u8] = include_bytes!("../../../assets/sprites/portrait_volva.png");
const WYRDSTONE_LOGO_PNG: &[u8] = include_bytes!("../../../assets/sprites/wyrdstone_logo.png");
const APP_ICON_PNG: &[u8] = include_bytes!("../../../assets/sprites/app_icon.png");
const TOOL_MJOLNIR_PNG: &[u8] = include_bytes!("../../../assets/sprites/tool_mjolnir.png");
const TOOL_SURTR_PNG: &[u8] = include_bytes!("../../../assets/sprites/tool_surtr.png");
const TOOL_THOR_BOLT_PNG: &[u8] = include_bytes!("../../../assets/sprites/tool_thor_bolt.png");
const TOOL_BIFROST_PNG: &[u8] = include_bytes!("../../../assets/sprites/tool_bifrost.png");
const TOOL_DRAKKAR_PNG: &[u8] = include_bytes!("../../../assets/sprites/tool_drakkar.png");
const TOOL_LOKI_PNG: &[u8] = include_bytes!("../../../assets/sprites/tool_loki.png");
const INTER_REGULAR_TTF: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");

/// Handles to all bundled painted artworks.
#[derive(Resource)]
pub struct FjordTextures {
    pub fjord_map: Handle<Image>,
    pub match3_bg: Handle<Image>,
    pub logo: Handle<Image>,
    pub buildings: HashMap<String, Handle<Image>>,
    pub portraits: HashMap<String, Handle<Image>>,
    pub tool_icons: HashMap<ActiveTool, Handle<Image>>,
}

/// Decodes the bundled app icon PNG into raw RGBA8 bytes and dimensions for native window icons.
pub fn app_icon_rgba() -> (Vec<u8>, u32, u32) {
    let img = decode_png(APP_ICON_PNG);
    let w = img.width();
    let h = img.height();
    (img.data, w, h)
}

impl FjordTextures {
    /// Returns the painted illustration handle for a given building ID.
    pub fn building_image(&self, building_id: &str) -> Option<Handle<Image>> {
        self.buildings.get(building_id).cloned()
    }

    /// Returns the painted character portrait for a Saga speaker (`"jarl"`, `"brokk"`, `"volva"`).
    pub fn portrait(&self, speaker: &str) -> Option<Handle<Image>> {
        self.portraits.get(speaker).cloned()
    }

    /// Returns the painted skill icon for one of the six active tools.
    pub fn tool_icon(&self, tool: ActiveTool) -> Option<Handle<Image>> {
        self.tool_icons.get(&tool).cloned()
    }
}

/// PreStartup system decoding all bundled painted PNG textures and the Latin-1 UI font into GPU assets.
pub fn init_fjord_textures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
) {
    // Replace Bevy's ASCII-only default font with Inter-Regular (full French/Latin-1 accents).
    if let Ok(inter_font) = Font::try_from_bytes(INTER_REGULAR_TTF.to_vec()) {
        fonts.insert(&Handle::default(), inter_font);
    }

    let fjord_map = images.add(decode_png(FJORD_MAP_PNG));
    let match3_bg = images.add(decode_png(MATCH3_BG_PNG));
    let logo = images.add(decode_png(WYRDSTONE_LOGO_PNG));

    let mut buildings = HashMap::new();
    buildings.insert(
        "great_hall".to_string(),
        images.add(decode_png(BLDG_GREAT_HALL_PNG)),
    );
    buildings.insert("forge".to_string(), images.add(decode_png(BLDG_FORGE_PNG)));
    buildings.insert(
        "longship_dock".to_string(),
        images.add(decode_png(BLDG_DOCK_PNG)),
    );
    buildings.insert(
        "odin_shrine".to_string(),
        images.add(decode_png(BLDG_SHRINE_PNG)),
    );
    buildings.insert(
        "meadery".to_string(),
        images.add(decode_png(BLDG_MEADERY_PNG)),
    );

    let mut portraits = HashMap::new();
    portraits.insert(
        "jarl".to_string(),
        images.add(decode_png(PORTRAIT_JARL_PNG)),
    );
    portraits.insert(
        "brokk".to_string(),
        images.add(decode_png(PORTRAIT_BROKK_PNG)),
    );
    portraits.insert(
        "volva".to_string(),
        images.add(decode_png(PORTRAIT_VOLVA_PNG)),
    );

    let mut tool_icons = HashMap::new();
    tool_icons.insert(
        ActiveTool::Mjolnir,
        images.add(decode_png(TOOL_MJOLNIR_PNG)),
    );
    tool_icons.insert(
        ActiveTool::SurtrEmber,
        images.add(decode_png(TOOL_SURTR_PNG)),
    );
    tool_icons.insert(
        ActiveTool::ThorBolt,
        images.add(decode_png(TOOL_THOR_BOLT_PNG)),
    );
    tool_icons.insert(
        ActiveTool::BifrostPrism,
        images.add(decode_png(TOOL_BIFROST_PNG)),
    );
    tool_icons.insert(
        ActiveTool::DrakkarCharge,
        images.add(decode_png(TOOL_DRAKKAR_PNG)),
    );
    tool_icons.insert(
        ActiveTool::LokiShuffle,
        images.add(decode_png(TOOL_LOKI_PNG)),
    );

    commands.insert_resource(FjordTextures {
        fjord_map,
        match3_bg,
        logo,
        buildings,
        portraits,
        tool_icons,
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
