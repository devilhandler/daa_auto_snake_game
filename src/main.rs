extern crate piston_window;
extern crate rand;

mod ai;
mod draw;
mod game;
mod pathfinding;
mod snake;

use piston_window::types::Color;
use piston_window::*;

use crate::draw::{to_coord, to_coord_u32};
use crate::game::Game;

const BACK_COLOR: Color = [0.5, 0.5, 0.5, 1.0];

fn main() {
    let (width, height) = (30, 30);

    let mut window: PistonWindow =
        WindowSettings::new("Snake", [to_coord_u32(width), to_coord_u32(height)])
            .exit_on_esc(true)
            .resizable(true)
            .build()
            .unwrap();

    println!("Kontrol: 1/M = Manual, 2/B = BFS, 3/A = A*, F = cepat, P = tampilkan jalur, R = restart, Esc = keluar");

    let mut game = Game::new(width, height);
    let mut last_title = String::new();

    // Ukuran papan dalam koordinat dunia.
    let board_w = to_coord(width);
    let board_h = to_coord(height);

    while let Some(event) = window.next() {
        if let Some(Button::Keyboard(key)) = event.press_args() {
            game.key_pressed(key);
        }

        window.draw_2d(&event, |c, g, _| {
            clear(BACK_COLOR, g);

            // Skalakan & pusatkan papan sesuai ukuran jendela yang SEBENARNYA.
            // Sebelumnya papan digambar dengan ukuran tetap, sehingga pada layar
            // dengan scaling Windows (mis. 125%/150%) atau jendela yang terpotong,
            // bagian bawah/kanan papan (termasuk makanan) berada di luar jendela.
            let [win_w, win_h] = c
                .viewport
                .map(|v| v.window_size)
                .unwrap_or([board_w, board_h]);
            let scale = (win_w / board_w).min(win_h / board_h);
            let off_x = (win_w - board_w * scale) / 2.0;
            let off_y = (win_h - board_h * scale) / 2.0;
            let c = c.trans(off_x, off_y).scale(scale, scale);

            game.draw(&c, g);
        });

        event.update(|arg| {
            game.update(arg.dt);
        });

        let title = game.title();
        if title != last_title {
            window.set_title(title.clone());
            last_title = title;
        }
    }
}
