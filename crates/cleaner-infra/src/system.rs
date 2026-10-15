//! Cross-platform system adapters: the Trash / Recycle Bin and starting other programs.

use std::io;
use std::path::Path;
use std::process::Command;

use cleaner_core::ports::system::{Launcher, Trash};
use trash::TrashContext;

/// Moves items to the macOS Trash, the Windows Recycle Bin or the freedesktop Trash (Linux).
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemTrash;

impl Trash for SystemTrash {
    fn move_to_trash(&self, path: &Path) -> io::Result<()> {
        #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
        let mut context = TrashContext::default();
        // `NSFileManager.trashItemAtURL`: no Finder automation prompt and works for app bundles.
        #[cfg(target_os = "macos")]
        {
            use trash::macos::{DeleteMethod, TrashContextExtMacos};
            context.set_delete_method(DeleteMethod::NsFileManager);
        }
        context.delete(path).map_err(io::Error::other)
    }
}

/// Starts a program detached from the app (it keeps running after the app quits).
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessLauncher;

impl Launcher for ProcessLauncher {
    fn spawn(&self, program: &Path, args: &[String]) -> io::Result<()> {
        Command::new(program).args(args).spawn().map(|_child| ())
    }
}
