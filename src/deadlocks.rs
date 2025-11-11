use crate::game::{ALL_DIRECTIONS, Game, MAX_SIZE, Push, Tile};

pub struct Deadlocks {
    /// Positions that are NOT reachable by unpushing a box from a goal.
    unreachable: [[bool; MAX_SIZE]; MAX_SIZE],
}

impl Deadlocks {
    pub fn new(game: &Game) -> Self {
        let mut reachable = [[false; MAX_SIZE]; MAX_SIZE];

        // For each goal, perform DFS via unpushes to find reachable positions
        for goal_idx in 0..game.box_count() {
            let goal_pos = game.goal_pos(goal_idx);
            Self::mark_reachable_from_goal(game, goal_pos, &mut reachable);
        }

        let mut unreachable = [[false; MAX_SIZE]; MAX_SIZE];
        for y in 0..game.height() {
            for x in 0..game.width() {
                unreachable[y as usize][x as usize] = !reachable[y as usize][x as usize];
            }
        }

        Deadlocks { unreachable }
    }

    fn mark_reachable_from_goal(
        game: &Game,
        goal_pos: (u8, u8),
        reachable: &mut [[bool; MAX_SIZE]; MAX_SIZE],
    ) {
        if reachable[goal_pos.1 as usize][goal_pos.0 as usize] {
            return;
        }

        let mut stack = Vec::new();

        // Start with box at goal position
        stack.push(goal_pos);
        reachable[goal_pos.1 as usize][goal_pos.0 as usize] = true;

        while let Some((box_x, box_y)) = stack.pop() {
            // Try all possible unpushes from this box position
            for direction in ALL_DIRECTIONS {
                if let Some((new_box_x, new_box_y)) = game.unmove_pos(box_x, box_y, direction) {
                    if let Some((player_x, player_y)) =
                        game.unmove_pos(new_box_x, new_box_y, direction)
                    {
                        // Check if new box position is reachable.
                        let new_box_tile = game.get_tile(new_box_x, new_box_y);
                        let player_tile = game.get_tile(player_x, player_y);
                        if (new_box_tile == Tile::Floor || new_box_tile == Tile::Goal)
                            && !reachable[new_box_y as usize][new_box_x as usize]
                            && (player_tile == Tile::Floor || player_tile == Tile::Goal)
                        {
                            reachable[new_box_y as usize][new_box_x as usize] = true;
                            stack.push((new_box_x, new_box_y));
                        }
                    }
                }
            }
        }
    }

    /// Returns true if the push leads to an unsolvable state.
    pub fn is_push_deadlock(&self, game: &Game, push: Push) -> bool {
        let box_pos = game.box_pos(push.box_index as usize);
        if let Some((dest_x, dest_y)) = game.move_pos(box_pos.0, box_pos.1, push.direction) {
            self.unreachable[dest_y as usize][dest_x as usize]
        } else {
            false
        }
    }

    /// Returns true if the unpush leads to an unsolvable state.
    pub fn is_unpush_deadlock(&self, _game: &Game, _push: Push) -> bool {
        false
    }
}
