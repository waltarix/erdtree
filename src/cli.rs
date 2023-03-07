use clap::{Parser, ValueEnum};
use ignore::{
    overrides::{Override, OverrideBuilder},
    WalkBuilder, WalkParallel,
};
use std::{
    convert::From,
    error::Error as StdError,
    fmt::{self, Display, Formatter},
    fs, io,
    path::{Path, PathBuf},
    usize,
};

/// Defines the CLI.
#[derive(Parser, Debug)]
#[command(name = "erdtree")]
#[command(author = "Benjamin Nguyen. <benjamin.van.nguyen@gmail.com>")]
#[command(version = "1.3.0")]
#[command(about = "erdtree (et) is a multi-threaded filetree visualizer and disk usage analyzer.", long_about = None)]
pub struct Clargs {
    /// Root directory to traverse; defaults to current working directory
    dir: Option<PathBuf>,

    /// Print physical or logical file size
    #[arg(short, long, value_enum, default_value_t = DiskUsage::Logical)]
    disk_usage: DiskUsage,

    /// Include or exclude files using glob patterns
    #[arg(short, long)]
    glob: Vec<String>,

    /// Include or exclude files using glob patterns; case insensitive
    #[arg(long)]
    iglob: Vec<String>,

    /// Process all glob patterns case insensitively
    #[arg(long)]
    glob_case_insensitive: bool,

    /// Show hidden files; disabled by default
    #[arg(short = 'H', long)]
    pub hidden: bool,

    /// Disable traversal of .git directory when traversing hidden files; disabled by default
    #[arg(long)]
    ignore_git: bool,

    /// Display file icons; disabled by default
    #[arg(short = 'I', long)]
    pub icons: bool,

    /// Ignore .gitignore; disabled by default
    #[arg(short, long)]
    pub ignore_git_ignore: bool,

    /// Maximum depth to display
    #[arg(short, long, value_name = "NUM")]
    pub level: Option<usize>,

    /// Total number of digits after the decimal to display for disk usage
    #[arg(short = 'n', long, default_value_t = 2, value_name = "NUM")]
    pub scale: usize,

    /// Sort-order to display directory content
    #[arg(short, long, value_enum, default_value_t = Order::Name)]
    sort: Order,

    /// Always sorts directories above files
    #[arg(short = 'D', long)]
    dirs_first: bool,

    /// Traverse symlink directories and consider their disk usage; disabled by default
    #[arg(short = 'S', long)]
    follow_links: bool,

    /// Number of threads to use
    #[arg(short, long, default_value_t = 4)]
    pub threads: usize,
}

/// Order in which to print nodes.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Order {
    /// Sort entries by file name
    Name,

    /// Sort entries by size smallest to largest, top to bottom
    Size,

    /// Sort entries by size largest to smallest, bottom to top
    SizeRev,

    /// No sorting
    None,
}

/// Display disk usage output as either logical size or physical size.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum DiskUsage {
    /// How many bytes does a file contain
    Logical,

    /// How much actual space on disk based on blocks allocated, taking into account sparse files
    /// and compression.
    Physical,
}

impl Clargs {
    /// Returns reference to the path of the root directory to be traversed.
    pub fn dir(&self) -> &Path {
        self.dir
            .as_ref()
            .map_or_else(|| Path::new("."), |pb| pb.as_path())
    }

    /// The sort-order used for printing.
    pub fn sort(&self) -> Order {
        self.sort
    }

    /// Getter for `dirs_first` field.
    pub fn dirs_first(&self) -> bool {
        self.dirs_first
    }

    /// Getter for `disk_usage` field.
    pub fn disk_usage(&self) -> &DiskUsage {
        &self.disk_usage
    }

    /// The max depth to print. Note that all directories are fully traversed to compute file
    /// sizes; this just determines how much to print.
    pub fn level(&self) -> Option<usize> {
        self.level
    }

    /// Ignore file overrides.
    pub fn overrides(&self) -> Result<Override, ignore::Error> {
        let mut builder = OverrideBuilder::new(self.dir());

        if self.ignore_git {
            builder.add("!.git/**/*")?;
        }

        if self.glob.is_empty() && self.iglob.is_empty() {
            return builder.build();
        }

        if self.glob_case_insensitive {
            builder.case_insensitive(true).unwrap();
        }

        for glob in self.glob.iter() {
            builder.add(glob)?;
        }

        // all subsequent patterns are case insensitive
        builder.case_insensitive(true).unwrap();
        for glob in self.iglob.iter() {
            builder.add(glob)?;
        }

        builder.build()
    }
}

impl TryFrom<&Clargs> for WalkParallel {
    type Error = Error;

    fn try_from(clargs: &Clargs) -> Result<Self, Self::Error> {
        let root = fs::canonicalize(clargs.dir())?;

        fs::metadata(&root).map_err(|e| Error::DirNotFound(format!("{}: {e}", root.display())))?;

        Ok(WalkBuilder::new(root)
            .follow_links(clargs.follow_links)
            .overrides(clargs.overrides()?)
            .git_ignore(!clargs.ignore_git_ignore)
            .hidden(!clargs.hidden)
            .threads(clargs.threads)
            .build_parallel())
    }
}

/// Errors which may occur during command-line argument parsing.
#[derive(Debug)]
pub enum Error {
    InvalidGlobPatterns(ignore::Error),
    DirNotFound(String),
    PathCanonicalization(io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidGlobPatterns(e) => write!(f, "Invalid glob patterns: {e}"),
            Error::DirNotFound(e) => write!(f, "{e}"),
            Error::PathCanonicalization(e) => write!(f, "{e}"),
        }
    }
}

impl StdError for Error {}

impl From<ignore::Error> for Error {
    fn from(value: ignore::Error) -> Self {
        Self::InvalidGlobPatterns(value)
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::PathCanonicalization(value)
    }
}
