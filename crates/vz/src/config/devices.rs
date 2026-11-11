//! Everything that isn't a disk: network, display, input, sound, sharing.

use objc2::rc::Retained;
use objc2::AnyThread;
use objc2_foundation::{NSArray, NSDictionary, NSString};
use objc2_virtualization::*;
use vista_kit::bundle::{Bundle, SERIAL};
use vista_kit::machine::{Network, Os};

use crate::util::{string, url};

pub unsafe fn apply(vm: &VZVirtualMachineConfiguration, bundle: &Bundle) -> Result<(), String> {
  let config = &bundle.config;
  let mac = config.os == Os::Macos;

  // Network
  if let Some(network) = network(&config.network, &config.mac)? {
    vm.setNetworkDevices(&NSArray::from_retained_slice(&[network]));
  }

  // Display
  let graphics: Retained<VZGraphicsDeviceConfiguration> = if mac {
    let device = VZMacGraphicsDeviceConfiguration::new();
    let display =
      VZMacGraphicsDisplayConfiguration::initWithWidthInPixels_heightInPixels_pixelsPerInch(
        VZMacGraphicsDisplayConfiguration::alloc(),
        config.display.width as isize,
        config.display.height as isize,
        config.display.ppi as isize,
      );
    device.setDisplays(&NSArray::from_retained_slice(&[display]));
    Retained::into_super(device)
  } else {
    let device = VZVirtioGraphicsDeviceConfiguration::new();
    let scanout = VZVirtioGraphicsScanoutConfiguration::initWithWidthInPixels_heightInPixels(
      VZVirtioGraphicsScanoutConfiguration::alloc(),
      config.display.width as isize,
      config.display.height as isize,
    );
    device.setScanouts(&NSArray::from_retained_slice(&[scanout]));
    Retained::into_super(device)
  };
  vm.setGraphicsDevices(&NSArray::from_retained_slice(&[graphics]));

  // Input. A Mac guest gets the Mac keyboard (with the globe key) and a
  // trackpad for gestures; the screen-coordinate pointer is the fallback for
  // guests whose tools predate the trackpad.
  let keyboard: Retained<VZKeyboardConfiguration> = if mac {
    Retained::into_super(VZMacKeyboardConfiguration::new())
  } else {
    Retained::into_super(VZUSBKeyboardConfiguration::new())
  };
  vm.setKeyboards(&NSArray::from_retained_slice(&[keyboard]));
  let mut pointers: Vec<Retained<VZPointingDeviceConfiguration>> = Vec::new();
  if mac {
    pointers.push(Retained::into_super(VZMacTrackpadConfiguration::new()));
  }
  pointers.push(Retained::into_super(
    VZUSBScreenCoordinatePointingDeviceConfiguration::new(),
  ));
  vm.setPointingDevices(&NSArray::from_retained_slice(&pointers));

  // Sound
  if config.audio {
    let sound = VZVirtioSoundDeviceConfiguration::new();
    let mut streams: Vec<Retained<VZVirtioSoundDeviceStreamConfiguration>> = Vec::new();
    let output = VZVirtioSoundDeviceOutputStreamConfiguration::new();
    output.setSink(Some(&VZHostAudioOutputStreamSink::new()));
    streams.push(Retained::into_super(output));
    if config.microphone {
      let input = VZVirtioSoundDeviceInputStreamConfiguration::new();
      input.setSource(Some(&VZHostAudioInputStreamSource::new()));
      streams.push(Retained::into_super(input));
    }
    sound.setStreams(&NSArray::from_retained_slice(&streams));
    vm.setAudioDevices(&NSArray::from_retained_slice(&[Retained::into_super(
      sound,
    )]));
  }

  vm.setEntropyDevices(&NSArray::from_retained_slice(&[Retained::into_super(
    VZVirtioEntropyDeviceConfiguration::new(),
  )]));
  vm.setMemoryBalloonDevices(&NSArray::from_retained_slice(&[Retained::into_super(
    VZVirtioTraditionalMemoryBalloonDeviceConfiguration::new(),
  )]));

  // Shared folders, plus Rosetta for Linux.
  let mut sharing: Vec<Retained<VZDirectorySharingDeviceConfiguration>> = Vec::new();
  let tools = vista_kit::launch::tools();
  // macOS mounts one automount share; Vista Tools rides along in it.
  let tools_in_shares = mac && tools.is_some();
  if !config.shares.is_empty() || tools_in_shares {
    let mut names: Vec<Retained<NSString>> = Vec::new();
    let mut dirs: Vec<Retained<VZSharedDirectory>> = Vec::new();
    if let (true, Some(tools)) = (tools_in_shares, &tools) {
      names.push(string("Vista Tools"));
      dirs.push(VZSharedDirectory::initWithURL_readOnly(
        VZSharedDirectory::alloc(),
        &url(tools),
        true,
      ));
    }
    for share in &config.shares {
      let path = std::path::Path::new(&share.path);
      if !path.is_dir() {
        continue;
      }
      let name = VZMultipleDirectoryShare::canonicalizedNameFromName(&string(&share.name))
        .unwrap_or_else(|| string("share"));
      names.push(name);
      dirs.push(VZSharedDirectory::initWithURL_readOnly(
        VZSharedDirectory::alloc(),
        &url(path),
        share.readonly,
      ));
    }
    if !dirs.is_empty() {
      let keys: Vec<&NSString> = names.iter().map(|n| &**n).collect();
      let values: Vec<&VZSharedDirectory> = dirs.iter().map(|d| &**d).collect();
      let map = NSDictionary::from_slices(&keys, &values);
      let share =
        VZMultipleDirectoryShare::initWithDirectories(VZMultipleDirectoryShare::alloc(), &map);
      // macOS guests mount this tag by themselves under /Volumes/My Shared Files.
      let tag = if mac {
        VZVirtioFileSystemDeviceConfiguration::macOSGuestAutomountTag()
      } else {
        string("vista")
      };
      let device = VZVirtioFileSystemDeviceConfiguration::initWithTag(
        VZVirtioFileSystemDeviceConfiguration::alloc(),
        &tag,
      );
      device.setShare(Some(&share));
      sharing.push(Retained::into_super(device));
    }
  }
  if let (false, Some(tools)) = (mac, &tools) {
    let directory =
      VZSharedDirectory::initWithURL_readOnly(VZSharedDirectory::alloc(), &url(tools), true);
    let share =
      VZSingleDirectoryShare::initWithDirectory(VZSingleDirectoryShare::alloc(), &directory);
    let device = VZVirtioFileSystemDeviceConfiguration::initWithTag(
      VZVirtioFileSystemDeviceConfiguration::alloc(),
      &string("vistatools"),
    );
    device.setShare(Some(&share));
    sharing.push(Retained::into_super(device));
  }
  if config.rosetta && config.os == Os::Linux && rosetta_installed() {
    if let Ok(share) =
      VZLinuxRosettaDirectoryShare::initWithError(VZLinuxRosettaDirectoryShare::alloc())
    {
      let device = VZVirtioFileSystemDeviceConfiguration::initWithTag(
        VZVirtioFileSystemDeviceConfiguration::alloc(),
        &string("rosetta"),
      );
      device.setShare(Some(&share));
      sharing.push(Retained::into_super(device));
    }
  }
  if !sharing.is_empty() {
    vm.setDirectorySharingDevices(&NSArray::from_retained_slice(&sharing));
  }

  // Clipboard sharing through the guest's spice-vdagent.
  if config.clipboard && !mac {
    let console = VZVirtioConsoleDeviceConfiguration::new();
    let port = VZVirtioConsolePortConfiguration::new();
    port.setName(Some(&VZSpiceAgentPortAttachment::spiceAgentPortName()));
    let spice = VZSpiceAgentPortAttachment::new();
    spice.setSharesClipboard(true);
    port.setAttachment(Some(&spice));
    console.ports().setObject_atIndexedSubscript(Some(&port), 0);
    vm.setConsoleDevices(&NSArray::from_retained_slice(&[Retained::into_super(
      console,
    )]));
  }

  // A USB controller: installer media hang off it, and on macOS 27 the
  // system's Virtual Machine Accessories menu passes USB devices through it.
  vm.setUsbControllers(&NSArray::from_retained_slice(&[Retained::into_super(
    VZXHCIControllerConfiguration::new(),
  )]));

  // The channel to Vista Tools inside the guest (commands, files).
  vm.setSocketDevices(&NSArray::from_retained_slice(&[Retained::into_super(
    VZVirtioSocketDeviceConfiguration::new(),
  )]));

  // A serial console written to the bundle: the first place to look when a
  // Linux guest won't boot.
  if !mac {
    if let Ok(attachment) = VZFileSerialPortAttachment::initWithURL_append_error(
      VZFileSerialPortAttachment::alloc(),
      &url(&bundle.file(SERIAL)),
      false,
    ) {
      let serial = VZVirtioConsoleDeviceSerialPortConfiguration::new();
      serial.setAttachment(Some(&attachment));
      vm.setSerialPorts(&NSArray::from_retained_slice(&[Retained::into_super(
        serial,
      )]));
    }
  }

  Ok(())
}

unsafe fn network(
  network: &Network,
  mac: &str,
) -> Result<Option<Retained<VZNetworkDeviceConfiguration>>, String> {
  let attachment: Retained<VZNetworkDeviceAttachment> = match network {
    Network::None => return Ok(None),
    Network::Shared => Retained::into_super(VZNATNetworkDeviceAttachment::new()),
    Network::Bridged { interface } => {
      let interfaces = VZBridgedNetworkInterface::networkInterfaces();
      let found = interfaces
        .iter()
        .find(|candidate| candidate.identifier().to_string() == *interface)
        .ok_or_else(|| {
          format!("The network interface {interface} isn't available for bridging on this Mac.")
        })?;
      Retained::into_super(VZBridgedNetworkDeviceAttachment::initWithInterface(
        VZBridgedNetworkDeviceAttachment::alloc(),
        &found,
      ))
    }
  };
  let device = VZVirtioNetworkDeviceConfiguration::new();
  if let Some(address) = VZMACAddress::initWithString(VZMACAddress::alloc(), &string(mac)) {
    device.setMACAddress(&address);
  }
  device.setAttachment(Some(&attachment));
  Ok(Some(Retained::into_super(device)))
}

pub fn rosetta_installed() -> bool {
  unsafe { VZLinuxRosettaDirectoryShare::availability() == VZLinuxRosettaAvailability::Installed }
}

/// Interfaces a bridged machine could use: (identifier, display name). Empty
/// unless Vista carries Apple's bridged-networking entitlement.
pub fn bridgeable() -> Vec<(String, String)> {
  unsafe {
    VZBridgedNetworkInterface::networkInterfaces()
      .iter()
      .map(|interface| {
        let id = interface.identifier().to_string();
        let name = interface
          .localizedDisplayName()
          .map(|n| n.to_string())
          .unwrap_or_else(|| id.clone());
        (id, name)
      })
      .collect()
  }
}
