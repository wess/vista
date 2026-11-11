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
  "viostor",
  "NetKVM",
  "viogpudo",
  "vioinput",
  "viorng",
  "vioserial",
  "viosock",
  "Balloon",
  "viofs",
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
    &[
      "attach",
      "-nobrowse",
      "-readonly",
      "-plist",
      &image.to_string_lossy(),
    ],
  )?;
  let device =
    plist_value(&plist, "dev-entry").ok_or_else(|| Error::msg("hdiutil gave no device"))?;
  let point =
    plist_value(&plist, "mount-point").ok_or_else(|| Error::msg("hdiutil gave no mount point"))?;
  Ok((device, PathBuf::from(point)))
}

fn detach(device: &str) {
  let _ = run("/usr/bin/hdiutil", &["detach", device, "-force"]);
}

fn copy(from: &Path, to: &Path, cancel: Option<&Path>) -> Result<()> {
  super::esd::command(
    Path::new("/usr/bin/ditto"),
    &[&from.to_string_lossy(), &to.to_string_lossy()],
    cancel,
  )
  .map(|_| ())
}

/// Build `install.img` in the bundle and return its path.
pub async fn prepare(
  client: &reqwest::Client,
  iso: &Path,
  bundle: &Path,
  name: &str,
  account: &Account,
  cancel: Option<PathBuf>,
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
  tokio::task::spawn_blocking(move || {
    build(&iso, &virtio, &bundle, &name, &account, cancel.as_deref())
  })
  .await
  .map_err(|e| Error::msg(e.to_string()))?
}

fn build(
  iso: &Path,
  virtio: &Path,
  bundle: &Path,
  name: &str,
  account: &Account,
  cancel: Option<&Path>,
) -> Result<PathBuf> {
  let image = bundle.join("install.img");
  let esd = iso
    .extension()
    .is_some_and(|extension| extension.eq_ignore_ascii_case("esd"));
  let size = std::fs::metadata(iso)?.len() * if esd { 3 } else { 1 } + 4 * 1024 * 1024 * 1024;
  std::fs::File::create(&image)?.set_len(sector_aligned(size))?;

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
      &[
        "partitionDisk",
        &disk,
        "GPT",
        "FAT32",
        "VISTABOOT",
        "3G",
        "ExFAT",
        "VISTAWIN",
        "R",
      ],
    )?;
    let volumes = mounted(&disk)?;
    let (boot, data) = (&volumes[0], &volumes[1]);

    let (iso_dev, source) = if iso
      .extension()
      .is_some_and(|extension| extension.eq_ignore_ascii_case("esd"))
    {
      (None, super::esd::unpack(iso, bundle, cancel)?)
    } else {
      let (device, source) = mount(iso)?;
      (Some(device), source)
    };
    let copied = (|| -> Result<()> {
      for entry in std::fs::read_dir(&source)?.flatten() {
        let name = entry.file_name();
        if name == "sources" {
          continue;
        }
        copy(&entry.path(), &boot.join(&name), cancel)?;
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
        copy(&entry.path(), &target, cancel)?;
      }
      Ok(())
    })();
    if let Some(device) = iso_dev {
      detach(&device);
    }
    copied?;
    if source == bundle.join("windows") {
      let _ = std::fs::remove_dir_all(&source);
    }

    let (virtio_dev, drivers) = mount(virtio)?;
    let injected = (|| -> Result<()> {
      let target = boot.join("$WinPEDriver$");
      for driver in DRIVERS {
        let folder = drivers.join(driver).join("w11").join("ARM64");
        if folder.is_dir() {
          copy(&folder, &target.join(driver), cancel)?;
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
    let resources = firmware()?;
    let bootloader = boot.join("efi/boot/bootaa64.efi");
    std::fs::rename(&bootloader, boot.join("efi/boot/windows.efi"))?;
    std::fs::create_dir_all(boot.join("efi/vista"))?;
    std::fs::copy(resources.join("boot.efi"), &bootloader)?;
    std::fs::copy(
      resources.join("display.efi"),
      boot.join("efi/vista/display.efi"),
    )?;
    std::fs::copy(resources.join("boot.efi"), vista.join("boot.efi"))?;
    std::fs::copy(resources.join("display.efi"), vista.join("display.efi"))?;
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
    let info = run(
      "/usr/sbin/diskutil",
      &["info", "-plist", &format!("{disk}s{slice}")],
    );
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

fn firmware() -> Result<PathBuf> {
  let executable = std::env::current_exe()?;
  let directory = executable
    .parent()
    .ok_or_else(|| Error::msg("No executable directory"))?;
  [directory.join("../Resources/windows"), directory.join("../windows")].into_iter()
    .find(|path| path.join("boot.efi").is_file() && path.join("display.efi").is_file())
    .ok_or_else(|| Error::msg("Windows boot firmware is missing. Reinstall Vista or run scripts/firmware.sh for a development build."))
}

fn sector_aligned(size: u64) -> u64 {
  size.div_ceil(512) * 512
}

pub fn repair_alignment(path: &Path) -> Result<()> {
  use std::io::{Read, Seek, SeekFrom};
  let mut file = std::fs::OpenOptions::new()
    .read(true)
    .write(true)
    .open(path)?;
  let size = file.metadata()?.len();
  if size % 512 == 0 {
    return Ok(());
  }
  let aligned = size / 512 * 512;
  if aligned < 1024 {
    return Err(Error::msg("The Windows installer image is too small."));
  }
  file.seek(SeekFrom::Start(512))?;
  let mut header = [0u8; 40];
  file.read_exact(&mut header)?;
  let backup = u64::from_le_bytes(header[32..40].try_into().unwrap());
  if &header[..8] != b"EFI PART" || backup != aligned / 512 - 1 {
    return Err(Error::msg(
      "The Windows installer has invalid disk geometry. Recreate it before starting.",
    ));
  }
  file.set_len(aligned)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn installer_images_round_to_a_complete_sector() {
    assert_eq!(sector_aligned(17_876_480_770), 17_876_481_024);
    assert_eq!(sector_aligned(4096), 4096);
  }

  #[test]
  fn repair_preserves_gpt_sectors_and_removes_only_the_unused_tail() {
    use std::io::{Seek, SeekFrom, Write};
    let path =
      std::env::temp_dir().join(format!("vistaalignment{}", uuid::Uuid::new_v4().simple()));
    let mut file = std::fs::File::create(&path).unwrap();
    file.set_len(1282).unwrap();
    file.seek(SeekFrom::Start(512)).unwrap();
    file.write_all(b"EFI PART").unwrap();
    file.seek(SeekFrom::Start(544)).unwrap();
    file.write_all(&1u64.to_le_bytes()).unwrap();
    drop(file);
    repair_alignment(&path).unwrap();
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 1024);
    std::fs::remove_file(path).unwrap();
  }
}
