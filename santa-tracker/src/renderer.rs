use crate::effects::{ChristmasTree, RgbEffect, Snowflake};
use crate::santa::SantaTracker;
use colored::*;
use crossterm::{cursor, execute, terminal};
use rand::Rng;
use std::io::{self, Write};

pub struct Renderer {
    width: u16,
    height: u16,
    snowflakes: Vec<Snowflake>,
    trees: Vec<ChristmasTree>,
    rgb_effect: RgbEffect,
    frame_count: u64,
}

impl Renderer {
    pub fn new() -> Result<Self, io::Error> {
        let (width, height) = terminal::size()?;
        let mut snowflakes = Vec::new();
        let mut rng = rand::thread_rng();

        // Initialize snowflakes
        for _ in 0..50 {
            snowflakes.push(Snowflake::new(0, width));
        }

        // Initialize trees at bottom
        let trees = vec![
            ChristmasTree::new(5, height - 8, 4),
            ChristmasTree::new(width - 15, height - 8, 4),
        ];

        Ok(Self {
            width,
            height,
            snowflakes,
            trees,
            rgb_effect: RgbEffect::new(),
            frame_count: 0,
        })
    }

    pub fn update(&mut self) {
        self.frame_count += 1;
        self.rgb_effect.update();

        // Update terminal size
        if let Ok((w, h)) = terminal::size() {
            self.width = w;
            self.height = h;
        }

        // Update snowflakes
        self.snowflakes.retain_mut(|sf| sf.update(self.height));

        // Add new snowflakes occasionally
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.3) && self.snowflakes.len() < 100 {
            self.snowflakes.push(Snowflake::new(0, self.width));
        }
    }

    pub fn render(&self, tracker: &SantaTracker) -> Result<(), io::Error> {
        let mut stdout = io::stdout();

        // Clear screen
        execute!(stdout, terminal::Clear(terminal::ClearType::All))?;

        // Render snowflakes
        for snowflake in &self.snowflakes {
            if snowflake.y >= 0.0 && (snowflake.y as u16) < self.height && snowflake.x < self.width {
                execute!(
                    stdout,
                    cursor::MoveTo(snowflake.x, snowflake.y as u16)
                )?;
                write!(stdout, "{}", snowflake.character.to_string().bright_white())?;
            }
        }

        // Render title with RGB effect
        let title = "🎅 SANTA TRACKER 2025 🎄";
        let title_x = (self.width.saturating_sub(title.len() as u16)) / 2;
        execute!(stdout, cursor::MoveTo(title_x, 1))?;
        
        for (i, ch) in title.chars().enumerate() {
            let colored = self.rgb_effect.colorize_text(&ch.to_string(), i as f64 * 10.0);
            write!(stdout, "{}", colored)?;
        }

        // Render border with christmas colors
        let border_y = 3;
        execute!(stdout, cursor::MoveTo(2, border_y))?;
        write!(stdout, "{}", "═".repeat(self.width.saturating_sub(4) as usize).red())?;

        // Render Santa status
        let status = tracker.get_status_message();
        let status_x = (self.width.saturating_sub(status.len() as u16)) / 2;
        execute!(stdout, cursor::MoveTo(status_x, 5))?;
        write!(stdout, "{}", status.bright_yellow())?;

        // Render location info
        let info_y = 7;
        let info_lines = vec![
            format!("📍 Current: {}", tracker.current_location.name).bright_cyan().to_string(),
            format!("🎯 Next: {}", tracker.next_location.name).bright_magenta().to_string(),
            format!("⚡ Speed: {:.0} km/h", tracker.speed).bright_green().to_string(),
            format!("🎁 Presents Delivered: {}", Self::format_number(tracker.presents_delivered)).bright_yellow().to_string(),
        ];

        for (i, line) in info_lines.iter().enumerate() {
            let x = 5;
            execute!(stdout, cursor::MoveTo(x, info_y + i as u16))?;
            write!(stdout, "{}", line)?;
        }

        // Render progress bar
        let progress_y = info_y + 5;
        let progress_width = self.width.saturating_sub(20);
        let filled = (progress_width as f64 * tracker.progress) as u16;
        
        execute!(stdout, cursor::MoveTo(5, progress_y))?;
        write!(stdout, "{}", "Progress: ".bright_white())?;
        write!(stdout, "{}", "█".repeat(filled as usize).green())?;
        write!(stdout, "{}", "░".repeat((progress_width - filled) as usize).bright_black())?;
        write!(stdout, " {}%", (tracker.progress * 100.0) as u16)?;

        // Render Christmas trees
        for tree in &self.trees {
            let tree_lines = tree.render();
            for (i, line) in tree_lines.iter().enumerate() {
                let y = tree.y + i as u16;
                if y < self.height {
                    execute!(stdout, cursor::MoveTo(tree.x, y))?;
                    write!(stdout, "{}", line)?;
                }
            }
        }

        // Render sleigh animation
        let sleigh_y = 12;
        let sleigh_x = 5 + ((self.frame_count / 2) % 30) as u16;
        if sleigh_x < self.width - 10 {
            execute!(stdout, cursor::MoveTo(sleigh_x, sleigh_y))?;
            write!(stdout, "{}", "🛷🦌".bright_red())?;
        }

        // Render footer
        let footer = "Press 'q' or ESC to quit";
        let footer_x = (self.width.saturating_sub(footer.len() as u16)) / 2;
        execute!(stdout, cursor::MoveTo(footer_x, self.height - 2))?;
        write!(stdout, "{}", footer.bright_black())?;

        stdout.flush()?;
        Ok(())
    }

    fn format_number(n: u64) -> String {
        n.to_string()
            .as_bytes()
            .rchunks(3)
            .rev()
            .map(std::str::from_utf8)
            .collect::<Result<Vec<&str>, _>>()
            .unwrap()
            .join(",")
    }
}
