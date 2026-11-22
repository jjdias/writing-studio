# Releasing Writing Studio

How a release is made, and what to try in the installed program before it goes out. The packages are built by CI; nothing here is built by hand.

## What a release is

Pushing a tag `vX.Y.Z` runs `.github/workflows/release.yml`. It checks the tag against the version in `Cargo.toml`, builds the packages with the Package workflow, and makes a GitHub Release with:

- `WritingStudio_X.Y.Z_x64_en-US.msi`, the Windows installer
- `Writing-Studio-windows-x64-portable.zip`, the same program to run from a folder
- `WritingStudio_X.Y.Z_amd64.deb`, for Debian and Ubuntu
- `SHA256SUMS.txt`, and the notes from `.github/release-notes.md`

Each package has a `licenses` folder: beside the program on Windows, and in `/usr/lib/WritingStudio/licenses` from the `.deb`. The installer and menu names come from `apps/desktop/packaging/`, as dx names the product after the binary.

## Steps

1. On `master`, with CI passing, set the version in `[workspace.package]` in `Cargo.toml`, build once so `Cargo.lock` follows, and commit.
2. Try a pre-release first: `git tag vX.Y.Z-rc.1`, then `git push origin vX.Y.Z-rc.1`. The release it makes is marked as a pre-release.
3. Download its packages and go through the check below on Windows, and on Linux (WSL2 on Windows 11 shows Linux windows: `wsl --install`, then a restart).
4. If something is wrong, fix it on `master` and try `-rc.2`. Delete a pre-release that is no longer wanted from its page on GitHub, or with `gh release delete vX.Y.Z-rc.1 --cleanup-tag`.
5. When the check passes, tag `vX.Y.Z` and push it.

## The check

Do each on Windows from the installer, and on Linux from the `.deb`. Write down what fails, and what looks different between the two.

1. **Install.** Windows: run the MSI; Writing Studio, with a space, is in the Start menu and on the desktop with the nib icon. Linux: `sudo apt install ./WritingStudio_X.Y.Z_amd64.deb` installs WebKitGTK with it, and Writing Studio is in the applications menu under Office.
2. **Start.** It opens without a console window, with the nib in the title bar and the taskbar. Note how long it takes to show the window.
3. **A project.** Create one in a new folder, add a chapter and a scene, write a few lines, close the program, open it again: the text is there.
4. **Spelling.** A misspelled word is listed; a suggestion replaces it; **Add to project dictionary** works, and the word can be taken out again. (This panel once failed only in release builds.)
5. **Compile.** Each built-in profile: Manuscript (Word), Ebook (with a cover), Web page, Plain text. Open each file in its own program. **Preview** shows the book.
6. **Fonts.** The compile font list shows the computer's fonts.
7. **Research.** Attach a file to a research note and open it: it opens in its own program.
8. **Sync.** With git installed, connect a project and sync it. Without git, the program says git is missing and everything else works.
9. **About.** Help (or Ctrl+/) ends with About: the version is this release's, and the folder it names holds `THIRD-PARTY-NOTICES.html`.
10. **Remove.** Windows: Settings → Apps removes it. Linux: `sudo apt remove writing-studio`. Projects stay where they were.
11. **Portable zip** (Windows): unzip it anywhere and open `Writing Studio.exe`; it runs the same.
