use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub const CONFIG_ENV_VAR: &str = "ERDTREE_CONFIG_PATH";

/// Reads the config file into a `String` if there is one. When `None` is provided as an argument
/// `ERDTREE_CONFIG_PATH` is used to locate the path of the configuration file; if that doesn't
/// yield the config file then `$HOME/.erdtreerc` is checked.
pub fn read_config_to_string<T: AsRef<Path>>(path: Option<T>) -> Option<String> {
    if let Some(p) = path {
        return fs::read_to_string(p).ok();
    }

    env::var_os(CONFIG_ENV_VAR)
        .map(PathBuf::from)
        .map(fs::read_to_string)
        .and_then(Result::ok)
        .or_else(|| {
            env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .map(|p| p.join("erdtree").join("config"))
                .map(fs::read_to_string)
                .and_then(Result::ok)
        })
}

/// Parses the config `str`, removing comments and preparing it as a format understood by
/// [`get_matches_from`].
///
/// [`get_matches_from`]: clap::builder::Command::get_matches_from
pub fn parse_config<'a>(config: &'a str) -> Vec<&'a str> {
    config
        .lines()
        .filter(|line| {
            line.trim_start()
                .chars()
                .next()
                .map(|ch| ch != '#')
                .unwrap_or(true)
        })
        .flat_map(str::split_ascii_whitespace)
        .collect::<Vec<&'a str>>()
}
