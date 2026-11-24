# Sonar

A file manager inspired by KDE Dolphin, built with Tauri 2 and Rust. Targets macOS and Arch Linux.

**Status: 0.1.1, early but usable for daily work.** Browsing, live updates, opening files, and file operations
(rename, new folder, trash, delete, copy/cut/paste with progress and conflict handling, undo), drag and drop
(within Sonar and with other apps), the system clipboard, tabs, split view and rubber-band selection, OS file
types and icons, image thumbnails, disk usage and mount/eject in the sidebar, a native menu bar on macOS,
archives (browse, extract, compress, passwords, archives inside archives), fuzzy search, Properties, Open With,
an editable Places sidebar, and Settings (startup, view defaults, thumbnails, confirmations). On Linux, it
can also act as the file chooser for other apps via the XDG Desktop Portal (see
[below](#using-sonar-as-the-file-chooser-linux)).

Folders with 100,000 files stay fast, file names that aren't valid UTF-8 work like any other, and a network
share that stops answering doesn't lock up the window.

Not there yet: Dolphin's F4 terminal panel and a quick preview (Space). See the
[v1 issue](https://github.com/samtoxie/sonar/issues/1) for what 1.0 should have.

## Install

### Arch Linux

From the AUR, with an AUR helper such as [paru](https://github.com/Morganamilo/paru):

```sh
paru -S sonar-bin   # the release build, ready to run
paru -S sonar       # or: built from the release's source (a few minutes)
paru -S sonar-git   # or: built from the latest development version
```

Install `udisks2` too if you want to mount drives from the sidebar.

### Nix

Run directly without installing:

```bash
nix run "github:samtoxie/sonar?dir=packaging/nix"
```

Install into your profile:

```bash
nix profile install "github:samtoxie/sonar?dir=packaging/nix"
```

Or add it to your NixOS / Home Manager `flake.nix` inputs:

```nix
{
  inputs = {
    sonar.url = "github:samtoxie/sonar?dir=packaging/nix";
  };

  # Then reference packages.\${pkgs.system}.default in your packages list
}
```

### Other Linux / macOS

Download the latest build from [Releases](https://github.com/samtoxie/sonar/releases):

- **Linux:** the AppImage (or the `.deb` / `.rpm`). Sonar uses a few standard tools at runtime
  (`shared-mime-info`, `xdg-utils`, `udisks2`, `libarchive` and an icon theme such as `breeze-icons`; all
  present on a usual KDE or GNOME install). If one is missing, the feature that needs it says which package to
  install. The `.deb`, `.rpm` and AUR packages also register Sonar as a file chooser backend for the
  XDG Desktop Portal — see [below](#using-sonar-as-the-file-chooser-linux).
- **macOS (Apple Silicon):** the `.dmg`. The build isn't notarized: after copying Sonar to Applications, open it
  once, then choose Open Anyway under System Settings → Privacy & Security.

To make Sonar the default file manager, see [below](#making-sonar-your-default-file-manager).

## Layout

```
crates/sonar-core/      UI-agnostic filesystem model (no Tauri dependency)
  src/listing.rs        read a directory into sorted Entry values
  src/entry.rs          one row in the details view
  src/sort.rs           sort keys, folders first, natural ("file2" < "file10") ordering
  src/breadcrumb.rs     path to clickable segments (paths under $HOME start with "Home")
  src/places.rs         sidebar: standard user dirs from the `dirs` crate, trash, volumes
  src/paths.rs          resolve typed locations (~, relative, file://)
  src/clipboard.rs      file-list formats: file:// URIs, uri-list, GNOME/KDE cut markers
  src/thumbnail.rs      image thumbnails, cached per the freedesktop thumbnail spec
  src/mimeglob.rs       file name -> MIME type from shared-mime-info's globs, indexed (Linux)
  src/uipath.rs         lossless paths for the UI: bytes that aren't UTF-8 survive the JSON boundary
  src/volumes.rs        parsing lsblk / udisksctl output (removable media on Linux)
  src/archive.rs        archives via bsdtar (libarchive): list, browse, extract, compress, passwords, nesting
  src/search.rs         recursive fuzzy search (fzf's algorithm via nucleo-matcher), streamed in batches
  src/properties.rs     Properties: type, sizes, dates, owner, permissions; live folder size totals
  src/desktop.rs        freedesktop .desktop files: parsing, Exec= field-code expansion (Open With on Linux)
  src/filetype.rs       extension-based type labels and icon hints
  src/watch.rs          debounced folder watcher (FSEvents / inotify via `notify`)
  src/ops/              file operations
    basic.rs            rename (never overwrites), new folder, trash, permanent delete
    transfer.rs         copy/move engine: progress, cancel, conflicts, atomic .sonar-part writes
    undo.rs             reversible record of each operation
    names.rs            name validation, "name (1).ext" generation
  src/platform/         OS-specific behaviour behind one API
    macos.rs            UF_HIDDEN flag, /Volumes, `open`, NSFileManager trash, renamex_np, APFS clonefile,
                        UTType names, NSWorkspace icons, `diskutil` eject
    linux.rs            `.hidden` files, /proc/self/mounts, `xdg-open`, FreeDesktop trash (`trash` crate), renameat2,
                        shared-mime-info types, icon themes (KDE/GTK setting), udisks mount/eject
    generic.rs          fallback so the crate still builds on other platforms
src-tauri/              Tauri shell: thin #[tauri::command] wrappers over sonar-core, plus per-view
                        state (current watcher, background item-count job)
  src/native/           UI-thread platform code: system clipboard (NSPasteboard / GTK), drag-out
  src/protocol.rs       sonar:// scheme serving OS icons and thumbnails to the webview
  src/menu.rs           native macOS menu bar (Linux gets Dolphin's ☰ menu in the toolbar)
  src/picker.rs         file-chooser requests from the XDG portal: picker window, pending requests
  src/portal.rs         the portal itself: org.freedesktop.impl.portal.FileChooser over D-Bus (Linux)
ui/                     static frontend, native ES modules (no bundler)
  js/app.js             window: tabs, split view, toolbar, location bar, sidebar, keyboard routing
  js/pane.js            one folder view: navigation, history, selection, filter, live updates
  js/views.js           details and icons renderers; only the rows near the viewport are in the DOM
  js/ops.js             file operations, jobs panel, conflict dialog, undo
  js/menu.js            HTML context menus (+ contextmenus.js for their contents)
  js/dnd.js             drag and drop: in-window, out to other apps, and drops from them
  js/store.js           settings, per-folder view properties, Places customisations
  js/properties.js      Properties dialog
  js/settings.js        Configure Sonar dialog
  js/openwith.js        Open With: app chooser, "always use" for a file type
  js/password.js        password prompt for encrypted archives
  js/picker.js          the file-chooser mode of a window (opened via the portal; ?picker=<id>)
```

## Prerequisites

- Rust (stable) and Node.js
- **macOS:** Xcode Command Line Tools
- **Arch Linux:** `sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module libappindicator-gtk3 librsvg`
  (runtime: see [Install](#install))

## Running

```sh
npm install
npm run dev      # tauri dev
npm run build    # release bundle (.app/.dmg on macOS; .deb/.rpm/AppImage on Linux)
```

## Making Sonar your default file manager

Sonar opens folders it is asked to show in a new tab of the running window (a file is shown selected in its
folder). `sonar ~/Downloads` works from a terminal too.

**macOS** (Finder itself can't be replaced, but folder-open requests can go to Sonar):

1. Copy Sonar.app to /Applications and open it once (the build is unsigned: allow it under
   System Settings → Privacy & Security).
2. `brew install duti && duti -s dev.sonar.filemanager public.folder all`

Now `open ~/Downloads`, folders opened from other apps and the Dock open in Sonar. Folders you double-click
inside Finder windows still open in Finder. Undo with `duti -s com.apple.finder public.folder all`.

**Arch Linux / KDE Plasma:** the AUR packages (and the `.deb`/`.rpm`) install the launcher; for the AppImage,
create `~/.local/share/applications/sonar.desktop` from `src-tauri/linux/sonar.desktop` with
`Exec=/path/to/Sonar.AppImage %U`. Then pick Sonar under System Settings → Default Applications → File Manager,
or run `xdg-mime default sonar.desktop inode/directory` (with the `.deb`/`.rpm`: `Sonar.desktop`).

## Using Sonar as the file chooser (Linux)

Apps that use the [XDG Desktop Portal](https://flatpak.github.io/xdg-desktop-portal/) for file dialogs —
Firefox, Chromium, Flatpak apps, most GTK/Qt apps — can open Sonar for "Open File" / "Save As" instead of
the GTK or KDE dialog. The `.deb`, `.rpm` and AUR packages install the portal registration
(`org.freedesktop.impl.portal.FileChooser`); it does nothing until you opt in.

Enable it in `~/.config/xdg-desktop-portal/portals.conf` (create the file if it doesn't exist):

```ini
[preferred]
org.freedesktop.impl.portal.FileChooser=org.freedesktop.impl.portal.desktop.sonar
```

(The value is the backend's name, which is the `.portal` file name minus the extension —
`org.freedesktop.impl.portal.desktop.sonar.portal`. Don't shorten it to `sonar`: an unknown backend
name makes the portal quietly fall back to the desktop default, or offer no chooser at all.)

then restart the portal service and the apps:

```sh
systemctl --user restart xdg-desktop-portal
```

(On GNOME/KDE the desktop's own `*-portals.conf` may list this interface explicitly; the user file above
takes precedence — if yours doesn't do it, create `gnome-portals.conf`/`kde-portals.conf` with the same
content instead.)

From then on, the app's file dialog is a normal Sonar window: navigate, type a name, pick filters, tick
the options the app offers, and hit its "Open"/"Save" button. Cancel (or Escape, or closing the window)
cancels the dialog. Sonar starts on demand via D-Bus when an app asks for a file and quits by itself a
few minutes after the last chooser closes; no need to keep it running.

- The AppImage can't register itself system-wide; use the `.deb`, `.rpm` or an AUR package instead
  (or copy the two files from `src-tauri/linux/` into `/usr/share/xdg-desktop-portal/portals/` and
  `/usr/share/dbus-1/services/`).
- Remove the line (or the file) and restart the portal to go back to the default chooser.

## Tests

```sh
cargo test --workspace                   # core: listing, sorting, ops engine, undo, watcher, platform
cargo test -p sonar-core -- --ignored    # also round-trips a file through the real Trash
npx playwright install chromium webkit   # once
npm run test:ui                          # UI suite in Chromium and WebKit
```

The UI tests (`tests/ui/`) run the real frontend against `mock-backend.js`, an in-memory filesystem that
implements every Tauri command and event, so they cover full flows (navigation, tabs, split view, rename,
copy with conflicts, undo, live refresh) without touching disk. WebKit is the engine Tauri uses on both
macOS and Linux. CI (`.github/workflows/ci.yml`) runs everything on Ubuntu and macOS, and the core crate on
Arch Linux.

## Keyboard

| Action | Linux | macOS |
|---|---|---|
| Back / Forward | Alt+← / Alt+→ | ⌘[ / ⌘] |
| Up | Alt+↑, Backspace | ⌘↑, Backspace |
| Open folder / file | Enter, double-click | Enter, ⌘↓, double-click |
| Edit location | Ctrl+L, F6, click empty space in the bar | ⌘L, F6 |
| Show hidden files | Ctrl+H, Alt+. | ⌘⇧., Ctrl+H |
| Reload | F5, Ctrl+R | ⌘R |
| Select | click, Shift/Ctrl+click, Shift+arrows, Ctrl+A | ⌘ instead of Ctrl |

### File operations

| Action | Linux | macOS |
|---|---|---|
| Rename | F2 | F2 |
| New folder | Ctrl+Shift+N | ⌘⇧N |
| Move to Trash | Delete | ⌘⌫, Delete |
| Delete permanently (asks first, unless turned off in Settings) | Shift+Delete | ⌘⌥⌫ |
| Copy / Cut / Paste | Ctrl+C / X / V | ⌘C / X / V |
| Paste as move (Finder style) | — | ⌘⌥V |
| Copy location (path as text) | Ctrl+Alt+C | ⌘⌥C |
| Undo | Ctrl+Z | ⌘Z |

Copy and cut use the system clipboard, so files move freely between Sonar and Finder, Dolphin, Nautilus and
other apps (Linux: `text/uri-list`, `x-special/gnome-copied-files`, `application/x-kde-cutselection`).

Drag items onto folders, a pane's empty space, places, breadcrumbs or tabs (hover a tab to switch to it).
Dropping asks Move Here / Copy Here; hold Shift to move or Ctrl (⌥ on macOS) to copy without asking. Drag
out of the window to drop files on other apps; files dragged in from other apps drop the same way. Drag on
empty space, or on a row outside its name, to rubber-band select (Ctrl/⌘ toggles, Shift adds).

### Archives

zip, 7z, rar, tar (gz/xz/bz2/zst), iso and more, through `bsdtar` (libarchive: built into macOS; on Arch
it's always installed since pacman depends on it; Debian/Ubuntu: `libarchive-tools`).

- Double-click an archive to browse it like a (read-only) folder; copy, paste or drag items out to extract
  just those. Opening a file inside an archive extracts it to a temporary folder first.
- **Extract Here** (context menu): a single top-level folder is extracted as is; several items go into a
  new folder named after the archive. Nothing is ever overwritten. With split view, **Extract to** the other side.
- **Compress** (context menu): zip, 7z, tar.gz, tar.xz or tar.bz2, next to the selection.
- **Password-protected** zips ask for the password when it's needed (browsing works without it), and again if
  it's wrong. Passwords are remembered until Sonar quits. (libarchive can't decrypt encrypted 7z or RAR.)
- **Archives inside archives** open like folders too: the inner archive is extracted to a temporary folder.

Extracting and compressing run as background jobs with progress, cancel and undo.

Deleting inside the Trash always deletes permanently. Copies and moves run in the background with a progress
panel; name clashes ask Skip / Keep Both / Replace (or Merge for folders), optionally for all remaining conflicts.

### Views, tabs and split

| Action | Linux | macOS |
|---|---|---|
| Icons / Details view | Ctrl+1 / Ctrl+3 | ⌘1 / ⌘2 |
| Search this folder and below | Ctrl+F | ⌘F |
| Filter current folder | Ctrl+I | ⌘⌥F |
| Properties | Alt+Enter | ⌘I, ⌥Enter |
| Split view on/off | F3 | F3 |
| New tab / Close tab | Ctrl+T / Ctrl+W | ⌘T / ⌘W |
| Reopen closed tab | Ctrl+Shift+T | ⌘⇧T |
| Next / previous tab | Ctrl+Tab / Ctrl+Shift+Tab | ⌃Tab, ⌘⇧] / ⌘⇧[ |
| Settings | Ctrl+Shift+, | ⌘, |

On macOS the menu bar's Go menu also has Finder's shortcuts (⌘⇧H Home, ⌘⇧D Desktop, ⌘⇧O Documents,
⌘⌥L Downloads); on Linux the same actions are in the toolbar's ☰ menu.

Tabs (and their split views) are restored on the next launch, and so are the window's size and position
(Settings can open Home or a chosen folder instead). Drag tabs to reorder them, double-click empty tab-bar
space for a new tab, and middle-click a tab to close it. Closing the last tab closes the window. In split view,
each side's header has a × to close that side.
Middle-click a folder, place or breadcrumb to open it in a background tab. Right-click items, empty space,
places, breadcrumbs or tabs for context menus. View mode and sort order are remembered per folder (or shared
by all folders, in Settings); hidden-file visibility is global.

Type a few letters to jump to a matching entry. Mouse back/forward buttons also work.

### Search, Properties, Open With, Places

- **Search** matches names fuzzily, like fzf (`rep2025` finds `report-2025.pdf`; `docs/rep` matches the
  path), best matches first, streaming in while it searches; case never matters. `*` and `?` switch to glob
  matching. Results show their folder; Enter/double-click opens, and "Open Containing Folder" jumps there.
  Network shares and system folders below the search folder are skipped (search inside one to include it).
- **Properties** shows type, size (folders are counted live), dates, owner, permissions and link targets.
- **Open With** (context menu) lists the apps registered for the file type; "Other Application…" opens a chooser
  that can also make the choice the default for that type.
- **Places**: drag folders onto the sidebar or use "Add to Places"; right-click a place to rename, hide or
  remove it; drag places to reorder. "Show Hidden Places" and "Reset Places" are on the sidebar's menu.

## Settings

Configure Sonar (☰ menu, Ctrl+Shift+,; on macOS Sonar → Settings…, ⌘,) sets what opens at startup, the
default view and sort order, whether views are remembered per folder, hidden files, image thumbnails, and
which actions ask for confirmation. Changes apply immediately.

## License

Sonar is licensed under the [European Union Public Licence v. 1.2](LICENSE) (EUPL-1.2).
