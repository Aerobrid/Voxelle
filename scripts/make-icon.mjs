// Generates a 1024x1024 PNG source icon derived from the Voxelle logo
// (navy rounded square + white waveform bars). No external deps: hand-encodes PNG via node:zlib.
// Output feeds `tauri icon` to produce all platform sizes/formats. Original derivation, not a copy.

import { deflateSync, crc32 } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";

const S = 1024;
const RADIUS = 184;
const NAVY = [16, 20, 39, 255]; // #101427
const WHITE = [244, 246, 255, 255]; // #f4f6ff
const CLEAR = [0, 0, 0, 0];

// 5 bars, heights echo the logo's waveform.
const BAR_W = 96;
const GAP = 56;
const heights = [360, 560, 760, 560, 360];
const totalW = heights.length * BAR_W + (heights.length - 1) * GAP;
const startX = (S - totalW) / 2;
const bars = heights.map((h, i) => ({
  x0: startX + i * (BAR_W + GAP),
  x1: startX + i * (BAR_W + GAP) + BAR_W,
  y0: (S - h) / 2,
  y1: (S - h) / 2 + h,
  r: BAR_W / 2,
}));

function insideRoundedRect(x, y, x0, y0, x1, y1, r) {
  if (x < x0 || x > x1 || y < y0 || y > y1) return false;
  const cx = Math.min(Math.max(x, x0 + r), x1 - r);
  const cy = Math.min(Math.max(y, y0 + r), y1 - r);
  return (x - cx) ** 2 + (y - cy) ** 2 <= r * r;
}

const raw = Buffer.alloc(S * (S * 4 + 1)); // +1 filter byte per row
let p = 0;
for (let y = 0; y < S; y++) {
  raw[p++] = 0; // filter: none
  for (let x = 0; x < S; x++) {
    let c = CLEAR;
    if (insideRoundedRect(x + 0.5, y + 0.5, 0, 0, S, S, RADIUS)) {
      c = NAVY;
      for (const b of bars) {
        if (insideRoundedRect(x + 0.5, y + 0.5, b.x0, b.y0, b.x1, b.y1, b.r)) {
          c = WHITE;
          break;
        }
      }
    }
    raw[p++] = c[0];
    raw[p++] = c[1];
    raw[p++] = c[2];
    raw[p++] = c[3];
  }
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const body = Buffer.concat([typeBuf, data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body) >>> 0, 0);
  return Buffer.concat([len, body, crc]);
}

const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(S, 0);
ihdr.writeUInt32BE(S, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
const png = Buffer.concat([
  sig,
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

mkdirSync("assets/branding", { recursive: true });
const out = "assets/branding/voxelle-icon-1024.png";
writeFileSync(out, png);
console.log(`wrote ${out} (${png.length} bytes)`);
