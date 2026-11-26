//! Algoritma pencarian jalur untuk ular otomatis: BFS dan A*.
//!
//! Papan dimodelkan sebagai grid 4-arah (atas, bawah, kiri, kanan).
//! Sel yang bisa dilewati adalah sel di dalam dinding (1..width-1, 1..height-1)
//! yang tidak termasuk di `blocked` (tubuh ular).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

pub type Pos = (i32, i32);

/// Algoritma yang sedang dipakai.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Algorithm {
    Bfs,
    AStar,
}

impl Algorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Algorithm::Bfs => "BFS",
            Algorithm::AStar => "A*",
        }
    }
}

/// Hasil pencarian: jalur (tanpa titik awal, diakhiri `goal`) dan jumlah node
/// yang di-expand. Jumlah node berguna untuk membandingkan efisiensi BFS vs A*.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: Option<Vec<Pos>>,
    pub expanded: usize,
}

pub struct Grid<'a> {
    pub width: i32,
    pub height: i32,
    pub blocked: &'a HashSet<Pos>,
}

impl<'a> Grid<'a> {
    pub fn in_bounds(&self, (x, y): Pos) -> bool {
        x > 0 && y > 0 && x < self.width - 1 && y < self.height - 1
    }

    pub fn passable(&self, p: Pos) -> bool {
        self.in_bounds(p) && !self.blocked.contains(&p)
    }

    pub fn neighbors(&self, (x, y): Pos) -> impl Iterator<Item = Pos> + '_ {
        [(x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y)]
            .into_iter()
            .filter(move |&p| self.passable(p))
    }

    pub fn search(&self, algo: Algorithm, start: Pos, goal: Pos) -> SearchResult {
        match algo {
            Algorithm::Bfs => self.bfs(start, goal),
            Algorithm::AStar => self.astar(start, goal),
        }
    }

    /// Breadth-First Search: menjelajah lapis demi lapis memakai antrian (FIFO).
    /// Pada grid tanpa bobot, BFS selalu menemukan jalur terpendek.
    /// Kompleksitas: O(V + E) = O(W * H).
    pub fn bfs(&self, start: Pos, goal: Pos) -> SearchResult {
        let mut queue = VecDeque::new();
        let mut came_from: HashMap<Pos, Pos> = HashMap::new();
        let mut visited: HashSet<Pos> = HashSet::new();
        let mut expanded = 0;

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            expanded += 1;
            if current == goal {
                return SearchResult { path: Some(reconstruct(&came_from, start, goal)), expanded };
            }
            for next in self.neighbors(current) {
                if visited.insert(next) {
                    came_from.insert(next, current);
                    queue.push_back(next);
                }
            }
        }

        SearchResult { path: None, expanded }
    }

    /// A*: seperti Dijkstra tetapi memprioritaskan node dengan f = g + h,
    /// di mana g = jarak dari start dan h = heuristik Manhattan ke goal.
    /// Manhattan admissible pada grid 4-arah, jadi jalur tetap optimal,
    /// namun biasanya node yang di-expand jauh lebih sedikit daripada BFS.
    pub fn astar(&self, start: Pos, goal: Pos) -> SearchResult {
        let mut open = BinaryHeap::new();
        let mut came_from: HashMap<Pos, Pos> = HashMap::new();
        let mut g_score: HashMap<Pos, i32> = HashMap::new();
        let mut closed: HashSet<Pos> = HashSet::new();
        let mut expanded = 0;

        g_score.insert(start, 0);
        open.push(Node { pos: start, g: 0, f: manhattan(start, goal) });

        while let Some(Node { pos: current, g, .. }) = open.pop() {
            if !closed.insert(current) {
                continue; // entri usang di heap
            }
            expanded += 1;
            if current == goal {
                return SearchResult { path: Some(reconstruct(&came_from, start, goal)), expanded };
            }
            for next in self.neighbors(current) {
                let tentative = g + 1;
                if tentative < *g_score.get(&next).unwrap_or(&i32::MAX) {
                    g_score.insert(next, tentative);
                    came_from.insert(next, current);
                    open.push(Node { pos: next, g: tentative, f: tentative + manhattan(next, goal) });
                }
            }
        }

        SearchResult { path: None, expanded }
    }

    /// Jumlah sel yang bisa dicapai dari `start` (flood fill). Dipakai sebagai
    /// langkah darurat: pilih arah yang menyisakan ruang paling luas.
    pub fn reachable_area(&self, start: Pos) -> usize {
        let mut queue = VecDeque::from([start]);
        let mut visited = HashSet::from([start]);
        while let Some(cur) = queue.pop_front() {
            for next in self.neighbors(cur) {
                if visited.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        visited.len()
    }
}

pub fn manhattan(a: Pos, b: Pos) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn reconstruct(came_from: &HashMap<Pos, Pos>, start: Pos, goal: Pos) -> Vec<Pos> {
    let mut path = vec![goal];
    let mut cur = goal;
    while let Some(&prev) = came_from.get(&cur) {
        if prev == start {
            break;
        }
        path.push(prev);
        cur = prev;
    }
    path.reverse();
    path
}

#[derive(Eq, PartialEq)]
struct Node {
    pos: Pos,
    g: i32,
    f: i32,
}

// BinaryHeap di Rust adalah max-heap, jadi urutan dibalik agar f terkecil keluar dulu.
// Jika f sama, utamakan g yang lebih besar (lebih dekat ke goal).
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f).then_with(|| self.g.cmp(&other.g))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(blocked: &HashSet<Pos>) -> Grid<'_> {
        Grid { width: 10, height: 10, blocked }
    }

    #[test]
    fn bfs_and_astar_find_same_length() {
        let blocked = HashSet::new();
        let g = grid(&blocked);
        let b = g.bfs((1, 1), (8, 8));
        let a = g.astar((1, 1), (8, 8));
        assert_eq!(b.path.as_ref().unwrap().len(), 14);
        assert_eq!(a.path.as_ref().unwrap().len(), 14);
        assert!(a.expanded < b.expanded);
        assert_eq!(*a.path.unwrap().last().unwrap(), (8, 8));
    }

    #[test]
    fn path_goes_around_wall() {
        // dinding vertikal di x = 4 kecuali y = 8
        let blocked: HashSet<Pos> = (1..8).map(|y| (4, y)).collect();
        let g = grid(&blocked);
        for algo in [Algorithm::Bfs, Algorithm::AStar] {
            let p = g.search(algo, (2, 2), (6, 2)).path.unwrap();
            assert!(p.contains(&(4, 8)));
            assert_eq!(p.len(), 16);
            // setiap langkah bertetangga
            let mut prev = (2, 2);
            for &s in &p {
                assert_eq!(manhattan(prev, s), 1);
                prev = s;
            }
        }
    }

    #[test]
    fn no_path_when_enclosed() {
        let blocked: HashSet<Pos> = [(2, 1), (1, 2)].into_iter().collect();
        let g = grid(&blocked);
        assert!(g.bfs((1, 1), (5, 5)).path.is_none());
        assert!(g.astar((1, 1), (5, 5)).path.is_none());
        assert_eq!(g.reachable_area((1, 1)), 1);
    }
}
