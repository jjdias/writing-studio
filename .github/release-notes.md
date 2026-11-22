Writing Studio {version}: a free, open-source writing app for novels and series. Everything stays on your computer.

## Install

**Windows 10 and 11**

- `WritingStudio_*_x64_en-US.msi` installs it, with a Start menu entry and a desktop shortcut. Remove it from Settings → Apps.
- `Writing-Studio-windows-x64-portable.zip` runs from any folder: unzip it and open `Writing Studio.exe`.
- The installer is not signed yet, so Windows may say it "protected your PC". Choose **More info**, then **Run anyway**.

**Debian and Ubuntu**

```bash
sudo apt install ./WritingStudio_*_amd64.deb
```

It needs WebKitGTK 4.1, which `apt` installs with it. On other Linux systems, build from source: see the README.

**Sync** between computers needs [git](https://git-scm.com/) installed. Writing and compiling do not.

## Check the download

`SHA256SUMS.txt` lists each file's SHA-256 checksum. On Windows, `Get-FileHash <file>` in PowerShell shows it; on Linux, `sha256sum -c SHA256SUMS.txt --ignore-missing`.

## Licenses

MIT or Apache-2.0, at your choice. Each package has a `licenses` folder with these, the dictionary's license, and the licenses of every library it is built from: beside the program on Windows, and in `/usr/lib/WritingStudio/licenses` on Linux.
