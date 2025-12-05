use crate::bits::{Bitvector, Index};
use crate::corral::{CorralResult, CorralSearcher};
use crate::frozen::{compute_frozen_boxes, compute_new_frozen_boxes};
use crate::game::{Checkpoint, Direction, Game, Move, Moves, Position, Pull, Push, ReachableSet};
use crate::heuristic::{Cost, Heuristic};
use crate::pqueue::PriorityQueue;
use crate::zobrist::Zobrist;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::ops::Range;
use std::rc::Rc;

/// Result of solving a puzzle
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveResult {
    /// Puzzle was solved
    Solved(Vec<Push>),
    /// Node limit exceeded before solution found
    Cutoff,
    /// Puzzle is impossible to solve
    Unsolvable,
}

/// Internal trait containing search logic that is polymorphic depending on the
/// direction of the search (forward vs reverse).
trait SearchHelper {
    type Move: Move;

    fn compute_moves(&self, game: &Game) -> ReachableSet<Self::Move>;
    fn compute_unmoves(&self, game: &Game) -> Moves<Self::Move>;

    fn apply_move(&self, game: &mut Game, move_: &Self::Move);
    fn apply_unmove(&self, game: &mut Game, move_: &Self::Move);

    fn is_dead_square(&self, game: &Game, pos: Position) -> bool;

    fn search_corrals(
        &mut self,
        game: &mut Game,
        reachable: &ReachableSet<Self::Move>,
    ) -> CorralResult<Self::Move>;

    fn compute_frozen_boxes(&self, game: &Game) -> Bitvector;
    fn compute_new_frozen_boxes(
        &self,
        frozen: &Bitvector,
        game: &Game,
        box_idx: Index,
    ) -> Bitvector;

    fn new_heuristic<H: Heuristic>(&self, game: &Game, frozen_boxes: Bitvector) -> H;

    fn to_push_by_pos(&self, game: &Game, move_: &Self::Move) -> PushByPos;
}

struct ForwardSearchHelper {
    corral_searcher: CorralSearcher,
    freeze_deadlocks: bool,
    dead_squares: bool,
    pi_corrals: bool,
}

struct ReverseSearchHelper {
    dead_squares: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchType {
    Forward,
    Reverse,
    Bidirectional,
}

#[derive(Debug, Copy, Clone)]
struct PushByPos {
    box_pos: Position,
    direction: Direction,
}

impl SearchHelper for ForwardSearchHelper {
    type Move = Push;

    fn compute_moves(&self, game: &Game) -> ReachableSet<Push> {
        game.compute_pushes()
    }

    fn compute_unmoves(&self, game: &Game) -> Moves<Push> {
        game.compute_pulls().moves.to_pushes()
    }

    fn apply_move(&self, game: &mut Game, push: &Push) {
        game.push(*push);
    }

    fn apply_unmove(&self, game: &mut Game, push: &Push) {
        game.pull(push.to_pull());
    }

    fn is_dead_square(&self, game: &Game, pos: Position) -> bool {
        if self.dead_squares {
            game.is_push_dead_square(pos)
        } else {
            false
        }
    }

    fn search_corrals(
        &mut self,
        game: &mut Game,
        reachable: &ReachableSet<Self::Move>,
    ) -> CorralResult<Self::Move> {
        if self.pi_corrals {
            self.corral_searcher.search(game, reachable)
        } else {
            CorralResult::None
        }
    }

    fn compute_frozen_boxes(&self, game: &Game) -> Bitvector {
        if self.freeze_deadlocks {
            compute_frozen_boxes(game)
        } else {
            Bitvector::new()
        }
    }

    fn compute_new_frozen_boxes(
        &self,
        frozen: &Bitvector,
        game: &Game,
        box_idx: Index,
    ) -> Bitvector {
        if self.freeze_deadlocks {
            compute_new_frozen_boxes(*frozen, game, box_idx)
        } else {
            Bitvector::new()
        }
    }

    fn new_heuristic<H: Heuristic>(&self, game: &Game, frozen_boxes: Bitvector) -> H {
        H::new_push(game, frozen_boxes)
    }

    fn to_push_by_pos(&self, game: &Game, push: &Push) -> PushByPos {
        PushByPos {
            box_pos: game.box_position(push.box_index()),
            direction: push.direction(),
        }
    }
}

impl SearchHelper for ReverseSearchHelper {
    type Move = Pull;

    fn compute_moves(&self, game: &Game) -> ReachableSet<Pull> {
        game.compute_pulls()
    }

    fn compute_unmoves(&self, game: &Game) -> Moves<Pull> {
        game.compute_pushes().moves.to_pulls()
    }

    fn apply_move(&self, game: &mut Game, pull: &Pull) {
        game.pull(*pull);
    }

    fn apply_unmove(&self, game: &mut Game, pull: &Pull) {
        game.push(pull.to_push())
    }

    fn is_dead_square(&self, game: &Game, pos: Position) -> bool {
        if self.dead_squares {
            game.is_pull_dead_square(pos)
        } else {
            false
        }
    }

    fn search_corrals(
        &mut self,
        _game: &mut Game,
        _reachable: &ReachableSet<Self::Move>,
    ) -> CorralResult<Self::Move> {
        CorralResult::None
    }

    fn compute_frozen_boxes(&self, _game: &Game) -> Bitvector {
        Bitvector::new()
    }

    fn compute_new_frozen_boxes(
        &self,
        _frozen: &Bitvector,
        _game: &Game,
        _box_idx: Index,
    ) -> Bitvector {
        Bitvector::new()
    }

    fn new_heuristic<H: Heuristic>(&self, game: &Game, frozen_boxes: Bitvector) -> H {
        H::new_pull(game, frozen_boxes)
    }

    fn to_push_by_pos(&self, game: &Game, pull: &Pull) -> PushByPos {
        let new_box_pos = game.box_position(pull.box_index());
        let old_box_pos = game.move_position(new_box_pos, pull.direction()).unwrap();
        PushByPos {
            box_pos: old_box_pos,
            direction: pull.direction().reverse(),
        }
    }
}

struct Node {
    checkpoint: Checkpoint,
    frozen_boxes: Bitvector,
}

struct TableEntry {
    parent_hash: u64,
    is_closed: bool,
}

struct Searcher<H, S> {
    game: Game,
    open_list: PriorityQueue<Node>,
    table: HashMap<u64, TableEntry>,
    zobrist: Rc<Zobrist>,
    heuristic: HashMap<u64, H>,
    helper: S,
}

enum ExpandNode {
    NotDone,
    Solved,
    Unsolvable,
}

impl<H: Heuristic, S: SearchHelper> Searcher<H, S> {
    fn new(
        game: &Game,
        zobrist: Rc<Zobrist>,
        initial_player_positions: &[Position],
        helper: S,
    ) -> Self {
        let mut open_list = PriorityQueue::new();
        let mut table = HashMap::new();
        let mut heuristic: HashMap<u64, H> = HashMap::new();
        let mut game = game.clone();

        // Loop through initial positions
        for &pos in initial_player_positions {
            // Set initial position
            game.set_player(pos);

            // Compute frozen boxes
            let frozen_boxes = helper.compute_frozen_boxes(&game);

            // Compute initial cost
            let frozen_boxes_hash = zobrist.compute_boxes_hash_subset(&game, frozen_boxes);
            let cost = heuristic
                .entry(frozen_boxes_hash)
                .or_insert_with(|| helper.new_heuristic(&game, frozen_boxes))
                .compute(&game);
            if cost == Cost::INFINITE {
                continue;
            }

            // Insert into open_list
            open_list.push(
                usize::from(cost),
                Node {
                    checkpoint: game.checkpoint(),
                    frozen_boxes,
                },
            );

            // Insert into transposition table
            table.insert(
                zobrist.compute_hash(&game),
                TableEntry {
                    parent_hash: 0,
                    is_closed: false,
                },
            );
        }

        Self {
            game,
            open_list,
            table,
            zobrist,
            heuristic,
            helper,
        }
    }

    fn expand_node<H2, S2>(&mut self, other_searcher: &Searcher<H2, S2>) -> ExpandNode {
        // Pop next node from open list
        let node = self.open_list.pop_min();
        if node.is_none() {
            // We've exhaused the open list
            return ExpandNode::Unsolvable;
        }
        let node = node.unwrap();

        // Restore the node's checkpoint
        self.game.restore(&node.checkpoint);

        // Compute reachable set
        let reachable = self.helper.compute_moves(&self.game);

        // Compute hash
        let boxes_hash = self.zobrist.compute_boxes_hash(&self.game);
        let player_hash = self.zobrist.player_hash(self.game.player());
        let uncanonical_hash = boxes_hash ^ player_hash;

        // Check tranposition table for uncanonical hash
        let entry = self.table.get_mut(&uncanonical_hash).unwrap();
        if entry.is_closed {
            // Someone else closed this node
            return ExpandNode::NotDone;
        } else {
            // Mark node as closed
            entry.is_closed = true;
        }
        let parent_hash = entry.parent_hash;

        // Compute canonical hash
        let canonical_player_pos = reachable.squares.top_left().unwrap();
        let canonical_player_hash = self.zobrist.player_hash(canonical_player_pos);
        let canonical_hash = boxes_hash ^ canonical_player_hash;

        // Check transposition table for canonical hash
        if canonical_hash != uncanonical_hash {
            match self.table.entry(canonical_hash) {
                Entry::Occupied(mut e) => {
                    let e = e.get_mut();
                    if e.is_closed {
                        // Someone else closed this node
                        return ExpandNode::NotDone;
                    } else {
                        // Mark node as closed
                        e.is_closed = true;
                    }
                }
                Entry::Vacant(e) => {
                    // Otherwise, insert a closed node
                    e.insert(TableEntry {
                        parent_hash,
                        is_closed: true,
                    });
                }
            }
        }

        // Check if we've hit the other side
        if other_searcher.table.contains_key(&canonical_hash) {
            return ExpandNode::Solved;
        }

        // Apply PI-corral pruning
        let moves = match self.helper.search_corrals(&mut self.game, &reachable) {
            CorralResult::Prune(pruned_moves) => pruned_moves,
            CorralResult::None => reachable.moves,
            CorralResult::Deadlocked => return ExpandNode::NotDone,
        };

        // Try each move
        for move_ in &moves {
            // Make sure we're not trying to push a frozen box
            if node.frozen_boxes.contains(move_.box_index()) {
                continue;
            }

            let old_box_pos = self.game.box_position(move_.box_index());
            let new_box_pos = self
                .game
                .move_position(old_box_pos, move_.direction())
                .unwrap();

            // Apply dead square pruning
            if self.helper.is_dead_square(&self.game, new_box_pos) {
                continue;
            }

            // Apply move
            self.helper.apply_move(&mut self.game, &move_);

            // Compute newly frozen boxes
            let new_frozen = self.helper.compute_new_frozen_boxes(
                &node.frozen_boxes,
                &self.game,
                move_.box_index(),
            );
            let child_frozen_boxes = node.frozen_boxes.union(&new_frozen);

            // Apply frozen box deadlock pruning
            if self.game.unsolved_boxes().contains_any(&child_frozen_boxes) {
                self.helper.apply_unmove(&mut self.game, &move_);
                continue;
            }

            // Compute child hash
            let child_boxes_hash = boxes_hash
                ^ self.zobrist.box_hash(old_box_pos)
                ^ self.zobrist.box_hash(new_box_pos);
            let child_hash = child_boxes_hash ^ self.zobrist.player_hash(self.game.player());

            // Check the transposition table
            match self.table.entry(child_hash) {
                Entry::Occupied(_) => {
                    // This node was already visited before, skip
                    self.helper.apply_unmove(&mut self.game, &move_);
                    continue;
                }
                Entry::Vacant(e) => {
                    // Insert an open node
                    e.insert(TableEntry {
                        parent_hash: canonical_hash,
                        is_closed: false,
                    });
                }
            };

            // Compute child cost using appropriate heuristic
            let frozen_hash = self
                .zobrist
                .compute_boxes_hash_subset(&self.game, child_frozen_boxes);
            let child_cost = self
                .heuristic
                .entry(frozen_hash)
                .or_insert_with(|| {
                    self.helper
                        .new_heuristic::<H>(&self.game, child_frozen_boxes)
                })
                .compute(&self.game);

            // If unsolvable, skip
            if child_cost == Cost::INFINITE {
                self.helper.apply_unmove(&mut self.game, &move_);
                continue;
            }

            // Insert into open list
            self.open_list.push(
                usize::from(child_cost),
                Node {
                    checkpoint: self.game.checkpoint(),
                    frozen_boxes: child_frozen_boxes,
                },
            );

            // Unapply move
            self.helper.apply_unmove(&mut self.game, &move_);
        }

        ExpandNode::NotDone
    }

    fn reconstruct_solution(&self) -> Vec<PushByPos> {
        let mut solution = Vec::new();
        let mut current_game = self.game.clone();
        let mut current_hash = self.zobrist.compute_hash(&current_game);

        // Work backwards until we reach an initial state (parent_hash == 0)
        loop {
            let entry = self
                .table
                .get(&current_hash)
                .expect("Failed to reconstruct solution: state not in transposition table");

            if entry.parent_hash == 0 {
                // Reached an initial state
                break;
            }

            // Compute all possible unmoves from current state
            let unmoves = self.helper.compute_unmoves(&current_game);

            // Try each unmove to find which one leads to parent state
            let mut found = false;
            for unmove in &unmoves {
                self.helper.apply_unmove(&mut current_game, &unmove);

                // Compute hash of this previous state
                let prev_hash = self.zobrist.compute_hash(&current_game);

                // Check if this matches the parent we're looking for
                if prev_hash == entry.parent_hash {
                    solution.push(self.helper.to_push_by_pos(&current_game, &unmove));
                    current_hash = prev_hash;
                    found = true;
                    break;
                }

                // Redo the unmove if it wasn't correct
                self.helper.apply_move(&mut current_game, &unmove);
            }

            assert!(
                found,
                "Failed to reconstruct solution: no unmove leads to parent state"
            );
        }

        solution
    }
}

pub struct Solver<H> {
    forward: Searcher<H, ForwardSearchHelper>,
    reverse: Searcher<H, ReverseSearchHelper>,
    game: Game,
    opts: SolverOpts,
}

pub struct SolverOpts {
    pub search_type: SearchType,
    pub max_nodes_explored: usize,
    pub freeze_deadlocks: bool,
    pub dead_squares: bool,
    pub pi_corrals: bool,
    pub deadlock_max_nodes: usize,
    pub trace_range: Range<usize>,
}

impl<H: Heuristic> Solver<H> {
    pub fn new(game: &Game, opts: SolverOpts) -> Self {
        let zobrist = Rc::new(Zobrist::new());
        let reverse_game = game.swap_boxes_and_goals();
        let forward_player_positions = [game.canonical_player_pos()];
        let reverse_player_positions = reverse_game.all_possible_player_positions();

        let forward_helper = ForwardSearchHelper {
            corral_searcher: CorralSearcher::new(zobrist.clone(), opts.deadlock_max_nodes),
            dead_squares: opts.dead_squares,
            pi_corrals: opts.pi_corrals,
            freeze_deadlocks: opts.freeze_deadlocks,
        };
        let reverse_helper = ReverseSearchHelper {
            dead_squares: opts.dead_squares,
        };

        let forward_searcher = Searcher::new(
            game,
            zobrist.clone(),
            &forward_player_positions,
            forward_helper,
        );
        let reverse_searcher = Searcher::new(
            &reverse_game,
            zobrist,
            &reverse_player_positions,
            reverse_helper,
        );

        Self {
            forward: forward_searcher,
            reverse: reverse_searcher,
            game: game.clone(),
            opts,
        }
    }

    pub fn solve(&mut self) -> (SolveResult, usize) {
        let mut nodes_explored = 0;
        let result;

        loop {
            let is_forward = match self.opts.search_type {
                SearchType::Forward => true,
                SearchType::Reverse => false,
                // TODO: try being greedy between the two sides
                SearchType::Bidirectional => nodes_explored % 2 == 0,
            };

            let expand_node = if is_forward {
                self.forward.expand_node(&self.reverse)
            } else {
                self.reverse.expand_node(&self.forward)
            };

            match expand_node {
                ExpandNode::NotDone => {
                    nodes_explored += 1;
                    if nodes_explored >= self.opts.max_nodes_explored {
                        result = SolveResult::Cutoff;
                        break;
                    }
                }
                ExpandNode::Solved => {
                    if is_forward {
                        self.reverse.game.restore(&self.forward.game.checkpoint());
                    } else {
                        self.forward.game.restore(&self.reverse.game.checkpoint());
                    }
                    let soln = self.reconstruct_solution();
                    result = SolveResult::Solved(soln);
                    break;
                }
                ExpandNode::Unsolvable => {
                    result = SolveResult::Unsolvable;
                    break;
                }
            }

            if self.opts.trace_range.contains(&nodes_explored) {
                let (dir, game) = if is_forward {
                    ("forward", &self.forward.game)
                } else {
                    ("reverse", &self.reverse.game)
                };
                println!("direction={} count={}:\n{}", dir, nodes_explored, game);
            }
        }

        (result, nodes_explored)
    }

    fn reconstruct_solution(&self) -> Vec<Push> {
        let forward_soln = self.forward.reconstruct_solution();
        let reverse_soln = self.reverse.reconstruct_solution();
        self.combine_solution(&forward_soln, &reverse_soln)
    }

    fn combine_solution(
        &self,
        forward_soln: &[PushByPos],
        reverse_soln: &[PushByPos],
    ) -> Vec<Push> {
        let mut game = self.game.clone();
        let mut soln = Vec::new();
        let chained = forward_soln.iter().rev().chain(reverse_soln.iter());

        for (i, push_by_pos) in chained.enumerate() {
            // Get box index at this position
            let box_index = game.box_index(push_by_pos.box_pos).unwrap_or_else(|| {
                panic!(
                    "Solution verification failed: no box at position {} for push {}",
                    push_by_pos.box_pos,
                    i + 1
                )
            });

            // Compute valid pushes at this state
            let valid_pushes = game.compute_pushes().moves;

            // Verify that this push is among the valid pushes
            let push = Push::new(box_index, push_by_pos.direction);
            assert!(
                valid_pushes.contains(push),
                "Solution verification failed: push {} (box at {}, direction {:?}) is not valid",
                i + 1,
                push_by_pos.box_pos,
                push_by_pos.direction
            );

            // Apply the push
            game.push(push);
            soln.push(push);
        }

        // Verify final state is solved
        assert!(
            game.is_solved(),
            "Solution verification failed: puzzle is not solved"
        );

        soln
    }
}

#[cfg(test)]
mod tests {
    use crate::heuristic::SimpleHeuristic;

    use super::*;

    #[test]
    fn test_solve_simple() {
        let game = parse_game(
            r#"
#####
#@$.#
#####
"#,
        );
        let mut solver = new_solver(game.clone());
        let result = solver.solve();

        if let (SolveResult::Solved(soln), _) = result {
            assert_eq!(soln.len(), 1);

            // Verify solution works
            let mut test_game = game.clone();
            for push in soln {
                test_game.push(push);
            }
            assert!(test_game.is_solved());
        } else {
            panic!();
        }
    }

    #[test]
    fn test_solve_already_solved() {
        let game = parse_game(
            r#"
####
#@*#
####
"#,
        );
        let mut solver = new_solver(game);
        let result = solver.solve();

        if let (SolveResult::Solved(moves), _) = result {
            assert_eq!(moves.len(), 0);
        } else {
            panic!();
        }
    }

    #[test]
    fn test_solve_two_moves() {
        let game = parse_game(
            r#"
######
#@$ .#
######
"#,
        );
        let mut solver = new_solver(game.clone());
        let result = solver.solve();

        if let (SolveResult::Solved(soln), _) = result {
            assert_eq!(soln.len(), 2);

            // Verify solution works
            let mut test_game = game.clone();
            for push in soln {
                test_game.push(push);
            }
            assert!(test_game.is_solved());
        } else {
            panic!();
        }
    }

    #[test]
    fn test_solve_impossible() {
        let game = parse_game(
            r#"
#######
#@$ #.#
#######
"#,
        );
        let mut solver = new_solver(game);
        let result = solver.solve();
        assert_eq!(result.0, SolveResult::Unsolvable);
    }

    fn parse_game(text: &str) -> Game {
        Game::from_text(text.trim_matches('\n')).unwrap()
    }

    fn new_solver(game: Game) -> Solver<SimpleHeuristic> {
        Solver::new(
            &game,
            SolverOpts {
                search_type: SearchType::Forward,
                max_nodes_explored: 10000,
                freeze_deadlocks: true,
                dead_squares: true,
                pi_corrals: true,
                deadlock_max_nodes: 1000,
                trace_range: 0..0,
            },
        )
    }
}
