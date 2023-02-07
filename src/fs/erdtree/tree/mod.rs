use super::order::Order;
use super::{super::error::Error, node::Node};
use crossbeam::channel::{self, Sender};
use ignore::{WalkParallel, WalkState};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    path::PathBuf,
    slice::Iter,
    thread,
};

#[cfg(test)]
mod test;

/// Used for padding between tree branches.
pub const SEP: &str = "   ";

/// The `│` box drawing character.
pub const VT: &str = "\x1b[35m\u{2502}\x1b[0m  ";

/// The `└─` box drawing characters.
pub const UPRT: &str = "\x1b[35m\u{2514}\u{2500}\x1b[0m ";

/// The `├─` box drawing characters.
pub const VTRT: &str = "\x1b[35m\u{251C}\u{2500}\x1b[0m ";

/// In-memory representation of the root-directory and its contents which respects `.gitignore`.
#[derive(Debug)]
pub struct Tree {
    level: Option<usize>,
    #[allow(dead_code)]
    order: Order,
    root: Node,
}

pub type TreeResult<T> = Result<T, Error>;
pub type Branches = HashMap<PathBuf, Vec<Node>>;
pub type TreeComponents = (Node, Branches);

impl Tree {
    /// Initializes a [Tree].
    pub fn new(walker: WalkParallel, order: Order, level: Option<usize>) -> TreeResult<Self> {
        let root = Self::traverse(walker, &order)?;

        Ok(Self {
            level,
            order,
            root,
        })
    }

    /// Returns a reference to the root [Node].
    pub fn root(&self) -> &Node {
        &self.root
    }

    /// Parallel traversal of the root directory and its contents taking `.gitignore` into
    /// consideration. Parallel traversal relies on `WalkParallel`. Any filesystem I/O or related
    /// system calls are expected to occur during parallel traversal; thus post-processing of all
    /// directory entries should be completely CPU-bound. If filesystem I/O or system calls occur
    /// outside of the parallel traversal step please report an issue.
    fn traverse(walker: WalkParallel, order: &Order) -> TreeResult<Node> {
        let (tx, rx) = channel::unbounded::<Node>();

        // Receives directory entries from the workers used for parallel traversal to construct the
        // components needed to assmemble a `Tree`.
        let tree_components = thread::spawn(move || -> TreeResult<TreeComponents> {
            let mut branches: Branches = HashMap::new();
            let mut root = None;

            while let Ok(node) = rx.recv() {
                if node.is_dir() {
                    let node_path = node.path();

                    if !branches.contains_key(node_path) {
                        branches.insert(node_path.to_owned(), vec![]);
                    }

                    if node.depth == 0 {
                        root = Some(node);
                        continue;
                    }
                }

                let parent = node.parent_path_buf().ok_or(Error::ExpectedParent)?;

                let update = branches.get_mut(&parent).map(|mut_ref| mut_ref.push(node));

                if update.is_none() {
                    branches.insert(parent, vec![]);
                }
            }

            let root_node = root.ok_or(Error::MissingRoot)?;

            Ok((root_node, branches))
        });

        // All filesystem I/O and related system-calls should be relegated to this. Directory
        // entries that are encountered are sent to the above thread for processing.
        walker.run(|| {
            Box::new(|entry_res| {
                let tx = Sender::clone(&tx);

                entry_res
                    .map(Node::from)
                    .map(|node| tx.send(node).unwrap())
                    .map(|_| WalkState::Continue)
                    .unwrap_or(WalkState::Skip)
            })
        });

        drop(tx);

        let (mut root, mut branches) = tree_components.join().unwrap()?;

        Self::assemble_tree(&mut root, &mut branches, order);

        Ok(root)
    }

    /// Takes the results of the parallel traversal and uses it to construct the [Tree] data
    /// structure. Sorting occurs if specified.
    fn assemble_tree(current_dir: &mut Node, branches: &mut Branches, order: &Order) {
        let dir_node = branches.remove(current_dir.path()).map(|children| {
            current_dir.set_children(children);
            current_dir
        });

        if let Some(node) = dir_node {
            let mut dir_size = 0;

            if let Some(node_iter) = node.children_mut().map(|nodes| nodes.iter_mut()) {
                node_iter.for_each(|node| {
                    if node.is_dir() {
                        Self::assemble_tree(node, branches, order);
                    }
                    dir_size += node.file_size.unwrap_or(0);
                });
            }

            if dir_size > 0 {
                node.set_file_size(dir_size)
            }

            if let Some(func) = order.comparator() {
                if let Some(nodes) = node.children_mut() {
                    nodes.sort_by(func)
                }
            }
        }
    }
}

impl Display for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let root = self.root();
        let level = self.level.unwrap_or(std::usize::MAX);
        let mut output = String::from("");

        #[inline]
        fn extend_output(output: &mut String, node: &Node, prefix: &str) {
            output.push_str(&format!("{prefix}{node}\n"));
        }

        #[inline]
        fn traverse(
            output: &mut String,
            children: Iter<Node>,
            base_prefix: &str,
            level: usize,
        ) {
            let mut peekable = children.peekable();

            loop {
                if let Some(child) = peekable.next() {
                    let last_entry = peekable.peek().is_none();

                    let mut prefix = base_prefix.to_owned();

                    if last_entry {
                        prefix.push_str(UPRT);
                    } else {
                        prefix.push_str(VTRT);
                    }

                    extend_output(output, child, &prefix);

                    if !child.is_dir() || child.depth + 1 > level {
                        continue;
                    }

                    let mut new_base = base_prefix.to_owned();

                    if child.is_dir() && last_entry {
                        new_base.push_str(SEP);
                    } else {
                        new_base.push_str(VT);
                    }

                    if let Some(iter_children) = child.children() {
                        traverse(output, iter_children, &new_base, level)
                    }

                    continue;
                }
                break;
            }
        }

        extend_output(&mut output, root, "");
        if let Some(iter_children) = root.children() {
            traverse(&mut output, iter_children, "", level)
        }

        write!(f, "{output}")
    }
}
