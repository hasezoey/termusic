use anyhow::{anyhow, Context, Result};
use pinyin::ToPinyin;
use rand::Rng;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::{
    ffi::OsStr,
    process::{Child, Command},
};
use unicode_segmentation::UnicodeSegmentation;

use crate::config::ServerOverlay;

#[must_use]
pub fn get_pin_yin(input: &str) -> String {
    let mut b = String::new();
    for (index, f) in input.to_pinyin().enumerate() {
        match f {
            Some(p) => {
                b.push_str(p.plain());
            }
            None => {
                if let Some(c) = input.to_uppercase().chars().nth(index) {
                    b.push(c);
                }
            }
        }
    }
    b
}

// TODO: decide filetype supported by backend instead of in library
#[must_use]
pub fn filetype_supported(current_node: &str) -> bool {
    let p = Path::new(current_node);

    if p.starts_with("http") {
        return true;
    }

    match p.extension() {
        Some(ext) if ext == "mkv" || ext == "mka" => true,
        Some(ext) if ext == "mp3" => true,
        Some(ext) if ext == "aiff" => true,
        Some(ext) if ext == "flac" => true,
        Some(ext) if ext == "m4a" => true,
        Some(ext) if ext == "aac" => true,
        Some(ext) if ext == "opus" => true,
        Some(ext) if ext == "ogg" => true,
        Some(ext) if ext == "wav" => true,
        Some(ext) if ext == "webm" => true,
        Some(_) | None => false,
    }
}

#[must_use]
pub fn is_playlist(current_node: &str) -> bool {
    let p = Path::new(current_node);

    match p.extension() {
        Some(ext) if ext == "m3u" => true,
        Some(ext) if ext == "m3u8" => true,
        Some(ext) if ext == "pls" => true,
        Some(ext) if ext == "asx" => true,
        Some(ext) if ext == "xspf" => true,
        Some(_) | None => false,
    }
}

/// Get the parent path of the given `path`, if there is none use the tempdir
#[must_use]
pub fn get_parent_folder(path: &Path) -> Cow<'_, Path> {
    if path.is_dir() {
        return path.into();
    }
    match path.parent() {
        Some(p) => p.into(),
        None => std::env::temp_dir().into(),
    }
}

pub fn get_app_config_path() -> Result<PathBuf> {
    let mut path = dirs::config_dir().ok_or_else(|| anyhow!("failed to find os config dir."))?;
    path.push("termusic");

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }
    Ok(path)
}

/// Get the podcast directoy resolved and created
fn get_podcast_save_path(config: &ServerOverlay) -> Result<PathBuf> {
    let full_path = shellexpand::path::tilde(&config.settings.podcast.download_dir);
    if !full_path.exists() {
        std::fs::create_dir_all(&full_path)?;
    }
    Ok(full_path.into_owned())
}

/// Get the download directory for the provided `pod_title` and create it if not existing
pub fn create_podcast_dir(config: &ServerOverlay, pod_title: String) -> Result<PathBuf> {
    let mut download_path = get_podcast_save_path(config).context("get podcast directory")?;
    download_path.push(pod_title);
    std::fs::create_dir_all(&download_path).context("creating podcast download directory")?;

    Ok(download_path)
}

/// Parse the playlist at `current_node`(from the tui tree) and return the media paths
pub fn playlist_get_vec(current_node: &str) -> Result<Vec<String>> {
    let playlist_path = Path::new(current_node);
    // get the directory the playlist is in
    let playlist_directory = absolute_path(
        playlist_path
            .parent()
            .ok_or_else(|| anyhow!("cannot get directory from playlist path"))?,
    )?;
    let playlist_str = std::fs::read_to_string(playlist_path)?;
    let items = crate::playlist::decode(&playlist_str)
        .with_context(|| playlist_path.display().to_string())?;
    let mut vec = Vec::with_capacity(items.len());
    for mut item in items {
        item.absoluteize(&playlist_directory);

        // TODO: refactor to return better values
        vec.push(item.to_string());
    }
    Ok(vec)
}

/// Some helper functions for dealing with Unicode strings.
#[allow(clippy::module_name_repetitions)]
pub trait StringUtils {
    /// Creates a string slice from `start` and taking `length`, counted by grapheme clusters.
    fn substr(&self, start: usize, length: usize) -> &str;
    /// Counts the total number of Unicode graphemes in the String.
    fn grapheme_len(&self) -> usize;
}

impl StringUtils for str {
    fn substr(&self, start: usize, length: usize) -> &str {
        // the logic below assumes "length > 0", so this is a fallback
        if length == 0 {
            return "";
        }

        let mut iter = self.grapheme_indices(true).skip(start);
        // get the start idx
        let Some((start_idx, _)) = iter.next() else {
            return "";
        };
        // skip all remaining wanted length, minus the one we already have
        match iter.nth(length - 1) {
            Some((end_idx, _)) => {
                // a grapheme index here is the beginning idx of the provided `grapheme`
                // as the grapheme we got here is the next *unwanted* character, use a exclusive range
                &self[start_idx..end_idx]
            }
            None => {
                // there was no character after the skip, so just take everything since the start
                &self[start_idx..]
            }
        }
    }

    fn grapheme_len(&self) -> usize {
        self.graphemes(true).count()
    }
}

// passthrough impl for "String", otherwise you would always have to cast it manually
impl StringUtils for String {
    #[inline]
    fn substr(&self, start: usize, length: usize) -> &str {
        (**self).substr(start, length)
    }

    #[inline]
    fn grapheme_len(&self) -> usize {
        self.as_str().grapheme_len()
    }
}

/// Spawn a detached process
/// # Panics
/// panics when spawn server failed
pub fn spawn_process<A: IntoIterator<Item = S> + Clone, S: AsRef<OsStr>>(
    prog: &Path,
    superuser: bool,
    shout_output: bool,
    args: A,
) -> std::io::Result<Child> {
    let mut cmd = if superuser {
        let mut cmd_t = Command::new("sudo");
        cmd_t.arg(prog);
        cmd_t
    } else {
        Command::new(prog)
    };
    cmd.stdin(Stdio::null());
    if !shout_output {
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
    }

    cmd.args(args);
    cmd.spawn()
}

/// Absolutize a given path with the current working directory.
///
/// This function, unlike [`std::fs::canonicalize`] does *not* hit the filesystem and so does not require the input path to exist yet.
///
/// Examples:
/// `./somewhere` -> `/absolute/./somewhere`
/// `.\somewhere` -> `C:\somewhere`
///
/// in the future consider replacing with [`std::path::absolute`] once stable
pub fn absolute_path(path: &Path) -> std::io::Result<Cow<'_, Path>> {
    if path.is_absolute() {
        Ok(Cow::Borrowed(path))
    } else {
        Ok(Cow::Owned(std::env::current_dir()?.join(path)))
    }
}

/// Absolutize a given path with the given base.
///
/// `base` is expected to be absoulte!
///
/// This function, unlike [`std::fs::canonicalize`] does *not* hit the filesystem and so does not require the input path to exist yet.
///
/// Examples:
/// `./somewhere` -> `/absolute/./somewhere`
/// `.\somewhere` -> `C:\somewhere`
///
/// in the future consider replacing with [`std::path::absolute`] once stable
#[must_use]
pub fn absolute_path_base<'a>(path: &'a Path, base: &Path) -> Cow<'a, Path> {
    if path.is_absolute() {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(base.join(path))
    }
}

/// Generate `len` random ascii character (a-z0-9)
#[must_use]
pub fn random_ascii(len: usize) -> String {
    rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(len)
        .map(|v| char::from(v).to_ascii_lowercase())
        .collect()
}

/// Helper function to defer formatting to later, without having to allocate a intermediate [`String`]
///
/// similar to [`format_args!`], but it can be returned by `move`d values
///
/// Source: <https://internals.rust-lang.org/t/suggestion-for-helper-for-writing-fmt-debug-impls-with-nested-structure/19477/2>
///
/// Example:
/// ```
/// # use std::fmt::Display;
/// # use termusiclib::utils::display_with;
/// // instead of `fn nested() -> String`
/// fn nested() -> impl Display {
///   let new_string = String::from("Hello allocated string");
///   // instead of `format!("Formatted! {}", new_string)`
///   display_with(move |f| write!(f, "Formatted! {}", new_string))
/// }
///
/// println!("No Extra allocation:\n{}", nested());
/// ```
pub fn display_with(
    f: impl Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result,
) -> impl std::fmt::Display {
    struct DisplayWith<F>(F);

    impl<F> std::fmt::Display for DisplayWith<F>
    where
        F: Fn(&mut std::fmt::Formatter<'_>) -> std::fmt::Result,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0(f)
        }
    }

    DisplayWith(f)
}

#[cfg(test)]
mod tests {
    use std::fmt::{Display, Write};

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_pin_yin() {
        assert_eq!(get_pin_yin("陈一发儿"), "chenyifaer".to_string());
        assert_eq!(get_pin_yin("Gala乐队"), "GALAledui".to_string());
        assert_eq!(get_pin_yin("乐队Gala乐队"), "leduiGALAledui".to_string());
        assert_eq!(get_pin_yin("Annett Louisan"), "ANNETT LOUISAN".to_string());
    }

    #[test]
    fn test_substr() {
        // 0 length fallback
        assert_eq!("abcde".substr(0, 0), "");

        assert_eq!("abcde".substr(0, 1), "a");
        assert_eq!("abcde".substr(4, 1), "e");

        // something starting beyond the current string
        assert_eq!("abcde".substr(100, 1), "");
        // requesting more length that is available
        assert_eq!("abcde".substr(3, 3), "de");

        assert_eq!("陈一发儿".substr(0, 1), "陈");
        assert_eq!("陈一发儿".substr(3, 1), "儿");
    }

    #[test]
    fn display_with_to_string() {
        fn nested() -> impl Display {
            let new_owned = String::from("Owned");

            display_with(move |f| write!(f, "Nested! {new_owned}"))
        }

        let mut str = String::new();

        let _ = write!(&mut str, "Formatted! {}", nested());

        assert_eq!(str, "Formatted! Nested! Owned");
    }
}
