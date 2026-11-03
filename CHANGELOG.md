# What's new in Grimorio

What changes in each version, from newest to oldest.

This file is the English version of `NOVEDADES.md`, with the same sections.
Grimorio ships it inside and shows it when you open a new version after
updating (and in Settings → about → «what's new»). Each version is a
`## X.Y.Z` section; what goes under it is plain Markdown.

## 0.1.20

- **Grimorio no longer freezes when left open.** When the system switched
  sound output (for example, Bluetooth headphones that turn themselves off
  after a while) with a video open or paused, the window froze and had to be
  force closed. That no longer happens.
- **A video that is already playing always switches output.** If headphones
  turned off and back on while a video was open, that video kept playing
  through them even after you chose the speakers. Now it moves to the output
  you choose.

## 0.1.19

- **Sound follows the system output.** If you switch from speakers to
  headphones (or back) while Grimorio is open, videos play through the new
  output, including the ones you open afterwards. Before, they stayed on
  whichever output was active when Grimorio started. The same goes for
  connecting or removing Bluetooth headphones.

## 0.1.18

- **Settings, redone.** The settings panel (Ctrl+,) is no longer one long
  list: it has five pages (general, grid, stacks and editor, updates and
  about), each with its title and a line saying what it is for. Settings
  are grouped in boxes, switches respond on the whole row, and pick-one
  options (theme, language, stacks, snapping) are a selector with a marker
  that slides to the chosen one. In the paper theme nothing feels cramped
  anymore.
- **Scrolling inside settings** no longer moves the grid behind it when you
  reach the end of a page.

## 0.1.17

- **Grimorio in English.** In Settings → appearance → language you choose
  español, English or the system default (if your system is in Spanish, it
  stays in Spanish). It applies the next time you open Grimorio. It covers
  everything: menus, notices, what Ctrl+Z undoes, the what's new panel and
  the browser extension, which follows the browser's language.
- **The search box understands both languages.** `type:video` and
  `tipo:vídeo`, `stars:>=4` and `estrellas:>=4`, `date:today` and
  `fecha:hoy`. Filter buttons write in the interface language, and a smart
  folder saved in one language keeps working in the other. Batch renaming
  also accepts `{name}` and `{date}`.
- **Video colors, right when playing.** On Linux, pressing play changed the
  colors of some videos: reds turned orange and skin looked yellowish, even
  though the thumbnail looked fine. It was a bug in the Qt that ships with
  Ubuntu, with HD videos (almost all of them: OBS, YouTube, phones). Now
  they play with the same colors as the thumbnail.
- **Shorter buttons in the inspector**, so they are no longer cut off.

## 0.1.16

- **More room to look.** In the viewer, the photo or video is no longer
  covered by the playback bar or by the versions in its stack. And with H, or
  the eye button, the whole interface disappears and what you are looking at
  fills the entire window without going full screen. The interface comes back
  when you move the mouse and hides again when you leave it still. H or Esc
  bring it back.
- **Buttons in the viewer.** At the top right you have edit, remove background
  (on photos), save frame (on videos), item only and full screen.
- **Right click in the viewer.** It opens the menu for what you are looking at
  instead of closing the viewer. Esc and double click still close it.
- **Delete removes the version you are viewing.** With a stack open in the
  viewer, Delete sends the version you see large to the trash, leaving the
  rest of the stack alone. Ctrl+Z brings it back.
- **Save a frame.** In a video, «save frame» (or S) saves what is on screen as
  an image in your library, in the same folders and with the same tags as the
  video.
- **A more comfortable video editor.** It opens with the whole video
  selected. The handles at the ends are bigger and show their time, and the
  section can be dragged as a whole without changing its length. Snapping
  sticks the start and end to round times, every half second (you can change
  it in Settings, «editor»). The cut is saved as a version or as a new video,
  and before saving you can see whether it will be instant or needs
  re-encoding.
- **Dragging a video's scrub bar** pauses it while you drag and starts it
  again when you let go.
- **Fixed:** when the editor was opened a second time with the same video, the
  section got stuck on a single frame, looping.

## 0.1.15

- **Fixed:** if you changed your computer's sound output (from headphones to
  speakers, for example) with Grimorio open, videos and sounds kept playing
  through the old one. Now they follow whichever one you choose.

## 0.1.14

- **Long videos without stutters.** When you opened a long video, preparing
  the scrub bar preview kept the whole computer busy for a few seconds (longer
  the longer the video), right while you were clicking on the bar. Now it is
  about thirty times lighter. And clicking or dragging the bar quickly no
  longer piles up jumps.
- **Fixed:** after opening something with Enter and closing it with Esc, the
  arrow keys stopped moving around the grid.

## 0.1.13

- **Trim videos and sounds.** With a video or a sound selected, press E (or
  right click, «open in editor»): mark the in and out points over the frames
  or the waveform, with I and O, and save. The section is saved as a version
  in its stack, without touching the original, and instantly: there is no
  re-encoding, so it takes the same time with an hour-long video. «Exact»
  re-encodes to cut on the precise frame.
- **Leave things out in the search bar.** A minus sign in front removes
  whatever matches it: `#food -#spicy`, `-tipo:video`, `-draft`. And a bar
  accepts either of two: `#food|dessert`.
- **Smart folders you can edit.** The + creates the folder right away, even
  if it is empty, and whatever you filter while it is open becomes its
  filters. «Save as folder» names it after its filters («food · not spicy»);
  double click to change it.
- **Suggested tags.** Below the tags of the selected item you get
  suggestions: from its name, from images that look like it, from tags that
  usually go with the ones it already has and from its folder. Also new ones
  (in italics), taken from the file name or from the folder it came from. One
  click and it is added.
- **3D models with their textures.** GLB, glTF, .blend and .fbx show their
  colors and textures in the grid and in the viewer. The ones you already had
  get updated with `grim miniaturas` (with Grimorio closed).
- **Touch-ups in the right click menu.** Rotate, create version, remove
  background and the editor, all together.
- **Fixed:** when you picked a tag in the search bar and clicked the cross,
  the filter was removed but the text stayed there.

## 0.1.12

- **A video's versions, in sight.** In the viewer, with a video that has
  stacked versions, the row for choosing a version was hidden by the video
  itself. Now it always sits on top, and so does the rating, in full screen
  too.

## 0.1.11

- **Two libraries at once.** In the library menu, each one has a button to
  open it in a new window (or Ctrl + click), and there is «open another in a
  new window…». The one you had open stays.
- **Copy from one library to another by dragging.** Drag from the grid of one
  window to the grid of another library: it is copied with its name, tags,
  rating, note, source and rotation. The original library keeps everything.
- **Scrolling with the wheel, without sliding.** Each notch moves the same
  amount and the grid stops where it should; with a touchpad it follows your
  finger.
- **Duplicate videos in a stack.** The viewer shows their versions for videos
  too, and plays the one you choose. V goes to the next version and Shift + V
  to the previous one, with photos too.
- **Sort by «oldest»**, with the first things that came in at the top.
- **Fixed:** in linked folders, what you sent to the trash could come back in
  on its own. And if an import from a linked folder failed, that folder
  stopped bringing things in until a restart.

## 0.1.10

- **Rotate photos and videos.** In the viewer, R rotates right and Shift + R
  left, or use their buttons. It stays rotated in the grid, the inspector and
  the viewer, and Ctrl + Z undoes it. The file is not touched.
- **Full screen video.** F, double click or its button. When the mouse is
  still, the controls move out of the way and only the video remains.
- **Zoom into a video without stopping it.** The wheel zooms in until you can
  see the pixels, as with photos; Z resets it. Jumps with Shift + arrow are now
  10 seconds.
- **Interface size from the keyboard.** Ctrl + «+» and Ctrl + «−», like in a
  browser; Ctrl + 0 sets it back to 100 %. Thumbnail size moves to Ctrl +
  wheel.
- **The video in the inspector, muted.** It has its own sound button and a
  setting to always play with sound. Space pauses and resumes it, and double
  click opens it large.
- **Buttons show their shortcut** when you hover over them.
- **Dragging an image into another app** works even when Grimorio fills the
  whole screen.
- **Fixed:** with the viewer open, Delete left the viewer empty and the arrow
  keys went back to the start. Now it moves on to the next one.

## 0.1.9

- **Reorder with Shift.** To move an image to a different spot in the library,
  drag it while holding Shift. A plain drag no longer moves the grid, so
  dragging an image out to another app is easier. Shift + click still selects
  a range.

## 0.1.8

- **Fixed:** a video could play twice at the same time. It happened when you
  selected it and opened it large with a double click: it kept playing
  underneath too.

## 0.1.7

- **Delete forever.** In the trash, Delete or the button removes what you
  choose from the disk, after asking. Before, they put it back in the library.
- **Restore.** In the trash, the button on the card restores the image.
- **Enter confirms** questions, such as «empty trash».
- **Fixed:** importing right after another import could say «another process
  is writing» when there was none.

## 0.1.6

- **Versions of an image.** The same image at several file sizes is stacked,
  and the grid shows just one. When you open it you choose which one to see.
- **Lighter versions.** Right click, «create version…». In JPEG, PNG or WebP,
  and you see how much it will weigh before creating it. The original is not
  touched.
- **Remove background.** Right click, «remove background». It runs on your
  computer and is kept as another version of the image.
- **Duplicate images.** If you import one you already had at another size,
  Grimorio offers to stack them.
- **Updating on Linux brings everything.** If you are coming from 0.1.5,
  Grimorio finishes installing itself the first time you open it.

## 0.1.5

- **Linked folders, as nodes.** The library menu opens a new view: your
  library on the left, with the disk folder where it lives, and each linked
  folder on the right, joined by a line along which whatever comes in flows.
  When something new arrives, its line pulses. From there you connect a new
  folder (the «+» node) or disconnect one (its ×); what already came in
  stays. A folder that is not there, such as one on a disconnected drive,
  looks dimmed and starts working again by itself when it comes back.
- **«Link» instead of «watch».** It does the same as before (whatever appears
  in that folder comes into the library by itself), with a name that says
  better what it does. In the tree, a linked folder has a chain link icon;
  clicking it opens the node view.
- **Rename the library.** «Rename this library…» in the menu under its name.
  It changes the name you see and leaves the folder on disk as it is, so
  nothing that points to it breaks.
- **What's new after updating.** When you open a new version for the first
  time, Grimorio shows exactly this. To see it again: Settings → about →
  «what's new».
- **A clearer portable version for Windows.** When you open the zip you only
  see `Grimorio.exe`, a `LEEME.txt` and the `programa` folder, instead of two
  hundred loose files.

## 0.1.4

- **Update from inside the app.** «Update now» downloads the new version,
  checks that it is exactly the published one and installs it; Grimorio
  closes and opens again by itself, with your library. On Windows with the
  installer, and on Linux if the program folder belongs to you; otherwise it
  opens the downloads page.
- **A new version notice you can actually see.** A colored pill in the bottom
  bar, a dot on the gear and a card at the very top of Settings.
- **Settings, at the top left.** The gear is now next to the library name,
  and a bit bigger.

## 0.1.3

- **A missing library no longer leaves you without a window.** If the library
  being opened cannot be found (a drive that is not mounted, a folder that was
  moved), Grimorio opens the last one you used, or creates one in Documents,
  and tells you which one it could not find. Before, it closed without saying
  anything.

## 0.1.2

- Settings in the «about» panel.

## 0.1.1

- **New version notice.** Once a day Grimorio asks its website which is the
  latest version, without sending anything of yours, and tells you if there
  is a new one. You can turn it off in Settings.

## 0.1.0

- The first published version: a visual library for Linux and Windows with
  images, video, sound, PDF and 3D models in a folder of your own; tags,
  folders, smart folders, search by text, color and date, and a viewer for
  each kind of file.
