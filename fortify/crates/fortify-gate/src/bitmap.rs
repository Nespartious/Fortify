use rand::Rng;

/// Captcha difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaDifficulty {
    Easy,
    Medium,
    Hard,
}

impl CaptchaDifficulty {
    pub fn from_difficulty_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "easy" => Self::Easy,
            "hard" => Self::Hard,
            _ => Self::Medium,
        }
    }
}

pub fn generate_bmp(text: &str) -> Vec<u8> {
    generate_bmp_with_difficulty(text, CaptchaDifficulty::Medium)
}

pub fn generate_bmp_with_difficulty(text: &str, difficulty: CaptchaDifficulty) -> Vec<u8> {
    let width = 300u32;
    let height = 100u32;

    // BMP lines must be padded to multiples of 4 bytes
    let row_size_bytes = width * 3;
    let padding = (4 - (row_size_bytes % 4)) % 4;
    let stride = row_size_bytes + padding;

    let mut pixel_data = vec![0u8; (stride * height) as usize];
    let mut rng = rand::thread_rng();

    // Fill background with slight variation based on difficulty
    let (base_r, base_g, base_b) = (0x15, 0x05, 0x20);
    for y in 0..height {
        for x in 0..width {
            let idx = (y * stride + x * 3) as usize;
            pixel_data[idx] = base_b;
            pixel_data[idx + 1] = base_g;
            pixel_data[idx + 2] = base_r;
        }
    }

    // Add noise based on difficulty
    let noise_density = match difficulty {
        CaptchaDifficulty::Easy => 0,
        CaptchaDifficulty::Medium => 800,
        CaptchaDifficulty::Hard => 2500,
    };

    // Add random noise pixels
    for _ in 0..noise_density {
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);
        let idx = (y * stride + x * 3) as usize;
        if idx + 2 < pixel_data.len() {
            // Random colors in theme palette
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

    // Drawing parameters - vary scale with difficulty
    let scale = match difficulty {
        CaptchaDifficulty::Easy => 5u32,
        CaptchaDifficulty::Medium => 4u32,
        CaptchaDifficulty::Hard => 3u32,
    };

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
        // Add per-character vertical offset for hard difficulty (wavy effect)
        let y_offset: i32 = match difficulty {
            CaptchaDifficulty::Easy => 0,
            CaptchaDifficulty::Medium => rng.gen_range(-2..=2),
            CaptchaDifficulty::Hard => rng.gen_range(-6..=6),
        };

        let char_x = start_x + (i as u32 * (char_pixel_width + spacing));
        let char_y = (start_y as i32 + y_offset).max(0) as u32;

        draw_char_bottom_up(
            &mut pixel_data,
            width,
            height,
            stride,
            c,
            char_x,
            char_y.min(height - char_pixel_height),
            scale,
            difficulty,
        );
    }

    // Add interference lines for medium/hard
    let line_count = match difficulty {
        CaptchaDifficulty::Easy => 0,
        CaptchaDifficulty::Medium => 2,
        CaptchaDifficulty::Hard => 5,
    };

    for _ in 0..line_count {
        draw_interference_line(&mut pixel_data, width, height, stride, &mut rng);
    }

    // Construct BMP File
    let file_size = 54 + pixel_data.len() as u32;
    let mut bmp = Vec::with_capacity(file_size as usize);

    // 1. Bitmap File Header (14 bytes)
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size.to_le_bytes());
    bmp.extend_from_slice(&[0, 0, 0, 0]); // Reserved
    bmp.extend_from_slice(&54u32.to_le_bytes()); // Offset to pixel data

    // 2. DIB Header (BITMAPINFOHEADER) (40 bytes)
    bmp.extend_from_slice(&40u32.to_le_bytes()); // Header size
    bmp.extend_from_slice(&(width as i32).to_le_bytes());
    bmp.extend_from_slice(&(height as i32).to_le_bytes()); // Positive = Bottom-Up
    bmp.extend_from_slice(&1u16.to_le_bytes()); // Planes
    bmp.extend_from_slice(&24u16.to_le_bytes()); // BPP
    bmp.extend_from_slice(&0u32.to_le_bytes()); // Compression
    bmp.extend_from_slice(&(pixel_data.len() as u32).to_le_bytes()); // Image size
    bmp.extend_from_slice(&2835u32.to_le_bytes()); // X PPM
    bmp.extend_from_slice(&2835u32.to_le_bytes()); // Y PPM
    bmp.extend_from_slice(&0u32.to_le_bytes()); // Colors used
    bmp.extend_from_slice(&0u32.to_le_bytes()); // Important colors

    bmp.extend_from_slice(&pixel_data);
    bmp
}

fn draw_interference_line(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    stride: u32,
    rng: &mut impl Rng,
) {
    // Draw a semi-random line across the image
    let y_start = rng.gen_range(20..height - 20);
    let y_end = rng.gen_range(20..height - 20);

    for x in 0..width {
        // Linear interpolation for y position
        let progress = x as f32 / width as f32;
        let y = (y_start as f32 + (y_end as f32 - y_start as f32) * progress) as u32;

        // Add some waviness
        let wave = ((x as f32 * 0.1).sin() * 3.0) as i32;
        let final_y = (y as i32 + wave).max(0).min(height as i32 - 1) as u32;

        let buffer_y = height - 1 - final_y;
        let idx = (buffer_y * stride + x * 3) as usize;
        if idx + 2 < pixels.len() {
            // Dark purple line
            pixels[idx] = 0x60; // B
            pixels[idx + 1] = 0x20; // G
            pixels[idx + 2] = 0x60; // R
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_char_bottom_up(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    stride: u32,
    c: char,
    x: u32,
    y: u32,
    scale: u32,
    difficulty: CaptchaDifficulty,
) {
    let font_data = get_font_bitmap(c);
    let mut rng = rand::thread_rng();

    for (row, row_byte) in font_data.iter().enumerate() {
        for col in 0..5 {
            if (row_byte >> (4 - col)) & 1 == 1 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = x + (col as u32 * scale) + dx;
                        let py = y + (row as u32 * scale) + dy;

                        if px >= width || py >= height {
                            continue;
                        }

                        // Skip some pixels randomly on hard difficulty (erosion effect)
                        if difficulty == CaptchaDifficulty::Hard && rng.gen_ratio(1, 10) {
                            continue;
                        }

                        let buffer_y = height - 1 - py;
                        let idx = (buffer_y * stride + px * 3) as usize;
                        if idx + 2 < pixels.len() {
                            // Vary green intensity slightly on harder difficulties
                            let green_val = match difficulty {
                                CaptchaDifficulty::Easy => 255u8,
                                CaptchaDifficulty::Medium => rng.gen_range(200..=255),
                                CaptchaDifficulty::Hard => rng.gen_range(150..=255),
                            };
                            pixels[idx] = 0;
                            pixels[idx + 1] = green_val;
                            pixels[idx + 2] = 0;
                        }
                    }
                }
            }
        }
    }
}

fn get_font_bitmap(c: char) -> [u8; 7] {
    match c {
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x0E, 0x11, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x1E, 0x01, 0x01, 0x0E],
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x13, 0x11, 0x11, 0x0E],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        _ => [0x1F, 0x11, 0x1F, 0x11, 0x1F, 0x11, 0x1F],
    }
}
