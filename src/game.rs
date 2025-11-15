use crate::bits::{Bitvector, BitvectorIter, LazyBitboard};
use arrayvec::ArrayVec;
use std::fmt;

pub const MAX_SIZE: usize = 64;
pub const MAX_BOXES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Goal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPos {
    Known(u8, u8),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub const ALL_DIRECTIONS: [Direction; 4] = [
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
pub struct Move {
    pub box_index: u8,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveByPos {
    pub box_pos: (u8, u8),
    pub direction: Direction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moves {
    // Bitset: bits[0] = Up, bits[1] = Down, bits[2] = Left, bits[3] = Right
    // Each Bitvector holds 64 bits for 64 boxes (box indices 0-63)
    bits: [Bitvector; 4],
}

impl Moves {
    fn new() -> Self {
        Moves {
            bits: [Bitvector::new(); 4],
        }
    }

    fn add(&mut self, box_index: u8, direction: Direction) {
        let dir_idx = direction.index();
        self.bits[dir_idx].set(box_index);
    }

    pub fn len(&self) -> usize {
        self.bits.iter().map(|bv| bv.len()).sum()
    }

    pub fn boxes(&self) -> Bitvector {
        self.bits[0]
            .union(&self.bits[1])
            .union(&self.bits[2])
            .union(&self.bits[3])
    }

    pub fn is_empty(&self) -> bool {
        self.boxes().is_empty()
    }

    pub fn contains(&self, push: Move) -> bool {
        let dir_idx = push.direction.index();
        self.bits[dir_idx].get(push.box_index)
    }

    pub fn iter(&self) -> MovesIter {
        MovesIter {
            bits: self.bits,
            dir_idx: 0,
            current_iter: self.bits[0].iter(),
        }
    }
}

pub struct MovesIter {
    bits: [Bitvector; 4],
    dir_idx: usize,
    current_iter: BitvectorIter,
}

impl Iterator for MovesIter {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(box_index) = self.current_iter.next() {
                let direction = Direction::from_index(self.dir_idx);
                return Some(Move {
                    box_index,
                    direction,
                });
            }

            // Move to next direction
            self.dir_idx += 1;
            if self.dir_idx >= 4 {
                return None;
            }
            self.current_iter = self.bits[self.dir_idx].iter();
        }
    }
}

impl IntoIterator for &'_ Moves {
    type Item = Move;
    type IntoIter = MovesIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Boxes {
    start_positions: [(u8, u8); MAX_BOXES],
    positions: [(u8, u8); MAX_BOXES],
    count: u8,
    // Maps board position to box index (255 = no box at this position)
    index: [[u8; MAX_SIZE]; MAX_SIZE],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Goals {
    positions: [(u8, u8); MAX_BOXES],
    count: u8,
}

impl Goals {
    fn new() -> Self {
        Goals {
            positions: [(0, 0); MAX_BOXES],
            count: 0,
        }
    }

    fn add(&mut self, x: u8, y: u8) {
        assert!(
            (self.count as usize) < MAX_BOXES,
            "Cannot add goal: maximum of {} goals exceeded",
            MAX_BOXES
        );
        self.positions[self.count as usize] = (x, y);
        self.count += 1;
    }
}

impl Boxes {
    fn new() -> Self {
        Boxes {
            start_positions: [(0, 0); MAX_BOXES],
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
        self.start_positions[self.count as usize] = (x, y);
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
    player: PlayerPos,
    empty_goals: u8,
    width: u8,
    height: u8,
    boxes: Boxes,
    goals: Goals,
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
        let mut goals = Goals::new();
        let mut empty_goals: u8 = 0;

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                match ch {
                    '#' => tiles[y][x] = Tile::Wall,
                    ' ' => tiles[y][x] = Tile::Floor,
                    '.' => {
                        tiles[y][x] = Tile::Goal;
                        goals.add(x as u8, y as u8);
                        empty_goals += 1;
                    }
                    '$' => {
                        tiles[y][x] = Tile::Floor;
                        boxes.add(x as u8, y as u8);
                    }
                    '*' => {
                        tiles[y][x] = Tile::Goal;
                        goals.add(x as u8, y as u8);
                        boxes.add(x as u8, y as u8);
                    }
                    '@' => {
                        tiles[y][x] = Tile::Floor;
                        if player_pos.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player_pos = Some(PlayerPos::Known(x as u8, y as u8));
                    }
                    '+' => {
                        tiles[y][x] = Tile::Goal;
                        if player_pos.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player_pos = Some(PlayerPos::Known(x as u8, y as u8));
                        goals.add(x as u8, y as u8);
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
        if goals.count != boxes.count {
            return Err(format!(
                "Goal count ({}) does not match box count ({})",
                goals.count, boxes.count
            ));
        }

        Ok(Game {
            tiles,
            player: player_pos,
            empty_goals,
            width: width as u8,
            height: height as u8,
            boxes,
            goals,
        })
    }

    pub fn get_tile(&self, x: u8, y: u8) -> Tile {
        self.tiles[y as usize][x as usize]
    }

    pub fn box_count(&self) -> usize {
        self.boxes.count as usize
    }

    pub fn box_pos(&self, index: usize) -> (u8, u8) {
        self.boxes.positions[index]
    }

    pub fn box_start_pos(&self, index: usize) -> (u8, u8) {
        self.boxes.start_positions[index]
    }

    pub fn goal_pos(&self, index: usize) -> (u8, u8) {
        self.goals.positions[index]
    }

    /// Get the box index at the given position, if any.
    /// Returns Some(box_index) if there is a box at (x, y), None otherwise.
    pub fn box_at(&self, x: u8, y: u8) -> Option<u8> {
        let idx = self.boxes.index[y as usize][x as usize];
        if idx == 255 { None } else { Some(idx) }
    }

    /// Move from position (x, y) in the given direction.
    /// Returns Some((new_x, new_y)) if the new position is within bounds, None otherwise.
    pub fn push_pos(&self, x: u8, y: u8, dir: Direction) -> Option<(u8, u8)> {
        let (dx, dy) = dir.delta();
        let new_x = x as i32 + dx as i32;
        let new_y = y as i32 + dy as i32;

        if new_x >= 0 && new_y >= 0 && new_x < self.width as i32 && new_y < self.height as i32 {
            Some((new_x as u8, new_y as u8))
        } else {
            None
        }
    }

    /// Move from position (x, y) in the opposite direction of dir.
    /// Returns Some((new_x, new_y)) if the new position is within bounds, None otherwise.
    pub fn pull_pos(&self, x: u8, y: u8, dir: Direction) -> Option<(u8, u8)> {
        let (dx, dy) = dir.delta();
        let new_x = x as i32 - dx as i32;
        let new_y = y as i32 - dy as i32;

        if new_x >= 0 && new_y >= 0 && new_x < self.width as i32 && new_y < self.height as i32 {
            Some((new_x as u8, new_y as u8))
        } else {
            None
        }
    }

    /// Pushes a box.
    /// Updates the player position to where the box was.
    /// Panics if the push is invalid (invalid box index, destination blocked, etc.)
    pub fn push(&mut self, push: Move) {
        assert!(
            (push.box_index as usize) < self.boxes.count as usize,
            "Invalid box index: {}",
            push.box_index
        );

        let (x, y) = self.boxes.positions[push.box_index as usize];
        let (new_x, new_y) = self
            .push_pos(x, y, push.direction)
            .expect("Push destination out of bounds");

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
        self.player = PlayerPos::Known(x, y);
    }

    pub fn push_by_pos(&mut self, push: MoveByPos) {
        let (x, y) = push.box_pos;
        let box_index = self
            .box_at(x, y)
            .unwrap_or_else(|| panic!("No box at position ({}, {})", x, y));

        self.push(Move {
            box_index,
            direction: push.direction,
        });
    }

    pub fn pull(&mut self, pull: Move) {
        assert!(
            (pull.box_index as usize) < self.boxes.count as usize,
            "Invalid box index: {}",
            pull.box_index
        );

        // Current box position (after the push we're undoing)
        let (new_x, new_y) = self.boxes.positions[pull.box_index as usize];

        // Calculate where box came from (opposite direction)
        let (old_x, old_y) = self
            .pull_pos(new_x, new_y, pull.direction)
            .expect("Pull source out of bounds");

        // Calculate where player was before the push
        let (player_old_x, player_old_y) = self
            .pull_pos(old_x, old_y, pull.direction)
            .expect("Pull player position out of bounds");

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
        self.player = PlayerPos::Known(player_old_x, player_old_y);
    }

    /// Check if all boxes are on goals (win condition)
    pub fn is_solved(&self) -> bool {
        self.empty_goals == 0
    }

    /// Make a game state  in the solved position where all boxes are on goals
    /// and the player position is unknown. This is useful for backward search.
    pub fn make_goal_state(&self) -> Self {
        let mut game = self.clone();

        // Move all boxes to their corresponding goals
        // Do this in two passes to avoid clobbering unprocessed boxes

        // First pass: clear all current positions in index
        for i in 0..game.boxes.count as usize {
            let current_pos = game.boxes.positions[i];
            game.boxes.index[current_pos.1 as usize][current_pos.0 as usize] = 255;
        }

        // Second pass: set all new positions
        for i in 0..game.boxes.count as usize {
            let goal_pos = game.goals.positions[i];
            game.boxes.positions[i] = goal_pos;
            game.boxes.index[goal_pos.1 as usize][goal_pos.0 as usize] = i as u8;
        }

        // Set empty_goals to 0 since all boxes are on goals
        game.empty_goals = 0;

        // Set player position to unknown
        game.player = PlayerPos::Unknown;

        game
    }

    /// Compute the canonical (lexicographically smallest reachable) player position.
    /// If player position is Unknown, returns Unknown.
    pub fn canonical_player_pos(&self) -> PlayerPos {
        if self.is_solved() {
            return PlayerPos::Unknown;
        }

        match self.player {
            PlayerPos::Known(x, y) => {
                let mut reachable = LazyBitboard::new();
                let (cx, cy) = self.player_dfs((x, y), &mut reachable, |_pos, _dir, _box_idx| {});
                PlayerPos::Known(cx, cy)
            }
            PlayerPos::Unknown => PlayerPos::Unknown,
        }
    }

    /// Compute all possible box pushes from the current game state.
    /// Uses a single DFS from player position to find all reachable boxes.
    /// Returns the pushes and the canonicalized (lexicographically smallest) player position.
    /// If the game is already solved, returns empty pushes and Unknown player position.
    /// Panics if the player position is Unknown (and game is not solved).
    pub fn compute_pushes(&self) -> (Moves, PlayerPos) {
        // Short-circuit if already solved
        if self.is_solved() {
            return (Moves::new(), PlayerPos::Unknown);
        }

        let PlayerPos::Known(x, y) = self.player else {
            panic!("Cannot compute pushes when player position is unknown");
        };

        let mut pushes = Moves::new();
        let mut reachable = LazyBitboard::new();
        let canonical_pos = self.player_dfs((x, y), &mut reachable, |_player_pos, dir, box_idx| {
            // For pushes: check if the box can move forward in the direction
            let box_pos = self.box_pos(box_idx as usize);
            if let Some((dest_x, dest_y)) = self.push_pos(box_pos.0, box_pos.1, dir) {
                let dest_tile = self.get_tile(dest_x, dest_y);
                if !self.boxes.has_box_at(dest_x, dest_y)
                    && (dest_tile == Tile::Floor || dest_tile == Tile::Goal)
                {
                    pushes.add(box_idx, dir);
                }
            }
        });
        (pushes, PlayerPos::Known(canonical_pos.0, canonical_pos.1))
    }

    /// Compute all possible pulls from the current game state.
    /// Returns the pulls and the canonicalized (lexicographically smallest) player position.
    /// If player position is Unknown, computes pulls from all possible player positions
    /// and returns Unknown as the canonical position.
    pub fn compute_pulls(&self) -> (Moves, PlayerPos) {
        let mut pulls = Moves::new();
        let mut reachable = LazyBitboard::new();

        match self.player {
            PlayerPos::Known(x, y) => {
                let canonical_pos = self.compute_pulls_helper((x, y), &mut reachable, &mut pulls);
                (pulls, PlayerPos::Known(canonical_pos.0, canonical_pos.1))
            }
            PlayerPos::Unknown => {
                assert!(self.is_solved());
                // Try each position as a potential player position
                for y in 0..self.height {
                    for x in 0..self.width {
                        // Skip if already explored from a previous position
                        if reachable.get(x, y) {
                            continue;
                        }

                        let tile = self.get_tile(x, y);
                        if (tile == Tile::Floor || tile == Tile::Goal)
                            && !self.boxes.has_box_at(x, y)
                        {
                            self.compute_pulls_helper((x, y), &mut reachable, &mut pulls);
                        }
                    }
                }
                (pulls, PlayerPos::Unknown)
            }
        }
    }

    fn compute_pulls_helper(
        &self,
        player: (u8, u8),
        reachable: &mut LazyBitboard,
        pulls: &mut Moves,
    ) -> (u8, u8) {
        self.player_dfs(player, reachable, |(x, y), dir, box_idx| {
            // For pull: box at (nx, ny), player at (x, y)
            // Box moves to (x, y), player moves to (x, y) - dir
            // Check if player destination is free
            if let Some((dest_x, dest_y)) = self.pull_pos(x, y, dir) {
                let dest_tile = self.get_tile(dest_x, dest_y);
                if !self.boxes.has_box_at(dest_x, dest_y)
                    && (dest_tile == Tile::Floor || dest_tile == Tile::Goal)
                {
                    pulls.add(box_idx, dir);
                }
            }
        })
    }

    /// Generic DFS helper to find all reachable player positions.
    /// Calls the `on_box` closure for each box adjacent to a reachable position.
    /// The closure receives (player_pos, direction, box_idx) and can handle box move logic.
    fn player_dfs<F>(
        &self,
        start_player: (u8, u8),
        reachable: &mut LazyBitboard,
        mut on_box: F,
    ) -> (u8, u8)
    where
        F: FnMut((u8, u8), Direction, u8),
    {
        let mut canonical_pos = start_player;

        // Stack-allocated stack for DFS using ArrayVec
        let mut stack: ArrayVec<(u8, u8), { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();

        // DFS from player position to find all reachable positions
        stack.push(start_player);
        reachable.set(start_player.0, start_player.1);

        while let Some((x, y)) = stack.pop() {
            // Check all 4 directions
            for &dir in &ALL_DIRECTIONS {
                if let Some((nx, ny)) = self.push_pos(x, y, dir) {
                    // If there's a box, notify the closure
                    if let Some(box_idx) = self.box_at(nx, ny) {
                        on_box((x, y), dir, box_idx);
                    } else {
                        let tile = self.get_tile(nx, ny);
                        if (tile == Tile::Floor || tile == Tile::Goal) && !reachable.get(nx, ny) {
                            // Continue DFS to this floor/goal tile
                            reachable.set(nx, ny);

                            // Update canonical position if this is lexicographically smaller
                            if (nx, ny) < canonical_pos {
                                canonical_pos = (nx, ny);
                            }

                            stack.push((nx, ny));
                        }
                    }
                }
            }
        }

        canonical_pos
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                let tile = self.tiles[y as usize][x as usize];
                let has_box = self.boxes.has_box_at(x, y);

                let is_player =
                    matches!(self.player, PlayerPos::Known(px, py) if (x, y) == (px, py));

                let ch = if is_player {
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
        assert_eq!(game.player, PlayerPos::Known(2, 3));
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
        assert_eq!(game.player, PlayerPos::Known(2, 1));
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
        game.push(Move {
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
        assert_eq!(game.player, PlayerPos::Known(2, 1));
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
        game.push(Move {
            box_index: box_idx,
            direction: Direction::Right,
        });
        assert_eq!(game.player, PlayerPos::Known(2, 1));
        assert!(game.boxes.has_box_at(3, 1));

        // Test pushing down
        let input = "#####\n\
                     # @ #\n\
                     # $ #\n\
                     # . #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.boxes.index[2][2];
        game.push(Move {
            box_index: box_idx,
            direction: Direction::Down,
        });
        assert_eq!(game.player, PlayerPos::Known(2, 2));
        assert_eq!(game.get_tile(2, 3), Tile::Goal);
        assert!(game.boxes.has_box_at(2, 3));

        // Test pushing left
        let input = "####\n\
                     # $@#\n\
                     # . #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.boxes.index[1][2];
        game.push(Move {
            box_index: box_idx,
            direction: Direction::Left,
        });
        assert_eq!(game.player, PlayerPos::Known(2, 1));
        assert!(game.boxes.has_box_at(1, 1));

        // Test pushing up
        let input = "#####\n\
                     # . #\n\
                     # $ #\n\
                     # @ #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.boxes.index[2][2];
        game.push(Move {
            box_index: box_idx,
            direction: Direction::Up,
        });
        assert_eq!(game.player, PlayerPos::Known(2, 2));
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
        game.push(Move {
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
        game.push(Move {
            box_index: box_idx,
            direction: Direction::Right,
        });

        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(!game.boxes.has_box_at(2, 1));
        assert_eq!(game.get_tile(3, 1), Tile::Floor);
        assert!(game.boxes.has_box_at(3, 1));
        assert_eq!(game.empty_goals, 1);
        assert_eq!(game.player, PlayerPos::Known(2, 1));
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
        game.push(Move {
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
        game.push(Move {
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
        game.push(Move {
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
        game.push(Move {
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
            Move {
                box_index: 0,
                direction: Direction::Up,
            },
            Move {
                box_index: 0,
                direction: Direction::Down,
            },
            Move {
                box_index: 1,
                direction: Direction::Left,
            },
            Move {
                box_index: 1,
                direction: Direction::Right,
            },
        ];

        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);

        // Check canonical position - should be lexicographically smallest reachable position
        // Player starts at (2, 3) and can reach many positions including (1, 1)
        assert_eq!(canonical_pos, PlayerPos::Known(1, 1));
    }

    #[test]
    fn test_compute_pulls() {
        // Test with a box that could have been pushed from the left
        let input = "#####\n\
                     # $+ #\n\
                     #####";
        let game = Game::from_text(input).unwrap();
        let (pulls, canonical_pos) = game.compute_pulls();
        let actual = pulls.iter().collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![Move {
                box_index: 0,
                direction: Direction::Left
            }]
        );
        assert_eq!(canonical_pos, PlayerPos::Known(3, 1));
    }

    #[test]
    fn test_compute_initial_pulls() {
        let input = "#######\n\
                     #@ *  #\n\
                     #######";
        let game = Game::from_text(input).unwrap().make_goal_state();
        let (pulls, canonical_pos) = game.compute_pulls();
        let mut actual = pulls.iter().collect::<Vec<_>>();
        actual.sort();

        let mut expected = vec![
            Move {
                box_index: 0,
                direction: Direction::Left,
            },
            Move {
                box_index: 0,
                direction: Direction::Right,
            },
        ];
        expected.sort();
        assert_eq!(canonical_pos, PlayerPos::Unknown);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_pull() {
        // Test pull restores original state
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
        let push = Move {
            box_index: box_idx,
            direction: Direction::Right,
        };
        game.push(push);

        // Verify state changed
        assert_eq!(game.player, PlayerPos::Known(2, 1));
        assert!(game.boxes.has_box_at(3, 1));
        assert_eq!(game.empty_goals, 0);
        assert!(game.is_solved());

        // Pull
        game.pull(push);

        // Should be back to original state
        assert_eq!(game.player, original_player);
        assert_eq!(game.boxes, original_boxes);
        assert_eq!(game.empty_goals, original_goals);
        assert!(!game.is_solved());
    }

    #[test]
    fn test_pull_all_directions() {
        // Test pull in all directions
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

            let push = Move {
                box_index: box_idx,
                direction,
            };

            game.push(push);
            game.pull(push);

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
