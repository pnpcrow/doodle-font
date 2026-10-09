# Third-party notices

Optional locally installed AI: llama.cpp b11531 (MIT, ggml-org), EmbeddingGemma 2 (Apache-2.0, Google DeepMind), Gemma 4 E2B (Apache-2.0, Google DeepMind). The installer pins ggml-org GGUF conversions and validates their SHA256. Original model cards: https://huggingface.co/google/embeddinggemma-2 and https://huggingface.co/google/gemma-4-E2B-it . Runtime license: https://github.com/ggml-org/llama.cpp/blob/b11531/LICENSE . Model Apache license: https://www.apache.org/licenses/LICENSE-2.0 . These models are separate downloads, not embedded into the application executable.

Doodle Font uses Rust crates from crates.io, pinned in `Cargo.lock`. Notable components include:

- egui / eframe / epaint: MIT OR Apache-2.0
- ttf-parser / rustybuzz / tiny-skia: MIT
- image: MIT OR Apache-2.0
- serde / serde_json / tokio / reqwest: MIT OR Apache-2.0
- unicode-segmentation: MIT OR Apache-2.0
- rfd: MIT
- zip: MIT
- uuid: Apache-2.0 OR MIT

The built-in three doodle templates and the application icon were created for this application and are offered as CC0-1.0 artwork. No Apple, Samsung, Noto, or OpenMoji emoji images are bundled. Windows fonts are read from the user's system and are not redistributed.

eframe includes its default text fonts. Their licenses and the full resolved crate license inventory are included in the packaged `licenses/` directory generated during packaging. This notice does not assign a license to the application's own source code.
