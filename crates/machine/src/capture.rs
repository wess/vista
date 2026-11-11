//! Screenshots of the guest's display.
//!
//! ScreenCaptureKit can capture a process's own windows without the Screen
//! Recording permission (`getCurrentProcessShareableContent`), which is what
//! makes this work for an agent with nobody at the keyboard. The window's
//! title bar is cropped off, so the image is exactly the guest's screen and
//! its pixels are the coordinates `input` takes.

use std::sync::Mutex;

use base64::Engine;
use block2::RcBlock;
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep};
use objc2_foundation::{NSPoint as CGPoint, NSRect as CGRect, NSSize as CGSize};
use objc2_core_graphics::CGImage;
use objc2_foundation::{NSDictionary, NSError};
use objc2_screen_capture_kit::{
  SCCaptureResolutionType, SCContentFilter, SCScreenshotManager, SCShareableContent,
  SCStreamConfiguration,
};
use tokio::sync::oneshot;
use vista_kit::ipc::Response;

use crate::state::with;

/// Geometry of the window being captured, in pixels.
#[derive(Clone, Copy)]
struct Shape {
  number: isize,
  scale: f64,
  width: f64,
  height: f64,
  titlebar: f64,
}

type Sender = std::sync::Arc<Mutex<Option<oneshot::Sender<Response>>>>;

fn send(tx: &Sender, response: Response) {
  if let Some(tx) = tx.lock().ok().and_then(|mut t| t.take()) {
    let _ = tx.send(response);
  }
}

pub async fn screenshot() -> Response {
  let (tx, rx) = oneshot::channel();
  let tx: Sender = std::sync::Arc::new(Mutex::new(Some(tx)));
  let start = tx.clone();
  DispatchQueue::main().exec_async(move || begin(start));
  match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
    Ok(Ok(response)) => response,
    _ => {
      send(&tx, Response::error("timed out"));
      Response::error("The screenshot didn't arrive in time.")
    }
  }
}

fn begin(tx: Sender) {
  let shape = with(|runner| {
    if !runner.visible {
      return Err("The machine is running in the background. Show its window first (show_machine).");
    }
    let window = runner.window.as_ref().ok_or("The machine has no window yet.")?;
    let frame = window.frame();
    let content = window.contentLayoutRect();
    let scale = window.backingScaleFactor();
    Ok(Shape {
      number: window.windowNumber(),
      scale,
      width: content.size.width,
      height: content.size.height,
      titlebar: frame.size.height - content.size.height,
    })
  });
  let shape = match shape {
    Some(Ok(shape)) => shape,
    Some(Err(err)) => return send(&tx, Response::error(err)),
    None => return send(&tx, Response::error("The machine is busy; try again.")),
  };

  let found = RcBlock::new(move |content: *mut SCShareableContent, err: *mut NSError| {
    let Some(content) = (unsafe { Retained::retain(content) }) else {
      let err = unsafe { Retained::retain(err) };
      return send(
        &tx,
        Response::error(format!(
          "Screen capture unavailable: {}",
          err.map(|e| vista_vz::util::describe(&e)).unwrap_or_default()
        )),
      );
    };
    let windows = unsafe { content.windows() };
    let Some(window) = windows
      .iter()
      .find(|w| unsafe { w.windowID() } as isize == shape.number)
    else {
      return send(&tx, Response::error("The machine's window isn't on screen."));
    };
    unsafe {
      let filter =
        SCContentFilter::initWithDesktopIndependentWindow(SCContentFilter::alloc(), &window);
      let config = SCStreamConfiguration::new();
      config.setWidth((shape.width * shape.scale) as usize);
      config.setHeight(((shape.height + shape.titlebar) * shape.scale) as usize);
      config.setShowsCursor(false);
      config.setIgnoreShadowsSingleWindow(true);
      config.setCaptureResolution(SCCaptureResolutionType::Best);
      let tx = tx.clone();
      let captured = RcBlock::new(move |image: *mut CGImage, err: *mut NSError| {
        let response = if image.is_null() {
          let err = Retained::retain(err);
          Response::error(format!(
            "Capture failed: {}",
            err.map(|e| vista_vz::util::describe(&e)).unwrap_or_default()
          ))
        } else {
          encode(&*image, shape)
        };
        send(&tx, response);
      });
      SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
        &filter,
        &config,
        Some(&captured),
      );
    }
  });
  unsafe { SCShareableContent::getCurrentProcessShareableContentWithCompletionHandler(&found) };
}

/// Crop the title bar and encode PNG.
fn encode(image: &CGImage, shape: Shape) -> Response {
  let full_height = CGImage::height(Some(image)) as f64;
  let full_width = CGImage::width(Some(image)) as f64;
  let top = (full_height - shape.height * shape.scale).max(0.0);
  let rect = CGRect::new(CGPoint::new(0.0, top), CGSize::new(full_width, full_height - top));
  let Some(cropped) = CGImage::with_image_in_rect(Some(image), rect) else {
    return Response::error("Couldn't crop the screenshot.");
  };
  let width = CGImage::width(Some(&cropped)) as u32;
  let height = CGImage::height(Some(&cropped)) as u32;
  let rep = NSBitmapImageRep::initWithCGImage(NSBitmapImageRep::alloc(), &cropped);
  let png = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) };
  let Some(png) = png else {
    return Response::error("Couldn't encode the screenshot.");
  };
  Response::Image {
    png: base64::engine::general_purpose::STANDARD.encode(png.to_vec()),
    width,
    height,
  }
}
