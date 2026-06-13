//! Ratatui layout and rendering.

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Length(8), // status panel
        Constraint::Min(3),    // strobe display
        Constraint::Length(3), // help
    ])
    .split(frame.area());

    render_title(frame, chunks[0]);
    render_status(frame, chunks[1], app);
    render_strobe(frame, chunks[2], app);
    render_help(frame, chunks[3]);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled("strobetune", Style::default().fg(Color::Cyan)),
        Span::raw("  —  analog-style strobe tuner"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::bordered());
    frame.render_widget(title, area);
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let transpose = if app.transpose == 0 {
        "0".to_string()
    } else {
        format!("{:+}", app.transpose)
    };

    fn label(k: &'static str) -> Span<'static> {
        Span::styled(k, Style::default().fg(Color::DarkGray))
    }
    fn value(v: String) -> Span<'static> {
        Span::styled(v, Style::default().fg(Color::White))
    }

    let mut lines = vec![
        Line::from(vec![label("Tuning:    "), value(app.tuning_name())]),
        Line::from(vec![label("Transpose: "), value(transpose)]),
        Line::from({
            let mut spans = vec![
                label("String:    "),
                value(format!("{} / {}", app.string_idx + 1, app.current_string().label)),
            ];
            if app.auto {
                spans.push(Span::styled(
                    "  [AUTO]",
                    Style::default().fg(Color::Magenta),
                ));
            }
            spans
        }),
        Line::from(vec![
            label("Reference: "),
            value(format!("{:.2} Hz", app.active_freq())),
        ]),
    ];

    // Input level bar + lamp indicator.
    let bar_width = 24usize;
    let filled = (app.strobe.level * bar_width as f32).round() as usize;
    let filled = filled.min(bar_width);
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);
    let (lamp_glyph, lamp_color) = if app.strobe.lamp_active {
        ("● LAMP", Color::Yellow)
    } else {
        ("○ lamp", Color::DarkGray)
    };
    lines.push(Line::from(vec![
        label("Level:     "),
        Span::styled(bar, Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(lamp_glyph, Style::default().fg(lamp_color)),
    ]));
    lines.push(Line::from(vec![label("Audio:     "), value(app.audio.status.clone())]));

    let panel = Paragraph::new(lines).block(Block::bordered().title("status"));
    frame.render_widget(panel, area);
}

fn render_strobe(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered()
        .title("strobe  ·  matched = still   sharp/flat = drift");
    // Inner dimensions (all-borders inset is 1 on each side).
    let width = area.width.saturating_sub(2) as usize;
    let height = area.height.saturating_sub(2) as usize;

    if width == 0 || height == 0 {
        frame.render_widget(block, area);
        return;
    }

    let peak = app.strobe.peak();

    // Hue is driven by how fast the strobe image is rotating: a still (in-tune)
    // band sits at a calm teal and shifts toward soft violet as the drift speeds
    // up. This reads the apparent motion of the *display*, not the input's pitch
    // — no frequency or cents is computed.
    let drift_t = (app.strobe.drift / config::DRIFT_FULL_RED).clamp(0.0, 1.0);
    let hue = config::DRIFT_HUE_STILL
        + drift_t * (config::DRIFT_HUE_FAST - config::DRIFT_HUE_STILL);

    // One row of vertical bands; each column samples the wheel at its position.
    let mut row: Vec<Span> = Vec::with_capacity(width);
    for c in 0..width {
        // Map columns right-to-left around the wheel so apparent drift matches
        // intuition: a sharp (too-high) string drifts the band rightward, a
        // flat (too-low) string drifts it leftward.
        let pos = 1.0 - c as f32 / width as f32;
        let b = app.strobe.brightness_at(pos, peak);
        // Brightness tracks accumulated lamp energy; at b≈0 the bar reads empty.
        let (r, g, bl) = hsv_to_rgb(hue, config::DRIFT_SATURATION, b);
        row.push(Span::styled("█", Style::default().fg(Color::Rgb(r, g, bl))));
    }

    // Replicate the band pattern down every row.
    let lines: Vec<Line> = (0..height).map(|_| Line::from(row.clone())).collect();
    let strobe = Paragraph::new(lines).block(block);
    frame.render_widget(strobe, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help = Paragraph::new(
        "1-6 string | a auto | t/T tuning | [ ] transpose | 0 reset | q quit",
    )
    .alignment(Alignment::Center)
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::bordered());
    frame.render_widget(help, area);
}

/// HSV → RGB (h in degrees, s and v in [0, 1]).
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
