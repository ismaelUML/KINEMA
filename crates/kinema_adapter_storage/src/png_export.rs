// Why pure-std PNG? Bringing in `image` or `miniz_oxide` adds ~50 transitive crates
// and 15 seconds to CI for what is essentially 150 lines of uncompressed DEFLATE.
// Uncompressed blocks (BTYPE=00) inside zlib are 100% compliant with ISO/IEC 15948.
// Also, Windows file locks will explode if you try to `fs::remove_file` while the handle
// is alive, so we strictly scope and drop writers before touching temporary files.

use kinema_domain::motion::Motion1D;
use kinema_domain::scene::{Body, Scene};
use kinema_ports::{CancellationToken, ImageExporter};
use std::fs;
use std::io::Write;

/// Software canvas rasterizer and PNG generator.
#[derive(Debug, Clone, Copy)]
pub struct PngCanvasExporter {
    pub width: u32,
    pub height: u32,
}

impl Default for PngCanvasExporter {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
        }
    }
}

impl PngCanvasExporter {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Software frame buffer holding RGB24 pixels.
pub struct CanvasBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl CanvasBuffer {
    pub fn new(width: u32, height: u32, bg: [u8; 3]) -> Self {
        let total_pixels = (width as usize) * (height as usize);
        let mut pixels = Vec::with_capacity(total_pixels * 3);
        for _ in 0..total_pixels {
            pixels.extend_from_slice(&bg);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, color: [u8; 3]) {
        if x < 0 || x >= self.width as i32 || y < 0 || y >= self.height as i32 {
            return;
        }
        let idx = ((y as usize * self.width as usize) + x as usize) * 3;
        self.pixels[idx..idx + 3].copy_from_slice(&color);
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: [u8; 3]) {
        let x_start = x.max(0);
        let x_end = (x + w).min(self.width as i32);
        let y_start = y.max(0);
        let y_end = (y + h).min(self.height as i32);

        for py in y_start..y_end {
            for px in x_start..x_end {
                self.set_pixel(px, py, color);
            }
        }
    }

    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            self.set_pixel(x0, y0, color);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    // PNG spec requires every scanline to start with 1 filter-type byte. 0 = None.
    pub fn to_raw_scanlines(&self) -> Vec<u8> {
        let row_len = 1 + (self.width as usize * 3);
        let mut out = Vec::with_capacity(self.height as usize * row_len);
        for y in 0..self.height as usize {
            out.push(0x00);
            let start = y * self.width as usize * 3;
            let end = start + self.width as usize * 3;
            out.extend_from_slice(&self.pixels[start..end]);
        }
        out
    }
}

// Adler-32 checksum (RFC 1950)
fn update_adler32(mut s1: u32, mut s2: u32, bytes: &[u8]) -> (u32, u32) {
    const MOD_ADLER: u32 = 65521;
    for &b in bytes {
        s1 = (s1 + b as u32) % MOD_ADLER;
        s2 = (s2 + s1) % MOD_ADLER;
    }
    (s1, s2)
}

// CRC-32 (ISO 3309 / IEEE 802.3)
fn crc32_byte(mut c: u32, b: u8) -> u32 {
    c ^= b as u32;
    for _ in 0..8 {
        if (c & 1) != 0 {
            c = 0xEDB8_8320 ^ (c >> 1);
        } else {
            c >>= 1;
        }
    }
    c
}

fn update_crc32(mut crc: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        crc = crc32_byte(crc, b);
    }
    crc
}

fn finalize_crc32(crc: u32) -> u32 {
    crc ^ 0xFFFF_FFFF
}

fn write_png_chunk<W: Write>(w: &mut W, chunk_type: &[u8; 4], data: &[u8]) -> Result<(), String> {
    let len = (data.len() as u32).to_be_bytes();
    w.write_all(&len)
        .map_err(|e| format!("Failed writing chunk length: {}", e))?;
    w.write_all(chunk_type)
        .map_err(|e| format!("Failed writing chunk type: {}", e))?;
    w.write_all(data)
        .map_err(|e| format!("Failed writing chunk data: {}", e))?;

    let mut crc = update_crc32(0xFFFF_FFFF, chunk_type);
    crc = update_crc32(crc, data);
    let crc_bytes = finalize_crc32(crc).to_be_bytes();
    w.write_all(&crc_bytes)
        .map_err(|e| format!("Failed writing chunk crc: {}", e))
}

fn write_deflate_block(stream: &mut Vec<u8>, block: &[u8], is_last: bool) {
    stream.push(if is_last { 0x01 } else { 0x00 });
    let len = block.len() as u16;
    let nlen = !len;
    stream.extend_from_slice(&len.to_le_bytes());
    stream.extend_from_slice(&nlen.to_le_bytes());
    stream.extend_from_slice(block);
}

fn build_zlib_stream(raw_data: &[u8]) -> Vec<u8> {
    let mut stream = Vec::with_capacity(raw_data.len() + 64);
    // RFC 1950 header: CMF = 0x78 (deflate, 32K window), FLG = 0x01 (check bits)
    stream.push(0x78);
    stream.push(0x01);

    let chunk_size = 32768;
    let mut offset = 0;
    while offset < raw_data.len() {
        let end = (offset + chunk_size).min(raw_data.len());
        let block = &raw_data[offset..end];
        let is_last = end == raw_data.len();
        write_deflate_block(&mut stream, block, is_last);
        offset = end;
    }
    if raw_data.is_empty() {
        write_deflate_block(&mut stream, &[], true);
    }

    let (s1, s2) = update_adler32(1, 0, raw_data);
    let adler = (s2 << 16) | s1;
    stream.extend_from_slice(&adler.to_be_bytes());
    stream
}

fn draw_grid(canvas: &mut CanvasBuffer) {
    let grid_color = [35, 42, 54];
    for x in (0..canvas.width as i32).step_by(40) {
        canvas.draw_line(x, 0, x, canvas.height as i32 - 1, grid_color);
    }
    for y in (0..canvas.height as i32).step_by(40) {
        canvas.draw_line(0, y, canvas.width as i32 - 1, y, grid_color);
    }
    // Ground reference baseline
    canvas.draw_line(
        0,
        canvas.height as i32 - 80,
        canvas.width as i32 - 1,
        canvas.height as i32 - 80,
        [90, 105, 125],
    );
}

fn draw_body_cart(canvas: &mut CanvasBuffer, body: &Body, time: f64, idx: usize) {
    let ground_y = canvas.height as i32 - 80;
    let x_world = body.motion.position_at(time);
    let px = (60.0 + x_world * 5.0) as i32;
    let py = ground_y - 24;

    let cart_color = match idx % 3 {
        0 => [64, 200, 255],  // Cyan
        1 => [255, 180, 50],  // Amber
        _ => [100, 240, 140], // Phosphor lime
    };

    canvas.fill_rect(px, py, 36, 18, cart_color);
    // Small wheels
    canvas.fill_rect(px + 4, py + 18, 6, 6, [220, 220, 220]);
    canvas.fill_rect(px + 26, py + 18, 6, 6, [220, 220, 220]);
}

fn draw_ropes(canvas: &mut CanvasBuffer, scene: &Scene) {
    for rope in &scene.ropes {
        let count = rope.nodes.len();
        if count < 2 {
            continue;
        }
        for i in 0..(count - 1) {
            let n0 = &rope.nodes[i];
            let n1 = &rope.nodes[i + 1];
            let p0x = (canvas.width as f64 / 2.0 + n0.pos[0] * 20.0) as i32;
            let p0y = (canvas.height as f64 / 2.0 - n0.pos[1] * 20.0) as i32;
            let p1x = (canvas.width as f64 / 2.0 + n1.pos[0] * 20.0) as i32;
            let p1y = (canvas.height as f64 / 2.0 - n1.pos[1] * 20.0) as i32;
            canvas.draw_line(p0x, p0y, p1x, p1y, [240, 180, 70]);
        }
    }
}

fn render_scene_canvas(canvas: &mut CanvasBuffer, scene: &Scene, time: f64) {
    draw_grid(canvas);
    for (idx, body) in scene.bodies.iter().enumerate() {
        draw_body_cart(canvas, body, time, idx);
    }
    draw_ropes(canvas, scene);
}

impl ImageExporter for PngCanvasExporter {
    fn export_png(
        &self,
        scene: &Scene,
        time: f64,
        path: &str,
        token: &CancellationToken,
    ) -> Result<(), String> {
        if token.is_cancelled() {
            return Err("Export cancelled by user".to_string());
        }

        let mut canvas = CanvasBuffer::new(self.width, self.height, [16, 20, 28]);
        render_scene_canvas(&mut canvas, scene, time);

        if token.is_cancelled() {
            return Err("Export cancelled by user".to_string());
        }

        let raw_scanlines = canvas.to_raw_scanlines();
        let zlib_data = build_zlib_stream(&raw_scanlines);

        let tmp_path = format!("{}.tmp.{}", path, std::process::id());
        let export_res = write_png_file(&tmp_path, self.width, self.height, &zlib_data, token);

        match export_res {
            Ok(()) => {
                let _ = fs::remove_file(path);
                fs::rename(&tmp_path, path)
                    .map_err(|e| format!("Failed to commit PNG file: {}", e))
            }
            Err(e) => {
                let _ = fs::remove_file(&tmp_path);
                Err(e)
            }
        }
    }
}

fn write_png_file(
    tmp_path: &str,
    width: u32,
    height: u32,
    zlib_data: &[u8],
    token: &CancellationToken,
) -> Result<(), String> {
    let mut file =
        fs::File::create(tmp_path).map_err(|e| format!("Could not create temp file: {}", e))?;

    // PNG 8-byte signature
    file.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])
        .map_err(|e| format!("Failed writing PNG signature: {}", e))?;

    // IHDR chunk: width(4), height(4), bit_depth(1), color_type=2 RGB(1), 0, 0, 0
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // 8 bits per channel
    ihdr.push(2); // RGB Truecolor
    ihdr.push(0); // Deflate
    ihdr.push(0); // Default filter
    ihdr.push(0); // No interlace
    write_png_chunk(&mut file, b"IHDR", &ihdr)?;

    if token.is_cancelled() {
        return Err("Export cancelled by user".to_string());
    }

    // IDAT chunk
    write_png_chunk(&mut file, b"IDAT", zlib_data)?;

    // IEND chunk
    write_png_chunk(&mut file, b"IEND", &[])?;

    file.flush()
        .map_err(|e| format!("Failed flushing PNG data: {}", e))?;
    // Explicit drop so Windows unlocks the file handle before rename/remove
    drop(file);

    if token.is_cancelled() {
        return Err("Export cancelled by user".to_string());
    }

    Ok(())
}
