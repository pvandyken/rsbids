use std::path::Path;
use path_clean::clean;


/// Return True if subpath is the same as or a subpath of parent
pub fn is_subpath_of(subpath: &Path, parent: &Path) -> bool {
    clean(parent).starts_with(clean(subpath))
}