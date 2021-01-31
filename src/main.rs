extern crate pancurses;

mod utils {
    #[derive(Debug, PartialEq, Eq)]
    pub struct Rect {
        pub left: i32,
        pub top: i32,
        pub width: i32,
        pub height: i32,
    }

    impl Rect {
        #[cfg(test)]
        pub const fn right(&self) -> i32 {
            let right = self.left + self.width - 1;
            right
        }

        pub const fn bottom(&self) -> i32 {
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

// helpers for transforming from one coordinate space to another
mod xform {
    use super::*;

    // the game-grid-coord-space is just a contiguous 2D array where it's origin always lies at zero
    // the window-coord-space accounts for where those grid cells are actually rendered on screen.
    // The on-screen grid resembles this pattern:
    //  ___ ___ ___
    // |   |   |   |
    // |___|___|___|
    // |   |   |   |
    // |___|___|___|
    // |   |   |   |
    // |___|___|___|
    //
    //
    // where a single game-grid cell actually comprises of a 3x2 block of window cells/chars.
    // Addiitionally it's important to remember that the grid may (will) be offset within the window.

    // Add space between cells in the grid when we render them to the window.
    // That'll just make it easier to see each cell
    pub fn game_grid_to_window(x: i32, y: i32, grid_left: i32, grid_top: i32) -> Rect {
        // To calculate the horizontal range of cells in the window...
        // 1. account for the grids left offset (where is the grid rendered in the window)
        // 2. account for the leading vertical border cell
        // 3. skip 3 cell spaces and the next vertical bar for every grid cell you move right
        let window_left = grid_left + 1 + (3 + 1) * x;

        // To calculate the vertical range of cells in the window...
        // 1. account for the grids top offset (where is the grid rendered in the window)
        // 2. account for the leading horizontal border cell
        // 3. skip 2 cell spaces (the second cell space also includes the next horizontal border)
        let window_top = grid_top + 1 + 2 * y;

        // every grid cell in the window is a 2x2 cell of characters
        let width = 3;
        let height = 2;

        Rect {
            left: window_left,
            top: window_top,
            width,
            height,
        }
    }

    pub fn window_to_game_grid(x: i32, y: i32, grid_left: i32, grid_top: i32) -> (i32, i32) {
        // first shift our window position so that our grid is aligned at the origin
        // additionally subtract an additional 1 to account for the grid border
        let window_at_origin = (x - grid_left - 1, y - grid_top - 1);

        // finally divide the x portion by 4 (3 cells + a border)
        // and divide the y portion by 2 (2 cells one of which includes the next border)
        (window_at_origin.0 / 4, window_at_origin.1 / 2)
    }
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
        let grid_pos =
            xform::window_to_game_grid(mouse_state.x, mouse_state.y, grid_left, grid_top);
        let hovered_over_grid_item = game_grid.item(grid_pos.0, grid_pos.1);
        if mouse_state.click {
            mouse_state.click = false;
            last_clicked_cell = hovered_over_grid_item.clone();
        }

        // use erase instead of clear
        window.erase();

        // render the debug mouse info
        window.mvaddstr(0, 0, mouse_state_str);
        window.mvaddstr(1, 0, format!("{:?}", last_clicked_cell));

        // add the leading border cells on top of the grid
        for col in 0..game_grid.width {
            window.mvaddstr(grid_top, grid_left + 1 + 4 * col, "___");
        }

        // render the grid
        for row in 0..game_grid.height {
            // add the leading border cells for each row
            window.mvaddch((row * 2) + grid_top + 1, grid_left, '|');
            window.mvaddch((row * 2) + grid_top + 2, grid_left, '|');

            // render each cell
            for col in 0..game_grid.width {
                window.mvaddstr((row * 2) + grid_top + 1, grid_left + 1 + 4 * col, "   |");
                window.mvaddstr((row * 2) + grid_top + 2, grid_left + 1 + 4 * col, "___|");
            }
        }

        // if we are hovering over a grid cell, highlight the selected cell
        if hovered_over_grid_item.is_some() {
            let highlighted_rect =
                xform::game_grid_to_window(grid_pos.0, grid_pos.1, grid_left, grid_top);
            for row in highlighted_rect.top..=highlighted_rect.bottom() {
                window.mvchgat(
                    row,
                    highlighted_rect.left,
                    highlighted_rect.width,
                    pancurses::A_BLINK,
                    0,
                );
            }
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
        // ....
        // ...x
        // ....
        let input = (3, 1);

        // |-----(1,5)
        // V___ ___ ___ ___
        // |   |   |   |   |
        // |___|___|___|___|
        // |   |   |   |xxx|
        // |___|___|___|___|
        // |   |   |   |   |
        // |___|___|___|___|
        let offset_left = 1;
        let offset_top = 5;
        let expected_result = Rect {
            left: 14,
            top: 8,
            width: 3,
            height: 2,
        };

        assert_eq!(
            expected_result,
            xform::game_grid_to_window(input.0, input.1, offset_left, offset_top)
        );
    }

    #[test]
    fn test_window_to_game_grid() {
        // |-----(4,2)
        // V___ ___ ___ ___
        // |   |   |   |   |
        // |___|___|___|___|
        // |   |   |   |   |
        // |___|___|__x|___|
        // |   |   |   |   |
        // |___|___|___|___|
        let offset_left = 4;
        let offset_top = 2;
        let input = (15, 6);

        // ....
        // ..x.
        // ....
        let expected_result = (2, 1);

        assert_eq!(
            expected_result,
            xform::window_to_game_grid(input.0, input.1, offset_left, offset_top)
        );
    }

    #[test]
    fn test_game_grid_to_window_to_game_grid() {
        // ....
        // ....
        // .x..
        let input = (1, 2);

        // |-----(6,7)
        // V___ ___ ___ ___
        // |   |   |   |   |
        // |___|___|___|___|
        // |   |   |   |   |
        // |___|___|___|___|
        // |   |xxx|   |   |
        // |___|___|___|___|
        let offset_left = 6;
        let offset_top = 7;
        let expected_result = Rect {
            left: 11,
            top: 12,
            width: 3,
            height: 2,
        };

        // First verify that we calculated the correct range of cells
        assert_eq!(
            expected_result,
            xform::game_grid_to_window(input.0, input.1, offset_left, offset_top)
        );

        // Next verify that each cell in that range maps back to our input
        for row in expected_result.top..=expected_result.bottom() {
            for col in expected_result.left..=expected_result.right() {
                assert_eq!(
                    input,
                    xform::window_to_game_grid(col, row, offset_left, offset_top)
                );
            }
        }
    }
}
