//! "Otak" ular otomatis: memutuskan langkah berikutnya memakai BFS atau A*.
//!
//! Strategi per langkah:
//! 1. Cari jalur ke makanan. Simulasikan ular mengikuti jalur itu; kalau setelah
//!    makan kepala masih bisa mencapai ekornya, jalur dianggap aman → ikuti.
//! 2. Kalau tidak aman / tidak ada jalur, kejar ekor sendiri (ekor selalu
//!    "membuka" ruang), sambil menunggu makanan bisa dijangkau dengan aman.
//! 3. Kalau ekor juga tidak terjangkau, pilih langkah yang menyisakan ruang
//!    paling luas (flood fill) agar ular bertahan selama mungkin.

use std::collections::{HashSet, VecDeque};

use crate::pathfinding::{Algorithm, Grid, Pos};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Strategy {
    Food,
    ChaseTail,
    Survive,
    Stuck,
}

impl Strategy {
    pub fn name(&self) -> &'static str {
        match self {
            Strategy::Food => "makanan",
            Strategy::ChaseTail => "kejar ekor",
            Strategy::Survive => "bertahan",
            Strategy::Stuck => "terjebak",
        }
    }
}

pub struct Decision {
    pub next: Option<Pos>,
    /// Jalur yang direncanakan (untuk digambar di layar).
    pub path: Vec<Pos>,
    /// Total node yang di-expand oleh algoritma pada langkah ini.
    pub expanded: usize,
    pub strategy: Strategy,
}

/// `body[0]` adalah kepala, `body[len-1]` adalah ekor.
pub fn decide(algo: Algorithm, width: i32, height: i32, body: &[Pos], food: Option<Pos>) -> Decision {
    let head = body[0];
    let tail = *body.last().unwrap();
    let mut expanded = 0;

    // Ekor akan bergeser saat kepala maju, jadi sel ekor boleh dilewati
    // (sama dengan aturan `overlap_tail` di snake.rs).
    let blocked: HashSet<Pos> = body[..body.len() - 1].iter().copied().collect();
    let grid = Grid { width, height, blocked: &blocked };

    // 1. Menuju makanan, jika aman.
    if let Some(food) = food {
        let res = grid.search(algo, head, food);
        expanded += res.expanded;
        if let Some(path) = res.path {
            let (safe, cost) = tail_reachable_after(algo, width, height, body, &path, food);
            expanded += cost;
            if safe {
                return Decision { next: Some(path[0]), path, expanded, strategy: Strategy::Food };
            }
        }
    }

    // 2. Kejar ekor. Hanya masuk akal jika ular cukup panjang (> 3); ular pendek
    //    yang berputar di tempat tidak pernah terjebak.
    if body.len() > 3 {
        let res = grid.search(algo, head, tail);
        expanded += res.expanded;
        if let Some(path) = res.path {
            return Decision { next: Some(path[0]), path, expanded, strategy: Strategy::ChaseTail };
        }
    }

    // 3. Bertahan: langkah dengan ruang tersisa terbesar.
    let best = grid
        .neighbors(head)
        .map(|n| {
            let mut b = blocked.clone();
            b.insert(head);
            b.remove(&tail);
            let area = Grid { width, height, blocked: &b }.reachable_area(n);
            (area, n)
        })
        .max_by_key(|&(area, _)| area);

    match best {
        Some((_, n)) => Decision { next: Some(n), path: vec![n], expanded, strategy: Strategy::Survive },
        None => Decision { next: None, path: vec![], expanded, strategy: Strategy::Stuck },
    }
}

/// Simulasikan ular mengikuti `path`, lalu cek apakah kepala barunya masih
/// bisa mencapai ekornya. Mengembalikan (aman?, node yang di-expand).
fn tail_reachable_after(
    algo: Algorithm,
    width: i32,
    height: i32,
    body: &[Pos],
    path: &[Pos],
    food: Pos,
) -> (bool, usize) {
    let mut sim: VecDeque<Pos> = body.iter().copied().collect();
    for &step in path {
        sim.push_front(step);
        if step != food {
            sim.pop_back();
        }
    }

    let head = sim[0];
    let tail = *sim.back().unwrap();
    // Kepala dan ekor bukan rintangan; sisanya rintangan.
    let blocked: HashSet<Pos> = sim.iter().skip(1).take(sim.len() - 2).copied().collect();
    let grid = Grid { width, height, blocked: &blocked };

    let res = grid.search(algo, head, tail);
    (res.path.is_some(), res.expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heads_straight_for_food() {
        let body = vec![(4, 2), (3, 2), (2, 2)];
        for algo in [Algorithm::Bfs, Algorithm::AStar] {
            let d = decide(algo, 30, 30, &body, Some((6, 4)));
            assert_eq!(d.strategy, Strategy::Food);
            assert_eq!(d.path.len(), 4);
            assert_eq!(*d.path.last().unwrap(), (6, 4));
            assert!(d.next == Some((5, 2)) || d.next == Some((4, 3)));
        }
    }

    #[test]
    fn never_steps_into_wall_or_body() {
        // ular di pojok kanan atas menghadap kanan; makanan di kiri bawah
        let body = vec![(28, 1), (27, 1), (26, 1), (25, 1)];
        let d = decide(Algorithm::AStar, 30, 30, &body, Some((1, 28)));
        assert_eq!(d.next, Some((28, 2)));
    }

    /// Jalankan game secara headless sampai ular cukup panjang; ular tidak boleh mati.
    #[test]
    fn plays_without_dying() {
        for algo in [Algorithm::Bfs, Algorithm::AStar] {
            let (w, h) = (12, 12);
            let mut body: VecDeque<Pos> = VecDeque::from([(4, 2), (3, 2), (2, 2)]);
            let mut seed: u64 = 42;
            let mut rand = |n: i32| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ((seed >> 33) % n as u64) as i32
            };
            let mut food = (6, 4);
            let mut eaten = 0;
            for _ in 0..20_000 {
                let v: Vec<Pos> = body.iter().copied().collect();
                let d = decide(algo, w, h, &v, Some(food));
                let next = d.next.expect("ular terjebak");
                // validasi langkah
                assert!(next.0 > 0 && next.1 > 0 && next.0 < w - 1 && next.1 < h - 1, "nabrak dinding");
                assert!(!v[..v.len() - 1].contains(&next), "nabrak badan");
                body.push_front(next);
                if next == food {
                    eaten += 1;
                    if eaten >= 40 {
                        break;
                    }
                    loop {
                        let f = (1 + rand(w - 2), 1 + rand(h - 2));
                        if !body.contains(&f) {
                            food = f;
                            break;
                        }
                    }
                } else {
                    body.pop_back();
                }
            }
            assert!(eaten >= 40, "{:?} hanya makan {}", algo, eaten);
        }
    }
}
