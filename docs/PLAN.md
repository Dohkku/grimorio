English · [Español](PLAN.es.md)

# Grimorio plan

## What it is

A visual library for Linux on par with Eagle: capture, organize, find and
preview references without the program getting in the way. Eagle only exists on
Windows and macOS, runs on Chromium 107 and does not share its code.

On Linux there are similar things (digiKam is for photographers and its
interface belongs to another era, TagStudio has the right philosophy but lacks
the performance, Hydrus is for archivists), but none of them fills the gap: **a
multi-format visual library, with an instant viewer, polished, extensible and
fast with hundreds of thousands of items**.

## Decisions made

| Topic | Decision |
|---|---|
| Interface | Rust core + QML layer (Qt 6) + thin C++ glue. Confirmed with numbers in M0 (see docs/M0.md) |
| Files | Copy into the library by default, like Eagle. `mover` and `referenciar` exist in the core and are exposed as a setting in M3 |
| License | Free software under AGPL-3.0 from the first public version. Library format documented in public |

## Scope

**MVP (0.1):** portable library, import (drag, folder, URL), hierarchical
folders with multiple membership, tags, virtualized grid, instant viewer,
inspector, search with filters, rating and notes.

**v1:** smart folders, color search, watched folders,
video/audio/PDF/fonts/RAW, duplicates, plugins.

**Later:** comparison and presentation, design formats (PSD, AI, Sketch),
sync. The browser extension is already done (queue, 16).

## Architecture

Three layers and one stable seam:

```
Layer 3 · UI shell        grid, viewer, inspector, shortcuts, themes, plugins
   ▲ commands · queries · subscriptions ▼
Layer 2 · Command bus     validation, permissions, undo, events
   ▲ internal Rust API ▼
Layer 1 · Core            storage, index, search, import, thumbnails
```

- Undo is implemented once, in the bus, and covers every operation.
- Plugins get the same bus with reduced permissions: there is no back door
  different from the one the application itself uses.
- The CLI (`grim`) is the first client of the bus. It exists from day one and
  is used for tests and measurements without opening a window.
- If the toolkit turns out to be a mistake, layer 3 is thrown away and the
  other two are kept.

Status: all three layers are standing. The explicit bus and undo arrived in M2:
every edit goes through `Library::editar`, which returns the way back.

## Performance budgets

With 100,000 items, on a mid-range laptop. They are measured at every
milestone; a regression is a blocking bug.

| Metric | Target | Measured in M0 |
|---|---|---|
| Startup to usable grid | < 400 ms | 324 ms |
| Dropped frames while scrolling | 0 | 0.09 % above 16.6 ms |
| Text search (p95) | < 30 ms | 11-18 ms |
| Tag 10,000 | no freezing | 344 ms, 2 frames |
| Open the viewer | < 120 ms | 6.2 ms (p95) |
| Memory at rest | < 400 MB | 124 MB |
| Import (12 MP JPEG) | > 20/s | 63.7/s at 6.75 MP |

## Milestones

- **M0 · Spike and skeleton**: core from the CLI, synthetic library of
  100,000, grid benchmark. *Gate: toolkit decision with numbers.*
  Status: **closed**. Qt Quick wins; wgpu stays as the bar and plan B.
- **M1 · Seeing**: window, grid, full screen viewer, folder tree, drag and
  drop, theme tokens with hot reload.
  *Gate: stutter-free scrolling with 100,000 and a viewer under 120 ms.*
  Status: **closed**. 131 fps in square grid and 94 in justified with 100,000
  items; viewer with a p95 of 7 ms. See docs/M1.md.
- **M2 · Organizing**: tags with autocompletion, rating, notes, batch editing,
  full inspector, search bar with filters, global undo, trash.
  *Gate: tag 10,000 items without freezing the interface.*
  Status: **closed**. 344 ms end to end with the grid at 133 fps and two
  dropped frames. See docs/M2.md.
- **M3 · Swallow everything**: video, audio, PDF, fonts, RAW in isolated
  processes; watched folders; reference mode; smart folders; color search;
  duplicates; desktop integration.
  *Gate: import 50 GB of mixed files without losing a single item.*
  Status: **in progress**. File families; RAW and audio cover art with no
  dependencies; video and PDF in isolated processes with a time limit; exact
  and perceptual duplicates (100,000 in half a second); video and sound
  playback in the viewer, with optional QtMultimedia; 3D models (STL, OBJ, PLY,
  glTF/GLB, 3MF, and `.blend`/`.fbx` through Blender) with thumbnail and their
  own viewer; waveform and spectrogram for sound; PDF page by page; a playable
  copy of the videos Qt cannot open. Still to do: fonts, smart folders, watched
  folders and desktop integration.
- **M4 · MVP 0.1**: `.deb`, AppImage, Flatpak; shortcuts; Spanish and English;
  importer for Eagle libraries; backups.
  *Gate: two weeks of my own daily use without data loss.*

Estimate to MVP: 12 to 17 weeks of focused work.

## Work queue

What has been asked for and is not done yet, in the order it is being done. It
empties from the top; closed items leave here and go into the milestone they
belong to.

| | What | Status |
|---|---|---|
| 1 | Inspector ratings kept in sync with the grid | done |
| 2 | Side panels with adjustable width | done |
| 3 | New folder next to the tree, and dragging folders to change the hierarchy | done |
| 4 | Dragging items from one folder to another: move instead of only adding | done |
| 5 | Video that plays on hover in the grid, looping | done |
| 6 | Video playable in the right-hand panel | done |
| 7 | Switch: play all visible videos at once | done |
| 8 | Mark as 18+, with a safe mode that blurs and lifts when opened | done |
| 9 | A ProRes video should not say «Internal data stream error» | done: H.264 copy |
| 10 | See the content of a PDF, beyond its cover | done |
| 11 | 3D models: STL for printing, `.blend`, and the exchange formats | done |
| 12 | Sound: waveform in time and frequency, jump to a moment with the mouse, slow motion | done |
| 13 | Richer video controls | done: frame by frame, speed, A–B, volume, preview on the scrub bar |
| 14 | The same image at several file sizes, as a single one | done: version stacks, lighter versions with the size shown, suggest stacking on import |
| 15 | Remove background | done: `grim-recorte` with ISNet, local, with the model bundled |
| 16 | Browser extension, mainly for Instagram | done for photos and carousels in Brave/Chrome; Instagram videos and Firefox, later |

About 8, what was asked in detail: safe mode blurs what is marked; opening a
marked item shows it **only while it is open**, and back in the grid it is
still blurred; turning safe mode off is the only thing that lifts it entirely.

About 7 there is a physical limit worth writing down: each playing video is a
decoding pipeline. The visible ones play, with a cap, and the players are
recycled instead of created and destroyed. Building and tearing down the
pipeline many times in a row crashes the program (see app/README.md).

## Risks

| Risk | Mitigation |
|---|---|
| The toolkit cannot handle the grid | Spike measured before writing any interface; layer 3 disposable by design |
| Two build chains (Cargo + CMake) | Integration and CI from the first commit |
| Massive JSON writes in batch operations | Immediate index, deferred flush, operation log |
| Drag and drop on Wayland | Test early on GNOME and KDE, use portals |
| Scope that stretches by itself | The scope table is a contract: adding means removing |
| One-person project | Core with real tests and a complete CLI: it survives a pause |
