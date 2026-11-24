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

//! Shared procedural RGBA pixel-compositing helpers for rune and VFX textures.

/// Alpha-composites one RGBA texel at `(x, y)` into a tightly packed RGBA8 buffer using Porter-Duff source-over.
pub fn blend_px(pixels: &mut [u8], width: usize, height: usize, x: usize, y: usize, rgba: [u8; 4]) {
    if x >= width || y >= height {
        return;
    }
    let idx = (y * width + x) * 4;
    if idx + 3 >= pixels.len() {
        return;
    }
    let src_a = rgba[3] as f32 / 255.0;
    let dst_a = pixels[idx + 3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0001 {
        return;
    }
    for c in 0..3 {
        let s = rgba[c] as f32;
        let d = pixels[idx + c] as f32;
        pixels[idx + c] = ((s * src_a + d * dst_a * (1.0 - src_a)) / out_a).clamp(0.0, 255.0) as u8;
    }
    pixels[idx + 3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_px_composites_alpha_and_ignores_out_of_bounds() {
        let mut buf = vec![0u8; 2 * 2 * 4];
        blend_px(&mut buf, 2, 2, 0, 1, [200, 100, 50, 255]);
        assert_eq!(&buf[8..12], &[200, 100, 50, 255]);

        // Out-of-bounds coordinates are a safe no-op.
        blend_px(&mut buf, 2, 2, 2, 0, [10, 20, 30, 255]);
        blend_px(&mut buf, 2, 2, 0, 2, [10, 20, 30, 255]);
        assert_eq!(&buf[0..8], &[0; 8]);
        assert_eq!(&buf[12..16], &[0; 4]);
    }
}
