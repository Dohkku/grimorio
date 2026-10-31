English · [Español](LEEME.es.md)

# The Grimorio browser extension

Saves images to your library without downloading them first:

- **Right click → «Save to Grimorio»**, on any website: on an image or a
  video it saves that file; on a link or on the page, the image that
  represents it (`og:image`). On Instagram it saves whatever is under the
  mouse, using the same path as the button, even when the photo is covered
  by layers.
- **On Instagram:** hovering over a photo or a video shows a «to Grimorio»
  button, also in a profile's grid. It takes the **full** original: what you
  see on screen is usually a thumbnail cropped by the server itself (square in
  the grid), so the photo is requested from the same API the Instagram
  website uses, with your session. In a carousel, «all» takes all of them,
  even the ones that have not loaded yet. If the API does not answer, it saves
  the largest version of that photo the browser has already downloaded (the
  same thing you would look for by hand in the «Img» tab of the developer
  tools), and if there is none, what is on screen. The notice says what was
  saved and how: «saved · 1080×1350 · original», or «already downloaded (the
  API answered 403)». The source that gets recorded is the link to the post,
  which does not expire; the image address does.
- **On TikTok:** the same «to Grimorio» button over each video, in the feed
  and in the grids, and right click saves whatever is under the mouse. The
  video plays from a `blob:`, which cannot be downloaded, so the extension
  requests the post page, which contains the location of the full video: it
  saves the largest version (often 1080p in H.265), without a watermark. From
  a photo carousel, the one on screen. If the page does not include the data
  (it asks you to log in, or the post was deleted), a photo is saved as it
  looks and a video is not saved.
- **On 4chan:** hovering over a thumbnail shows «to Grimorio», and right
  click saves the same thing: the full file (image, gif or webm) instead of
  the «…s.jpg» thumbnail you see, with its original name (also when it is a
  spoiler) and the link to the post («/po/thread/123#p456»). The catalog only
  has thumbnails, so that is what gets saved, with the thread subject.
- **Where:** the extension button shows whether Grimorio is open and lets you
  choose the folder everything goes into.


## Install

1. **Connect Grimorio to the browser**, once: Settings → browser →
   «connect browsers», or in a terminal `grim navegador instalar`.
   It registers `grim` as a native program in Brave, Chrome, Chromium, Edge,
   Vivaldi and Firefox, whichever are installed.
2. **Load the extension**: in `brave://extensions` (or `chrome://extensions`)
   turn on developer mode, click «Load unpacked» and pick this folder
   (Settings → browser → «open the extension folder» opens it).

Its id is always `nhmdhkmjpnpclpiopkajinahdcmacfko`: it comes from the public
key in `manifest.json`, and it is the one `grim` allows to talk to Grimorio.

## How it works

```
extension ──native messaging──▶ grim ──local socket──▶ running Grimorio
```

- The extension cannot talk to other programs; the browser only lets it talk
  to one registered for it, which the browser launches itself. That program
  is `grim` (see `crates/grimorio-cli/src/nativo.rs`).
- `grim` passes each message to the application through a local socket in
  `$XDG_RUNTIME_DIR` that only your user can open (see `app/src/puerta.h`).
  It is deliberately not a port: any website you visit could call a server
  on `localhost`.
- The extension downloads the image, with your session, and sends it whole;
  if it is larger than 40 MB it sends the address and Grimorio downloads it.
- If Grimorio is closed, saving something opens it. Checking whether it is
  running or asking for its folders opens nothing.

None of this leaves your computer: the photo goes from the browser to your
library.

## Tests

```sh
cd navegador && npm install && npm test
```

They test `elegir.js` (which image is under the mouse going through the
layers, which is the largest size, the post link, the name) against HTML
shaped like Instagram's, without its classes, just like the code does;
`tiktok-datos.js` (which post each video belongs to, which version to
download) with the shape of what TikTok serves; and `4chan-datos.js` (the
original behind each thumbnail, its name and its post) with the HTML of a
thread.
