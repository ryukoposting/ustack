use std::path::{Path, PathBuf};

pub trait HasAnySymlinks {
    fn has_any_symlinks(&self) -> bool;
}

impl HasAnySymlinks for Path {
    fn has_any_symlinks(&self) -> bool {
        if self.is_symlink() {
            true
        } else if let Some(parent) = self.parent() {
            parent.has_any_symlinks()
        } else {
            false
        }
    }
}

impl HasAnySymlinks for PathBuf {
    #[inline]
    fn has_any_symlinks(&self) -> bool {
        self.as_path().has_any_symlinks()
    }
}
