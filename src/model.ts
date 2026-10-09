export type Locale = 'default' | 'ko' | 'ja';
export type Status = 'missing' | 'edited' | 'blank' | 'system' | 'unsupported';
export type Point = [number, number];
export type Shape = {
  id: string; name: string; type: 'path' | 'rect' | 'ellipse' | 'image';
  points: Point[]; x: number; y: number; width: number; height: number;
  color: string; stroke: number; fill: boolean; opacity: number; visible: boolean; locked: boolean;
  src?: string;
};
export type Artwork = { shapes: Shape[]; advance: number };
export type Permissions = { display: boolean; edit: boolean; embed: boolean; redistribute: boolean; ai: boolean };
export type Template = {
  id: string; name: string; provider: string; version: string; source: string;
  license: string; attribution: string; permissions: Permissions; original: Artwork | null;
};
export type Glyph = {
  id: string; text: string; locale: Locale; group: string; name: string;
  mapping: 'unicode' | 'sequence' | 'custom' | 'pua' | 'control';
  status: Status; locked: boolean; artwork: Artwork; template?: Template;
  baseline?: Artwork; candidates: { id: string; name: string; artwork: Artwork }[];
};
export type Component = { id: string; name: string; artwork: Artwork; template?: Template };
export type Project = {
  schemaVersion: 1; id: string; name: string; family: string; createdAt: string; updatedAt: string;
  glyphs: Glyph[]; components: Component[]; templates: Template[];
};
export const uid = () => crypto.randomUUID();
export const clone = <T,>(x: T): T => structuredClone(x);
export const codepoints = (s: string) => Array.from(s).map(c => `U+${c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0')}`);
export const graphemes = (s: string) => [...new Intl.Segmenter('ko', { granularity: 'grapheme' }).segment(s)].map(x => x.segment);
export const isControl = (s: string) => /^[\p{Cc}\p{Cf}]+$/u.test(s);
export const keyOf = (s: string, locale: Locale) => `${locale}:${s}`;
export const displayText = (s: string) => ({ ' ': '␣', '\u00a0': 'NBSP', '\u3000': '全', '\t': '⇥', '\n': '↵', '\r\n': '↵', '\u200d': 'ZWJ' }[s] || s);
export const shape = (type: Shape['type'], options: Partial<Shape> = {}): Shape => ({ id: uid(), name: type === 'path' ? '자유로운 선' : type === 'ellipse' ? '동그라미' : type === 'rect' ? '네모' : '가져온 이미지', type, points: [], x: 150, y: 150, width: 600, height: 600, color: '#34352f', stroke: 36, fill: false, opacity: 1, visible: true, locked: false, ...options });
export const path = (points: Point[], color = '#34352f', stroke = 42) => shape('path', { points, color, stroke });
export function makeGlyph(text: string, locale: Locale = 'default', group = 'custom', custom = false): Glyph {
  const cp = text.codePointAt(0)!;
  return { id: uid(), text, locale, group, name: displayText(text), mapping: isControl(text) ? 'control' : custom ? 'custom' : cp >= 0xe000 && cp <= 0xf8ff ? 'pua' : Array.from(text).length === 1 ? 'unicode' : 'sequence', status: isControl(text) ? 'unsupported' : 'missing', locked: false, artwork: { shapes: [], advance: text === ' ' || text === '\u00a0' ? 400 : 1000 }, candidates: [] };
}
export function addTexts(project: Project, texts: string[], locale: Locale, group: string, custom = false): Project {
  const keys = new Set(project.glyphs.map(g => keyOf(g.text, g.locale)));
  const glyphs = [...project.glyphs];
  for (const text of texts) if (text && !keys.has(keyOf(text, locale))) { glyphs.push(makeGlyph(text, locale, group, custom)); keys.add(keyOf(text, locale)); }
  return { ...project, glyphs };
}
export function editArtwork(g: Glyph, artwork: Artwork): Glyph {
  if (g.locked || g.mapping === 'control' || (g.template && !g.template.permissions.edit)) return g;
  return { ...g, artwork: clone(artwork), status: 'edited' };
}
export function restore(g: Glyph, mode: 'template' | 'baseline' | 'system'): Glyph {
  if (g.locked || g.mapping === 'control') throw new Error('잠긴 글리프 또는 제어 문자는 복원할 수 없어요.');
  if (mode === 'system') return { ...g, status: 'system' };
  const artwork = mode === 'template' ? g.template?.original : g.baseline;
  if (!artwork) throw new Error('복원할 원본 데이터가 없어요. 해당 버전의 원본을 가져와 주세요.');
  return { ...g, artwork: clone(artwork), status: 'edited' };
}
export function applyTemplate(g: Glyph, template: Template): Glyph {
  if (g.locked || g.mapping === 'control') throw new Error('이 글리프는 보호되어 있어요.');
  if (!template.permissions.display || !template.original) throw new Error('표시할 수 있는 템플릿 원본이 없어요.');
  return { ...g, template: clone(template), artwork: clone(template.original), baseline: undefined, candidates: [], status: 'edited' };
}
export function acceptCandidate(g: Glyph, id: string): Glyph {
  if (g.status === 'blank' || g.status === 'unsupported') return g;
  const candidate = g.candidates.find(c => c.id === id);
  return candidate ? editArtwork(g, candidate.artwork) : g;
}
export function moveShape(s: Shape, dx: number, dy: number): Shape {
  return { ...s, x: s.x + dx, y: s.y + dy, points: s.points.map(([x,y]) => [x + dx, y + dy]) };
}
export function smooth(points: Point[]): Point[] {
  if (points.length < 3) return points;
  return [points[0], ...points.slice(1, -1).map((p, i): Point => [(points[i][0] + 2 * p[0] + points[i + 2][0]) / 4, (points[i][1] + 2 * p[1] + points[i + 2][1]) / 4]), points[points.length - 1]];
}
export const range = (a: number, b: number) => Array.from({ length: b - a + 1 }, (_, i) => String.fromCodePoint(a + i));
export const presets = [
  { id: 'latin', name: '라틴 알파벳', mark: 'Aa', locale: 'default' as Locale, chars: () => range(32, 126) },
  { id: 'latin-ext', name: '확장 라틴 · 결합 부호', mark: 'É', locale: 'default' as Locale, chars: () => [...range(0xc0, 0x17f), ...range(0x300, 0x36f)] },
  { id: 'hangul', name: '한글', mark: '가', locale: 'ko' as Locale, chars: () => range(0xac00, 0xd7a3) },
  { id: 'jamo', name: '한글 자모 · 옛한글', mark: 'ㄱ', locale: 'ko' as Locale, chars: () => [...range(0x1100,0x11ff), ...range(0x3131,0x318e), ...range(0xa960,0xa97c), ...range(0xd7b0,0xd7c6), ...range(0xd7cb,0xd7fb)] },
  { id: 'han-ko', name: '한국한자', mark: '韓', locale: 'ko' as Locale, chars: () => range(0x4e00, 0x9fff) },
  { id: 'kana', name: '히라가나 · 가타카나', mark: 'あ', locale: 'ja' as Locale, chars: () => [...range(0x3041,0x3096), ...range(0x3099,0x30ff), ...range(0xff61,0xff9f)] },
  { id: 'han-ja', name: '일본한자', mark: '漢', locale: 'ja' as Locale, chars: () => range(0x4e00,0x9fff) },
  { id: 'symbols', name: '기호 · 이모지', mark: '☺', locale: 'default' as Locale, chars: () => [...graphemes('♡♥★☆✦✿☀☁☂→←↑↓✓✕∞±×÷=≠😀😊🥰😎😭👍👍🏽❤️👩‍💻👨‍👩‍👧‍👦🌷🌈✨🎉🍀'), ...range(0x2190,0x21ff)] },
  { id: 'spaces', name: '공백 · 제어 문자', mark: '␣', locale: 'default' as Locale, chars: () => [' ', '\u00a0', '\u3000', '\u2002', '\u2003', '\u2009', '\t', '\n', '\u200d'] },
  { id: 'custom', name: '사용자 정의', mark: '◇', locale: 'default' as Locale, chars: () => [] },
];
export const statusLabels: Record<Status, string> = { missing: '미작성', edited: '작성됨', blank: '의도적 빈칸', system: '시스템 표시', unsupported: '미지원' };
export function builtinTemplates(): Template[] {
  const permissions = { display: true, edit: true, embed: true, redistribute: true, ai: true };
  const base = { provider: 'Doodle Font', version: '1.0.0', source: 'builtin://doodle-font/1.0.0', license: 'CC0-1.0', attribution: 'Doodle Font original artwork', permissions };
  return [
    { ...base, id: 'doodle-a-v1', name: '연필로 쓴 A', original: { advance: 900, shapes: [path([[160,790],[222,617],[313,350],[415,176],[450,160],[500,192],[580,409],[705,780]]), path([[242,574],[388,552],[565,560]], '#34352f', 36)] } },
    { ...base, id: 'doodle-smile-v1', name: '햇살 같은 미소', original: { advance: 1000, shapes: [shape('ellipse', { x: 160, y: 140, width: 680, height: 680, fill: true, color: '#f6be4f' }), shape('ellipse', { x: 320, y: 340, width: 44, height: 78, fill: true }), shape('ellipse', { x: 620, y: 340, width: 44, height: 78, fill: true }), path([[300,530],[350,590],[425,630],[500,642],[575,627],[645,583],[680,530]], '#34352f', 34)] } },
    { ...base, id: 'doodle-heart-v1', name: '작은 마음', original: { advance: 1000, shapes: [shape('path', { points: [[500,775],[180,453],[156,345],[193,235],[295,186],[405,201],[500,299],[596,204],[706,186],[809,246],[846,355],[816,459]], fill: true, color: '#df795c', stroke: 0 })] } },
  ];
}
export function newProject(): Project {
  const templates = builtinTemplates();
  let p: Project = { schemaVersion: 1, id: uid(), name: '나의 첫 번째 폰트', family: 'Doodle Hand', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), glyphs: [], components: [], templates };
  p = addTexts(p, range(65, 90).concat(range(97,122)), 'default', 'latin');
  p = addTexts(p, graphemes('가나다라마바사아자차카타파하'), 'ko', 'hangul');
  p = addTexts(p, graphemes('❤️😊✦🌷'), 'default', 'symbols');
  p = addTexts(p, [' ', '\u00a0','\u3000','\t','\n','\u200d'], 'default', 'spaces');
  p.glyphs = p.glyphs.map(g => g.text === 'A' ? applyTemplate(g, templates[0]) : g.text === '❤️' ? applyTemplate(g, templates[2]) : g.text === '😊' ? applyTemplate(g, templates[1]) : g);
  return p;
}
export function previewTokens(text: string, glyphs: Glyph[], locale: Locale): { text: string; glyph?: Glyph }[] {
  const map = new Map<string,Glyph>();
  for (const g of glyphs) if (g.locale === 'default') map.set(g.text,g);
  for (const g of glyphs) if (g.locale === locale) map.set(g.text,g);
  const custom = [...map.values()].filter(g => g.mapping === 'custom').sort((a,b) => b.text.length-a.text.length);
  const tokens: { text: string; glyph?: Glyph }[] = [];
  const segments = new Map([...new Intl.Segmenter('ko',{granularity:'grapheme'}).segment(text)].map(s => [s.index,s.segment]));
  let i = 0;
  while (i < text.length) {
    const match = custom.find(g => text.startsWith(g.text,i));
    const t = match?.text || segments.get(i) || String.fromCodePoint(text.codePointAt(i)!);
    tokens.push({ text: t, glyph: match || map.get(t) }); i += t.length;
  }
  return tokens;
}
export const isAuthored = (g: Glyph) => ['edited','blank'].includes(g.status) && g.mapping !== 'control';
