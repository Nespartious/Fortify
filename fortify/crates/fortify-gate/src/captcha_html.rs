//! HTML Rendering for Multi-Type Captcha System
//!
//! Generates pure HTML/CSS forms for each captcha type.
//! NO JAVASCRIPT - all interaction via form submissions.

use crate::captcha_types::*;

/// Common CSS styles for captcha pages
pub fn captcha_css() -> &'static str {
    r#"
    :root {
        --bg-color: #0d0211;
        --panel-bg: #150520;
        --neon-pink: #d500f9;
        --neon-cyan: #00e5ff;
        --neon-green: #00e676;
        --neon-orange: #ff9100;
        --grid-color: rgba(213, 0, 249, 0.15);
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
        background-color: var(--bg-color);
        background-image: 
            linear-gradient(var(--grid-color) 1px, transparent 1px),
            linear-gradient(90deg, var(--grid-color) 1px, transparent 1px);
        background-size: 50px 50px;
        font-family: 'Courier New', Courier, monospace;
        color: var(--neon-cyan);
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        overflow-x: hidden;
    }
    .panel {
        background: rgba(21, 5, 32, 0.95);
        border: 2px solid var(--neon-cyan);
        box-shadow: 0 0 20px rgba(0, 229, 255, 0.3), inset 0 0 30px rgba(0,0,0,0.8);
        padding: 2.5rem 2rem;
        width: 100%;
        max-width: 520px;
        position: relative;
        border-radius: 4px;
    }
    .panel.threat {
        border-color: var(--neon-orange);
        box-shadow: 0 0 20px rgba(255, 145, 0, 0.3), inset 0 0 30px rgba(0,0,0,0.8);
    }
    h1 {
        text-align: center;
        margin: 0 0 8px 0;
        color: var(--neon-pink);
        text-shadow: 2px 2px 0px rgba(255,0,255,0.4);
        font-size: 2rem;
        letter-spacing: 4px;
        text-transform: uppercase;
        font-weight: 900;
    }
    .subtitle {
        text-align: center;
        color: #fff;
        margin-bottom: 25px;
        font-size: 0.8rem;
        letter-spacing: 2px;
        opacity: 0.7;
        border-bottom: 1px solid var(--neon-pink);
        padding-bottom: 10px;
    }
    .instruction {
        text-align: center;
        color: var(--neon-cyan);
        font-size: 1rem;
        margin-bottom: 20px;
        padding: 12px;
        background: rgba(0, 229, 255, 0.1);
        border: 1px solid var(--neon-cyan);
    }
    .instruction strong {
        color: var(--neon-pink);
        font-size: 1.2rem;
    }
    .captcha-container {
        background: #000;
        border: 1px solid var(--neon-cyan);
        padding: 20px;
        margin-bottom: 25px;
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        gap: 15px;
        min-height: 100px;
    }
    .option-btn {
        background: rgba(21, 5, 32, 0.9);
        border: 2px solid var(--neon-cyan);
        color: var(--neon-cyan);
        padding: 20px;
        font-size: 2.5rem;
        cursor: pointer;
        transition: all 0.2s;
        min-width: 80px;
        text-align: center;
    }
    .option-btn:hover {
        background: var(--neon-cyan);
        color: #000;
        box-shadow: 0 0 15px var(--neon-cyan);
    }
    .option-btn.small {
        padding: 15px 25px;
        font-size: 1.2rem;
    }
    .option-btn.text {
        font-size: 1rem;
        padding: 12px 20px;
    }
    .text-input {
        width: 100%;
        background: rgba(0, 0, 0, 0.6);
        border: 1px solid var(--neon-cyan);
        border-left: 5px solid var(--neon-cyan);
        color: var(--neon-green);
        padding: 15px;
        font-family: inherit;
        font-size: 1.4rem;
        text-align: center;
        outline: none;
        margin-bottom: 20px;
        text-transform: uppercase;
        letter-spacing: 2px;
    }
    .text-input:focus {
        box-shadow: 0 0 15px rgba(0, 229, 255, 0.4);
        background: rgba(0, 229, 255, 0.1);
    }
    .submit-btn {
        width: 100%;
        background: var(--neon-cyan);
        border: none;
        color: #000;
        padding: 16px;
        font-family: inherit;
        font-size: 1.1rem;
        font-weight: 900;
        cursor: pointer;
        text-transform: uppercase;
        letter-spacing: 3px;
        transition: all 0.2s;
    }
    .submit-btn:hover {
        background: var(--neon-pink);
        color: #fff;
        box-shadow: 0 0 20px var(--neon-pink);
    }
    .footer {
        margin-top: 20px;
        display: flex;
        justify-content: space-between;
        font-size: 0.65rem;
        color: #555;
        text-transform: uppercase;
    }
    .sequence-display {
        display: flex;
        gap: 15px;
        justify-content: center;
        align-items: center;
        margin-bottom: 20px;
        font-size: 2rem;
    }
    .sequence-item {
        background: rgba(0, 229, 255, 0.1);
        border: 1px solid var(--neon-cyan);
        padding: 15px 20px;
        min-width: 50px;
        text-align: center;
    }
    .sequence-item.question {
        color: var(--neon-pink);
        border-color: var(--neon-pink);
    }
    .silhouette {
        font-size: 5rem;
        padding: 20px;
        background: #000;
        border: 2px solid var(--neon-cyan);
        margin-bottom: 20px;
        text-align: center;
        filter: grayscale(100%) brightness(0.3);
    }
    .arrow-grid {
        display: grid;
        grid-template-columns: repeat(2, 1fr);
        gap: 15px;
        max-width: 300px;
        margin: 0 auto;
    }
    .scrambled-word {
        font-size: 2.5rem;
        letter-spacing: 8px;
        text-align: center;
        color: var(--neon-pink);
        margin-bottom: 20px;
        padding: 15px;
        background: rgba(213, 0, 249, 0.1);
        border: 1px solid var(--neon-pink);
    }
    .hint {
        text-align: center;
        font-size: 0.9rem;
        color: var(--neon-orange);
        margin-bottom: 15px;
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
    is_threat: bool,
    subtitle: &str,
    instruction: &str,
) -> String {
    let panel_class = if is_threat { "panel threat" } else { "panel" };

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
            <div class="captcha-container" style="justify-content: center;">
                <img src="/gate/captcha/{}" alt="Security Challenge" style="border: 1px solid #222;">
            </div>
            
            <input type="text" name="captcha" class="text-input" placeholder="ENTER CODE" required autocomplete="off" autofocus>
            <input type="hidden" name="session_id" value="{}">
            <input type="hidden" name="captcha_type" value="bmptext">
            
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
        captcha_id,
        session_id
    )
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
        &format!("Select all <strong>{}</strong>", challenge.target_category),
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
        &format!("What does this silhouette show?"),
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
        background: rgba(0, 229, 255, 0.2);
        border: 1px solid var(--neon-cyan);
        height: 8px;
        width: 100%;
        overflow: hidden;
        margin-bottom: 8px;
    }}
    .timer-progress {{
        height: 100%;
        background: var(--neon-cyan);
        width: 100%;
        animation: countdown {0}s linear forwards;
    }}
    .panel.threat .timer-progress {{
        background: var(--neon-orange);
    }}
    @keyframes countdown {{
        from {{ width: 100%; }}
        to {{ width: 0%; }}
    }}
    .timer-text {{
        font-size: 0.75rem;
        color: var(--neon-cyan);
        letter-spacing: 2px;
    }}
    .panel.threat .timer-text {{
        color: var(--neon-orange);
    }}
    .expired-overlay {{
        display: none;
        position: fixed;
        top: 0;
        left: 0;
        right: 0;
        bottom: 0;
        background: rgba(13, 2, 17, 0.95);
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
        background: rgba(21, 5, 32, 0.95);
        border: 2px solid var(--neon-pink);
        box-shadow: 0 0 30px rgba(213, 0, 249, 0.4);
        max-width: 400px;
    }}
    .expired-icon {{
        font-size: 3rem;
        margin-bottom: 15px;
    }}
    .expired-title {{
        color: var(--neon-pink);
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
        background: var(--neon-cyan);
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
        background: var(--neon-pink);
        color: #fff;
        box-shadow: 0 0 20px var(--neon-pink);
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
