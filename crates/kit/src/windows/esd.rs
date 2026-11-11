use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Result};
use regex::Regex;

fn tool() -> Result<PathBuf> {
  let directory = std::env::current_exe()?
    .parent()
    .map(Path::to_path_buf)
    .ok_or_else(|| Error::msg("No executable directory"))?;
  [directory.join("../Resources/windows/imagex"), directory.join("../windows/imagex"),
    PathBuf::from("/opt/homebrew/bin/wimlib-imagex"), PathBuf::from("/usr/local/bin/wimlib-imagex")]
    .into_iter().find(|path| path.is_file())
    .ok_or_else(|| Error::msg("The Windows media preparation tool is missing. Reinstall Vista; development builds require wimlib."))
}

pub(super) fn command(tool: &Path, args: &[&str], cancel: Option<&Path>) -> Result<Vec<u8>> {
  let base = std::env::temp_dir().join(format!("vistamedia{}", uuid::Uuid::new_v4().simple()));
  let stdout = base.with_extension("out");
  let stderr = base.with_extension("err");
  let result = (|| -> Result<Vec<u8>> {
    let mut child = Command::new(tool)
      .args(args)
      .stdout(std::fs::File::create(&stdout)?)
      .stderr(std::fs::File::create(&stderr)?)
      .spawn()?;
    loop {
      if cancel.is_some_and(Path::exists) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Error::msg("Cancelled."));
      }
      if let Some(status) = child.try_wait()? {
        if !status.success() {
          return Err(Error::msg(format!(
            "Windows media preparation failed: {}",
            std::fs::read_to_string(&stderr).unwrap_or_default().trim()
          )));
        }
        return Ok(std::fs::read(&stdout)?);
      }
      std::thread::sleep(std::time::Duration::from_millis(100));
    }
  })();
  let _ = std::fs::remove_file(stdout);
  let _ = std::fs::remove_file(stderr);
  result
}

fn text(bytes: &[u8]) -> String {
  if bytes.starts_with(&[0xff, 0xfe]) {
    String::from_utf16_lossy(
      &bytes[2..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>(),
    )
  } else {
    String::from_utf8_lossy(bytes).into_owned()
  }
}

fn images(xml: &str) -> Result<[String; 4]> {
  let image = Regex::new(r#"(?s)<IMAGE INDEX="([0-9]+)">(.*?)</IMAGE>"#).expect("WIM images");
  let mut found: [Option<String>; 4] = Default::default();
  for capture in image.captures_iter(xml) {
    let details = capture[2].to_lowercase();
    let slot = if details.contains("<name>windows setup media</name>") {
      Some(0)
    } else if details.contains("<name>microsoft windows pe (arm64)</name>") {
      Some(1)
    } else if details.contains("<name>microsoft windows setup (arm64)</name>") {
      Some(2)
    } else if details.contains("<editionid>professional</editionid>")
      && details.contains("<arch>12</arch>")
    {
      Some(3)
    } else {
      None
    };
    if let Some(slot) = slot {
      found[slot] = Some(capture[1].into());
    }
  }
  let [Some(media), Some(pe), Some(setup), Some(pro)] = found else {
    return Err(Error::msg(
      "Microsoft’s installer does not contain Arm64 Windows Setup and Windows 11 Pro",
    ));
  };
  Ok([media, pe, setup, pro])
}

pub(super) fn unpack(esd: &Path, bundle: &Path, cancel: Option<&Path>) -> Result<PathBuf> {
  let tool = tool()?;
  let esd = esd.to_string_lossy();
  let info = text(&command(&tool, &["info", &esd, "--xml"], cancel)?);
  let [media, pe, setup, pro] = images(&info)?;
  let folder = bundle.join("windows");
  std::fs::create_dir_all(&folder)?;
  command(
    &tool,
    &[
      "apply",
      &esd,
      &media,
      &folder.to_string_lossy(),
      "--no-acls",
      "--no-attributes",
    ],
    cancel,
  )?;
  let sources = folder.join("sources");
  std::fs::create_dir_all(&sources)?;
  let boot = sources.join("boot.wim");
  let install = sources.join("install.wim");
  command(
    &tool,
    &[
      "export",
      &esd,
      &pe,
      &boot.to_string_lossy(),
      "--compress=LZX",
      "--threads=2",
    ],
    cancel,
  )?;
  command(
    &tool,
    &[
      "export",
      &esd,
      &setup,
      &boot.to_string_lossy(),
      "--boot",
      "--threads=2",
    ],
    cancel,
  )?;
  command(
    &tool,
    &[
      "export",
      &esd,
      &pro,
      &install.to_string_lossy(),
      "--compress=LZX",
      "--threads=2",
    ],
    cancel,
  )?;
  Ok(folder)
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  #[ignore = "requires wimlib"]
  fn builds_setup_media_from_a_real_esd_archive() {
    let root = std::env::temp_dir().join(format!("vistaesd{}", uuid::Uuid::new_v4().simple()));
    let source = root.join("source");
    let output = root.join("output");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(source.join("sample.txt"), b"sample").unwrap();
    let esd = root.join("sample.esd");
    let tool = tool().unwrap();
    for (index, name) in [
      "Windows Setup Media",
      "Microsoft Windows PE (arm64)",
      "Microsoft Windows Setup (arm64)",
      "Windows 11 Pro",
    ]
    .iter()
    .enumerate()
    {
      command(
        &tool,
        &[
          if index == 0 { "capture" } else { "append" },
          &source.to_string_lossy(),
          &esd.to_string_lossy(),
          name,
          "--compress=LZMS",
          "--solid",
          "--threads=2",
        ],
        None,
      )
      .unwrap();
    }
    command(
      &tool,
      &[
        "info",
        &esd.to_string_lossy(),
        "4",
        "--image-property=WINDOWS/EDITIONID=Professional",
        "--image-property=WINDOWS/ARCH=12",
      ],
      None,
    )
    .unwrap();
    let prepared = unpack(&esd, &output, None).unwrap();
    assert!(prepared.join("sample.txt").is_file());
    let boot = text(
      &command(
        &tool,
        &[
          "info",
          &prepared.join("sources/boot.wim").to_string_lossy(),
          "--xml",
        ],
        None,
      )
      .unwrap(),
    );
    assert_eq!(boot.matches("<IMAGE INDEX=").count(), 2);
    let install = text(
      &command(
        &tool,
        &[
          "info",
          &prepared.join("sources/install.wim").to_string_lossy(),
          "--xml",
        ],
        None,
      )
      .unwrap(),
    );
    assert!(install.contains("Professional"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn selects_images_by_role_instead_of_assuming_pro_index() {
    let xml = "<WIM><IMAGE INDEX=\"1\"><NAME>Windows Setup Media</NAME></IMAGE><IMAGE INDEX=\"2\"><NAME>Microsoft Windows PE (arm64)</NAME></IMAGE><IMAGE INDEX=\"3\"><NAME>Microsoft Windows Setup (arm64)</NAME></IMAGE><IMAGE INDEX=\"7\"><WINDOWS><ARCH>12</ARCH><EDITIONID>Professional</EDITIONID></WINDOWS></IMAGE></WIM>";
    assert_eq!(images(xml).unwrap(), ["1", "2", "3", "7"]);
    assert!(images(&xml.replace("<ARCH>12", "<ARCH>9")).is_err());
  }
}
