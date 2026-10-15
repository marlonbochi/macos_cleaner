### Install

**macOS** (Apple Silicon and Intel, macOS 13+): download the `.dmg`, open it and drag **Disk
Cleaner** to **Applications**. The app is not notarized by Apple yet: open it once, then go to
**System Settings → Privacy & Security** and click **Open Anyway** (or run
`xattr -dr com.apple.quarantine "/Applications/Disk Cleaner.app"`). Grant **Full Disk Access** when
asked so the scan can see every folder.

**Windows** (10/11, x64): download `Disk.Cleaner_<version>_x64-setup.exe` and run it. The installer
is not signed yet, so SmartScreen may warn you: click **More info → Run anyway**.

**Debian / Ubuntu** (amd64): download the `.deb` and install it with
`sudo apt install ./Disk.Cleaner_<version>_amd64.deb`.

To uninstall, use **Settings → Uninstall Disk Cleaner** inside the app: it removes the app, its
data and its stored API keys.

Upgrading from **macOS Cleaner** 0.1.x: your catalogue, settings and API keys are moved over on the
first launch.
