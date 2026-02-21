use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_animation")]
    pub animation: String,

    #[serde(default = "default_opacity")]
    pub opacity: f32,

    #[serde(default = "default_fps")]
    pub fps: u32,

    #[serde(default = "default_shell")]
    pub shell: String,

    #[serde(default = "default_background")]
    pub background: String,

    #[serde(default = "default_disable_animation_alt_screen")]
    pub disable_animation_alt_screen: bool,
}

fn default_animation() -> String {
    "starfield".to_string()
}

fn default_opacity() -> f32 {
    0.8
}

fn default_fps() -> u32 {
    30
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

fn default_background() -> String {
    "#0a0a23".to_string()
}

fn default_disable_animation_alt_screen() -> bool {
    false
}

impl Default for Config {
    fn default() -> Self {
        Self {
            animation: default_animation(),
            opacity: default_opacity(),
            fps: default_fps(),
            shell: default_shell(),
            background: default_background(),
            disable_animation_alt_screen: default_disable_animation_alt_screen(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("tui_bg").join("config.toml");
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Parse the background hex color string to RGB tuple.
    /// Accepts "#RRGGBB" format. Falls back to (10, 10, 35) on parse failure.
    pub fn background_rgb(&self) -> (u8, u8, u8) {
        parse_hex_color(&self.background).unwrap_or((10, 10, 35))
    }
}

fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let toml_str = r##"
            animation = "matrix"
            opacity = 0.5
            fps = 60
            shell = "/bin/bash"
            background = "#ff0000"
        "##;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.animation, "matrix");
        assert_eq!(config.opacity, 0.5);
        assert_eq!(config.fps, 60);
        assert_eq!(config.shell, "/bin/bash");
        assert_eq!(config.background, "#ff0000");
    }

    #[test]
    fn test_parse_partial_config_uses_defaults() {
        let toml_str = r#"opacity = 0.3"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.animation, "starfield");
        assert_eq!(config.opacity, 0.3);
        assert_eq!(config.fps, 30);
    }

    #[test]
    fn test_empty_config_uses_all_defaults() {
        let toml_str = "";
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.animation, "starfield");
        assert_eq!(config.opacity, 0.8);
        assert_eq!(config.fps, 30);
        assert_eq!(config.background, "#0a0a23");
        assert!(!config.disable_animation_alt_screen);
    }

    #[test]
    fn test_disable_animation_alt_screen_override() {
        let toml_str = r#"disable_animation_alt_screen = false"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(!config.disable_animation_alt_screen);
    }

    #[test]
    fn test_parse_hex_color_valid() {
        assert_eq!(parse_hex_color("#ff0000"), Some((255, 0, 0)));
        assert_eq!(parse_hex_color("#0a0a23"), Some((10, 10, 35)));
        assert_eq!(parse_hex_color("#ffffff"), Some((255, 255, 255)));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color("ff0000"), None); // missing #
        assert_eq!(parse_hex_color("#fff"), None);    // too short
        assert_eq!(parse_hex_color("#zzzzzz"), None); // invalid hex
    }

    #[test]
    fn test_background_rgb() {
        let config = Config {
            background: "#0a0a23".to_string(),
            ..Config::default()
        };
        assert_eq!(config.background_rgb(), (10, 10, 35));
    }

    #[test]
    fn test_background_rgb_fallback() {
        let config = Config {
            background: "invalid".to_string(),
            ..Config::default()
        };
        assert_eq!(config.background_rgb(), (10, 10, 35));
    }
}
