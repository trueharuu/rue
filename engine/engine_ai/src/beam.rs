use std::cmp::Reverse;
use std::collections::BinaryHeap;

use engine_core::{piece::Mino, piece_location::PieceLocation};
use engine_nav::{game::Game, movegen::movegen};
use rayon::prelude::*;

use crate::{
    model::Model,
    reward::{Reward, Value},
};

#[derive(Clone, Debug)]
pub struct Node {
    pub item: Game,
    pub id: usize,
    pub value: Value,
    pub reward: Reward,
}

impl Node {
    pub fn score(&self) -> Value {
        self.value + self.reward
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.score() == other.score()
    }
}

impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.score().cmp(&other.score()))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score().cmp(&other.score())
    }
}

pub struct Beam<'a> {
    pub root: &'a Game,
    pub model: &'a Model,
    pub depth: usize,
    pub width: usize,
}

impl<'a> Beam<'a> {
    pub fn new(root: &'a Game, eval: &'a Model, depth: usize, width: usize) -> Self {
        Self {
            root,
            model: eval,
            depth,
            width,
        }
    }

    pub fn search(&self, queue: &[Mino]) -> Option<PieceLocation> {
        // keep a min-heap of the best `width` nodes by wrapping Node in Reverse
        let mut heap = BinaryHeap::<Reverse<Node>>::with_capacity(self.width + 1);
        let mut searched = vec![];

        assert!(queue.len() as usize <= self.width);
        assert!(queue.len() > 0);

        let mut arena = vec![];
        movegen(
            &mut arena,
            &self.root.board,
            queue[0],
            Some(self.root.hold.unwrap_or_else(|| queue[1])),
            true,
        );
        searched = arena.clone();

        let initial_nodes: Vec<Node> = arena
            .par_iter()
            .enumerate()
            .filter_map(|(id, child)| {
                let mut game = self.root.clone();
                let pi = game.advance(queue[0], child);

                if queue.len() > 1 && !game.can_spawn_piece(queue[1]) {
                    return None;
                }

                let (value, reward) = self.model.evaluate(&game, &pi);
                Some(Node {
                    item: game,
                    id,
                    value,
                    reward,
                })
            })
            .collect();

        heap = self.collect_top_k(initial_nodes);

        for idx in 1..self.depth {
            if heap.is_empty() {
                break;
            }

            let current_nodes: Vec<Node> = heap.iter().map(|r| r.0.clone()).collect();
            let next_piece = queue.get(idx + 1).copied();

            let layer_nodes: Vec<Node> = current_nodes
                .par_iter()
                .flat_map(|node| {
                    let current_piece = queue.get(idx).copied().or(node.item.hold);
                    let Some(c) = current_piece else {
                        return Vec::new();
                    };

                    let mut arena = Vec::new();
                    let start = movegen(
                        &mut arena,
                        &node.item.board,
                        c,
                        node.item.hold.or(next_piece),
                        true,
                    );

                    arena[start..]
                        .iter()
                        .filter_map(|loc| {
                            let mut game = node.item.clone();
                            let pi = game.advance(c, loc);

                            if let Some(n) = next_piece
                                && !game.can_spawn_piece(n)
                            {
                                return None;
                            }

                            let (value, reward) = self.model.evaluate(&game, &pi);
                            Some(Node {
                                item: game,
                                id: node.id,
                                value,
                                reward,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            heap = self.collect_top_k(layer_nodes);
        }

        // pick the best node (highest score) from the heap
        if let Some(best) = heap
            .into_iter()
            .map(|r| r.0)
            .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap())
        {
            return Some(searched[best.id].clone());
        }

        None
    }

    pub fn insert_if_better(&self, heap: &mut BinaryHeap<Reverse<Node>>, node: Node) {
        if heap.len() < self.width {
            heap.push(Reverse(node));
        } else if let Some(worst) = heap.peek() {
            // worst is the smallest node due to Reverse wrapper
            if node.score() > worst.0.score() {
                heap.pop();
                heap.push(Reverse(node));
            }
        }
    }

    fn collect_top_k(&self, nodes: Vec<Node>) -> BinaryHeap<Reverse<Node>> {
        let mut heap = BinaryHeap::<Reverse<Node>>::with_capacity(self.width + 1);
        for node in nodes {
            self.insert_if_better(&mut heap, node);
        }
        heap
    }
}
