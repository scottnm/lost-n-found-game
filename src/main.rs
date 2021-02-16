extern crate pancurses;
extern crate snm_rand_utils;

use snm_rand_utils::range_rng::*;

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

        pub const fn center_x(&self) -> i32 {
            self.left + self.width / 2
        }

        pub const fn center_y(&self) -> i32 {
            self.top + self.height / 2
        }
    }
}

use utils::Rect;

const TITLE: &str = "Lost-n-Found";

mod game {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub enum HintDir {
        Left,
        Up,
        Right,
        Down,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum GridItem {
        Solution,
        Hint(HintDir),
    }

    #[derive(Debug, Clone, Copy)]
    pub struct GridCell {
        pub item: GridItem,
        pub revealed: bool,
    }

    pub struct GameGrid {
        data: Box<[GridCell]>,
        width: i32,
        height: i32,
    }

    impl GameGrid {
        pub fn new(width: i32, height: i32, rng: &mut dyn RangeRng<i32>) -> Self {
            let solution_cell = (rng.gen_range(0, width), rng.gen_range(0, height));

            let num_cells = (width * height) as usize;
            let mut data = Vec::with_capacity(num_cells);
            for row in 0..height {
                for col in 0..width {
                    fn displacement_to_hint_direction(
                        x_displacement: i32,
                        y_displacement: i32,
                    ) -> HintDir {
                        assert!(x_displacement != 0 || y_displacement != 0);
                        if x_displacement.abs() > y_displacement.abs() {
                            if x_displacement > 0 {
                                HintDir::Left
                            } else {
                                HintDir::Right
                            }
                        } else {
                            if y_displacement > 0 {
                                HintDir::Up
                            } else {
                                HintDir::Down
                            }
                        }
                    }

                    let x_displacement = col - solution_cell.0;
                    let y_displacement = row - solution_cell.1;
                    let item = {
                        if x_displacement == 0 && y_displacement == 0 {
                            GridItem::Solution
                        } else {
                            let hint =
                                displacement_to_hint_direction(x_displacement, y_displacement);
                            GridItem::Hint(hint)
                        }
                    };

                    data.push(GridCell {
                        item,
                        revealed: false,
                    });
                }
            }

            GameGrid {
                data: data.into_boxed_slice(),
                width,
                height,
            }
        }

        pub fn width(&self) -> i32 {
            self.width
        }

        pub fn height(&self) -> i32 {
            self.height
        }

        pub fn item(&self, x: i32, y: i32) -> Option<GridCell> {
            if x < 0 || x >= self.width || y < 0 || y >= self.height {
                return None;
            }

            let index = (self.width * y + x) as usize;
            Some(self.data[index])
        }

        pub fn mut_item(&mut self, x: i32, y: i32) -> Option<&mut GridCell> {
            if x < 0 || x >= self.width || y < 0 || y >= self.height {
                return None;
            }

            let index = (self.width * y + x) as usize;
            Some(&mut self.data[index])
        }
    }
}

use game::*;

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

enum Color {
    Green,
    Yellow,
    Magenta,
    Cyan,
    BlackOnWhite,
}

impl Color {
    fn to_num(&self) -> u8 {
        match self {
            Color::Green => 1,
            Color::Yellow => 2,
            Color::Magenta => 3,
            Color::Cyan => 4,
            Color::BlackOnWhite => 5,
        }
    }

    fn setup() {
        pancurses::start_color();
        pancurses::init_pair(
            Color::Green.to_num() as i16,
            pancurses::COLOR_GREEN,
            pancurses::COLOR_BLACK,
        );
        pancurses::init_pair(
            Color::Yellow.to_num() as i16,
            pancurses::COLOR_YELLOW,
            pancurses::COLOR_BLACK,
        );
        pancurses::init_pair(
            Color::Cyan.to_num() as i16,
            pancurses::COLOR_CYAN,
            pancurses::COLOR_BLACK,
        );
        pancurses::init_pair(
            Color::Magenta.to_num() as i16,
            pancurses::COLOR_MAGENTA,
            pancurses::COLOR_BLACK,
        );
        pancurses::init_pair(
            Color::BlackOnWhite.to_num() as i16,
            pancurses::COLOR_BLACK,
            pancurses::COLOR_WHITE,
        );
    }

    pub fn to_color_pair(&self) -> pancurses::chtype {
        pancurses::COLOR_PAIR(self.to_num() as pancurses::chtype)
    }
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

    Color::setup();
    loop {
        run_game(&window);
    }
}

fn run_game(window: &pancurses::Window) {
    // Not using a Rect because this grid isn't ACTUALLY sized normally like a rect. There are spaces
    let mut rng = ThreadRangeRng::new();
    let mut game_grid = GameGrid::new(25, 20, &mut rng);

    let center_cell_range =
        xform::game_grid_to_window(game_grid.width() / 2, game_grid.height() / 2, 0, 0);
    let grid_left = (window.get_max_x() / 2) - center_cell_range.center_x();
    let grid_top = (window.get_max_y() / 2) - center_cell_range.center_y();
    let grid_center = (
        grid_left + center_cell_range.center_x(),
        grid_top + center_cell_range.center_y(),
    );

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

    enum GameResult {
        Win,
        Lose,
    }

    struct GameOverState {
        result: GameResult,
        msg_timer: std::time::Instant,
    }

    const BOARD_FINISH_MSG_TIME: std::time::Duration = std::time::Duration::from_secs(5);
    let mut game_over_state: Option<GameOverState> = None;

    let game_time_limit = std::time::Duration::from_secs(10);
    let game_start_time = std::time::Instant::now();

    let mut last_clicked_cell: Option<GridCell> = None;

    while game_over_state.is_none()
        || game_over_state.as_ref().unwrap().msg_timer.elapsed() < BOARD_FINISH_MSG_TIME
    {
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
        let hovered_over_grid_cell: Option<GridCell> = {
            let hovered_over_grid_cell = game_grid.mut_item(grid_pos.0, grid_pos.1);
            if mouse_state.click {
                mouse_state.click = false;
                // FIXME: ugh why is this necessary to use an optional mutable ref???
                if let Some(&mut ref mut item) = hovered_over_grid_cell {
                    item.revealed = true;
                }

                if let Some(&mut ref mut cell) = hovered_over_grid_cell {
                    if game_over_state.is_none() {
                        if let GridItem::Solution = cell.item {
                            game_over_state = Some(GameOverState {
                                result: GameResult::Win,
                                msg_timer: std::time::Instant::now(),
                            });
                        }
                    }

                    last_clicked_cell = Some(cell.clone())
                } else {
                    last_clicked_cell = None
                }
            }
            hovered_over_grid_cell.copied()
        };

        // check for the lose state
        if game_over_state.is_none() && game_start_time.elapsed() > game_time_limit {
            game_over_state = Some(GameOverState {
                result: GameResult::Lose,
                msg_timer: std::time::Instant::now(),
            });
        }

        // use erase instead of clear
        window.erase();

        // render the debug mouse info
        window.mvaddstr(0, 0, mouse_state_str);
        window.mvaddstr(1, 0, format!("{:?}", last_clicked_cell));

        // add the leading border cells on top of the grid
        for col in 0..game_grid.width() {
            window.mvaddstr(grid_top, grid_left + 1 + 4 * col, "___");
        }

        // render the grid
        for row in 0..game_grid.height() {
            let row_offset = (row * 2) + grid_top + 1;
            // add the leading border cells for each row
            window.mvaddch(row_offset, grid_left, '|');
            window.mvaddch(row_offset + 1, grid_left, '|');

            // render each cell
            for col in 0..game_grid.width() {
                let col_offset = grid_left + 1 + 4 * col;
                // safe to unwrap since we are iterating over the grid by its own bounds
                let grid_cell = game_grid.item(col, row).unwrap();
                let (grid_item_lines, grid_item_attributes) = if grid_cell.revealed {
                    match grid_cell.item {
                        GridItem::Solution => {
                            (["***|", "***|"], Color::BlackOnWhite.to_color_pair())
                        }
                        GridItem::Hint(hint_dir) => match hint_dir {
                            HintDir::Left => (["<--|", "___|"], Color::Cyan.to_color_pair()),
                            HintDir::Right => (["-->|", "___|"], Color::Yellow.to_color_pair()),
                            HintDir::Up => ([" ^ |", "_|_|"], Color::Magenta.to_color_pair()),
                            HintDir::Down => ([" | |", "_V_|"], Color::Green.to_color_pair()),
                        },
                    }
                } else {
                    (["   |", "___|"], pancurses::A_NORMAL)
                };

                window.attron(grid_item_attributes);
                window.mvaddstr(row_offset, col_offset, grid_item_lines[0]);
                window.mvaddstr(row_offset + 1, col_offset, grid_item_lines[1]);
                window.attroff(grid_item_attributes);
                window.attroff(pancurses::A_BLINK);
            }
        }

        // if we are hovering over a grid cell, highlight the selected cell
        if hovered_over_grid_cell.is_some() {
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

        if let Some(game_over) = game_over_state.as_ref() {
            let game_over_text = match game_over.result {
                GameResult::Lose => "Failed! Exiting in...",
                GameResult::Win => "Success! Next board in...",
            };

            let elapsed_time = game_over.msg_timer.elapsed();
            let secs_left = if BOARD_FINISH_MSG_TIME >= elapsed_time {
                // adjust the time by a half second so that the time reads better.
                let adjusted_time = BOARD_FINISH_MSG_TIME + std::time::Duration::from_millis(500);
                (adjusted_time - elapsed_time).as_secs()
            } else {
                0
            };

            let time_text = format!("{} secs", secs_left);

            window.attron(pancurses::A_BLINK);
            for (i, text) in [game_over_text, &time_text].iter().enumerate() {
                window.mvaddstr(
                    grid_center.1 + (i as i32),
                    grid_center.0 - (text.len() / 2) as i32,
                    text,
                );
            }
            window.attroff(pancurses::A_BLINK);
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
