extern crate pancurses;

mod utils {
    pub struct Rect {
        pub left: i32,
        pub top: i32,
        pub width: i32,
        pub height: i32,
    }

    impl Rect {
        pub const fn _right(&self) -> i32 {
            let right = self.left + self.width - 1;
            right
        }

        pub const fn _bottom(&self) -> i32 {
            let bottom = self.top + self.height - 1;
            bottom
        }

        pub const fn _center_x(&self) -> i32 {
            self.left + self.width / 2
        }

        pub const fn _center_y(&self) -> i32 {
            self.top + self.height / 2
        }
    }
}

use utils::Rect;

const TITLE: &str = "Lost-n-Found";

// Add space between cells in the grid when we render them to the window.
// That'll just make it easier to see each cell
fn get_window_pos_for_cell(x: i32, y: i32, grid_left: i32, grid_top: i32) -> (i32, i32) {
    (x * 2 + grid_left, y * 2 + grid_top)
}

fn main() {
    let window = pancurses::initscr();
    pancurses::noecho(); // prevent key inputs rendering to the screen
    pancurses::cbreak();
    pancurses::curs_set(0);
    pancurses::set_title(TITLE);
    window.nodelay(true); // don't block waiting for key inputs (we'll poll)
    window.keypad(true); // let special keys be captured by the program (i.e. esc/backspace/del/arrow keys)

    const WIN: Rect = Rect {
        left: 0,
        top: 0,
        width: 100,
        height: 60,
    };
    pancurses::resize_term(WIN.height, WIN.width);

    // Not using a Rect because this grid isn't ACTUALLY sized normally like a rect. There are spaces
    const GRID_WIDTH: i32 = 25;
    const GRID_HEIGHT: i32 = 20;
    const GRID_LEFT: i32 = (WIN.width - GRID_WIDTH * 2) / 2;
    const GRID_TOP: i32 = (WIN.height - GRID_HEIGHT * 2) / 2;

    window.clear();
    for row in 0..GRID_HEIGHT {
        for col in 0..GRID_WIDTH {
            let grid_char = if row == 0 {
                std::char::from_u32('0' as u32 + (col % 10) as u32).unwrap()
            } else if col == 0 {
                std::char::from_u32('0' as u32 + (row % 10) as u32).unwrap()
            } else {
                'x'
            };

            let (x, y) = get_window_pos_for_cell(col, row, GRID_LEFT, GRID_TOP);
            window.mvaddch(y, x, grid_char);
        }
    }
    window.refresh();

    std::thread::sleep(std::time::Duration::from_secs(5));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_window_pos_for_cell() {
        let offset_left = 1;
        let offset_top = 5;

        // |-----(1,5)
        // V
        // ....
        // ....
        // ...x
        let input = (3, 2);

        // |-----(1,5)
        // V
        // . . . .
        // . . . .
        // . . . x
        let expected_result = (7, 9);

        assert_eq!(
            expected_result,
            get_window_pos_for_cell(input.0, input.1, offset_left, offset_top)
        )
    }
}
