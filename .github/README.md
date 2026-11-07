<p align="center">
  <img src="../pjassets/WaveR.png" width="200" alt="WaveR Logo">
</p>

<h1 align="center">WaveR</h1>
<p align="center">A simple, cozy local music player.</p>
<p align="center"><sub>Pronounced Wave-R</sub></p>

<p align="center">
  <picture><source media="(prefers-color-scheme: dark)" srcset="https://www.shieldcn.dev/github/release/FRANORDE/WaveR.svg?size=sm&amp;mode=dark&amp;font=jetbrains-mono"><img alt="Release" src="https://www.shieldcn.dev/github/release/FRANORDE/WaveR.svg?size=sm&amp;mode=light&amp;font=jetbrains-mono"></picture>
  <picture><source media="(prefers-color-scheme: dark)" srcset="https://www.shieldcn.dev/github/last-commit/FRANORDE/WaveR.svg?variant=secondary&amp;size=sm&amp;mode=dark&amp;font=jetbrains-mono"><img alt="Last commit" src="https://www.shieldcn.dev/github/last-commit/FRANORDE/WaveR.svg?variant=secondary&amp;size=sm&amp;mode=light&amp;font=jetbrains-mono"></picture>
  <picture><source media="(prefers-color-scheme: dark)" srcset="https://www.shieldcn.dev/github/license/FRANORDE/WaveR.svg?variant=ghost&amp;size=sm&amp;mode=dark&amp;font=jetbrains-mono"><img alt="License" src="https://www.shieldcn.dev/github/license/FRANORDE/WaveR.svg?variant=ghost&amp;size=sm&amp;mode=light&amp;font=jetbrains-mono"></picture>
  <picture><source media="(prefers-color-scheme: dark)" srcset="https://www.shieldcn.dev/github/commits/FRANORDE/WaveR.svg?variant=secondary&amp;size=sm&amp;mode=dark&amp;font=jetbrains-mono"><img alt="Commits" src="https://www.shieldcn.dev/github/commits/FRANORDE/WaveR.svg?variant=secondary&amp;size=sm&amp;mode=light&amp;font=jetbrains-mono"></picture>
</p>

# Features

Plays mp3, flac, wav and ogg files

### Queue

When selecting a folder, WaveR will automatically put all the songs in the queue.
Alternively, you can use the `Add to queue` button and select the songs you want.
You can remove songs from the queue using the `X` button next to them.

### Volume

In the bottom-right corner, you can find the volume bar.
The volume slider only changes WaveR’s volume, not the system volume.

# Installation

## Installer

Download the latest `WaveR-Setup-<version>.exe` from the [releases page](https://github.com/FRANORDE/WaveR/releases/latest) (use releases! Downloading the code directly might be unstable).

Run the installer and follow the wizard. By default WaveR is installed in `%USERPROFILE%\.waver` and no administrator rights are needed.

The wizard lets you choose:
- whether to create a desktop shortcut (a Start Menu entry is always created)

> **Note:** since the installer is not code-signed, Windows SmartScreen may show a warning the first time.
> Click **More info → Run anyway**.

### Uninstall

Use **Settings → Apps → Installed apps** and remove WaveR.

## Scoop

Requires [Scoop](https://scoop.sh/) installed

---

```
scoop bucket add franorde https://github.com/FRANORDE/scoop-bucket
scoop install waver
```

### Uninstall

```
scoop uninstall waver
scoop bucket rm franorde
```

---

More info at [scoop-bucket](https://github.com/FRANORDE/scoop-bucket/tree/main#installation)

# Usage

Open WaveR from the Start Menu or your desktop shortcut (if created).
Then select `Open file` or `Add folder` and select your song/folder.

<p>
  <img src="../pjassets/WaveRphoto.png" width="500" style="max-width:100%;" alt="WaveR Photo">
</p>

# Building from source

This is only needed if you want to compile WaveR yourself.

**Prerequisite:** [Rust](https://rustup.rs) and [Git](https://git-scm.com/install/) must be installed.

```
git clone https://github.com/FRANORDE/WaveR.git
cd WaveR
cargo build --release
```

The executable will be in `target\release\WaveR.exe`.

# Notes, credits and licenses

## Notes

- Some AI was used for development, but the code was manually reviewed and tested.
- Free and open-source software is the best software forever.

## Thanks to

- [Inno Setup](https://jrsoftware.org/isinfo.php) for the great installer system
- [egui/eframe](https://github.com/emilk/egui) for the simple GUI
- [rodio](https://github.com/RustAudio/rodio) for the rust audio playback library
- [rfd](https://github.com/PolyMeilex/rfd) for the native file dialogs
- You, for using WaveR!

## Licenses

After installing with the installer, all licenses can also be found in the `licenses` folder next to the app.

----

WaveR is released under the [MIT License](../LICENSE).

WaveR is built with [egui/eframe](https://github.com/emilk/egui) and other open-source Rust libraries. Their licenses (mostly MIT and Apache-2.0) are listed in [THIRD-PARTY-NOTICES.txt](../licenses/THIRD-PARTY-NOTICES.txt).

### Trademarks

WaveR is an independent community project and is not affiliated with other apps or products that share a similar name.
