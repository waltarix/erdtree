use crate::{
    context::Context,
    disk_usage::{
        file_size::{byte, DiskUsage, FileSize},
        units::{BinPrefix, PrefixKind, SiPrefix},
    },
    render::theme,
    styles,
    tree::node::Node,
};
use std::{
    ffi::OsStr,
    fmt::{self, Display},
    path::Path,
};

#[cfg(unix)]
use chrono::{DateTime, Local};

#[cfg(unix)]
use crate::{
    context::time,
    disk_usage::file_size::{block, BLOCK_SIZE_BYTES},
    styles::PLACEHOLDER,
};

/// Constitutes a single cell in a given row of the output. The `kind` field denotes what type of
/// data actually goes into the cell once rendered. Each `kind` which is of type [Kind] has its own
/// rules for rendering. Cell's do not have to be of a consistent width.
pub struct Cell<'a> {
    ctx: &'a Context,
    node: &'a Node,
    kind: Kind<'a>,
}

/// The type of data that a [Cell] should render.
pub enum Kind<'a> {
    FileName {
        prefix: Option<&'a str>,
    },
    FilePath,
    FileSize,
    #[cfg(unix)]
    Datetime,
    #[cfg(unix)]
    Ino,
    #[cfg(unix)]
    Nlink,
    #[cfg(unix)]
    Permissions,
    #[cfg(unix)]
    Owner,
    #[cfg(unix)]
    Group,
}

impl<'a> Cell<'a> {
    /// Initializes a new [Cell].
    pub const fn new(node: &'a Node, ctx: &'a Context, kind: Kind<'a>) -> Self {
        Self { ctx, node, kind }
    }

    /// Rules on how to render a file-name with icons and a prefix if applicable. The order in
    /// which items are rendered are: prefix-icon-name.
    #[inline]
    fn fmt_name(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.node;
        let ctx = self.ctx;

        match self.kind {
            Kind::FileName { prefix } => {
                let pre = prefix.unwrap_or_default();
                let name = theme::stylize_file_name(node);

                if !ctx.icons {
                    return write!(f, "{pre}{name}");
                }

                let icon = node.compute_icon(ctx.no_color());

                write!(f, "{pre}{icon} {name}")
            },

            _ => unreachable!(),
        }
    }

    /// Rules on how to render a file's path
    #[inline]
    fn fmt_path(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.node;
        let ctx = self.ctx;

        let path = if node.depth() == 0 {
            let file_name = node.file_name();
            <OsStr as AsRef<Path>>::as_ref(file_name).display()
        } else {
            node.path()
                .strip_prefix(ctx.dir_canonical())
                .unwrap_or_else(|_| node.path())
                .display()
        };

        let formatted_path = node.style().map_or_else(
            || path.to_string(),
            |style| format!("{}", style.paint(path.to_string())),
        );

        if !ctx.icons {
            return write!(f, "{formatted_path}");
        }

        let icon = node.compute_icon(ctx.no_color());

        write!(f, "{icon} {formatted_path}")
    }

    /// Rules on how to render the file size.
    #[inline]
    fn fmt_file_size(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.node;
        let ctx = self.ctx;

        let Some(file_size) = node.file_size() else {
            return Self::fmt_size_placeholder(f, ctx)
        };

        match file_size {
            FileSize::Byte(metric) => Self::fmt_bytes(f, metric, ctx),
            FileSize::Line(metric) => Self::fmt_unitless_disk_usage(f, metric, ctx),
            FileSize::Word(metric) => Self::fmt_unitless_disk_usage(f, metric, ctx),

            #[cfg(unix)]
            FileSize::Block(metric) => Self::fmt_block_usage(f, metric, ctx),
        }
    }

    /// Rules on how to format nlink for rendering.
    #[cfg(unix)]
    #[inline]
    fn fmt_nlink(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.node;
        let ctx = self.ctx;

        let max_width = ctx.max_nlink_width;

        let out = node.nlink().map_or_else(
            || format!("{PLACEHOLDER:>max_width$}"),
            |num| format!("{num:>max_width$}"),
        );

        let formatted_nlink = if let Ok(style) = styles::get_nlink_style() {
            style.paint(out).to_string()
        } else {
            out
        };

        write!(f, "{formatted_nlink}")
    }

    /// Rules on how to format ino for rendering.
    #[cfg(unix)]
    #[inline]
    fn fmt_ino(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.node;
        let ctx = self.ctx;

        let max_width = ctx.max_ino_width;

        let out = node.ino().map_or_else(
            || format!("{PLACEHOLDER:>max_width$}"),
            |num| format!("{num:>max_width$}"),
        );

        let formatted_ino = if let Ok(style) = styles::get_ino_style() {
            style.paint(out).to_string()
        } else {
            out
        };

        write!(f, "{formatted_ino}")
    }

    /// Rules on how to format owner.
    #[cfg(unix)]
    #[inline]
    fn fmt_owner(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let max_owner_width = self.ctx.max_owner_width;

        let owner = self.node.owner().unwrap_or(styles::PLACEHOLDER);

        if let Ok(style) = styles::get_owner_style() {
            let formatted_owner = format!("{owner:>max_owner_width$}");
            return write!(f, "{}", style.paint(formatted_owner));
        }

        write!(f, "{owner:>max_owner_width$}")
    }

    /// Rules on how to format group.
    #[cfg(unix)]
    #[inline]
    fn fmt_group(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let max_group_width = self.ctx.max_group_width;

        let group = self.node.group().unwrap_or(styles::PLACEHOLDER);

        if let Ok(style) = styles::get_group_style() {
            let formatted_group = format!("{group:>max_group_width$}");
            return write!(f, "{}", style.paint(formatted_group));
        }

        write!(f, "{group:>max_group_width$}")
    }

    /// Rules on how to format datetime for rendering.
    #[cfg(unix)]
    #[inline]
    fn fmt_datetime(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.node;
        let ctx = self.ctx;

        let datetime = match ctx.time() {
            time::Stamp::Create => node.created(),
            time::Stamp::Access => node.accessed(),
            time::Stamp::Mod => node.modified(),
        };

        let out = datetime.map(DateTime::<Local>::from).map_or_else(
            || format!("{PLACEHOLDER:>12}"),
            |dt| format!("{:>12}", self.fmt_timestamp(dt)),
        );

        let formatted_datetime = if let Ok(style) = styles::get_datetime_style() {
            style.paint(out).to_string()
        } else {
            out
        };

        write!(f, "[{formatted_datetime}]")
    }

    /// Rules on how to format timestamp
    #[cfg(unix)]
    #[inline]
    fn fmt_timestamp(&self, dt: DateTime<Local>) -> String {
        let time_format = self.ctx.time_format();
        let delayed_format = match time_format {
            time::Format::Default => dt.format("%d %h %H:%M %g"),
            time::Format::Iso => dt.format("%Y-%m-%d %H:%M:%S"),
            time::Format::IsoStrict => dt.format("%Y-%m-%dT%H:%M:%S%Z"),
            time::Format::Short => dt.format("%Y-%m-%d"),
        };

        format!("{delayed_format:>12}")
    }

    /// Rules on how to format permissions for rendering
    #[cfg(unix)]
    #[inline]
    fn fmt_permissions(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let node = self.node;
        let ctx = self.ctx;

        let formatted_perms = if ctx.octal {
            theme::style_oct_permissions(node)
        } else {
            theme::style_sym_permissions(node)
        };

        write!(f, "{formatted_perms}")
    }

    /// Formatter for the placeholder for file sizes.
    #[inline]
    fn fmt_size_placeholder(f: &mut fmt::Formatter<'_>, ctx: &Context) -> fmt::Result {
        if ctx.suppress_size || ctx.max_size_width == 0 {
            return write!(f, "");
        }

        let mut padding = ctx.max_size_width + 1;

        match ctx.disk_usage {
            DiskUsage::Logical | DiskUsage::Physical => match ctx.unit {
                PrefixKind::Si if ctx.human => padding += 2,
                PrefixKind::Bin if ctx.human => padding += 3,
                PrefixKind::Si => padding += 0,
                PrefixKind::Bin => padding += 1,
            },
            _ => padding -= 1,
        }

        let formatted_placeholder = format!("{:>padding$}", styles::PLACEHOLDER);

        if let Ok(style) = styles::get_placeholder_style() {
            write!(f, "{}", style.paint(formatted_placeholder))
        } else {
            write!(f, "{formatted_placeholder}")
        }
    }

    /// Rules to format disk usage as bytes
    #[inline]
    fn fmt_bytes(f: &mut fmt::Formatter<'_>, metric: &byte::Metric, ctx: &Context) -> fmt::Result {
        let max_size_width = ctx.max_size_width;
        let max_unit_width = ctx.max_size_unit_width;
        let out = format!("{metric}");

        let [size, unit]: [&str; 2] = out.split(' ').collect::<Vec<&str>>().try_into().unwrap();

        if ctx.no_color() {
            return write!(f, "{size:>max_size_width$} {unit:>max_unit_width$}");
        }

        let color = if metric.human_readable {
            styles::get_du_theme().unwrap().get(unit).unwrap()
        } else {
            match ctx.unit {
                PrefixKind::Si => {
                    let pre = SiPrefix::from(metric.value);
                    styles::get_du_theme().unwrap().get(pre.as_str()).unwrap()
                },
                PrefixKind::Bin => {
                    let pre = BinPrefix::from(metric.value);
                    styles::get_du_theme().unwrap().get(pre.as_str()).unwrap()
                },
            }
        };

        let out = color.paint(format!("{size:>max_size_width$} {unit:>max_unit_width$}"));

        write!(f, "{out}")
    }

    #[inline]
    #[cfg(unix)]
    fn fmt_block_usage(
        f: &mut fmt::Formatter<'_>,
        metric: &block::Metric,
        ctx: &Context,
    ) -> fmt::Result {
        let max_size_width = ctx.max_size_width;

        if ctx.no_color() {
            return write!(f, "{metric:>max_size_width$}");
        }

        let bytes = metric.value * u64::from(BLOCK_SIZE_BYTES);

        let color = match ctx.unit {
            PrefixKind::Si => {
                let pre = SiPrefix::from(bytes);
                styles::get_du_theme().unwrap().get(pre.as_str()).unwrap()
            },
            PrefixKind::Bin => {
                let pre = BinPrefix::from(bytes);
                styles::get_du_theme().unwrap().get(pre.as_str()).unwrap()
            },
        };

        let out = color.paint(format!("{metric:>max_size_width$}"));

        write!(f, "{out}")
    }

    /// Rules to format disk usage as unit-less values such as word count, lines, and blocks (unix).
    #[inline]
    fn fmt_unitless_disk_usage<M: Display>(
        f: &mut fmt::Formatter<'_>,
        metric: &M,
        ctx: &Context,
    ) -> fmt::Result {
        let max_size_width = ctx.max_size_width;

        if ctx.no_color() {
            return write!(f, "{metric:>max_size_width$}");
        }
        let color = styles::get_du_theme().unwrap().get("B").unwrap();

        write!(f, "{}", color.paint(format!("{metric:>max_size_width$}")))
    }
}

impl Display for Cell<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::FileName { prefix: _prefix } => self.fmt_name(f),
            Kind::FilePath => self.fmt_path(f),
            Kind::FileSize => self.fmt_file_size(f),

            #[cfg(unix)]
            Kind::Ino => self.fmt_ino(f),

            #[cfg(unix)]
            Kind::Nlink => self.fmt_nlink(f),

            #[cfg(unix)]
            Kind::Datetime => self.fmt_datetime(f),

            #[cfg(unix)]
            Kind::Permissions => self.fmt_permissions(f),

            #[cfg(unix)]
            Kind::Owner => self.fmt_owner(f),

            #[cfg(unix)]
            Kind::Group => self.fmt_group(f),
        }
    }
}
