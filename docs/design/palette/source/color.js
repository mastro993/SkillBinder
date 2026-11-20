/* Color math: OKLCH to sRGB hex and back, mixing, and WCAG contrast. */
const clamp = (x, a, b) => Math.min(b, Math.max(a, x));
const lin = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const gam = (c) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);
function oklchToRgb(L, C, H) {
  const a = C * Math.cos((H * Math.PI) / 180), b = C * Math.sin((H * Math.PI) / 180);
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ].map(gam);
}
function ok(L, C, H) {
  // Lower chroma until the color fits sRGB, so hue and lightness survive.
  L = clamp(L, 0, 1);
  for (let c = C; c >= 0; c -= 0.002) {
    const rgb = oklchToRgb(L, c, H);
    if (rgb.every((v) => v >= -0.0005 && v <= 1.0005)) return toHex(rgb);
  }
  return toHex(oklchToRgb(L, 0, H));
}
const toHex = (rgb) => '#' + rgb.map((v) => Math.round(clamp(v, 0, 1) * 255).toString(16).padStart(2, '0')).join('').toUpperCase();
const fromHex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
function hexToOklch(hex) {
  const [r, g, b] = fromHex(hex).map(lin);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return [L, Math.hypot(A, B), ((Math.atan2(B, A) * 180) / Math.PI + 360) % 360];
}
const mix = (a, b, t) => toHex(fromHex(a).map((v, i) => v * (1 - t) + fromHex(b)[i] * t));
function luminance(hex) { const [r, g, b] = fromHex(hex).map(lin); return 0.2126 * r + 0.7152 * g + 0.0722 * b; }
function contrast(a, b) { const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); }
