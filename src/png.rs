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
    let mut idat = vec![0x78u8, 0x01u8];
    let mut pos = 0usize;
    while pos < raw.len() {
        let n = (raw.len() - pos).min(65535);
        let bfinal = if pos + n >= raw.len() { 1u8 } else { 0u8 };
        idat.push(bfinal);
        idat.extend_from_slice(&(n as u16).to_le_bytes());
        idat.extend_from_slice(&(!(n as u16)).to_le_bytes());
        idat.extend_from_slice(&raw[pos..pos + n]);
        pos += n;
    }
    idat.extend_from_slice(&adler32(&raw).to_be_bytes());
    chunk(out, b"IDAT", &idat);
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

/// Peint les bits d'une ligne de glyphe (dx = cisaillement italique).
fn paint_bits(r: &mut Raster, x: f64, y: f64, u: f64, bits: u8, dx: f64, c: [u8; 4]) {
    for col in 0..5u8 {
        if bits & (1 << (4 - col)) != 0 {
            r.rect(x + col as f64 * u + dx, y, u, u, c);
        }
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
