# 11 — Image codec libraries: zlib, libpng, libjpeg, giflib, libexif

Stand 2026-10-06, TASK/LDG-M2. Why these five: zView's codecs
(`Applications/z-tools/zview/plugins/<fmt>/Makefile`) and
`docs/analysis/zview-ldg-2026-10-04.md` §1–2. Evidence for every "verified"
below: `measurements/measurement-2026-10-06-ldg-m2/RUNINFO.TXT`.

All five are **upstream release tarballs, unmodified**, pinned by SHA-256,
built into static AArch64 archives for NewTOS programs and `.NLD` modules
(PIE: compiled `-fPIE`, so they link into a `-pie -z text` image with
`R_AARCH64_RELATIVE` relocations only). What a target needs that `configure`
would normally write is a config header in the library's overlay directory, each
answer commented; nothing else is changed.

## 1. The libraries

| Library | Version | Pinned tarball (origin, hashes, licence in `<lib>-sources.sha256`) | Archive | Build script | Config |
|---|---|---|---|---|---|
| zlib | 1.3.2 | `sdk/zlib/zlib-1.3.2.tar.xz` (+ `.asc`, signature checked) | `sdk/zlib/libz.a` | `sdk/zlib/build-zlib.sh` | `zconf.h` generated from `zconf.h.in` with configure's two seds (unistd/stdarg) |
| libpng | 1.6.59 | `sdk/libpng/libpng-1.6.59.tar.xz` | `sdk/libpng/libpng.a` | `sdk/libpng/build-libpng.sh` | upstream `sdk/libpng/libpng-1.6.59/scripts/pnglibconf.h.prebuilt`; AArch64 NEON filters on (upstream default) |
| libjpeg (IJG) | 9f | `sdk/libjpeg/jpegsrc.v9f.tar.gz` | `sdk/libjpeg/libjpeg.a` | `sdk/libjpeg/build-libjpeg.sh` | `sdk/libjpeg/overlay/jconfig.h`; memory manager `jmemnobs` |
| giflib | 6.1.3 | `sdk/giflib/giflib-6.1.3.tar.gz` | `sdk/giflib/libgif.a` | `sdk/giflib/build-giflib.sh` | none (bundled `openbsd-reallocarray.c`) |
| libexif | 0.6.26 | `sdk/libexif/libexif-0.6.26.tar.xz` (+ `.asc`, signature checked) | `sdk/libexif/libexif.a` | `sdk/libexif/build-libexif.sh` | `sdk/libexif/overlay/config.h` (no NLS, `localtime_r`) |

Version choice: the current upstream release of each, checked against what
the codecs call. `prerequisites.txt` names only zlib 1.2.8; giflib must be
≥ 5.1 (two-argument `DGifOpenFileName`/`DGifCloseFile`, `sdk/zview/z-tools-04805b7/zview/plugins/gif/gifldg.c:104,345`) —
6.x changed only `EGifSpew()`, which no codec calls; libpng 1.6, IJG and
libexif are used through their long-stable classic APIs.

Each build script reads the library's **own** source list (zlib `Makefile.in`
OBJZ/OBJG, libpng `Makefile.am` incl. the `PNG_ARM_NEON` block, IJG
`Makefile.am` LIBSOURCES, giflib `Makefile` SOURCES, libexif `Makefile.am` +
the four maker-note `Makefile-files`), so a version bump cannot silently drop
or add a file. Any file that does not compile stops the build — there is no
expected-failure list for these libraries. The shared steps (verify +
extract, overlay, compile, archive) are in `sdk/vendor-lib.sh`.

Rebuild rules (`sdk/lib-stale.sh`, `lib_ensure <name>`): each archive is
rebuilt when its overlay, build script, pin file, tarball or
`sdk/vendor-lib.sh` — or anything libc watches — is newer. Chains:
`zlib`: libc → zlib; `libpng`: libc → zlib → libm → libpng (png.h needs
zlib.h, pngpriv.h needs OpenLibm's `math.h`); `libexif`: libc → libm →
libexif; `libjpeg`, `giflib`: libc → lib. `<LIB>_REFRESH=1` (e.g.
`LIBPNG_REFRESH=1`) forces a re-extract; a newer tarball does so on its own.
Two consecutive builds produce byte-identical archives.

Known upstream warning (left visible): libexif `exif-entry.c:1243/1267/1280`
`-Wswitch` on the GPS tag `#define`s. All other libraries build warning-free
with their own warning flags.

## 2. Linking a program against them

The worked example is `sdk/examples/libtest/build-libtest.sh`. In short:

```sh
# headers (extracted trees; run the build scripts or lib_ensure first)
-I sdk/zlib/zlib-1.3.2        # zlib.h zconf.h
-I sdk/libpng/libpng-1.6.59   # png.h pngconf.h pnglibconf.h
-I sdk/libjpeg/jpeg-9f        # jpeglib.h jconfig.h jmorecfg.h jerror.h
-I sdk/giflib/giflib-6.1.3    # gif_lib.h
-I sdk/libexif/libexif-0.6.26 # #include <libexif/exif-data.h>
-I sdk/libm/include -I sdk/libm/openlibm-0.8.8/include   # math.h, if the program uses it
-I sdk/libc/mintlib/include

# archives, one group, before libm/libc
--start-group sdk/libpng/libpng.a sdk/zlib/libz.a sdk/libjpeg/libjpeg.a \
              sdk/giflib/libgif.a sdk/libexif/libexif.a \
              sdk/libm/libm.a sdk/libc/libc.a --end-group
```

In a build script: `. sdk/lib-stale.sh; lib_ensure libpng; lib_ensure libjpeg; …`.
The flags are the app set (`-target aarch64-unknown-none-elf -ffreestanding
-nostdlib -O2 -fPIE -fno-builtin -nostdinc -funsigned-char`,
`-Wno-macro-redefined` for the clang-`float.h`/OpenLibm `INFINITY` pair).
Include `<stdio.h>` before `<jpeglib.h>` (IJG rule).

Per codec (zView `plugins/*/Makefile`):

| Codec | Needs (besides libldg, libshare, libc) |
|---|---|
| png | libpng, zlib, libm |
| jpg | libjpeg, libexif, libm |
| gif | giflib |
| bmp, tga, degas, neochrome, eureka, godpaint, img | none |
| tiff | libtiff, libjpeg, zlib, **liblzma**, libm — not built (M2 scope ends at the demo codecs) |

zView itself links `-lpng -lz` too, and `-lfreetype` only for PDF (xpdf).

## 3. The test: LIBTEST.NCP

`sdk/examples/libtest/`:

| File | Role |
|---|---|
| `libtest.c` | the test; host and target from one source |
| `gen-data.py` | makes the 14 test files (PNG by its own encoder: every filter type, Adam7, palette+tRNS, 16-bit, 1-bit, gray+alpha; JPEG/GIF via Pillow; EXIF APP1 with a thumbnail built byte by byte) and the **Python references** → `libtest-data.h` |
| `make-ref.sh` | builds the same pinned sources for the host, runs `libtest.c` there → **host references** `libtest-ref.h`; `--check` = SDK host test `libtest-host` |
| `build-libtest.sh` | target build → `sdk/examples/libtest/build/LIBTEST.NCP` |

38 checks: zlib (crc32/adler32 vs Python, compress2/uncompress, a Python-made
stream inflated, gz file round trip), libpng (six decodes to RGBA8 exactly as
`pngldg.c` asks, `png_get_text`, write + read back), libjpeg (baseline 4:2:0,
progressive 4:4:4, grayscale, EXIF/COM markers, `jpeg_mem_src` = file decode,
encode + decode back), libexif (tag text as `jpgldg.c` formats it, thumbnail
bytes and pixels), giflib (plain, interlaced + transparency + comment,
2-frame animation, read with `gifldg.c`'s record loop). Results are CRC-32
(own implementation, not zlib's) + length. PNG, GIF, the inflated stream,
the EXIF thumbnail and crc32/adler32 have a Python reference computed without
the libraries; JPEG pixels only a host reference (Pillow uses libjpeg-turbo,
whose chroma upsampling differs from IJG 9).

Verified (RUNINFO): QEMU raspi4b, `\AUTO\LIBTEST.NCP`, memprot on and strict:
`libtest: ALL PASS (38 checks)`; host `make-ref.sh --check` likewise.

Run it yourself: put `LIBTEST.NCP` into `C:\AUTO\` (or start it from the
desktop); it writes and deletes `LT_*.TMP`/`LT_Z.GZ` in the current directory.

## 4. Not here

- `libshare` (every codec's `libshare_init/exit`; since TASK/LDG-M3-B2,
  [10](10-ldg-modules.md) §6), libtiff/liblzma, freetype — not built (tiff and
  PDF are left out of zView, [13](13-zview.md)).
- The codecs themselves: a syntax probe of `pngldg.c`/`gifldg.c`/`jpgldg.c`
  against these headers shows **no library API mismatch**, but `jpgldg.c`
  assigns a `FILE *` to the `int32` field `_priv_var` and back without a
  cast (`jpgldg.c:469,592,702,750`): clang rejects that (`-Wint-conversion`
  is an error), and it truncates if `int32` is made 32-bit as
  `zview-ldg-2026-10-04.md` §1.3 suggests — a codec-side LP64 issue for M4
  (`measurements/measurement-2026-10-06-ldg-m2/checks/codec-syntax-probe.txt`). Built and handled in TASK/ZVIEW-M4: [13](13-zview.md) §3.
- Shared/SLB variants: everything is static.
