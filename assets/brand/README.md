# Hush identity

The mark combines an **H** with a speech tail, in leaf green (`#c3e6a3`) and dark forest (`#16251f`). Use the complete tile for app icons; use the transparent mark or outlined wordmark where a tile would be distracting. Keep at least one quarter of the mark's width clear around it. Do not stretch, rotate or add effects.

- `hush-icon.svg`: master app icon, with transparent corners.
- `hush-icon-1024.png`, `hush-icon-512.png`: reusable PNG exports.
- `hush-mark.svg`, `hush-mark-light.svg`: transparent forest and leaf marks.
- `hush-wordmark.svg`, `hush-wordmark.png`: icon and outlined Manrope wordmark; no installed font required.
- `../../native/assets/icon.ico`: Windows executable and installer sizes, 16–256 px.
- `../../native/assets/icon.png`, `icon.rgba`: 256 px native window and UI artwork. The raw RGBA asset avoids adding a decoder to the application. A single cached texture is shared by all UI instances of the logo.

To regenerate all artwork, install Python `fonttools` and ImageMagick, then run from the repository root:

```sh
python3 scripts/brand-assets.py
```

Normal app/CI builds use the committed assets and do not require these design tools. The wordmark uses the bundled Manrope font under its [SIL Open Font License](../../native/assets/OFL-Manrope.txt).
