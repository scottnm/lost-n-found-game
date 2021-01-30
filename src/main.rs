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

#[derive(Debug, Clone, Copy)]
enum GridItem {
    X,
}

struct GameGrid {
    _data: Box<[GridItem]>,
    width: i32,
    height: i32,
}

impl GameGrid {
    fn new(width: i32, height: i32) -> Self {
        GameGrid {
            _data: vec![GridItem::X; (width * height) as usize].into_boxed_slice(),
            width,
            height,
        }
    }

    fn item(&self, x: i32, y: i32) -> Option<GridItem> {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return None;
        }

        let index = (self.width * y + x) as usize;
        Some(self._data[index])
    }
}

// Add space between cells in the grid when we render them to the window.
// That'll just make it easier to see each cell
fn game_grid_to_window(x: i32, y: i32, grid_left: i32, grid_top: i32) -> (i32, i32) {
    (x * 2 + grid_left, y * 2 + grid_top)
}

fn window_to_game_grid(x: i32, y: i32, grid_left: i32, grid_top: i32) -> (i32, i32) {
    ((x - grid_left) / 2, (y - grid_top) / 2)
}

fn setup_pancurses_mouse() {
    let mut oldmask: pancurses::mmask_t = 0;
    let mousemask = pancurses::BUTTON1_CLICKED | pancurses::REPORT_MOUSE_POSITION;
    pancurses::mousemask(mousemask, &mut oldmask);
}

fn main() {
    let window = pancurses::initscr();
    pancurses::noecho(); // prevent key inputs rendering to the screen
    pancurses::cbreak();
    pancurses::curs_set(0);
    pancurses::set_title(TITLE);
    setup_pancurses_mouse();
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
    let game_grid = GameGrid::new(25, 20);
    let grid_left = (WIN.width - game_grid.width * 2) / 2;
    let grid_top = (WIN.height - game_grid.height * 2) / 2;

    #[derive(Debug)]
    struct MouseState {
        click: bool,
        x: i32,
        y: i32,
    };

    let mut mouse_state = MouseState {
        click: false,
        x: 0,
        y: 0,
    };

    let mut last_clicked_cell = None;

    let bkgd_char = window.getbkgd();

    loop {
        // If we get a mouse event, update our mouse state
        if let Some(pancurses::Input::KeyMouse) = window.getch() {
            if let Ok(mouse_event) = pancurses::getmouse() {
                mouse_state = MouseState {
                    click: (mouse_event.bstate & pancurses::BUTTON1_CLICKED) != 0,
                    x: mouse_event.x,
                    y: mouse_event.y,
                };
            }
        }

        // snap the mouse_state_str before I potentially consume the click
        let mouse_state_str = format!("{:?}", mouse_state);

        // convert the mouse position to an item in a grid cell
        let grid_pos = window_to_game_grid(mouse_state.x, mouse_state.y, grid_left, grid_top);
        if mouse_state.click {
            mouse_state.click = false;
            last_clicked_cell = game_grid.item(grid_pos.0, grid_pos.1);
        }

        // use erase instead of clear
        window.erase();

        // render the debug mouse info
        window.mvaddstr(0, 0, mouse_state_str);
        window.mvaddstr(1, 0, format!("{:?}", last_clicked_cell));

        // render the grid
        for row in 0..game_grid.height {
            for col in 0..game_grid.width {
                let grid_char = if row == 0 {
                    std::char::from_u32('0' as u32 + (col % 10) as u32).unwrap()
                } else if col == 0 {
                    std::char::from_u32('0' as u32 + (row % 10) as u32).unwrap()
                } else {
                    'x'
                };

                let (x, y) = game_grid_to_window(col, row, grid_left, grid_top);
                window.mvaddch(y, x, grid_char);
            }
        }

        // if our mouse is over a grid character, highlight it!
        let highlight_pos = game_grid_to_window(grid_pos.0, grid_pos.1, grid_left, grid_top);
        if window.mvinch(highlight_pos.1, highlight_pos.0) != bkgd_char {
            window.mvchgat(highlight_pos.1, highlight_pos.0, 1, pancurses::A_BLINK, 0);
        }

        window.refresh();

        // Yield for 1/30th of a second. Don't hog that CPU.
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_grid_to_window() {
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
            game_grid_to_window(input.0, input.1, offset_left, offset_top)
        )
    }
}
