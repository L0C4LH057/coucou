// IPC server abstraction module:
// Windows: Named Pipe (`pipe::windows`)
// Unix/Linux: Unix Domain Socket (`pipe::unix`)

#[cfg(target_os = "windows")]
mod windows_impl;
#[cfg(target_os = "windows")]
use windows_impl as imp;

#[cfg(not(target_os = "windows"))]
mod unix_impl;
#[cfg(not(target_os = "windows"))]
use unix_impl as imp;

pub use imp::{acknowledge, answer, decline, pipe_name, start, Pending, Reply};
