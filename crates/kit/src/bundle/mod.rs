//! A machine on disk: a `.vista` directory holding its configuration, disks,
//! firmware state and snapshots.
//!
//! ```text
//! Ubuntu.vista/
//!   config.json        hardware, see machine::Config
//!   disk.img           the system disk, a sparse raw image
//!   nvram              EFI variables (generic machines)
//!   auxiliary          boot state (macOS machines)
//!   hardwaremodel      the Mac hardware model the guest was installed for
//!   machineid          the platform's machine identifier
//!   suspended.vzvmsave memory, when the machine is suspended
//!   snapshots/<id>/    copy-on-write clones of all of the above
//! ```

mod copy;
mod files;
mod ops;
mod snapshot;

pub use copy::clone_path;
pub use files::*;
pub use ops::{create, delete, duplicate, load, save, usage, Bundle};
pub use snapshot::{Snapshot, Snapshots};
