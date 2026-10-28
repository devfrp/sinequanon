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
}
