//! XDG directories owned by gnome-furigana.

use std::path::PathBuf;

const APP: &str = "gnome-furigana";

fn xdg_dir(variable: &str, fallback: &str) -> PathBuf {
    let base = std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map_or_else(|| home_dir().join(fallback), PathBuf::from);
    base.join(APP)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from)
}

/// `~/.config/gnome-furigana`: user overrides.
#[must_use]
pub fn config_dir() -> PathBuf {
    xdg_dir("XDG_CONFIG_HOME", ".config")
}

/// `~/.local/share/gnome-furigana`: `JMdict` index and installed components.
#[must_use]
pub fn data_dir() -> PathBuf {
    xdg_dir("XDG_DATA_HOME", ".local/share")
}
