# Keeping a project on more than one computer

Sync is optional. A project that is never synced works the same.

Writing Studio syncs through **git**, with any git host you choose. It runs the `git` program installed on your computer, with the keys or tokens you set up for git. It stores no passwords, and it has no account or server of its own.

## Before the first sync

1. **Install git.** If it is not installed, History says so, and everything else still works.
2. **Tell git who you are**, once per computer. Commits carry this name and email:

   ```bash
   git config --global user.name "Your Name"
   ```

   ```bash
   git config --global user.email "you@example.com"
   ```

3. **Make a private repository** with a git host, empty, with no README. Every host has a "New repository" page; choose **Private**.

   **Anyone who can read the repository can read the book.** A public repository makes the manuscript public.

4. **Let git reach it** without asking for a password, because the program cannot show git's password prompt:
   - With **SSH**: add your SSH public key to your account on the host. Use the address that starts with `git@`.
   - With **HTTPS**: set up git's credential helper with an access token from the host, and use the address that starts with `https://`.

   Test it in a terminal with `git ls-remote <address>`. If that works without a prompt, sync will.

## Connecting a project

Open the project, go to **History**, and under **Sync** paste the repository's address, then **Connect**. The project folder becomes a git repository connected to that address. The first **Sync** sends the project there.

On another computer, clone the repository into a folder, and open that folder in Writing Studio:

```bash
git clone <address> "My Novel.writingstudio"
```

## Syncing

**Sync** in the top bar, or **Sync now** in History:

1. saves everything, and commits it with a message that names the scenes that changed;
2. brings in what the other computer sent;
3. if both computers changed the project, merges the two;
4. sends the result.

The manuscript cannot be edited while a sync runs. Closing the window commits what is saved, without the network, so the next sync has it.

History shows when the project last synced, and how much there is to send or receive.

## Earlier versions

**History** → **Earlier syncs** lists the selected scene as it was each time it was synced. **Compare** shows what changed since then; **Restore** puts that text back as one step that Ctrl+Z undoes.

## When both computers changed the same thing

Nothing written on either computer is dropped.

- Paragraphs changed on only one computer are taken.
- **Paragraphs changed differently on both** are both kept, this computer's first. The scene opens with track changes on, and the other computer's paragraphs are listed as changes: **Accept** keeps them, **Reject** takes them out.
- A title, status, or other detail changed differently on both keeps this computer's.
- **A context** merges by paragraph, as a scene's text does: each computer's new paragraphs are kept, and a paragraph changed differently on both keeps both versions, this computer's first, and the sync says so. What each file lists as `connected:` is not merged: the program writes it again from the item.
- Something deleted on one computer and changed on the other is kept.
- A file in `attachments/` (one added to a note, or a cover) changed on both keeps this computer's, with the other's beside it, ending in `-theirs`.

History lists everything the last sync could not combine.

## What is not synced

`.local/` stays on each computer: snapshots, backups, track-changes baselines, writing sessions, the theme, and the window's layout.

## Large research files

Research files are stored as they are. Many large files (scans, audio) make the repository slow. Git LFS keeps them outside the main history; set it up in the project folder before adding them:

```bash
git lfs track "attachments/**"
```

## When sync stops

The notice at the foot of the window says why. The common cases:

- **"git does not know your name and email yet"**: step 2 above.
- **A message about permission, authentication, or a host key**: git could not reach the repository without a prompt; step 4 above.
- **"git took too long"**: the network is slow or down. Sync again later; nothing was lost.
- **"git is in the middle of a merge or rebase"**: something was started with git in a terminal. Finish or abort it there.
- **"inside another git repository"**: move the project folder out of the other repository's folder.
