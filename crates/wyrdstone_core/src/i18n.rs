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

//! Internationalization catalog supporting English and French locales.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Fr,
    En,
}

impl Language {
    /// Returns the alternate language for quick toggling.
    pub fn toggle(self) -> Self {
        match self {
            Language::Fr => Language::En,
            Language::En => Language::Fr,
        }
    }
}

/// Runtime localization lookup table.
#[derive(Debug, Clone)]
pub struct I18nCatalog {
    language: Language,
    en: HashMap<String, String>,
    fr: HashMap<String, String>,
}

impl I18nCatalog {
    /// Builds a catalog from English and French JSON locale strings.
    pub fn from_json_strs(
        en_json: &str,
        fr_json: &str,
        initial: Language,
    ) -> Result<Self, serde_json::Error> {
        let en: HashMap<String, String> = serde_json::from_str(en_json)?;
        let fr: HashMap<String, String> = serde_json::from_str(fr_json)?;
        Ok(Self {
            language: initial,
            en,
            fr,
        })
    }

    /// Returns the active language.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Sets the active language.
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// Switches between French and English.
    pub fn toggle_language(&mut self) {
        self.language = self.language.toggle();
    }

    /// Resolves a translation key for the active language, falling back to English, then the key.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        let active = match self.language {
            Language::En => &self.en,
            Language::Fr => &self.fr,
        };
        if let Some(value) = active.get(key) {
            return value.as_str();
        }
        if let Some(fallback) = self.en.get(key) {
            return fallback.as_str();
        }
        key
    }

    /// Resolves a translation key and substitutes `{name}` placeholders.
    pub fn t_fmt(&self, key: &str, args: &[(&str, String)]) -> String {
        let mut text = self.t(key).to_string();
        for (name, val) in args {
            let token = format!("{{{name}}}");
            text = text.replace(&token, val);
        }
        text
    }

    /// Returns keys present in English but missing in French, and vice versa.
    pub fn missing_keys(&self) -> (Vec<String>, Vec<String>) {
        let mut missing_in_fr: Vec<String> = self
            .en
            .keys()
            .filter(|k| !self.fr.contains_key(*k))
            .cloned()
            .collect();
        let mut missing_in_en: Vec<String> = self
            .fr
            .keys()
            .filter(|k| !self.en.contains_key(*k))
            .cloned()
            .collect();
        missing_in_fr.sort();
        missing_in_en.sort();
        (missing_in_fr, missing_in_en)
    }
}
