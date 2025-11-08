mod game;
mod solver;

use game::Game;
use solver::Solver;

fn main() {
    let input = "####\n\
                     # .#\n\
                     #  ###\n\
                     #*@  #\n\
                     #  $ #\n\
                     #  ###\n\
                     ####";
    let game = Game::from_text(input).unwrap();
    let mut solver = Solver::new();

    solver.solve(&game);
}
