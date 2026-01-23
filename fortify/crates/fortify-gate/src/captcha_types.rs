//! Multi-type Captcha System
//!
//! Supports multiple captcha types for different security scenarios:
//! - BmpText: Traditional text-based image captcha (current/default)
//! - Emoji: Select the emoji matching a description
//! - Direction: Click the arrow pointing in the specified direction
//! - Sequence: Complete the pattern/sequence
//! - WordUnscramble: Unscramble the letters to form a word
//! - ImageRotation: Select the correctly oriented image
//! - Silhouette: Identify the silhouette category
//!
//! NO JAVASCRIPT - All captchas work via pure HTML forms

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Available captcha types
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptchaType {
    /// Traditional text-based BMP image captcha
    #[default]
    BmpText,
    /// Select emoji matching description (e.g., "happy face")
    Emoji,
    /// Click arrow pointing in specified direction
    Direction,
    /// Complete the sequence/pattern
    Sequence,
    /// Unscramble letters to form a word
    WordUnscramble,
    /// Select correctly rotated image
    ImageRotation,
    /// Identify silhouette category
    Silhouette,
}

impl CaptchaType {
    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BmpText => "Text Image",
            Self::Emoji => "Emoji Selection",
            Self::Direction => "Arrow Direction",
            Self::Sequence => "Sequence Pattern",
            Self::WordUnscramble => "Word Unscramble",
            Self::ImageRotation => "Image Rotation",
            Self::Silhouette => "Silhouette ID",
        }
    }

    /// Brief description
    pub fn description(&self) -> &'static str {
        match self {
            Self::BmpText => "Type the characters shown in the image",
            Self::Emoji => "Click the emoji that matches the description",
            Self::Direction => "Click the arrow pointing in the shown direction",
            Self::Sequence => "Select the next item in the sequence",
            Self::WordUnscramble => "Unscramble the letters to form the word",
            Self::ImageRotation => "Select the image that is right-side up",
            Self::Silhouette => "Identify what the silhouette represents",
        }
    }

    /// Is this a "heavier" captcha (more computationally intensive)
    pub fn is_heavy(&self) -> bool {
        matches!(self, Self::ImageRotation | Self::Silhouette)
    }

    /// All available captcha types
    pub fn all() -> Vec<CaptchaType> {
        vec![
            Self::BmpText,
            Self::Emoji,
            Self::Direction,
            Self::Sequence,
            Self::WordUnscramble,
            Self::ImageRotation,
            Self::Silhouette,
        ]
    }
}

/// Global captcha configuration for the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaConfig {
    /// Captcha type used at the Gate (initial verification)
    pub gate_captcha_type: CaptchaType,
    /// Captcha type used for threat/demotion (re-verification)
    pub threat_captcha_type: CaptchaType,
    /// Whether threat captcha is enabled (if false, use gate type)
    pub threat_captcha_enabled: bool,
    /// Randomly cycle between captcha types
    pub random_cycling: bool,
    /// Types to include in random cycling
    pub cycling_types: Vec<CaptchaType>,
    /// Per-type specific configurations
    pub type_configs: HashMap<CaptchaType, CaptchaTypeConfig>,
}

impl Default for CaptchaConfig {
    fn default() -> Self {
        let mut type_configs = HashMap::new();
        for captcha_type in CaptchaType::all() {
            type_configs.insert(captcha_type, CaptchaTypeConfig::default_for(captcha_type));
        }

        Self {
            gate_captcha_type: CaptchaType::BmpText,
            threat_captcha_type: CaptchaType::Emoji,
            threat_captcha_enabled: true,
            random_cycling: false,
            cycling_types: vec![
                CaptchaType::BmpText,
                CaptchaType::Emoji,
                CaptchaType::Direction,
            ],
            type_configs,
        }
    }
}

impl CaptchaConfig {
    /// Get the captcha type to use based on context
    pub fn get_captcha_type(&self, is_threat: bool) -> CaptchaType {
        if self.random_cycling && !self.cycling_types.is_empty() {
            let mut rng = rand::rng();
            let idx = rng.random_range(0..self.cycling_types.len());
            return self.cycling_types[idx];
        }

        if is_threat && self.threat_captcha_enabled {
            self.threat_captcha_type
        } else {
            self.gate_captcha_type
        }
    }

    /// Get type-specific config
    pub fn get_type_config(&self, captcha_type: CaptchaType) -> CaptchaTypeConfig {
        self.type_configs
            .get(&captcha_type)
            .cloned()
            .unwrap_or_else(|| CaptchaTypeConfig::default_for(captcha_type))
    }
}

/// Type-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaTypeConfig {
    /// Whether this captcha type is enabled
    pub enabled: bool,
    /// Number of options to display (for selection-based captchas)
    pub option_count: usize,
    /// Difficulty level (1-3, type-specific interpretation)
    pub difficulty: u8,
    /// Custom settings as key-value pairs
    pub custom: HashMap<String, String>,
}

impl CaptchaTypeConfig {
    pub fn default_for(captcha_type: CaptchaType) -> Self {
        match captcha_type {
            CaptchaType::BmpText => Self {
                enabled: true,
                option_count: 0, // N/A for text input
                difficulty: 2,   // Medium
                custom: HashMap::new(),
            },
            CaptchaType::Emoji => Self {
                enabled: true,
                option_count: 6, // 6 emoji options
                difficulty: 2,
                custom: HashMap::new(),
            },
            CaptchaType::Direction => Self {
                enabled: true,
                option_count: 4, // 4 arrows (up, down, left, right)
                difficulty: 1,   // Simple
                custom: HashMap::new(),
            },
            CaptchaType::Sequence => Self {
                enabled: true,
                option_count: 4, // 4 options for next in sequence
                difficulty: 2,
                custom: HashMap::new(),
            },
            CaptchaType::WordUnscramble => Self {
                enabled: true,
                option_count: 0, // Text input
                difficulty: 2,
                custom: HashMap::new(),
            },
            CaptchaType::ImageRotation => Self {
                enabled: true,
                option_count: 4, // 4 rotation options (0°, 90°, 180°, 270°)
                difficulty: 2,
                custom: HashMap::new(),
            },
            CaptchaType::Silhouette => Self {
                enabled: true,
                option_count: 4, // 4 category options
                difficulty: 2,
                custom: HashMap::new(),
            },
        }
    }
}

// ============================================================================
// EMOJI CAPTCHA
// ============================================================================

/// Emoji categories with unicode characters and descriptions
pub struct EmojiCategory {
    pub name: &'static str,
    pub description: &'static str,
    pub emojis: Vec<&'static str>,
}

/// Get all emoji categories
/// Each category has visually distinct emojis that are easy to identify
/// and render correctly in most browsers including Tor Browser
pub fn get_emoji_categories() -> Vec<EmojiCategory> {
    vec![
        EmojiCategory {
            name: "happy",
            description: "smiling or happy face",
            emojis: vec!["😀", "😃", "😄", "😁", "😊", "🙂"],
        },
        EmojiCategory {
            name: "sad",
            description: "sad or crying face",
            emojis: vec!["😢", "😭", "😞", "😔", "🙁", "😥"],
        },
        EmojiCategory {
            name: "angry",
            description: "red angry face",
            // Use only the most recognizable angry faces with red coloring
            emojis: vec!["😠", "😡", "🤬", "😤"],
        },
        EmojiCategory {
            name: "heart",
            description: "heart",
            emojis: vec!["❤️", "💕", "💖", "💗", "💓", "💘"],
        },
        EmojiCategory {
            name: "animal",
            description: "animal face",
            emojis: vec!["🐶", "🐱", "🐭", "🐰", "🦊", "🐻", "🐼"],
        },
        EmojiCategory {
            name: "food",
            description: "food",
            emojis: vec!["🍕", "🍔", "🍟", "🌮", "🍩", "🍪"],
        },
        EmojiCategory {
            name: "tree",
            description: "tree or plant",
            emojis: vec!["🌲", "🌳", "🌴", "🌵", "🌺", "🌻"],
        },
        EmojiCategory {
            name: "sun",
            description: "sun",
            emojis: vec!["☀️", "🌞", "🔆", "💛"],
        },
        EmojiCategory {
            name: "moon",
            description: "moon",
            emojis: vec!["🌙", "🌛", "🌜", "🌝"],
        },
        EmojiCategory {
            name: "star",
            description: "star",
            emojis: vec!["⭐", "🌟", "✨", "💫"],
        },
    ]
}

/// Generate an emoji captcha challenge
#[derive(Debug, Clone)]
pub struct EmojiChallenge {
    pub target_category: String,
    pub target_description: String,
    pub options: Vec<EmojiOption>,
    pub correct_index: usize,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
pub struct EmojiOption {
    pub emoji: String,
    pub index: usize,
}

impl EmojiChallenge {
    pub fn generate(option_count: usize) -> Self {
        Self::generate_validated(option_count).unwrap_or_else(|| {
            // Fallback: generate a simple valid challenge
            Self::generate_simple_fallback(option_count)
        })
    }

    /// Generate with validation, returns None if invalid
    fn generate_validated(option_count: usize) -> Option<Self> {
        let categories = get_emoji_categories();
        let mut rng = rand::rng();

        // Need at least as many categories as options for variety
        if option_count > categories.len() {
            return None;
        }

        // Pick a target category
        let target_idx = rng.random_range(0..categories.len());
        let target = &categories[target_idx];

        // Pick a random emoji from target category
        let correct_emoji_idx = rng.random_range(0..target.emojis.len());
        let correct_emoji = target.emojis[correct_emoji_idx].to_string();

        // Build options: one correct, rest from other categories
        let mut options: Vec<EmojiOption> = Vec::with_capacity(option_count);
        let correct_position = rng.random_range(0..option_count);

        // Create a shuffled list of available category indices (excluding target)
        let mut available_cats: Vec<usize> =
            (0..categories.len()).filter(|&i| i != target_idx).collect();
        // Shuffle for randomness
        for i in (1..available_cats.len()).rev() {
            let j = rng.random_range(0..=i);
            available_cats.swap(i, j);
        }
        let mut cat_iter = available_cats.into_iter();

        for i in 0..option_count {
            if i == correct_position {
                options.push(EmojiOption {
                    emoji: correct_emoji.clone(),
                    index: i,
                });
            } else if let Some(cat_idx) = cat_iter.next() {
                let other_cat = &categories[cat_idx];
                let emoji_idx = rng.random_range(0..other_cat.emojis.len());
                options.push(EmojiOption {
                    emoji: other_cat.emojis[emoji_idx].to_string(),
                    index: i,
                });
            } else {
                // Not enough categories - validation will fail
                return None;
            }
        }

        let challenge = Self {
            target_category: target.name.to_string(),
            target_description: target.description.to_string(),
            options,
            correct_index: correct_position,
            created_at: Instant::now(),
        };

        // Validate before returning
        if challenge.validate() {
            Some(challenge)
        } else {
            None
        }
    }

    /// Simple fallback that guarantees validity
    fn generate_simple_fallback(option_count: usize) -> Self {
        let categories = get_emoji_categories();
        let mut rng = rand::rng();

        // Use first N categories to ensure uniqueness
        let count = option_count.min(categories.len());
        let correct_position = rng.random_range(0..count);

        let target = &categories[correct_position];

        let mut options = Vec::with_capacity(count);
        for (i, cat) in categories.iter().enumerate().take(count) {
            options.push(EmojiOption {
                emoji: cat.emojis[0].to_string(),
                index: i,
            });
        }

        Self {
            target_category: target.name.to_string(),
            target_description: target.description.to_string(),
            options,
            correct_index: correct_position,
            created_at: Instant::now(),
        }
    }

    /// Validate that the challenge is solvable
    pub fn validate(&self) -> bool {
        // 1. Must have options
        if self.options.is_empty() {
            return false;
        }

        // 2. Correct index must be valid
        if self.correct_index >= self.options.len() {
            return false;
        }

        // 3. No duplicate emojis (each option must be unique)
        let mut seen = std::collections::HashSet::new();
        for opt in &self.options {
            if !seen.insert(&opt.emoji) {
                return false; // Duplicate found
            }
        }

        true
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    pub fn verify(&self, solution: &str) -> bool {
        if let Ok(selected_index) = solution.parse::<usize>() {
            selected_index == self.correct_index
        } else {
            false
        }
    }
}

// ============================================================================
// DIRECTION/ARROW CAPTCHA
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArrowDirection {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl ArrowDirection {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Up => "UP",
            Self::Down => "DOWN",
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
            Self::UpLeft => "UP-LEFT",
            Self::UpRight => "UP-RIGHT",
            Self::DownLeft => "DOWN-LEFT",
            Self::DownRight => "DOWN-RIGHT",
        }
    }

    pub fn arrow_char(&self) -> &'static str {
        match self {
            Self::Up => "↑",
            Self::Down => "↓",
            Self::Left => "←",
            Self::Right => "→",
            Self::UpLeft => "↖",
            Self::UpRight => "↗",
            Self::DownLeft => "↙",
            Self::DownRight => "↘",
        }
    }

    pub fn all_basic() -> Vec<ArrowDirection> {
        vec![Self::Up, Self::Down, Self::Left, Self::Right]
    }

    pub fn all() -> Vec<ArrowDirection> {
        vec![
            Self::Up,
            Self::Down,
            Self::Left,
            Self::Right,
            Self::UpLeft,
            Self::UpRight,
            Self::DownLeft,
            Self::DownRight,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct DirectionChallenge {
    pub target_direction: ArrowDirection,
    pub options: Vec<ArrowOption>,
    pub correct_index: usize,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
pub struct ArrowOption {
    pub direction: ArrowDirection,
    pub index: usize,
}

impl DirectionChallenge {
    pub fn generate(include_diagonals: bool) -> Self {
        let directions = if include_diagonals {
            ArrowDirection::all()
        } else {
            ArrowDirection::all_basic()
        };

        let mut rng = rand::rng();
        let target_idx = rng.random_range(0..directions.len());
        let target = directions[target_idx];

        // Use all directions as options
        let options: Vec<ArrowOption> = directions
            .iter()
            .enumerate()
            .map(|(i, &d)| ArrowOption {
                direction: d,
                index: i,
            })
            .collect();

        Self {
            target_direction: target,
            options,
            correct_index: target_idx,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    pub fn verify(&self, solution: &str) -> bool {
        if let Ok(selected_index) = solution.parse::<usize>() {
            selected_index == self.correct_index
        } else {
            false
        }
    }
}

// ============================================================================
// SEQUENCE CAPTCHA
// ============================================================================

#[derive(Debug, Clone)]
pub enum SequenceType {
    /// A, B, C, ? -> D
    Alphabet,
    /// 1, 2, 3, ? -> 4
    Numbers,
    /// 2, 4, 6, ? -> 8
    EvenNumbers,
    /// 1, 3, 5, ? -> 7
    OddNumbers,
    /// 1, 4, 9, ? -> 16 (squares)
    Squares,
    /// ●, ○, ●, ? -> ○
    Alternating,
    /// ▲, ■, ●, ? -> ▲
    Shapes,
}

#[derive(Debug, Clone)]
pub struct SequenceChallenge {
    pub sequence_display: Vec<String>,
    pub question_text: String,
    pub options: Vec<SequenceOption>,
    pub correct_index: usize,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
pub struct SequenceOption {
    pub display: String,
    pub index: usize,
}

impl SequenceChallenge {
    pub fn generate(option_count: usize) -> Self {
        let mut rng = rand::rng();

        // Generate different sequence types
        let sequence_type = rng.random_range(0..5);

        let (sequence, correct_answer, wrong_answers) = match sequence_type {
            0 => {
                // Alphabet: A, B, C, ? -> D
                let start = rng.random_range(0..23) as u8; // A-W
                let seq: Vec<String> = (0..3)
                    .map(|i| ((b'A' + start + i) as char).to_string())
                    .collect();
                let correct = ((b'A' + start + 3) as char).to_string();
                let wrongs: Vec<String> = (0..option_count - 1)
                    .map(|i| {
                        let offset = if i < 2 { i + 4 } else { 26 - i };
                        ((b'A' + (start + offset as u8) % 26) as char).to_string()
                    })
                    .collect();
                (seq, correct, wrongs)
            }
            1 => {
                // Numbers: 1, 2, 3, ? -> 4
                let start = rng.random_range(1..20);
                let seq: Vec<String> = (0..3).map(|i| (start + i).to_string()).collect();
                let correct = (start + 3).to_string();
                let wrongs: Vec<String> = (0..option_count - 1)
                    .map(|i| {
                        let offset = match i {
                            0 => 4,
                            1 => 5,
                            _ => (i + 3) as i32,
                        };
                        (start + offset).to_string()
                    })
                    .collect();
                (seq, correct, wrongs)
            }
            2 => {
                // Even: 2, 4, 6, ? -> 8
                let start = rng.random_range(1..10) * 2;
                let seq: Vec<String> = (0..3).map(|i| (start + i * 2).to_string()).collect();
                let correct = (start + 6).to_string();
                let wrongs: Vec<String> = vec![
                    (start + 7).to_string(),
                    (start + 8).to_string(),
                    (start + 5).to_string(),
                ];
                (seq, correct, wrongs)
            }
            3 => {
                // Odd: 1, 3, 5, ? -> 7
                let start = rng.random_range(0..10) * 2 + 1;
                let seq: Vec<String> = (0..3).map(|i| (start + i * 2).to_string()).collect();
                let correct = (start + 6).to_string();
                let wrongs: Vec<String> = vec![
                    (start + 8).to_string(),
                    (start + 5).to_string(),
                    (start + 4).to_string(),
                ];
                (seq, correct, wrongs)
            }
            _ => {
                // Shapes alternating: ●, ○, ●, ? -> ○
                let shapes_a = ["●", "■", "▲"];
                let shapes_b = ["○", "□", "△"];
                let shape_idx = rng.random_range(0..shapes_a.len());
                let seq: Vec<String> = vec![
                    shapes_a[shape_idx].to_string(),
                    shapes_b[shape_idx].to_string(),
                    shapes_a[shape_idx].to_string(),
                ];
                let correct = shapes_b[shape_idx].to_string();
                let wrongs: Vec<String> = shapes_a
                    .iter()
                    .chain(shapes_b.iter())
                    .filter(|&&s| s != shapes_b[shape_idx])
                    .take(option_count - 1)
                    .map(|s| s.to_string())
                    .collect();
                (seq, correct, wrongs)
            }
        };

        // Build options with correct answer in random position
        // Ensure we always have enough wrong answers
        let correct_position = rng.random_range(0..option_count);
        let mut options: Vec<SequenceOption> = Vec::with_capacity(option_count);

        // Extend wrong_answers if we don't have enough
        let mut wrongs = wrong_answers;
        while wrongs.len() < option_count - 1 {
            // Generate more plausible wrong answers
            let extra = format!("{}", rng.random_range(1..100));
            if !wrongs.contains(&extra) && extra != correct_answer {
                wrongs.push(extra);
            }
        }
        let mut wrong_iter = wrongs.into_iter();

        for i in 0..option_count {
            if i == correct_position {
                options.push(SequenceOption {
                    display: correct_answer.clone(),
                    index: i,
                });
            } else if let Some(wrong) = wrong_iter.next() {
                options.push(SequenceOption {
                    display: wrong,
                    index: i,
                });
            } else {
                // Fallback: use a placeholder (should never reach here now)
                options.push(SequenceOption {
                    display: "?".to_string(),
                    index: i,
                });
            }
        }

        let challenge = Self {
            sequence_display: sequence,
            question_text: "What comes next?".to_string(),
            options,
            correct_index: correct_position,
            created_at: Instant::now(),
        };

        // Validate and retry if needed
        if challenge.validate() {
            challenge
        } else {
            // Fallback to simple number sequence
            Self::generate_simple_fallback(option_count)
        }
    }

    /// Simple fallback that guarantees validity
    fn generate_simple_fallback(option_count: usize) -> Self {
        let mut rng = rand::rng();
        let start = rng.random_range(1..10);
        let sequence: Vec<String> = (0..3).map(|i| (start + i).to_string()).collect();
        let correct_answer = (start + 3).to_string();

        let correct_position = rng.random_range(0..option_count);
        let mut options = Vec::with_capacity(option_count);
        let mut wrong_val = start + 4;

        for i in 0..option_count {
            if i == correct_position {
                options.push(SequenceOption {
                    display: correct_answer.clone(),
                    index: i,
                });
            } else {
                options.push(SequenceOption {
                    display: wrong_val.to_string(),
                    index: i,
                });
                wrong_val += 1;
            }
        }

        Self {
            sequence_display: sequence,
            question_text: "What comes next?".to_string(),
            options,
            correct_index: correct_position,
            created_at: Instant::now(),
        }
    }

    /// Validate that the challenge is solvable
    pub fn validate(&self) -> bool {
        // 1. Must have options
        if self.options.is_empty() {
            return false;
        }

        // 2. Correct index must be valid
        if self.correct_index >= self.options.len() {
            return false;
        }

        // 3. All options must have valid display text
        for opt in &self.options {
            if opt.display.is_empty() || opt.display == "?" {
                return false;
            }
        }

        // 4. No duplicate options
        let mut seen = std::collections::HashSet::new();
        for opt in &self.options {
            if !seen.insert(&opt.display) {
                return false;
            }
        }

        true
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    pub fn verify(&self, solution: &str) -> bool {
        if let Ok(selected_index) = solution.parse::<usize>() {
            selected_index == self.correct_index
        } else {
            false
        }
    }
}

// ============================================================================
// WORD UNSCRAMBLE CAPTCHA
// ============================================================================

/// Common words for unscrambling (4-6 letters)
pub const UNSCRAMBLE_WORDS: &[&str] = &[
    "apple", "table", "chair", "house", "water", "light", "music", "earth", "cloud", "storm",
    "river", "ocean", "plant", "stone", "metal", "paper", "clock", "phone", "radio", "field",
    "grass", "track", "train", "plane", "coast", "beach", "frost", "flame", "flash", "globe",
    "grape", "lemon", "melon", "peach", "pizza", "salad", "bread", "cream", "sugar", "spice",
];

#[derive(Debug, Clone)]
pub struct WordUnscrambleChallenge {
    pub original_word: String,
    pub scrambled: String,
    pub hint: Option<String>,
    pub created_at: Instant,
}

impl WordUnscrambleChallenge {
    pub fn generate(difficulty: u8) -> Self {
        let mut rng = rand::rng();
        let word_idx = rng.random_range(0..UNSCRAMBLE_WORDS.len());
        let word = UNSCRAMBLE_WORDS[word_idx].to_string();

        // Scramble the word
        let mut chars: Vec<char> = word.chars().collect();
        for _ in 0..word.len() * 2 {
            let i = rng.random_range(0..chars.len());
            let j = rng.random_range(0..chars.len());
            chars.swap(i, j);
        }

        // Make sure it's actually scrambled (not same as original)
        let scrambled: String = chars.iter().collect();
        let scrambled = if scrambled == word {
            chars.reverse();
            chars.iter().collect()
        } else {
            scrambled
        };

        // Add hint for easy difficulty
        let hint = if difficulty <= 1 {
            Some(format!(
                "Hint: First letter is '{}'",
                word.chars().next().unwrap_or(' ')
            ))
        } else {
            None
        };

        Self {
            original_word: word,
            scrambled: scrambled.to_uppercase(),
            hint,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    pub fn verify(&self, answer: &str) -> bool {
        self.original_word.eq_ignore_ascii_case(answer.trim())
    }
}

// ============================================================================
// IMAGE ROTATION CAPTCHA
// ============================================================================

/// Rotation angles for the rotation captcha
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RotationAngle {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

impl RotationAngle {
    pub fn degrees(&self) -> u32 {
        match self {
            Self::Deg0 => 0,
            Self::Deg90 => 90,
            Self::Deg180 => 180,
            Self::Deg270 => 270,
        }
    }

    pub fn display(&self) -> &'static str {
        match self {
            Self::Deg0 => "↑ Upright",
            Self::Deg90 => "→ 90° Right",
            Self::Deg180 => "↓ Upside Down",
            Self::Deg270 => "← 90° Left",
        }
    }

    pub fn all() -> Vec<RotationAngle> {
        vec![Self::Deg0, Self::Deg90, Self::Deg180, Self::Deg270]
    }
}

/// Simple shape icons for rotation (rendered as ASCII art)
pub const ROTATION_SHAPES: &[(&str, &str)] = &[
    ("arrow", "▲"),
    ("house", "⌂"),
    ("person", "♀"),
    ("tree", "🌲"),
    ("cup", "☕"),
];

#[derive(Debug, Clone)]
pub struct ImageRotationChallenge {
    pub shape_name: String,
    pub shape_char: String,
    pub options: Vec<RotationOption>,
    pub correct_index: usize,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
pub struct RotationOption {
    pub rotation: RotationAngle,
    pub index: usize,
    /// Visual representation (may be rotated text/symbol)
    pub display: String,
}

impl ImageRotationChallenge {
    pub fn generate() -> Self {
        let mut rng = rand::rng();

        // Pick a shape
        let shape_idx = rng.random_range(0..ROTATION_SHAPES.len());
        let (shape_name, shape_char) = ROTATION_SHAPES[shape_idx];

        // The correct answer is always the upright one (Deg0)
        // We show rotated versions and user must pick the upright one
        let angles = RotationAngle::all();
        let correct_index = 0; // Deg0 is always correct

        let options: Vec<RotationOption> = angles
            .iter()
            .enumerate()
            .map(|(i, &angle)| RotationOption {
                rotation: angle,
                index: i,
                display: format!("{} ({}°)", shape_char, angle.degrees()),
            })
            .collect();

        Self {
            shape_name: shape_name.to_string(),
            shape_char: shape_char.to_string(),
            options,
            correct_index,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    pub fn verify(&self, solution: &str) -> bool {
        if let Ok(selected_index) = solution.parse::<usize>() {
            selected_index == self.correct_index
        } else {
            false
        }
    }
}

// ============================================================================
// SILHOUETTE CAPTCHA
// ============================================================================

/// Silhouette categories
pub struct SilhouetteCategory {
    pub name: &'static str,
    pub description: &'static str,
    /// Unicode symbols representing silhouettes
    pub symbols: Vec<&'static str>,
}

pub fn get_silhouette_categories() -> Vec<SilhouetteCategory> {
    vec![
        SilhouetteCategory {
            name: "vehicle",
            description: "vehicle or transportation",
            symbols: vec!["🚗", "🚕", "🚙", "🏎️", "🚌", "🚐", "🛻", "🚚"],
        },
        SilhouetteCategory {
            name: "animal",
            description: "animal or creature",
            symbols: vec!["🐕", "🐈", "🐎", "🐄", "🦁", "🐘", "🦅", "🐟"],
        },
        SilhouetteCategory {
            name: "building",
            description: "building or structure",
            symbols: vec!["🏠", "🏢", "🏭", "🏰", "🗼", "⛪", "🕌", "🏛️"],
        },
        SilhouetteCategory {
            name: "person",
            description: "person or people",
            symbols: vec!["🧍", "🚶", "🏃", "💃", "🕺", "🧑‍🤝‍🧑", "👨‍👩‍👧", "🤸"],
        },
    ]
}

#[derive(Debug, Clone)]
pub struct SilhouetteChallenge {
    pub silhouette_symbol: String,
    pub correct_category: String,
    pub options: Vec<SilhouetteOption>,
    pub correct_index: usize,
    pub created_at: Instant,
}

#[derive(Debug, Clone)]
pub struct SilhouetteOption {
    pub category_name: String,
    pub description: String,
    pub index: usize,
}

impl SilhouetteChallenge {
    pub fn generate(option_count: usize) -> Self {
        let categories = get_silhouette_categories();
        let mut rng = rand::rng();

        // Limit options to available categories
        let actual_count = option_count.min(categories.len());

        // Pick correct category and symbol
        let correct_cat_idx = rng.random_range(0..categories.len());
        let correct_cat = &categories[correct_cat_idx];
        let symbol_idx = rng.random_range(0..correct_cat.symbols.len());
        let symbol = correct_cat.symbols[symbol_idx].to_string();

        // Position correct answer randomly
        let correct_position = rng.random_range(0..actual_count);

        // Build shuffled list of wrong categories
        let mut other_cats: Vec<usize> = (0..categories.len())
            .filter(|&i| i != correct_cat_idx)
            .collect();
        // Shuffle for randomness
        for i in (1..other_cats.len()).rev() {
            let j = rng.random_range(0..=i);
            other_cats.swap(i, j);
        }
        let mut cat_iter = other_cats.into_iter();

        // Build options - always ensure correct answer is included
        let mut options: Vec<SilhouetteOption> = Vec::with_capacity(actual_count);

        for i in 0..actual_count {
            if i == correct_position {
                options.push(SilhouetteOption {
                    category_name: correct_cat.name.to_string(),
                    description: correct_cat.description.to_string(),
                    index: i,
                });
            } else if let Some(cat_idx) = cat_iter.next() {
                let cat = &categories[cat_idx];
                options.push(SilhouetteOption {
                    category_name: cat.name.to_string(),
                    description: cat.description.to_string(),
                    index: i,
                });
            } else {
                // Fallback: duplicate first other category (shouldn't happen)
                let cat = &categories[(correct_cat_idx + 1) % categories.len()];
                options.push(SilhouetteOption {
                    category_name: cat.name.to_string(),
                    description: cat.description.to_string(),
                    index: i,
                });
            }
        }

        let challenge = Self {
            silhouette_symbol: symbol,
            correct_category: correct_cat.name.to_string(),
            options,
            correct_index: correct_position,
            created_at: Instant::now(),
        };

        // Validate
        if challenge.validate() {
            challenge
        } else {
            // Fallback: use first N categories
            Self::generate_simple_fallback(actual_count)
        }
    }

    /// Simple fallback that guarantees validity
    fn generate_simple_fallback(option_count: usize) -> Self {
        let categories = get_silhouette_categories();
        let mut rng = rand::rng();

        let correct_position = rng.random_range(0..option_count.min(categories.len()));
        let correct_cat = &categories[correct_position];
        let symbol = correct_cat.symbols[0].to_string();

        let options: Vec<SilhouetteOption> = (0..option_count.min(categories.len()))
            .map(|i| {
                let cat = &categories[i];
                SilhouetteOption {
                    category_name: cat.name.to_string(),
                    description: cat.description.to_string(),
                    index: i,
                }
            })
            .collect();

        Self {
            silhouette_symbol: symbol,
            correct_category: correct_cat.name.to_string(),
            options,
            correct_index: correct_position,
            created_at: Instant::now(),
        }
    }

    /// Validate that the challenge is solvable
    pub fn validate(&self) -> bool {
        // 1. Must have options
        if self.options.is_empty() {
            return false;
        }

        // 2. Correct index must be valid
        if self.correct_index >= self.options.len() {
            return false;
        }

        // 3. Correct category must be in options at correct_index
        if self.options[self.correct_index].category_name != self.correct_category {
            return false;
        }

        // 4. No duplicate categories
        let mut seen = std::collections::HashSet::new();
        for opt in &self.options {
            if !seen.insert(&opt.category_name) {
                return false;
            }
        }

        true
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.created_at.elapsed() > timeout
    }

    pub fn verify(&self, solution: &str) -> bool {
        if let Ok(selected_index) = solution.parse::<usize>() {
            selected_index == self.correct_index
        } else {
            false
        }
    }
}

// ============================================================================
// UNIFIED CAPTCHA CHALLENGE
// ============================================================================

/// Unified challenge that can hold any captcha type
#[derive(Debug, Clone)]
pub enum CaptchaData {
    BmpText { text: String, image_data: Vec<u8> },
    Emoji(EmojiChallenge),
    Direction(DirectionChallenge),
    Sequence(SequenceChallenge),
    WordUnscramble(WordUnscrambleChallenge),
    ImageRotation(ImageRotationChallenge),
    Silhouette(SilhouetteChallenge),
}

impl CaptchaData {
    pub fn captcha_type(&self) -> CaptchaType {
        match self {
            Self::BmpText { .. } => CaptchaType::BmpText,
            Self::Emoji(_) => CaptchaType::Emoji,
            Self::Direction(_) => CaptchaType::Direction,
            Self::Sequence(_) => CaptchaType::Sequence,
            Self::WordUnscramble(_) => CaptchaType::WordUnscramble,
            Self::ImageRotation(_) => CaptchaType::ImageRotation,
            Self::Silhouette(_) => CaptchaType::Silhouette,
        }
    }

    /// Verify a text answer (for text-based captchas)
    pub fn verify_text(&self, answer: &str) -> bool {
        match self {
            Self::BmpText { text, .. } => text.eq_ignore_ascii_case(answer),
            Self::WordUnscramble(c) => c.verify(answer),
            _ => false,
        }
    }

    /// Verify an index selection (for selection-based captchas)
    pub fn verify_selection(&self, index: &str) -> bool {
        match self {
            Self::Emoji(c) => c.verify(index),
            Self::Direction(c) => c.verify(index),
            Self::Sequence(c) => c.verify(index),
            Self::ImageRotation(c) => c.verify(index),
            Self::Silhouette(c) => c.verify(index),
            _ => false,
        }
    }

    /// Validate that the CAPTCHA is solvable
    /// Returns true if the CAPTCHA has a valid correct answer in its options
    pub fn validate(&self) -> bool {
        match self {
            Self::BmpText { text, image_data } => {
                // BmpText is valid if it has text and image data
                !text.is_empty() && !image_data.is_empty()
            }
            Self::Emoji(c) => c.validate(),
            Self::Direction(_) => true, // Direction always uses all 4/8 arrows
            Self::Sequence(c) => c.validate(),
            Self::WordUnscramble(c) => {
                // Valid if word exists and scrambled is different
                !c.original_word.is_empty() && c.scrambled != c.original_word
            }
            Self::ImageRotation(_) => true, // Always has 4 rotation options
            Self::Silhouette(c) => c.validate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_challenge() {
        let challenge = EmojiChallenge::generate(6);
        assert_eq!(challenge.options.len(), 6);
        assert!(challenge.validate(), "Emoji challenge should be valid");
        assert!(challenge.verify(&challenge.correct_index.to_string()));
        assert!(!challenge.verify(&((challenge.correct_index + 1) % 6).to_string()));
    }

    #[test]
    fn test_emoji_challenge_many() {
        // Generate many challenges to catch edge cases
        for _ in 0..100 {
            let challenge = EmojiChallenge::generate(6);
            assert!(challenge.validate(), "Generated challenge must be valid");
            assert_eq!(challenge.options.len(), 6);
            assert!(challenge.correct_index < 6);
        }
    }

    #[test]
    fn test_direction_challenge() {
        let challenge = DirectionChallenge::generate(false);
        assert_eq!(challenge.options.len(), 4);
        assert!(challenge.verify(&challenge.correct_index.to_string()));
    }

    #[test]
    fn test_sequence_challenge() {
        let challenge = SequenceChallenge::generate(4);
        assert_eq!(
            challenge.options.len(),
            4,
            "Sequence should always have requested options"
        );
        assert!(challenge.validate(), "Sequence challenge should be valid");
        assert!(challenge.verify(&challenge.correct_index.to_string()));
    }

    #[test]
    fn test_sequence_challenge_many() {
        // Generate many to catch edge cases
        for _ in 0..100 {
            let challenge = SequenceChallenge::generate(4);
            assert!(challenge.validate(), "Generated challenge must be valid");
            assert_eq!(challenge.options.len(), 4);
            assert!(challenge.correct_index < 4);
        }
    }

    #[test]
    fn test_silhouette_challenge() {
        let challenge = SilhouetteChallenge::generate(4);
        assert!(challenge.validate(), "Silhouette challenge should be valid");
        assert!(challenge.verify(&challenge.correct_index.to_string()));
    }

    #[test]
    fn test_silhouette_challenge_many() {
        for _ in 0..100 {
            let challenge = SilhouetteChallenge::generate(4);
            assert!(challenge.validate(), "Generated challenge must be valid");
            assert!(challenge.correct_index < challenge.options.len());
        }
    }

    #[test]
    fn test_word_unscramble() {
        let challenge = WordUnscrambleChallenge::generate(2);
        assert!(challenge.verify(&challenge.original_word));
        assert!(challenge.verify(&challenge.original_word.to_uppercase()));
    }
}
