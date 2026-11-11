@echo off
setlocal EnableExtensions
set "MEDIA=%~1"
set "LOG=%MEDIA%\vista\install.log"
echo [%time%] Starting Windows installation > "%LOG%"
if exist "%MEDIA%\$WinPEDriver$\viostor\viostor.inf" drvload "%MEDIA%\$WinPEDriver$\viostor\viostor.inf" >> "%LOG%" 2>&1
set COUNT=0
for /l %%d in (0,1,7) do (
  > X:\probe.txt echo select disk %%d
  diskpart /s X:\probe.txt >nul 2>&1 && (
    >> X:\probe.txt echo select partition 1
    diskpart /s X:\probe.txt >nul 2>&1 || (set TARGET=%%d& set /a COUNT+=1)
  )
)
if not "%COUNT%"=="1" (echo Expected one empty system disk, found %COUNT% >> "%LOG%" & goto failed)
echo Installing to empty disk %TARGET% >> "%LOG%"
(
  echo select disk %TARGET%
  echo clean
  echo convert gpt
  echo create partition efi size=260
  echo format quick fs=fat32 label=System
  echo assign letter=S
  echo create partition msr size=16
  echo create partition primary
  echo format quick fs=ntfs label=Windows
  echo assign letter=W
) > X:\partition.txt
diskpart /s X:\partition.txt >> "%LOG%" 2>&1 || goto failed
set "SRC="
for %%d in (C D E F G H I J K L M N O P Q R S T U V W Y Z) do if exist %%d:\sources\install.wim set "SRC=%%d:\sources\install.wim"
for %%d in (C D E F G H I J K L M N O P Q R S T U V W Y Z) do if exist %%d:\sources\install.esd set "SRC=%%d:\sources\install.esd"
if not defined SRC (echo Windows system image is missing >> "%LOG%" & goto failed)
echo [%time%] Applying Windows 11 Pro >> "%LOG%"
dism /Apply-Image /ImageFile:"%SRC%" /Name:"Windows 11 Pro" /ApplyDir:W:\ /Compact >> "%LOG%" 2>&1 || goto failed
echo [%time%] Installing virtual hardware drivers >> "%LOG%"
dism /Image:W:\ /Add-Driver /Driver:"%MEDIA%\$WinPEDriver$" /Recurse >> "%LOG%" 2>&1 || goto failed
mkdir W:\Windows\Panther 2>nul
copy /y "%MEDIA%\vista\unattend.xml" W:\Windows\Panther\unattend.xml >> "%LOG%" || goto failed
bcdboot W:\Windows /s S: /f UEFI >> "%LOG%" 2>&1 || goto failed
mkdir S:\EFI\vista 2>nul
mkdir S:\EFI\Boot 2>nul
copy /y S:\EFI\Microsoft\Boot\bootmgfw.efi S:\EFI\Microsoft\Boot\windows.efi >> "%LOG%" || goto failed
copy /y "%MEDIA%\vista\boot.efi" S:\EFI\Microsoft\Boot\bootmgfw.efi >> "%LOG%" || goto failed
copy /y "%MEDIA%\vista\boot.efi" S:\EFI\Boot\bootaa64.efi >> "%LOG%" || goto failed
copy /y "%MEDIA%\vista\display.efi" S:\EFI\vista\display.efi >> "%LOG%" || goto failed
echo [%time%] Windows image applied successfully >> "%LOG%"
copy /y "%LOG%" W:\Windows\Panther\vista.log >nul
echo OK> "%MEDIA%\vista\result"
wpeutil shutdown
goto waiting
:failed
echo [%time%] Installation failed. See the preceding error. >> "%LOG%"
echo FAILED> "%MEDIA%\vista\result"
wpeutil shutdown
:waiting
ping -n 60 127.0.0.1 >nul
goto waiting
