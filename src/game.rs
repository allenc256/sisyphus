use crate::bits::{Bitboard, Bitvector, BitvectorIter, LazyBitboard, RawBitboard};
pub use crate::bits::{Index, Position};
use arrayvec::ArrayVec;
use std::{fmt, marker::PhantomData};

pub const MAX_SIZE: usize = 64;
pub const MAX_BOXES: usize = 64;
pub const NO_BOX: Index = Index(255);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Goal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub fn reverse(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

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

pub trait Move: fmt::Display {
    fn new(box_index: Index, direction: Direction) -> Self;
    fn box_index(&self) -> Index;
    fn direction(&self) -> Direction;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Push {
    box_index: Index,
    direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pull {
    box_index: Index,
    direction: Direction,
}

impl Push {
    pub fn new(box_index: Index, direction: Direction) -> Self {
        Self {
            box_index,
            direction,
        }
    }

    pub fn to_pull(self) -> Pull {
        Pull {
            box_index: self.box_index,
            direction: self.direction.reverse(),
        }
    }
}

impl Pull {
    pub fn new(box_index: Index, direction: Direction) -> Self {
        Self {
            box_index,
            direction,
        }
    }

    pub fn to_push(self) -> Push {
        Push {
            box_index: self.box_index,
            direction: self.direction.reverse(),
        }
    }
}

impl Move for Push {
    fn new(box_index: Index, direction: Direction) -> Self {
        Self {
            box_index,
            direction,
        }
    }

    fn box_index(&self) -> Index {
        self.box_index
    }

    fn direction(&self) -> Direction {
        self.direction
    }
}

impl Move for Pull {
    fn new(box_index: Index, direction: Direction) -> Self {
        Self {
            box_index,
            direction,
        }
    }

    fn box_index(&self) -> Index {
        self.box_index
    }

    fn direction(&self) -> Direction {
        self.direction
    }
}

impl fmt::Display for Push {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Push #{} {}", self.box_index.0 + 1, self.direction)
    }
}

impl fmt::Display for Pull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pull #{} {}", self.box_index.0 + 1, self.direction)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moves<T> {
    // Bitset: bits[0] = Up, bits[1] = Down, bits[2] = Left, bits[3] = Right
    // Each Bitvector holds 64 bits for 64 boxes (box indices 0-63)
    bits: [Bitvector; 4],
    phantom: PhantomData<T>,
}

impl<T: Move> Moves<T> {
    pub fn new() -> Self {
        Moves {
            bits: [Bitvector::new(); 4],
            phantom: PhantomData,
        }
    }

    pub fn add(&mut self, box_index: Index, direction: Direction) {
        let dir_idx = direction.index();
        self.bits[dir_idx].add(box_index);
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

    pub fn contains(&self, move_: T) -> bool {
        let dir_idx = move_.direction().index();
        self.bits[dir_idx].contains(move_.box_index())
    }

    pub fn remove(&mut self, move_: T) {
        let dir_idx = move_.direction().index();
        self.bits[dir_idx].remove(move_.box_index());
    }

    pub fn iter(&self) -> MovesIter<T> {
        MovesIter {
            bits: self.bits,
            dir_idx: 0,
            current_iter: self.bits[0].iter(),
            phantom: PhantomData,
        }
    }
}

impl<T: Move> Default for Moves<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl Moves<Push> {
    pub fn to_pulls(self) -> Moves<Pull> {
        Moves {
            bits: swizzle_bits(self.bits),
            phantom: PhantomData,
        }
    }
}

impl Moves<Pull> {
    pub fn to_pushes(self) -> Moves<Push> {
        Moves {
            bits: swizzle_bits(self.bits),
            phantom: PhantomData,
        }
    }
}

fn swizzle_bits(bits: [Bitvector; 4]) -> [Bitvector; 4] {
    [bits[1], bits[0], bits[3], bits[2]]
}

pub struct MovesIter<T> {
    bits: [Bitvector; 4],
    dir_idx: usize,
    current_iter: BitvectorIter,
    phantom: PhantomData<T>,
}

impl<T: Move> Iterator for MovesIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(box_index) = self.current_iter.next() {
                let direction = Direction::from_index(self.dir_idx);
                return Some(T::new(box_index, direction));
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

impl<T: Move> IntoIterator for &'_ Moves<T> {
    type Item = T;
    type IntoIter = MovesIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct ReachableSet<T> {
    /// Moves the player can currently make
    pub moves: Moves<T>,
    /// Open squares the player can currently reach
    pub squares: LazyBitboard,
    /// Boxes the player can currently reach
    pub boxes: Bitvector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Boxes {
    positions: ArrayVec<Position, MAX_BOXES>,
    // Maps board position to box index (NO_BOX = no box at this position)
    index: [[Index; MAX_SIZE]; MAX_SIZE],
    // Boxes that are not on goal positions
    unsolved: Bitvector,
}

impl Boxes {
    fn new() -> Self {
        Boxes {
            positions: ArrayVec::new(),
            index: [[NO_BOX; MAX_SIZE]; MAX_SIZE],
            unsolved: Bitvector::new(),
        }
    }

    fn add(&mut self, pos: Position, is_goal: bool) -> Index {
        let index = Index(self.positions.len() as u8);
        self.index[pos.1 as usize][pos.0 as usize] = index;
        self.positions.push(pos);
        if !is_goal {
            self.unsolved.add(index);
        }
        index
    }

    fn move_(&mut self, from: Position, to: Position, from_is_goal: bool, to_is_goal: bool) {
        let idx = self.index[from.1 as usize][from.0 as usize];
        self.positions[idx.0 as usize] = to;
        self.index[from.1 as usize][from.0 as usize] = NO_BOX;
        self.index[to.1 as usize][to.0 as usize] = idx;

        // Update unsolved boxes
        if from_is_goal {
            self.unsolved.add(idx);
        }
        if to_is_goal {
            self.unsolved.remove(idx);
        }
    }

    fn has_box_at(&self, pos: Position) -> bool {
        self.index[pos.1 as usize][pos.0 as usize] != NO_BOX
    }

    fn clear(&mut self) {
        for pos in &self.positions {
            self.index[pos.1 as usize][pos.1 as usize] = NO_BOX;
        }
        self.positions.clear();
        self.unsolved = Bitvector::new();
    }
}

pub struct Checkpoint {
    player: Position,
    boxes: ArrayVec<Position, MAX_BOXES>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    tiles: [[Tile; MAX_SIZE]; MAX_SIZE],
    player: Position,
    width: u8,
    height: u8,
    boxes: Boxes,
    goal_positions: ArrayVec<Position, MAX_BOXES>,
    push_dead_squares: RawBitboard,
    pull_dead_squares: RawBitboard,
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
        let mut player = None;
        let mut boxes = Boxes::new();
        let mut goal_positions = ArrayVec::new();

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                match ch {
                    '#' => tiles[y][x] = Tile::Wall,
                    ' ' => tiles[y][x] = Tile::Floor,
                    '.' => {
                        tiles[y][x] = Tile::Goal;
                        goal_positions.push(Position(x as u8, y as u8));
                    }
                    '$' => {
                        tiles[y][x] = Tile::Floor;
                        boxes.add(Position(x as u8, y as u8), false);
                    }
                    '*' => {
                        tiles[y][x] = Tile::Goal;
                        goal_positions.push(Position(x as u8, y as u8));
                        boxes.add(Position(x as u8, y as u8), true);
                    }
                    '@' => {
                        tiles[y][x] = Tile::Floor;
                        if player.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player = Some(Position(x as u8, y as u8));
                    }
                    '+' => {
                        tiles[y][x] = Tile::Goal;
                        if player.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player = Some(Position(x as u8, y as u8));
                        goal_positions.push(Position(x as u8, y as u8));
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

        let Some(player) = player else {
            return Err("No player found on board".to_owned());
        };

        // Validate that the number of goals matches the number of boxes
        if goal_positions.len() != boxes.positions.len() {
            return Err(format!(
                "Goal count ({}) does not match box count ({})",
                goal_positions.len(),
                boxes.positions.len()
            ));
        }

        let mut game = Game {
            tiles,
            player,
            width: width as u8,
            height: height as u8,
            boxes,
            goal_positions,
            push_dead_squares: RawBitboard::new(),
            pull_dead_squares: RawBitboard::new(),
        };
        game.compute_dead_squares();
        Ok(game)
    }

    /// Compute all dead squares where a box can never reach any goal.
    fn compute_dead_squares(&mut self) {
        let mut push_reachable = RawBitboard::new();
        let mut pull_reachable = RawBitboard::new();

        // For each goal, find all squares that can reach it via reverse pushes
        for &goal_pos in &self.goal_positions {
            self.dfs_push_reachable(goal_pos, &mut push_reachable);
            self.dfs_pull_reachable(goal_pos, &mut pull_reachable);
        }

        self.push_dead_squares = push_reachable.invert();
        self.pull_dead_squares = pull_reachable.invert();
    }

    /// Generic DFS helper that explores positions starting from a given position.
    /// The `should_visit` closure receives (from_pos, to_pos, direction) and
    /// returns true if to_pos should be added to the stack and marked as visited.
    fn dfs<B: Bitboard>(
        &self,
        start_pos: Position,
        visited: &mut B,
        mut should_visit: impl FnMut(Position, Position, Direction) -> bool,
    ) {
        assert!(
            self.get_tile(start_pos) != Tile::Wall,
            "start position cannot be a wall"
        );

        let mut stack: ArrayVec<Position, { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();

        visited.set(start_pos);
        stack.push(start_pos);

        while let Some(from_pos) = stack.pop() {
            for direction in ALL_DIRECTIONS {
                if let Some(to_pos) = self.move_position(from_pos, direction) {
                    if self.get_tile(to_pos) != Tile::Wall
                        && !visited.get(to_pos)
                        && should_visit(from_pos, to_pos, direction)
                    {
                        visited.set(to_pos);
                        stack.push(to_pos);
                    }
                }
            }
        }
    }

    /// DFS to find all squares from which a box could be pushed to reach the given position.
    /// Uses reverse pushes (pulls).
    fn dfs_push_reachable(&self, start_pos: Position, reachable: &mut RawBitboard) {
        if reachable.get(start_pos) {
            return;
        }

        self.dfs(start_pos, reachable, |_from_pos, to_pos, direction| {
            // Check that there is room for the player
            if let Some(player_pos) = self.move_position(to_pos, direction) {
                self.get_tile(player_pos) != Tile::Wall
            } else {
                false
            }
        });
    }

    /// DFS to find all squares from which a box could be pulled to reach the given position.
    /// Uses forward pushes.
    fn dfs_pull_reachable(&self, start_pos: Position, reachable: &mut RawBitboard) {
        if reachable.get(start_pos) {
            return;
        }

        self.dfs(start_pos, reachable, |from_pos, _to_pos, direction| {
            // Check that there is room for the player
            if let Some(player_pos) = self.move_position(from_pos, direction.reverse()) {
                self.get_tile(player_pos) != Tile::Wall
            } else {
                false
            }
        });
    }

    pub fn get_tile(&self, pos: Position) -> Tile {
        self.tiles[pos.1 as usize][pos.0 as usize]
    }

    pub fn box_count(&self) -> usize {
        self.boxes.positions.len()
    }

    pub fn set_player(&mut self, pos: Position) {
        self.player = pos;
    }

    #[allow(dead_code)]
    pub fn player(&self) -> Position {
        self.player
    }

    pub fn box_positions(&self) -> &[Position] {
        &self.boxes.positions
    }

    pub fn goal_positions(&self) -> &[Position] {
        &self.goal_positions
    }

    pub fn unsolved_boxes(&self) -> Bitvector {
        self.boxes.unsolved
    }

    pub fn is_push_dead_square(&self, pos: Position) -> bool {
        self.push_dead_squares.get(pos)
    }

    pub fn is_pull_dead_square(&self, pos: Position) -> bool {
        self.pull_dead_squares.get(pos)
    }

    /// Get the box index at the given position, if any.
    /// Returns Some(box_index) if there is a box at the position, None otherwise.
    pub fn box_index(&self, pos: Position) -> Option<Index> {
        let idx = self.boxes.index[pos.1 as usize][pos.0 as usize];
        if idx == NO_BOX { None } else { Some(idx) }
    }

    /// Get the position of a box given its index.
    pub fn box_position(&self, box_index: Index) -> Position {
        self.boxes.positions[box_index.0 as usize]
    }

    /// Move from position in the given direction.
    /// Returns Some(new_position) if the new position is within bounds, None otherwise.
    pub fn move_position(&self, pos: Position, dir: Direction) -> Option<Position> {
        let (dx, dy) = dir.delta();
        let new_x = pos.0 as i32 + dx as i32;
        let new_y = pos.1 as i32 + dy as i32;

        if new_x >= 0 && new_y >= 0 && new_x < self.width as i32 && new_y < self.height as i32 {
            Some(Position(new_x as u8, new_y as u8))
        } else {
            None
        }
    }

    /// Pushes a box.
    /// Updates the player position to where the box was.
    /// Panics if the push is invalid (invalid box index, destination blocked, etc.)
    pub fn push(&mut self, push: Push) {
        let box_pos = self.box_position(push.box_index);
        let new_pos = self
            .move_position(box_pos, push.direction)
            .expect("Push destination out of bounds");

        let dest_tile = self.get_tile(new_pos);
        assert!(
            !self.boxes.has_box_at(new_pos)
                && (dest_tile == Tile::Floor || dest_tile == Tile::Goal),
            "Cannot push box to {}: destination blocked",
            new_pos
        );

        let source_tile = self.get_tile(box_pos);
        let source_is_goal = source_tile == Tile::Goal;
        let dest_is_goal = dest_tile == Tile::Goal;

        // Update box position
        self.boxes
            .move_(box_pos, new_pos, source_is_goal, dest_is_goal);

        // Update player position to where the box was
        self.player = box_pos;
    }

    pub fn pull(&mut self, pull: Pull) {
        // Current box position (after the push we're undoing)
        let new_pos = self.box_position(pull.box_index);

        // Calculate where box came from (opposite direction)
        let old_pos = self
            .move_position(new_pos, pull.direction)
            .expect("Pull source out of bounds");

        // Calculate where player was before the push
        let player_old_pos = self
            .move_position(old_pos, pull.direction)
            .expect("Pull player position out of bounds");

        let current_tile = self.get_tile(new_pos);
        let current_is_goal = current_tile == Tile::Goal;
        let old_tile = self.get_tile(old_pos);
        let old_is_goal = old_tile == Tile::Goal;

        // Move box back
        self.boxes
            .move_(new_pos, old_pos, current_is_goal, old_is_goal);

        // Restore player position
        self.player = player_old_pos;
    }

    /// Check if all boxes are on goals (win condition)
    pub fn is_solved(&self) -> bool {
        self.boxes.unsolved.is_empty()
    }

    /// Create a new game state with boxes and goals swapped.
    /// Boxes are placed at goal positions, and goals become where boxes originally were.
    /// This is useful for backward search.
    pub fn swap_boxes_and_goals(&self) -> Self {
        // Build new boxes with positions at goal locations
        let mut boxes = Boxes::new();
        let new_goal_positions = self.boxes.positions.clone();

        for &goal_pos in &self.goal_positions {
            // Box is on goal if it's on one of the new goals (original box positions)
            let is_goal = new_goal_positions.contains(&goal_pos);
            boxes.add(goal_pos, is_goal);
        }

        // Update tiles: old goals become floor, old box positions become goals
        let mut tiles = self.tiles;
        for &old_goal in &self.goal_positions {
            tiles[old_goal.1 as usize][old_goal.0 as usize] = Tile::Floor;
        }
        for &new_goal in &new_goal_positions {
            tiles[new_goal.1 as usize][new_goal.0 as usize] = Tile::Goal;
        }

        let mut game = Game {
            tiles,
            boxes,
            goal_positions: new_goal_positions,
            push_dead_squares: RawBitboard::new(),
            pull_dead_squares: RawBitboard::new(),
            ..self.clone()
        };
        game.compute_dead_squares();
        game
    }

    /// Compute the canonical (lexicographically smallest reachable) player position.
    pub fn canonical_player_pos(&self) -> Position {
        let mut visited = LazyBitboard::new();
        self.player_dfs(self.player, &mut visited, |_pos, _dir, _box_idx| {});
        visited.top_left().unwrap()
    }

    pub fn compute_pushes(&self) -> ReachableSet<Push> {
        let mut moves = Moves::new();
        let mut visited = LazyBitboard::new();
        let mut boxes = Bitvector::new();
        self.player_dfs(self.player, &mut visited, |_player_pos, dir, box_idx| {
            boxes.add(box_idx);
            let box_pos = self.box_position(box_idx);
            if let Some(dest_pos) = self.move_position(box_pos, dir) {
                if !self.is_blocked(dest_pos) {
                    moves.add(box_idx, dir);
                }
            }
        });
        ReachableSet {
            moves,
            squares: visited,
            boxes,
        }
    }

    fn is_blocked(&self, pos: Position) -> bool {
        self.get_tile(pos) == Tile::Wall || self.boxes.has_box_at(pos)
    }

    pub fn compute_pulls(&self) -> ReachableSet<Pull> {
        let mut moves = Moves::new();
        let mut visited = LazyBitboard::new();
        let mut boxes = Bitvector::new();
        self.player_dfs(self.player, &mut visited, |player_pos, dir, box_idx| {
            boxes.add(box_idx);
            if let Some(dest_pos) = self.move_position(player_pos, dir.reverse()) {
                if !self.is_blocked(dest_pos) {
                    moves.add(box_idx, dir.reverse());
                }
            }
        });
        ReachableSet {
            moves,
            squares: visited,
            boxes,
        }
    }

    /// Compute all possible canonical player positions (assuming the player's real position is unknown).
    /// Returns positions for which at least one box is reachable from that connected region.
    pub fn all_possible_player_positions(&self) -> Vec<Position> {
        let mut all_visited = LazyBitboard::new();
        let mut result: Vec<Position> = Vec::new();

        for y in 0..self.height {
            for x in 0..self.width {
                let mut local_visited = LazyBitboard::new();
                let pos = Position(x, y);

                // Skip if already explored or blocked
                if all_visited.get(pos) || self.is_blocked(pos) {
                    continue;
                }

                let mut found_box = false;
                self.player_dfs(pos, &mut local_visited, |_pos, _dir, _box_idx| {
                    found_box = true
                });
                if found_box {
                    result.push(local_visited.top_left().unwrap());
                }
                all_visited.set_all(&local_visited);
            }
        }

        result
    }

    /// Generic DFS helper to find all reachable player positions.
    /// Calls the `on_box` closure for each box adjacent to a reachable position.
    /// The closure receives (player_pos, direction, box_idx) and can handle box move logic.
    fn player_dfs<F>(&self, start_player: Position, visited: &mut LazyBitboard, mut on_box: F)
    where
        F: FnMut(Position, Direction, Index),
    {
        self.dfs(start_player, visited, |from_pos, to_pos, direction| {
            // If there's a box, notify the closure but don't visit
            if let Some(box_idx) = self.box_index(to_pos) {
                on_box(from_pos, direction, box_idx);
                false
            } else {
                true
            }
        });
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            player: self.player,
            boxes: self.boxes.positions.clone(),
        }
    }

    pub fn restore(&mut self, checkpoint: &Checkpoint) {
        self.player = checkpoint.player;
        self.boxes.clear();
        for &pos in &checkpoint.boxes {
            self.boxes.add(pos, self.get_tile(pos) == Tile::Goal);
        }
    }

    /// Project the game down to a subset of boxes in-place.
    /// Updates the game to only contain the boxes specified in the input bitvector.
    /// Box indexes may be renumbered after projection.
    pub fn project(&mut self, boxes_to_keep: Bitvector) {
        let mut new_boxes = Boxes::new();

        // Iterate through boxes to keep and add them to the new game
        for box_idx in boxes_to_keep {
            let pos = self.boxes.positions[box_idx.0 as usize];
            let is_goal = self.get_tile(pos) == Tile::Goal;
            new_boxes.add(pos, is_goal);
        }

        self.boxes = new_boxes;
    }
}

impl AsRef<Game> for Game {
    fn as_ref(&self) -> &Game {
        self
    }
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                let pos = Position(x, y);
                let tile = self.tiles[y as usize][x as usize];
                let has_box = self.boxes.has_box_at(pos);
                let is_player = pos == self.player;

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
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_parse_basic_board() {
        let game = parse_game(
            r#"
####
# .#
#  ###
#*@  #
#  $ #
#  ###
####
"#,
        )
        .unwrap();

        assert_eq!(game.width, 6);
        assert_eq!(game.height, 7);
        assert_eq!(game.player, Position(2, 3));
    }

    #[test]
    fn test_no_player() {
        let result = parse_game(
            r#"
####
#  #
####
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_players() {
        let result = parse_game(
            r#"
####
#@@#
####
"#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_player_on_goal() {
        let game = parse_game(
            r#"
#####
#$+ #
#$. #
#####
"#,
        )
        .unwrap();
        assert_eq!(game.player, Position(2, 1));
        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
    }

    #[test]
    fn test_display() {
        let input = r#"
####
# .#
#  ###
#*@  #
#  $ #
#  ###
####
"#;
        let game = parse_game(input).unwrap();
        let output = game.to_string();
        assert_eq!(output.trim(), input.trim_matches('\n'));
    }

    #[test]
    fn test_is_solved() {
        let solved = parse_game(
            r#"
####
#*@#
####
"#,
        )
        .unwrap();
        assert!(solved.is_solved());

        let unsolved = parse_game(
            r#"
####
#$.#
# @#
####
"#,
        )
        .unwrap();
        assert!(!unsolved.is_solved());
    }

    #[test]
    fn test_empty_goals_tarcking() {
        // Board with 1 box on goal, 1 box not on goal
        let game = parse_game(
            r#"
####
# .#
#  ###
#*@  #
#  $ #
#  ###
####
"#,
        )
        .unwrap();
        assert_eq!(game.boxes.unsolved.len(), 1);
        assert!(!game.is_solved());

        // Board with all boxes on goals
        let all_solved = parse_game(
            r#"
####
#*@#
####
"#,
        )
        .unwrap();
        assert_eq!(all_solved.boxes.unsolved.len(), 0);
        assert!(all_solved.is_solved());

        // Board with no boxes on goals
        let none_solved = parse_game(
            r#"
####
#$.#
# @#
####
"#,
        )
        .unwrap();
        assert_eq!(none_solved.boxes.unsolved.len(), 1);
        assert!(!none_solved.is_solved());
    }

    #[test]
    fn test_goal_box_count_validation() {
        // More goals than boxes - should fail
        let more_goals = parse_game(
            r#"
####
#..##
# $@#
#####
"#,
        );
        assert!(more_goals.is_err());

        // More boxes than goals - should fail
        let more_boxes = parse_game(
            r#"
####
#$$##
# .@#
#####
"#,
        );
        assert!(more_boxes.is_err());

        // Equal goals and boxes - should succeed
        let balanced = parse_game(
            r#"
####
#$.##
# * #
# @##
####
"#,
        );
        assert!(balanced.is_ok());
    }

    #[test]
    fn test_push_basic() {
        // Simple board: player can push box right onto goal
        let mut game = parse_game(
            r#"
#####
#@$.#
#####
"#,
        )
        .unwrap();

        // Push box right (box at position (2,1) is box index 0)
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        // Box should now be on goal at (3, 1)
        assert_eq!(game.get_tile(Position(3, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(3, 1)));
        // Original box position should be floor
        assert_eq!(game.get_tile(Position(2, 1)), Tile::Floor);
        assert!(!game.boxes.has_box_at(Position(2, 1)));
        // Player should be at old box position
        assert_eq!(game.player, Position(2, 1));
        // Should be solved
        assert!(game.is_solved());
        assert_eq!(game.boxes.unsolved.len(), 0);
    }

    #[test]
    fn test_push_all_directions() {
        // Test pushing right
        let mut game = parse_game(
            r#"
#####
#@$ #
# . #
#####
"#,
        )
        .unwrap();
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });
        assert_eq!(game.player, Position(2, 1));
        assert!(game.boxes.has_box_at(Position(3, 1)));

        // Test pushing down
        let mut game = parse_game(
            r#"
#####
# @ #
# $ #
# . #
#####
"#,
        )
        .unwrap();
        let box_idx = game.boxes.index[2][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Down,
        });
        assert_eq!(game.player, Position(2, 2));
        assert_eq!(game.get_tile(Position(2, 3)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(2, 3)));

        // Test pushing left
        let mut game = parse_game(
            r#"
#####
# $@#
# . #
#####
"#,
        )
        .unwrap();
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Left,
        });
        assert_eq!(game.player, Position(2, 1));
        assert!(game.boxes.has_box_at(Position(1, 1)));

        // Test pushing up
        let mut game = parse_game(
            r#"
#####
# . #
# $ #
# @ #
#####
"#,
        )
        .unwrap();
        let box_idx = game.boxes.index[2][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Up,
        });
        assert_eq!(game.player, Position(2, 2));
        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(2, 1)));
    }

    #[test]
    fn test_push_floor_to_goal() {
        // Push box from floor onto goal
        let mut game = parse_game(
            r#"
#####
#@$.#
#####
"#,
        )
        .unwrap();

        assert_eq!(game.boxes.unsolved.len(), 1);
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        assert_eq!(game.get_tile(Position(3, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(3, 1)));
        assert_eq!(game.boxes.unsolved.len(), 0);
    }

    #[test]
    fn test_push_goal_to_floor() {
        // Push box from goal onto floor
        let mut game = parse_game(
            r#"
######
#@*  #
######
"#,
        )
        .unwrap();

        assert_eq!(game.boxes.unsolved.len(), 0);
        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(2, 1)));

        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
        assert!(!game.boxes.has_box_at(Position(2, 1)));
        assert_eq!(game.get_tile(Position(3, 1)), Tile::Floor);
        assert!(game.boxes.has_box_at(Position(3, 1)));
        assert_eq!(game.boxes.unsolved.len(), 1);
        assert_eq!(game.player, Position(2, 1));
    }

    #[test]
    fn test_push_goal_to_goal() {
        // Push box from one goal to another goal
        let mut game = parse_game(
            r#"
######
#@*.$#
######
"#,
        )
        .unwrap();

        assert_eq!(game.boxes.unsolved.len(), 1);
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });

        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
        assert!(!game.boxes.has_box_at(Position(2, 1)));
        assert_eq!(game.get_tile(Position(3, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(3, 1)));
        assert_eq!(game.boxes.unsolved.len(), 1);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_push_no_box() {
        let mut game = parse_game(
            r#"
#####
#@  #
#####
"#,
        )
        .unwrap();

        // Try to push with invalid box index
        game.push(Push {
            box_index: Index(10),
            direction: Direction::Right,
        });
    }

    #[test]
    #[should_panic(expected = "destination blocked")]
    fn test_push_blocked() {
        let mut game = parse_game(
            r#"
####
#@$##
# . #
#####
"#,
        )
        .unwrap();

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
        let mut game = parse_game(
            r#"
#######
#@$$  #
# ..  #
#######
"#,
        )
        .unwrap();

        // Try to push box into another box
        let box_idx = game.boxes.index[1][2];
        game.push(Push {
            box_index: box_idx,
            direction: Direction::Right,
        });
    }

    #[test]
    fn test_compute_pushes() {
        let game = parse_game(
            r#"
####
# .#
#  ###
#*@  #
#  $ #
#  ###
####
"#,
        )
        .unwrap();
        let reachable = game.compute_pushes();
        let actual = reachable.moves.iter().collect::<HashSet<_>>();
        let expected = HashSet::from([
            Push {
                box_index: Index(0),
                direction: Direction::Up,
            },
            Push {
                box_index: Index(0),
                direction: Direction::Down,
            },
            Push {
                box_index: Index(1),
                direction: Direction::Left,
            },
            Push {
                box_index: Index(1),
                direction: Direction::Right,
            },
        ]);
        assert_eq!(expected, actual);

        // Check canonical position - should be lexicographically smallest reachable position
        // Player starts at (2, 3) and can reach many positions including (1, 1)
        assert_eq!(reachable.squares.top_left(), Some(Position(1, 1)));
    }

    #[test]
    fn test_compute_pulls() {
        // Test with a box that could have been pushed from the left
        let game = parse_game(
            r#"
######
# $+ #
######
"#,
        )
        .unwrap();
        let reachable = game.compute_pulls();
        let actual = reachable.moves.iter().collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![Pull {
                box_index: Index(0),
                direction: Direction::Right
            }]
        );
        assert_eq!(reachable.squares.top_left(), Some(Position(3, 1)));
    }

    #[test]
    fn test_pull() {
        // Test pull restores original state
        let mut game = parse_game(
            r#"
#####
#@$.#
#####
"#,
        )
        .unwrap();

        // Save original state
        let original_player = game.player;
        let original_boxes = game.boxes.clone();
        let original_goals = game.boxes.unsolved.len();

        // Push box right
        let box_idx = game.boxes.index[1][2];
        let push = Push {
            box_index: box_idx,
            direction: Direction::Right,
        };
        game.push(push);

        // Verify state changed
        assert_eq!(game.player, Position(2, 1));
        assert!(game.boxes.has_box_at(Position(3, 1)));
        assert_eq!(game.boxes.unsolved.len(), 0);
        assert!(game.is_solved());

        // Pull
        game.pull(push.to_pull());

        // Should be back to original state
        assert_eq!(game.player, original_player);
        assert_eq!(game.boxes, original_boxes);
        assert_eq!(game.boxes.unsolved.len(), original_goals);
        assert!(!game.is_solved());
    }

    #[test]
    fn test_pull_all_directions() {
        // Test pull in all directions

        // Test pushing right
        let mut game = parse_game(
            r#"
#####
#@$ #
# . #
#####
"#,
        )
        .unwrap();
        let original = game.clone();
        let box_idx = game.boxes.positions[0];
        let box_idx = game.boxes.index[box_idx.1 as usize][box_idx.0 as usize];
        let push = Push {
            box_index: box_idx,
            direction: Direction::Right,
        };
        game.push(push);
        game.pull(push.to_pull());
        assert_eq!(game.player, original.player);
        assert_eq!(game.boxes, original.boxes);
        assert_eq!(game.boxes.unsolved.len(), original.boxes.unsolved.len());

        // Test pushing down
        let mut game = parse_game(
            r#"
#####
# @ #
# $ #
# . #
#####
"#,
        )
        .unwrap();
        let original = game.clone();
        let box_idx = game.boxes.positions[0];
        let box_idx = game.boxes.index[box_idx.1 as usize][box_idx.0 as usize];
        let push = Push {
            box_index: box_idx,
            direction: Direction::Down,
        };
        game.push(push);
        game.pull(push.to_pull());
        assert_eq!(game.player, original.player);
        assert_eq!(game.boxes, original.boxes);
        assert_eq!(game.boxes.unsolved.len(), original.boxes.unsolved.len());

        // Test pushing left
        let mut game = parse_game(
            r#"
#####
# $@#
# . #
#####
"#,
        )
        .unwrap();
        let original = game.clone();
        let box_idx = game.boxes.positions[0];
        let box_idx = game.boxes.index[box_idx.1 as usize][box_idx.0 as usize];
        let push = Push {
            box_index: box_idx,
            direction: Direction::Left,
        };
        game.push(push);
        game.pull(push.to_pull());
        assert_eq!(game.player, original.player);
        assert_eq!(game.boxes, original.boxes);
        assert_eq!(game.boxes.unsolved.len(), original.boxes.unsolved.len());

        // Test pushing up
        let mut game = parse_game(
            r#"
#####
# . #
# $ #
# @ #
#####
"#,
        )
        .unwrap();
        let original = game.clone();
        let box_idx = game.boxes.positions[0];
        let box_idx = game.boxes.index[box_idx.1 as usize][box_idx.0 as usize];
        let push = Push {
            box_index: box_idx,
            direction: Direction::Up,
        };
        game.push(push);
        game.pull(push.to_pull());
        assert_eq!(game.player, original.player);
        assert_eq!(game.boxes, original.boxes);
        assert_eq!(game.boxes.unsolved.len(), original.boxes.unsolved.len());
    }

    fn parse_game(text: &str) -> Result<Game, String> {
        Game::from_text(text.trim_matches('\n'))
    }
}
