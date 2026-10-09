import { describe, it, expect } from 'vitest';
import { acceptCandidate, addTexts, applyTemplate, builtinTemplates, clone, codepoints, editArtwork, graphemes, makeGlyph, newProject, previewTokens, restore } from './model';
describe('exact Unicode and independent artwork', () => {
  it('preserves ZWJ, selectors and decomposed text', () => {
    const s = '👩‍💻❤️e\u0301';
    expect(graphemes(s)).toEqual(['👩‍💻','❤️','e\u0301']);
    expect(graphemes(s).join('')).toBe(s);
    expect(codepoints('❤️')).toEqual(['U+2764','U+FE0F']);
  });
  it('keeps Korean and Japanese forms separate', () => {
    let p = newProject(); p = addTexts(p,['骨'],'ko','han-ko'); p = addTexts(p,['骨'],'ja','han-ja');
    expect(p.glyphs.filter(g => g.text === '骨')).toHaveLength(2);
    expect(addTexts(p,['骨'],'ja','han-ja').glyphs).toHaveLength(p.glyphs.length);
  });
  it('restores the exact immutable template after serialization', () => {
    let g = applyTemplate(makeGlyph('A'),builtinTemplates()[0]);
    g.artwork.shapes[0].color = '#ffffff';
    g = JSON.parse(JSON.stringify(g));
    expect(restore(g,'template').artwork.shapes[0].color).toBe('#34352f');
    expect(restore(g,'system').text).toBe('A');
    expect(() => restore(makeGlyph('B'),'template')).toThrow();
  });
  it('protects locked, blank, control and template restrictions', () => {
    const g = makeGlyph('A'); g.locked = true;
    expect(editArtwork(g,{shapes:[],advance:42})).toBe(g);
    const control = makeGlyph('\n'); expect(editArtwork(control,{shapes:[],advance:42})).toBe(control);
    const blank = {...clone(g), locked:false, status:'blank' as const, candidates:[{id:'c',name:'test',artwork:{shapes:[],advance:50}}]};
    expect(acceptCandidate(blank,'c')).toBe(blank);
  });
  it('matches longest custom mapping and exact regional forms', () => {
    const custom = makeGlyph('hello','default','custom',true);
    const ko = makeGlyph('骨','ko'); const ja = makeGlyph('骨','ja');
    expect(previewTokens('hello骨❤️',[custom,ko,ja],'ja').map(t=>t.glyph?.id)).toEqual([custom.id,ja.id,undefined]);
  });
});
