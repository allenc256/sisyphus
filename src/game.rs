use crate::bits::{Bitvector, BitvectorIter, LazyBitboard};
use arrayvec::ArrayVec;
use std::{fmt, marker::PhantomData};

pub const MAX_SIZE: usize = 64;
pub const MAX_BOXES: usize = 64;
pub const NO_BOX: BoxIndex = BoxIndex(255);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoxIndex(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position(pub u8, pub u8);

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Goal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPos {
    Known(Position),
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
    fn new(box_index: BoxIndex, direction: Direction) -> Self;
    fn box_index(&self) -> BoxIndex;
    fn direction(&self) -> Direction;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Push {
    box_index: BoxIndex,
    direction: Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pull {
    box_index: BoxIndex,
    direction: Direction,
}

impl Push {
    pub fn to_pull(&self) -> Pull {
        Pull {
            box_index: self.box_index,
            direction: self.direction.reverse(),
        }
    }
}

impl Pull {
    pub fn to_push(&self) -> Push {
        Push {
            box_index: self.box_index,
            direction: self.direction.reverse(),
        }
    }
}

impl Move for Push {
    fn new(box_index: BoxIndex, direction: Direction) -> Self {
        Self {
            box_index,
            direction,
        }
    }

    fn box_index(&self) -> BoxIndex {
        self.box_index
    }

    fn direction(&self) -> Direction {
        self.direction
    }
}

impl Move for Pull {
    fn new(box_index: BoxIndex, direction: Direction) -> Self {
        Self {
            box_index,
            direction,
        }
    }

    fn box_index(&self) -> BoxIndex {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moves<T> {
    // Bitset: bits[0] = Up, bits[1] = Down, bits[2] = Left, bits[3] = Right
    // Each Bitvector holds 64 bits for 64 boxes (box indices 0-63)
    bits: [Bitvector; 4],
    phantom: PhantomData<T>,
}

impl<T: Move> Moves<T> {
    fn new() -> Self {
        Moves {
            bits: [Bitvector::new(); 4],
            phantom: PhantomData,
        }
    }

    fn add(&mut self, box_index: BoxIndex, direction: Direction) {
        let dir_idx = direction.index();
        self.bits[dir_idx].add(box_index.0);
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
        self.bits[dir_idx].contains(move_.box_index().0)
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

impl Moves<Push> {
    pub fn to_pulls(&self) -> Moves<Pull> {
        Moves {
            bits: swizzle_bits(self.bits),
            phantom: PhantomData,
        }
    }
}

impl Moves<Pull> {
    pub fn to_pushes(&self) -> Moves<Push> {
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
                return Some(T::new(BoxIndex(box_index), direction));
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Boxes {
    positions: ArrayVec<Position, MAX_BOXES>,
    // Maps board position to box index (NO_BOX = no box at this position)
    index: [[BoxIndex; MAX_SIZE]; MAX_SIZE],
}

impl Boxes {
    fn new() -> Self {
        Boxes {
            positions: ArrayVec::new(),
            index: [[NO_BOX; MAX_SIZE]; MAX_SIZE],
        }
    }

    fn add(&mut self, pos: Position) {
        self.index[pos.1 as usize][pos.0 as usize] = BoxIndex(self.positions.len() as u8);
        self.positions.push(pos);
    }

    fn move_box(&mut self, from: Position, to: Position) {
        let idx = self.index[from.1 as usize][from.0 as usize];
        self.positions[idx.0 as usize] = to;
        self.index[from.1 as usize][from.0 as usize] = NO_BOX;
        self.index[to.1 as usize][to.0 as usize] = idx;
    }

    fn has_box_at(&self, pos: Position) -> bool {
        self.index[pos.1 as usize][pos.0 as usize] != NO_BOX
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
    start_positions: ArrayVec<Position, MAX_BOXES>,
    goal_positions: ArrayVec<Position, MAX_BOXES>,
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
        let mut start_positions = ArrayVec::new();
        let mut goal_positions = ArrayVec::new();
        let mut empty_goals: u8 = 0;

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                match ch {
                    '#' => tiles[y][x] = Tile::Wall,
                    ' ' => tiles[y][x] = Tile::Floor,
                    '.' => {
                        tiles[y][x] = Tile::Goal;
                        goal_positions.push(Position(x as u8, y as u8));
                        empty_goals += 1;
                    }
                    '$' => {
                        tiles[y][x] = Tile::Floor;
                        boxes.add(Position(x as u8, y as u8));
                        start_positions.push(Position(x as u8, y as u8));
                    }
                    '*' => {
                        tiles[y][x] = Tile::Goal;
                        goal_positions.push(Position(x as u8, y as u8));
                        start_positions.push(Position(x as u8, y as u8));
                        boxes.add(Position(x as u8, y as u8));
                    }
                    '@' => {
                        tiles[y][x] = Tile::Floor;
                        if player_pos.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player_pos = Some(PlayerPos::Known(Position(x as u8, y as u8)));
                    }
                    '+' => {
                        tiles[y][x] = Tile::Goal;
                        if player_pos.is_some() {
                            return Err("Multiple players found".to_string());
                        }
                        player_pos = Some(PlayerPos::Known(Position(x as u8, y as u8)));
                        goal_positions.push(Position(x as u8, y as u8));
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
        if goal_positions.len() != boxes.positions.len() {
            return Err(format!(
                "Goal count ({}) does not match box count ({})",
                goal_positions.len(),
                boxes.positions.len()
            ));
        }

        Ok(Game {
            tiles,
            player: player_pos,
            empty_goals,
            width: width as u8,
            height: height as u8,
            boxes,
            start_positions,
            goal_positions,
        })
    }

    pub fn get_tile(&self, pos: Position) -> Tile {
        self.tiles[pos.1 as usize][pos.0 as usize]
    }

    pub fn box_count(&self) -> usize {
        self.boxes.positions.len()
    }

    pub fn box_positions(&self) -> &[Position] {
        &self.boxes.positions
    }

    pub fn start_positions(&self) -> &[Position] {
        &self.start_positions
    }

    pub fn goal_positions(&self) -> &[Position] {
        &self.goal_positions
    }

    /// Get the box index at the given position, if any.
    /// Returns Some(box_index) if there is a box at the position, None otherwise.
    pub fn box_at(&self, pos: Position) -> Option<BoxIndex> {
        let idx = self.boxes.index[pos.1 as usize][pos.0 as usize];
        if idx == NO_BOX { None } else { Some(idx) }
    }

    /// Get the position of a box given its index.
    pub fn box_position(&self, box_index: BoxIndex) -> Position {
        assert!(
            (box_index.0 as usize) < self.boxes.positions.len(),
            "Invalid box index"
        );
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
        let dest_is_goal = dest_tile == Tile::Goal;

        // Update empty_goals count
        if source_tile == Tile::Goal {
            self.empty_goals += 1;
        }
        if dest_is_goal {
            self.empty_goals -= 1;
        }

        // Update box position
        self.boxes.move_box(box_pos, new_pos);

        // Update player position to where the box was
        self.player = PlayerPos::Known(box_pos);
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
        let old_tile = self.get_tile(old_pos);

        // Update empty_goals count
        if current_tile == Tile::Goal {
            self.empty_goals += 1; // Removing box from goal
        }
        if old_tile == Tile::Goal {
            self.empty_goals -= 1; // Placing box on goal
        }

        // Move box back
        self.boxes.move_box(new_pos, old_pos);

        // Restore player position
        self.player = PlayerPos::Known(player_old_pos);
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
        for &pos in game.boxes.positions.iter() {
            game.boxes.index[pos.1 as usize][pos.0 as usize] = NO_BOX;
        }

        // Second pass: set all new positions
        for (goal_idx, &goal_pos) in game.goal_positions.iter().enumerate() {
            let box_idx = BoxIndex(goal_idx as u8);
            game.boxes.positions[goal_idx] = goal_pos;
            game.boxes.index[goal_pos.1 as usize][goal_pos.0 as usize] = box_idx;
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
            PlayerPos::Known(pos) => {
                let mut reachable = LazyBitboard::new();
                let canonical_pos = self.player_dfs(pos, &mut reachable, |_pos, _dir, _box_idx| {});
                PlayerPos::Known(canonical_pos)
            }
            PlayerPos::Unknown => PlayerPos::Unknown,
        }
    }

    /// Compute all possible box pushes from the current game state.
    /// Uses a single DFS from player position to find all reachable boxes.
    /// Returns the pushes and the canonicalized (lexicographically smallest) player position.
    /// If the game is already solved, returns empty pushes and Unknown player position.
    /// Panics if the player position is Unknown (and game is not solved).
    pub fn compute_pushes(&self) -> (Moves<Push>, PlayerPos) {
        let mut visited = LazyBitboard::new();
        let mut boxes = Bitvector::new();
        self.compute_pushes_helper(&mut visited, &mut boxes)
    }

    fn compute_pushes_helper(
        &self,
        visited: &mut LazyBitboard,
        boxes: &mut Bitvector,
    ) -> (Moves<Push>, PlayerPos) {
        // We don't report moves when the game is solved.
        if self.is_solved() {
            return (Moves::new(), PlayerPos::Unknown);
        }

        let PlayerPos::Known(pos) = self.player else {
            panic!("Cannot compute pushes when player position is unknown");
        };
        let mut pushes = Moves::new();
        let canonical_pos = self.player_dfs(pos, visited, |_player_pos, dir, box_idx| {
            boxes.add(box_idx.0);
            let box_pos = self.box_position(box_idx);
            if let Some(dest_pos) = self.move_position(box_pos, dir) {
                if !self.is_blocked(dest_pos) {
                    pushes.add(box_idx, dir);
                }
            }
        });
        (pushes, PlayerPos::Known(canonical_pos))
    }

    pub fn compute_pi_corral_pushes(&self) -> (Moves<Push>, PlayerPos) {
        let mut player_reachable = LazyBitboard::new();
        let mut player_reachable_boxes = Bitvector::new();
        let mut corrals_visited = LazyBitboard::new();
        let mut min_cost = usize::MAX;

        // Start by computing pushes, reachable positions, and reachable boxes
        let (mut pushes, canonical_pos) =
            self.compute_pushes_helper(&mut player_reachable, &mut player_reachable_boxes);

        // Now walk through each push and examine the other side of the push for
        // a PI corral. Note that we only need to consider corrals that are the
        // other side of a valid player push (any corral NOT on the other side
        // of a player push full the "P" condition of a PI-corral).
        for push in pushes.iter() {
            let box_pos = self.box_position(push.box_index);
            let Some(new_pos) = self.move_position(box_pos, push.direction) else {
                panic!("Invalid push");
            };
            if !player_reachable.get(new_pos.0, new_pos.1)
                && !corrals_visited.get(new_pos.0, new_pos.1)
            {
                if let Some((corral_pushes, cost)) = self.compute_pi_corral_helper(
                    new_pos,
                    &player_reachable,
                    player_reachable_boxes,
                    &mut corrals_visited,
                ) {
                    // If we've found a PI-corral, check if this is is the
                    // lowest "cost" PI-corral we've found so far. If it is, set
                    // the player pushes to this PI-corral's pushes.
                    if cost < min_cost {
                        pushes = corral_pushes;
                        min_cost = cost;
                    }
                }
            }
        }

        (pushes, canonical_pos)
    }

    fn compute_pi_corral_helper(
        &self,
        pos: Position,
        player_reachable: &LazyBitboard,
        player_reachable_boxes: Bitvector,
        corrals_visited: &mut LazyBitboard,
    ) -> Option<(Moves<Push>, usize)> {
        assert!(!player_reachable.get(pos.0, pos.1));

        let mut stack: ArrayVec<Position, { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();
        let mut corral_visited = LazyBitboard::new();
        let mut corral_edge = Bitvector::new();
        let mut pushes = Moves::new();
        let mut can_prune = false;

        // Start DFS from the given position
        stack.push(pos);
        corral_visited.set(pos.0, pos.1);
        corrals_visited.set(pos.0, pos.1);

        // Perform DFS to find full extent of corral
        while let Some(curr_pos) = stack.pop() {
            let is_goal = self.get_tile(curr_pos) == Tile::Goal;

            // We've hit a box
            if let Some(box_idx) = self.box_at(curr_pos) {
                // Box not on goal: corral requires pushes to solve the puzzle
                if !is_goal {
                    can_prune = true;
                }
                // If we've hit the edge of the corral, stop exploring further
                if player_reachable_boxes.contains(box_idx.0) {
                    corral_edge.add(box_idx.0);
                    continue;
                }
            } else if is_goal {
                // Goal without a box: corral requires pushes to solve the puzzle
                can_prune = true;
            }

            // Otherwise, continue searching in all directions
            for &dir in &ALL_DIRECTIONS {
                if let Some(next_pos) = self.move_position(curr_pos, dir) {
                    if self.get_tile(next_pos) != Tile::Wall
                        && !corral_visited.get(next_pos.0, next_pos.1)
                    {
                        stack.push(next_pos);
                        corral_visited.set(next_pos.0, next_pos.1);
                        corrals_visited.set(next_pos.0, next_pos.1);
                    }
                }
            }
        }

        if !can_prune {
            return None;
        }

        // Check the PI conditions over the edge boxes
        for box_idx in corral_edge.iter() {
            let box_pos = self.box_position(BoxIndex(box_idx));
            for &dir in &ALL_DIRECTIONS {
                if let (Some(next_pos), Some(player_pos)) = (
                    self.move_position(box_pos, dir),
                    self.move_position(box_pos, dir.reverse()),
                ) {
                    // Ignore pushes originating from within the corral
                    if corral_visited.get(player_pos.0, player_pos.1) {
                        continue;
                    }
                    // Ignore pushes into a wall or box
                    if self.is_blocked(next_pos) {
                        continue;
                    }
                    // Ignore pushes coming from a wall
                    if self.get_tile(player_pos) == Tile::Wall {
                        continue;
                    }
                    // Check I condition: the push must lead into the corral
                    if !corral_visited.get(next_pos.0, next_pos.1) {
                        return None;
                    }
                    // Check P condition: the player must be capable of making the push
                    if !player_reachable.get(player_pos.0, player_pos.1) {
                        return None;
                    }
                    // Everything checks out for this push
                    pushes.add(BoxIndex(box_idx), dir);
                }
            }
        }

        let cost = pushes.len();
        Some((pushes, cost))
    }

    fn is_blocked(&self, pos: Position) -> bool {
        self.get_tile(pos) == Tile::Wall || self.boxes.has_box_at(pos)
    }

    /// Compute all possible pulls from the current game state.
    /// Returns the pulls and the canonicalized (lexicographically smallest) player position.
    /// If player position is Unknown, computes pulls from all possible player positions
    /// and returns Unknown as the canonical position.
    pub fn compute_pulls(&self) -> (Moves<Pull>, PlayerPos) {
        let mut pulls = Moves::new();
        let mut visited = LazyBitboard::new();

        match self.player {
            PlayerPos::Known(pos) => {
                let canonical_pos = self.compute_pulls_helper(pos, &mut visited, &mut pulls);
                (pulls, PlayerPos::Known(canonical_pos))
            }
            PlayerPos::Unknown => {
                assert!(self.is_solved());
                // Try each position as a potential player position
                for y in 0..self.height {
                    for x in 0..self.width {
                        // Skip if already explored from a previous position
                        if visited.get(x, y) {
                            continue;
                        }

                        let pos = Position(x, y);
                        let tile = self.get_tile(pos);
                        if (tile == Tile::Floor || tile == Tile::Goal)
                            && !self.boxes.has_box_at(pos)
                        {
                            self.compute_pulls_helper(pos, &mut visited, &mut pulls);
                        }
                    }
                }
                (pulls, PlayerPos::Unknown)
            }
        }
    }

    fn compute_pulls_helper(
        &self,
        player: Position,
        visited: &mut LazyBitboard,
        pulls: &mut Moves<Pull>,
    ) -> Position {
        self.player_dfs(player, visited, |player_pos, dir, box_idx| {
            if let Some(dest_pos) = self.move_position(player_pos, dir.reverse()) {
                if !self.is_blocked(dest_pos) {
                    pulls.add(box_idx, dir.reverse());
                }
            }
        })
    }

    /// Generic DFS helper to find all reachable player positions.
    /// Calls the `on_box` closure for each box adjacent to a reachable position.
    /// The closure receives (player_pos, direction, box_idx) and can handle box move logic.
    fn player_dfs<F>(
        &self,
        start_player: Position,
        visited: &mut LazyBitboard,
        mut on_box: F,
    ) -> Position
    where
        F: FnMut(Position, Direction, BoxIndex),
    {
        let mut canonical_pos = start_player;

        // Stack-allocated stack for DFS using ArrayVec
        let mut stack: ArrayVec<Position, { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();

        // DFS from player position to find all reachable positions
        stack.push(start_player);
        visited.set(start_player.0, start_player.1);

        while let Some(pos) = stack.pop() {
            // Check all 4 directions
            for &dir in &ALL_DIRECTIONS {
                if let Some(next_pos) = self.move_position(pos, dir) {
                    // If there's a box, notify the closure
                    if let Some(box_idx) = self.box_at(next_pos) {
                        on_box(pos, dir, box_idx);
                    } else {
                        let tile = self.get_tile(next_pos);
                        if (tile == Tile::Floor || tile == Tile::Goal)
                            && !visited.get(next_pos.0, next_pos.1)
                        {
                            // Continue DFS to this floor/goal tile
                            visited.set(next_pos.0, next_pos.1);

                            // Update canonical position if this is lexicographically smaller
                            if next_pos < canonical_pos {
                                canonical_pos = next_pos;
                            }

                            stack.push(next_pos);
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
                let pos = Position(x, y);
                let tile = self.tiles[y as usize][x as usize];
                let has_box = self.boxes.has_box_at(pos);

                let is_player =
                    matches!(self.player, PlayerPos::Known(player_pos) if pos == player_pos);

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
        assert_eq!(game.player, PlayerPos::Known(Position(2, 3)));
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
        assert_eq!(game.player, PlayerPos::Known(Position(2, 1)));
        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
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
        assert_eq!(game.get_tile(Position(3, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(3, 1)));
        // Original box position should be floor
        assert_eq!(game.get_tile(Position(2, 1)), Tile::Floor);
        assert!(!game.boxes.has_box_at(Position(2, 1)));
        // Player should be at old box position
        assert_eq!(game.player, PlayerPos::Known(Position(2, 1)));
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
        assert_eq!(game.player, PlayerPos::Known(Position(2, 1)));
        assert!(game.boxes.has_box_at(Position(3, 1)));

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
        assert_eq!(game.player, PlayerPos::Known(Position(2, 2)));
        assert_eq!(game.get_tile(Position(2, 3)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(2, 3)));

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
        assert_eq!(game.player, PlayerPos::Known(Position(2, 1)));
        assert!(game.boxes.has_box_at(Position(1, 1)));

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
        assert_eq!(game.player, PlayerPos::Known(Position(2, 2)));
        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(2, 1)));
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

        assert_eq!(game.get_tile(Position(3, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(3, 1)));
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
        assert_eq!(game.empty_goals, 1);
        assert_eq!(game.player, PlayerPos::Known(Position(2, 1)));
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

        assert_eq!(game.get_tile(Position(2, 1)), Tile::Goal);
        assert!(!game.boxes.has_box_at(Position(2, 1)));
        assert_eq!(game.get_tile(Position(3, 1)), Tile::Goal);
        assert!(game.boxes.has_box_at(Position(3, 1)));
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
            box_index: BoxIndex(10),
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
                box_index: BoxIndex(0),
                direction: Direction::Up,
            },
            Push {
                box_index: BoxIndex(0),
                direction: Direction::Down,
            },
            Push {
                box_index: BoxIndex(1),
                direction: Direction::Left,
            },
            Push {
                box_index: BoxIndex(1),
                direction: Direction::Right,
            },
        ];

        expected.sort();
        actual.sort();
        assert_eq!(expected, actual);

        // Check canonical position - should be lexicographically smallest reachable position
        // Player starts at (2, 3) and can reach many positions including (1, 1)
        assert_eq!(canonical_pos, PlayerPos::Known(Position(1, 1)));
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
            vec![Pull {
                box_index: BoxIndex(0),
                direction: Direction::Right
            }]
        );
        assert_eq!(canonical_pos, PlayerPos::Known(Position(3, 1)));
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
            Pull {
                box_index: BoxIndex(0),
                direction: Direction::Left,
            },
            Pull {
                box_index: BoxIndex(0),
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
        let push = Push {
            box_index: box_idx,
            direction: Direction::Right,
        };
        game.push(push);

        // Verify state changed
        assert_eq!(game.player, PlayerPos::Known(Position(2, 1)));
        assert!(game.boxes.has_box_at(Position(3, 1)));
        assert_eq!(game.empty_goals, 0);
        assert!(game.is_solved());

        // Pull
        game.pull(push.to_pull());

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

            let push = Push {
                box_index: box_idx,
                direction,
            };

            game.push(push);
            game.pull(push.to_pull());

            assert_eq!(game.player, original.player, "Failed for {:?}", direction);
            assert_eq!(game.boxes, original.boxes, "Failed for {:?}", direction);
            assert_eq!(
                game.empty_goals, original.empty_goals,
                "Failed for {:?}",
                direction
            );
        }
    }

    #[test]
    fn test_pi_corral_1() {
        let game = parse_game(
            r#"
########
#  $  .#
#   $@.#
#  $  .#
####   #
   # $.#
   #####
"#,
        );

        check_pi_corral(&game, 3, 2, None);
    }

    #[test]
    fn test_pi_corral_2() {
        let game = parse_game(
            r#"
########
#  $  .#
#   $@.#
#  $# .#
####   #
   # $.#
   #####
"#,
        );

        let mut expected_moves = Moves::new();
        expected_moves.add(BoxIndex(0), Direction::Left);
        expected_moves.add(BoxIndex(1), Direction::Left);
        let expected_size = 2;

        check_pi_corral(&game, 3, 2, Some((expected_moves, expected_size)));
    }

    #[test]
    fn test_pi_corral_3() {
        let game = parse_game(
            r#"
########
#.$.$ .#
#.  $@$#
#. $   #
####   #
   #   #
   #####
"#,
        );

        let mut expected_moves = Moves::new();
        expected_moves.add(BoxIndex(1), Direction::Left);
        expected_moves.add(BoxIndex(2), Direction::Left);
        expected_moves.add(BoxIndex(4), Direction::Left);
        let expected_size = 3;

        check_pi_corral(&game, 3, 2, Some((expected_moves, expected_size)));
    }

    #[test]
    fn test_pi_corral_4() {
        let game = parse_game(
            r#"
########
#.  $ .#
#. $@ $#
#. $$  #
####   #
   #  .#
   #####
"#,
        );

        check_pi_corral(&game, 2, 2, None);
    }

    #[test]
    fn test_pi_corral_5() {
        let game = parse_game(
            r#"
########
#.  $ .#
#. $@ $#
#. $#  #
####   #
   #   #
   #####
"#,
        );

        let mut expected_moves = Moves::new();
        expected_moves.add(BoxIndex(0), Direction::Left);
        expected_moves.add(BoxIndex(1), Direction::Left);
        let expected_size = 2;

        check_pi_corral(&game, 2, 2, Some((expected_moves, expected_size)));
    }

    #[test]
    fn test_pi_corral_6() {
        let game = parse_game(
            r#"
##########
#   #    #
#.  $ @$.#
####$$####
  #    #
  # .. #
  ######
"#,
        );

        let mut expected_moves = Moves::new();
        expected_moves.add(BoxIndex(0), Direction::Left);
        let expected_size = 1;

        check_pi_corral(&game, 3, 2, Some((expected_moves, expected_size)));
        check_pi_corral(&game, 5, 4, None);
    }

    #[test]
    fn test_pi_corral_7() {
        let game = parse_game(
            r#"
        ########
        #      #
        # $#$ ##
        # $  @#
        ##$ $$#
######### $ # ###
#....  ## $  $  #
##...    $   $  #
#....  ##########
########
"#,
        );

        let mut corral1_moves = Moves::new();
        corral1_moves.add(BoxIndex(8), Direction::Right);
        corral1_moves.add(BoxIndex(10), Direction::Right);
        let corral1_size = 2;

        let mut corral2_moves = Moves::new();
        corral2_moves.add(BoxIndex(9), Direction::Left);
        let corral2_size = 1;

        check_pi_corral(&game, 13, 5, None);
        check_pi_corral(&game, 14, 7, Some((corral1_moves, corral1_size)));
        check_pi_corral(&game, 8, 7, Some((corral2_moves, corral2_size)));
    }

    fn parse_game(text: &str) -> Game {
        Game::from_text(text.trim_matches('\n')).unwrap()
    }

    fn check_pi_corral(game: &Game, x: u8, y: u8, expected_result: Option<(Moves<Push>, usize)>) {
        let mut player_reachable = LazyBitboard::new();
        let mut player_reachable_boxes: Bitvector = Bitvector::new();
        let mut corrals_visited = LazyBitboard::new();

        game.compute_pushes_helper(&mut player_reachable, &mut player_reachable_boxes);

        let result = game.compute_pi_corral_helper(
            Position(x, y),
            &player_reachable,
            player_reachable_boxes,
            &mut corrals_visited,
        );

        assert_eq!(result, expected_result);
    }
}
