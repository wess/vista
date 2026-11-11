//! Answer files for an unattended Windows 11 for Arm install.
//!
//! `setup()` runs in Windows PE: it lifts Setup's hardware checks and hands
//! over to `script()`, which partitions disk 0, applies Windows 11 Pro with
//! DISM, makes it bootable, and copies `system()` into place so the first
//! real boot creates the account and skips the out-of-box screens.

use crate::machine::Account;

const COMPONENT: &str = r#"processorArchitecture="arm64" publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS""#;

fn escape(text: &str) -> String {
  text
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

/// A Windows computer name: 15 characters, letters, digits and hyphens.
pub fn computer_name(name: &str) -> String {
  let cleaned: String = name
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
    .collect::<String>()
    .trim_matches('-')
    .chars()
    .take(15)
    .collect();
  if cleaned.is_empty() || cleaned.chars().all(|c| c.is_ascii_digit()) {
    "VISTA-PC".into()
  } else {
    cleaned
  }
}

/// A Windows account name: 20 characters, none of the reserved ones.
pub fn user_name(name: &str) -> String {
  let cleaned: String = name
    .chars()
    .filter(|c| !"\"/\\[]:;|=,+*?<>@ ".contains(*c))
    .take(20)
    .collect();
  match cleaned.to_lowercase().as_str() {
    "" | "administrator" | "guest" => "vista".into(),
    _ => cleaned,
  }
}

pub fn setup() -> String {
  let bypass = ["BypassTPMCheck", "BypassSecureBootCheck", "BypassRAMCheck", "BypassCPUCheck", "BypassStorageCheck"]
    .iter()
    .enumerate()
    .map(|(i, key)| {
      format!(
        r#"        <RunSynchronousCommand wcm:action="add"><Order>{}</Order><Path>reg.exe add HKLM\SYSTEM\Setup\LabConfig /v {key} /t REG_DWORD /d 1 /f</Path></RunSynchronousCommand>"#,
        i + 1
      )
    })
    .collect::<Vec<_>>()
    .join("\n");
  format!(
    r#"<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend" xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <settings pass="windowsPE">
    <component name="Microsoft-Windows-International-Core-WinPE" {COMPONENT}>
      <SetupUILanguage><UILanguage>en-US</UILanguage></SetupUILanguage>
      <InputLocale>0409:00000409</InputLocale>
      <SystemLocale>en-US</SystemLocale>
      <UILanguage>en-US</UILanguage>
      <UserLocale>en-US</UserLocale>
    </component>
    <component name="Microsoft-Windows-Setup" {COMPONENT}>
      <RunSynchronous>
{bypass}
        <RunSynchronousCommand wcm:action="add"><Order>6</Order><Path>cmd.exe /c "for %d in (C D E F G H I J K L M N O P Q R S T U V W Y Z) do if exist %d:\vista\install.cmd call %d:\vista\install.cmd %d:"</Path></RunSynchronousCommand>
      </RunSynchronous>
      <UserData><AcceptEula>true</AcceptEula></UserData>
    </component>
  </settings>
</unattend>
"#
  )
}

pub fn script() -> String {
  [
    "@echo off",
    "set MEDIA=%1",
    "for %%d in (C D E F G H I J K L M N O P Q R S T U V W Y Z) do if exist %%d:\\sources\\install.wim set SRC=%%d:\\sources\\install.wim",
    "for %%d in (C D E F G H I J K L M N O P Q R S T U V W Y Z) do if exist %%d:\\sources\\install.esd set SRC=%%d:\\sources\\install.esd",
    "(echo select disk 0& echo clean& echo convert gpt& echo create partition efi size=260& echo format quick fs=fat32 label=System& echo assign letter=S& echo create partition msr size=16& echo create partition primary& echo format quick fs=ntfs label=Windows& echo assign letter=W) > X:\\vistadisk.txt",
    "diskpart /s X:\\vistadisk.txt",
    "dism /Apply-Image /ImageFile:%SRC% /Name:\"Windows 11 Pro\" /ApplyDir:W:\\",
    "dism /Image:W:\\ /Add-Driver /Driver:%MEDIA%\\$WinPEDriver$ /Recurse",
    "bcdboot W:\\Windows /s S: /f UEFI",
    "mkdir W:\\Windows\\Panther",
    "copy /y %MEDIA%\\vista\\unattend.xml W:\\Windows\\Panther\\unattend.xml",
    "wpeutil shutdown",
  ]
  .join("\r\n")
}

pub fn system(name: &str, account: &Account) -> String {
  let computer = escape(&computer_name(name));
  let user = escape(&user_name(&account.username));
  let full = escape(if account.fullname.is_empty() { &account.username } else { &account.fullname });
  let password = escape(&account.password);
  let autologon = if account.autologin {
    format!(
      "<AutoLogon><Enabled>true</Enabled><Username>{user}</Username><Password><Value>{password}</Value><PlainText>true</PlainText></Password></AutoLogon>"
    )
  } else {
    String::new()
  };
  format!(
    r#"<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend" xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <settings pass="specialize">
    <component name="Microsoft-Windows-Shell-Setup" {COMPONENT}>
      <ComputerName>{computer}</ComputerName>
      <TimeZone>UTC</TimeZone>
    </component>
    <component name="Microsoft-Windows-Deployment" {COMPONENT}>
      <RunSynchronous>
        <RunSynchronousCommand wcm:action="add"><Order>1</Order><Path>reg.exe add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\OOBE" /v BypassNRO /t REG_DWORD /d 1 /f</Path></RunSynchronousCommand>
      </RunSynchronous>
    </component>
  </settings>
  <settings pass="oobeSystem">
    <component name="Microsoft-Windows-International-Core" {COMPONENT}>
      <InputLocale>0409:00000409</InputLocale>
      <SystemLocale>en-US</SystemLocale>
      <UILanguage>en-US</UILanguage>
      <UserLocale>en-US</UserLocale>
    </component>
    <component name="Microsoft-Windows-Shell-Setup" {COMPONENT}>
      <OOBE>
        <HideEULAPage>true</HideEULAPage>
        <HideOEMRegistrationScreen>true</HideOEMRegistrationScreen>
        <HideOnlineAccountScreens>true</HideOnlineAccountScreens>
        <HideWirelessSetupInOOBE>true</HideWirelessSetupInOOBE>
        <HideLocalAccountScreen>true</HideLocalAccountScreen>
        <ProtectYourPC>3</ProtectYourPC>
      </OOBE>
      <UserAccounts>
        <LocalAccounts>
          <LocalAccount wcm:action="add">
            <Name>{user}</Name>
            <DisplayName>{full}</DisplayName>
            <Group>Administrators</Group>
            <Password><Value>{password}</Value><PlainText>true</PlainText></Password>
          </LocalAccount>
        </LocalAccounts>
      </UserAccounts>
      {autologon}
    </component>
  </settings>
</unattend>
"#
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn names_are_windows_safe() {
    assert_eq!(computer_name("My Windows Box!"), "My-Windows-Box");
    assert_eq!(computer_name("1234"), "VISTA-PC");
    assert_eq!(user_name("Administrator"), "vista");
    assert_eq!(user_name("we:ss"), "wess");
  }

  #[test]
  fn values_are_escaped() {
    let account = Account {
      fullname: "A & B".into(),
      username: "ab".into(),
      password: "p<w>".into(),
      autologin: true,
      ssh: false,
    };
    let xml = system("Box", &account);
    assert!(xml.contains("A &amp; B"));
    assert!(xml.contains("p&lt;w&gt;"));
    assert!(setup().contains("BypassTPMCheck"));
  }
}
