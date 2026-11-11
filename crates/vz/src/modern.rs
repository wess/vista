//! Framework additions newer than the published bindings: UEFI Secure Boot
//! (macOS 27) and guest provisioning for macOS guests (macOS 27). Each call
//! checks the host answers the selector first, so Vista still runs on an older
//! Mac and simply doesn't offer the feature.

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, NSObjectProtocol};
use objc2::{msg_send, sel};
use objc2_foundation::NSError;
use objc2_virtualization::{VZEFIVariableStore, VZMacOSVirtualMachineStartOptions};
use vista_kit::machine::Account;

use crate::util::{describe, string};

/// Whether this Mac's Virtualization framework has Secure Boot.
pub fn has_secure_boot() -> bool {
  AnyClass::get(c"VZEFISignatureDatabaseConfiguration").is_some()
}

/// Whether this Mac can set up a macOS guest's first account by itself.
pub fn has_provisioning() -> bool {
  AnyClass::get(c"VZMacGuestProvisioningOptions").is_some()
}

/// Turn on Secure Boot in a fresh variable store with Apple's default
/// platform key and the default signature databases (which include the
/// certificates Windows and shim-based Linux are signed with).
///
/// # Safety
/// Call on the main thread with a store nothing else is using.
pub unsafe fn enable_secure_boot(store: &VZEFIVariableStore) -> Result<(), String> {
  if !store.respondsToSelector(sel!(enableSecureBootUsingDefaultPlatformKeyWithError:)) {
    return Err("Secure Boot needs macOS 27 or later on this Mac.".into());
  }
  let result: Result<(), Retained<NSError>> =
    msg_send![store, enableSecureBootUsingDefaultPlatformKeyWithError: _];
  result.map_err(|err| format!("Couldn't enable Secure Boot: {}", describe(&err)))?;
  let result: Result<(), Retained<NSError>> =
    msg_send![store, enrollDefaultSecureBootSignaturesWithError: _];
  result.map_err(|err| format!("Couldn't enroll Secure Boot keys: {}", describe(&err)))
}

/// Start options that create the first account in a freshly installed macOS
/// guest and skip Setup Assistant. `None` on hosts without provisioning.
///
/// # Safety
/// Call on the main thread.
pub unsafe fn provisioning(
  account: &Account,
) -> Option<Retained<VZMacOSVirtualMachineStartOptions>> {
  let class = AnyClass::get(c"VZMacGuestProvisioningOptions")?;
  let options: Retained<AnyObject> = msg_send![class, new];
  let _: () = msg_send![&options, setFullName: &*string(&account.fullname)];
  let _: () = msg_send![&options, setUsername: &*string(&account.username)];
  let _: () = msg_send![&options, setPassword: &*string(&account.password)];
  let _: () = msg_send![&options, setLogsInAutomatically: account.autologin];
  let _: () = msg_send![&options, setEnablesRemoteLogin: account.ssh];
  let start = VZMacOSVirtualMachineStartOptions::new();
  if !start.respondsToSelector(sel!(setGuestProvisioningOptions:error:)) {
    return None;
  }
  let result: Result<(), Retained<NSError>> =
    msg_send![&start, setGuestProvisioningOptions: &*options, error: _];
  result.ok().map(|_| start)
}

/// Windows on this framework (experimental): Windows Boot Manager needs the
/// CPU's performance counters and fine-grained trap handling emulated.
/// These switches are private to the framework, so each is only flipped when
/// the host answers it.
///
/// # Safety
/// Call on the main thread, before the configuration is validated.
pub unsafe fn windows_platform(platform: &objc2_virtualization::VZGenericPlatformConfiguration) {
  if platform.respondsToSelector(sel!(_setPerformanceMonitoringUnitEmulationEnabled:)) {
    let _: () = msg_send![platform, _setPerformanceMonitoringUnitEmulationEnabled: true];
  }
  if platform.respondsToSelector(sel!(_setFineGrainedTrapsEmulationEnabled:)) {
    let _: () = msg_send![platform, _setFineGrainedTrapsEmulationEnabled: true];
  }
}

/// A linear framebuffer for firmware and boot loaders that can't draw through
/// virtio-gpu (Windows Boot Manager). Private; `None` when the host lacks it.
///
/// # Safety
/// Call on the main thread.
pub unsafe fn linear_framebuffer(
  width: u32,
  height: u32,
) -> Option<Retained<objc2_virtualization::VZGraphicsDeviceConfiguration>> {
  let class = AnyClass::get(c"_VZLinearFramebufferGraphicsDeviceConfiguration")?;
  let allocated: objc2::rc::Allocated<AnyObject> = msg_send![class, alloc];
  let size = objc2_foundation::NSSize::new(width as f64, height as f64);
  let device: Option<Retained<AnyObject>> = msg_send![allocated, initWithBackingStoreSize: size];
  // SAFETY: the private class subclasses VZGraphicsDeviceConfiguration.
  device.map(|d| Retained::cast_unchecked(d))
}
