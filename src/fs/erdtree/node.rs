use super::{super::inode::Inode, disk_usage::DiskUsage, get_ls_colors};
use crate::{
    fs::erdtree::disk_usage::FileSize,
    icons::{self, icon_from_ext, icon_from_file_name, icon_from_file_type},
};
use ansi_term::Style;
use ignore::DirEntry;
use lscolors::Style as LS_Style;
use std::{
    borrow::Cow,
    convert::From,
    ffi::{OsStr, OsString},
    fmt::{self, Display, Formatter},
    fs::{self, FileType},
    path::{Path, PathBuf},
    slice::Iter,
};

/// A node of [`Tree`] that can be created from a [DirEntry]. Any filesystem I/O and
/// relevant system calls are expected to complete after initialization. A `Node` when `Display`ed
/// uses ANSI colors determined by the file-type and [`LS_COLORS`].
///
/// [`Tree`]: super::tree::Tree
/// [`LS_COLORS`]: super::tree::ui::LS_COLORS
#[derive(Debug)]
pub struct Node {
    pub depth: usize,
    pub file_size: Option<FileSize>,
    children: Option<Vec<Node>>,
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
    #[allow(clippy::too_many_arguments)]
    /// Initializes a new [Node].
    pub fn new(
        depth: usize,
        file_size: Option<FileSize>,
        children: Option<Vec<Node>>,
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
            children,
            depth,
            file_name,
            file_size,
            file_type,
            inode,
            path,
            show_icon,
            style,
            symlink_target,
            symlink_target_style,
        }
    }

    /// Returns a mutable reference to `children` if any.
    pub fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        self.children.as_mut()
    }

    /// Returns an iter over a `children` slice if any.
    pub fn children(&self) -> Option<Iter<Node>> {
        self.children.as_ref().map(|children| children.iter())
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
        self.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
    }

    /// Is the Node a symlink.
    pub fn is_symlink(&self) -> bool {
        self.symlink_target.is_some()
    }

    /// Path to symlink target.
    pub fn symlink_target_path(&self) -> Option<&Path> {
        self.symlink_target.as_deref()
    }

    /// Returns the file name of the symlink target if [Node] represents a symlink.
    pub fn symlink_target_file_name(&self) -> Option<&OsStr> {
        // self.symlink_target_path().and_then(|path| path.file_name())
        // self.symlink_target_path()
        //     .and_then(|path| Some(path.as_os_str()))
        self.symlink_target_path().map(|path| path.as_os_str())
    }

    /// Returns reference to underlying [FileType].
    pub fn file_type(&self) -> Option<&FileType> {
        self.file_type.as_ref()
    }

    /// Returns the path to the [Node]'s parent, if any. This is a pretty expensive operation used
    /// during parallel traversal. Perhaps an area for optimization.
    pub fn parent_path_buf(&self) -> Option<PathBuf> {
        let mut path_buf = self.path.clone();

        if path_buf.pop() {
            Some(path_buf)
        } else {
            None
        }
    }

    /// Returns a reference to `path`.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Sets `children`.
    pub fn set_children(&mut self, children: Vec<Node>) {
        self.children = Some(children);
    }

    /// Gets 'file_size'.
    pub fn file_size(&self) -> Option<&FileSize> {
        self.file_size.as_ref()
    }

    /// Sets `file_size`.
    pub fn set_file_size(&mut self, size: FileSize) {
        self.file_size = Some(size);
    }

    /// Sets 'style'.
    pub fn style(&self) -> &Style {
        &self.style
    }

    /// Returns reference to underlying [Inode] if any.
    pub fn inode(&self) -> Option<&Inode> {
        self.inode.as_ref()
    }

    /// Gets stylized icon for node if enabled. Icons without extensions are styled based on the
    /// [`LS_COLORS`] foreground configuration of the associated file name.
    ///
    /// [`LS_COLORS`]: super::tree::ui::LS_COLORS
    fn get_icon(&self) -> Option<String> {
        if !self.show_icon {
            return None;
        }

        let path = self.symlink_target_path().unwrap_or_else(|| self.path());

        if let Some(icon) = path.extension().and_then(icon_from_ext) {
            return Some(self.stylize(icon));
        }

        if let Some(icon) = self.file_type().and_then(icon_from_file_type) {
            return Some(self.stylize(icon));
        }

        let file_name = self
            .symlink_target_file_name()
            .unwrap_or_else(|| self.file_name());

        if let Some(icon) = icon_from_file_name(file_name) {
            return Some(self.stylize(icon));
        }

        Some(icons::get_default_icon().to_owned())
    }

    /// Stylizes input, `entity` based on [`LS_COLORS`]
    ///
    /// [`LS_COLORS`]: super::tree::ui::LS_COLORS
    fn stylize(&self, entity: &str) -> String {
        self.style().paint(entity).to_string()
    }

    /// Stylizes symlink name for display.
    fn stylize_link_name(&self) -> Option<String> {
        self.symlink_target_file_name().map(|name| {
            let file_name = self.file_name_lossy();
            let styled_name = self.stylize(&file_name);
            let target_name = self.symlink_target_style.paint(name.to_string_lossy());
            format!("{} -> {}", styled_name, target_name)
        })
    }
}

/// Used to be converted directly into a [Node].
pub struct NodePrecursor<'a> {
    disk_usage: &'a DiskUsage,
    dir_entry: DirEntry,
    show_icon: bool,
    scale: usize,
}

impl<'a> NodePrecursor<'a> {
    /// Yields a [NodePrecursor] which is used for convenient conversion into a [Node].
    pub fn new(
        disk_usage: &'a DiskUsage,
        dir_entry: DirEntry,
        show_icon: bool,
        scale: usize,
    ) -> Self {
        Self {
            disk_usage,
            dir_entry,
            show_icon,
            scale,
        }
    }
}

impl From<NodePrecursor<'_>> for Node {
    fn from(precursor: NodePrecursor) -> Self {
        let NodePrecursor {
            disk_usage,
            dir_entry,
            show_icon,
            scale,
        } = precursor;

        let children = None;

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
            |os_str| os_str.to_owned(),
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

        if let Some(ref ft) = file_type {
            if ft.is_file() {
                if let Some(ref md) = metadata {
                    file_size = match disk_usage {
                        DiskUsage::Logical => Some(FileSize::logical(md, scale)),
                        DiskUsage::Physical => FileSize::physical(path, md, scale),
                    }
                }
            }
        };

        let inode = metadata.map(Inode::try_from).transpose().ok().flatten();

        Self::new(
            depth,
            file_size,
            children,
            file_name,
            file_type,
            inode,
            path.into(),
            show_icon,
            style,
            symlink_target,
            symlink_target_style,
        )
    }
}

impl Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let size = match self.file_size() {
            Some(size) => format!("({})", size),
            None => "".to_owned(),
        };

        let (icon, icon_padding) = match self.get_icon() {
            Some(icon) => (icon, 1),
            None => ("".to_owned(), 0),
        };

        let styled_name = match self.stylize_link_name() {
            Some(name) => name,
            None => {
                let file_name = self.file_name_lossy();
                self.stylize(&file_name)
            }
        };

        let output = format!(
            "{icon}{:<icon_padding$}{styled_name} {size}",
            "",
            icon_padding = icon_padding
        );

        write!(f, "{output}")
    }
}
