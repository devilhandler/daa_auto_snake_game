//! Benchmark BFS vs A* untuk laporan (Bab 4.3 dan 4.4).
//!
//! Jalankan dengan mode release agar waktu yang terukur realistis:
//!     cargo run --release --example benchmark
//!
//! Bagian 1: satu kali pencarian dari pojok ke pojok pada grid n x n
//!           (kosong dan dengan 20% rintangan acak).
//! Bagian 2: simulasi game penuh tanpa grafis di papan 30 x 30.

#![allow(dead_code)]

#[path = "../src/pathfinding.rs"]
mod pathfinding;
#[path = "../src/ai.rs"]
mod ai;

use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use ai::Strategy;
use pathfinding::{Algorithm, Grid, Pos};

const SIZES: [i32; 6] = [10, 20, 50, 100, 200, 300];
const ALGOS: [Algorithm; 2] = [Algorithm::Bfs, Algorithm::AStar];

fn main() {
    println!("# Benchmark BFS vs A* - Automatic Snake Game");
    println!("(semua waktu dalam milidetik; build release)\n");

    single_search("Grid kosong", 0.0);
    single_search("Grid dengan 20% rintangan acak", 0.20);
    game_simulation(30, 10);
}

/// Waktu rata-rata satu pencarian (ms), diulang `reps` kali.
fn time_search(grid: &Grid, algo: Algorithm, s: Pos, t: Pos, reps: u32) -> f64 {
    let start = Instant::now();
    for _ in 0..reps {
        std::hint::black_box(grid.search(algo, s, t));
    }
    start.elapsed().as_secs_f64() * 1000.0 / reps as f64
}

fn single_search(title: &str, obstacle_ratio: f64) {
    println!("## {} (start = (1,1), goal = pojok berlawanan)\n", title);
    println!("| Grid | V | E | Algoritma | Run 1 (ms) | Run 2 (ms) | Rata-rata (ms) | Node di-expand | Panjang jalur |");
    println!("|---|---|---|---|---|---|---|---|---|");

    for &n in &SIZES {
        let mut rng = StdRng::seed_from_u64(2026 + n as u64);
        let (s, t) = ((1, 1), (n - 2, n - 2));

        // Buat rintangan acak; ulangi sampai goal bisa dicapai.
        let blocked: HashSet<Pos> = loop {
            let b: HashSet<Pos> = (1..n - 1)
                .flat_map(|y| (1..n - 1).map(move |x| (x, y)))
                .filter(|&p| p != s && p != t)
                .filter(|_| rng.gen_bool(obstacle_ratio))
                .collect();
            let g = Grid { width: n, height: n, blocked: &b };
            if g.bfs(s, t).path.is_some() {
                break b;
            }
        };
        let grid = Grid { width: n, height: n, blocked: &blocked };

        let (v, e) = count_graph(&grid);
        // Jumlah ulangan disesuaikan ukuran agar tiap run memakan waktu yang wajar.
        let reps = (2_000_000 / v.max(1)).clamp(3, 20_000) as u32;

        for algo in ALGOS {
            time_search(&grid, algo, s, t, 1); // pemanasan
            let r1 = time_search(&grid, algo, s, t, reps);
            let r2 = time_search(&grid, algo, s, t, reps);
            let res = grid.search(algo, s, t);
            println!(
                "| {n}x{n} | {v} | {e} | {} | {:.4} | {:.4} | {:.4} | {} | {} |",
                algo.name(),
                r1,
                r2,
                (r1 + r2) / 2.0,
                res.expanded,
                res.path.map(|p| p.len()).unwrap_or(0)
            );
        }
    }
    println!();
}

/// V = sel yang bisa dilewati, E = pasangan sel bertetangga yang sama-sama bisa dilewati.
fn count_graph(grid: &Grid) -> (usize, usize) {
    let mut v = 0;
    let mut e = 0;
    for y in 1..grid.height - 1 {
        for x in 1..grid.width - 1 {
            if grid.passable((x, y)) {
                v += 1;
                e += grid.neighbors((x, y)).count();
            }
        }
    }
    (v, e / 2)
}

struct GameStats {
    score: usize,
    steps: usize,
    expanded: usize,
    decide_ms: f64,
    outcome: &'static str,
}

/// Mainkan satu game tanpa grafis dengan aturan yang sama seperti game.rs.
fn play(algo: Algorithm, size: i32, seed: u64) -> GameStats {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut body: VecDeque<Pos> = VecDeque::from([(4, 2), (3, 2), (2, 2)]);
    let mut food = (6, 4);
    let mut st = GameStats { score: 0, steps: 0, expanded: 0, decide_ms: 0.0, outcome: "" };
    // Kalau ular tidak makan selama 4x jumlah sel, ia dianggap berputar-putar
    // tanpa akhir (mengejar ekor terus karena makanan tidak pernah "aman").
    let stall_limit = 4 * ((size - 2) * (size - 2)) as usize;
    let mut since_food = 0;

    loop {
        let v: Vec<Pos> = body.iter().copied().collect();
        let t0 = Instant::now();
        let d = ai::decide(algo, size, size, &v, Some(food));
        st.decide_ms += t0.elapsed().as_secs_f64() * 1000.0;
        st.expanded += d.expanded;
        st.steps += 1;

        let next = match d.next {
            Some(p) if d.strategy != Strategy::Stuck => p,
            _ => {
                st.outcome = "mati (terjebak)";
                return st;
            }
        };
        body.push_front(next);
        if next == food {
            st.score += 1;
            since_food = 0;
            let free: Vec<Pos> = (1..size - 1)
                .flat_map(|y| (1..size - 1).map(move |x| (x, y)))
                .filter(|p| !body.contains(p))
                .collect();
            if free.is_empty() {
                st.outcome = "menang (papan penuh)";
                return st;
            }
            food = free[rng.gen_range(0..free.len())];
        } else {
            body.pop_back();
            since_food += 1;
        }

        if since_food >= stall_limit {
            st.outcome = "berputar (tak bisa makan lagi)";
            return st;
        }
    }
}

fn game_simulation(size: i32, games: u64) {
    println!("## Simulasi game penuh ({size}x{size}, {games} game per algoritma, seed sama)\n");
    println!("| Algoritma | Game | Skor | Langkah | Rata-rata node/langkah | Rata-rata waktu/keputusan (ms) | Hasil |");
    println!("|---|---|---|---|---|---|---|");

    for algo in ALGOS {
        let (mut sum_score, mut sum_steps, mut sum_exp, mut sum_ms) = (0, 0, 0, 0.0);
        for g in 0..games {
            let st = play(algo, size, 1000 + g);
            println!(
                "| {} | {} | {} | {} | {:.1} | {:.4} | {} |",
                algo.name(),
                g + 1,
                st.score,
                st.steps,
                st.expanded as f64 / st.steps as f64,
                st.decide_ms / st.steps as f64,
                st.outcome
            );
            sum_score += st.score;
            sum_steps += st.steps;
            sum_exp += st.expanded;
            sum_ms += st.decide_ms;
        }
        println!(
            "| **{} (rata-rata)** | - | {:.1} | {:.0} | {:.1} | {:.4} | - |",
            algo.name(),
            sum_score as f64 / games as f64,
            sum_steps as f64 / games as f64,
            sum_exp as f64 / sum_steps as f64,
            sum_ms / sum_steps as f64
        );
    }
    println!();
}
