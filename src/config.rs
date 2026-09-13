use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationKey {
    Space,
    LeftRightArrow,
    Both,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    #[serde(default = "default_input_time_ms")]
    pub input_time_ms: u64,
    #[serde(default = "default_hold_delay_ms")]
    pub hold_delay_ms: u64,
    #[serde(default = "default_activation_key")]
    pub activation_key: String,
}

fn default_languages() -> Vec<String> {
    vec!["French".to_string()]
}

fn default_input_time_ms() -> u64 {
    200
}

fn default_hold_delay_ms() -> u64 {
    250
}

fn default_activation_key() -> String {
    "Both".to_string()
}

impl Config {
    pub fn activation_key_parsed(&self) -> ActivationKey {
        match self.activation_key.as_str() {
            "Space" => ActivationKey::Space,
            "LeftRightArrow" => ActivationKey::LeftRightArrow,
            _ => ActivationKey::Both,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            languages: default_languages(),
            input_time_ms: default_input_time_ms(),
            hold_delay_ms: default_hold_delay_ms(),
            activation_key: default_activation_key(),
        }
    }
}

pub fn config_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".config").join("quickaccent").join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

pub fn load_config() -> Config {
    let path = config_path();
    eprintln!("[QuickAccent] Looking for config at: {}", path.display());

    match std::fs::read_to_string(&path) {
        Ok(contents) => match toml::from_str::<Config>(&contents) {
            Ok(config) => {
                eprintln!("[QuickAccent] Loaded config: languages = {:?}, input_time_ms = {}, hold_delay_ms = {}, activation_key = {}",
                    config.languages, config.input_time_ms, config.hold_delay_ms, config.activation_key);
                config
            }
            Err(e) => {
                eprintln!("[QuickAccent] Failed to parse config: {}. Using defaults.", e);
                Config::default()
            }
        },
        Err(_) => {
            eprintln!("[QuickAccent] No config file found. Creating default at {}", path.display());
            let config = Config::default();
            // Try to create default config file
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let default_toml = r#"# QuickAccent Configuration
# Available languages:
#   Catalan, CrimeanTatar, Croatian, Czech, Danish, Dutch, Esperanto,
#   Estonian, Finnish, French, German, Greek, Hungarian, IPA, Iceland,
#   Irish, Italian, Kurdish, Lithuanian, Maltese, Maori, Norwegian,
#   Pinyin, Polish, Portuguese, ProtoIndoEuropean, Romanian,
#   Romanization, ScottishGaelic, Serbian, Slovak, Slovenian,
#   Spanish, Swedish, Turkish, Vietnamese, Welsh
# Additional sets: Special, Currency
# Extensions: Typography, Arrows, Math, CurrencyExtended
# See docs/CHARACTERS.md for symbol bindings.
# Hebrew/Yiddish use phonetic Latin keys; see docs/CHARACTERS.md.

languages = ["French"]

# Minimum time (ms) the letter must be held before accent is committed.
# If released sooner, it's treated as a false start and the trigger key
# (space/arrow) is replayed. Default: 200
# input_time_ms = 200

# Minimum hold time (ms) before a trigger (Space) will show the accent banner.
# Quick taps below this are treated as normal typing. Default: 250
# (PowerToys uses 200)
# hold_delay_ms = 250

# Which key(s) trigger the accent overlay: "Space", "LeftRightArrow", or "Both"
# Default: "Both"
# activation_key = "Both"
"#;
            std::fs::write(&path, default_toml).ok();
            config
        }
    }
}

pub fn read_config() -> Option<Config> {
    let path = config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|contents| toml::from_str::<Config>(&contents).ok())
}

/// Parse config from a TOML string (tests + future tooling).
pub fn parse_config_str(contents: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(contents)
}

/// Persist a new `languages` list, touching nothing else in the file so the
/// user's comments and other settings survive. The config watcher picks the
/// change up like a manual edit.
pub fn set_languages(languages: &[String]) -> std::io::Result<()> {
    let path = config_path();
    let current = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            load_config(); // writes the commented default template
            std::fs::read_to_string(&path).unwrap_or_default()
        }
    };
    std::fs::write(&path, replace_languages(&current, languages))
}

/// Replace the `languages = [...]` assignment (single- or multi-line) in a
/// TOML document, or append one if missing.
fn replace_languages(toml: &str, languages: &[String]) -> String {
    let rendered = format!(
        "languages = [{}]",
        languages.iter().map(|l| format!("{l:?}")).collect::<Vec<_>>().join(", ")
    );
    let mut out = String::with_capacity(toml.len() + rendered.len());
    let mut lines = toml.lines();
    let mut replaced = false;
    while let Some(line) = lines.next() {
        let key = line.trim_start();
        if !replaced && key.starts_with("languages") && key[9..].trim_start().starts_with('=') {
            // Skip the rest of a multi-line array.
            let mut rest = &line[line.find('=').unwrap() + 1..];
            while !rest.contains(']') {
                match lines.next() {
                    Some(l) => rest = l,
                    None => break,
                }
            }
            out.push_str(&rendered);
            out.push('\n');
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        if !out.is_empty() && !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(&rendered);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let c = Config::default();
        assert_eq!(c.languages, vec!["French".to_string()]);
        assert_eq!(c.input_time_ms, 200);
        assert_eq!(c.hold_delay_ms, 250);
        assert_eq!(c.activation_key, "Both");
        assert_eq!(c.activation_key_parsed(), ActivationKey::Both);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let c = parse_config_str("# no keys\n").unwrap();
        assert_eq!(c.languages, vec!["French".to_string()]);
        assert_eq!(c.input_time_ms, 200);
        assert_eq!(c.hold_delay_ms, 250);
        assert_eq!(c.activation_key_parsed(), ActivationKey::Both);
    }

    #[test]
    fn full_toml_parse() {
        let c = parse_config_str(
            r#"
            languages = ["German", "Spanish"]
            input_time_ms = 100
            hold_delay_ms = 300
            activation_key = "Space"
            "#,
        )
        .unwrap();
        assert_eq!(c.languages, vec!["German".to_string(), "Spanish".to_string()]);
        assert_eq!(c.input_time_ms, 100);
        assert_eq!(c.hold_delay_ms, 300);
        assert_eq!(c.activation_key_parsed(), ActivationKey::Space);
    }

    #[test]
    fn activation_key_parsing() {
        let mut c = Config::default();
        c.activation_key = "Space".into();
        assert_eq!(c.activation_key_parsed(), ActivationKey::Space);
        c.activation_key = "LeftRightArrow".into();
        assert_eq!(c.activation_key_parsed(), ActivationKey::LeftRightArrow);
        c.activation_key = "Both".into();
        assert_eq!(c.activation_key_parsed(), ActivationKey::Both);
        c.activation_key = "whatever".into();
        assert_eq!(c.activation_key_parsed(), ActivationKey::Both);
    }

    #[test]
    fn invalid_toml_errors() {
        assert!(parse_config_str("languages = [").is_err());
    }

    #[test]
    fn replace_languages_keeps_everything_else() {
        let langs = ["German".to_string(), "Spanish".to_string()];
        let doc = "# QuickAccent Configuration\n# comment\n\nlanguages = [\"French\"]\n\n# hold\n# hold_delay_ms = 250\ninput_time_ms = 100\n";
        let out = replace_languages(doc, &langs);
        assert_eq!(
            out,
            "# QuickAccent Configuration\n# comment\n\nlanguages = [\"German\", \"Spanish\"]\n\n# hold\n# hold_delay_ms = 250\ninput_time_ms = 100\n"
        );
        let parsed = parse_config_str(&out).unwrap();
        assert_eq!(parsed.languages, langs);
        assert_eq!(parsed.input_time_ms, 100);
    }

    #[test]
    fn replace_languages_handles_multiline_missing_and_lookalikes() {
        let langs = ["Welsh".to_string()];
        let multi = "languages = [\n  \"French\",\n  \"German\",\n]\nhold_delay_ms = 300\n";
        assert_eq!(replace_languages(multi, &langs), "languages = [\"Welsh\"]\nhold_delay_ms = 300\n");
        // Commented-out or similarly named keys are left alone; a missing key is appended.
        let none = "# languages = [\"French\"]\nlanguages_extra = 1\n";
        assert_eq!(
            replace_languages(none, &langs),
            "# languages = [\"French\"]\nlanguages_extra = 1\n\nlanguages = [\"Welsh\"]\n"
        );
        assert_eq!(replace_languages("", &[]), "languages = []\n");
    }
}
