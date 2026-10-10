# Doodle Font mark

The terracotta rounded square with a white lowercase **d** is defined by curves in `src/brand_art.rs`.

Regenerate with `cargo run --locked --bin generate_brand`. This produces:

- `brand.png`: 256-pixel RGBA mark shared by the title bar and native window/taskbar icon.
- `icon.ico`: 16, 20, 24, 32, 40, 48, 64, 128 and 256-pixel renders for the Windows executable resource in `build.rs`.

Each resolution is rasterized from the same geometry. Transparent edges are unpremultiplied when encoding PNG. The header caches one texture; it does not redraw a font-dependent letter. `tests/brand.rs` compares every embedded frame against the canonical curves and confirms the header/native PNG is identical to the 256-pixel mark.
