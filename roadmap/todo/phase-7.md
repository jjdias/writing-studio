# Phase 7 — Windows and Linux release

In progress: stages 1 to 3 are done.

Today Writing Studio runs by building it from source with `cargo run`. This phase makes it something a writer downloads and installs on Windows or Linux, built and checked by the project's own CI, with no cost to the project.

## Where things are now

- The program is one executable. The stylesheet, the editor scripts, and the English dictionary are compiled into it (`include_str!`), so a package is that file and its license notices.
- It runs on Windows, where it is written and tried. It has never been built on Linux.
- On Windows the window is WebView2, which Windows 10 and 11 already have. On Linux it is WebKitGTK, which the package has to require or carry.
- Sync runs the `git` installed on the computer. A computer without git can write and compile, and sync says git is missing.
- There is no CI. The repository is public on GitHub, so GitHub Actions costs nothing.
- There is no icon, no version shown in the program, and no package.

## What is not in this phase

- **macOS, iOS, and Android are dropped.** Distributing on Apple's systems needs a paid developer membership, and a phone needs a different interface from this desktop one. The code is not made to depend on Windows or Linux, but nothing is built or checked for other systems.
- **Dioxus Web/WASM is dropped** with them: a project is a folder on disk, which a browser page cannot keep.
- **Sync without installed git** (a git engine built into the program) moved to the bonus list. It was here for phones; on Windows and Linux git installs in a minute, and building it in needs a decision on how the program keeps SSH keys or tokens.
- No automatic updates: checking for one needs the network. A writer downloads the next release.
- No app store. Packages are on the project's GitHub Releases page.

## The work

### 1. CI on Windows and Linux

**Done.** `.github/workflows/ci.yml` builds and tests the workspace on both. The first Linux build passed with nothing to fix: 258 tests on Linux and 256 on Windows, the two more on Linux being the symlink checks that only run on Unix. A run takes about four minutes on Linux and five on Windows from a cold cache.

- A GitHub Actions workflow runs on every push and pull request: build the workspace and run its tests on `windows-latest` and `ubuntu-latest`.
- The Linux job installs the system libraries the window needs (WebKitGTK 4.1, GTK 3, and what Dioxus desktop's menus use). Whatever does not build or pass on Linux is fixed in this stage: the first Linux build is where it shows.
- `cargo fmt --check` is not added: the code is not formatted with rustfmt today, and formatting it all is a change of its own.
- The README says what a Linux computer needs to build from source.

### 2. The program's identity

**Done.** The icon is a pen nib on ink blue, drawn in `apps/desktop/icons/icon.svg`. `make-icons.ps1` beside it makes the PNGs, `icon.ico` (16 to 256 px), and `window-64.rgba`, the window's icon as raw pixels, with only what Windows has: Edge draws the SVG and .NET scales and writes the files. The window takes the raw pixels, so the program needs no image decoder. `Dioxus.toml` has the id, the publisher, and the icons for stage 3. **About Writing Studio** is at the end of Help: the version, the license, the source, and what spelling carries, as text, since a link would open inside the window. The third-party notice it will point to comes with stage 4.

- An icon, in the sizes Windows and Linux ask for, as the window's icon and the packages'. It needs a design the maintainer approves.
- An id for the desktop files and the installer (`io.github.jjdias.WritingStudio`), and the publisher's name.
- **Help → About**: the version, the license, and the third-party notices (stage 4), shown in the program.

### 3. Packages

**Done.** `.github/workflows/package.yml` builds the packages with `dx bundle`, installed as the binary Dioxus publishes (compiling it fails on the Windows runner). The Windows MSI and the portable zip are 7.5 MB each, and the Linux `.deb` 9.1 MB. The executable is `writing-studio`, so the packages name the product WritingStudio; `apps/desktop/build.rs` puts the icon and the version into it with `winresource`, a build-only dependency used only on Windows. A release-only build error in the spelling panel (`rsx!` writes a button's text after making its handler) was fixed, and CI now checks the release build. The start time is measured in stage 6, from the installed program.

- **Windows:** an installer (MSI) that adds the program to the Start menu and can be removed from Settings, and a portable `.zip` that runs from any folder.
- **Linux:** a `.deb` for Debian and Ubuntu that requires WebKitGTK from the system. An AppImage, for other distributions, moved to the bonus list: its build failed on the Ubuntu runner without saying why.
- Built with `dx bundle` from the Dioxus CLI, matched to the Dioxus version in `Cargo.lock` (0.6.3), with its settings in `apps/desktop/Dioxus.toml`. It is a build tool, installed in CI, not a dependency of the program.
- A release build profile: optimized, stripped, and built as one unit, so the download is small and the program starts fast. Its size and start time are written down here once, as the measure for later changes.

### 4. Licenses in every package

**Done once CI builds it.** The repository now has `LICENSE-MIT` and `LICENSE-APACHE` (it had neither, though the README and every manifest named them). The Package workflow fills `apps/desktop/licenses/` (ignored by git) before bundling, and `Dioxus.toml` makes every package install it beside the program: the two licenses, the dictionary's as `dictionary-LICENSE.txt`, and `THIRD-PARTY-NOTICES.html`, which `cargo-about` writes from `about.toml` and `about.hbs` with each crate's license text. `about.toml` lists the licenses a crate may be under, so a new kind stops the build. Every crate is permissive but `spellbook` and `option-ext` (MPL-2.0, used unchanged); the notice says where their source is. Help's About points to the folder. Generated locally: 454 crates for Windows and Linux, 309 KB.

- The program's own licenses (MIT and Apache-2.0), the dictionary's license (`crates/spellcheck/dictionaries/en_US.LICENSE.txt`), and a notice of every crate the program is built from with its license, generated at build time.
- `spellbook` is MPL-2.0: the notice says where its source is.

### 5. Releases

- Pushing a version tag (`v0.1.0`) builds the packages in CI and attaches them, with their SHA-256 checksums, to a GitHub Release. Nothing is published by hand.
- The version comes from the workspace's `Cargo.toml`; the tag must match it, or the release stops.
- **Signing the Windows installer** is a decision for the maintainer: without it, Windows SmartScreen warns on first run. SignPath Foundation signs open-source projects for free, after the project applies. Until then the release notes say how to get past the warning.

### 6. A pass over the packaged program

- Start the installed program on Windows and on Linux, open and create a project, write, compile each format, and sync, from the package rather than from `cargo run`.
- What differs between WebView2 and WebKitGTK (fonts, the editor, file dialogs, keyboard shortcuts) is fixed where it can be, or written down where it cannot.
- Linux is tried in WSL2 on the maintainer's Windows computer, which shows Linux windows on Windows 11. Installing WSL2 is the maintainer's to do (`wsl --install`, then a restart).

## How changes are checked

The tests in this repository are the only testing for now (see "How the work is done now" in [../README.md](../README.md)).

- CI runs every test on Windows and on Linux. A change is done when both pass.
- The release workflow is checked by a tag on a test version before the first real release.
- How the packaged program looks is checked by the maintainer in stage 6, as in Phases 8 and 9.
