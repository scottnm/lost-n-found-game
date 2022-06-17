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

    pub struct Timer {
        start_time: std::time::Instant,
        duration: std::time::Duration,
    }

    impl Timer {
        pub fn new(duration: std::time::Duration) -> Self {
            Timer {
                start_time: std::time::Instant::now(),
                duration,
            }
        }

        pub fn time_left(&self) -> std::time::Duration {
            self.duration - std::cmp::min(self.start_time.elapsed(), self.duration)
        }

        pub fn finished(&self) -> bool {
            // n.b. should be const, but that feature hasn't yet stabilized
            let zero = std::time::Duration::new(0, 0);
            self.time_left() == zero
        }
    }
}

use utils::Rect;
use utils::Timer;

const TITLE: &str = "Lost-n-Found";

mod game {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HintDir {
        Left,
        Up,
        Right,
        Down,
    }

    impl HintDir {
        pub fn flip(&self) -> Self {
            match self {
                HintDir::Left => HintDir::Right,
                HintDir::Right => HintDir::Left,
                HintDir::Down => HintDir::Up,
                HintDir::Up => HintDir::Down,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TrapType {
        Confusion,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum GridItem {
        Solution,
        Hint(HintDir),
        Trap(TrapType),
        Empty,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct GridCell {
        pub item: GridItem,
        pub revealed: bool,
    }

    struct CellTimer {
        x: i32,
        y: i32,
        timer: Timer,
    }

    pub struct GameGrid {
        cells: Box<[GridCell]>,
        timers: Vec<CellTimer>,
        max_revealed_cells: usize,
        width: i32,
        height: i32,
    }

    impl GameGrid {
        pub fn new(
            width: i32,
            height: i32,
            max_revealed_cells: usize,
            rng: &mut dyn RangeRng<usize>,
        ) -> Self {
            let solution_cell = (
                rng.gen_range(0, width as usize) as i32,
                rng.gen_range(0, height as usize) as i32,
            );

            let num_cells = (width * height) as usize;
            let mut cells = Vec::with_capacity(num_cells);
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
                            enum RandomCell {
                                Empty,
                                Trap,
                                Hint,
                            }

                            // 70% chance of generating a hint, 20% chance of generating a dud, 10% of generating a trap
                            const RANDOM_CELL_DISTRIBUTION: [RandomCell; 10] = [
                                RandomCell::Trap,
                                RandomCell::Empty,
                                RandomCell::Empty,
                                RandomCell::Hint,
                                RandomCell::Hint,
                                RandomCell::Hint,
                                RandomCell::Hint,
                                RandomCell::Hint,
                                RandomCell::Hint,
                                RandomCell::Hint,
                            ];

                            let random_cell = snm_rand_utils::range_rng::select_rand(
                                &RANDOM_CELL_DISTRIBUTION,
                                rng,
                            );

                            match random_cell {
                                RandomCell::Empty => GridItem::Empty,
                                RandomCell::Trap => GridItem::Trap(TrapType::Confusion),
                                RandomCell::Hint => GridItem::Hint(displacement_to_hint_direction(
                                    x_displacement,
                                    y_displacement,
                                )),
                            }
                        }
                    };

                    cells.push(GridCell {
                        item,
                        revealed: false,
                    });
                }
            }

            GameGrid {
                cells: cells.into_boxed_slice(),
                timers: Vec::with_capacity(max_revealed_cells + 1),
                max_revealed_cells,
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

        pub fn cell(&self, x: i32, y: i32) -> Option<GridCell> {
            if x < 0 || x >= self.width || y < 0 || y >= self.height {
                return None;
            }

            let index = (self.width * y + x) as usize;
            Some(self.cells[index])
        }

        fn mut_cell(&mut self, x: i32, y: i32) -> Option<&mut GridCell> {
            if x < 0 || x >= self.width || y < 0 || y >= self.height {
                return None;
            }

            let index = (self.width * y + x) as usize;
            Some(&mut self.cells[index])
        }

        pub fn try_reveal(&mut self, x: i32, y: i32) -> Option<GridItem> {
            let revealed_item = self.mut_cell(x, y).map(|mut_cell| {
                mut_cell.revealed = true;
                mut_cell.item
            });

            if revealed_item.is_some() {
                self.timers.push(CellTimer {
                    x,
                    y,
                    timer: Timer::new(std::time::Duration::from_secs(4)),
                });
            }

            revealed_item
        }

        pub fn reset_expired_cells(&mut self) {
            if self.timers.is_empty() {
                return;
            }

            if self.timers.len() > self.max_revealed_cells || self.timers[0].timer.finished() {
                let oldest_cell_timer = self.timers.remove(0);
                let cell_to_revert = self
                    .mut_cell(oldest_cell_timer.x, oldest_cell_timer.y)
                    .unwrap();
                cell_to_revert.revealed = false;
            }
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
    pancurses::mousemask(mousemask, Some(&mut oldmask));
}

enum Color {
    BlackOnGreen,
    BlackOnYellow,
    BlackOnRed,
    BlackOnBlue,
    BlackOnWhite,
    BlackOnGray,
    BlackOnDarkGray,
    BlackOnOrange,
}

impl Color {
    fn to_num(&self) -> u8 {
        match self {
            Color::BlackOnGreen => 1,
            Color::BlackOnYellow => 2,
            Color::BlackOnRed => 3,
            Color::BlackOnBlue => 4,
            Color::BlackOnWhite => 5,
            Color::BlackOnGray => 6,
            Color::BlackOnDarkGray => 7,
            Color::BlackOnOrange => 8,
        }
    }

    fn setup() {
        pancurses::start_color();

        pancurses::init_color(pancurses::COLOR_GREEN, 500, 1000, 500);
        pancurses::init_pair(
            Color::BlackOnGreen.to_num() as i16,
            pancurses::COLOR_BLACK,
            pancurses::COLOR_GREEN,
        );

        pancurses::init_color(pancurses::COLOR_YELLOW, 750, 750, 500);
        pancurses::init_pair(
            Color::BlackOnYellow.to_num() as i16,
            pancurses::COLOR_BLACK,
            pancurses::COLOR_YELLOW,
        );

        pancurses::init_color(pancurses::COLOR_BLUE, 500, 500, 1000);
        pancurses::init_pair(
            Color::BlackOnBlue.to_num() as i16,
            pancurses::COLOR_BLACK,
            pancurses::COLOR_BLUE,
        );

        pancurses::init_color(pancurses::COLOR_RED, 1000, 500, 500);
        pancurses::init_pair(
            Color::BlackOnRed.to_num() as i16,
            pancurses::COLOR_BLACK,
            pancurses::COLOR_RED,
        );

        pancurses::init_pair(
            Color::BlackOnWhite.to_num() as i16,
            pancurses::COLOR_BLACK,
            pancurses::COLOR_WHITE,
        );

        const CUSTOM_GRAY: i16 = 10;
        pancurses::init_color(CUSTOM_GRAY, 500, 500, 500);
        pancurses::init_pair(
            Color::BlackOnGray.to_num() as i16,
            pancurses::COLOR_BLACK,
            CUSTOM_GRAY,
        );

        const CUSTOM_DARK_GRAY: i16 = 11;
        pancurses::init_color(CUSTOM_DARK_GRAY, 250, 250, 250);
        pancurses::init_pair(
            Color::BlackOnDarkGray.to_num() as i16,
            pancurses::COLOR_BLACK,
            CUSTOM_DARK_GRAY,
        );

        const CUSTOM_ORANGE: i16 = 12;
        pancurses::init_color(CUSTOM_ORANGE, 750, 450, 0);
        pancurses::init_pair(
            Color::BlackOnOrange.to_num() as i16,
            pancurses::COLOR_BLACK,
            CUSTOM_ORANGE,
        );
    }

    pub fn to_color_pair(&self) -> pancurses::chtype {
        pancurses::COLOR_PAIR(self.to_num() as pancurses::chtype)
    }
}

#[derive(PartialEq, Eq)]
enum GameResult {
    Win,
    Lose,
}

struct GameOverState {
    result: GameResult,
    msg_timer: Timer,
    frozen_game_time: std::time::Duration,
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
    let mut level = 1;
    loop {
        let result = run_game(level, &window);
        if result == GameResult::Lose {
            break;
        }

        level += 1;
    }
}

#[derive(Debug)]
struct MouseState {
    click: bool,
    x: i32,
    y: i32,
}

fn get_mouse_update(window: &pancurses::Window) -> Option<MouseState> {
    if let Some(pancurses::Input::KeyMouse) = window.getch() {
        if let Ok(mouse_event) = pancurses::getmouse() {
            return Some(MouseState {
                click: (mouse_event.bstate & pancurses::BUTTON1_CLICKED) != 0,
                x: mouse_event.x,
                y: mouse_event.y,
            });
        }
    }

    None
}

fn render_level_header(level: usize, level_rect: &Rect, window: &pancurses::Window) {
    window.mvaddstr(level_rect.top, level_rect.left, format!("Level: {}", level));
}

fn render_game_timer(
    time_remaining: std::time::Duration,
    time_rect: &Rect,
    window: &pancurses::Window,
) {
    assert!(time_remaining >= std::time::Duration::new(0, 0));
    window.mvaddstr(
        time_rect.top,
        time_rect.left,
        format!(
            "Time: {:02}.{:03}",
            time_remaining.as_secs(),
            time_remaining.subsec_millis(),
        ),
    );
}

fn render_game_board(
    game_grid: &GameGrid,
    game_over_state: &Option<GameOverState>,
    confusion_state: Option<bool>,
    grid_rect: &Rect,
    window: &pancurses::Window,
    mouse_state: &MouseState,
) {
    // add the leading border cells on top of the grid
    let border_attribute = Color::BlackOnDarkGray.to_color_pair();

    let game_lost = game_over_state
        .as_ref()
        .map(|game_over| game_over.result == GameResult::Lose)
        .unwrap_or(false);

    let show_confusion = confusion_state.is_some() && !game_lost;

    fn generate_cell(c: u64) -> [[u64; 3]; 2] {
        const EMPTY: u64 = ' ' as u64;
        [[c, EMPTY, c], [c, EMPTY, c]]
    }

    let left_cell = generate_cell('<' as u64);
    let right_cell = generate_cell('>' as u64);
    let up_cell = generate_cell('^' as u64);
    let down_cell = generate_cell('v' as u64);
    let diamond_cell = generate_cell(pancurses::ACS_DIAMOND());
    let empty_cell = generate_cell(' ' as u64);
    let confusion_trap_cell = generate_cell('~' as u64);

    // render the grid
    for row in 0..game_grid.height() {
        let row_offset = (row * 2) + grid_rect.top + 1;

        // render each cell
        for col in 0..game_grid.width() {
            let col_offset = grid_rect.left + 1 + 4 * col;
            // safe to unwrap since we are iterating over the grid by its own bounds
            let grid_cell = game_grid.cell(col, row).unwrap();

            // show the cell if it's currently revealed or if we lost
            let show_cell = grid_cell.revealed || game_lost;

            let (grid_item_lines, grid_item_attributes) = if show_cell {
                match grid_cell.item {
                    GridItem::Solution => (
                        diamond_cell,
                        Color::BlackOnWhite.to_color_pair() | pancurses::A_BLINK,
                    ),
                    GridItem::Hint(hint_dir) => {
                        // If we are confused and want to show it, flip the hint directions
                        let hint_dir = if show_confusion && confusion_state.unwrap() {
                            hint_dir.flip()
                        } else {
                            hint_dir
                        };

                        match hint_dir {
                            HintDir::Left => (left_cell, Color::BlackOnBlue.to_color_pair()),
                            HintDir::Right => (right_cell, Color::BlackOnYellow.to_color_pair()),
                            HintDir::Up => (up_cell, Color::BlackOnRed.to_color_pair()),
                            HintDir::Down => (down_cell, Color::BlackOnGreen.to_color_pair()),
                        }
                    }
                    GridItem::Trap(trap_type) => match trap_type {
                        TrapType::Confusion => {
                            (confusion_trap_cell, Color::BlackOnOrange.to_color_pair())
                        }
                    },
                    GridItem::Empty => (empty_cell, Color::BlackOnGray.to_color_pair()),
                }
            } else {
                (empty_cell, Color::BlackOnDarkGray.to_color_pair())
            };

            window.attron(grid_item_attributes);
            window.mv(row_offset, col_offset);
            for c in &grid_item_lines[0] {
                window.addch(*c);
            }
            // use underlines to draw interior horizontal borders for cells
            window.attron(pancurses::A_UNDERLINE);
            window.mv(row_offset + 1, col_offset);
            for c in &grid_item_lines[1] {
                window.addch(*c);
            }
            window.attroff(pancurses::A_UNDERLINE);
            window.attroff(grid_item_attributes);

            // draw interior vertical borders for cells
            if col < game_grid.width() - 1 {
                window.attron(border_attribute);
                window.mvaddch(row_offset, col_offset + 3, pancurses::ACS_VLINE());
                window.mvaddch(row_offset + 1, col_offset + 3, pancurses::ACS_VLINE());
                window.attroff(border_attribute);
            }
        }
    }

    // if we are hovering over a grid cell, highlight the selected cell
    let mouse_game_grid_pos =
        xform::window_to_game_grid(mouse_state.x, mouse_state.y, grid_rect.left, grid_rect.top);
    if game_grid
        .cell(mouse_game_grid_pos.0, mouse_game_grid_pos.1)
        .is_some()
    {
        let highlighted_rect = xform::game_grid_to_window(
            mouse_game_grid_pos.0,
            mouse_game_grid_pos.1,
            grid_rect.left,
            grid_rect.top,
        );

        for row in highlighted_rect.top..=highlighted_rect.bottom() {
            for col in highlighted_rect.left..=highlighted_rect.right() {
                window.mvchgat(row, col, 1, window.mvinch(row, col) | pancurses::A_BLINK, 0);
            }
        }
    }
}

fn render_game_over_text(
    game_over_state: &GameOverState,
    window: &pancurses::Window,
    game_over_rect: &Rect,
) {
    let (game_over_text, game_over_attributes) = match game_over_state.result {
        GameResult::Lose => ("Failed! Exiting in...", Color::BlackOnRed.to_color_pair()),
        GameResult::Win => (
            "Success! Next board in...",
            Color::BlackOnGreen.to_color_pair(),
        ),
    };

    // adjust the time by a half second so that the time reads better.
    let adjusted_time_left =
        game_over_state.msg_timer.time_left() + std::time::Duration::from_millis(500);
    let secs_left = adjusted_time_left.as_secs();

    let time_text = format!("{} secs", secs_left);

    window.attron(game_over_attributes);
    for (i, text) in [game_over_text, &time_text].iter().enumerate() {
        window.mvaddstr(
            game_over_rect.center_y() + (i as i32),
            game_over_rect.center_x() - (text.len() / 2) as i32,
            text,
        );
    }
    window.attroff(game_over_attributes);
}

fn get_board_time_from_level(level: usize) -> std::time::Duration {
    const MAX_TIME_SECS: u64 = 15;
    const MAX_TIME_REDUCTION_SECS: u64 = 10;
    const MIN_AFFECTED_LEVEL: usize = 6; // don't start reducing the board time until we get to at least level 6

    let adjusted_level = level - std::cmp::min(level, MIN_AFFECTED_LEVEL);
    let difficulty_step = adjusted_level as u64 / 3; // every 3 levels the difficulty step increases
    let time_reduction_in_secs = difficulty_step * 2; // every difficulty step drops the timer by 2 seconds
    let capped_time_reduction_in_secs =
        std::cmp::min(time_reduction_in_secs, MAX_TIME_REDUCTION_SECS);

    std::time::Duration::from_secs(MAX_TIME_SECS - capped_time_reduction_in_secs)
}

fn get_grid_size_from_level(level: usize) -> (i32, i32) {
    // start as a 15x10 board and increase by 1 in each dimension every 3 levels
    const START_BOARD_SIZE: (i32, i32) = (15, 10);
    const MAX_BOARD_GROWTH: i32 = 10;

    let difficulty_step = level / 3; // every 3 levels the difficulty step increases
    let board_growth = difficulty_step as i32; // every difficulty step increases the board by 1 in each dimension
    let capped_board_growth = std::cmp::min(board_growth, MAX_BOARD_GROWTH);

    (
        START_BOARD_SIZE.0 + capped_board_growth,
        START_BOARD_SIZE.1 + capped_board_growth,
    )
}

fn get_max_revealed_cells_from_level(level: usize) -> usize {
    const INITIAL_MAX_REVEALED_CELLS: usize = 6;
    const MAX_REVEALED_CELL_REDUCTION: usize = 5;

    let difficulty_step = level / 5; // every 5 levels, you lose 1 extra revealed cell
    let revealed_cell_reduction = difficulty_step; // every difficulty step increases the board by 1 in each dimension
    let capped_revealed_cell_reduction =
        std::cmp::min(revealed_cell_reduction, MAX_REVEALED_CELL_REDUCTION);
    INITIAL_MAX_REVEALED_CELLS - capped_revealed_cell_reduction
}

fn run_game(level: usize, window: &pancurses::Window) -> GameResult {
    // Not using a Rect because this grid isn't ACTUALLY sized normally like a rect. There are spaces
    let mut rng = ThreadRangeRng::new();

    let (game_grid_width, game_grid_height) = get_grid_size_from_level(level);
    let max_revealed_cells = get_max_revealed_cells_from_level(level);
    let mut game_grid = GameGrid::new(
        game_grid_width,
        game_grid_height,
        max_revealed_cells,
        &mut rng,
    );

    let grid_bounds = xform::game_grid_to_window(game_grid.width(), game_grid.height(), 0, 0);
    let grid_rect = Rect {
        left: (window.get_max_x() - grid_bounds.right()) / 2,
        top: (window.get_max_y() - grid_bounds.bottom()) / 2,
        width: grid_bounds.right(),
        height: grid_bounds.bottom(),
    };

    let game_over_rect = Rect {
        left: grid_rect.left,
        top: grid_rect.bottom() + 2,
        width: grid_rect.width,
        height: 2,
    };

    let time_rect = Rect {
        left: grid_rect.left,
        top: grid_rect.top - 4,
        width: 30,
        height: 2,
    };

    let level_rect = Rect {
        left: time_rect.left,
        top: time_rect.top - 1,
        width: 30,
        height: 2,
    };

    let mut mouse_state = MouseState {
        click: false,
        x: 0,
        y: 0,
    };

    const BOARD_FINISH_MSG_TIME: std::time::Duration = std::time::Duration::from_secs(5);
    const CONFUSION_TIME: std::time::Duration = std::time::Duration::from_secs(3);

    let game_timer = Timer::new(get_board_time_from_level(level));
    let mut confusion_timer = None;

    let mut game_over_state: Option<GameOverState> = None;
    while game_over_state.is_none() || !game_over_state.as_ref().unwrap().msg_timer.finished() {
        // If we get a mouse event, update our mouse state
        mouse_state.click = false; // clear out any mouse state from the last frame
        if let Some(mouse_update) = get_mouse_update(&window) {
            mouse_state = mouse_update;
        }

        // Update the board and check if we've triggered a game over
        if game_over_state.is_none() {
            game_grid.reset_expired_cells();

            // check for the lose state
            if game_timer.finished() {
                game_over_state = Some(GameOverState {
                    result: GameResult::Lose,
                    msg_timer: Timer::new(BOARD_FINISH_MSG_TIME),
                    frozen_game_time: game_timer.time_left(),
                });
            } else if mouse_state.click {
                // convert the mouse position to an item in a grid cell
                let grid_pos = xform::window_to_game_grid(
                    mouse_state.x,
                    mouse_state.y,
                    grid_rect.left,
                    grid_rect.top,
                );

                match game_grid.try_reveal(grid_pos.0, grid_pos.1) {
                    // check if our last input triggered a win state
                    Some(GridItem::Solution) => {
                        game_over_state = Some(GameOverState {
                            result: GameResult::Win,
                            msg_timer: Timer::new(BOARD_FINISH_MSG_TIME),
                            frozen_game_time: game_timer.time_left(),
                        })
                    }
                    // check if our last input revealed a trap
                    Some(GridItem::Trap(trap_type)) => match trap_type {
                        TrapType::Confusion => {
                            confusion_timer = Some(Timer::new(CONFUSION_TIME));
                        }
                    },
                    _ => (),
                }
            }
        }

        // Clear the confusion timer once it expires
        if confusion_timer.is_some() && confusion_timer.as_ref().unwrap().finished() {
            confusion_timer = None;
        }

        fn flip_every_half_second(time: std::time::Duration) -> bool {
            time.as_millis() % 1000 < 500
        }

        // If the confusion timer is set, flip the confusion state every half second
        let confusion_state = confusion_timer.as_ref().map(|enabled_confusion_timer| {
            flip_every_half_second(enabled_confusion_timer.time_left())
        });

        let game_time_remaining = match &game_over_state {
            Some(game_over) => game_over.frozen_game_time,
            None => game_timer.time_left(),
        };

        // use erase instead of clear to avoid tearing
        window.erase();

        render_level_header(level, &level_rect, &window);
        render_game_timer(game_time_remaining, &time_rect, &window);
        render_game_board(
            &game_grid,
            &game_over_state,
            confusion_state,
            &grid_rect,
            &window,
            &mouse_state,
        );

        if let Some(game_over) = &game_over_state {
            render_game_over_text(game_over, &window, &game_over_rect);
        }

        window.refresh();

        // Yield for 1/30th of a second. Don't hog that CPU.
        std::thread::sleep(std::time::Duration::from_millis(33));
    }

    game_over_state.unwrap().result
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
