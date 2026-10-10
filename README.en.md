<img src="assets/brand.png" width="72" height="72" alt="Doodle Font logo">

# Doodle Font

**Turn your own drawings into a font on Windows.**

Draw letters or small pictograms, try them in a sentence, and export your own font. All basic editing and export features work without AI.

[한국어](README.md) · **English** · [日本語](README.ja.md)

[Download](https://github.com/pnpcrow/doodle-font/releases/latest) · [Release notes (Korean)](https://github.com/pnpcrow/doodle-font/releases/tag/v0.1.0) · [Report an issue](https://github.com/pnpcrow/doodle-font/issues)

## Get started

1. Download **Doodle-Font-0.1.0-windows-x64.zip** from the [latest release](https://github.com/pnpcrow/doodle-font/releases/latest).
2. Extract the entire ZIP and run **Doodle-Font.exe**. No installer is needed.
3. Choose Korean, English, or Japanese in **Settings → Display language**. The default follows your system language.

**Requirements:** Windows 10/11, 64-bit (x64), and a graphics driver with OpenGL support. The current version is **0.1.0**.

## What can you make?

| Feature | What you can do |
| --- | --- |
| Draw letters | Use pens, shapes, colors, and layers, or import PNG/JPEG images. |
| Work with different scripts | Choose Hangul, Latin letters, kana, Han characters, and symbols, or add just the characters from a sentence. |
| Preview text | See how your letters look together and check their spacing. |
| Use templates | Start from a template and restore its original artwork when needed. |
| Export fonts | Save **TTF or OTF** for desktop use, or **WOFF2 or WOFF** for websites. |

You can also create color letters. Support for color and special character sequences varies between the applications that use your font.

## Make your first font

1. Open **Project** to set the project name and font family name.
2. In **Draw**, select a character and draw it on the canvas.
3. Open **Text preview** to see how your letters work together.
4. Open **Export font**, choose a format, check the output, and save it. Choose TTF or OTF to install a font on Windows.

## Use AI when you want it

Choose an option in **Settings → AI assist**. AI is off by default.

| Option | How it works |
| --- | --- |
| Off | Draw and export fonts yourself. |
| Local AI | Selecting it for the first time installs the model automatically (about a 3.4 GB download). Once installed, it works without an internet connection or API key. |
| OpenAI compatible / Anthropic compatible | Enter your service's API endpoint, model name, and key. An internet connection is required, and service charges may apply. |

Ask for feedback on a drawing or generate alternatives. Compare a candidate with your current drawing, then select **Apply this candidate** to use it. You choose separately whether to send drawings to an API. API keys are encrypted using your current Windows account.

## Save your work and adjust the workspace

Your work is saved automatically on this PC. Use **Project → Save project copy** to make a separate copy or move it elsewhere. The default data folder is `%LOCALAPPDATA%/DoodleFont`.

Change the display language and light/dark theme in Settings. Drag the companion to move it, or right-click it for options. If the prompt panel covers your drawing, collapse it and reopen it from the bottom status bar.

## Learn more

- [Release notes and downloads (Korean)](https://github.com/pnpcrow/doodle-font/releases/tag/v0.1.0)
- [Third-party software and font notices](THIRD_PARTY_NOTICES.md)
- [Development, build, and validation guide (Korean)](https://github.com/pnpcrow/doodle-font/blob/main/docs/DEVELOPMENT.md)
