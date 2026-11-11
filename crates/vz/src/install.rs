//! Installing macOS from a restore image onto a fresh bundle.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_foundation::NSError;
use objc2_virtualization::*;
use vista_kit::bundle::{self, Bundle, AUXILIARY, HARDWARE};

use crate::util::{describe, url, write_data};

/// The objects an install keeps alive, and its progress.
pub struct InstallProgress {
  installer: Retained<VZMacOSInstaller>,
  pub machine: Retained<VZVirtualMachine>,
}

impl InstallProgress {
  pub fn fraction(&self) -> f64 {
    unsafe { self.installer.progress().fractionCompleted() }
  }
}

/// Retained objects cross from the framework's queue to the main queue,
/// where they are only ever touched.
struct OnMain<T>(T);
unsafe impl<T> Send for OnMain<T> {}

impl<T> OnMain<T> {
  fn take(self) -> T {
    self.0
  }
}

type Done = Box<dyn FnOnce(Result<(), String>)>;
type Started = Rc<RefCell<Option<Box<dyn FnOnce(InstallProgress)>>>>;

/// Install the restore image at `ipsw` into `bundle`. `started` receives the
/// running installer (to watch progress); `done` fires once with the outcome.
/// Must be called on the main thread.
pub fn install_macos(
  bundle: Bundle,
  ipsw: PathBuf,
  started: impl FnOnce(InstallProgress) + 'static,
  done: impl FnOnce(Result<(), String>) + 'static,
) {
  let started: Started = Rc::new(RefCell::new(Some(Box::new(started))));
  let done: Rc<RefCell<Option<Done>>> = Rc::new(RefCell::new(Some(Box::new(done))));
  let finish = {
    let done = done.clone();
    move |result: Result<(), String>| {
      if let Some(done) = done.borrow_mut().take() {
        done(result);
      }
    }
  };

  // The image loads off the main queue; everything after happens back on it.
  let state = OnMain((bundle, ipsw.clone(), started, finish));
  let state = std::sync::Mutex::new(Some(state));
  let loaded = RcBlock::new(move |image: *mut VZMacOSRestoreImage, err: *mut NSError| {
    let image = unsafe { Retained::retain(image) };
    let err = unsafe { Retained::retain(err) };
    let Some(state) = state.lock().ok().and_then(|mut s| s.take()) else {
      return;
    };
    let payload = OnMain((state, image, err));
    DispatchQueue::main().exec_async(move || {
      let (state, image, err) = payload.take();
      let (bundle, ipsw, started, finish) = state.take();
      match image {
        Some(image) => begin(bundle, &ipsw, &image, started, finish),
        None => finish(Err(format!(
          "Couldn't read the restore image: {}",
          err.map(|e| describe(&e)).unwrap_or_default()
        ))),
      }
    });
  });
  unsafe {
    VZMacOSRestoreImage::loadFileURL_completionHandler(&url(&ipsw), &loaded);
  }
}

fn begin(
  mut bundle: Bundle,
  ipsw: &Path,
  image: &VZMacOSRestoreImage,
  started: Started,
  finish: impl Fn(Result<(), String>) + 'static,
) {
  let prepared = unsafe { prepare(&mut bundle, image) };
  let machine = match prepared.and_then(|_| crate::build(&bundle, false)) {
    Ok(built) => unsafe {
      VZVirtualMachine::initWithConfiguration(VZVirtualMachine::alloc(), &built.configuration)
    },
    Err(err) => return finish(Err(err)),
  };
  let installer = unsafe {
    VZMacOSInstaller::initWithVirtualMachine_restoreImageURL(
      VZMacOSInstaller::alloc(),
      &machine,
      &url(ipsw),
    )
  };
  let completed = RcBlock::new(move |err: *mut NSError| {
    let err = unsafe { Retained::retain(err) };
    finish(match err {
      None => Ok(()),
      Some(err) => Err(format!("macOS installation failed: {}", describe(&err))),
    });
  });
  unsafe { installer.installWithCompletionHandler(&completed) };
  if let Some(started) = started.borrow_mut().take() {
    started(InstallProgress { installer, machine });
  }
}

/// Write the hardware model and boot storage the image needs, and raise the
/// machine's CPU and memory to the image's minimums.
unsafe fn prepare(bundle: &mut Bundle, image: &VZMacOSRestoreImage) -> Result<(), String> {
  let requirements = image
    .mostFeaturefulSupportedConfiguration()
    .ok_or("This Mac can't run the macOS version in that restore image.")?;
  let model = requirements.hardwareModel();
  if !model.isSupported() {
    return Err("This Mac doesn't support the hardware that macOS version needs.".into());
  }
  write_data(&bundle.file(HARDWARE), &model.dataRepresentation()).map_err(|e| e.to_string())?;
  VZMacAuxiliaryStorage::initCreatingStorageAtURL_hardwareModel_options_error(
    VZMacAuxiliaryStorage::alloc(),
    &url(&bundle.file(AUXILIARY)),
    &model,
    VZMacAuxiliaryStorageInitializationOptions::AllowOverwrite,
  )
  .map_err(|err| format!("Couldn't create the boot storage: {}", describe(&err)))?;

  let config = &mut bundle.config;
  config.cpus = config.cpus.max(requirements.minimumSupportedCPUCount() as u32);
  let minimum = requirements.minimumSupportedMemorySize() / (1024 * 1024);
  config.memory = config.memory.max(minimum);
  bundle::save(&bundle.path, config).map_err(|e| e.to_string())
}
