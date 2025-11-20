use crate::bits::{Bitvector, BitvectorIter, LazyBitboard};
use arrayvec::ArrayVec;
use std::{fmt, marker::PhantomData};

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

    pub fn reverse(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
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
pub struct Move<T: GameType> {
    box_index: u8,
    direction: Direction,
    game_type: T,
}

impl<T: GameType> Move<T> {
    pub fn new(box_index: u8, direction: Direction) -> Self {
        Self {
            box_index,
            direction,
            game_type: T::default(),
        }
    }

    pub fn box_index(&self) -> u8 {
        self.box_index
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveByPos<T: GameType> {
    box_x: u8,
    box_y: u8,
    direction: Direction,
    game_type: T,
}

impl<T: GameType> MoveByPos<T> {
    pub fn new(box_x: u8, box_y: u8, direction: Direction) -> Self {
        Self {
            box_x,
            box_y,
            direction,
            game_type: T::default(),
        }
    }

    pub fn box_x(&self) -> u8 {
        self.box_x
    }

    pub fn box_y(&self) -> u8 {
        self.box_y
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

impl From<&MoveByPos<Reverse>> for MoveByPos<Forward> {
    fn from(move_by_pos: &MoveByPos<Reverse>) -> Self {
        Self {
            box_x: move_by_pos.box_x,
            box_y: move_by_pos.box_y,
            direction: move_by_pos.direction.reverse(),
            game_type: Forward,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moves<T: GameType> {
    // Bitset: bits[0] = Up, bits[1] = Down, bits[2] = Left, bits[3] = Right
    // Each Bitvector holds 64 bits for 64 boxes (box indices 0-63)
    bits: [Bitvector; 4],
    game_type: T,
}

impl<T: GameType> Moves<T> {
    fn new() -> Self {
        Moves {
            bits: [Bitvector::new(); 4],
            game_type: T::default(),
        }
    }

    fn add(&mut self, box_index: u8, direction: Direction) {
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

    pub fn contains(&self, push: Move<T>) -> bool {
        let dir_idx = push.direction.index();
        self.bits[dir_idx].contains(push.box_index)
    }

    pub fn iter(&self) -> MovesIter<T> {
        MovesIter {
            bits: self.bits,
            dir_idx: 0,
            current_iter: self.bits[0].iter(),
            game_type: PhantomData,
        }
    }
}

impl From<Moves<Reverse>> for Moves<Forward> {
    fn from(moves: Moves<Reverse>) -> Self {
        Moves {
            bits: swizzle(moves.bits),
            game_type: Forward,
        }
    }
}

impl From<Moves<Forward>> for Moves<Reverse> {
    fn from(moves: Moves<Forward>) -> Self {
        Moves {
            bits: swizzle(moves.bits),
            game_type: Reverse,
        }
    }
}

fn swizzle(bits: [Bitvector; 4]) -> [Bitvector; 4] {
    [bits[1], bits[0], bits[3], bits[2]]
}

pub struct MovesIter<T: GameType> {
    bits: [Bitvector; 4],
    dir_idx: usize,
    current_iter: BitvectorIter,
    game_type: PhantomData<T>,
}

impl<T: GameType> Iterator for MovesIter<T> {
    type Item = Move<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(box_index) = self.current_iter.next() {
                let direction = Direction::from_index(self.dir_idx);
                return Some(Move {
                    box_index,
                    direction,
                    game_type: T::default(),
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

impl<T: GameType> IntoIterator for &'_ Moves<T> {
    type Item = Move<T>;
    type IntoIter = MovesIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game<T: GameType> {
    tiles: [[Tile; MAX_SIZE]; MAX_SIZE],
    player: PlayerPos,
    width: u8,
    height: u8,
    box_positions: ArrayVec<(u8, u8), MAX_BOXES>,
    // Maps board position to box index (255 = no box at this position)
    box_index: [[u8; MAX_SIZE]; MAX_SIZE],
    start_positions: ArrayVec<(u8, u8), MAX_BOXES>,
    goal_positions: ArrayVec<(u8, u8), MAX_BOXES>,
    empty_goals: usize,
    game_type: T,
}

pub trait GameType: Clone + Copy + PartialEq + Eq + Default + std::fmt::Debug {
    fn is_forward(&self) -> bool;
    fn apply_move(&self, game: &mut Game<Self>, move_: Move<Self>);
    fn unapply_move(&self, game: &mut Game<Self>, move_: Move<Self>);
    fn compute_moves(&self, game: &Game<Self>, pi_corrals: bool) -> (Moves<Self>, PlayerPos);
    fn compute_unmoves(&self, game: &Game<Self>) -> (Moves<Self>, PlayerPos);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Forward;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Reverse;

impl<T: GameType> Game<T> {
    pub fn game_type(&self) -> T {
        self.game_type
    }

    pub fn get_tile(&self, x: u8, y: u8) -> Tile {
        self.tiles[y as usize][x as usize]
    }

    pub fn box_count(&self) -> usize {
        self.box_positions.len()
    }

    pub fn box_pos(&self, index: usize) -> (u8, u8) {
        self.box_positions[index]
    }

    pub fn start_pos(&self, index: usize) -> (u8, u8) {
        self.start_positions[index]
    }

    pub fn goal_pos(&self, index: usize) -> (u8, u8) {
        self.goal_positions[index]
    }

    /// Get the box index at the given position, if any.
    /// Returns Some(box_index) if there is a box at (x, y), None otherwise.
    pub fn box_at(&self, x: u8, y: u8) -> Option<u8> {
        let idx = self.box_index[y as usize][x as usize];
        if idx == 255 { None } else { Some(idx) }
    }

    /// Move from position (x, y) in the given direction.
    /// Returns Some((new_x, new_y)) if the new position is within bounds, None otherwise.
    pub fn move_pos(&self, x: u8, y: u8, dir: Direction) -> Option<(u8, u8)> {
        let (dx, dy) = dir.delta();
        let new_x = x as i32 + dx as i32;
        let new_y = y as i32 + dy as i32;

        if new_x >= 0 && new_y >= 0 && new_x < self.width as i32 && new_y < self.height as i32 {
            Some((new_x as u8, new_y as u8))
        } else {
            None
        }
    }

    /// Pushes a box by box index.
    /// Updates the player position to where the box was.
    pub fn push(&mut self, box_index: u8, direction: Direction) {
        assert!(
            (box_index as usize) < self.box_positions.len(),
            "Invalid box index: {}",
            box_index
        );
        let (x, y) = self.box_positions[box_index as usize];
        self.push_by_pos(x, y, direction);
    }

    /// Pushes a box by box position.
    /// Updates the player position to where the box was.
    pub fn push_by_pos(&mut self, x: u8, y: u8, direction: Direction) {
        self.move_box(x, y, direction);
        self.player = PlayerPos::Known(x, y);
    }

    pub fn pull(&mut self, box_index: u8, direction: Direction) {
        assert!(
            (box_index as usize) < self.box_positions.len(),
            "Invalid box index: {}",
            box_index
        );

        // Update box position
        let (x, y) = self.box_positions[box_index as usize];
        let (old_x, old_y) = self.move_box(x, y, direction);

        // Update player position
        let (player_old_x, player_old_y) = self
            .move_pos(old_x, old_y, direction)
            .expect("Pull player position out of bounds");
        self.player = PlayerPos::Known(player_old_x, player_old_y);
    }

    fn move_box(&mut self, x: u8, y: u8, direction: Direction) -> (u8, u8) {
        let (new_x, new_y) = self
            .move_pos(x, y, direction)
            .expect("Push destination out of bounds");

        assert!(!self.is_blocked(new_x, new_y), "destination blocked");

        // Update box position
        let idx = self.box_index[y as usize][x as usize];
        self.box_positions[idx as usize] = (new_x, new_y);
        self.box_index[y as usize][x as usize] = 255;
        self.box_index[new_y as usize][new_x as usize] = idx;

        // Update empty goals
        if self.get_tile(x, y) == Tile::Goal {
            self.empty_goals += 1;
        }
        if self.get_tile(new_x, new_y) == Tile::Goal {
            self.empty_goals -= 1;
        }

        (new_x, new_y)
    }

    /// Check if all boxes are on goals (win condition)
    pub fn is_solved(&self) -> bool {
        self.empty_goals == 0
    }

    /// Check if there is a box at the given position
    fn has_box_at(&self, x: u8, y: u8) -> bool {
        self.box_index[y as usize][x as usize] != 255
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
    pub fn compute_pushes(&self) -> (Moves<Forward>, PlayerPos) {
        let mut visited = LazyBitboard::new();
        let mut boxes = Bitvector::new();
        self.compute_pushes_helper(&mut visited, &mut boxes)
    }

    fn compute_pushes_helper(
        &self,
        visited: &mut LazyBitboard,
        boxes: &mut Bitvector,
    ) -> (Moves<Forward>, PlayerPos) {
        if self.is_solved() {
            return (Moves::new(), PlayerPos::Unknown);
        }
        let PlayerPos::Known(x, y) = self.player else {
            panic!("Cannot compute pushes when player position is unknown");
        };
        let mut pushes = Moves::new();
        let canonical_pos = self.player_dfs((x, y), visited, |_player_pos, dir, box_idx| {
            boxes.add(box_idx);
            let box_pos = self.box_pos(box_idx as usize);
            if let Some((dest_x, dest_y)) = self.move_pos(box_pos.0, box_pos.1, dir) {
                if !self.is_blocked(dest_x, dest_y) {
                    pushes.add(box_idx, dir);
                }
            }
        });
        (pushes, PlayerPos::Known(canonical_pos.0, canonical_pos.1))
    }

    pub fn compute_pi_corral_pushes(&self) -> (Moves<Forward>, PlayerPos) {
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
            let (bx, by) = self.box_pos(push.box_index as usize);
            let Some((nx, ny)) = self.move_pos(bx, by, push.direction) else {
                panic!("Invalid push");
            };
            if !player_reachable.get(nx, ny) && !corrals_visited.get(nx, ny) {
                if let Some((corral_pushes, cost)) = self.compute_pi_corral_helper(
                    nx,
                    ny,
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
        x: u8,
        y: u8,
        player_reachable: &LazyBitboard,
        player_reachable_boxes: Bitvector,
        corrals_visited: &mut LazyBitboard,
    ) -> Option<(Moves<Forward>, usize)> {
        assert!(!player_reachable.get(x, y));

        let mut stack: ArrayVec<(u8, u8), { MAX_SIZE * MAX_SIZE }> = ArrayVec::new();
        let mut corral_visited = LazyBitboard::new();
        let mut corral_edge = Bitvector::new();
        let mut pushes = Moves::new();
        let mut can_prune = false;

        // Start DFS from the given position
        stack.push((x, y));
        corral_visited.set(x, y);
        corrals_visited.set(x, y);

        // Perform DFS to find full extent of corral
        while let Some((cx, cy)) = stack.pop() {
            let is_goal = self.get_tile(cx, cy) == Tile::Goal;

            // We've hit a box
            if let Some(box_idx) = self.box_at(cx, cy) {
                // Box not on goal: corral requires pushes to solve the puzzle
                if !is_goal {
                    can_prune = true;
                }
                // If we've hit the edge of the corral, stop exploring further
                if player_reachable_boxes.contains(box_idx) {
                    corral_edge.add(box_idx);
                    continue;
                }
            } else if is_goal {
                // Goal without a box: corral requires pushes to solve the puzzle
                can_prune = true;
            }

            // Otherwise, continue searching in all directions
            for &dir in &ALL_DIRECTIONS {
                if let Some((nx, ny)) = self.move_pos(cx, cy, dir) {
                    if self.get_tile(nx, ny) != Tile::Wall && !corral_visited.get(nx, ny) {
                        stack.push((nx, ny));
                        corral_visited.set(nx, ny);
                        corrals_visited.set(nx, ny);
                    }
                }
            }
        }

        if !can_prune {
            return None;
        }

        // Check the PI conditions over the edge boxes
        for box_idx in corral_edge.iter() {
            let (bx, by) = self.box_pos(box_idx as usize);
            for &dir in &ALL_DIRECTIONS {
                if let (Some((nx, ny)), Some((px, py))) = (
                    self.move_pos(bx, by, dir),
                    self.move_pos(bx, by, dir.reverse()),
                ) {
                    // Ignore pushes originating from within the corral
                    if corral_visited.get(px, py) {
                        continue;
                    }
                    // Ignore pushes into a wall or box
                    if self.is_blocked(nx, ny) {
                        continue;
                    }
                    // Ignore pushes coming from a wall
                    if self.get_tile(px, py) == Tile::Wall {
                        continue;
                    }
                    // Check I condition: the push must lead into the corral
                    if !corral_visited.get(nx, ny) {
                        return None;
                    }
                    // Check P condition: the player must be capable of making the push
                    if !player_reachable.get(px, py) {
                        return None;
                    }
                    // Everything checks out for this push
                    pushes.add(box_idx, dir);
                }
            }
        }

        let cost = pushes.len();
        Some((pushes, cost))
    }

    fn is_blocked(&self, x: u8, y: u8) -> bool {
        self.get_tile(x, y) == Tile::Wall || self.has_box_at(x, y)
    }

    /// Compute all possible pulls from the current game state.
    /// Returns the pulls and the canonicalized (lexicographically smallest) player position.
    /// If player position is Unknown, computes pulls from all possible player positions
    /// and returns Unknown as the canonical position.
    pub fn compute_pulls(&self) -> (Moves<Reverse>, PlayerPos) {
        let mut pulls = Moves::new();
        let mut visited = LazyBitboard::new();

        match self.player {
            PlayerPos::Known(x, y) => {
                let canonical_pos = self.compute_pulls_helper((x, y), &mut visited, &mut pulls);
                (pulls, PlayerPos::Known(canonical_pos.0, canonical_pos.1))
            }
            PlayerPos::Unknown => {
                // Try each position as a potential player position
                for y in 0..self.height {
                    for x in 0..self.width {
                        // Skip if already explored from a previous position
                        if visited.get(x, y) {
                            continue;
                        }

                        let tile = self.get_tile(x, y);
                        if (tile == Tile::Floor || tile == Tile::Goal) && !self.has_box_at(x, y) {
                            self.compute_pulls_helper((x, y), &mut visited, &mut pulls);
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
        visited: &mut LazyBitboard,
        pulls: &mut Moves<Reverse>,
    ) -> (u8, u8) {
        self.player_dfs(player, visited, |(x, y), dir, box_idx| {
            if let Some((dest_x, dest_y)) = self.move_pos(x, y, dir.reverse()) {
                if !self.is_blocked(dest_x, dest_y) {
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
        start_player: (u8, u8),
        visited: &mut LazyBitboard,
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
        visited.set(start_player.0, start_player.1);

        while let Some((x, y)) = stack.pop() {
            // Check all 4 directions
            for &dir in &ALL_DIRECTIONS {
                if let Some((nx, ny)) = self.move_pos(x, y, dir) {
                    // If there's a box, notify the closure
                    if let Some(box_idx) = self.box_at(nx, ny) {
                        on_box((x, y), dir, box_idx);
                    } else {
                        let tile = self.get_tile(nx, ny);
                        if (tile == Tile::Floor || tile == Tile::Goal) && !visited.get(nx, ny) {
                            // Continue DFS to this floor/goal tile
                            visited.set(nx, ny);

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

    pub fn compute_moves(&self, pi_corrals: bool) -> (Moves<T>, PlayerPos) {
        self.game_type().compute_moves(self, pi_corrals)
    }

    pub fn compute_unmoves(&self) -> (Moves<T>, PlayerPos) {
        self.game_type().compute_unmoves(self)
    }

    pub fn apply_move(&mut self, move_: Move<T>) {
        self.game_type().apply_move(self, move_);
    }

    pub fn unapply_move(&mut self, move_: Move<T>) {
        self.game_type().unapply_move(self, move_);
    }
}

impl<T: GameType> fmt::Display for Game<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                let tile = self.tiles[y as usize][x as usize];
                let has_box = self.has_box_at(x, y);

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

impl GameType for Forward {
    fn apply_move(&self, game: &mut Game<Self>, move_: Move<Self>) {
        game.push(move_.box_index, move_.direction);
    }

    fn unapply_move(&self, game: &mut Game<Self>, move_: Move<Self>) {
        game.pull(move_.box_index, move_.direction.reverse());
    }

    fn is_forward(&self) -> bool {
        true
    }

    fn compute_moves(&self, game: &Game<Self>, pi_corrals: bool) -> (Moves<Self>, PlayerPos) {
        if pi_corrals {
            game.compute_pi_corral_pushes()
        } else {
            game.compute_pushes()
        }
    }

    fn compute_unmoves(&self, game: &Game<Self>) -> (Moves<Self>, PlayerPos) {
        let (pulls, canonical_pos) = game.compute_pulls();
        let pushes = pulls.into();
        (pushes, canonical_pos)
    }
}

impl GameType for Reverse {
    fn apply_move(&self, game: &mut Game<Self>, move_: Move<Self>) {
        game.pull(move_.box_index, move_.direction);
    }

    fn unapply_move(&self, game: &mut Game<Self>, move_: Move<Self>) {
        game.push(move_.box_index, move_.direction.reverse());
    }

    fn is_forward(&self) -> bool {
        false
    }

    fn compute_moves(&self, game: &Game<Self>, _pi_corrals: bool) -> (Moves<Self>, PlayerPos) {
        game.compute_pulls()
    }

    fn compute_unmoves(&self, game: &Game<Self>) -> (Moves<Self>, PlayerPos) {
        let (pushes, canonical_pos) = game.compute_pushes();
        let pulls = pushes.into();
        (pulls, canonical_pos)
    }
}

impl Game<Forward> {
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
        let mut box_positions = ArrayVec::new();
        let mut box_index = [[255u8; MAX_SIZE]; MAX_SIZE];
        let mut start_positions = ArrayVec::new();
        let mut goal_positions = ArrayVec::new();
        let mut empty_goals = 0;

        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                match ch {
                    '#' => tiles[y][x] = Tile::Wall,
                    ' ' => tiles[y][x] = Tile::Floor,
                    '.' => {
                        tiles[y][x] = Tile::Goal;
                        goal_positions.push((x as u8, y as u8));
                        empty_goals += 1;
                    }
                    '$' => {
                        tiles[y][x] = Tile::Floor;
                        let box_idx = box_positions.len() as u8;
                        start_positions.push((x as u8, y as u8));
                        box_positions.push((x as u8, y as u8));
                        box_index[y][x] = box_idx;
                    }
                    '*' => {
                        tiles[y][x] = Tile::Goal;
                        goal_positions.push((x as u8, y as u8));
                        let box_idx = box_positions.len() as u8;
                        start_positions.push((x as u8, y as u8));
                        box_positions.push((x as u8, y as u8));
                        box_index[y][x] = box_idx;
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
                        goal_positions.push((x as u8, y as u8));
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
        if goal_positions.len() != box_positions.len() {
            return Err(format!(
                "Goal count ({}) does not match box count ({})",
                goal_positions.len(),
                box_positions.len()
            ));
        }

        Ok(Game {
            tiles,
            player: player_pos,
            width: width as u8,
            height: height as u8,
            box_positions,
            box_index,
            start_positions,
            goal_positions,
            empty_goals,
            game_type: Forward,
        })
    }

    /// Make a game state  in the solved position where all boxes are on goals
    /// and the player position is unknown. This is useful for backward search.
    pub fn make_goal_state(&self) -> Game<Reverse> {
        let mut game = self.clone();

        // Move all boxes to their corresponding goals
        // Do this in two passes to avoid clobbering unprocessed boxes

        // First pass: clear all current positions in index
        for i in 0..game.box_positions.len() {
            let current_pos = game.box_positions[i];
            game.box_index[current_pos.1 as usize][current_pos.0 as usize] = 255;
        }

        // Second pass: set all new positions
        for i in 0..game.box_positions.len() {
            let goal_pos = game.goal_positions[i];
            game.box_positions[i] = goal_pos;
            game.box_index[goal_pos.1 as usize][goal_pos.0 as usize] = i as u8;
        }

        game.player = PlayerPos::Unknown;
        game.empty_goals = 0;

        (&game).into()
    }
}

impl From<&Game<Reverse>> for Game<Forward> {
    fn from(game: &Game<Reverse>) -> Self {
        Game {
            tiles: game.tiles,
            player: game.player,
            width: game.width,
            height: game.height,
            box_positions: game.box_positions.clone(),
            box_index: game.box_index,
            start_positions: game.start_positions.clone(),
            goal_positions: game.goal_positions.clone(),
            empty_goals: game.empty_goals,
            game_type: Forward,
        }
    }
}

impl From<&Game<Forward>> for Game<Reverse> {
    fn from(game: &Game<Forward>) -> Self {
        Game {
            tiles: game.tiles,
            player: game.player,
            width: game.width,
            height: game.height,
            box_positions: game.box_positions.clone(),
            box_index: game.box_index,
            start_positions: game.start_positions.clone(),
            goal_positions: game.goal_positions.clone(),
            empty_goals: game.empty_goals,
            game_type: Reverse,
        }
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
        assert!(!game.is_solved());

        // Board with all boxes on goals
        let all_solved = "####\n\
                          #*@#\n\
                          ####";
        let game = Game::from_text(all_solved).unwrap();
        assert!(game.is_solved());

        // Board with no boxes on goals
        let none_solved = "####\n\
                           #$.#\n\
                           # @#\n\
                           ####";
        let game = Game::from_text(none_solved).unwrap();
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
        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);

        // Box should now be on goal at (3, 1)
        assert_eq!(game.get_tile(3, 1), Tile::Goal);
        assert!(game.has_box_at(3, 1));
        // Original box position should be floor
        assert_eq!(game.get_tile(2, 1), Tile::Floor);
        assert!(!game.has_box_at(2, 1));
        // Player should be at old box position
        assert_eq!(game.player, PlayerPos::Known(2, 1));
        // Should be solved
        assert!(game.is_solved());
    }

    #[test]
    fn test_push_all_directions() {
        // Test pushing right
        let input = "####\n\
                     #@$ #\n\
                     # . #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);
        assert_eq!(game.player, PlayerPos::Known(2, 1));
        assert!(game.has_box_at(3, 1));

        // Test pushing down
        let input = "#####\n\
                     # @ #\n\
                     # $ #\n\
                     # . #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.box_index[2][2];
        game.push(box_idx, Direction::Down);
        assert_eq!(game.player, PlayerPos::Known(2, 2));
        assert_eq!(game.get_tile(2, 3), Tile::Goal);
        assert!(game.has_box_at(2, 3));

        // Test pushing left
        let input = "####\n\
                     # $@#\n\
                     # . #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Left);
        assert_eq!(game.player, PlayerPos::Known(2, 1));
        assert!(game.has_box_at(1, 1));

        // Test pushing up
        let input = "#####\n\
                     # . #\n\
                     # $ #\n\
                     # @ #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();
        let box_idx = game.box_index[2][2];
        game.push(box_idx, Direction::Up);
        assert_eq!(game.player, PlayerPos::Known(2, 2));
        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(game.has_box_at(2, 1));
    }

    #[test]
    fn test_push_floor_to_goal() {
        // Push box from floor onto goal
        let input = "####\n\
                     #@$.#\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();

        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);

        assert_eq!(game.get_tile(3, 1), Tile::Goal);
        assert!(game.has_box_at(3, 1));
    }

    #[test]
    fn test_push_goal_to_floor() {
        // Push box from goal onto floor
        let input = "#####\n\
                     #@*  #\n\
                     #####";
        let mut game = Game::from_text(input).unwrap();

        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(game.has_box_at(2, 1));

        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);

        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(!game.has_box_at(2, 1));
        assert_eq!(game.get_tile(3, 1), Tile::Floor);
        assert!(game.has_box_at(3, 1));
        assert_eq!(game.player, PlayerPos::Known(2, 1));
    }

    #[test]
    fn test_push_goal_to_goal() {
        // Push box from one goal to another goal
        let input = "######\n\
                     #@*.$#\n\
                     ######";
        let mut game = Game::from_text(input).unwrap();

        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);

        assert_eq!(game.get_tile(2, 1), Tile::Goal);
        assert!(!game.has_box_at(2, 1));
        assert_eq!(game.get_tile(3, 1), Tile::Goal);
        assert!(game.has_box_at(3, 1));
    }

    #[test]
    #[should_panic(expected = "Invalid box index")]
    fn test_push_no_box() {
        let input = "####\n\
                     #@  #\n\
                     ####";
        let mut game = Game::from_text(input).unwrap();

        // Try to push with invalid box index
        game.push(10, Direction::Right);
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
        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);
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
        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);
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
                game_type: Forward,
            },
            Move {
                box_index: 0,
                direction: Direction::Down,
                game_type: Forward,
            },
            Move {
                box_index: 1,
                direction: Direction::Left,
                game_type: Forward,
            },
            Move {
                box_index: 1,
                direction: Direction::Right,
                game_type: Forward,
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
                direction: Direction::Right,
                game_type: Reverse
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
                game_type: Reverse,
            },
            Move {
                box_index: 0,
                direction: Direction::Right,
                game_type: Reverse,
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
        let original_box_positions = game.box_positions.clone();

        // Push box right
        let box_idx = game.box_index[1][2];
        game.push(box_idx, Direction::Right);

        // Verify state changed
        assert_eq!(game.player, PlayerPos::Known(2, 1));
        assert!(game.has_box_at(3, 1));
        assert!(game.is_solved());

        // Pull
        game.pull(box_idx, Direction::Left);

        // Should be back to original state
        assert_eq!(game.player, original_player);
        assert_eq!(game.box_positions, original_box_positions);
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
            let box_pos = game.box_positions[0];
            let box_idx = game.box_index[box_pos.1 as usize][box_pos.0 as usize];

            game.push(box_idx, direction);
            game.pull(box_idx, direction.reverse());

            assert_eq!(game.player, original.player, "Failed for {:?}", direction);
            assert_eq!(
                game.box_positions, original.box_positions,
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
        expected_moves.add(0, Direction::Left);
        expected_moves.add(1, Direction::Left);
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
        expected_moves.add(1, Direction::Left);
        expected_moves.add(2, Direction::Left);
        expected_moves.add(4, Direction::Left);
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
        expected_moves.add(0, Direction::Left);
        expected_moves.add(1, Direction::Left);
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
        expected_moves.add(0, Direction::Left);
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
        corral1_moves.add(8, Direction::Right);
        corral1_moves.add(10, Direction::Right);
        let corral1_size = 2;

        let mut corral2_moves = Moves::new();
        corral2_moves.add(9, Direction::Left);
        let corral2_size = 1;

        check_pi_corral(&game, 13, 5, None);
        check_pi_corral(&game, 14, 7, Some((corral1_moves, corral1_size)));
        check_pi_corral(&game, 8, 7, Some((corral2_moves, corral2_size)));
    }

    fn parse_game(text: &str) -> Game<Forward> {
        Game::from_text(text.trim_matches('\n')).unwrap()
    }

    fn check_pi_corral(
        game: &Game<Forward>,
        x: u8,
        y: u8,
        expected_result: Option<(Moves<Forward>, usize)>,
    ) {
        let mut player_reachable = LazyBitboard::new();
        let mut player_reachable_boxes: Bitvector = Bitvector::new();
        let mut corrals_visited = LazyBitboard::new();

        game.compute_pushes_helper(&mut player_reachable, &mut player_reachable_boxes);

        let result = game.compute_pi_corral_helper(
            x,
            y,
            &player_reachable,
            player_reachable_boxes,
            &mut corrals_visited,
        );

        assert_eq!(result, expected_result);
    }
}
