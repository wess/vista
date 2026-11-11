//! The one Objective-C object the runner defines: it hears from the machine
//! (it stopped), from the window (someone clicked close), and from the menus.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{define_class, msg_send, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSWindow, NSWindowDelegate};
use objc2_foundation::NSError;
use objc2_virtualization::{VZVirtualMachine, VZVirtualMachineDelegate};

use crate::lifecycle;

define_class!(
  // SAFETY: NSObject has no subclassing requirements and Delegate has no Drop.
  #[unsafe(super(NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "VistaMachineDelegate"]
  pub struct Delegate;

  unsafe impl NSObjectProtocol for Delegate {}

  unsafe impl VZVirtualMachineDelegate for Delegate {
    #[unsafe(method(guestDidStopVirtualMachine:))]
    fn guest_did_stop(&self, _machine: &VZVirtualMachine) {
      lifecycle::stopped(None);
    }

    #[unsafe(method(virtualMachine:didStopWithError:))]
    fn did_stop_with_error(&self, _machine: &VZVirtualMachine, error: &NSError) {
      lifecycle::stopped(Some(vista_vz::util::describe(error)));
    }
  }

  unsafe impl NSWindowDelegate for Delegate {
    #[unsafe(method(windowShouldClose:))]
    fn window_should_close(&self, _sender: &NSWindow) -> bool {
      lifecycle::close_window();
      false
    }
  }

  impl Delegate {
    #[unsafe(method(showVista:))]
    fn show_vista(&self, _sender: Option<&AnyObject>) {
      let _ = std::process::Command::new("/usr/bin/open")
        .args(["-b", "io.wess.vista"])
        .spawn();
    }

    #[unsafe(method(pauseMachine:))]
    fn pause_machine(&self, _sender: Option<&AnyObject>) {
      lifecycle::pause(Box::new(|_| {}));
    }

    #[unsafe(method(resumeMachine:))]
    fn resume_machine(&self, _sender: Option<&AnyObject>) {
      lifecycle::resume(Box::new(|_| {}));
    }

    #[unsafe(method(restartMachine:))]
    fn restart_machine(&self, _sender: Option<&AnyObject>) {
      let _ = lifecycle::restart();
    }

    #[unsafe(method(shutDownMachine:))]
    fn shut_down_machine(&self, _sender: Option<&AnyObject>) {
      let _ = lifecycle::request_stop();
    }

    #[unsafe(method(forceStopMachine:))]
    fn force_stop_machine(&self, _sender: Option<&AnyObject>) {
      lifecycle::kill(Box::new(|_| {}));
    }

    #[unsafe(method(suspendMachine:))]
    fn suspend_machine(&self, _sender: Option<&AnyObject>) {
      lifecycle::suspend(Box::new(|_| {}));
    }

    #[unsafe(method(snapshotMachine:))]
    fn snapshot_machine(&self, _sender: Option<&AnyObject>) {
      lifecycle::snapshot(String::new(), Box::new(|_| {}));
    }

    #[unsafe(method(ejectInstaller:))]
    fn eject_installer(&self, _sender: Option<&AnyObject>) {
      lifecycle::eject();
    }

    #[unsafe(method(quitMachine:))]
    fn quit_machine(&self, _sender: Option<&AnyObject>) {
      lifecycle::close_window();
    }
  }
);

impl Delegate {
  pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
    let this = Self::alloc(mtm).set_ivars(());
    unsafe { msg_send![super(this), init] }
  }
}
