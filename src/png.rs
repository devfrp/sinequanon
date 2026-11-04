//! Raster logicielle (RGBA) et encodeur PNG minimal — 100 % Rust, sans
//! dépendance externe : les données IDAT utilisent des blocs deflate
//! « stored » (valides, non compressés).

use std::path::Path;
use std::sync::OnceLock;

/// Petite image RGBA dessinable.
#[derive(Clone)]
pub struct Raster {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>, // RGBA
}

/// Segment découpé dans un rectangle (algorithme de Liang-Barsky).
#[allow(clippy::too_many_arguments)]
pub fn clip_segment(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> Option<(f64, f64, f64, f64)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let p = [-dx, dx, -dy, dy];
    let q = [x0 - minx, maxx - x0, y0 - miny, maxy - y0];
    for i in 0..4 {
        if p[i] == 0.0 {
            if q[i] < 0.0 {
                return None;
            }
            continue;
        }
        let r = q[i] / p[i];
        if p[i] < 0.0 {
            if r > t1 {
                return None;
            }
            if r > t0 {
                t0 = r;
            }
        } else {
            if r < t0 {
                return None;
            }
            if r < t1 {
                t1 = r;
            }
        }
    }
    Some((x0 + t0 * dx, y0 + t0 * dy, x0 + t1 * dx, y0 + t1 * dy))
}

impl Raster {
    pub fn new(w: usize, h: usize, bg: [u8; 4]) -> Self {
        let mut px = vec![0u8; w * h * 4];
        for ch in px.chunks_exact_mut(4) {
            ch.copy_from_slice(&bg);
        }
        Self { w, h, px }
    }

    pub fn set(&mut self, x: i64, y: i64, c: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.w as i64 || y >= self.h as i64 {
            return;
        }
        let i = ((y as usize) * self.w + x as usize) * 4;
        let a = c[3] as u16;
        for (k, &cv) in c[..3].iter().enumerate() {
            let dst = self.px[i + k] as u16;
            self.px[i + k] = ((cv as u16 * a + dst * (255 - a)) / 255) as u8;
        }
        self.px[i + 3] = 255;
    }

    /// Trace un segment (coordonnées px), épaisseur en px, découpé aux
    /// limites du raster.
    pub fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: [u8; 4], thickness: f64) {
        let Some((x0, y0, x1, y1)) =
            clip_segment(x0, y0, x1, y1, -1.0, -1.0, self.w as f64, self.h as f64)
        else {
            return;
        };
        let thickness = thickness.max(1.0);
        if thickness <= 1.5 {
            bresenham(
                self,
                x0.round() as i64,
                y0.round() as i64,
                x1.round() as i64,
                y1.round() as i64,
                c,
            );
        } else {
            let r = (thickness / 2.0).ceil() as i64;
            let r2 = (thickness / 2.0).powi(2) + 0.25;
            let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
            let steps = (len * 2.0).ceil().max(1.0) as usize;
            for s in 0..=steps {
                let t = s as f64 / steps as f64;
                let cx = (x0 + (x1 - x0) * t).round() as i64;
                let cy = (y0 + (y1 - y0) * t).round() as i64;
                for dy in -r..=r {
                    for dx in -r..=r {
                        if (dx * dx + dy * dy) as f64 <= r2 {
                            self.set(cx + dx, cy + dy, c);
                        }
                    }
                }
            }
        }
    }

    /// Bitmap Windows 24 bits (rangées du bas vers le haut, BGR) —
    /// format d'export de l'original (man. p. 100).
    pub fn write_bmp(&self, path: &Path) -> std::io::Result<()> {
        let w = self.w as u32;
        let h = self.h as u32;
        let row_len = (w * 3).div_ceil(4) * 4;
        let data_size = row_len * h;
        let total = 14u32 + 40 + data_size;
        let mut out = Vec::with_capacity(total as usize);
        // BITMAPFILEHEADER (octets 0..14)
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&total.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        // BITMAPINFOHEADER (40 octets)
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&data_size.to_le_bytes());
        out.extend_from_slice(&3780u32.to_le_bytes()); // 96 dpi
        out.extend_from_slice(&3780u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        // pixels BGR, rangée par rangée, du bas vers le haut + padding
        let roww = (self.w * 3) as u32;
        for y in (0..self.h).rev() {
            for x in 0..self.w {
                let i = (y * self.w + x) * 4;
                out.push(self.px[i + 2]);
                out.push(self.px[i + 1]);
                out.push(self.px[i]);
            }
            out.resize(out.len() + (row_len - roww) as usize, 0);
        }
        std::fs::write(path, out)
    }

    /// GIF89a 256 couleurs (palette 6-6-6 + 32 gris) avec LZW —
    /// format d'export de l'original (man. p. 100).
    pub fn write_gif(&self, path: &Path) -> std::io::Result<()> {
        // palette : 216 couleurs 6-6-6 puis 32 niveaux de gris
        let mut palette: Vec<[u8; 3]> = Vec::with_capacity(256);
        for r in 0..6u32 {
            for g in 0..6u32 {
                for b in 0..6u32 {
                    palette.push([(r * 51) as u8, (g * 51) as u8, (b * 51) as u8]);
                }
            }
        }
        for i in 0..32u32 {
            let g = (i * 8).clamp(0, 255) as u8;
            palette.push([g, g, g]);
        }
        palette.resize(256, [0, 0, 0]);
        // table de correspondance : clé 3-3-2 → indice le plus proche
        let mut lut = vec![0u16; 512];
        for (k, slot) in lut.iter_mut().enumerate() {
            let rr = (((k >> 6) & 7) * 255 / 7) as i32;
            let gg = (((k >> 3) & 7) * 255 / 7) as i32;
            let bb = ((k & 7) * 255 / 3) as i32;
            let (mut best, mut bd) = (0usize, u32::MAX);
            for (j, c) in palette.iter().enumerate() {
                let dr = rr - c[0] as i32;
                let dg = gg - c[1] as i32;
                let db = bb - c[2] as i32;
                let d = (dr * dr + dg * dg + db * db) as u32;
                if d < bd {
                    bd = d;
                    best = j;
                }
            }
            *slot = best as u16;
        }
        // clé 3-3-2 : 3 bits de rouge, 3 de vert, 2 (poids forts) de bleu
        let nearest = |r: u8, g: u8, b: u8| {
            lut[(((r as u16) >> 5) << 6) as usize
                | (((g as u16) >> 5) << 3) as usize
                | ((b as u16) >> 6) as usize]
        };
        // indices de palette des pixels
        let mut indexes = Vec::with_capacity(self.w * self.h);
        for px in self.px.chunks_exact(4) {
            indexes.push(nearest(px[0], px[1], px[2]));
        }
        // compression LZW GIF (codes de longueur variable, petit-boutiste)
        let mut data = Vec::new();
        let mut bitbuf: u32 = 0;
        let mut bitcnt = 0u32;
        let mut bytes = Vec::new();
        let mut write_code = |code: u16, size: u32| {
            bitbuf |= (code as u32) << bitcnt;
            bitcnt += size;
            while bitcnt >= 8 {
                bytes.push((bitbuf & 0xff) as u8);
                bitbuf >>= 8;
                bitcnt -= 8;
            }
        };
        let clear: u16 = 256;
        let fin: u16 = 257;
        let mut dict: std::collections::HashMap<(u16, u16), u16> =
            std::collections::HashMap::with_capacity(4096);
        let mut code_size = 9u32;
        let mut next_code = 258u16;
        write_code(clear, code_size);
        if !indexes.is_empty() {
            let mut cur = indexes[0];
            for &v in indexes.iter().skip(1) {
                if let Some(&c) = dict.get(&(cur, v)) {
                    cur = c;
                } else {
                    write_code(cur, code_size);
                    if next_code < 4096 {
                        dict.insert((cur, v), next_code);
                        next_code += 1;
                        if next_code == (1u16 << code_size) + 1 && code_size < 12 {
                            code_size += 1;
                        }
                    } else {
                        write_code(clear, code_size);
                        dict.clear();
                        code_size = 9;
                        next_code = 258;
                    }
                    cur = v;
                }
            }
            write_code(cur, code_size);
        }
        write_code(fin, code_size);
        if bitcnt > 0 {
            bytes.push((bitbuf & 0xff) as u8);
        }
        // blocs de données de ≤ 255 octets
        data.push(8u8); // taille LZW minimale
        for c in bytes.chunks(255) {
            data.push(c.len() as u8);
            data.extend_from_slice(c);
        }
        data.push(0); // fin des blocs
                      // entête GIF89a
        let mut out = Vec::new();
        out.extend_from_slice(b"GIF89a");
        out.extend_from_slice(&(self.w as u16).to_le_bytes());
        out.extend_from_slice(&(self.h as u16).to_le_bytes());
        out.push(0xf7); // table de couleurs globale sur 256 valeurs
        out.push(0);
        out.push(0);
        for c in &palette {
            out.extend_from_slice(c);
        }
        out.push(b','); // descripteur d'image
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(self.w as u16).to_le_bytes());
        out.extend_from_slice(&(self.h as u16).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&data);
        out.push(b';'); // fin du GIF
        std::fs::write(path, out)
    }

    /// Convertit en RGB et écrit un fichier PNG.
    pub fn write_png(&self, path: &Path) -> std::io::Result<()> {
        let mut rgb = Vec::with_capacity(self.w * self.h * 3);
        for px in self.px.chunks_exact(4) {
            rgb.extend_from_slice(&px[..3]);
        }
        let mut out = Vec::with_capacity(rgb.len() / 2 + 1024);
        encode_png(self.w as u32, self.h as u32, &rgb, &mut out);
        std::fs::write(path, out)
    }
}

fn bresenham(r: &mut Raster, x0: i64, y0: i64, x1: i64, y1: i64, c: [u8; 4]) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    loop {
        r.set(x, y, c);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn crc32(data: &[u8]) -> u32 {
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    let table = *TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *e = c;
        }
        t
    });
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Flux zlib avec blocs deflate « stored » (non compressés) : en-tête
/// 0x78 0x01 (CMF/FLG), blocs fragmentés à 65 535 octets max et somme
/// adler-32 finale. Réutilisée par le PNG (IDAT) et le PDF (FlateDecode).
pub(crate) fn zlib_store(raw: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78u8, 0x01u8];
    let mut pos = 0usize;
    while pos < raw.len() {
        let n = (raw.len() - pos).min(65535);
        let bfinal = if pos + n >= raw.len() { 1u8 } else { 0u8 };
        out.push(bfinal);
        out.extend_from_slice(&(n as u16).to_le_bytes());
        out.extend_from_slice(&(!(n as u16)).to_le_bytes());
        out.extend_from_slice(&raw[pos..pos + n]);
        pos += n;
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut body = Vec::with_capacity(4 + data.len());
    body.extend_from_slice(tag);
    body.extend_from_slice(data);
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
}

/// Encode une image RGB 8 bits (3 octets par pixel) en PNG.
pub fn encode_png(w: u32, h: u32, rgb: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB, pas d'entrelacement
    chunk(out, b"IHDR", &ihdr);

    // scanlines avec filtre 0
    let stride = w as usize * 3 + 1;
    let mut raw = Vec::with_capacity(stride * h as usize);
    for y in 0..h as usize {
        raw.push(0u8);
        let start = y * w as usize * 3;
        raw.extend_from_slice(&rgb[start..start + w as usize * 3]);
    }
    // flux zlib : en-tête + blocs deflate « stored »
    chunk(out, b"IDAT", &zlib_store(&raw));
    chunk(out, b"IEND", &[]);
}

// --- police bitmap 5×7 -------------------------------------------------------

/// Un glyphe : 7 lignes de 5 pixels (bit 4 = colonne de gauche).
pub type Glyphe = [u8; 7];

/// Table des glyphes : chiffres, lettres, ponctuation et les symboles
/// mathématiques utilisés dans les tableaux (∞ √ π θ ′ − ≤ ≥ ×).
const GLYPHES: &[(char, Glyphe)] = &[
    (' ', [0x00; 7]),
    ('!', [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04]),
    ('"', [0x0A, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00]),
    ('\'', [0x04, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00]),
    ('(', [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02]),
    (')', [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08]),
    ('*', [0x00, 0x0A, 0x04, 0x1F, 0x04, 0x0A, 0x00]),
    ('+', [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00]),
    (',', [0x00, 0x00, 0x00, 0x00, 0x0C, 0x04, 0x08]),
    ('-', [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00]),
    ('.', [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C]),
    ('/', [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10]),
    ('0', [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    ('1', [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E]),
    ('2', [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F]),
    ('3', [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E]),
    ('4', [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02]),
    ('5', [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E]),
    ('6', [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E]),
    ('7', [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08]),
    ('8', [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E]),
    ('9', [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C]),
    (':', [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x0C, 0x00]),
    (';', [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x04, 0x08]),
    ('<', [0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x02]),
    ('=', [0x00, 0x00, 0x1F, 0x00, 0x1F, 0x00, 0x00]),
    ('>', [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x08]),
    ('?', [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04]),
    ('A', [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
    ('B', [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E]),
    ('C', [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E]),
    ('D', [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E]),
    ('E', [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F]),
    ('F', [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10]),
    ('G', [0x0E, 0x11, 0x10, 0x13, 0x11, 0x11, 0x0E]),
    ('H', [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11]),
    ('I', [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E]),
    ('J', [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C]),
    ('K', [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11]),
    ('L', [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F]),
    ('M', [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11]),
    ('N', [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11]),
    ('O', [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    ('P', [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10]),
    ('Q', [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D]),
    ('R', [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11]),
    ('S', [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E]),
    ('T', [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
    ('U', [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    ('V', [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04]),
    ('W', [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A]),
    ('X', [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11]),
    ('Y', [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04]),
    ('Z', [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F]),
    ('[', [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E]),
    ('\\', [0x10, 0x10, 0x08, 0x04, 0x02, 0x01, 0x01]),
    (']', [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E]),
    ('^', [0x04, 0x0A, 0x00, 0x00, 0x00, 0x00, 0x00]),
    ('_', [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F]),
    ('`', [0x04, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00]),
    ('a', [0x00, 0x00, 0x0E, 0x01, 0x0F, 0x11, 0x0F]),
    ('b', [0x10, 0x10, 0x1E, 0x11, 0x11, 0x11, 0x1E]),
    ('c', [0x00, 0x00, 0x0F, 0x10, 0x10, 0x10, 0x0F]),
    ('d', [0x01, 0x01, 0x0F, 0x11, 0x11, 0x11, 0x0F]),
    ('e', [0x00, 0x00, 0x0E, 0x11, 0x1F, 0x10, 0x0E]),
    ('f', [0x0C, 0x04, 0x1E, 0x04, 0x04, 0x04, 0x04]),
    ('g', [0x00, 0x0E, 0x11, 0x11, 0x0E, 0x01, 0x0E]),
    ('h', [0x10, 0x10, 0x1F, 0x11, 0x11, 0x11, 0x11]),
    ('i', [0x04, 0x00, 0x04, 0x04, 0x04, 0x04, 0x04]),
    ('j', [0x01, 0x00, 0x01, 0x01, 0x01, 0x09, 0x06]),
    ('k', [0x10, 0x10, 0x14, 0x18, 0x14, 0x12, 0x11]),
    ('l', [0x0C, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E]),
    ('m', [0x00, 0x00, 0x1B, 0x15, 0x15, 0x15, 0x15]),
    ('n', [0x00, 0x00, 0x1F, 0x11, 0x11, 0x11, 0x11]),
    ('o', [0x00, 0x00, 0x0E, 0x11, 0x11, 0x11, 0x0E]),
    ('p', [0x00, 0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10]),
    ('q', [0x00, 0x0F, 0x11, 0x11, 0x0F, 0x01, 0x01]),
    ('r', [0x00, 0x00, 0x1C, 0x12, 0x10, 0x10, 0x10]),
    ('s', [0x00, 0x00, 0x0F, 0x10, 0x0E, 0x01, 0x1E]),
    ('t', [0x08, 0x08, 0x1E, 0x08, 0x08, 0x09, 0x06]),
    ('u', [0x00, 0x00, 0x11, 0x11, 0x11, 0x11, 0x0E]),
    ('v', [0x00, 0x00, 0x11, 0x11, 0x11, 0x0A, 0x04]),
    ('w', [0x00, 0x00, 0x11, 0x11, 0x15, 0x15, 0x0A]),
    ('x', [0x00, 0x00, 0x11, 0x0A, 0x04, 0x0A, 0x11]),
    ('y', [0x00, 0x00, 0x11, 0x11, 0x0A, 0x04, 0x08]),
    ('z', [0x00, 0x00, 0x1F, 0x02, 0x04, 0x08, 0x1F]),
    ('{', [0x02, 0x04, 0x04, 0x08, 0x04, 0x04, 0x02]),
    ('|', [0x04, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
    ('}', [0x08, 0x04, 0x04, 0x02, 0x04, 0x04, 0x08]),
    ('−', [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00]),
    ('≤', [0x02, 0x04, 0x08, 0x10, 0x08, 0x04, 0x0E]),
    ('≥', [0x08, 0x04, 0x02, 0x01, 0x02, 0x04, 0x0E]),
    ('∞', [0x00, 0x00, 0x0A, 0x15, 0x15, 0x15, 0x0A]),
    ('√', [0x03, 0x04, 0x04, 0x08, 0x18, 0x10, 0x10]),
    ('π', [0x00, 0x00, 0x0E, 0x15, 0x15, 0x15, 0x00]),
    ('θ', [0x00, 0x0E, 0x11, 0x1F, 0x11, 0x0E, 0x00]),
    ('′', [0x02, 0x04, 0x08, 0x00, 0x00, 0x00, 0x00]),
    ('×', [0x00, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x00]),
    ('î', [0x0A, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
    ('ï', [0x0A, 0x0A, 0x04, 0x04, 0x04, 0x04, 0x04]),
    ('ç', [0x00, 0x00, 0x0F, 0x10, 0x10, 0x0F, 0x04]),
    // Ω (univers des possibles, arbres de probabilités)
    ('Ω', [0x0E, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x1F]),
];

/// Lettres accentuées : (caractère, base, 2 lignes d'accent au-dessus).
const ACCENTS: &[(char, char, [u8; 2])] = &[
    ('é', 'e', [0x02, 0x04]),
    ('è', 'e', [0x04, 0x02]),
    ('ê', 'e', [0x0A, 0x04]),
    ('à', 'a', [0x04, 0x02]),
    ('â', 'a', [0x0A, 0x04]),
    ('ù', 'u', [0x04, 0x02]),
    ('û', 'u', [0x0A, 0x04]),
    ('ô', 'o', [0x0A, 0x04]),
];

/// Cherche le glyphe d'un caractère.
pub fn glyphe(ch: char) -> Option<Glyphe> {
    GLYPHES.iter().find(|(c, _)| *c == ch).map(|(_, g)| *g)
}

/// Côté d'un pixel de glyphe (échelle entière) pour une hauteur de casse.
fn echelle(size_px: f64) -> f64 {
    (size_px / 7.0).round().max(1.0)
}

/// Largeur d'un texte en pixels (avance 6 px par caractère à l'échelle 1),
/// utilisée pour l'ancrage des textes statistiques.
pub fn texte_largeur_px(s: &str, size_px: f64) -> f64 {
    s.chars().count() as f64 * 6.0 * echelle(size_px)
}

/// Peint les bits d'une ligne de glyphe (dx = cisaillement italique).
fn paint_bits(r: &mut Raster, x: f64, y: f64, u: f64, bits: u8, dx: f64, c: [u8; 4]) {
    for col in 0..5u8 {
        if bits & (1 << (4 - col)) != 0 {
            r.rect(x + col as f64 * u + dx, y, u, u, c);
        }
    }
}

/// Solide avec tirets : trait pointillé pour les équipotentielles.
pub fn dashed(r: &mut Raster, x0: f64, y0: f64, x1: f64, y1: f64, c: [u8; 4], thickness: f64) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let long = (dx * dx + dy * dy).sqrt();
    if long < 1.0 {
        return;
    }
    let px = dx / long;
    let py = dy / long;
    // 4 px pleins, 3 px vides à l'échelle du raster
    let pas = 7.0f64.max(2.0 * (thickness + 3.0));
    let mut t = 0.0;
    while t < long {
        let t2 = (t + pas * 0.57).min(long);
        r.line(
            x0 + px * t,
            y0 + py * t,
            x0 + px * t2,
            y0 + py * t2,
            c,
            thickness,
        );
        t += pas;
    }
}

/// Remplit un polygone (par balayage, règle pair-impair).
pub fn poly(r: &mut Raster, pts: &[(f64, f64)], c: [u8; 4]) {
    if pts.len() < 3 {
        return;
    }
    let ymin = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let ymax = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let mut xs: Vec<f64> = Vec::new();
    let y0 = ymin.floor() as i64;
    let y1 = (ymax + 1.0).ceil() as i64;
    for y in y0..y1 {
        let yc = y as f64 + 0.5;
        xs.clear();
        for seg in pts.windows(2) {
            let (xa, ya) = seg[0];
            let (xb, yb) = seg[1];
            if (ya <= yc && yb > yc) || (yb <= yc && ya > yc) {
                xs.push(xa + (yc - ya) * (xb - xa) / (yb - ya));
            }
        }
        if xs.len() < 2 {
            continue;
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut k = 0;
        while k + 1 < xs.len() {
            r.line(xs[k], yc, xs[k + 1], yc, c, 1.0);
            k += 2;
        }
    }
}

/// Disque plein, sous forme de polygone 24 faces (suffisant à cette échelle).
pub fn disque(r: &mut Raster, cx: f64, cy: f64, ray: f64, c: [u8; 4]) {
    let n = 24;
    let mut pts = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let a = std::f64::consts::TAU * i as f64 / n as f64;
        pts.push((cx + ray * a.cos(), cy + ray * a.sin()));
    }
    poly(r, &pts, c);
}

/// Texte vertical (rotation +90°, bas vers le haut) pour les étiquettes
/// des modalités d'un histogramme qualitatif.
pub fn draw_text_vertical(r: &mut Raster, x: f64, y: f64, txt: &str, size_px: f64, c: [u8; 4]) {
    // la police avance de 6 px par caractère à l'échelle 1 ; on écrit
    // chaque colonne du glyphe tournée en ligne verticale
    let u = echelle(size_px);
    let mut avance = 0.0;
    for ch in txt.chars() {
        if let Some(g) = glyphe(ch) {
            for (bit, &ligne) in g.iter().enumerate() {
                for col in 0..5u8 {
                    if ligne & (1 << (4 - col)) != 0 {
                        // bas du glyphe = ligne 6, haut = ligne 0 ;
                        // colonne 4 (droite) devient le haut de la lettre
                        r.rect(
                            x + (6 - bit) as f64 * u - 1.0,
                            y - avance - (4 - col) as f64 * u,
                            u,
                            u,
                            c,
                        );
                    }
                }
            }
        }
        avance += 6.0 * u;
    }
}

impl Raster {
    /// Remplit un rectangle (coordonnées px, arrondies au pixel).
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, c: [u8; 4]) {
        let x0 = x.floor() as i64;
        let y0 = y.floor() as i64;
        let x1 = (x + w).ceil() as i64;
        let y1 = (y + h).ceil() as i64;
        for py in y0..y1 {
            for px in x0..x1 {
                self.set(px, py, c);
            }
        }
    }

    /// Ligne pointillée : tirets alternés (mm), pour la grille des
    /// statistiques. `pas_mm` = tiret+trou ; longueur du tiret = moitié.
    #[allow(clippy::too_many_arguments)]
    pub fn dash_line(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        c: [u8; 4],
        w_px: f64,
        pas_mm: f64,
        ppm: f64,
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= 0.0 {
            return;
        }
        let n = (len / pas_mm).ceil() as i64 + 1;
        for i in 0..n {
            let t0 = (i as f64 * pas_mm / len).min(1.0);
            let t1 = ((i as f64 * pas_mm + pas_mm * 0.5) / len).min(1.0);
            if t1 <= t0 {
                continue;
            }
            self.line(
                (x0 + t0 * dx) * ppm,
                (y0 + t0 * dy) * ppm,
                (x0 + t1 * dx) * ppm,
                (y0 + t1 * dy) * ppm,
                c,
                w_px,
            );
        }
    }

    /// Dessine une chaîne avec la police bitmap 5×7. `(x, y)` = ligne de
    /// base du premier caractère (px), `size_px` = hauteur de casse.
    /// Renvoie l'abscisse de fin. Les caractères sans glyphe avancent
    /// d'une cellule sans être dessinés.
    pub fn draw_text(
        &mut self,
        x: f64,
        y: f64,
        s: &str,
        size_px: f64,
        c: [u8; 4],
        italique: bool,
    ) -> f64 {
        let u = echelle(size_px);
        let mut cx = x;
        for ch in s.chars() {
            if ch == '\u{305}' {
                // marque combinante U+0305 : surligne (barre au-dessus
                // de la casse) sur la cellule précédente, sans avance
                let (bx, by) = (cx - 6.0 * u, y - 9.0 * u);
                self.rect(bx, by, 6.0 * u, 2.0 * u, c);
                continue;
            }
            let (base, acc) = match ACCENTS.iter().find(|(a, _, _)| *a == ch) {
                Some((_, b, acc)) => (*b, Some(*acc)),
                None => (ch, None),
            };
            if let Some(g) = glyphe(base) {
                for (r, &bits) in g.iter().enumerate() {
                    let dy = y - 7.0 * u + r as f64 * u;
                    let dx = if italique {
                        (6 - r) as f64 * 0.22 * u
                    } else {
                        0.0
                    };
                    paint_bits(self, cx, dy, u, bits, dx, c);
                }
                if let Some(acc) = acc {
                    for (r, &bits) in acc.iter().enumerate() {
                        let dy = y - 7.0 * u + r as f64 * u;
                        let dx = if italique {
                            (6 - r) as f64 * 0.22 * u
                        } else {
                            0.0
                        };
                        paint_bits(self, cx, dy, u, bits, dx, c);
                    }
                }
            }
            cx += 6.0 * u;
        }
        cx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmp_24_bits_aller_retour() {
        let dir = std::env::temp_dir().join("sqn-test-bmp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.bmp");
        let mut r = Raster::new(4, 3, [255, 255, 255, 255]);
        r.set(0, 0, [255, 0, 0, 255]); // pixel haut-gauche rouge
        r.set(3, 2, [0, 0, 255, 255]); // pixel bas-droite bleu
        r.write_bmp(&path).unwrap();
        let data = std::fs::read(&path).unwrap();
        assert_eq!(&data[..2], b"BM");
        // 14 octets d'entête fichier + 40 d'entête DIB
        // + 3 rangées de 4 px × 3 octets (multiple de 4, padding nul)
        let expected = 14 + 40 + 3 * 12;
        assert_eq!(data.len(), expected);
        // rangée du bas (y = 2) en premier : pixel (3,2) bleu → BGR [255,0,0]
        assert_eq!(&data[54 + 9..54 + 12], &[255, 0, 0]);
        // rangée du haut (y = 0) en dernier : pixel (0,0) rouge → BGR [0,0,255]
        assert_eq!(&data[54 + 2 * 12..54 + 2 * 12 + 3], &[0, 0, 255]);
    }

    #[test]
    fn gif_aller_retour_lzw() {
        let dir = std::env::temp_dir().join("sqn-test-gif");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.gif");
        // dégradé d'un rouge à l'autre : 8 pixels
        let mut r = Raster::new(8, 1, [255, 255, 255, 255]);
        for x in 0..8u8 {
            r.set(x as i64, 0, [x * 34, (7 - x) * 34, 0, 255]);
        }
        r.write_gif(&path).unwrap();
        let g = std::fs::read(&path).unwrap();
        assert_eq!(&g[..6], b"GIF89a");
        assert_eq!(&g[6..8], &8u16.to_le_bytes());
        assert_eq!(g[10], 0xf7); // table globale 256 couleurs
                                 // palette 256 couleurs
        let palette: Vec<[u8; 3]> = (0..256)
            .map(|k| [g[13 + 3 * k], g[14 + 3 * k], g[15 + 3 * k]])
            .collect();
        // données d'image : descripteur 10 octets après l'entête (13 + 768)
        let mut i = 13 + 768;
        assert_eq!(g[i], b',');
        i += 1 + 8 + 1; // ',' + 4 coordonnées u16 + 1 flag
        let min_size = g[i] as usize;
        assert_eq!(min_size, 8);
        i += 1;
        // blocs
        let mut lzw = Vec::new();
        while g[i] != 0 {
            let len = g[i] as usize;
            lzw.extend_from_slice(&g[i + 1..i + 1 + len]);
            i += 1 + len;
        }
        // décodeur LZW minimal
        let mut dict: Vec<Vec<u16>> = (0..256).map(|k| vec![k as u16]).collect();
        dict.push(vec![]); // 256 clair
        dict.push(vec![]); // 257 fin
        let mut bits = 0u64;
        let mut acc = 0u64;
        let mut code_size = 9u32;
        let mut out: Vec<u16> = Vec::new();
        let mut prev: Option<u16> = None;
        let mut pos = 0usize;
        while out.len() < 256 * 4 && pos < lzw.len() * 8 {
            acc |= ((lzw[pos / 8] >> (pos % 8)) as u64 & 1) << bits;
            bits += 1;
            pos += 1;
            if bits == code_size as u64 {
                let code = acc as u16;
                acc = 0;
                bits = 0;
                match code {
                    256 => {
                        dict.truncate(258);
                        code_size = 9;
                        prev = None;
                    }
                    257 => break,
                    c => {
                        let entry = if let Some(e) = dict.get(c as usize) {
                            e.clone()
                        } else {
                            let mut e = dict[prev.unwrap() as usize].clone();
                            e.push(e[0]);
                            e
                        };
                        if let Some(p) = prev {
                            let mut ne = dict[p as usize].clone();
                            ne.push(entry[0]);
                            let nlen = ne.len();
                            dict.push(ne);
                            if nlen >= (1usize << code_size) && code_size < 12 {
                                code_size += 1;
                            }
                        }
                        out.extend(entry);
                        prev = Some(code);
                    }
                }
            }
        }
        assert_eq!(out.len(), 8, "{out:?}");
        // chaque indice doit ramener la couleur 6-6-6 la plus proche :
        // le dégradé rouge croît et le vert décroît le long des pixels
        let rouge: Vec<i32> = out.iter().map(|&v| palette[v as usize][0] as i32).collect();
        let vert: Vec<i32> = out.iter().map(|&v| palette[v as usize][1] as i32).collect();
        assert!(rouge.windows(2).all(|w| w[0] <= w[1]), "{rouge:?}");
        assert!(vert.windows(2).all(|w| w[0] >= w[1]), "{vert:?}");
        assert_eq!(g[g.len() - 1], b';');
    }

    #[test]
    fn entete_png_valide() {
        let mut out = Vec::new();
        // 3×2 pixels rouges
        let mut rgb = vec![0u8; 3 * 3 * 2];
        for px in rgb.chunks_exact_mut(3) {
            px.copy_from_slice(&[255, 0, 0]);
        }
        encode_png(3, 2, &rgb, &mut out);
        assert_eq!(&out[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        // IHDR : longueur 13, puis dimensions
        assert_eq!(&out[8..12], &13u32.to_be_bytes());
        assert_eq!(&out[16..20], &3u32.to_be_bytes());
        assert_eq!(&out[20..24], &2u32.to_be_bytes());
    }

    #[test]
    fn crc32_connu() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn adler32_connu() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn clip_segment_basique() {
        // segment traversant tout le carré unité
        let c = clip_segment(-5.0, 0.5, 5.0, 0.5, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(c, Some((0.0, 0.5, 1.0, 0.5)));
        // segment dehors
        assert_eq!(clip_segment(-5.0, 2.0, 5.0, 2.0, 0.0, 0.0, 1.0, 1.0), None);
        // segment diagonal coupé aux deux bouts
        let c = clip_segment(-1.0, -1.0, 2.0, 2.0, 0.0, 0.0, 1.0, 1.0).unwrap();
        assert!((c.0 - 0.0).abs() < 1e-9 && (c.3 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn raster_line_epaisse() {
        let mut r = Raster::new(20, 20, [255, 255, 255, 255]);
        r.line(2.0, 10.0, 18.0, 10.0, [0, 0, 0, 255], 4.0);
        let center = r.px[(10 * 20 + 10) * 4];
        assert_eq!(center, 0);
        // hors de la ligne : blanc
        let above = r.px[(4 * 20 + 10) * 4];
        assert_eq!(above, 255);
    }

    #[test]
    fn police_couvre_les_symboles_des_tableaux() {
        for ch in "0123456789+−-().,/xf′gt∞√πθ≤≥=|".chars() {
            assert!(glyphe(ch).is_some(), "glyphe manquant : {ch:?}");
        }
        // accents composés
        for (a, base, _) in ACCENTS.iter().copied() {
            assert!(glyphe(base).is_some(), "base manquante pour {a:?}");
        }
        // caractères inconnus : pas de glyphe
        assert!(glyphe('@').is_none());
        assert!(glyphe('~').is_none());
    }

    #[test]
    fn draw_text_marque_des_pixels() {
        let mut r = Raster::new(80, 24, [255, 255, 255, 255]);
        let fin = r.draw_text(2.0, 20.0, "x+1", 14.0, [0, 0, 0, 255], false);
        assert!(fin > 20.0, "avance = {fin}");
        let noirs = r.px.chunks_exact(4).filter(|p| p[0] == 0).count();
        assert!(noirs >= 30, "pixels noirs = {noirs}");
        // au-dessus des glyphes : blanc
        assert_eq!(r.px[(40) * 4], 255);
        // italique : décalage horizontal des lignes hautes
        let mut r2 = Raster::new(80, 24, [255, 255, 255, 255]);
        r2.draw_text(2.0, 20.0, "x", 14.0, [0, 0, 0, 255], true);
        let noirs2 = r2.px.chunks_exact(4).filter(|p| p[0] == 0).count();
        assert!(noirs2 >= 5);
    }

    #[test]
    fn rect_remplit() {
        let mut r = Raster::new(10, 10, [255, 255, 255, 255]);
        r.rect(2.0, 2.0, 4.0, 3.0, [0, 0, 0, 255]);
        assert_eq!(r.px[(3 * 10 + 4) * 4], 0);
        assert_eq!(r.px[(3 * 10 + 7) * 4], 255);
    }
}
