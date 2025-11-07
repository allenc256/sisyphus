use std::fmt;

const MAX_SIZE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Wall,
    Floor,
    Goal,
    Box,
    BoxOnGoal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    tiles: [[Tile; MAX_SIZE]; MAX_SIZE],
    player: (u8, u8),
    empty_goals: u8,
    width: u8,
    height: u8,
    // Mapping from (x, y) position to unique index (255 = wall/invalid)
    indexes: [[u8; MAX_SIZE]; MAX_SIZE],
    // Bitset representing which positions have boxes (indexed by position index)
    box_bitset: [u32; 8],
}

impl Board {
    /// Build position-to-index mapping using flood-fill from player position.
    /// Returns a 2D array where each reachable position maps to a unique u8 index,
    /// and unreachable/wall positions map to 255.
    fn build_position_indexes(
        tiles: &[[Tile; MAX_SIZE]; MAX_SIZE],
        player_pos: (u8, u8),
        width: usize,
        height: usize,
    ) -> [[u8; MAX_SIZE]; MAX_SIZE] {
        let mut indexes = [[255u8; MAX_SIZE]; MAX_SIZE];
        let mut next_index = 0u8;
        let mut queue: Vec<(u8, u8)> = Vec::new();

        // Start flood-fill from player position
        queue.push(player_pos);
        indexes[player_pos.1 as usize][player_pos.0 as usize] = next_index;
        next_index += 1;

        let directions = [(0, -1), (1, 0), (0, 1), (-1, 0)];

        while let Some((x, y)) = queue.pop() {
            // Check all 4 directions
            for (dx, dy) in directions.iter() {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;

                // Check bounds
                if nx >= 0 && ny >= 0 && nx < width as i32 && ny < height as i32 {
                    let nx_usize = nx as usize;
                    let ny_usize = ny as usize;

                    // If not visited and not a wall
                    if indexes[ny_usize][nx_usize] == 255 && tiles[ny_usize][nx_usize] != Tile::Wall
                    {
                        indexes[ny_usize][nx_usize] = next_index;
                        next_index += 1;
                        queue.push((nx as u8, ny as u8));
                    }
                }
            }
        }

        indexes
    }

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
        let mut box_count: u8 = 0;
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
                        tiles[y][x] = Tile::Box;
                        box_count += 1;
                    }
                    '*' => {
                        tiles[y][x] = Tile::BoxOnGoal;
                        box_count += 1;
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
        if goal_count != box_count {
            return Err(format!(
                "Goal count ({}) does not match box count ({})",
                goal_count, box_count
            ));
        }

        // Build position-to-index mapping using flood-fill
        let position_to_index = Self::build_position_indexes(&tiles, player_pos, width, height);

        // Build box bitset from tiles
        let mut box_bitset = [0u32; 8];
        for y in 0..height {
            for x in 0..width {
                let tile = tiles[y][x];
                if tile == Tile::Box || tile == Tile::BoxOnGoal {
                    let index = position_to_index[y][x];
                    if index < 255 {
                        let word_idx = (index / 32) as usize;
                        let bit_idx = index % 32;
                        box_bitset[word_idx] |= 1u32 << bit_idx;
                    }
                }
            }
        }

        Ok(Board {
            tiles,
            player: player_pos,
            empty_goals,
            width: width as u8,
            height: height as u8,
            indexes: position_to_index,
            box_bitset,
        })
    }

    pub fn width(&self) -> usize {
        self.width as usize
    }

    pub fn height(&self) -> usize {
        self.height as usize
    }

    pub fn player_pos(&self) -> (u8, u8) {
        self.player
    }

    pub fn get_tile(&self, x: u8, y: u8) -> Tile {
        self.tiles[y as usize][x as usize]
    }

    /// Check if all boxes are on goals (win condition)
    pub fn is_solved(&self) -> bool {
        self.empty_goals == 0
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for y in 0..self.height {
            let mut line = String::new();
            for x in 0..self.width {
                let tile = self.tiles[y as usize][x as usize];

                let ch = if (x, y) == self.player {
                    match tile {
                        Tile::Goal => '+',
                        _ => '@',
                    }
                } else {
                    match tile {
                        Tile::Wall => '#',
                        Tile::Floor => ' ',
                        Tile::Goal => '.',
                        Tile::Box => '$',
                        Tile::BoxOnGoal => '*',
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
        let input = "####\n# .#\n#  ###\n#*@  #\n#  $ #\n#  ###\n####";
        let board = Board::from_text(input).unwrap();

        assert_eq!(board.width(), 6);
        assert_eq!(board.height(), 7);
        assert_eq!(board.player_pos(), (2, 3));
    }

    #[test]
    fn test_no_player() {
        let input = "####\n#  #\n####";
        assert!(Board::from_text(input).is_err());
    }

    #[test]
    fn test_multiple_players() {
        let input = "####\n#@@#\n####";
        assert!(Board::from_text(input).is_err());
    }

    #[test]
    fn test_player_on_goal() {
        let input = "####\n#$+ #\n#$. #\n####";
        let board = Board::from_text(input).unwrap();
        assert_eq!(board.player_pos(), (2, 1));
        assert_eq!(board.get_tile(2, 1), Tile::Goal);
    }

    #[test]
    fn test_display() {
        let input = "####\n# .#\n#  ###\n#*@  #\n#  $ #\n#  ###\n####";
        let board = Board::from_text(input).unwrap();
        let output = board.to_string();
        assert_eq!(output.trim(), input);
    }

    #[test]
    fn test_is_solved() {
        let solved = "####\n#*@#\n####";
        let board = Board::from_text(solved).unwrap();
        assert!(board.is_solved());

        let unsolved = "####\n#$.#\n# @#\n####";
        let board = Board::from_text(unsolved).unwrap();
        assert!(!board.is_solved());
    }

    #[test]
    fn test_empty_goals_tarcking() {
        // Board with 1 box on goal, 1 box not on goal
        let input = "####\n# .#\n#  ###\n#*@  #\n#  $ #\n#  ###\n####";
        let board = Board::from_text(input).unwrap();
        assert_eq!(board.empty_goals, 1);
        assert!(!board.is_solved());

        // Board with all boxes on goals
        let all_solved = "####\n#*@#\n####";
        let board = Board::from_text(all_solved).unwrap();
        assert_eq!(board.empty_goals, 0);
        assert!(board.is_solved());

        // Board with no boxes on goals
        let none_solved = "####\n#$.#\n# @#\n####";
        let board = Board::from_text(none_solved).unwrap();
        assert_eq!(board.empty_goals, 1);
        assert!(!board.is_solved());
    }

    #[test]
    fn test_goal_box_count_validation() {
        // More goals than boxes - should fail
        let more_goals = "####\n#..#\n# $@#\n####";
        assert!(Board::from_text(more_goals).is_err());

        // More boxes than goals - should fail
        let more_boxes = "####\n#$$#\n# .@#\n####";
        assert!(Board::from_text(more_boxes).is_err());

        // Equal goals and boxes - should succeed
        let balanced = "####\n#$.#\n# * #\n# @#\n####";
        assert!(Board::from_text(balanced).is_ok());
    }

    #[test]
    fn test_position_mapping() {
        // Simple board with player at (1, 1), balanced boxes and goals
        let input = "####\n#@*#\n#$.#\n####";
        let board = Board::from_text(input).unwrap();

        // Player position should have index 0
        assert_eq!(board.indexes[1][1], 0);

        // Walls should have index 255
        assert_eq!(board.indexes[0][0], 255);
        assert_eq!(board.indexes[0][3], 255);

        // Adjacent floor positions should be reachable
        let idx_2_1 = board.indexes[1][2];
        assert!(idx_2_1 < 255);

        let idx_1_2 = board.indexes[2][1];
        assert!(idx_1_2 < 255);

        // All reachable positions should have unique indices
        assert_ne!(board.indexes[1][1], board.indexes[1][2]);
    }

    #[test]
    fn test_position_mapping_unreachable() {
        // Board with unreachable area - no boxes/goals to keep validation happy
        let input = "#####\n#@  #\n#####\n#   #\n#####";
        let board = Board::from_text(input).unwrap();

        // Player position and adjacent should be reachable
        assert!(board.indexes[1][1] < 255);
        assert!(board.indexes[1][2] < 255);
        assert!(board.indexes[1][3] < 255);

        // Area below wall should be unreachable
        assert_eq!(board.indexes[3][1], 255);
        assert_eq!(board.indexes[3][2], 255);
    }

    #[test]
    fn test_box_bitset() {
        // Board with boxes at known positions
        let input = "####\n#@*#\n#$.#\n####";
        let board = Board::from_text(input).unwrap();

        // Get indices for box positions
        let box1_idx = board.indexes[1][2]; // Box on goal at (2,1)
        let box2_idx = board.indexes[2][1]; // Box at (1,2)

        // Check that boxes are present in bitset
        assert!(has_box_at_index(&board, box1_idx));
        assert!(has_box_at_index(&board, box2_idx));

        // Check that non-box positions don't have boxes
        let player_idx = board.indexes[1][1];
        assert!(!has_box_at_index(&board, player_idx));

        // Verify box count matches number of set bits
        let set_bits = board
            .box_bitset
            .iter()
            .map(|&word| word.count_ones())
            .sum::<u32>();
        assert!(set_bits == 2);
    }

    fn has_box_at_index(board: &Board, idx: u8) -> bool {
        if idx == 255 {
            return false;
        }
        let word_idx = (idx / 32) as usize;
        let bit_idx = idx % 32;
        return (board.box_bitset[word_idx] & (1u32 << bit_idx)) != 0;
    }
}
