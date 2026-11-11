//! The Windows install disk (experimental).
//!
//! Apple's framework has no CD-ROM, and Windows only reads ISO 9660 from one,
//! so the ISO can't be attached as-is. Vista builds a USB disk instead, with
//! macOS's own tools: a FAT32 partition the firmware boots from (everything
//! but the system image), an exFAT one holding `install.wim`, the answer
//! files, a small script that applies the image with DISM (Setup's own disk
//! step doesn't work on this framework's disks), and the Arm64 virtio drivers
//! Windows needs for the disk, network and display.

use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

use super::unattend;
use crate::machine::Account;
use crate::{Error, Result};

/// virtio-win, pinned: the "stable" alias redirects to plain http.
const VIRTIO: &str = "https://fedorapeople.org/groups/virt/virtio-win/direct-downloads/archive-virtio/virtio-win-0.1.302-1/virtio-win-0.1.302.iso";
const DRIVERS: [&str; 9] = [
  "viostor", "NetKVM", "viogpudo", "vioinput", "viorng", "vioserial", "viosock", "Balloon", "viofs",
];

fn run(program: &str, args: &[&str]) -> Result<String> {
  let output = Command::new(program).args(args).output()?;
  if !output.status.success() {
    return Err(Error::msg(format!(
      "{program} {}: {}",
      args.join(" "),
      String::from_utf8_lossy(&output.stderr).trim()
    )));
  }
  Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn plist_value(plist: &str, key: &str) -> Option<String> {
  let pattern = Regex::new(&format!(r"<key>{key}</key>\s*<string>([^<]*)</string>")).ok()?;
  pattern.captures(plist).map(|c| c[1].to_string())
}

/// Mount an image read-only, out of sight, and return where.
fn mount(image: &Path) -> Result<(String, PathBuf)> {
  let plist = run(
    "/usr/bin/hdiutil",
    &["attach", "-nobrowse", "-readonly", "-plist", &image.to_string_lossy()],
  )?;
  let device = plist_value(&plist, "dev-entry").ok_or_else(|| Error::msg("hdiutil gave no device"))?;
  let point = plist_value(&plist, "mount-point").ok_or_else(|| Error::msg("hdiutil gave no mount point"))?;
  Ok((device, PathBuf::from(point)))
}

fn detach(device: &str) {
  let _ = run("/usr/bin/hdiutil", &["detach", device, "-force"]);
}

fn copy(from: &Path, to: &Path) -> Result<()> {
  run("/usr/bin/ditto", &[&from.to_string_lossy(), &to.to_string_lossy()]).map(|_| ())
}

/// Build `install.img` in the bundle and return its path.
pub async fn prepare(
  client: &reqwest::Client,
  iso: &Path,
  bundle: &Path,
  name: &str,
  account: &Account,
) -> Result<PathBuf> {
  // The drivers ISO is shared by every Windows machine.
  let virtio = crate::paths::images().join("virtio-win.iso");
  if !virtio.exists() {
    let transfer = crate::download::Transfer::new();
    crate::download::fetch(client, VIRTIO, &virtio, None, &transfer).await?;
  }
  let iso = iso.to_path_buf();
  let bundle = bundle.to_path_buf();
  let name = name.to_string();
  let account = account.clone();
  tokio::task::spawn_blocking(move || build(&iso, &virtio, &bundle, &name, &account))
    .await
    .map_err(|e| Error::msg(e.to_string()))?
}

fn build(iso: &Path, virtio: &Path, bundle: &Path, name: &str, account: &Account) -> Result<PathBuf> {
  let image = bundle.join("install.img");
  let size = std::fs::metadata(iso)?.len() + 4 * 1024 * 1024 * 1024;
  std::fs::File::create(&image)?.set_len(size)?;

  let attached = run(
    "/usr/bin/hdiutil",
    &[
      "attach",
      "-imagekey",
      "diskimage-class=CRawDiskImage",
      "-nomount",
      &image.to_string_lossy(),
    ],
  )?;
  let disk = attached
    .split_whitespace()
    .next()
    .filter(|d| d.starts_with("/dev/disk"))
    .ok_or_else(|| Error::msg("couldn't attach the new install disk"))?
    .to_string();

  let outcome = (|| -> Result<()> {
    run(
      "/usr/sbin/diskutil",
      &["partitionDisk", &disk, "GPT", "FAT32", "VISTABOOT", "3G", "ExFAT", "VISTAWIN", "R"],
    )?;
    let volumes = mounted(&disk)?;
    let (boot, data) = (&volumes[0], &volumes[1]);

    let (iso_dev, source) = mount(iso)?;
    let copied = (|| -> Result<()> {
      for entry in std::fs::read_dir(&source)?.flatten() {
        let name = entry.file_name();
        if name == "sources" {
          continue;
        }
        copy(&entry.path(), &boot.join(&name))?;
      }
      std::fs::create_dir_all(boot.join("sources"))?;
      std::fs::create_dir_all(data.join("sources"))?;
      for entry in std::fs::read_dir(source.join("sources"))?.flatten() {
        let file = entry.file_name();
        let target = if file == "install.wim" || file == "install.esd" {
          data.join("sources").join(&file)
        } else {
          boot.join("sources").join(&file)
        };
        copy(&entry.path(), &target)?;
      }
      Ok(())
    })();
    detach(&iso_dev);
    copied?;

    let (virtio_dev, drivers) = mount(virtio)?;
    let injected = (|| -> Result<()> {
      let target = boot.join("$WinPEDriver$");
      for driver in DRIVERS {
        let folder = drivers.join(driver).join("w11").join("ARM64");
        if folder.is_dir() {
          copy(&folder, &target.join(driver))?;
        }
      }
      Ok(())
    })();
    detach(&virtio_dev);
    injected?;

    let vista = boot.join("vista");
    std::fs::create_dir_all(&vista)?;
    std::fs::write(boot.join("autounattend.xml"), unattend::setup())?;
    std::fs::write(vista.join("install.cmd"), unattend::script())?;
    std::fs::write(vista.join("unattend.xml"), unattend::system(name, account))?;
    let _ = run("/usr/sbin/dot_clean", &["-m", &boot.to_string_lossy()]);
    let _ = run("/usr/sbin/dot_clean", &["-m", &data.to_string_lossy()]);
    Ok(())
  })();
  let _ = run("/usr/sbin/diskutil", &["eject", &disk]);
  detach(&disk);
  outcome.map(|_| image)
}

/// The mount points of the FAT32 and exFAT partitions on `disk`.
fn mounted(disk: &str) -> Result<Vec<PathBuf>> {
  let mut points = Vec::new();
  for slice in 1..=4 {
    let info = run("/usr/sbin/diskutil", &["info", "-plist", &format!("{disk}s{slice}")]);
    let Ok(info) = info else { continue };
    let label = plist_value(&info, "VolumeName").unwrap_or_default();
    if label == "VISTABOOT" || label == "VISTAWIN" {
      let point = plist_value(&info, "MountPoint")
        .filter(|p| !p.is_empty())
        .ok_or_else(|| Error::msg(format!("{label} didn't mount")))?;
      points.push((label, PathBuf::from(point)));
    }
  }
  points.sort_by(|a, b| a.0.cmp(&b.0));
  if points.len() != 2 {
    return Err(Error::msg("the install disk's partitions didn't appear"));
  }
  // VISTABOOT sorts before VISTAWIN.
  Ok(points.into_iter().map(|(_, p)| p).collect())
}
