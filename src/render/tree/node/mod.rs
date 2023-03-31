use crate::{
    fs::inode::Inode,
    icons::{self, icon_from_ext, icon_from_file_name, icon_from_file_type},
    render::{
        context::Context,
        disk_usage::file_size::{DiskUsage, FileSize},
        styles::get_ls_colors,
    },
};
use ansi_term::Style;
use ignore::DirEntry;
use indextree::{Arena, Node as NodeWrapper, NodeId};
use layout::SizeLocation;
use lscolors::Style as LS_Style;
use std::{
    borrow::{Cow, ToOwned},
    convert::From,
    ffi::{OsStr, OsString},
    fmt::{self, Formatter},
    fs::{self, FileType},
    path::{Path, PathBuf},
};

/// For determining orientation of disk usage information for [Node].
mod layout;

/// A node of [`Tree`] that can be created from a [DirEntry]. Any filesystem I/O and
/// relevant system calls are expected to complete after initialization. A `Node` when `Display`ed
/// uses ANSI colors determined by the file-type and [`LS_COLORS`].
///
/// [`Tree`]: super::Tree
/// [`LS_COLORS`]: crate::render::styles::LS_COLORS
#[derive(Debug)]
pub struct Node {
    pub depth: usize,
    pub file_size: Option<FileSize>,
    file_name: OsString,
    file_type: Option<FileType>,
    inode: Option<Inode>,
    path: PathBuf,
    show_icon: bool,
    style: Style,
    symlink_target: Option<PathBuf>,
    symlink_target_style: Style,
}

impl Node {
    /// Initializes a new [Node].
    pub const fn new(
        depth: usize,
        file_size: Option<FileSize>,
        file_name: OsString,
        file_type: Option<FileType>,
        inode: Option<Inode>,
        path: PathBuf,
        show_icon: bool,
        style: Style,
        symlink_target: Option<PathBuf>,
        symlink_target_style: Style,
    ) -> Self {
        Self {
            depth,
            file_size,
            file_name,
            file_type,
            inode,
            path,
            show_icon,
            style,
            symlink_target,
            symlink_target_style,
        }
    }

    /// Returns a reference to `file_name`. If file is a symlink then `file_name` is the name of
    /// the symlink not the target.
    pub fn file_name(&self) -> &OsStr {
        &self.file_name
    }

    /// Converts `OsStr` to `String`; if fails does a lossy conversion replacing non-Unicode
    /// sequences with Unicode replacement scalar value.
    pub fn file_name_lossy(&self) -> Cow<'_, str> {
        self.file_name()
            .to_str()
            .map_or_else(|| self.file_name().to_string_lossy(), Cow::from)
    }

    /// Returns `true` if node is a directory.
    pub fn is_dir(&self) -> bool {
        self.file_type().map_or(false, FileType::is_dir)
    }

    /// Is the Node a symlink.
    pub const fn is_symlink(&self) -> bool {
        self.symlink_target.is_some()
    }

    /// Path to symlink target.
    pub fn symlink_target_path(&self) -> Option<&Path> {
        self.symlink_target.as_deref()
    }

    /// Returns the file name of the symlink target if [Node] represents a symlink.
    pub fn symlink_target_file_name(&self) -> Option<&OsStr> {
        self.symlink_target_path().and_then(Path::file_name)
    }

    /// Returns reference to underlying [FileType].
    pub const fn file_type(&self) -> Option<&FileType> {
        self.file_type.as_ref()
    }

    /// Returns the path to the [Node]'s parent, if any.
    pub fn parent_path(&self) -> Option<&Path> {
        self.path.parent()
    }

    /// Returns a reference to `path`.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets 'file_size'.
    pub const fn file_size(&self) -> Option<&FileSize> {
        self.file_size.as_ref()
    }

    /// Sets `file_size`.
    pub fn set_file_size(&mut self, size: FileSize) {
        self.file_size = Some(size);
    }

    /// Sets 'style'.
    pub const fn style(&self) -> &Style {
        &self.style
    }

    /// Returns reference to underlying [Inode] if any.
    pub const fn inode(&self) -> Option<&Inode> {
        self.inode.as_ref()
    }

    /// Gets stylized icon for node if enabled. Icons without extensions are styled based on the
    /// [`LS_COLORS`] foreground configuration of the associated file name.
    ///
    /// [`LS_COLORS`]: crate::render::styles::LS_COLORS
    fn get_icon(&self) -> Option<String> {
        if !self.show_icon {
            return None;
        }

        let path = self.symlink_target_path().unwrap_or_else(|| self.path());

        if let Some(icon) = self.file_type().and_then(icon_from_file_type) {
            return Some(self.stylize_icon(icon));
        }

        if let Some(icon) = path.extension().and_then(icon_from_ext) {
            return Some(self.stylize_icon(icon));
        }

        let file_name = self
            .symlink_target_file_name()
            .unwrap_or_else(|| self.file_name());
        if let Some(icon) = icon_from_file_name(file_name) {
            return Some(self.stylize_icon(icon));
        }

        Some(icons::get_default_icon().to_owned())
    }

    /// Stylizes input, `entity` based on [`LS_COLORS`]
    ///
    /// [`LS_COLORS`]: crate::render::styles::LS_COLORS
    fn stylize(&self, entity: &str) -> String {
        self.style().paint(entity).to_string()
    }

    fn stylize_icon(&self, icon: &str) -> String {
        self.style()
            .foreground
            .map_or_else(|| icon.to_string(), |fg| fg.paint(icon).to_string())
    }

    /// Stylizes symlink name for display.
    fn stylize_link_name(&self) -> Option<String> {
        self.symlink_target_file_name().map(|name| {
            let file_name = self.file_name_lossy();
            let styled_name = self.stylize(&file_name);
            let target_name = self.symlink_target_style.paint(name.to_string_lossy());
            format!("{styled_name} -> {target_name}")
        })
    }

    /// General method for printing a `Node`. The `Display` (and `ToString`) traits are not used,
    /// to give more control over the output.
    ///
    /// Format a node for display with size on the right.
    ///
    /// Example:
    /// `| Some Directory (12.3 KiB)`
    ///
    ///
    /// Format a node for display with size on the left.
    ///
    /// Example:
    /// `  1.23 MiB | Some File`
    ///
    /// Note the two spaces to the left of the first character of the number -- even if never used,
    /// numbers are padded to 3 digits to the left of the decimal (and ctx.scale digits after)
    pub fn display(&self, f: &mut Formatter, prefix: &str, ctx: &Context) -> fmt::Result {
        let size_loc = SizeLocation::from(ctx);

        let size = self.file_size().map_or_else(
            || size_loc.default_string(ctx),
            |size| size_loc.format(size),
        );

        let (icon, icon_padding) = self
            .get_icon()
            .map_or_else(|| (String::new(), 0), |icon| (icon, 1));

        let styled_name = self.stylize_link_name().unwrap_or_else(|| {
            let file_name = self.file_name_lossy();
            self.stylize(&file_name)
        });

        match size_loc {
            SizeLocation::Right => {
                write!(
                    f,
                    "{prefix}{icon}{:<icon_padding$}{styled_name} {size}",
                    "",
                    icon_padding = icon_padding
                )
            }
            SizeLocation::Left => {
                write!(
                    f,
                    "{size} {prefix}{icon}{:<icon_padding$}{styled_name}",
                    "",
                    icon_padding = icon_padding
                )
            }
        }
    }

    /// Unix file identifiers that you'd find in the `ls -l` command.
    #[cfg(unix)]
    pub fn file_type_identifier(&self) -> Option<&str> {
        use std::os::unix::fs::FileTypeExt;

        let file_type = self.file_type()?;

        let iden = if file_type.is_dir() {
            "d"
        } else if file_type.is_file() {
            "-"
        } else if file_type.is_symlink() {
            "l"
        } else if file_type.is_fifo() {
            "p"
        } else if file_type.is_socket() {
            "s"
        } else if file_type.is_char_device() {
            "c"
        } else if file_type.is_block_device() {
            "b"
        } else {
            return None;
        };

        Some(iden)
    }

    /// File identifiers.
    #[cfg(not(unix))]
    pub fn file_type_identifier(&self) -> Option<&str> {
        let file_type = self.file_type()?;

        let iden = if file_type.is_dir() {
            "d"
        } else if file_type.is_file() {
            "-"
        } else if file_type.is_symlink() {
            "l"
        } else {
            return None;
        };

        Some(iden)
    }
}

impl From<(&DirEntry, &Context)> for Node {
    fn from(data: (&DirEntry, &Context)) -> Self {
        let (dir_entry, ctx) = data;
        let Context {
            disk_usage,
            icons,
            scale,
            suppress_size,
            prefix,
            ..
        } = ctx;

        let scale = *scale;
        let prefix = *prefix;
        let icons = *icons;

        let depth = dir_entry.depth();

        let file_type = dir_entry.file_type();

        let path = dir_entry.path();

        let symlink_target = dir_entry
            .path_is_symlink()
            .then(|| fs::read_link(path))
            .transpose()
            .ok()
            .flatten();

        let file_name = path.file_name().map_or_else(
            || OsString::from(path.display().to_string()),
            ToOwned::to_owned,
        );

        let metadata = dir_entry.metadata().ok();

        let style = get_ls_colors()
            .style_for_path_with_metadata(path, metadata.as_ref())
            .map(LS_Style::to_ansi_term_style)
            .unwrap_or_default();

        let symlink_target_style = symlink_target
            .as_ref()
            .and_then(|path| {
                get_ls_colors()
                    .style_for_path(path)
                    .map(LS_Style::to_ansi_term_style)
            })
            .unwrap_or_default();

        let mut file_size = None;

        if !suppress_size {
            file_type.and_then(|ft| {
                ft.is_file().then(|| {
                    file_size = metadata.as_ref().and_then(|md| match disk_usage {
                        DiskUsage::Logical => Some(FileSize::logical(md, prefix, scale)),
                        DiskUsage::Physical => FileSize::physical(path, md, prefix, scale),
                    });
                })
            });
        };

        let inode = metadata.map(Inode::try_from).transpose().ok().flatten();

        Self::new(
            depth,
            file_size,
            file_name,
            file_type,
            inode,
            path.into(),
            icons,
            style,
            symlink_target,
            symlink_target_style,
        )
    }
}

impl From<(NodeId, &mut Arena<Self>)> for &Node {
    fn from((node_id, tree): (NodeId, &mut Arena<Self>)) -> Self {
        tree.get(node_id).map(NodeWrapper::get).unwrap()
    }
}
