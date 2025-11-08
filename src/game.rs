use std::fmt;

const MAX_SIZE: usize = 64;
const MAX_BOXES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Goal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

const ALL_DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Down,
    Direction::Left,
    Direction::Right,
];

impl Direction {
    fn delta(&self) -> (i8, i8) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    fn index(&self) -> usize {
        match self {
            Direction::Up => 0,
            Direction::Down => 1,
            Direction::Left => 2,
            Direction::Right => 3,
        }
    }

    fn from_index(idx: usize) -> Direction {
        match idx {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            3 => Direction::Right,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::Up => write!(f, "Up"),
            Direction::Down => write!(f, "Down"),
            Direction::Left => write!(f, "Left"),
            Direction::Right => write!(f, "Right"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Push {
    pub box_index: u8,
    pub direction: Direction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pushes {
    // Bitset: bits[0] = Up for all boxes, bits[1] = Down, bits[2] = Left, bits[3] = Right
    // Each u32 holds 32 bits for 32 boxes
    bits: [u32; 4],
}

impl Pushes {
    fn new() -> Self {
        Pushes { bits: [0; 4] }
    }

    fn add(&mut self, box_index: u8, direction: Direction) {
        let dir_idx = direction.index();
        self.bits[dir_idx] |= 1u32 << box_index;
    }

    pub fn len(&self) -> usize {
        self.bits
            .iter()
            .map(|&word| word.count_ones() as usize)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&word| word == 0)
    }

    pub fn iter(&self) -> PushesIter {
        PushesIter {
            moves: self,
            dir_idx: 0,
            box_bits: self.bits[0],
        }
    }
}

pub struct PushesIter<'a> {
    moves: &'a Pushes,
    dir_idx: usize,
    box_bits: u32,
}

impl Iterator for PushesIter<'_> {
    type Item = Push;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Find next set bit in current direction
            if self.box_bits != 0 {
                let box_index = self.box_bits.trailing_zeros() as u8;
                self.box_bits &= self.box_bits - 1; // Clear lowest set bit

                let direction = Direction::from_index(self.dir_idx);
                return Some(Push {
                    box_index,
                    direction,
                });
            }

            // Move to next direction
            self.dir_idx += 1;
            if self.dir_idx >= 4 {
                return None;
            }
            self.box_bits = self.moves.bits[self.dir_idx];
        }
    }
}

impl<'a> IntoIterator for &'a Pushes {
    type Item = Push;
    type IntoIter = PushesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Boxes {
    positions: [(u8, u8); MAX_BOXES],
    count: u8,
    // Maps board position to box index (255 = no box at this position)
    index: [[u8; MAX_SIZE]; MAX_SIZE],
}

impl Boxes {
    fn new() -> Self {
        Boxes {
            positions: [(0, 0); MAX_BOXES],
            count: 0,
            index: [[255u8; MAX_SIZE]; MAX_SIZE],
        }
    }

    fn add(&mut self, x: u8, y: u8) {
        assert!(
            (self.count as usize) < MAX_BOXES,
            "Cannot add box: maximum of {} boxes exceeded",
            MAX_BOXES
        );
        self.positions[self.count as usize] = (x, y);
        self.index[y as usize][x as usize] = self.count;
        self.count += 1;
    }

    fn move_box(&mut self, from_x: u8, from_y: u8, to_x: u8, to_y: u8) {
        let idx = self.index[from_y as usize][from_x as usize];
        self.positions[idx as usize] = (to_x, to_y);
        self.index[from_y as usize][from_x as usize] = 255;
        self.index[to_y as usize][to_x as usize] = idx;
    }

    fn has_box_at(&self, x: u8, y: u8) -> bool {
        self.index[y as usize][x as usize] != 255
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    tiles: [[Tile; MAX_SIZE]; MAX_SIZE],
    player: (u8, u8),
    empty_goals: u8,
    width: u8,
    height: u8,
    boxes: Boxes,
}

impl Game {
    /// Parse a Sokoban board from text format.
    ///
    /// Characters:
    /// - `#` = Wall
    /// - ` ` = Floor (empty space)
    /// - `.` = Goal (target location for boxes)
    /// - `$` = Box
    /// - `@` = Player
    /// - `*` = Box on goal
    /// - `+` = Player on goal
    pub fn from_text(text: &str) -> Result<Self, String> {
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            return Err("Empty board".to_string());
        }

        let height = lines.len();
        let width = lines.iter().map(|line| line.len()).max().unwrap_or(0);

        if width > MAX_SIZE {
            return Err(format!(
                "Board width {} exceeds maximum size {}",
                width, MAX_SIZE
            ));
        }
        if height > MAX_SIZE {
            return Err(format!(
                "Board height {} exceeds maximum size {}",
                height, MAX_SIZE
            ));
        }

        let mut tiles = [[Tile::Floor; MAX_SIZE]; MAX_SIZE];
        let mut player_pos = None;
        let mut boxes = Boxes::new();
        let mut empty_goals: u8 = 0;
        let mut goal_count: u8 = 0;

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                match ch {
                    '#' => tiles[y][x] = Tile::Wall,
                    ' ' => tiles[y][x] = Tile::Floor,
                    '.' => {
                        tiles[y][x] = Tile::Goal;
                        goal_count += 1;
                        empty_goals += 1;
                    }
                    '$' => {
                        tiles[y][x] = Tile::Floor;
                        boxes.add(x as u8, y as u8);
                    }
                    '*' => {
                        tiles[y][x] = Tile::Goal;
                        boxes.add(x as u8, y as u8);
                        goal_count += 1;
                    }
                    '@' => {
                        tiles[y][x] = Tile::Floor;
                        if player_pos.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player_pos = Some((x as u8, y as u8));
                    }
                    '+' => {
                        tiles[y][x] = Tile::Goal;
                        if player_pos.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player_pos = Some((x as u8, y as u8));
                        goal_count += 1;
                        empty_goals += 1;
                    }
                    _ => {
                        return Err(format!(
                            "Invalid character '{}' at position ({}, {})",
                            ch, x, y
                        ));
                    }
                }
            }
        }

        let player_pos = player_pos.ok_or("No player found on board")?;

        // Validate that the number of goals matches the number of boxes
        if goal_count != boxes.count {
            return Err(format!(
                "Goal count ({}) does not match box count ({})",
                goal_count, boxes.count
            ));
        }

        Ok(Game {
            tiles,
            player: player_pos,
            empty_goals,
            width: width as u8,
            height: height as u8,
            boxes,
        })
    }

    pub fn set_player_pos(&mut self, x: u8, y: u8) {
        self.player = (x, y);
    }

    pub fn get_tile(&self, x: u8, y: u8) -> Tile {
        self.tiles[y as usize][x as usize]
    }

    /// Move from position (x, y) in the given direction.
    /// Returns Some((new_x, new_y)) if the new position is within bounds, None otherwise.
    fn move_pos(&self, x: u8, y: u8, dir: Direction) -> Option<(u8, u8)> {
        let (dx, dy) = dir.delta();
        let new_x = x as i32 + dx as i32;
        let new_y = y as i32 + dy as i32;

        if new_x >= 0 && new_y >= 0 && new_x < self.width as i32 && new_y < self.height as i32 {
            Some((new_x as u8, new_y as u8))
        } else {
            None
        }
    }

    /// Push a box according to the given Push.
    /// Updates the player position to where the box was.
    /// Panics if the push is invalid (invalid box index, destination blocked, etc.)
    pub fn push(&mut self, push: Push) {
        assert!(
            (push.box_index as usize) < self.boxes.count as usize,
            "Invalid box index: {}",
            push.box_index
        );

        let (x, y) = self.boxes.positions[push.box_index as usize];
        let (dx, dy) = push.direction.delta();
        let new_x = (x as i8 + dx) as u8;
        let new_y = (y as i8 + dy) as u8;

        let dest_tile = self.get_tile(new_x, new_y);
        assert!(
            !self.boxes.has_box_at(new_x, new_y)
                && (dest_tile == Tile::Floor || dest_tile == Tile::Goal),
            "Cannot push box to ({}, {}): destination blocked",
            new_x,
            new_y
        );

        let source_tile = self.get_tile(x, y);
        let dest_is_goal = dest_tile == Tile::Goal;

        // Update empty_goals count
        if source_tile == Tile::Goal {
            self.empty_goals += 1;
        }
        if dest_is_goal {
            self.empty_goals -= 1;
        }

        // Update box position
        self.boxes.move_box(x, y, new_x, new_y);

        // Update player position to where the box was
        self.player = (x, y);
    }

    /// Undo a push operation.
    /// Moves the box back in the opposite direction and restores player position.
    /// Panics if the unpush is invalid (invalid box index).
    pub fn unpush(&mut self, push: Push) {
        assert!(
            (push.box_index as usize) < self.boxes.count as usize,
            "Invalid box index: {}",
            push.box_index
        );

        // Current box position (after the push we're undoing)
        let (new_x, new_y) = self.boxes.positions[push.box_index as usize];

        // Calculate where box came from (opposite direction)
        let (dx, dy) = push.direction.delta();
        let old_x = (new_x as i8 - dx) as u8;
        let old_y = (new_y as i8 - dy) as u8;

        // Calculate where player was before the push
        let player_old_x = (old_x as i8 - dx) as u8;
        let player_old_y = (old_y as i8 - dy) as u8;

        let current_tile = self.get_tile(new_x, new_y);
        let old_tile = self.get_tile(old_x, old_y);

        // Update empty_goals count
        if current_tile == Tile::Goal {
            self.empty_goals += 1; // Removing box from goal
        }
        if old_tile == Tile::Goal {
            self.empty_goals -= 1; // Placing box on goal
        }

        // Move box back
        self.boxes.move_box(new_x, new_y, old_x, old_y);

        // Restore player position
        self.player = (player_old_x, player_old_y);
    }

    /// Check if all boxes are on goals (win condition)
    pub fn is_solved(&self) -> bool {
        self.empty_goals == 0
    }

    /// Compute all possible box pushes from the current game state.
    /// Uses a single DFS from player position to find all reachable boxes.
    /// Returns the pushes and the canonicalized (lexicographically smallest) player position.
    pub fn compute_pushes(&self) -> (Pushes, (u8, u8)) {
        let mut pushes = Pushes::new();
        let mut reachable = [[false; MAX_SIZE]; MAX_SIZE];
        let mut canonical_pos = self.player;

        // Stack-allocated stack for DFS
        let mut stack: [(u8, u8); MAX_SIZE * MAX_SIZE] = [(0, 0); MAX_SIZE * MAX_SIZE];
        let mut stack_size = 0;

        // DFS from player position to find all reachable positions
        stack[stack_size] = self.player;
        stack_size += 1;
        reachable[self.player.1 as usize][self.player.0 as usize] = true;

        while stack_size > 0 {
            stack_size -= 1;
            let (x, y) = stack[stack_size];

            // Check all 4 directions for adjacent boxes
            for &dir in &ALL_DIRECTIONS {
                if let Some((nx, ny)) = self.move_pos(x, y, dir) {
                    // If there's a box, check if we can push it
                    let box_idx = self.boxes.index[ny as usize][nx as usize];
                    if box_idx != 255 {
                        // Check if destination is free
                        if let Some((dest_x, dest_y)) = self.move_pos(nx, ny, dir) {
                            let dest_tile = self.get_tile(dest_x, dest_y);
                            if !self.boxes.has_box_at(dest_x, dest_y)
                                && (dest_tile == Tile::Floor || dest_tile == Tile::Goal)
                            {
                                // Valid push
                                pushes.add(box_idx, dir);
                            }
                        }
                    } else {
                        let tile = self.get_tile(nx, ny);
                        if (tile == Tile::Floor || tile == Tile::Goal)
                            && !reachable[ny as usize][nx as usize]
                        {
                            // Continue DFS to this floor/goal tile
                            reachable[ny as usize][nx as usize] = true;

                            // Update canonical position if this is lexicographically smaller
                            if (nx, ny) < canonical_pos {
                                canonical_pos = (nx, ny);
                            }

                            stack[stack_size] = (nx, ny);
                            stack_size += 1;
                        }
                    }
                }
            }
        }

        (pushes, canonical_pos)
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                let tile = self.tiles[y as usize][x as usize];
                let has_box = self.boxes.has_box_at(x, y);

                let ch = if (x, y) == self.player {
                    match tile {
                        Tile::Goal => '+',
                        _ => '@',
                    }
                } else if has_box {
                    match tile {
                        Tile::Goal => '*',
                        _ => '$',
                    }
                } else {
                    match tile {
                        Tile::Wall => '#',
                        Tile::Floor => ' ',
                        Tile::Goal => '.',
                    }
                };
                line.push(ch);
            }
            // Trim trailing spaces to match original input format
            writeln!(f, "{}", line.trim_end())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_board() {
        let input = "####\n\
                     # .#\n\
                     #  ###\n\
                     #*@  #\n\
                     #  $ #\n\
                     #  ###\n\
                     ####";
        let game = Game::from_text(input).unwrap();

        assert_eq!(game.width, 6);
        assert_eq!(game.height, 7);
        assert_eq!(game.player, (2, 3));
    }

    #[test]
    fn test_no_player() {
        let input = "####\n\
                     #  #\n\
                     ####";
        assert!(Game::from_text(input).is_err());
    }

    #[test]
    fn test_multiple_players() {
        let input = "####\n\
                     #@@#\n\
                     ####";
        assert!(Game::from_text(input).is_err());
    }

    #[test]
    fn test_player_on_goal() {
        let input = "####\n\
                     #$+ #\n\
                     #$. #\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        assert_eq!(game.player, (2, 1));
        assert_eq!(game.get_tile(2, 1), Tile::Goal);
    }

    #[test]
    fn test_display() {
        let input = "####\n\
                     # .#\n\
                     #  ###\n\
                     #*@  #\n\
                     #  $ #\n\
                     #  ###\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let output = game.to_string();
        assert_eq!(output.trim(), input);
    }

    #[test]
    fn test_is_solved() {
        let solved = "####\n\
                      #*@#\n\
                      ####";
        let game = Game::from_text(solved).unwrap();
        assert!(game.is_solved());

        let unsolved = "####\n\
                        #$.#\n\
                        # @#\n\
                        ####";
        let board = Game::from_text(unsolved).unwrap();
        assert!(!board.is_solved());
    }

    #[test]
    fn test_empty_goals_tarcking() {
        // Board with 1 box on goal, 1 box not on goal
        let input = "####\n\
                     # .#\n\
                     #  ###\n\
                     #*@  #\n\
                     #  $ #\n\
                     #  ###\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        assert_eq!(game.empty_goals, 1);
        assert!(!game.is_solved());

        // Board with all boxes on goals
        let all_solved = "####\n\
                          #*@#\n\
                          ####";
        let game = Game::from_text(all_solved).unwrap();
        assert_eq!(game.empty_goals, 0);
        assert!(game.is_solved());

        // Board with no boxes on goals
        let none_solved = "####\n\
                           #$.#\n\
                           # @#\n\
                           ####";
        let game = Game::from_text(none_solved).unwrap();
        assert_eq!(game.empty_goals, 1);
        assert!(!game.is_solved());
    }

    #[test]
    fn test_goal_box_count_validation() {
        // More goals than boxes - should fail
        let more_goals = "####\n\
                          #..#\n\
                          # $@#\n\
                          ####";
        assert!(Game::from_text(more_goals).is_err());

        // More boxes than goals - should fail
        let more_boxes = "####\n\
                          #$$#\n\
                          # .@#\n\
                          ####";
        assert!(Game::from_text(more_boxes).is_err());

        // Equal goals and boxes - should succeed
        let balanced = "####\n\
                        #$.#\n\
                        # * #\n\
                        # @#\n\
                        ####";
        assert!(Game::from_text(balanced).is_ok());
    }

    #[test]
    fn test_push_basic() {
        // Simple board: player can push box right onto goal
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();

        // Push box right (box at position (2,1) is box index 0)
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        // Box should now be on goal at (3, 1)
        assert_eq!(game.get_tile(3, 1), Tile::Goal);
        assert!(game.boxes.has_box_at(3, 1));
        // Original box position should be floor
        assert_eq!(game.get_tile(2, 1), Tile::Floor);
        assert!(!game.boxes.has_box_at(2, 1));
        // Player should be at old box position
        assert_eq!(game.player, (2, 1));
        // Should be solved
        assert!(game.is_solved());
        assert_eq!(game.empty_goals, 0);
    }

    #[test]
    fn test_push_all_directions() {
        // Test pushing right
        let input = "####\n\
                     #@$ #\n\
                     # . #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });
        assert_eq!(game.player, (2, 1));
        assert!(game.boxes.has_box_at(3, 1));

        // Test pushing down
        let input = "#####\n\
                     # @ #\n\
                     # $ #\n\
                     # . #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.boxes.index[2][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Down,
        });
        assert_eq!(game.player, (2, 2));
        assert_eq!(game.get_tile(2, 3), Tile::Goal);
        assert!(game.boxes.has_box_at(2, 3));

        // Test pushing left
        let input = "####\n\
                     # $@#\n\
                     # . #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Left,
        });
        assert_eq!(game.player, (2, 1));
        assert!(game.boxes.has_box_at(1, 1));

        // Test pushing up
        let input = "#####\n\
                     # . #\n\
                     # $ #\n\
                     # @ #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.boxes.index[2][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Up,
        });
        assert_eq!(game.player, (2, 2));
        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(game.boxes.has_box_at(2, 1));
    }

    #[test]
    fn test_push_floor_to_goal() {
        // Push box from floor onto goal
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();

        assert_eq!(game.empty_goals, 1);
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        assert_eq!(game.get_tile(3, 1), Tile::Goal);
        assert!(game.boxes.has_box_at(3, 1));
        assert_eq!(game.empty_goals, 0);
    }

    #[test]
    fn test_push_goal_to_floor() {
        // Push box from goal onto floor
        let input = "#####\n\
                     #@*  #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();

        assert_eq!(game.empty_goals, 0);
        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(game.boxes.has_box_at(2, 1));

        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(!game.boxes.has_box_at(2, 1));
        assert_eq!(game.get_tile(3, 1), Tile::Floor);
        assert!(game.boxes.has_box_at(3, 1));
        assert_eq!(game.empty_goals, 1);
        assert_eq!(game.player, (2, 1));
    }

    #[test]
    fn test_push_goal_to_goal() {
        // Push box from one goal to another goal
        let input = "######\n\
                     #@*.$#\n\
                     ######";
        let mut game = Game::from_text(input).unwrap();

        assert_eq!(game.empty_goals, 1);
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(!game.boxes.has_box_at(2, 1));
        assert_eq!(game.get_tile(3, 1), Tile::Goal);
        assert!(game.boxes.has_box_at(3, 1));
        assert_eq!(game.empty_goals, 1);
    }

    #[test]
    #[should_panic(expected = "Invalid box index")]
    fn test_push_no_box() {
        let input = "####\n\
                     #@  #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();

        // Try to push with invalid box index
        game.push(Push {
            box_index: 10,
            direction: Direction::Right,
        });
    }

    #[test]
    #[should_panic(expected = "destination blocked")]
    fn test_push_blocked() {
        let input = "####\n\
                     #@$##\n\
                     # . #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();

        // Try to push box into wall
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });
    }

    #[test]
    #[should_panic(expected = "destination blocked")]
    fn test_push_into_another_box() {
        let input = "######\n\
                     #@$$  #\n\
                     # ..  #\n\
                     ######";
        let mut game = Game::from_text(input).unwrap();

        // Try to push box into another box
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });
    }

    #[test]
    fn test_compute_pushes() {
        let input = "####\n\
                     # .#\n\
                     #  ###\n\
                     #*@  #\n\
                     #  $ #\n\
                     #  ###\n\
                     ####";
        let game = Game::from_text(input).unwrap();
        let (pushes, canonical_pos) = game.compute_pushes();
        let mut actual = pushes.iter().collect::<Vec<_>>();
        let mut expected = vec![
            Push {
                box_index: 0,
                direction: Direction::Up,
            },
            Push {
                box_index: 0,
                direction: Direction::Down,
            },
            Push {
                box_index: 1,
                direction: Direction::Left,
            },
            Push {
                box_index: 1,
                direction: Direction::Right,
            },
        ];

        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);

        // Check canonical position - should be lexicographically smallest reachable position
        // Player starts at (2, 3) and can reach many positions including (1, 1)
        assert_eq!(canonical_pos, (1, 1));
    }

    #[test]
    fn test_unpush() {
        // Test unpush restores original state
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();

        // Save original state
        let original_player = game.player;
        let original_boxes = game.boxes.clone();
        let original_goals = game.empty_goals;

        // Push box right
        let box_idx = game.boxes.index[1][2];
        let push = Push {
            box_index: box_idx,
            direction: Direction::Right,
        };
        game.push(push);

        // Verify state changed
        assert_eq!(game.player, (2, 1));
        assert!(game.boxes.has_box_at(3, 1));
        assert_eq!(game.empty_goals, 0);
        assert!(game.is_solved());

        // Unpush
        game.unpush(push);

        // Should be back to original state
        assert_eq!(game.player, original_player);
        assert_eq!(game.boxes, original_boxes);
        assert_eq!(game.empty_goals, original_goals);
        assert!(!game.is_solved());
    }

    #[test]
    fn test_unpush_all_directions() {
        // Test unpush in all directions
        let tests = vec![
            (Direction::Right, "####\n#@$ #\n# . #\n####"),
            (Direction::Down, "#####\n# @ #\n# $ #\n# . #\n#####"),
            (Direction::Left, "####\n# $@#\n# . #\n####"),
            (Direction::Up, "#####\n# . #\n# $ #\n# @ #\n#####"),
        ];

        for (direction, input) in tests {
            let mut game = Game::from_text(input).unwrap();
            let original = game.clone();

            // Find the box
            let box_idx = game.boxes.positions[0];
            let box_idx = game.boxes.index[box_idx.1 as usize][box_idx.0 as usize];

            let push = Push {
                box_index: box_idx,
                direction,
            };

            game.push(push);
            game.unpush(push);

            assert_eq!(game.player, original.player, "Failed for {:?}", direction);
            assert_eq!(game.boxes, original.boxes, "Failed for {:?}", direction);
            assert_eq!(
                game.empty_goals, original.empty_goals,
                "Failed for {:?}",
                direction
            );
        }
    }
}
