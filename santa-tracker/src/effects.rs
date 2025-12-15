use rand::Rng;
use colored::*;

#[derive(Clone)]
pub struct Snowflake {
    pub x: u16,
    pub y: f64,
    pub speed: f64,
    pub character: char,
}

impl Snowflake {
    pub fn new(x: u16, max_width: u16) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            x: if x == 0 { rng.gen_range(0..max_width) } else { x },
            y: 0.0,
            speed: rng.gen_range(0.1..0.4),
            character: if rng.gen_bool(0.5) { '❄' } else { '.' },
        }
    }

    pub fn update(&mut self, max_height: u16) -> bool {
        self.y += self.speed;
        self.y < max_height as f64
    }
}

pub struct ChristmasTree {
    pub x: u16,
    pub y: u16,
    pub size: u16,
}

impl ChristmasTree {
    pub fn new(x: u16, y: u16, size: u16) -> Self {
        Self { x, y, size }
    }

    pub fn render(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut rng = rand::thread_rng();

        // Star on top
        lines.push(format!("{}⭐{}", " ".repeat(self.size as usize), ""));

        // Tree layers
        for i in 0..self.size {
            let width = 1 + (i * 2);
            let padding = self.size - i;
            let mut layer = String::new();
            
            layer.push_str(&" ".repeat(padding as usize));
            
            for j in 0..width {
                if j == 0 || j == width - 1 {
                    layer.push_str(&"🌲".green().to_string());
                } else {
                    // Random ornaments
                    let ornament = match rng.gen_range(0..6) {
                        0 => "●".red(),
                        1 => "●".yellow(),
                        2 => "●".blue(),
                        3 => "●".magenta(),
                        4 => "○".bright_white(),
                        _ => "🌲".green(),
                    };
                    layer.push_str(&ornament.to_string());
                }
            }
            lines.push(layer);
        }

        // Tree trunk
        let trunk_padding = self.size as usize - 1;
        lines.push(format!("{}{}{}",
            " ".repeat(trunk_padding),
            "|||".truecolor(139, 69, 19),
            ""
        ));
        lines.push(format!("{}{}{}",
            " ".repeat(trunk_padding),
            "|||".truecolor(139, 69, 19),
            ""
        ));

        lines
    }
}

pub struct RgbEffect {
    pub hue: f64,
}

impl RgbEffect {
    pub fn new() -> Self {
        Self { hue: 0.0 }
    }

    pub fn update(&mut self) {
        self.hue = (self.hue + 2.0) % 360.0;
    }

    pub fn get_rgb(&self, offset: f64) -> (u8, u8, u8) {
        let hue = (self.hue + offset) % 360.0;
        Self::hsv_to_rgb(hue, 1.0, 1.0)
    }

    fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        (
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }

    pub fn colorize_text(&self, text: &str, offset: f64) -> ColoredString {
        let (r, g, b) = self.get_rgb(offset);
        text.truecolor(r, g, b)
    }
}
