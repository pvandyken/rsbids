use std::path::Path;


/// Return True if subpath is the same as or a subpath of parent
pub fn is_subpath_of(subpath: &Path, parent: &Path) -> bool {
    let subpath_comps = subpath.components().count();
    let parent_comps = parent.components().count();
    if subpath_comps < parent_comps {
        return false;
    }
    if let Some(path) = subpath.ancestors().nth(subpath_comps - parent_comps) {
        parent == path
    } else {
        false
    }
}