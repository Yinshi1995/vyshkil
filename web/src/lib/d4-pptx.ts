// D4 "Підготовка" — client-side pptx via pptxgenjs.
// Візуальний стиль — за еталонними презентаціями Dopovid:
//   темний фон 26241D, порожні нумеровані квадрати, емблема, cream текст.
// Лише кількості груп, без імен людей (CLAUDE.md).

// Dynamic import — pptxgenjs is ~350KB, lazy-loaded on first use
type PptxGenJSType = typeof import("pptxgenjs").default;
let PptxGenJS: PptxGenJSType | null = null;
async function getPptxGenJS(): Promise<PptxGenJSType> {
  if (!PptxGenJS) {
    const mod = await import("pptxgenjs");
    PptxGenJS = mod.default;
  }
  return PptxGenJS;
}

// ---------------------------------------------------------------------------
// Data types (mirror server D4DataJson)
// ---------------------------------------------------------------------------

interface KindData {
  total: number;
  finishing: number;
  started: number;
}

interface UnitData {
  label: string;
  bzvp: KindData;
  special: KindData;
  adaptation: KindData;
}

interface CorpsData {
  label: string;
  units: UnitData[];
  yesterday_bzvp: number;
  yesterday_special: number;
  yesterday_adaptation: number;
}

export interface D4Data {
  date: string;
  corps: CorpsData[];
}

// ---------------------------------------------------------------------------
// Theme — dark military (matching Dopovid + reference presentations)
// ---------------------------------------------------------------------------

const SLIDE_W = 13.333;
const FONT = { head: "Arial", body: "Arial" };

const C = {
  bg:        "26241D",  // slide background (dark)
  panel:     "302D24",  // panels / KPI tile bg
  panelAlt:  "4D4634",  // lighter panel (table header)
  border:    "453F2E",  // borders on dark
  accent:    "F39200",  // orange
  accentDark:"855000",  // dark amber for outlines
  text:      "F5F2EC",  // primary text (cream on dark)
  text2:     "C7C1B0",  // secondary text
  muted:     "9C947F",  // muted (footer etc)
  white:     "FFFFFF",
  rowEven:   "302D24",  // alternating table row
  rowOdd:    "2E2B23",  // alternating table row
  total:     "F39200",  // total row bg
  callout:   "2A2820",  // callout bar bg (deeper dark)
} as const;

const M = 0.45;
const CW = SLIDE_W - 2 * M;
const MAX_TABLE_ROWS = 11;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function thousands(n: number): string {
  if (n === 0) return "0";
  return n.toLocaleString("uk-UA");
}

function signedDelta(d: number): string {
  if (d > 0) return `+${thousands(d)}`;
  if (d < 0) return thousands(d);
  return "0";
}

function formatChanges(k: KindData): string {
  const f = k.finishing;
  const s = k.started;
  if (f === 0 && s === 0) return "—";
  const parts: string[] = [];
  if (f > 0) parts.push(String(f));
  if (s > 0) parts.push(`+${s}`);
  return parts.join("/") || "—";
}

function sumKind(units: UnitData[], field: "bzvp" | "special" | "adaptation"): KindData {
  let total = 0, finishing = 0, started = 0;
  for (const u of units) {
    total += u[field].total;
    finishing += u[field].finishing;
    started += u[field].started;
  }
  return { total, finishing, started };
}

function parseDate(iso: string): string {
  const [y, m, d] = iso.split("-");
  return `${d}.${m}.${y}`;
}

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

function unitHasData(u: UnitData): boolean {
  return u.bzvp.total + u.special.total + u.adaptation.total > 0;
}

async function loadEmblemBase64(): Promise<string | null> {
  try {
    const resp = await fetch("/emblem.png");
    if (!resp.ok) return null;
    const blob = await resp.blob();
    return new Promise((resolve) => {
      const reader = new FileReader();
      reader.onloadend = () => resolve(reader.result as string);
      reader.readAsDataURL(blob);
    });
  } catch {
    return null;
  }
}

// ---------------------------------------------------------------------------
// Slide furniture
// ---------------------------------------------------------------------------

function footer(slide: PptxGenJS.Slide, classification = "ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ") {
  slide.addText(classification, {
    x: SLIDE_W - M - 4, y: 7.06, w: 4, h: 0.28,
    fontSize: 9.5, color: C.muted, fontFace: FONT.body, charSpacing: 3,
    align: "right", valign: "middle",
  });
}

function emblem(slide: PptxGenJS.Slide, data: string, mode: "title" | "content") {
  // 1:1 aspect ratio
  if (mode === "title") {
    slide.addImage({ data, x: 5.77, y: 0.45, w: 1.80, h: 1.80 });
  } else {
    slide.addImage({ data, x: 12.08, y: 0.30, w: 0.88, h: 0.88 });
  }
}

function sectionHeader(
  slide: PptxGenJS.Slide, pptx: PptxGenJS,
  num: string, title: string, subtitle?: string,
) {
  // Hollow numbered box (orange outline, no fill — matching refs exactly)
  slide.addShape(pptx.ShapeType.rect, {
    x: M, y: 0.52, w: 0.72, h: 0.72,
    fill: { type: "none" } as any,
    line: { color: C.accent, width: 1.5 },
  });
  slide.addText(num, {
    x: M, y: 0.52, w: 0.72, h: 0.72,
    fontSize: 22, bold: true, color: C.accent, fontFace: FONT.head,
    align: "center", valign: "middle",
  });

  // Title
  slide.addText(title, {
    x: M + 0.97, y: 0.50, w: CW - 1.6, h: 0.52,
    fontSize: 22, bold: true, color: C.text, fontFace: FONT.head,
    valign: "middle",
  });

  if (subtitle) {
    slide.addText(subtitle, {
      x: M + 0.99, y: 1.04, w: CW - 1.6, h: 0.34,
      fontSize: 12.5, color: C.text2, fontFace: FONT.body, valign: "top",
    });
  }
}

// ---------------------------------------------------------------------------
// KPI tile — dark panel, cream/orange text
// ---------------------------------------------------------------------------

function kpiTile(
  slide: PptxGenJS.Slide, pptx: PptxGenJS,
  x: number, y: number, w: number, h: number,
  label: string, value: string, sub: string,
  valueColor = C.text,
) {
  // Panel background
  slide.addShape(pptx.ShapeType.rect, {
    x, y, w, h,
    fill: { color: C.panel },
    line: { color: C.border, width: 0.75 },
  });
  // Big number
  const numH = h * 0.42;
  slide.addText(value, {
    x, y: y + h * 0.05, w, h: numH,
    fontSize: Math.min(54, h * 26), bold: true, color: valueColor,
    fontFace: FONT.body, align: "center", valign: "middle",
    isTextBox: true, fit: "none", wrap: false,
  });
  // Label
  slide.addText(label, {
    x: x + 0.15, y: y + numH + h * 0.05, w: w - 0.3, h: 0.40,
    fontSize: 14, bold: true, color: C.text,
    fontFace: FONT.body, align: "center", valign: "middle",
  });
  // Subtitle
  slide.addText(sub, {
    x: x + 0.15, y: y + h - 0.55, w: w - 0.3, h: 0.40,
    fontSize: 11, color: C.text2, fontFace: FONT.body,
    align: "center", valign: "middle",
  });
}

function kpiTileSmall(
  slide: PptxGenJS.Slide, pptx: PptxGenJS,
  x: number, y: number, w: number, h: number,
  label: string, value: string, sub: string,
  valueColor = C.text,
) {
  slide.addShape(pptx.ShapeType.rect, {
    x, y, w, h,
    fill: { color: C.panel },
    line: { color: C.border, width: 0.75 },
  });
  slide.addText(value, {
    x, y: y + 0.06, w, h: h * 0.45,
    fontSize: 25, bold: true, color: valueColor,
    fontFace: FONT.body, align: "center", valign: "middle",
    isTextBox: true, fit: "none", wrap: false,
  });
  slide.addText(label, {
    x: x + 0.08, y: y + h * 0.50, w: w - 0.16, h: 0.26,
    fontSize: 10, bold: true, color: C.text,
    fontFace: FONT.body, align: "center", valign: "middle",
  });
  slide.addText(sub, {
    x: x + 0.08, y: y + h - 0.30, w: w - 0.16, h: 0.24,
    fontSize: 9, color: C.text2, fontFace: FONT.body,
    align: "center", valign: "middle",
  });
}

// ---------------------------------------------------------------------------
// Callout bar
// ---------------------------------------------------------------------------

function calloutBar(
  slide: PptxGenJS.Slide, pptx: PptxGenJS,
  y: number, label: string, text: string,
) {
  const barH = 0.70;
  slide.addShape(pptx.ShapeType.rect, {
    x: M, y, w: CW, h: barH,
    fill: { color: C.callout },
    line: { color: C.border, width: 0.75 },
  });
  // Label on top line
  slide.addText(label, {
    x: M + 0.32, y, w: CW - 0.6, h: 0.30,
    fontSize: 11, bold: true, color: C.accent, fontFace: FONT.body,
    charSpacing: 0.5, valign: "bottom",
  });
  // Text on bottom line
  slide.addText(text, {
    x: M + 0.32, y: y + 0.30, w: CW - 0.6, h: 0.35,
    fontSize: 12, color: C.text2, fontFace: FONT.body, valign: "top",
  });
}

// ---------------------------------------------------------------------------
// Title slide
// ---------------------------------------------------------------------------

function titleSlide(pptx: PptxGenJS, date: string, emblemData: string | null) {
  const slide = pptx.addSlide({ masterName: "TAKTOBLIK" });

  if (emblemData) emblem(slide, emblemData, "title");

  slide.addText("ДЛЯ СЛУЖБОВОГО КОРИСТУВАННЯ", {
    x: SLIDE_W - M - 4.5, y: 0.22, w: 4.5, h: 0.28,
    fontSize: 9.5, color: C.muted, fontFace: FONT.body, charSpacing: 3,
    align: "right", valign: "middle",
  });

  slide.addShape(pptx.ShapeType.line, {
    x: 1.35, y: 2.86, w: SLIDE_W - 2.7, h: 0,
    line: { color: C.accent, width: 2.2 },
  });

  slide.addText('УГРУПОВАННЯ ВІЙСЬК (СИЛ) "ПІВДЕНЬ"', {
    x: 1.35, y: 2.96, w: 10.63, h: 0.52,
    fontSize: 20, bold: true, color: C.text2, fontFace: FONT.head,
    charSpacing: 3.2, align: "center", valign: "middle",
  });

  slide.addText("ПІДГОТОВКА", {
    x: 1.35, y: 3.50, w: 10.63, h: 1.02,
    fontSize: 34, bold: true, color: C.text, fontFace: FONT.head,
    charSpacing: 1.5, align: "center", valign: "middle",
  });

  slide.addText("БЗВП  ·  ФАХОВА ПІДГОТОВКА  ·  АДАПТАЦІЯ", {
    x: 0.50, y: 5.45, w: 12.33, h: 0.35,
    fontSize: 13, color: C.text2, fontFace: FONT.body,
    charSpacing: 2, align: "center", valign: "middle",
  });

  slide.addText(`СТАНОМ НА ${parseDate(date).toUpperCase()}`, {
    x: 0.50, y: 5.95, w: 12.33, h: 0.32,
    fontSize: 14, bold: true, color: C.accent, fontFace: FONT.body,
    charSpacing: 3, align: "center", valign: "middle",
  });
}

// ---------------------------------------------------------------------------
// Overview slide
// ---------------------------------------------------------------------------

function overviewSlide(pptx: PptxGenJS, data: D4Data, emblemData: string | null) {
  const slide = pptx.addSlide({ masterName: "TAKTOBLIK" });
  footer(slide);
  if (emblemData) emblem(slide, emblemData, "content");
  sectionHeader(slide, pptx, "01", "ЗАГАЛЬНА ДИНАМІКА ПІДГОТОВКИ", `Станом на ${parseDate(data.date)}`);

  let bT = 0, sT = 0, aT = 0, bY = 0, sY = 0, aY = 0;
  for (const c of data.corps) {
    const b = sumKind(c.units, "bzvp");
    const s = sumKind(c.units, "special");
    const a = sumKind(c.units, "adaptation");
    bT += b.total; sT += s.total; aT += a.total;
    bY += c.yesterday_bzvp; sY += c.yesterday_special; aY += c.yesterday_adaptation;
  }
  const allT = bT + sT + aT;
  const allY = bY + sY + aY;

  const tileW = (CW - 0.20 * 3) / 4;
  const tileH = 2.50;
  const tileY = 1.80;
  const gap = 0.20;

  kpiTile(slide, pptx, M, tileY, tileW, tileH,
    "Усього залучено", thousands(allT),
    `зміна за добу: ${signedDelta(allT - allY)}`, C.accent);

  kpiTile(slide, pptx, M + (tileW + gap), tileY, tileW, tileH,
    "БЗВП", thousands(bT),
    `${thousands(bY)} → ${thousands(bT)}`);

  kpiTile(slide, pptx, M + (tileW + gap) * 2, tileY, tileW, tileH,
    "Фахова підготовка", thousands(sT),
    `${thousands(sY)} → ${thousands(sT)}`);

  kpiTile(slide, pptx, M + (tileW + gap) * 3, tileY, tileW, tileH,
    "Адаптація", thousands(aT),
    `${thousands(aY)} → ${thousands(aT)}`);

  const cY = tileY + tileH + 0.25;
  calloutBar(slide, pptx, cY, "ПІДСУМОК",
    `Загальна кількість: ${thousands(allT)} о/с.  Зміна за добу: ${signedDelta(allT - allY)}.`);
}

// ---------------------------------------------------------------------------
// Table helpers
// ---------------------------------------------------------------------------

function makeHeaderRow(): PptxGenJS.TableRow {
  const hdr = (text: string, align: "left" | "center" = "center"): PptxGenJS.TableCell => ({
    text: text.toUpperCase(),
    options: {
      bold: true,
      color: C.accent,
      fill: { color: C.panelAlt },
      align,
      fontSize: 9,
      fontFace: FONT.body,
      charSpacing: 0.5,
      valign: "middle",
    },
  });
  return [
    hdr("Підрозділ", "left"),
    hdr("БЗВП"), hdr("Зміни"),
    hdr("Фахова"), hdr("Зміни"),
    hdr("Адаптація"), hdr("Зміни"),
  ];
}

function makeDataRow(u: UnitData, rowIdx: number): PptxGenJS.TableRow {
  const bg = rowIdx % 2 === 0 ? C.rowOdd : C.rowEven;
  const cell = (text: string, align: "left" | "center" = "center", color = C.text, bold = false): PptxGenJS.TableCell => ({
    text,
    options: {
      fill: { color: bg }, color, bold, align,
      fontSize: 9, fontFace: FONT.body, valign: "middle",
    },
  });
  const chg = (k: KindData): PptxGenJS.TableCell => {
    const txt = formatChanges(k);
    const clr = k.started > 0 ? C.accent : C.text;
    return cell(txt, "center", clr, k.started > 0);
  };
  return [
    cell(u.label, "left"),
    cell(thousands(u.bzvp.total)), chg(u.bzvp),
    cell(thousands(u.special.total)), chg(u.special),
    cell(thousands(u.adaptation.total)), chg(u.adaptation),
  ];
}

function makeTotalRow(bt: KindData, st: KindData, at: KindData): PptxGenJS.TableRow {
  const tot = (text: string, align: "left" | "center" = "center"): PptxGenJS.TableCell => ({
    text,
    options: {
      bold: true, color: C.bg,
      fill: { color: C.total }, align,
      fontSize: 9, fontFace: FONT.body, valign: "middle",
    },
  });
  return [
    tot("ВСЬОГО", "left"),
    tot(thousands(bt.total)), tot(formatChanges(bt)),
    tot(thousands(st.total)), tot(formatChanges(st)),
    tot(thousands(at.total)), tot(formatChanges(at)),
  ];
}

const TABLE_OPTS: Partial<PptxGenJS.TableProps> = {
  w: CW,
  colW: [4.03, 1.60, 1.10, 1.60, 1.10, 1.60, 1.40],
  rowH: 0.34,
  fontFace: FONT.body,
  fontSize: 9,
  color: C.text,
  valign: "middle",
  border: { type: "solid", pt: 0.75, color: C.border },
  margin: [0.03, 0.08, 0.03, 0.08],
  autoPage: false,
};

// ---------------------------------------------------------------------------
// Corps slides — auto-paginated
// ---------------------------------------------------------------------------

function corpsSlides(
  pptx: PptxGenJS,
  corps: CorpsData,
  sectionNum: number,
  emblemData: string | null,
): number {
  // Filter out units where everything is zero
  const activeUnits = corps.units.filter(unitHasData);
  if (activeUnits.length === 0) return 0;

  const bt = sumKind(activeUnits, "bzvp");
  const st = sumKind(activeUnits, "special");
  const at = sumKind(activeUnits, "adaptation");
  const allT = bt.total + st.total + at.total;
  const allY = corps.yesterday_bzvp + corps.yesterday_special + corps.yesterday_adaptation;
  const totalRow = makeTotalRow(bt, st, at);
  const headerRow = makeHeaderRow();

  const pages: UnitData[][] = [];
  for (let i = 0; i < activeUnits.length; i += MAX_TABLE_ROWS) {
    pages.push(activeUnits.slice(i, i + MAX_TABLE_ROWS));
  }
  if (pages.length === 0) pages.push([]);

  let slidesCreated = 0;

  pages.forEach((pageUnits, pageIdx) => {
    const isFirst = pageIdx === 0;
    const isLast = pageIdx === pages.length - 1;
    const slide = pptx.addSlide({ masterName: "TAKTOBLIK" });
    footer(slide);
    if (emblemData) emblem(slide, emblemData, "content");
    slidesCreated++;

    const numStr = pad2(sectionNum);

    if (isFirst) {
      sectionHeader(slide, pptx, numStr, corps.label.toUpperCase(),
        `Загалом залучено: ${thousands(allT)} о/с  ·  Зміна за добу: ${signedDelta(allT - allY)}`);

      // Mini KPI tiles
      const mtW = (CW - 0.20 * 3) / 4;
      const mtH = 0.95;
      const mtY = 1.60;
      const mtGap = 0.20;

      const miniData: [string, number, number][] = [
        ["БЗВП", bt.total, bt.total - corps.yesterday_bzvp],
        ["ФАХОВА", st.total, st.total - corps.yesterday_special],
        ["АДАПТАЦІЯ", at.total, at.total - corps.yesterday_adaptation],
        ["ЗАГАЛОМ", allT, allT - allY],
      ];
      miniData.forEach(([label, val, delta], i) => {
        kpiTileSmall(slide, pptx, M + i * (mtW + mtGap), mtY, mtW, mtH,
          label, thousands(val), `${signedDelta(delta)}`,
          label === "ЗАГАЛОМ" ? C.accent : C.text);
      });

      const TABLE_TOP = 2.75;
      const startIdx = 0;
      const rows: PptxGenJS.TableRow[] = [headerRow];
      pageUnits.forEach((u, i) => rows.push(makeDataRow(u, startIdx + i)));
      if (isLast) rows.push(totalRow);
      slide.addTable(rows, { ...TABLE_OPTS, x: M, y: TABLE_TOP } as PptxGenJS.TableProps);

      if (isLast) {
        const tableBottom = TABLE_TOP + 0.34 * rows.length + 0.12;
        if (tableBottom < 6.2) {
          const finTotal = bt.finishing + st.finishing + at.finishing;
          const startTotal = bt.started + st.started + at.started;
          calloutBar(slide, pptx, tableBottom, corps.label.toUpperCase(),
            `Залучено ${thousands(allT)}  ·  Завершують ${thousands(finTotal)}  ·  Розпочинають +${thousands(startTotal)}`);
        }
      }
    } else {
      sectionHeader(slide, pptx, numStr, `${corps.label.toUpperCase()} (продовження)`,
        `Сторінка ${pageIdx + 1} з ${pages.length}`);

      const TABLE_TOP = 1.70;
      const startIdx = pageIdx * MAX_TABLE_ROWS;
      const rows: PptxGenJS.TableRow[] = [headerRow];
      pageUnits.forEach((u, i) => rows.push(makeDataRow(u, startIdx + i)));
      if (isLast) rows.push(totalRow);
      slide.addTable(rows, { ...TABLE_OPTS, x: M, y: TABLE_TOP } as PptxGenJS.TableProps);

      if (isLast) {
        const tableBottom = TABLE_TOP + 0.34 * rows.length + 0.12;
        if (tableBottom < 6.2) {
          const finTotal = bt.finishing + st.finishing + at.finishing;
          const startTotal = bt.started + st.started + at.started;
          calloutBar(slide, pptx, tableBottom, corps.label.toUpperCase(),
            `Залучено ${thousands(allT)}  ·  Завершують ${thousands(finTotal)}  ·  Розпочинають +${thousands(startTotal)}`);
        }
      }
    }
  });

  return slidesCreated;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function buildD4Pptx(data: D4Data): Promise<void> {
  const Pptx = await getPptxGenJS();
  const pptx = new Pptx();
  pptx.layout = "LAYOUT_WIDE";
  pptx.author = "Taktoblik";
  pptx.title = `Підготовка ${parseDate(data.date)}`;
  pptx.theme = { headFontFace: FONT.head, bodyFontFace: FONT.body };

  // Dark background master (matching reference presentations)
  pptx.defineSlideMaster({
    title: "TAKTOBLIK",
    background: { color: C.bg },
  });

  const emblemData = await loadEmblemBase64();

  titleSlide(pptx, data.date, emblemData);
  overviewSlide(pptx, data, emblemData);

  let sectionNum = 2;
  for (const corps of data.corps) {
    const created = corpsSlides(pptx, corps, sectionNum, emblemData);
    if (created > 0) sectionNum++;
  }

  await pptx.writeFile({ fileName: `D4_Підготовка_${data.date}.pptx` });
}
