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

//! Embeds the Wyrdstone Windows application icon into the compiled `.exe`.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/sprites/app_icon.ico");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        #[cfg(windows)]
        {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("../../assets/sprites/app_icon.ico");
            if let Err(err) = res.compile() {
                if std::env::var("PROFILE").as_deref() == Ok("release") {
                    panic!("Failed to embed Windows icon in release build: {err}");
                }
                println!("cargo:warning=Failed to embed Windows icon: {err}");
            }
        }
    }
}
