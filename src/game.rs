use piston_window::types::Color;
use piston_window::*;

use rand::{thread_rng, Rng};

use crate::ai::{self, Strategy};
use crate::draw::{draw_block, draw_rectangle};
use crate::pathfinding::{Algorithm, Pos};
use crate::snake::{Direction, Snake};

const FOOD_COLOR: Color = [0.8, 0.0, 0.0, 1.0];
const BORDER_COLOR: Color = [0.0, 0.0, 0.0, 1.0];
const GAMEOVER_COLOR: Color = [0.9, 0.0, 0.0, 0.5];
const WIN_COLOR: Color = [0.0, 0.6, 0.9, 0.5];
const PATH_COLOR: Color = [0.2, 0.4, 1.0, 0.35];

const MOVING_PERIOD: f64 = 0.1;
const FAST_MOVING_PERIOD: f64 = 0.02;
const RESTART_TIME: f64 = 1.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Manual,
    Auto(Algorithm),
}

impl Mode {
    fn name(&self) -> &'static str {
        match self {
            Mode::Manual => "Manual",
            Mode::Auto(a) => a.name(),
        }
    }
}

pub struct Game {
    snake: Snake,

    food_exists: bool,
    food_x: i32,
    food_y: i32,

    width: i32,
    height: i32,

    game_over: bool,
    won: bool,
    waiting_time: f64,

    mode: Mode,
    fast: bool,
    show_path: bool,

    // statistik untuk ditampilkan di judul jendela
    score: usize,
    best_score: usize,
    last_expanded: usize,
    total_expanded: usize,
    steps: usize,
    strategy: Option<Strategy>,
    planned_path: Vec<Pos>,
}

impl Game {
    pub fn new(width: i32, height: i32) -> Game {
        Game {
            snake: Snake::new(2, 2),
            waiting_time: 0.0,
            food_exists: true,
            food_x: 6,
            food_y: 4,
            width,
            height,
            game_over: false,
            won: false,
            mode: Mode::Auto(Algorithm::AStar),
            fast: false,
            show_path: true,
            score: 0,
            best_score: 0,
            last_expanded: 0,
            total_expanded: 0,
            steps: 0,
            strategy: None,
            planned_path: Vec::new(),
        }
    }

    pub fn key_pressed(&mut self, key: Key) {
        // Tombol pengaturan berlaku kapan saja.
        match key {
            Key::D1 | Key::M => return self.set_mode(Mode::Manual),
            Key::D2 | Key::B => return self.set_mode(Mode::Auto(Algorithm::Bfs)),
            Key::D3 | Key::A => return self.set_mode(Mode::Auto(Algorithm::AStar)),
            Key::F => {
                self.fast = !self.fast;
                return;
            }
            Key::P => {
                self.show_path = !self.show_path;
                return;
            }
            Key::R => return self.restart(),
            _ => {}
        }

        if self.game_over || self.mode != Mode::Manual {
            return;
        }

        // Hanya tombol panah yang menggerakkan ular (sebelumnya tombol apa pun
        // membuat ular maju satu langkah ekstra).
        let dir = match key {
            Key::Up => Direction::Up,
            Key::Down => Direction::Down,
            Key::Left => Direction::Left,
            Key::Right => Direction::Right,
            _ => return,
        };

        if dir == self.snake.head_direction().opposite() {
            return;
        }

        self.update_snake(Some(dir));
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.planned_path.clear();
        self.strategy = None;
    }

    pub fn title(&self) -> String {
        let mut t = format!(
            "Snake | Mode: {} | Skor: {} | Terbaik: {}",
            self.mode.name(),
            self.score,
            self.best_score
        );
        if let Mode::Auto(_) = self.mode {
            let avg = if self.steps > 0 { self.total_expanded / self.steps } else { 0 };
            t += &format!(" | Node: {} (rata2 {})", self.last_expanded, avg);
            if let Some(s) = self.strategy {
                t += &format!(" | {}", s.name());
            }
        }
        if self.fast {
            t += " | CEPAT";
        }
        t
    }

    pub fn draw(&self, con: &Context, g: &mut G2d) {
        if self.show_path && !self.game_over {
            for &(x, y) in &self.planned_path {
                draw_block(PATH_COLOR, x, y, con, g);
            }
        }

        self.snake.draw(con, g);

        if self.food_exists {
            draw_block(FOOD_COLOR, self.food_x, self.food_y, con, g);
        }

        draw_rectangle(BORDER_COLOR, 0, 0, self.width, 1, con, g);
        draw_rectangle(BORDER_COLOR, 0, self.height - 1, self.width, 1, con, g);
        draw_rectangle(BORDER_COLOR, 0, 0, 1, self.height, con, g);
        draw_rectangle(BORDER_COLOR, self.width - 1, 0, 1, self.height, con, g);

        if self.game_over {
            let color = if self.won { WIN_COLOR } else { GAMEOVER_COLOR };
            draw_rectangle(color, 0, 0, self.width, self.height, con, g);
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        self.waiting_time += delta_time;

        if self.game_over {
            if self.waiting_time > RESTART_TIME {
                self.restart();
            }
            return;
        }

        if !self.food_exists {
            self.add_food();
            if self.game_over {
                return; // papan penuh = menang
            }
        }

        let period = if self.fast { FAST_MOVING_PERIOD } else { MOVING_PERIOD };
        if self.waiting_time > period {
            let dir = match self.mode {
                Mode::Manual => None,
                Mode::Auto(algo) => self.auto_direction(algo),
            };
            self.update_snake(dir);
        }
    }

    /// Hitung arah berikutnya memakai BFS / A*.
    fn auto_direction(&mut self, algo: Algorithm) -> Option<Direction> {
        let body = self.snake.body_positions();
        let food = if self.food_exists { Some((self.food_x, self.food_y)) } else { None };

        let decision = ai::decide(algo, self.width, self.height, &body, food);
        self.last_expanded = decision.expanded;
        self.total_expanded += decision.expanded;
        self.steps += 1;
        self.strategy = Some(decision.strategy);
        self.planned_path = decision.path;

        // Kalau benar-benar terjebak, biarkan ular lurus (dan kalah).
        let (nx, ny) = decision.next?;
        let (hx, hy) = body[0];
        match (nx - hx, ny - hy) {
            (0, -1) => Some(Direction::Up),
            (0, 1) => Some(Direction::Down),
            (-1, 0) => Some(Direction::Left),
            (1, 0) => Some(Direction::Right),
            _ => None,
        }
    }

    fn check_eating(&mut self) {
        let (head_x, head_y) = self.snake.head_position();
        if self.food_exists && self.food_x == head_x && self.food_y == head_y {
            self.food_exists = false;
            self.snake.restore_tail();
            self.score += 1;
            self.best_score = self.best_score.max(self.score);
        }
    }

    fn check_if_snake_alive(&mut self, dir: Option<Direction>) -> bool {
        let (next_x, next_y) = self.snake.next_head(dir);

        if self.snake.overlap_tail(next_x, next_y) {
            return false;
        }

        next_x > 0 && next_y > 0 && next_x < self.width - 1 && next_y < self.height - 1
    }

    fn add_food(&mut self) {
        // Kumpulkan semua sel kosong di DALAM dinding, lalu pilih satu secara acak.
        // Ini menjamin makanan selalu di area yang terlihat dan tidak pernah
        // terjadi loop tak berujung saat papan hampir penuh.
        let occupied = self.snake.body_positions();
        let free: Vec<Pos> = (1..self.height - 1)
            .flat_map(|y| (1..self.width - 1).map(move |x| (x, y)))
            .filter(|p| !occupied.contains(p))
            .collect();

        if free.is_empty() {
            self.won = true;
            self.game_over = true;
            self.waiting_time = 0.0;
            return;
        }

        let (x, y) = free[thread_rng().gen_range(0..free.len())];
        self.food_x = x;
        self.food_y = y;
        self.food_exists = true;
    }

    fn update_snake(&mut self, dir: Option<Direction>) {
        if self.check_if_snake_alive(dir) {
            self.snake.move_forward(dir);
            self.check_eating();
        } else {
            self.game_over = true;
        }

        self.waiting_time = 0.0;
    }

    fn restart(&mut self) {
        self.snake = Snake::new(2, 2);
        self.waiting_time = 0.0;
        self.food_exists = true;
        self.food_x = 6;
        self.food_y = 4;
        self.game_over = false;
        self.won = false;
        self.score = 0;
        self.last_expanded = 0;
        self.total_expanded = 0;
        self.steps = 0;
        self.strategy = None;
        self.planned_path.clear();
    }
}
