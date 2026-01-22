//! Stub for CAPTCHA image generation in orchestrator
//!
//! This provides a simplified captcha image generator for pre-generation.
//! The full implementation is in fortify-gate::bitmap, but we need a
//! standalone version here to avoid circular dependencies.

use rand::Rng;

/// Generate a simple CAPTCHA image for pre-generation pool
/// Returns BMP image bytes
pub fn generate_captcha_image(text: &str) -> Vec<u8> {
    let width = 300u32;
    let height = 100u32;

    // BMP lines must be padded to multiples of 4 bytes
    let row_size_bytes = width * 3;
    let padding = (4 - (row_size_bytes % 4)) % 4;
    let stride = row_size_bytes + padding;

    let mut pixel_data = vec![0u8; (stride * height) as usize];
    let mut rng = rand::thread_rng();

    // Fill background with dark purple theme
    let (base_r, base_g, base_b) = (0x15, 0x05, 0x20);
    for y in 0..height {
        for x in 0..width {
            let idx = (y * stride + x * 3) as usize;
            pixel_data[idx] = base_b;
            pixel_data[idx + 1] = base_g;
            pixel_data[idx + 2] = base_r;
        }
    }

    // Add noise (Medium difficulty = 800 noise pixels)
    for _ in 0..800 {
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);
        let idx = (y * stride + x * 3) as usize;
        if idx + 2 < pixel_data.len() {
            let colors = [
                (0x00, 0x80, 0x00), // Dark green
                (0x80, 0x00, 0x80), // Purple
                (0x00, 0x40, 0x00), // Darker green
                (0x40, 0x20, 0x40), // Dark magenta
            ];
            let (r, g, b) = colors[rng.gen_range(0..colors.len())];
            pixel_data[idx] = b;
            pixel_data[idx + 1] = g;
            pixel_data[idx + 2] = r;
        }
    }

    // Drawing parameters (Medium difficulty)
    let scale = 4u32;
    let char_pixel_width = 5 * scale;
    let char_pixel_height = 7 * scale;
    let spacing = scale;

    let total_chars = text.len() as u32;
    if total_chars == 0 {
        return vec![];
    }

    let total_text_width = (total_chars * char_pixel_width) + ((total_chars - 1) * spacing);
    let start_x = if total_text_width < width {
        (width - total_text_width) / 2
    } else {
        0
    };
    let start_y = if char_pixel_height < height {
        (height - char_pixel_height) / 2
    } else {
        0
    };

    for (i, c) in text.chars().enumerate() {
        let y_offset: i32 = rng.gen_range(-2..=2);

        let char_x = start_x + (i as u32 * (char_pixel_width + spacing));
        let pattern = get_char_pattern(c);

        // Draw character
        for (row, &pattern_row) in pattern.iter().enumerate() {
            for col in 0..5 {
                if (pattern_row >> (4 - col)) & 1 == 1 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            let px = char_x + col * scale + sx;
                            let py_signed =
                                start_y as i32 + (row as u32 * scale) as i32 + sy as i32 + y_offset;

                            if py_signed >= 0 && (py_signed as u32) < height && px < width {
                                let py = py_signed as u32;
                                let idx = (py * stride + px * 3) as usize;
                                // Neon green text
                                pixel_data[idx] = 0x00; // B
                                pixel_data[idx + 1] = 0xFF; // G
                                pixel_data[idx + 2] = 0x00; // R
                            }
                        }
                    }
                }
            }
        }
    }

    // Build BMP file
    let file_header_size = 14u32;
    let info_header_size = 40u32;
    let pixel_data_size = (stride * height) as u32;
    let file_size = file_header_size + info_header_size + pixel_data_size;

    let mut bmp = Vec::with_capacity(file_size as usize);

    // File header
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&[0u8; 4]); // Reserved
    bmp.extend_from_slice(&(file_header_size + info_header_size).to_le_bytes());

    // Info header
    bmp.extend_from_slice(&info_header_size.to_le_bytes());
    bmp.extend_from_slice(&width.to_le_bytes());
    bmp.extend_from_slice(&(height as i32).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes()); // Planes
    bmp.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
    bmp.extend_from_slice(&0u32.to_le_bytes()); // Compression
    bmp.extend_from_slice(&pixel_data_size.to_le_bytes());
    bmp.extend_from_slice(&2835u32.to_le_bytes()); // X pixels/meter
    bmp.extend_from_slice(&2835u32.to_le_bytes()); // Y pixels/meter
    bmp.extend_from_slice(&0u32.to_le_bytes()); // Colors used
    bmp.extend_from_slice(&0u32.to_le_bytes()); // Important colors

    // BMP stores rows bottom-to-top
    for y in (0..height).rev() {
        let row_start = (y * stride) as usize;
        let row_end = row_start + stride as usize;
        bmp.extend_from_slice(&pixel_data[row_start..row_end]);
    }

    bmp
}

/// 5x7 pixel patterns for characters
fn get_char_pattern(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        _ => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
    }
}
