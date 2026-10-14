//! Moving items to the user's Trash.

use std::io;
use std::path::Path;

use cleaner_core::ports::system::Trash;
use trash::TrashContext;
use trash::macos::{DeleteMethod, TrashContextExtMacos};

/// Uses `NSFileManager.trashItemAtURL`: no Finder automation prompt and works for app bundles.
/// (Finder's "Put Back" is not offered; items can still be dragged back from the Trash.)
#[derive(Debug, Default, Clone, Copy)]
pub struct MacTrash;

impl Trash for MacTrash {
    fn move_to_trash(&self, path: &Path) -> io::Result<()> {
        let mut context = TrashContext::default();
        context.set_delete_method(DeleteMethod::NsFileManager);
        context.delete(path).map_err(io::Error::other)
    }
}
