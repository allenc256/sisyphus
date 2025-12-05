<p align="center">
  <img width="400" height="400" alt="temp" src="https://github.com/user-attachments/assets/096231ec-7d5a-4e77-a966-c508ac5e708a" />
</p>

A program for solving the game of Sokoban. In contrast to my previous solver
[dum-dum](https://github.com/allenc256/dum-dum) for double-dummy Bridge hands,
this program was coded using AI assistance. It's served as a fun testbed for
vibe-coding something containing non-trivial logic. 

My findings have been that Claude Code (Sonnet 4.5) was exceptional for helping
with a lot of "easier" tasks (e.g., level parsing logic, basic game logic,
writing tests, basic search logic, basic heuristic logic, zobrist hashing), but
struggled on certain "harder" tasks. For example, Claude struggled with
understanding how to properly implement bidirectional search (though forward
search was fine) as well as more complex deadlock detection techniques. These
were most likely "out-of-distribution" for the model and had to be coded mostly
by hand.

In all, I'd say about 60% of the code was vibe-coded, and about 40% was
hand-written. Not bad! Overall, AI assistance felt like a big productivity
boost, and it wouldn't be surprising if future versions could hand the more
"harder" tasks as well!

## Building

This is a standard rust project, so just:

```
cargo build --release
```

## Usage

```
Usage: sisyphus [OPTIONS] <FILE> <LEVEL> [LEVEL_END]

Arguments:
  <FILE>       Path to the levels file (XSB format)
  <LEVEL>      Level number to solve (1-indexed), or start of range
  [LEVEL_END]  Optional end of level range (inclusive, 1-indexed)

Options:
  -p, --print-solution
          Print the solution step-by-step
  -n, --max-nodes <MAX_NODES>
          Maximum number of nodes to explore before giving up [default: 5000000]
  -H, --heuristic <HEURISTIC>
          Heuristic to use for solving [default: hungarian] [possible values: simple, greedy, hungarian, null]
  -d, --direction <DIRECTION>
          Search type [default: bidirectional] [possible values: forward, reverse, bidirectional]
      --no-freeze-deadlocks
          Disable freeze deadlock detection
      --no-dead-squares
          Disable dead square pruning
      --no-pi-corrals
          Disable PI-corral pruning
      --deadlock-max-nodes <DEADLOCK_MAX_NODES>
          Maximum nodes to explore when searching for corral deadlocks [default: 20]
  -t, --trace-range <TRACE_RANGE>
          Range of node counts to trace (e.g., "100..200", "100..=200", or "100")
  -h, --help
          Print help
```

The level format follows the standard XSB conventions (description
[here](http://sokobano.de/wiki/index.php?title=Level_format), or see [example
levels](levels/)). The current implementation has a few limitations it imposes
on levels:

* The maximum number of boxes in a level is 64.
* The maximum size of each level is 64x64.


### Example

```
$ sisyphus levels/microban.txt 1 10

level: 1    solved: Y  steps: 8      states: 16            elapsed: 0 ms
level: 2    solved: Y  steps: 3      states: 4             elapsed: 0 ms
level: 3    solved: Y  steps: 13     states: 20            elapsed: 0 ms
level: 4    solved: Y  steps: 7      states: 15            elapsed: 0 ms
level: 5    solved: Y  steps: 6      states: 11            elapsed: 0 ms
level: 6    solved: Y  steps: 31     states: 35            elapsed: 0 ms
level: 7    solved: Y  steps: 6      states: 8             elapsed: 0 ms
level: 8    solved: Y  steps: 32     states: 96            elapsed: 0 ms
level: 9    solved: Y  steps: 10     states: 22            elapsed: 0 ms
level: 10   solved: Y  steps: 21     states: 66            elapsed: 0 ms
---
solved:  10/10         steps: 137    states: 293           elapsed: 0 ms
```

## Technical Details

The solver implements the following search key techniques:

* **Bidirectional greedy search** - the solver simultaneously performs searching
  in both the forward and reverse directions, completing when the two searches
  intersect. The search is greedy / best-first.

* **Transposition table** - the solver implements a transposition table to avoid
  re-searching positions that have already been searched. The transposition
  table is keyed on 64-bit Zobrist hashes. Note that positions are canonicalized
  during search by placing the player in the top-left most reachable position.

* **Hungarian algorithm heuristic** - the solver uses the Hungarian algorithm to
  compute estimated distances between boxes and goals. There is a fallback to a
  simpler but faster algorithm when the number of boxes is too high and the
  O(n^3) running time of the Hungarian algorithm becomes prohibitive.

* **Frozen box detection** - the solver is capable of detecting frozen boxes
  during search. There are used in two ways: (1) any box which is frozen but not
  on a goal constitutes a deadlock (e.g., a "freeze deadlock"), and (2)
  non-deadlocked frozen boxes trigger dynamic recomputation of the heuristic
  since the frozen boxes can be treated like walls.

* **PI-corral pruning** - the solver performs simple PI-corral pruning, as
  described [here](http://sokobano.de/wiki/index.php?title=Solver#PI-Corrals).
  Corral deadlocks can similarly be detected.

The solver is powerful enough to easily solve simple levels (e.g.,
[Microban](http://www.abelmartin.com/rj/sokobanJS/Skinner/David%20W.%20Skinner%20-%20Sokoban.htm)
levels by David W. Skinner), but it certainly has not implemented some of the
more complex techniques used by SOTA solvers.

