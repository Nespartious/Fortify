//! HTML Rendering for Multi-Type Captcha System
//!
//! Generates pure HTML/CSS forms for each captcha type.
//! NO JAVASCRIPT - all interaction via form submissions.

use crate::captcha_types::*;
use fortify_core::templates::{BrandingVars, TemplateEngine, TemplateType};
use std::collections::HashMap;

/// Common CSS styles for captcha pages - Citadel/Gold theme
pub fn captcha_css() -> &'static str {
    r#"
    :root {
        /* Citadel/Gold Theme */
        --bg-deep: #141417;
        --bg-surface: #1e1e23;
        --bg-elevated: #26262d;
        --border-subtle: #3a3a42;
        --brand-primary: #c9a227;
        --brand-secondary: #a68b5b;
        --text-primary: #f5f0e8;
        --text-secondary: #a8a4a0;
        --text-muted: #6b6862;
        --status-warning: #e4bc5e;
        --status-error: #e05252;
        --accent-green: #6aa84f;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
        background: var(--bg-deep);
        font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, sans-serif;
        color: var(--text-primary);
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 24px;
    }
    .panel {
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3);
        padding: 32px 28px;
        width: 100%;
        max-width: 520px;
        position: relative;
        border-radius: 4px;
    }
    .panel.threat {
        border-color: var(--status-warning);
        border-left: 3px solid var(--status-warning);
    }
    h1 {
        text-align: center;
        margin: 0 0 8px 0;
        color: var(--brand-primary);
        font-size: 1.4rem;
        letter-spacing: 0.1em;
        text-transform: uppercase;
        font-weight: 500;
    }
    .subtitle {
        text-align: center;
        color: var(--text-muted);
        margin-bottom: 24px;
        font-size: 0.75rem;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        padding-bottom: 16px;
        border-bottom: 1px solid var(--border-subtle);
    }
    .instruction {
        text-align: center;
        color: var(--text-secondary);
        font-size: 0.9rem;
        margin-bottom: 20px;
        padding: 14px;
        background: var(--bg-elevated);
        border: 1px solid var(--border-subtle);
        border-left: 3px solid var(--brand-primary);
        border-radius: 0 4px 4px 0;
    }
    .instruction strong {
        color: var(--brand-primary);
        font-size: 1.1rem;
    }
    .captcha-container {
        background: var(--bg-elevated);
        border: 1px solid var(--border-subtle);
        padding: 20px;
        margin-bottom: 24px;
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        gap: 12px;
        min-height: 100px;
        border-radius: 4px;
    }
    .option-btn {
        background: var(--bg-surface);
        border: 1px solid var(--border-subtle);
        color: var(--text-primary);
        padding: 18px;
        font-size: 2rem;
        cursor: pointer;
        transition: all 0.2s;
        min-width: 70px;
        text-align: center;
        border-radius: 3px;
    }
    .option-btn:hover {
        background: var(--brand-primary);
        color: var(--bg-deep);
        border-color: var(--brand-primary);
    }
    .option-btn.small {
        padding: 12px 20px;
        font-size: 1.1rem;
    }
    .option-btn.text {
        font-size: 0.95rem;
        padding: 10px 16px;
    }
    .text-input {
        width: 100%;
        background: var(--bg-elevated);
        border: 1px solid var(--border-subtle);
        border-left: 3px solid var(--brand-primary);
        color: var(--text-primary);
        padding: 14px;
        font-family: inherit;
        font-size: 1.2rem;
        text-align: center;
        outline: none;
        margin-bottom: 20px;
        text-transform: uppercase;
        letter-spacing: 2px;
        border-radius: 0 4px 4px 0;
    }
    .text-input:focus {
        border-color: var(--brand-primary);
        background: color-mix(in srgb, var(--brand-primary) 5%, var(--bg-elevated));
    }
    .submit-btn {
        width: 100%;
        background: var(--brand-primary);
        border: none;
        color: var(--bg-deep);
        padding: 14px;
        font-family: inherit;
        font-size: 0.9rem;
        font-weight: 600;
        cursor: pointer;
        text-transform: uppercase;
        letter-spacing: 0.15em;
        transition: all 0.2s;
        border-radius: 3px;
    }
    .submit-btn:hover {
        filter: brightness(1.15);
        box-shadow: 0 4px 16px color-mix(in srgb, var(--brand-primary) 25%, transparent);
    }
    .footer {
        margin-top: 20px;
        display: flex;
        justify-content: space-between;
        font-size: 0.65rem;
        color: var(--text-muted);
        text-transform: uppercase;
        letter-spacing: 0.1em;
    }
    .sequence-display {
        display: flex;
        gap: 12px;
        justify-content: center;
        align-items: center;
        margin-bottom: 20px;
        font-size: 1.8rem;
    }
    .sequence-item {
        background: var(--bg-elevated);
        border: 1px solid var(--border-subtle);
        padding: 14px 18px;
        min-width: 48px;
        text-align: center;
        border-radius: 3px;
    }
    .sequence-item.question {
        color: var(--brand-primary);
        border-color: var(--brand-primary);
    }
    .silhouette {
        font-size: 4rem;
        padding: 20px;
        background: var(--bg-elevated);
        border: 1px solid var(--border-subtle);
        margin-bottom: 20px;
        text-align: center;
        filter: grayscale(100%) brightness(0.4);
        border-radius: 4px;
    }
    .arrow-grid {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 12px;
        max-width: 280px;
        margin: 0 auto;
    }
    .scrambled-word {
        font-size: 2rem;
        letter-spacing: 6px;
        text-align: center;
        color: var(--brand-primary);
        margin-bottom: 20px;
        padding: 14px;
        background: var(--bg-elevated);
        border: 1px solid var(--brand-secondary);
        border-radius: 4px;
    }
    .hint {
        text-align: center;
        font-size: 0.85rem;
        color: var(--status-warning);
        margin-bottom: 14px;
    }
    "#
}

/// Generate HTML for BMP Text captcha (current/default)
pub fn render_bmp_text_captcha(session_id: &str, captcha_id: &str, is_threat: bool) -> String {
    render_bmp_text_captcha_with_message(
        session_id,
        captcha_id,
        is_threat,
        "▸ SECURE GATEWAY ACCESS ◂",
        "Type the characters shown below",
    )
}

pub fn render_bmp_text_captcha_with_message(
    session_id: &str,
    captcha_id: &str,
    _is_threat: bool,
    _subtitle: &str,
    _instruction: &str,
) -> String {
    // Use the new template engine for consistent styling
    let engine = TemplateEngine::new();
    let branding = BrandingVars::from_env();

    let mut extra_vars = HashMap::new();
    extra_vars.insert(
        "CAPTCHA_IMAGE_URL".to_string(),
        format!("/gate/captcha/{}", captcha_id),
    );
    extra_vars.insert("SESSION_ID".to_string(), session_id.to_string());
    extra_vars.insert("CAPTCHA_TYPE".to_string(), "bmptext".to_string());

    engine.render_with_branding(TemplateType::Captcha, &branding, Some(&extra_vars))
}

/// Generate HTML for Emoji Selection captcha
pub fn render_emoji_captcha(
    session_id: &str,
    challenge: &EmojiChallenge,
    is_threat: bool,
) -> String {
    render_emoji_captcha_with_message(
        session_id,
        challenge,
        is_threat,
        "▸ EMOJI VERIFICATION ◂",
        &format!(
            "Click the <strong>{}</strong>",
            challenge.target_description
        ),
    )
}

pub fn render_emoji_captcha_with_message(
    session_id: &str,
    challenge: &EmojiChallenge,
    is_threat: bool,
    subtitle: &str,
    instruction: &str,
) -> String {
    let panel_class = if is_threat { "panel threat" } else { "panel" };

    let mut options_html = String::new();
    for opt in &challenge.options {
        options_html.push_str(&format!(
            r#"<button type="submit" name="selection" value="{}" class="option-btn">{}</button>"#,
            opt.index, opt.emoji
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>FORTIFY /// ACCESS CONTROL</title>
    <style>{}</style>
</head>
<body>
    <div class="{}">
        <h1>FORTIFY</h1>
        <div class="subtitle">{}</div>
        
        <div class="instruction">
            {}
        </div>
        
        <form method="POST" action="/gate/verify">
            <div class="captcha-container">
                {}
            </div>
            
            <input type="hidden" name="session_id" value="{}">
            <input type="hidden" name="captcha_type" value="emoji">
        </form>
        
        <div class="footer">
            <span>ONION-V3</span>
            <span>NO-JS</span>
        </div>
    </div>
</body>
</html>"#,
        captcha_css(),
        panel_class,
        subtitle,
        instruction,
        options_html,
        session_id
    )
}

/// Generate HTML for Direction/Arrow captcha
pub fn render_direction_captcha(
    session_id: &str,
    challenge: &DirectionChallenge,
    is_threat: bool,
) -> String {
    render_direction_captcha_with_message(
        session_id,
        challenge,
        is_threat,
        "▸ DIRECTION VERIFICATION ◂",
        &format!(
            "Click the arrow pointing <strong>{}</strong>",
            challenge.target_direction.name()
        ),
    )
}

pub fn render_direction_captcha_with_message(
    session_id: &str,
    challenge: &DirectionChallenge,
    is_threat: bool,
    subtitle: &str,
    instruction: &str,
) -> String {
    let panel_class = if is_threat { "panel threat" } else { "panel" };

    let mut options_html = String::new();
    for opt in &challenge.options {
        options_html.push_str(&format!(
            r#"<button type="submit" name="selection" value="{}" class="option-btn" style="font-size: 3rem;">{}</button>"#,
            opt.index, opt.direction.arrow_char()
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>FORTIFY /// ACCESS CONTROL</title>
    <style>{}</style>
</head>
<body>
    <div class="{}">
        <h1>FORTIFY</h1>
        <div class="subtitle">{}</div>
        
        <div class="instruction">
            {}
        </div>
        
        <form method="POST" action="/gate/verify">
            <div class="captcha-container">
                <div class="arrow-grid">
                    {}
                </div>
            </div>
            
            <input type="hidden" name="session_id" value="{}">
            <input type="hidden" name="captcha_type" value="direction">
        </form>
        
        <div class="footer">
            <span>ONION-V3</span>
            <span>NO-JS</span>
        </div>
    </div>
</body>
</html>"#,
        captcha_css(),
        panel_class,
        subtitle,
        instruction,
        options_html,
        session_id
    )
}

/// Generate HTML for Sequence captcha
pub fn render_sequence_captcha(
    session_id: &str,
    challenge: &SequenceChallenge,
    is_threat: bool,
) -> String {
    render_sequence_captcha_with_message(
        session_id,
        challenge,
        is_threat,
        "▸ SEQUENCE VERIFICATION ◂",
        &challenge.question_text,
    )
}

pub fn render_sequence_captcha_with_message(
    session_id: &str,
    challenge: &SequenceChallenge,
    is_threat: bool,
    subtitle: &str,
    instruction: &str,
) -> String {
    let panel_class = if is_threat { "panel threat" } else { "panel" };

    // Build sequence display
    let mut sequence_html = String::new();
    for item in &challenge.sequence_display {
        sequence_html.push_str(&format!(r#"<span class="sequence-item">{}</span>"#, item));
    }
    sequence_html.push_str(r#"<span class="sequence-item question">?</span>"#);

    // Build options
    let mut options_html = String::new();
    for opt in &challenge.options {
        options_html.push_str(&format!(
            r#"<button type="submit" name="selection" value="{}" class="option-btn small">{}</button>"#,
            opt.index, opt.display
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>FORTIFY /// ACCESS CONTROL</title>
    <style>{}</style>
</head>
<body>
    <div class="{}">
        <h1>FORTIFY</h1>
        <div class="subtitle">{}</div>
        
        <div class="instruction">
            {}
        </div>
        
        <div class="sequence-display">
            {}
        </div>
        
        <form method="POST" action="/gate/verify">
            <div class="captcha-container">
                {}
            </div>
            
            <input type="hidden" name="session_id" value="{}">
            <input type="hidden" name="captcha_type" value="sequence">
        </form>
        
        <div class="footer">
            <span>ONION-V3</span>
            <span>NO-JS</span>
        </div>
    </div>
</body>
</html>"#,
        captcha_css(),
        panel_class,
        subtitle,
        instruction,
        sequence_html,
        options_html,
        session_id
    )
}

/// Generate HTML for Word Unscramble captcha
pub fn render_word_unscramble_captcha(
    session_id: &str,
    challenge: &WordUnscrambleChallenge,
    is_threat: bool,
) -> String {
    render_word_unscramble_captcha_with_message(
        session_id,
        challenge,
        is_threat,
        "▸ WORD VERIFICATION ◂",
        "Unscramble the letters to form a word",
    )
}

pub fn render_word_unscramble_captcha_with_message(
    session_id: &str,
    challenge: &WordUnscrambleChallenge,
    is_threat: bool,
    subtitle: &str,
    instruction: &str,
) -> String {
    let panel_class = if is_threat { "panel threat" } else { "panel" };

    let hint_html = if let Some(ref hint) = challenge.hint {
        format!(r#"<div class="hint">{}</div>"#, hint)
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>FORTIFY /// ACCESS CONTROL</title>
    <style>{}</style>
</head>
<body>
    <div class="{}">
        <h1>FORTIFY</h1>
        <div class="subtitle">{}</div>
        
        <div class="instruction">
            {}
        </div>
        
        <div class="scrambled-word">{}</div>
        
        {}
        
        <form method="POST" action="/gate/verify">
            <input type="text" name="captcha" class="text-input" placeholder="TYPE THE WORD" required autocomplete="off" autofocus>
            <input type="hidden" name="session_id" value="{}">
            <input type="hidden" name="captcha_type" value="wordunscramble">
            
            <button type="submit" class="submit-btn">AUTHENTICATE</button>
        </form>
        
        <div class="footer">
            <span>ONION-V3</span>
            <span>NO-JS</span>
        </div>
    </div>
</body>
</html>"#,
        captcha_css(),
        panel_class,
        subtitle,
        instruction,
        challenge.scrambled,
        hint_html,
        session_id
    )
}

/// Generate HTML for Image Rotation captcha
pub fn render_image_rotation_captcha(
    session_id: &str,
    challenge: &ImageRotationChallenge,
    is_threat: bool,
) -> String {
    render_image_rotation_captcha_with_message(
        session_id,
        challenge,
        is_threat,
        "▸ ROTATION VERIFICATION ◂",
        &format!(
            "Click the <strong>{}</strong> that is upright",
            challenge.shape_name
        ),
    )
}

pub fn render_image_rotation_captcha_with_message(
    session_id: &str,
    challenge: &ImageRotationChallenge,
    is_threat: bool,
    subtitle: &str,
    instruction: &str,
) -> String {
    let panel_class = if is_threat { "panel threat" } else { "panel" };

    let mut options_html = String::new();
    for opt in &challenge.options {
        // CSS transform for rotation
        let transform = match opt.rotation.degrees() {
            90 => "transform: rotate(90deg);",
            180 => "transform: rotate(180deg);",
            270 => "transform: rotate(270deg);",
            _ => "",
        };
        options_html.push_str(&format!(
            r#"<button type="submit" name="selection" value="{}" class="option-btn" style="{}font-size: 3rem;">{}</button>"#,
            opt.index, transform, challenge.shape_char
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>FORTIFY /// ACCESS CONTROL</title>
    <style>{}</style>
</head>
<body>
    <div class="{}">
        <h1>FORTIFY</h1>
        <div class="subtitle">{}</div>
        
        <div class="instruction">
            {}
        </div>
        
        <form method="POST" action="/gate/verify">
            <div class="captcha-container">
                {}
            </div>
            
            <input type="hidden" name="session_id" value="{}">
            <input type="hidden" name="captcha_type" value="rotation">
        </form>
        
        <div class="footer">
            <span>ONION-V3</span>
            <span>NO-JS</span>
        </div>
    </div>
</body>
</html>"#,
        captcha_css(),
        panel_class,
        subtitle,
        instruction,
        options_html,
        session_id
    )
}

/// Generate HTML for Silhouette captcha
pub fn render_silhouette_captcha(
    session_id: &str,
    challenge: &SilhouetteChallenge,
    is_threat: bool,
) -> String {
    render_silhouette_captcha_with_message(
        session_id,
        challenge,
        is_threat,
        "▸ SILHOUETTE VERIFICATION ◂",
        "What does this silhouette show?",
    )
}

pub fn render_silhouette_captcha_with_message(
    session_id: &str,
    challenge: &SilhouetteChallenge,
    is_threat: bool,
    subtitle: &str,
    instruction: &str,
) -> String {
    let panel_class = if is_threat { "panel threat" } else { "panel" };

    let mut options_html = String::new();
    for opt in &challenge.options {
        options_html.push_str(&format!(
            r#"<button type="submit" name="selection" value="{}" class="option-btn text">{}</button>"#,
            opt.index, opt.description
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>FORTIFY /// ACCESS CONTROL</title>
    <style>{}</style>
</head>
<body>
    <div class="{}">
        <h1>FORTIFY</h1>
        <div class="subtitle">{}</div>
        
        <div class="instruction">
            {}
        </div>
        
        <div class="silhouette">{}</div>
        
        <form method="POST" action="/gate/verify">
            <div class="captcha-container" style="flex-direction: column; gap: 10px;">
                {}
            </div>
            
            <input type="hidden" name="session_id" value="{}">
            <input type="hidden" name="captcha_type" value="silhouette">
        </form>
        
        <div class="footer">
            <span>ONION-V3</span>
            <span>NO-JS</span>
        </div>
    </div>
</body>
</html>"#,
        captcha_css(),
        panel_class,
        subtitle,
        instruction,
        challenge.silhouette_symbol,
        options_html,
        session_id
    )
}

/// Render the appropriate captcha HTML based on captcha data
/// Generate appropriate messaging based on reason for challenge
pub fn reason_message(reason: Option<&str>) -> (&'static str, &'static str) {
    match reason {
        Some("rate_limit") => (
            "⚡ RATE LIMIT EXCEEDED ⚡",
            "Complete this challenge to prove you're human and continue with elevated access privileges."
        ),
        Some("demotion") => (
            "⚠ ACCESS DOWNGRADED ⚠",
            "Your trust level was reduced due to suspicious activity. Complete this challenge to restore access."
        ),
        _ => (
            "▸ SECURE GATEWAY ACCESS ◂",
            "Complete this verification to access the protected service."
        ),
    }
}

pub fn render_captcha_page(
    session_id: &str,
    captcha_id: &str,
    data: &CaptchaData,
    is_threat: bool,
) -> String {
    render_captcha_page_with_reason(session_id, captcha_id, data, is_threat, None)
}

pub fn render_captcha_page_with_reason(
    session_id: &str,
    captcha_id: &str,
    data: &CaptchaData,
    is_threat: bool,
    reason: Option<&str>,
) -> String {
    let (title, message) = reason_message(reason);
    match data {
        CaptchaData::BmpText { .. } => {
            render_bmp_text_captcha_with_message(session_id, captcha_id, is_threat, title, message)
        }
        CaptchaData::Emoji(challenge) => {
            // For emoji CAPTCHA, we need the specific instruction, not the generic message
            let instruction = format!("Select all <strong>{}</strong>", challenge.target_category);
            render_emoji_captcha_with_message(session_id, challenge, is_threat, title, &instruction)
        }
        CaptchaData::Direction(challenge) => {
            // For direction CAPTCHA, we need the specific instruction
            let instruction = format!(
                "Click the arrow pointing <strong>{}</strong>",
                challenge.target_direction.name()
            );
            render_direction_captcha_with_message(
                session_id,
                challenge,
                is_threat,
                title,
                &instruction,
            )
        }
        CaptchaData::Sequence(challenge) => {
            render_sequence_captcha_with_message(session_id, challenge, is_threat, title, message)
        }
        CaptchaData::WordUnscramble(challenge) => render_word_unscramble_captcha_with_message(
            session_id, challenge, is_threat, title, message,
        ),
        CaptchaData::ImageRotation(challenge) => render_image_rotation_captcha_with_message(
            session_id, challenge, is_threat, title, message,
        ),
        CaptchaData::Silhouette(challenge) => {
            render_silhouette_captcha_with_message(session_id, challenge, is_threat, title, message)
        }
    }
}

/// CSS for countdown timer
pub fn timer_css(timeout_seconds: u64) -> String {
    format!(
        r#"
    .timer-container {{
        text-align: center;
        margin-bottom: 15px;
    }}
    .timer-bar {{
        background: rgba(201, 162, 39, 0.2);
        border: 1px solid var(--brand-primary);
        height: 8px;
        width: 100%;
        overflow: hidden;
        margin-bottom: 8px;
    }}
    .timer-progress {{
        height: 100%;
        background: var(--brand-primary);
        width: 100%;
        animation: countdown {0}s linear forwards;
    }}
    .panel.threat .timer-progress {{
        background: var(--warning);
    }}
    @keyframes countdown {{
        from {{ width: 100%; }}
        to {{ width: 0%; }}
    }}
    .timer-text {{
        font-size: 0.75rem;
        color: var(--brand-primary);
        letter-spacing: 2px;
    }}
    .panel.threat .timer-text {{
        color: var(--warning);
    }}
    .expired-overlay {{
        display: none;
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(20, 20, 23, 0.95);
        z-index: 1000;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        animation: fadeIn 0.5s ease-out {0}s forwards;
        opacity: 0;
        pointer-events: none;
    }}
    @keyframes fadeIn {{
        to {{ 
            opacity: 1;
            pointer-events: auto;
        }}
    }}
    .expired-overlay.active {{
        display: flex;
    }}
    .expired-content {{
        text-align: center;
        padding: 40px;
        background: rgba(24, 24, 27, 0.95);
        border: 2px solid var(--brand-primary);
        box-shadow: 0 0 30px rgba(201, 162, 39, 0.4);
        max-width: 400px;
    }}
    .expired-icon {{
        font-size: 3rem;
        margin-bottom: 15px;
    }}
    .expired-title {{
        color: var(--brand-primary);
        font-size: 1.5rem;
        letter-spacing: 3px;
        margin-bottom: 10px;
    }}
    .expired-message {{
        color: #888;
        font-size: 0.85rem;
        margin-bottom: 20px;
    }}
    .refresh-btn {{
        background: var(--brand-primary);
        border: none;
        color: #000;
        padding: 14px 40px;
        font-family: inherit;
        font-size: 1rem;
        font-weight: 900;
        cursor: pointer;
        text-transform: uppercase;
        letter-spacing: 2px;
        text-decoration: none;
        display: inline-block;
        transition: all 0.2s;
    }}
    .refresh-btn:hover {{
        background: var(--brand-light);
        color: #000;
        box-shadow: 0 0 20px var(--brand-primary);
    }}
    "#,
        timeout_seconds
    )
}

/// Generate the timer HTML component
pub fn timer_html(timeout_seconds: u64) -> String {
    format!(
        r#"
    <div class="timer-container">
        <div class="timer-bar">
            <div class="timer-progress"></div>
        </div>
        <div class="timer-text">TIME REMAINING: {0} SECONDS</div>
    </div>
    <div class="expired-overlay active">
        <div class="expired-content">
            <div class="expired-icon">⏱</div>
            <div class="expired-title">TIME EXPIRED</div>
            <div class="expired-message">Your verification window has elapsed. Please refresh to try again.</div>
            <a href="" class="refresh-btn">⟳ REFRESH</a>
        </div>
    </div>
    "#,
        timeout_seconds
    )
}

/// Render captcha page with countdown timer
pub fn render_captcha_page_with_timer(
    session_id: &str,
    captcha_id: &str,
    data: &CaptchaData,
    is_threat: bool,
    timeout_seconds: u64,
) -> String {
    render_captcha_page_with_timer_and_reason(
        session_id,
        captcha_id,
        data,
        is_threat,
        timeout_seconds,
        None,
    )
}

pub fn render_captcha_page_with_timer_and_reason(
    session_id: &str,
    captcha_id: &str,
    data: &CaptchaData,
    is_threat: bool,
    timeout_seconds: u64,
    reason: Option<&str>,
) -> String {
    let base_page =
        render_captcha_page_with_reason(session_id, captcha_id, data, is_threat, reason);

    // Inject timer CSS into the <style> block and timer HTML after the panel opening
    let timer_css_content = timer_css(timeout_seconds);
    let timer_html_content = timer_html(timeout_seconds);

    // Insert timer CSS before </style>
    let with_css = base_page.replace("</style>", &format!("{}</style>", timer_css_content));

    // Insert timer HTML after the panel div opening (after class="panel...">)
    // We'll insert after the subtitle div for better placement
    let with_timer = if with_css.contains("<div class=\"subtitle\">") {
        // Insert after subtitle closing tag
        with_css.replacen(
            "</div>\n    <div class=\"instruction\">",
            &format!(
                "</div>\n    {}\n    <div class=\"instruction\">",
                timer_html_content
            ),
            1,
        )
    } else if with_css.contains("</h1>") {
        // Fallback: insert after h1
        with_css.replacen("</h1>", &format!("</h1>\n    {}", timer_html_content), 1)
    } else {
        // Last resort: just append to beginning of body
        with_css
    };

    with_timer
}
