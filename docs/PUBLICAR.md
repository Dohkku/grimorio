English · [Español](PUBLICAR.es.md)

# Publishing a version

The whole list, in order. Building and packaging is done by GitHub Actions (see
«Publishing a version» in [WINDOWS.md](WINDOWS.md)); this is what is left to do
by hand before and after.

## Before: the documentation

Documentation falls behind by itself if nobody checks. Before each version:

1. **`CHANGELOG.md` and `NOVEDADES.md`**: a `## X.Y.Z` section at the very
   top of each, the same text in English and in Spanish, with what the people
   using it will notice (leave out how it is built inside). It is what the app
   shows after updating, in its language, and the text of the release (English
   first, Spanish folded below). Short and plain.
2. **`README.md`**: if there is something new that can be seen, one line in
   «What it does» (and the same in `README.es.md`, «Qué hace»).
3. **Translations**: if the interface gained or changed texts, update
   `app/traducciones/` (see its `LEEME.md`).
4. **`docs/FORMATO.md`**: if anything about what is stored on disk changes (a
   field in `item.json`, a new file in the library, the cache).
5. **The number**: `version` in `Cargo.toml` and `project(... VERSION ...)` in
   `app/CMakeLists.txt`. CI writes it again from the tag, but locally it has to
   say the same thing.

The tests watch what can be watched (`crates/grimorio-core/tests/documentacion.rs`):
they fail if `item.json` has a field that `FORMATO.md` does not describe, if
the library writes a file that is not documented or if the version in
`Cargo.toml` has no section in `NOVEDADES.md` or in `CHANGELOG.md`. The rest (that the README
mentions what is new, that the release notes are easy to understand) is this
list.

## Publishing

1. `cargo test --release` and `ctest` in `app/build`: everything green.
2. The public commit is one per version, «Grimorio X.Y.Z», on top of the
   previous one on `main`.
3. Tag `vX.Y.Z` and push: CI builds Linux and Windows, runs the tests on both
   and creates the release with the sections from `CHANGELOG.md` and
   `NOVEDADES.md`.
4. If CI fails before creating the release: fix it with a commit «Grimorio
   X.Y.Z: whatever it is» on top and move the tag. With no release created
   there is nothing to break.

The `modelo-isnet-1` release has to exist: CI downloads the background removal
model from there.

## After: telling people

Only once the release files can be downloaded: on the website,
`public/version.json` (version, URL and sha256 for Linux and Windows) and the
`/descargar/` URLs in `vercel.json`, then deploy. Installed copies of Grimorio
check it once a day and show a notice in the bottom bar.
