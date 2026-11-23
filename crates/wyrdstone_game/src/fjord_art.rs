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
use wyrdstone_core::{ActiveTool, PatronGod};

const FJORD_MAP_PNG: &[u8] = include_bytes!("../../../assets/sprites/fjord_map.png");
const MATCH3_BG_PNG: &[u8] = include_bytes!("../../../assets/sprites/match3_bg.png");
const BLDG_GREAT_HALL_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_great_hall.png");
const BLDG_FORGE_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_forge.png");
const BLDG_DOCK_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_dock.png");
const BLDG_SHRINE_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_shrine.png");
const BLDG_MEADERY_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_meadery.png");
const BLDG_WATCHTOWER_PNG: &[u8] = include_bytes!("../../../assets/sprites/bldg_watchtower.png");
const MAP_BLDG_GREAT_HALL_PNG: &[u8] =
    include_bytes!("../../../assets/sprites/map_bldg_great_hall.png");
const MAP_BLDG_FORGE_PNG: &[u8] = include_bytes!("../../../assets/sprites/map_bldg_forge.png");
const MAP_BLDG_DOCK_PNG: &[u8] =
    include_bytes!("../../../assets/sprites/map_bldg_longship_dock.png");
const MAP_BLDG_SHRINE_PNG: &[u8] =
    include_bytes!("../../../assets/sprites/map_bldg_odin_shrine.png");
const MAP_BLDG_MEADERY_PNG: &[u8] = include_bytes!("../../../assets/sprites/map_bldg_meadery.png");
const MAP_BLDG_WATCHTOWER_PNG: &[u8] =
    include_bytes!("../../../assets/sprites/map_bldg_watchtower.png");
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
const BOSS_FENRIR_PNG: &[u8] = include_bytes!("../../../assets/sprites/boss_fenrir.png");
const BOSS_JORMUNGANDR_PNG: &[u8] = include_bytes!("../../../assets/sprites/boss_jormungandr.png");
const BOSS_SURTR_PNG: &[u8] = include_bytes!("../../../assets/sprites/boss_surtr.png");
const BOSS_HEL_PNG: &[u8] = include_bytes!("../../../assets/sprites/boss_hel.png");
const PATRON_THOR_PNG: &[u8] = include_bytes!("../../../assets/sprites/patron_thor.png");
const PATRON_FREYA_PNG: &[u8] = include_bytes!("../../../assets/sprites/patron_freya.png");
const PATRON_LOKI_PNG: &[u8] = include_bytes!("../../../assets/sprites/patron_loki.png");
const PATRON_ODIN_PNG: &[u8] = include_bytes!("../../../assets/sprites/patron_odin.png");
const AMULET_FREYA_PNG: &[u8] = include_bytes!("../../../assets/sprites/amulet_freya.png");
const AMULET_HEIMDALL_PNG: &[u8] = include_bytes!("../../../assets/sprites/amulet_heimdall.png");
const AMULET_THOR_PNG: &[u8] = include_bytes!("../../../assets/sprites/amulet_thor.png");
const AMULET_ODIN_PNG: &[u8] = include_bytes!("../../../assets/sprites/amulet_odin.png");
const AMULET_VIDAR_PNG: &[u8] = include_bytes!("../../../assets/sprites/amulet_vidar.png");
const COL_MIDGARD_PNG: &[u8] = include_bytes!("../../../assets/sprites/col_midgard.png");
const COL_NIDAVELLIR_PNG: &[u8] = include_bytes!("../../../assets/sprites/col_nidavellir.png");
const COL_ASGARD_PNG: &[u8] = include_bytes!("../../../assets/sprites/col_asgard.png");
const COL_JOTUNHEIM_PNG: &[u8] = include_bytes!("../../../assets/sprites/col_jotunheim.png");
const COL_VANAHEIM_PNG: &[u8] = include_bytes!("../../../assets/sprites/col_vanaheim.png");
const INTER_REGULAR_TTF: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
const CINZEL_BOLD_TTF: &[u8] = include_bytes!("../../../assets/fonts/CinzelDecorative-Bold.ttf");

/// Handles to all bundled painted artworks and typography.
#[derive(Resource)]
pub struct FjordTextures {
    pub display_font: Handle<Font>,
    pub fjord_map: Handle<Image>,
    pub match3_bg: Handle<Image>,
    pub logo: Handle<Image>,
    pub buildings: HashMap<String, Handle<Image>>,
    pub map_buildings: HashMap<String, Handle<Image>>,
    pub portraits: HashMap<String, Handle<Image>>,
    pub boss_portraits: HashMap<String, Handle<Image>>,
    pub patron_portraits: HashMap<PatronGod, Handle<Image>>,
    pub tool_icons: HashMap<ActiveTool, Handle<Image>>,
    pub amulet_icons: HashMap<String, Handle<Image>>,
    pub collection_icons: HashMap<String, Handle<Image>>,
}

/// Decodes the bundled app icon PNG into raw RGBA8 bytes and dimensions for native window icons.
pub fn app_icon_rgba() -> (Vec<u8>, u32, u32) {
    let img = decode_png(APP_ICON_PNG);
    let w = img.width();
    let h = img.height();
    (img.data, w, h)
}

impl FjordTextures {
    /// Returns the painted illustration handle for a given building ID (Inspector view).
    pub fn building_image(&self, building_id: &str) -> Option<Handle<Image>> {
        self.buildings.get(building_id).cloned()
    }

    /// Returns the transparent isometric cutout sprite for a given building on the Fjord map.
    pub fn map_building_image(&self, building_id: &str) -> Option<Handle<Image>> {
        self.map_buildings
            .get(building_id)
            .or_else(|| self.buildings.get(building_id))
            .cloned()
    }

    /// Returns the painted character portrait for a Saga speaker (`"jarl"`, `"brokk"`, `"volva"`).
    pub fn portrait(&self, speaker: &str) -> Option<Handle<Image>> {
        self.portraits.get(speaker).cloned()
    }

    /// Returns the painted skill icon for one of the six active tools.
    pub fn tool_icon(&self, tool: ActiveTool) -> Option<Handle<Image>> {
        self.tool_icons.get(&tool).cloned()
    }

    /// Returns the painted portrait for a Mythological Boss by ID.
    pub fn boss_portrait(&self, boss_id: &str) -> Option<Handle<Image>> {
        self.boss_portraits.get(boss_id).cloned()
    }

    /// Returns the painted portrait for a Norse Patron God.
    pub fn patron_portrait(&self, god: PatronGod) -> Option<Handle<Image>> {
        self.patron_portraits.get(&god).cloned()
    }

    /// Returns the painted medallion icon for a Nordic Amulet by ID.
    pub fn amulet_icon(&self, amulet_id: &str) -> Option<Handle<Image>> {
        self.amulet_icons.get(amulet_id).cloned()
    }

    /// Returns the painted realm crest for a Relic Collection by ID.
    pub fn collection_icon(&self, col_id: &str) -> Option<Handle<Image>> {
        self.collection_icons.get(col_id).cloned()
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
    let display_font = Font::try_from_bytes(CINZEL_BOLD_TTF.to_vec())
        .map(|f| fonts.add(f))
        .unwrap_or_default();

    let fjord_map = images.add(decode_png(FJORD_MAP_PNG));
    let match3_bg = images.add(decode_png(MATCH3_BG_PNG));
    let logo = images.add(decode_logo_png(WYRDSTONE_LOGO_PNG));

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
    buildings.insert(
        "watchtower".to_string(),
        images.add(decode_png(BLDG_WATCHTOWER_PNG)),
    );

    let mut map_buildings = HashMap::new();
    map_buildings.insert(
        "great_hall".to_string(),
        images.add(decode_png(MAP_BLDG_GREAT_HALL_PNG)),
    );
    map_buildings.insert(
        "forge".to_string(),
        images.add(decode_png(MAP_BLDG_FORGE_PNG)),
    );
    map_buildings.insert(
        "longship_dock".to_string(),
        images.add(decode_png(MAP_BLDG_DOCK_PNG)),
    );
    map_buildings.insert(
        "odin_shrine".to_string(),
        images.add(decode_png(MAP_BLDG_SHRINE_PNG)),
    );
    map_buildings.insert(
        "meadery".to_string(),
        images.add(decode_png(MAP_BLDG_MEADERY_PNG)),
    );
    map_buildings.insert(
        "watchtower".to_string(),
        images.add(decode_png(MAP_BLDG_WATCHTOWER_PNG)),
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

    let mut boss_portraits = HashMap::new();
    boss_portraits.insert(
        "boss_fenrir".to_string(),
        images.add(decode_png(BOSS_FENRIR_PNG)),
    );
    boss_portraits.insert(
        "boss_jormungandr".to_string(),
        images.add(decode_png(BOSS_JORMUNGANDR_PNG)),
    );
    boss_portraits.insert(
        "boss_surtr".to_string(),
        images.add(decode_png(BOSS_SURTR_PNG)),
    );
    boss_portraits.insert("boss_hel".to_string(), images.add(decode_png(BOSS_HEL_PNG)));

    let mut patron_portraits = HashMap::new();
    patron_portraits.insert(PatronGod::Thor, images.add(decode_png(PATRON_THOR_PNG)));
    patron_portraits.insert(PatronGod::Freya, images.add(decode_png(PATRON_FREYA_PNG)));
    patron_portraits.insert(PatronGod::Loki, images.add(decode_png(PATRON_LOKI_PNG)));
    patron_portraits.insert(PatronGod::Odin, images.add(decode_png(PATRON_ODIN_PNG)));

    let mut amulet_icons = HashMap::new();
    amulet_icons.insert(
        "amulet_freya".to_string(),
        images.add(decode_png(AMULET_FREYA_PNG)),
    );
    amulet_icons.insert(
        "amulet_heimdall".to_string(),
        images.add(decode_png(AMULET_HEIMDALL_PNG)),
    );
    amulet_icons.insert(
        "amulet_thor".to_string(),
        images.add(decode_png(AMULET_THOR_PNG)),
    );
    amulet_icons.insert(
        "amulet_odin".to_string(),
        images.add(decode_png(AMULET_ODIN_PNG)),
    );
    amulet_icons.insert(
        "amulet_vidar".to_string(),
        images.add(decode_png(AMULET_VIDAR_PNG)),
    );

    let mut collection_icons = HashMap::new();
    collection_icons.insert(
        "col_midgard".to_string(),
        images.add(decode_png(COL_MIDGARD_PNG)),
    );
    collection_icons.insert(
        "col_nidavellir".to_string(),
        images.add(decode_png(COL_NIDAVELLIR_PNG)),
    );
    collection_icons.insert(
        "col_asgard".to_string(),
        images.add(decode_png(COL_ASGARD_PNG)),
    );
    collection_icons.insert(
        "col_jotunheim".to_string(),
        images.add(decode_png(COL_JOTUNHEIM_PNG)),
    );
    collection_icons.insert(
        "col_vanaheim".to_string(),
        images.add(decode_png(COL_VANAHEIM_PNG)),
    );

    commands.insert_resource(FjordTextures {
        display_font,
        fjord_map,
        match3_bg,
        logo,
        buildings,
        map_buildings,
        portraits,
        boss_portraits,
        patron_portraits,
        tool_icons,
        amulet_icons,
        collection_icons,
    });
}

/// Decodes `wyrdstone_logo.png` and smoothly keys out its dark navy canvas so the sculpted
/// emblem and aurora glow float cleanly over the animated fjord sky without a box seam.
fn decode_logo_png(bytes: &[u8]) -> Image {
    let mut img = decode_png(bytes);
    let w = img.width() as usize;
    let h = img.height() as usize;
    if w == 0 || h == 0 || img.data.len() < w * h * 4 {
        return img;
    }
    // Sample top-left corner as the backdrop key color.
    let bg_r = img.data[0] as f32;
    let bg_g = img.data[1] as f32;
    let bg_b = img.data[2] as f32;
    let half_w = w as f32 * 0.5;
    let half_h = h as f32 * 0.5;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let r = img.data[idx] as f32;
            let g = img.data[idx + 1] as f32;
            let b = img.data[idx + 2] as f32;
            let a = img.data[idx + 3] as f32;

            let dr = r - bg_r;
            let dg = g - bg_g;
            let db = b - bg_b;
            let color_dist = (dr * dr + dg * dg + db * db).sqrt();
            // Smooth alpha ramp: dark navy backdrop fades completely to 0, while teal aurora
            // and bright gold/stone lettering stay 100% opaque.
            let key_alpha = ((color_dist - 16.0) / 34.0).clamp(0.0, 1.0);

            // Soft elliptical edge feathering so no outer corner artifact can survive.
            let nx = (x as f32 - half_w) / half_w;
            let ny = (y as f32 - half_h) / half_h;
            let edge_r = (nx * nx * 0.92 + ny * ny * 0.98).sqrt();
            let edge_fade = (1.0 - ((edge_r - 0.86) / 0.14).clamp(0.0, 1.0)).powf(1.4);

            img.data[idx + 3] = (a * key_alpha * edge_fade).clamp(0.0, 255.0) as u8;
        }
    }
    img
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
