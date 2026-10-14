# App icon

`icon.svg` is the source (1024 × 1024, macOS icon grid: an 824 px squircle centred on the canvas).

Regenerate the bundle icons after changing it:

```bash
rsvg-convert -w 1024 -h 1024 assets/icon/icon.svg -o assets/icon/icon.png
npx tauri icon assets/icon/icon.png
```

Then delete the generated `android/`, `ios/`, `Square*Logo.png`, `StoreLogo.png`, `icon.ico` and
`64x64.png` files — the app only ships on macOS.
