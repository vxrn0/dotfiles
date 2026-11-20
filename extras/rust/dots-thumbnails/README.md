# dots-thumbnails

Phases 4 and 5a of the Rust backend. One binary replaces both thumbnail scripts
in `.config/quickshell/dotfiles/scripts/thumbnails`:

- `thumbgen.py`, the `GnomeDesktop.DesktopThumbnailFactory` wrapper called first
  by `services/Wallpapers.qml`;
- `generate-thumbnails-magick.sh`, the ImageMagick fallback called only when the
  Python wrapper exits non-zero.

## What the two scripts actually are

They are not a split by input type and neither is a complete implementation of
the other. They overlap: both produce `md5(file URI).png` under
`$XDG_CACHE_HOME/thumbnails/<size>`, and both had the same job.

`thumbgen.py` is the intended main path. It uses GNOME's factory, so it runs the
thumbnailers installed in `$XDG_DATA_HOME/thumbnailers` and
`$XDG_DATA_DIRS/thumbnailers` inside bubblewrap, checks `Thumb::URI` and
`Thumb::MTime`, honours GNOME's failed-thumbnail records under
`thumbnails/fail/gnome-thumbnail-factory` (the freedesktop `.png.failed`
  files are not used by this GNOME version), and supports all four sizes.

On this machine the Python script cannot run at all: it imports `loguru`, which
is not installed (the venv is only a symlink to system Python), and Python 3.14
defaults multiprocessing to `forkserver`, which loses the module-global factory.
So the magick script was the only one producing thumbnails. The fallback is
weaker: GIF and video suffixes are skipped, existing output is trusted with no
mtime check, the URI encoder is a bash loop (non-ASCII is encoded as codepoints,
not UTF-8), no `Thumb::*` chunks are written, and `$HOME` is used instead of
`$XDG_CACHE_HOME`.

One binary can serve both call sites, so this crate does. It keeps both CLI
dialects because the QML files are out of scope and still call the old paths.
The primary call passes `--machine_progress`, the fallback does not; that is how
the two dialects are told apart. A bare `-d DIR` without `--machine_progress`
is treated as the fallback, which is the only ambiguous form. With no mode flag
at all it is treated as the GNOME dialect, so a missing `-d` fails like click.

## GNOME dialect

`--size`, `--img_dirs` (split on whitespace, exactly like the Python),
`--workers`, `--only_images`, `--recursive`, `--machine_progress`, all short
forms. File order is directory order, progress lines are the same
`PROGRESS n/total FILE path`, and the final line is
`Thumbnail Generation Completed!`.

The factory follows the reference:

- URI: Gio's `g_filename_to_uri` escaping (unreserved plus
  `!$&'()*+,=:@/`; semicolon is escaped).
- mtime: whole seconds, as PyGObject converts the float.
- lookup: a PNG is fresh only when it decodes and both `Thumb::URI` and
  `Thumb::MTime` match.
- can_thumbnail: MIME must have a registered thumbnailer, and a valid record in
  `thumbnails/fail/gnome-thumbnail-factory` suppresses it. Like the Python, this
  tool never writes a failed record itself.
- generation: the same thumbnailer command with `%u`, `%i`, `%o`, `%s`, `%m`
  expansion, inside `bwrap` with `/usr` and `/etc` read-only, a private `/tmp`,
  `/proc`, `/dev`, no network and no home directory. If `bwrap` is absent the
  thumbnailer runs directly, matching a GNOME build compiled without bubblewrap.
- save: image data is passed through untouched; all text chunks are replaced by
  `Software=GNOME::ThumbnailFactory`, `Thumb::MTime` and `Thumb::URI`, preserving
  `Thumb::Image::Width/Height` when the thumbnailer supplied them. The file is
  `0600`, the size directory `0700`, and the rename is atomic.

MIME is sniffed from the first bytes for the formats wallpapers use (PNG, JPEG,
GIF, WebP, TIFF, BMP, ICO, SVG, AVIF/HEIF, JXL, MP4, WebM), with an extension
fallback. This replaces Gio's shared-mime-info lookup.

## Magick dialect

`--file/-f` and `--directory/-d`, `--size/-s`. The parser's quirks are kept:
parsing stops as soon as a mode is seen, so `--file a.png --size large` still
uses `normal`; an unknown size or missing size value means 128; a missing file
or directory prints the old line and exits 2; a usage error exits 1. The cache
is `$HOME/.cache/thumbnails/<size>`, not the XDG cache, as in the shell script.
GIF/MP4/WebM/MKV/AVI/MOV are skipped case-insensitively, an existing output file
is never refreshed, and ImageMagick is invoked with exactly
`magick <abs> -resize <n>x<n> <out>`.

## Deliberate differences

- One binary instead of two scripts. The old files are untouched; callers must
  point both properties at this binary (outside this zone).
- The Python wrapper's dead imports are not reintroduced. The comparison harness
  supplies a no-op `loguru` module and forces the fork start method so the
  reference can run; the production binary has neither workaround.
- `--workers` is accepted and ignored. Thumbnails are generated sequentially;
  output order is the same as Python's ordered `imap`. The `tqdm` bar is also
  omitted; `--machine_progress` output is identical.
- The bash fallback's non-ASCII URI bug is preserved, not fixed, because the
  cache names would otherwise differ.
- A missing `magick` is not swallowed; it prints an error. Thumbnailer stderr is
  still captured and dropped, like GNOME's factory.
- The GNOME setting `org.gnome.desktop.thumbnailers disable` is not read.
  Thumbnailers are always considered enabled.

## Build and test

```sh
cargo build --release --offline
cargo test --offline
./tests/compare.sh
```

`tests/compare.sh` creates a temporary tree with PNG, JPEG, transparent PNG,
GIF, SVG, a non-ASCII name, text and a fake video, redirects `HOME` and every
`XDG_*`, and runs the original scripts and the binary on the same inputs. It
compares cache file names, decoded pixels (max channel difference <= 2, mean
<= 0.05; measured 0/0), `Thumb::URI`, `Thumb::MTime`, `Software`, stdout, exit
codes, and the logging `magick` wrapper's argument log.

Measured on 2025-11-21: 15 cases, 15 matched, 0 failed. The GNOME cases use the
real installed thumbnailers, so the generated pixels are byte-identical to the
reference, not merely within tolerance. The fallback cases use the same
ImageMagick through a logging wrapper; only ImageMagick's `tIME` second differs.

Unverified: live QML callers (out of this zone), a desktop with
`disable-all=true`, and distributions whose thumbnailers are not under
`/usr`/`/etc` and therefore need extra bubblewrap binds.
