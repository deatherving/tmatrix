use std::io::{self, BufWriter, Write};
use std::time::{Duration, Instant};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use rand::Rng;


const BINARY_GLYPHS: &[char] = &['0', '1'];

const LETTERS_GLYPHS: &[char] = &[
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
    'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

struct Config {
    glyphs: &'static [char],
    /// Screen columns occupied by each character (1 for narrow, 2 for full-width).
    char_width: u16,
}

fn parse_args() -> Config {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--binary" || a == "-b") {
        Config { glyphs: BINARY_GLYPHS, char_width: 1 }
    } else {
        // --letters / -l is the default; accepted explicitly too
        Config { glyphs: LETTERS_GLYPHS, char_width: 1 }
    }
}

fn rand_glyph(glyphs: &[char], rng: &mut impl Rng) -> char {
    glyphs[rng.gen_range(0..glyphs.len())]
}

// depth=0 is the leading edge, increasing depth fades to black
fn stream_color(depth: u16) -> Color {
    match depth {
        0 => Color::Rgb { r: 255, g: 255, b: 255 },
        1 => Color::Rgb { r: 180, g: 255, b: 180 },
        2..=3 => Color::Rgb { r: 0, g: 255, b: 65 },
        4..=8 => Color::Rgb { r: 0, g: 210, b: 50 },
        9..=15 => Color::Rgb { r: 0, g: 140, b: 35 },
        16..=24 => Color::Rgb { r: 0, g: 75, b: 20 },
        _ => Color::Rgb { r: 0, g: 35, b: 10 },
    }
}

struct Drop {
    col: u16, // logical column
    head: i32,
    length: u16,
    speed: u8,
    tick: u8,
    chars: Vec<char>,
    active: bool,
    delay: u16,
}

impl Drop {
    fn new(col: u16, rows: u16, glyphs: &[char], rng: &mut impl Rng) -> Self {
        let length = Self::rand_length(rows, rng);
        Drop {
            col,
            head: -(rng.gen_range(0..rows.max(1)) as i32),
            length,
            speed: rng.gen_range(1..=3),
            tick: 0,
            chars: (0..length).map(|_| rand_glyph(glyphs, rng)).collect(),
            active: true,
            delay: 0,
        }
    }

    fn rand_length(rows: u16, rng: &mut impl Rng) -> u16 {
        let max = rows.max(10).min(50);
        rng.gen_range(8..=max)
    }

    fn update(&mut self, glyphs: &[char], rng: &mut impl Rng, rows: u16) {
        if !self.active {
            if self.delay > 0 {
                self.delay -= 1;
            } else {
                self.length = Self::rand_length(rows, rng);
                self.chars = (0..self.length).map(|_| rand_glyph(glyphs, rng)).collect();
                self.speed = rng.gen_range(1..=3);
                self.head = -(rng.gen_range(0i32..5));
                self.active = true;
            }
            return;
        }

        self.chars[0] = rand_glyph(glyphs, rng);

        self.tick += 1;
        if self.tick >= self.speed {
            self.tick = 0;
            self.head += 1;
            if rng.gen_bool(0.15) {
                let idx = rng.gen_range(0..self.chars.len());
                self.chars[idx] = rand_glyph(glyphs, rng);
            }
        }

        if self.head >= rows as i32 + self.length as i32 {
            self.active = false;
            self.delay = rng.gen_range(0..15);
        }
    }

    fn stamp(&self, buf: &mut Vec<Vec<Cell>>, rows: u16) {
        if !self.active {
            return;
        }
        for i in 0u16..self.length {
            let row = self.head - i as i32;
            if row >= 0 && row < rows as i32 {
                buf[row as usize][self.col as usize] = Cell::Glyph {
                    ch: self.chars[i as usize],
                    color: stream_color(i),
                };
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum Cell {
    Empty,
    Glyph { ch: char, color: Color },
}

fn make_grid(rows: u16, logical_cols: u16) -> Vec<Vec<Cell>> {
    vec![vec![Cell::Empty; logical_cols as usize]; rows as usize]
}

fn build_drops(logical_cols: u16, rows: u16, glyphs: &[char], rng: &mut impl Rng) -> Vec<Drop> {
    let mut drops = Vec::with_capacity(logical_cols as usize * 2);
    for c in 0..logical_cols {
        drops.push(Drop::new(c, rows, glyphs, rng));
        let mut b = Drop::new(c, rows, glyphs, rng);
        b.head -= (rows / 2) as i32;
        drops.push(b);
    }
    drops
}

fn main() -> io::Result<()> {
    let mut out = BufWriter::new(io::stdout());
    enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, Hide, Clear(ClearType::All), SetAttribute(Attribute::Bold))?;

    let result = run(&mut out);

    let _ = out.flush();
    let _ = execute!(out, Show, ResetColor, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    result
}

fn run<W: Write>(out: &mut W) -> io::Result<()> {
    let mut rng = rand::thread_rng();
    let config = parse_args();
    let (mut cols, mut rows) = terminal::size()?;
    let mut logical_cols = (cols / config.char_width).max(1);

    let mut drops = build_drops(logical_cols, rows, config.glyphs, &mut rng);
    let mut curr = make_grid(rows, logical_cols);
    let mut prev = make_grid(rows, logical_cols);

    let frame_dur = Duration::from_millis(50);
    let mut last_frame = Instant::now();

    loop {
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter,
                    ..
                }) => return Ok(()),
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers,
                    ..
                }) if modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
                Event::Resize(new_cols, new_rows) => {
                    cols = new_cols;
                    rows = new_rows;
                    logical_cols = (cols / config.char_width).max(1);
                    drops = build_drops(logical_cols, rows, config.glyphs, &mut rng);
                    curr = make_grid(rows, logical_cols);
                    prev = make_grid(rows, logical_cols);
                    execute!(out, Clear(ClearType::All))?;
                }
                _ => {}
            }
        }

        let now = Instant::now();
        if now.duration_since(last_frame) < frame_dur {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        last_frame = now;

        for row in &mut curr {
            for cell in row.iter_mut() {
                *cell = Cell::Empty;
            }
        }
        for drop in &mut drops {
            drop.update(config.glyphs, &mut rng, rows);
            drop.stamp(&mut curr, rows);
        }

        // Render only changed cells. Each logical column maps to char_width screen columns.
        let clear_str = " ".repeat(config.char_width as usize);
        for r in 0..rows as usize {
            for c in 0..logical_cols as usize {
                if curr[r][c] == prev[r][c] {
                    continue;
                }
                let screen_col = c as u16 * config.char_width;
                queue!(out, MoveTo(screen_col, r as u16))?;
                match &curr[r][c] {
                    Cell::Glyph { ch, color } => {
                        queue!(out, SetForegroundColor(*color), Print(ch))?;
                    }
                    Cell::Empty => {
                        queue!(out, Print(&clear_str))?;
                    }
                }
            }
        }

        out.flush()?;
        std::mem::swap(&mut curr, &mut prev);
    }
}
