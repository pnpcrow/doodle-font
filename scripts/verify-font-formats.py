"""Development-only independent validation. Run the Rust verify binary first.

Requires fontTools[woff] (includes Brotli); never used by the desktop application.
"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.pens.basePen import NullPen
import struct
import base64
import json
import sys

ROOT = Path(__file__).resolve().parent.parent / "artifacts" / "verification"


def windows_load(path):
    """Use the real Windows memory and file loaders, without installing any font."""
    if sys.platform != "win32":
        return
    import ctypes as c
    from ctypes import wintypes as w
    gdi = c.WinDLL("gdi32")
    gdi.AddFontMemResourceEx.argtypes = [c.c_void_p, w.DWORD, c.c_void_p, c.POINTER(w.DWORD)]
    gdi.AddFontMemResourceEx.restype = w.HANDLE
    gdi.RemoveFontMemResourceEx.argtypes = [w.HANDLE]
    gdi.RemoveFontMemResourceEx.restype = w.BOOL
    gdi.AddFontResourceExW.argtypes = [w.LPCWSTR, w.DWORD, c.c_void_p]
    gdi.AddFontResourceExW.restype = c.c_int
    gdi.RemoveFontResourceExW.argtypes = [w.LPCWSTR, w.DWORD, c.c_void_p]
    gdi.RemoveFontResourceExW.restype = w.BOOL
    data = path.read_bytes()
    buffer = c.create_string_buffer(data)
    count = w.DWORD()
    handle = gdi.AddFontMemResourceEx(buffer, len(data), None, c.byref(count))
    assert handle, f"Windows rejected font bytes: {path.name}"
    try:
        assert count.value > 0, path.name
    finally:
        assert gdi.RemoveFontMemResourceEx(handle)
    name = str(path.resolve())
    loaded = gdi.AddFontResourceExW(name, 0x10, None)  # FR_PRIVATE
    assert loaded > 0, f"Windows rejected font file: {path.name}"
    assert gdi.RemoveFontResourceExW(name, 0x10, None)


def raw(font, tag):
    return font.reader[tag]


def verify(stem):
    with TTFont(ROOT / f"{stem}.ttf", checkChecksums=2) as reference:
        for extension in ("ttf", "otf", "woff", "woff2"):
            path = ROOT / f"{stem}.{extension}"
            with TTFont(path, checkChecksums=2, recalcTimestamp=False) as font:
                font.ensureDecompiled(recurse=True)
                assert font["maxp"].numGlyphs == reference["maxp"].numGlyphs
                assert font.getBestCmap().keys() == reference.getBestCmap().keys()
                bmp = font["cmap"].getcmap(3, 1)
                full = font["cmap"].getcmap(3, 10)
                assert bmp is not None and bmp.format == 4
                assert full is not None and full.format == 12
                expected = {cp: name for cp, name in full.cmap.items()
                            if cp < 0xffff and font.getGlyphID(name) != 0}
                assert bmp.cmap == expected, (path.name, "BMP/full-Unicode mismatch")
                for tag in reference.reader.keys():
                    if tag in ("head", "glyf", "loca", "maxp") and extension == "otf":
                        continue
                    actual = raw(font, tag)
                    expected = raw(reference, tag)
                    if tag == "head" and extension == "woff2":
                        # WOFF2 mandates flag bit 11 and a new global checksum.
                        assert struct.unpack_from(">H", actual, 16)[0] & (1 << 11)
                        actual = actual[:8] + expected[8:12] + actual[12:16] + expected[16:18] + actual[18:]
                    assert actual == expected, (path.name, tag)
                if extension == "otf":
                    assert font.sfntVersion == "OTTO"
                    assert "glyf" not in font and "loca" not in font
                    assert font["maxp"].tableVersion == 0x5000
                    top = font["CFF "].cff.topDictIndex[0]
                    assert top.ROS == ("Adobe", "Identity", 0)
                    assert len(top.charset) == font["maxp"].numGlyphs
                    for name in top.charset:
                        cs = top.CharStrings[name]
                        cs.draw(NullPen())
                        assert cs.width == font["hmtx"][name][0], (path.name, name)
                # Reconstruct a conventional font using an independent implementation.
                font.flavor = None
                target = ROOT / f"decoded-{stem}-{extension}.{'otf' if extension == 'otf' else 'ttf'}"
                font.save(target)
                with TTFont(target, checkChecksums=2) as decoded:
                    decoded.ensureDecompiled(recurse=True)
                if extension in ("ttf", "otf"):
                    windows_load(path)
                print(f"PASS {path.name}: {path.stat().st_size:,} bytes / {font['maxp'].numGlyphs:,} glyphs")


if __name__ == "__main__":
    for mode in ("mono", "color", "bitmap"):
        verify(f"demo-{mode}")
    if (ROOT / "large-font.otf").exists():
        verify("large-font")
    # Self-contained browser fixture: FontFace.load exercises the browser's font sanitizer.
    fonts = []
    for mode in ("mono", "color", "bitmap"):
        for extension in ("ttf", "otf", "woff", "woff2"):
            name = f"demo-{mode}.{extension}"
            fonts.append([name, base64.b64encode((ROOT / name).read_bytes()).decode()])
    html = """<!doctype html><meta charset="utf-8"><title>Doodle Font export verification</title>
<style>body{font:16px sans-serif;background:#f8f7f3;padding:24px}main{display:grid;grid-template-columns:repeat(4,1fr);gap:12px}article{background:white;padding:14px;border:1px solid #ddd;border-radius:12px}p{font-size:38px;word-break:break-all;font-feature-settings:'rlig' 1}</style>
<h1>Doodle Font · 12 export combinations</h1><div id="status">Loading fonts…</div><main></main><script>
const fonts = FONT_DATA;
Promise.all(fonts.map(async ([name,data],i)=>{
 const card=document.createElement('article');document.querySelector('main').append(card);
 const title=document.createElement('h3');title.textContent=name;card.append(title);
 const sample=document.createElement('p');sample.lang='ja';sample.textContent='A❤️😊 fi 骨';card.append(sample);
 try{const f=await new FontFace('doodle'+i,'url(data:font/'+name.split('.').pop()+';base64,'+data+')').load();
 document.fonts.add(f);sample.style.fontFamily='doodle'+i;card.dataset.result='PASS';return true;}
 catch(e){card.dataset.result='FAIL';card.append(String(e));return false;}
})).then(results=>{const ok=results.every(Boolean);document.body.dataset.status=ok?'PASS':'FAIL';document.getElementById('status').textContent=results.filter(Boolean).length+'/12 fonts loaded';});
</script>""".replace("FONT_DATA", json.dumps(fonts))
    (ROOT / "browser-fonts.html").write_text(html, encoding="utf-8")
