# Third-party notices

The local typography engine includes Noto Sans CJK and Noto Serif CJK variable fonts (Adobe/Google/Noto contributors) and Single Day (Copyright 2015 DXKorea Inc.). These fonts are licensed under the SIL Open Font License 1.1. Pinned sources and SHA256 hashes are in `assets/fonts/manifest.json`; full licenses are in `assets/fonts/*OFL.txt` and packaged in `licenses/typefaces/`. AI candidates derived from these fonts retain their source metadata. Accepting such candidates carries the metadata into the project, and exported font name tables and reports include OFL terms; split ZIP exports also include license files. The application source code has not been assigned the fonts' license.

Optional locally installed AI: llama.cpp b11531 (MIT, ggml-org), Gemma 4 E2B (Apache-2.0, Google DeepMind), and the independent Needle 3 decision model/runtime (Apache-2.0, Cactus Compute). The installers pin revisions and validate SHA256, with manifests in `assets/local-ai-manifest.json` and `assets/needle-ai-manifest.json`. Original model cards: https://huggingface.co/google/gemma-4-E2B-it and https://huggingface.co/Cactus-Compute/needle3 . Runtime licenses: https://github.com/ggml-org/llama.cpp/blob/b11531/LICENSE and https://huggingface.co/Cactus-Compute/needle3/blob/2ae11323dc000f5e70c49f7403efa6af12ba9e67/LICENSE . The Apache license is supplied with both installations. These models are separate downloads, not embedded into the application executable. Legacy EmbeddingGemma files are no longer required or executed; existing user files are retained.

Doodle Font uses Rust crates from crates.io, pinned in `Cargo.lock`. Notable components include:

- egui / eframe / epaint: MIT OR Apache-2.0
- ttf-parser / rustybuzz / tiny-skia: MIT
- image: MIT OR Apache-2.0
- serde / serde_json / tokio / reqwest: MIT OR Apache-2.0
- unicode-segmentation: MIT OR Apache-2.0
- rfd: MIT
- zip: MIT
- flate2 (WOFF compression): MIT OR Apache-2.0
- brotli (WOFF2 compression): BSD-3-Clause AND MIT
- uuid: Apache-2.0 OR MIT

The built-in three doodle templates and the application icon were created for this application and are offered as CC0-1.0 artwork. No Apple, Samsung, Noto, or OpenMoji emoji images are bundled. Windows fonts are read from the user's system and are not redistributed.

eframe includes its default text fonts. Their licenses and the full resolved crate license inventory are included in the packaged `licenses/` directory generated during packaging. This notice does not assign a license to the application's own source code.
