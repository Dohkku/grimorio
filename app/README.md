English · [Español](README.es.md)

# app: Grimorio

The application. The core lives in `crates/grimorio-core`; between the two
there is a C ABI bridge (`app/puente`) with its own thread, a JSON command bus
and immutable views. The explanation is in `docs/M1.md` (the application) and
`docs/M2.md` (undo, trash and batch editing).

## Build

```sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j
```

`cargo` is only invoked to build the bridge.

## Run

```sh
./build/grimorio ~/Referencias.grimorio
./build/grimorio ~/Referencias.grimorio --tema ../temas/papel.json
```

The theme reloads when you save it, with the program open. Without `--tema`
the one chosen in the settings (`Ctrl+,`) is used: «night» or «paper», which
are built into the binary. The interface size is also there (text, icons,
margins and panels, from 80 to 150 %). It is applied by multiplying the theme
tokens, not with `QT_SCALE_FACTOR`, so thumbnails stay sharp.

For a test library: `grim synth ~/bench.grimorio --items 100000`.

## Keys

| | |
|---|---|
| `Ctrl+F` | search |
| `Ctrl+A` | select all |
| `Ctrl+Z` / `Ctrl+Shift+Z` | undo and redo |
| `Del` | move to the trash (inside the trash, restore) |
| `Ctrl+K` | tag the selection |
| `F2` | rename (with several selected, rename with a pattern) |
| `Ctrl+C` | copy the selection (files, and the image if it is one) |
| `Ctrl+V` | paste: image, files or a web address |
| `Ctrl+E` | export the selection to a folder |
| `Ctrl+Shift+N` | new folder |
| `Ctrl+Shift+X` | capture an area of the screen |
| `Ctrl+L` | cycle through the three views: justified, grid and list |
| `Ctrl+I` | collapse or open the inspector |
| `Ctrl+,` | settings: theme, interface size, grid; and «About» |
| `Ctrl+±` · `Ctrl+0` | interface size; `Ctrl+0` resets it to 100 % |
| `Ctrl`+wheel | over the grid: thumbnail size |
| `0`…`5` | rate the selection |
| `Enter` | open the viewer |
| `Space` | in the viewer: next, or pause and resume if it is playing |
| `Space` | outside the viewer, with a video in the inspector: pause and resume it |
| `F` · double click on a video | in the viewer: full screen; `Esc` leaves it without closing the viewer |
| wheel · `Z` | in the viewer, with a video: zoom in without pausing (dragging moves the frame); `Z` resets it |
| wheel · `+` `−` · `Z` | in the viewer, with a photo: zoom in and out down to the pixel; fit to window ⇄ 100 % |
| `C` | in the viewer, with a photo: copy the color of the pixel under the mouse |
| `R` / `Shift+R` | in the viewer, with a photo or a video: rotate it right or left. It stays rotated (in the grid, the inspector and the viewer) and `Ctrl+Z` undoes it; the file is not touched |
| `Shift+←/→` | in the viewer: ten seconds back or forward |
| `,` / `.` | in the viewer: one frame back or forward (it pauses first) |
| `[` / `]` | in the viewer: slower or faster, from 0.1× to 2× |
| `I` / `O` | in the viewer: start and end of the looped range |
| `X` | in the viewer: clear the range |
| `L` / `M` | in the viewer: loop and mute |
| `PgUp` / `PgDn` | in a PDF: previous and next page |
| arrows | move around the grid, with `Shift` to extend the selection |
| `Esc` | close whatever is open, or clear the selection |

The arrows belong to the grid, so **touching the grid gives it the focus
back**: clicking a folder or typing in the search box takes the focus away (it
has to), and without that return the arrows kept talking to the panel for the
rest of the session. You would select a photo with the mouse and moving with
the keyboard did nothing. The gap between cells counts as grid and also gives
the focus back.

While a text field has the focus, the shortcuts that compete with typing are
turned off: `Enter`, `Esc`, `Del`, `F2`, `Ctrl+Z` and the numbers. A window
`Shortcut` takes the key **before** it reaches the field, and without this,
naming a folder while an image was selected opened the viewer instead of
creating the folder.

Both side panels are widened by dragging the line that separates them, and a
double click on that same line returns them to what the theme says. The width
is stored in `~/.config/Grimorio/Grimorio.conf`, not in the theme: the theme is
project data, and this belongs to whoever is sitting at the screen.

Click to select, `Ctrl+click` to add, `Shift+click` for a range. Right click
on a cell opens its actions (view, open externally, copy, export, move,
rename, adult content, trash); they are the same as the inspector buttons.
Dragging files onto the window imports them.

## The window

Three columns, like a studio: **where you look from** on the left, **what you
look at** in the middle and **what you selected** on the right. The two side
columns run from top to bottom; the top bar and the status bar belong only to
the middle.

* **Left.** The library name (click it for the others), the fixed views
  («All», «Untagged», «Recent», «Trash»), the smart folders and the folder
  tree, each with its colored dot. At the bottom, how many items there are
  and how much space they take, with the split by type in a bar (hovering
  over each segment says what it is).
* **Middle.** At the top, the name of what you are looking at and how many
  items it has; on the right, the search box, the three views (justified,
  grid, list), the sort order and «filters», which expands the filter row and
  shows how many are active even when collapsed. That row also holds «names»
  (the name under each cell) and the cell size.
* **Right.** The inspector, always present even when nothing is selected:
  that way the grid does not change width every time something is selected.
  Large preview (with several selected, a mosaic of the first ones), name
  (click to change it), rating, palette (clicking a color searches for things
  that look like it), tags, note, technical details with folders and source,
  and the actions. `Ctrl+I` collapses it.

The fixed views are searches like any other, written in the search box:
«Untagged» is `etiquetado:no` and «Recent» is `fecha:7d`. The search box is
still the only thing that decides what is shown.

In **list** view, each item is a row with its thumbnail, name, type,
dimensions, file size, rating and date. Clicking a column header sorts by it.
It is the same cell as in the other two views, so selecting, dragging to a
folder, reordering and moving with the arrows all work the same way.

The view, the names and the inspector are remembered in
`~/.config/Grimorio/Grimorio.conf`.

## Folders

A folder is a tag with a hierarchy, and the file does not live in it: an item
can be in several.

«All» means *having none selected*: besides its own row, you get back to it by
clicking the selected folder again or the empty space below the tree. The
fixed views at the top are not folders and cannot be renamed or dragged.

| You do | What happens |
|---|---|
| click a folder | you look at that folder |
| click the one already selected | back to «All» |
| click the empty space below the tree | back to «All» |
| right click a folder | create inside, rename, color, link a folder on disk, delete |
| right click the empty space | create at the root |

Touching the panel takes the focus away from the grid: if you keep typing after
clicking here, the keys no longer reach the photos.

Dragging does what the person dragging expects:

| Dragged | From | Does |
|---|---|---|
| the selection | a folder | **moves** it: it goes into the target folder and leaves the source folder, in a single operation that is undone in one step |
| the selection | «All» or a search | **adds** it, without taking it out of anything |
| the selection | anywhere, onto the empty space below the tree | removes it from the folder that was being viewed |
| an item, with `Shift` | the grid, within it | **reorders** it: the others make room and it stays where it is dropped. Without `Shift` the grid stays still, so it can be dragged out to another application without getting in the way. `Shift+click` without dragging still selects a range |
| a folder | the empty space below the tree | goes back to the root |

A folder dropped on another row does one thing or another depending on
**where** in the row it is dropped, and the line that appears says which
before you drop:

```
─── top edge ─────►  goes before that one, as a sibling
    middle         ►  goes inside that one, as its last child
─── bottom edge ──►  goes after that one, as a sibling
```

Without the bands, the tree could only nest under another folder and the
order between siblings could not be changed. Items have no bands: wherever
they land within the row, they go into the folder, because an item has no
place between two folders.

Nesting and placing are the **same** operation in the core (`recolocar`), a
single one: dragging a row is one gesture, and two operations would leave two
undo entries for something that was seen happening once. When placing, the
`pos` values of the group are renumbered in steps of ten; if two siblings were
tied, the alphabetical tiebreak would override what the person dragging just
decided.

A folder cannot be dropped inside itself or one of its descendants: the row
stops lighting up before the drop, not after. What cannot be done is dimmed,
but the row **does not reject** the drag: Qt never notifies a drop area again
once it has rejected a drag, so rejecting left the row deaf for the rest of the
drag and also took away its two edges, which is exactly where dropping was
allowed.

### What travels in an internal drag

Two Qt Quick traps, both invisible: everything looked right and nothing
worked. They are pinned down by the `arrastre` test, which loads the real rows
and moves the mouse.

**The payload does not go in `Drag.mimeData`.** In an internal drag that map
does not reach the other side: the receiver of the drop sees the keys and the
source, and an empty `formats`, so `getDataAsString` always returns an empty
string. The target row lit up (that only looks at the keys) and dropping did
nothing, because the identifier to move arrived blank. What crosses over is
the key, which says **what kind of thing** is being dragged; **which one** is
kept by the window for the duration of the gesture. The keys still do their
own job: deciding which area accepts what. That is why the window-wide import
area asks for `text/uri-list`, which is what a file from outside carries:
without that key it grabbed any internal drag and lit up the «drop here to
import» overlay when you moved a photo to a folder.

**You can see what is being dragged from the first pixel.** A ghost attached
to the cursor (the thumbnail of the selection with a count, or the folder
name) and a closed-hand cursor. Without that, dragging was an act of faith:
the only signal came at the end of the way, when the target row lit up. The
position is set by the drag item itself, which is the one that already knows
it.

**The drag item is attached to the cursor by hand** (`Bulto.qml`). Qt looks
for the drop area where the drag item is, not where the mouse is, and left to
itself the item starts at the corner of whatever releases it and also falls
behind by the drag threshold, because `MouseArea` subtracts it so the dragged
thing does not jump when the drag starts. With rows two fingers tall that is
half a band of error: you asked for «inside» and got «after».

### When a grid video freezes

Qt can print «Failed to start video surface due to main thread blocked» on
startup: the multimedia engine asks the interface thread to take over to set
up the output, gets tired of waiting and never asks again. It happens while the
grid is decoding its first screen. The player keeps counting position and the
cell stays a still photo forever.

`Reproductor.qml` counts the frames that reach the output and, if none arrive
within three seconds, starts again. Three times and it gives up. Neither the
position nor the size can tell you this is happening: the position advances
anyway (decoding works; displaying is what fails) and the size is filled in as
soon as the format is known. Measured: with eight of those warnings, all five
players reported a size and a position.

A handful of frames per round does not mean it is broken either: with the
screen off the compositor stops requesting draws and the whole window drops to
one frame per second. That is why any frame counts, with no minimum.

### The grid players are never destroyed, and that has to be verified

It is written at the very top of `MallaVideos.qml` and even so it was not
true. The trickle that creates the players one at a time used
`model: <a number>`, and `Repeater` **recreates all** its children every time
that number changes: counting births and deaths, five players were born
fifteen times and died ten. Exactly what the header says crashes the program.
Now the model is a list that is only appended to, and it is five births and
zero deaths.

Three more things that used to be paid for on the interface thread and no
longer are:

* **Changing query empties the assignment immediately.** It holds indexes, and
  an index from the previous view points to something else in the new one
  (often a photo, which the player decodes as a one-frame video and paints
  where the old video was, at its size). That was the square that appeared
  floating out of place when entering a folder.
* **Losing its slot does not release the source.** The source is set by hand
  instead of being bound to the assignment: without a slot the player goes
  silent and hides, but keeps what it had. When it was bound, the source went
  blank and that tears down the whole pipeline. Measured on a folder change
  with autoplay video: **2.3 s** until the folder appeared, against 0.6 s with
  autoplay video off. Without tearing down, 0.7 s with video on.
* **Leaving the screen pauses instead of stopping**, and coming back in starts
  playback. `stop` releases the pipeline; `pause` decodes nothing and resumes
  instantly. And starting on return was needed: stopping was easy to request
  and nobody was there to request starting, so a video that lost its slot and
  got the same one back stayed on its cover forever. That is what happened
  when entering a folder whose videos were already playing.
* **Paused for three seconds, it is released.** Just pausing is expensive: a
  paused pipeline does not decode but holds on to everything it has, and on a
  machine with GPU video decoding each one carries its own context. Measured
  scrolling the grid up and down five times, with five videos: pausing only,
  from 0.93 to 1.33 GB; releasing after three seconds, from 0.82 to 0.86. The
  delay is what lets both things coexist: going back to the folder you just
  left costs nothing, and what does not come back does not stay taking up
  memory.

These three were fixed while the mode that played every visible video still
existed. That mode is gone (the numbers that killed it are in «Playback»),
but the fixes still apply: the grid player is a single one that comes and
goes the same way, and the viewer and the inspector set up pipelines of their
own.

## Not a single frame to JavaScript

The grid rescue counts the frames that reach the output, and it used to count
them with a `Connections` on `videoFrameChanged`. Each notification hands the
frame to JavaScript as a value, and that value holds on to the decoder texture
until the garbage collector runs, which cannot see that memory and is in no
hurry. With video on the graphics card, one 1080p texture per frame: video
memory grew by **250 MB per second** with a single looping video, and the same
with an empty handler. Clicking between two videos multiplied it (inspector
and grid playing at the same time): in four minutes the program held 13.5 GB
of graphics memory and 14 GB of RAM, stuttered and died.

Now they are counted in C++ (`src/fotogramas.cpp`), connecting by signal name
to avoid linking QtMultimedia, and the frame never leaves that code. Measured
with the same looping video: from 353 to 2580 MiB in nine seconds before; a
flat 284 MiB for twenty-one seconds after. The rule: **no signal that carries a
`QVideoFrame` is listened to from QML.**

## Switching video: pause, and release later

Clicking another video froze the interface for almost a second. Opening the
new one was fine (that runs in the background and comes back in a
millisecond); the cost was releasing the old one: Qt 6.4 brings the pipeline
down to NULL and waits, on the interface thread, and a running pipeline takes
much longer to come down than a paused one. Now the player pauses at once,
hides the output and switches source 100 ms later. Measured switching between
two videos every second and a half: from a median of **804 ms** (1.3 s max) to
**179 ms** (251). With a 50 ms wait the pause had not settled yet; with 200
nothing more is gained. A burst of switches only pays for the last one.

## The focus ring

Selecting something opens the inspector, and opening it narrows the grid: the
whole row is laid out again and what you just selected moves, sometimes to
another row. With only the cell's own border, you have to find it again with
your eyes every time.

The jumps stay (the justified grid works that way, and avoiding them would
mean always reserving the inspector's space), but you can avoid losing track.
A ring with a halo marks what the inspector shows and **slides** from where it
was to where it goes, in 180 ms, with a flash of light that fades on arrival.
You follow it with your eyes instead of searching for it.

Three decisions inside:

* **A single one for the whole grid**, above the cells and the videos, rather
  than one per cell. Asking each of the three hundred and eighty-four cells
  whether it is the selected one means three hundred and eighty-four bindings
  to `modelo.actual`, and this paints the same thing with one node. It is the
  same reason the videos sit above the cells and not inside them.
* **The cell reports where it was when clicked**, before touching the
  selection. Without that the ring has nowhere to start from: by the time
  anyone learns which one is selected, the grid has already been laid out
  again and the old position no longer exists. And the report goes out on
  **press**, not on release, because selection happens on press.
* **The halo is three bands**, each wider and fainter, and they are wide
  borders instead of filled rectangles: a fill tints the photo cyan, and the
  point is to highlight the photo without covering it. Without an effects
  module (the Qt in Ubuntu 24.04 does not ship `QtQuick.Effects`), this is
  what a gradient looks like.

## Moving with the arrows, and what it cost

Each arrow changes the focus, and changing the focus moved half the
application. Three things that were paid on every key press and no longer are:

* **The model notification is translated to slots.** The model notifies per
  item and the grid serves per slot; the shortcut in place ignored which item
  was being notified and repainted all four hundred slots with all eight
  roles. Selecting sends two notifications (the one being left and the one
  being taken), so each key press did it twice.
* **The closed viewer no longer loads anything.** Its full-size preview was
  bound to `modelo.actual` without checking whether the viewer was open: each
  key press decoded a large image for nobody.
* **The inspector waits for you to stop** before giving the source to the
  player. Without that, moving across videos with the arrows sets up and tears
  down a multimedia pipeline per key press, on the interface thread.

And the focus ring **jumps instead of travelling when keys come in quick
succession**: the travel and the glow are there so you do not lose track of a
jump, and when moving fast you are not looking at any of them. Each one
repainted the three halo bands sixty times per second for half a second, and
with a key every sixty milliseconds that never stopped.

**A warning about measuring this:** on a virtual display (`Xvfb`) there is no
graphics card, `llvmpipe` draws in software, and during an arrow-key walk
**97 % of the CPU time goes to its threads**. There, any measurement says «the
cost is repainting» and hides everything else. The numbers above come from
comparing with the piece removed, not from a profile; and the absolute number
on a virtual display is useless for deciding anything about a machine with a
GPU.

## Switching folders quickly

Two bugs that looked the same (cells showing the photo of another item) and
only showed up when switching quickly between two folders. Both are written
down because they do not show in a calm screenshot: you need the burst.

**The slots did not notice that their content changed.** The grid distributes
items into slots and `repartir` only notifies the ones that change position or
size. If in the new view the item in a slot had the same shape and landed in
the same place as the one in the previous view, the slot kept the old id. Now,
when the model resets, the content of all of them is refreshed.

**The image provider looked up in one view and decoded from another.** It runs
on the image loading thread: it looked up the id's position in the published
view and then took another reference to decode. If the interface thread
switched views between those two steps, the old view's position was read in
the new one, and since the Qt cache stores the image under the URL of the
requested id, the wrong photo stuck. Now it takes the view once and does
everything with it. And switching views and cloning them happen under the same
lock: before, cloning the old one right as it was being released meant touching
memory that had already been freed.

## Renaming the library and its linked folders

«Rename this library…», in the menu of the name at the top left, changes the
displayed name and nothing else: the `.grimorio` folder stays where and as it
is, because its path lives in the recent list, in shortcuts and in whatever
other programs point to. In `library.json` only `name` is touched; everything
else in the file stays there, even if it comes from a newer version. The
recent list shows that name and not the folder's.

**Linked folders** are folders on disk whose new content goes into the library
automatically. They used to be called «vigilar» («watch»), which in Spanish
sounds like surveillance; internally it is still `vigiladas.json` and the same
commands, so the format does not change. The library menu opens their node view
(`PanelVinculos.qml`): the library on the left, with the folder it lives in;
each linked folder on the right, with where its content goes; a line between
the two along which strokes run towards the library, and which pulses when
something actually comes in. A folder that is not there (an unplugged disk)
appears dimmed, with a dotted, still line, and works again when it comes back.
From there you connect a new one (the «+» node) and disconnect one (its ×);
whatever already came in stays. Paths can be clicked to open them in the file
manager, and the link icon of a linked folder in the tree opens the same view.

## One window per library

Opening it twice on the same folder means two programs writing to
`folders.json` and to the index, with the
second one to save overwriting the first. With autoplay video on, they also
fight over the graphics card's decoder (each video is a pipeline, and two
programs are twice as many): the card runs out of sessions and playback fails
with «No decoder available».

So the second one does not start, and says so:

```
error: «~/Referencias.grimorio» ya está abierta (proceso 2274646)
```

The lock is `abierta.lock` inside the library and it stores the pid of the
process holding it. A hard shutdown does not leave it locked forever: the next
one sees that process is gone and takes it over. `grim` does not go through
this: the command line keeps working with the window open.

## Which library opens

In order: the path passed in (or `GRIMORIO_LIB`), the last one opened, and if
there is none, a new one at `Documentos/Grimorio.grimorio`. A path to a file
inside the library opens that library.

A path that leads to no library (an unmounted disk, a moved folder, an old
`GRIMORIO_LIB`) **does not leave you without a window**: the next one in the
list opens and the status bar says which one was not found. Before, the
program quit, and with «already open» on top of that, because the lock cannot
be created in a folder that does not exist. The automated runs (`--bench`,
`--captura`, `--guion`…) do fail: measuring another library without saying so
would produce numbers that are wrong.

## Search

The field at the top understands filters as well as words, and the «filters»
button writes into that same field:

```
tipografía etiqueta:rótulo estrellas:>=4 peso:>2mb orientacion:vertical
```

Fields: `etiqueta:` (or `#etiqueta`), `tipo:`, `ext:`, `estrellas:`, `ancho:`,
`alto:`, `peso:`, `orientacion:`, `color:#rrggbb`, `orden:`, `papelera:`,
`etiquetado:` (`etiquetado:no` is whatever has no tag at all; it is a separate
field and not `etiqueta:no`, which would search for the tag «no»).
They accept `>=`, `>`, `<=`, `<`; a bare number means «exactly that».
Anything that is not understood is searched as text.

`tipo:` picks only between family and extension: `tipo:video` is all videos
and `tipo:png` is the PNGs. `adulto:` and `+18:` are the same.

The CLI's `grim search` understands this same line: it uses the very same
parser, shared by both.

## Safe mode

An item is marked as adult content from the inspector, or from the CLI with
`grim adult ID`. The mark lives in `item.json`, is undone like everything else
and is searched with `adulto:si` (or `+18:si`).

Marking does not hide anything by itself: what decides is **safe mode**, the
button at the top, which only appears if something is marked or if it is on.
With it:

- in the grid and in the inspector, marked items are blurred with a `+18` on
  top;
- a marked video does not play on hover;
- **the viewer shows it in full**: opening an item means asking to see it.
  Back in the grid it is blurred again. The only thing that lifts it entirely
  is turning the mode off.

The blur is the thumbnail itself requested tiny and stretched to the cell. No
effects module is needed (the Qt in Ubuntu 24.04 ships neither
`QtQuick.Effects` nor the shader compiler) and, more importantly, the real
pixels never reach the screen: what gets decoded is a few dozen pixels, rather
than the image with something covering it.

## New versions

Grimorio tells you about new versions, and updates itself only **if you ask
it to**. Once a day (and eight seconds after startup, so it does not compete
with the first screen of thumbnails) it requests
`https://grimorio.frederickandrade.com/version.json`:

```json
{
  "version": "0.2.0",
  "descargas": "https://grimorio.frederickandrade.com/",
  "paquetes": {
    "linux":   { "url": "https://github.com/Dohkku/grimorio/releases/download/v0.2.0/Grimorio-0.2.0-linux-x64.tar.gz",
                 "sha256": "…" },
    "windows": { "url": "https://github.com/Dohkku/grimorio/releases/download/v0.2.0/Grimorio-0.2.0-windows-x64-instalador.exe",
                 "sha256": "…" }
  }
}
```

If it is newer than its own, the status bar shows a pill in the selection
color, «Grimorio 0.2.0 available · update», with an × that ignores it until
the next one; the gear at the top left gets a dot, and Settings opens with a
card at the very top: «update now», «see the page» and «not now». Before, it
was a notice-sized text in a corner, and nobody saw it. «Notify about new
versions» turns it off completely and «check now» asks right away (and shows
an ignored version again).

**Update now** downloads the package for its system, checks that its SHA-256
matches the one in `version.json` and installs it:

- **Windows**: it launches the installer with `/SILENT /actualizar=1`, in the
  same mode it was installed in (`/ALLUSERS` if it is in Program Files,
  `/CURRENTUSER` otherwise; otherwise there would be two copies) and quits.
  The `.iss` reopens it when it finishes, as the user and not as
  administrator.
- **Linux**: it opens the `.tar.gz`, copies `grimorio` and `grim` alongside as
  `.nuevo`, and swaps each one with a rename; the previous one stays as
  `.anterior`. It reopens with the same library, through an `sh` that waits for
  the old process to release the lock.

Only **published packages** do this (`-DGRIMORIO_PAQUETE=ON`, which
`release.yml` sets), installed somewhere they can be replaced: with the
installer on Windows (not the portable zip) and in a folder owned by the user
on Linux. A development build, the portable version or a root-owned `/opt`
open the download page, as before. `paquetes` is new in 0.1.4: earlier
versions do not look at it and keep opening the page.

- **The website and not the GitHub API.** The file is ours: if the downloads
  move, the link changes and the programs already installed keep working.
  GitHub also rate-limits anonymous requests.
- **It sends nothing.** A GET without cookies and with a bare `Grimorio` user
  agent, without the version. The website's privacy page explains this.
  Downloading the package is another identical GET, and only when you click.
- **Only from the releases, and with a checksum.** A package is valid if it is
  `https`, from `github.com/Dohkku/grimorio/releases/download/` and comes with
  a SHA-256; a download that does not match is deleted without installing, and
  the card says why. What runs has to be exactly what was published, bit for
  bit.
- **The last known result is saved** (`novedades/*` in the settings): the
  notice appears on startup even if that day there is no check due or no
  network.
- **Only an https link.** The notice opens the browser with whatever the file
  says; anything else is replaced with the home page.
- **Automated runs do not check** (`--bench`, `--captura`, `--visor`,
  `--lote`, `--guion`), nor does anything started with `GRIMORIO_SIN_RED=1`,
  such as the test walkthrough: a test cannot depend on the network or show a
  notice in what it captures.
- **To test it**, `GRIMORIO_NOVEDADES_URL` changes where `version.json` is read
  from (a `python3 -m http.server` serving your own is enough).

The comparison and the parsing of the file are in `src/version.h`, without
networking, and the `version` test covers them. How the new number is
published: `docs/WINDOWS.md`.

### What's new

`NOVEDADES.md` (Spanish) and `CHANGELOG.md` (English), at the root, are the
single source of what each version brings: one `## X.Y.Z` section per version,
the same in both. The app shows the one in its language. It is built into the program as a
resource, and the first time a version is opened after updating, the window
shows the sections of the versions that had not been seen
(`novedades/vistos` in the settings); «got it» marks them as seen. Someone
installing for the first time sees nothing: if there were no previous settings
(`bibliotecas/recientes`), there is nothing «new». Someone coming from a
version older than this feature sees only the current one, because there is
no way to know which one they came from. Settings → About → «What's new»
shows all of them, whenever you want. They do not depend on the network; only
automated runs skip them.

Publishing (`release.yml`) copies its version's section as the text of the
GitHub release, and fails early if it does not exist: a version cannot be
published without saying what it brings. The website shows it at
`/novedades/`.

## Measure and review

```sh
./build/grimorio BIBLIOTECA --bench 12       # automated walkthrough and frames
./build/grimorio BIBLIOTECA --visor 24       # how long the viewer takes to show
./build/grimorio BIBLIOTECA --bench 12 --lote 10000   # batch editing with the grid running
./build/grimorio BIBLIOTECA --guion /tmp/x-  # walks through the interface and captures each step
./build/grimorio BIBLIOTECA --captura a.png  # one frame and exit
./build/grimorio BIBLIOTECA --filtro "tipo:video" --captura a.png   # with the search set
```

Script mode batch-tags, autocompletes, filters by a tag, writes a note,
creates a folder, opens the viewer, moves things to the trash and asks before
emptying it, leaving a screenshot of each step. At the end **it cleans up
after itself using undo**: if the stack got the order wrong, the library would
not end up as it was.

The two benchmarks that edit (`--lote` and `grim bench batch`) undo what they
do when they finish, so they do not change the library they are measuring.

## Tests

```sh
ctest --test-dir build --output-on-failure   # geometry, drag, colors, qmllint
cargo test -p grimorio-puente                 # the command bus and the C boundary
```

- `filas`: the grid geometry, without Qt Quick and without a model. It
  includes a time budget: laying out a hundred thousand items again has to fit
  in one frame.
- `version`: when the new version notice appears. «0.10.0» is ahead of
  «0.9.0», a broken file announces nothing and a link that is not https is not
  opened.
- `sin_colores_a_mano`: not a single color or size written by hand in the QML.
  If the style does not come entirely from the theme, the promise that the
  visuals can be changed breaks silently.
- `arrastre`: moves a real mouse over the real tree rows, without a display.
  It is the only thing that catches whether the drag payload reaches the other
  side and whether the bands land where the cursor points: both looked fine in
  a screenshot and did not work.
- `qml_sin_avisos`: clean `qmllint`.
- `qml_tipos_propios`: `qmllint` on the two files that use `Onda` and
  `Visor3D`. Those types are registered from C++ at startup and qmllint cannot
  see them; mixed in with the rest, «Failed to import Grimorio» made the whole
  test bail out and left everything else unchecked.

## Formats

Images and digital negatives are handled in pure Rust: from a RAW, the preview
JPEG the camera stores inside it is extracted, without `libraw`.

In the grid, a sound is shown as its waveform over time: a gray cell that says
«audio» cannot be told apart from the one next to it, and the shape of a sharp
hit, a voice or a song can. If it has cover art (ID3v2 `APIC` or the FLAC
`PICTURE` block, read without dependencies), it goes in a square to the left of
the waveform: the cover of an album is the same for twelve tracks, and the
waveform is not. It is analyzed at 22 kHz: at 8 kHz everything above 4 kHz was
lost and the thumbnail was misleading.

Whatever was imported before the core knew how to draw something keeps its old
thumbnail until a new one is requested, with the application closed:

```sh
grim miniaturas --tipo audio     # or modelo, video… without --tipo, everything
```

It does not touch tags, ratings, notes, folders or the name.

3D models (STL, OBJ, PLY, glTF/GLB and 3MF) are read in pure Rust and their
thumbnail is drawn by the core without a graphics card: a rasterizer with a
depth buffer and supersampling, in clay color, seen from the top right. An STL
with 815,000 triangles comes out in 0.3 s. On import, the triangle count and
the size of the part in millimeters are also stored, which is the first thing
you ask before printing it. `.blend` and `.fbx` are opened by Blender without a
window (`blender -b`), if it is installed: a script dumps the already evaluated
geometry (with modifiers applied) and the core draws it like the others.

That script deliberately skips Blender's glTF exporter: with it, a scene with
438,000 triangles took 26 s and one with 124,000, 41. Dumping with
`foreach_get`, 1.5 and 1.9 s, almost all of it Blender starting up.

The thumbnail stored inside the `.blend` is a fallback and not the first
choice: Blender saves it at 128 px and with the scene lighting, and in a 320
cell it comes out blurry and almost always dark.

Video and PDF need two system tools, and **they are not required**: without
them the files are still imported into the library, with their data, and stay
without a thumbnail until the tools are installed.

```sh
sudo apt install ffmpeg poppler-utils
```

## Playback

The viewer plays video and audio if the Qt multimedia module is present, which
is not required either:

```sh
sudo apt install qml6-module-qtmultimedia
```

Without it, a video in the viewer stays on its cover frame and offers to open
it with the system player. The `import` lives in `Reproductor.qml`, on its
own: an import that fails takes down the file that contains it, and if it were
in `Visor.qml` there would be no viewer for photos either.

Video also plays outside the viewer:

| Where | When |
|---|---|
| in the grid cell | on hover, and only that one |
| in the inspector | whenever a video is selected and the viewer is closed |
| in the viewer | when it opens |

In the grid it loops without sound (a grid that makes noise by itself is
impossible to look at); in the inspector and in the viewer, with sound and
controls.

**One video, rather than all the visible ones.** There was a mode that played
every visible video with a cap of eight, and it was removed after measuring on
the development machine (NVIDIA, GPU decoding, 61 items with 5 videos):

| | memory | CPU |
|---|---|---|
| five videos playing by themselves | 0.88 GB | 33 % of one core |
| idle, no mouse over anything | 0.28 GB | 0 % |
| with the mouse over a video | 0.41 GB | 7 % |

Each playing video is a whole decoding pipeline and, on a machine with a GPU
decoder, a graphics card context. Eight at once was less a cap than a promise
that you would hear the fan.

The button at the top has two states: `vídeo: al pasar` (video: on hover) and
`vídeo: quieto` (video: still).

A few things in the player are not decoration and should not be undone:

- **`play()` is requested twice, and the second time it is checked instead of
  remembered.** On media that has not loaded yet it does nothing and leaves no
  warning, and nobody tries again: the video in the inspector stayed on its
  cover with the play triangle showing, waiting for a click that should not
  have been needed. It is requested again when the media reports it is ready.
  Remembering «there was one pending» is not enough: changing the source
  triggers two things (the handler of whoever sets it and the media reload)
  and the order between them is not guaranteed, so the old media used up the
  flag and the new one stayed stopped forever. In the grid there is no pause
  button, so there it checks directly: if it has a slot and a source, it has
  to be playing. In the viewer and the inspector there is one, and whatever
  was requested by hand is respected.
- **It is created once and never destroyed.** Setting up and tearing down the
  multimedia pipeline many times in a row crashes the program («double free
  or corruption» inside GStreamer; with `--visor 8` it shows up on the first
  try). A single player whose source is changed avoids that.
- **It is created 120 ms late.** Starting it in the same frame the viewer
  opens costs the whole frame: opening a video went from 6 ms to 691 ms, and
  what you see in the meantime is nothing. Measured with `--visor`.
- **In the grid they sit above the cells, not inside them.** Inside, one would
  have to be created when the mouse enters a cell and destroyed when it leaves,
  which is exactly what crashes the program. Above, there are a few whose
  position and source are changed. And the assignment is deferred while the
  grid is scrolling: reassigning them every frame only tears down half-built
  pipelines.

## What is not a photo, in the viewer

Each family has its own way of being viewed, and the viewer picks it by
itself.

What is needed to show it is extracted the first time it is opened and stored
in `cache/` inside the library: the playable copy of a video, the waveform of a
sound, the pages of a PDF, the mesh of a model. These are derived files: the
whole folder can be deleted and is rebuilt. They are requested from a thread
pool of their own (`Derivados`, `src/derivados.h`) and **not** through the core
bus: the bus has one thread, the editing one, and converting a ten-minute video
cannot keep a rating change waiting.

### Video the player cannot open

ProRes, DNxHD and friends: ffmpeg reads them, but the GStreamer under Qt only
decodes them with `gstreamer1.0-libav`, which Ubuntu does not install. Without
it, the viewer said «Internal data stream error» and stayed on the cover.

Now, when it fails, the player asks the core for an 8-bit H.264 copy (the only
thing every decoder understands, the graphics card's included), says it is
preparing it and switches to it when it arrives. The first time it takes as
long as ffmpeg takes (a few seconds for a short clip); after that it is in the
cache. It works the same in the viewer, the inspector and the grid.

### The controls

At the bottom, the seek bar and a row of buttons: −5 s, one frame back, pause,
one frame forward, +5 s, the time with tenths, the speed, the A–B range, loop
and volume. Hovering over the bar shows the exact time and, for a video, the
frame at that point: it comes from a filmstrip of up to a hundred frames that
ffmpeg makes once, and showing one means cropping the image, so nothing is
decoded while the mouse moves.

Speed: click goes up one step, right click or wheel down goes down one, middle
click goes back to 1×. In slow motion the sound drops in pitch: Qt 6.4 does not
correct pitch when the rate changes.

The A–B loop is watched by a 15 ms timer and not by the position notification:
the position reports whenever it feels like it, and a half-second loop that
overshoots by a hundred milliseconds is no loop at all.

In the inspector the controls are the bare minimum (pause, time and bar): the
whole row does not fit in 300 px.

### Audio

The waveform over time at the top and the frequency spectrum at the bottom,
with the same range and the same playhead. Click or drag to go to a point; the
wheel zooms around the mouse and with Shift it scrolls; double click shows the
whole thing. Hovering shows the time and, over the spectrum, the frequency.
When zoomed in and playing, the view follows the playhead.

The analysis belongs to the core (`grimorio-core/src/senal.rs`): ffmpeg decodes
to mono `f32` at 44.1 kHz and it is processed **in chunks** (an hour of audio
is six hundred megabytes in `f32`, and there is no need to keep it in memory).
Peaks every 256 samples and a spectrogram of up to 4096 columns by 256 rows on
a logarithmic scale, from 30 Hz to 22 kHz, with its own 2048-point FFT. The
spectrum colors come from the theme: from the background to the selection
color and from there to the text color.

A trap that took a while to find: the player's `MouseArea` that pauses and
resumes on click is declared after the waveform, so it ended up on top and
swallowed the clicks. Picking a point paused playback. With the waveform in
front, that `MouseArea` is disabled.

### PDF

The whole document, page after page. Before drawing anything, the sizes of all
pages are requested (`pdfinfo`), so it has its real length from the start and
the scrollbar does not jump. Each page is drawn by `pdftoppm` at the width it
takes on screen, rounded to steps of 256 px: that way zooming in a little does
not redraw it, and the cache does not fill up with nearly identical variants.
Ctrl + wheel or the buttons zoom in.

### 3D models

Dragging rotates, right or middle button pans, the wheel zooms and double click
returns to the starting view, which is the same as the thumbnail's: opening is
not a jump. Buttons for front, top, side and iso, wireframe, and open with the
system program. At the top left, the dimensions and the triangle count.

The viewer is hand-written OpenGL in a `QQuickFramebufferObject`
(`src/visor3d.h`) and **not Qt Quick 3D**: Qt Quick 3D is licensed under GPL or
commercial terms, and Grimorio started out as a closed-source binary. For a
mesh without textures, a vertex buffer and two shaders are enough, and those
are in Qt Gui, which is LGPL. That is why `main.cpp` pins the scene to OpenGL.

The mesh arrives from the core centered, at unit scale and without normals: the
shader derives them from the slope of the position (`dFdx`/`dFdy`), which gives
flat faces (what you want to see in a part for printing) and halves the file
size. The projection is orthographic: in a part, parallel lines should stay
parallel. It is framed by its bounding sphere and not by its box, which, when
rotated, poked out at the corners and cut off a tip.

The wireframe is drawn at half intensity instead of opaque: on a print mesh of
a million triangles, opaque covered the whole part with a smear of color.

## Modules needed on Ubuntu 24.04

Debian does not declare them as dependencies of `qml6-module-qtquick`, so
without them the binary builds but `import QtQuick` fails at startup:

```sh
sudo apt install qt6-base-dev qt6-declarative-dev cmake \
                 qml6-module-qtqml qml6-module-qtqml-workerscript \
                 qml6-module-qtqml-models qml6-module-qtquick \
                 qml6-module-qtquick-window qml6-module-qtquick-templates \
                 qml6-module-qtquick-controls qml6-module-qtquick-layouts \
                 qml6-module-qtquick-shapes
```
