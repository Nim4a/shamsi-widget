# Installer

`shamsi-widget.nsi` — a **NSIS 3** script that packages the release build of Shamsi Widget into a
per-user setup wizard. No admin rights, no UAC prompt: everything lands in the current user's profile.

## Build

Put the release artefacts in one folder (the three files the wizard ships):

```
shamsi_widget.exe   # release build, fonts embedded
widget.ico          # tray icon, must sit next to the exe
README.md           # project readme
```

then run (the `.nsi` keeps a UTF-8 BOM so the Persian strings compile correctly):

```bash
makensis /DSRCDIR="<abs path of that folder>" shamsi-widget.nsi
# -> ShamsiWidget-Setup-1.0.0.exe
```

`makensis` lives in `C:\Program Files (x86)\NSIS\makensis.exe` (default NSIS install).

## What the wizard does

| Step | Behaviour |
|:--|:--|
| Language | **فارسی (default, RTL)** + English, picked on the first page |
| Install dir | `%LOCALAPPDATA%\Programs\Shamsi Widget`, remembered in `HKCU\Software\ShamsiWidget\InstallDir` |
| Install | closes a running `shamsi_widget.exe`, copies exe + icon + readme, writes the uninstall entry, creates `unins000.exe` |
| Start Menu | `Shamsi Widget` program group with the app and the uninstaller |
| Components | optional desktop shortcut, optional *run at Windows startup* (`…\Run\ShamsiWidget`) |
| Finish | optional “run Shamsi Widget now”, optional link to the repository |
| Uninstall | removes files, shortcuts, Start-Menu group and registry entries; a running instance is closed first. It guesses the *autostart* value and only deletes the `Run` entry when it points inside `$INSTDIR`. **`%APPDATA%\ShamsiWidget` (settings) is deliberately preserved** — reinstalling keeps colours and position. |

## Verification done

End-to-end silent test on this machine (`/S` install into a scratch dir with a redirected `APPDATA`):
files installed → installed exe launched → `/S` uninstall → install dir, Start-Menu group and the
*Add & Remove Programs* key all removed, a foreign `Run\ShamsiWidget` value left untouched, and the real
`%APPDATA%\ShamsiWidget\settings.ini` byte-identical before/after.
