use crate::hash;
use ansi_term::{Color, Style};
use error::Error;
use lscolors::LsColors;
use once_cell::sync::OnceCell;
use std::collections::HashMap;

/// Errors for this module.
pub mod error;

/// Used as general placeholder for an empty field.
pub const PLACEHOLDER: &str = "-";

/// Used for padding between tree branches.
pub const SEP: &str = "   ";

/// The `│` box drawing character.
pub const VT: &str = "\u{2502}  ";

/// The `┌─` box drawing character.
pub const DRT: &str = "\u{250C}\u{2500} ";

/// The `└─` box drawing characters.
pub const UPRT: &str = "\u{2514}\u{2500} ";

/// The `├─` box drawing characters.
pub const VTRT: &str = "\u{251C}\u{2500} ";

/// A runtime evaluated static. [`LS_COLORS`] the `LS_COLORS` environment variable to determine what
/// ANSI colors to use when printing the names of files. If `LS_COLORS` is not set it will fallback
/// to a default defined in the `lscolors` crate.
///
/// **Note for `MacOS`**: `MacOS` uses the `LSCOLORS` environment variable which is in a format not
/// supported by the `lscolors` crate. Mac users can either set their own `LS_COLORS` environment
/// variable to customize output color or rely on the default.
static LS_COLORS: OnceCell<LsColors> = OnceCell::new();

/// Runtime evaluated static that contains ANSI-colored box drawing characters used for the
/// printing of [`super::tree::Tree`]'s branches.
static TREE_THEME: OnceCell<ThemesMap> = OnceCell::new();

/// Runtime evaluated static that contains ANSI-colored box drawing characters used for the
/// printing of [`super::tree::Tree`]'s branches for descendents of symlinks.
static LINK_THEME: OnceCell<ThemesMap> = OnceCell::new();

/// Runtime evaluated static that contains styles for disk usage output.
static DU_THEME: OnceCell<HashMap<&'static str, Style>> = OnceCell::new();

/// Runtime evaluated static that contains styles for permissions.
#[cfg(unix)]
static PERMISSIONS_THEME: OnceCell<HashMap<char, Style>> = OnceCell::new();

/// Runtime evaluated static that contains style for octal permissions.
#[cfg(unix)]
static OCTAL_PERMISSIONS_STYLE: OnceCell<Style> = OnceCell::new();

/// Runtime evaluated static that contains style for the general use placeholder "-".
static PLACEHOLDER_STYLE: OnceCell<Style> = OnceCell::new();

/// Runtime evaluated static that contains style for inode number i.e. `ino`.
#[cfg(unix)]
static INO_STYLE: OnceCell<Style> = OnceCell::new();

/// Runtime evaluated static that contains style for the file owner string.
#[cfg(unix)]
static OWNER_STYLE: OnceCell<Style> = OnceCell::new();

/// Runtime evaluated static that contains style for the file owner string.
#[cfg(unix)]
static GROUP_STYLE: OnceCell<Style> = OnceCell::new();

/// Runtime evaluated static that contains style for number of hardlinks i.e. `nlink`.
#[cfg(unix)]
static NLINK_STYLE: OnceCell<Style> = OnceCell::new();

/// Runtime evaluated static that contains style for datetime column.
#[cfg(unix)]
static DATETIME_STYLE: OnceCell<Style> = OnceCell::new();

/// Map of the names box-drawing elements to their styled strings.
pub type ThemesMap = HashMap<&'static str, String>;

/// Initializes both [`LS_COLORS`] and all themes. If `plain` argument is `true` then plain colorless
/// themes are used and [`LS_COLORS`] won't be initialized.
pub fn init(plain: bool) {
    #[cfg(windows)]
    let _ = ansi_term::enable_ansi_support();

    if plain {
        init_plain();
    } else {
        init_ls_colors();
        init_themes();
    }
}

/// Getter for [`LS_COLORS`]. Returns an error if not initialized.
#[inline]
pub fn get_ls_colors() -> Result<&'static LsColors, Error<'static>> {
    LS_COLORS.get().ok_or(Error::Uninitialized("LS_COLORS"))
}

/// Getter for [`DU_THEME`]. Returns an error if not initialized.
#[inline]
pub fn get_du_theme() -> Result<&'static HashMap<&'static str, Style>, Error<'static>> {
    DU_THEME.get().ok_or(Error::Uninitialized("DU_THEME"))
}

/// Getter for [`TREE_THEME`]. Returns an error if not initialized.
#[inline]
pub fn get_tree_theme() -> Result<&'static ThemesMap, Error<'static>> {
    TREE_THEME.get().ok_or(Error::Uninitialized("TREE_THEME"))
}

/// Getter for [`LINK_THEME`]. Returns an error if not initialized.
#[inline]
pub fn get_link_theme() -> Result<&'static ThemesMap, Error<'static>> {
    LINK_THEME.get().ok_or(Error::Uninitialized("LINK_THEME"))
}

/// Getter for [`PERMISSIONS_THEME`]. Returns an error if not initialized.
#[cfg(unix)]
#[inline]
pub fn get_permissions_theme() -> Result<&'static HashMap<char, Style>, Error<'static>> {
    PERMISSIONS_THEME
        .get()
        .ok_or(Error::Uninitialized("PERMISSIONS_THEME"))
}

/// Getter for [`OCTAL_PERMISSIONS_STYLE`]. Returns an error if not initialized.
#[cfg(unix)]
#[inline]
pub fn get_octal_permissions_style() -> Result<&'static Style, Error<'static>> {
    OCTAL_PERMISSIONS_STYLE
        .get()
        .ok_or(Error::Uninitialized("OCTAL_PERMISSIONS_STYLE"))
}

/// Getter for [`PLACEHOLDER_STYLE`]. Returns an error if not initialized.
#[inline]
pub fn get_placeholder_style() -> Result<&'static Style, Error<'static>> {
    PLACEHOLDER_STYLE
        .get()
        .ok_or(Error::Uninitialized("PLACEHOLDER_STYLE"))
}

/// Getter for [`INO_STYLE`]. Returns an error if not initialized.
#[cfg(unix)]
#[inline]
pub fn get_ino_style() -> Result<&'static Style, Error<'static>> {
    INO_STYLE.get().ok_or(Error::Uninitialized("INO_STYLE"))
}

/// Getter for [`OWNER_STYLE`]. Returns an error if not initialized.
#[cfg(unix)]
#[inline]
pub fn get_owner_style() -> Result<&'static Style, Error<'static>> {
    OWNER_STYLE.get().ok_or(Error::Uninitialized("OWNER_STYLE"))
}

/// Getter for [`GROUP_STYLE`]. Returns an error if not initialized.
#[cfg(unix)]
#[inline]
pub fn get_group_style() -> Result<&'static Style, Error<'static>> {
    GROUP_STYLE.get().ok_or(Error::Uninitialized("GROUP_STYLE"))
}

/// Getter for [`NLINK_STYLE`]. Returns an error if not initialized.
#[cfg(unix)]
#[inline]
pub fn get_nlink_style() -> Result<&'static Style, Error<'static>> {
    NLINK_STYLE.get().ok_or(Error::Uninitialized("NLINK_STYLE"))
}

/// Getter for [`DATETIME_STYLE`]. Returns an error if not initialized.
#[cfg(unix)]
#[inline]
pub fn get_datetime_style() -> Result<&'static Style, Error<'static>> {
    DATETIME_STYLE
        .get()
        .ok_or(Error::Uninitialized("DATETIME_STYLE"))
}

/// Initializes [`LS_COLORS`] by reading in the `LS_COLORS` environment variable. If it isn't set, a
/// default determined by `lscolors` crate will be used.
fn init_ls_colors() {
    LS_COLORS
        .set(LsColors::from_env().unwrap_or_default())
        .unwrap();
}

/// Colorless themes
fn init_plain() {
    let theme = hash! {
        "vt" => VT.to_owned(),
        "uprt" => UPRT.to_owned(),
        "drt" => DRT.to_owned(),
        "vtrt" => VTRT.to_owned()
    };
    TREE_THEME.set(theme).unwrap();

    let link_theme = hash! {
        "vt" => VT.to_owned(),
        "uprt" => UPRT.to_owned(),
        "drt" => DRT.to_owned(),
        "vtrt" => VTRT.to_owned()
    };
    LINK_THEME.set(link_theme).unwrap();
}

/// Initialize themes for the `--long` view.
#[cfg(unix)]
#[inline]
fn init_themes_for_long_view() {
    let permissions_theme = hash! {
        '-' => Color::RGB(0x80, 0x80, 0x80).normal(),
        'd' => Color::RGB(0x85, 0xd8, 0xff).normal(),
        'l' => Color::Cyan.normal(),
        'r' => Color::Yellow.normal(),
        'w' => Color::RGB(0xfa, 0x80, 0x72).normal(),
        'x' => Color::Green.normal(),
        's' | 'S' | 't' | 'T' => Color::Red.normal(),
        '@' => Color::Cyan.normal(),
        ' ' => Color::White.normal()
    };
    PERMISSIONS_THEME.set(permissions_theme).unwrap();

    let octal_permissions_style = Color::Yellow.normal();
    OCTAL_PERMISSIONS_STYLE
        .set(octal_permissions_style)
        .unwrap();

    let ino_style = Color::RGB(0xd3, 0xd3, 0xd3).normal();
    INO_STYLE.set(ino_style).unwrap();

    let nlink_style = Color::RGB(0xdd, 0xa0, 0xdd).normal();
    NLINK_STYLE.set(nlink_style).unwrap();

    let datetime_style = Color::RGB(0xad, 0xff, 0x2f).normal();
    DATETIME_STYLE.set(datetime_style).unwrap();

    let owner_style = Color::Cyan.normal();
    OWNER_STYLE.set(owner_style).unwrap();

    let group_style = Color::Green.normal();
    GROUP_STYLE.set(group_style).unwrap();
}

/// Initializes all color themes.
fn init_themes() {
    let theme = hash! {
        "vt" => format!("{}", Color::White.paint(VT)),
        "uprt" => format!("{}", Color::White.paint(UPRT)),
        "drt" => format!("{}", Color::White.paint(DRT)),
        "vtrt" => format!("{}", Color::White.paint(VTRT))
    };
    TREE_THEME.set(theme).unwrap();

    let link_theme = hash! {
        "vt" => format!("{}", Color::White.paint(VT)),
        "uprt" => format!("{}", Color::White.paint(UPRT)),
        "drt" => format!("{}", Color::White.paint(DRT)),
        "vtrt" => format!("{}", Color::White.paint(VTRT))
    };
    LINK_THEME.set(link_theme).unwrap();

    let du_theme = hash! {
        "B" => Color::RGB(0xc0, 0xc0, 0xc0).normal(),
        "KB" | "KiB" => Color::RGB(0x90, 0xee, 0x90).normal(),
        "MB" | "MiB" => Color::RGB(0xf0, 0xe6, 0x8c).normal(),
        "GB" | "GiB" => Color::RGB(0xff, 0x7f, 0x50).normal(),
        "TB" | "TiB" => Color::Red.normal()
    };
    DU_THEME.set(du_theme).unwrap();

    let placeholder_style = Color::Purple.normal();
    PLACEHOLDER_STYLE.set(placeholder_style).unwrap();

    #[cfg(unix)]
    init_themes_for_long_view();
}
