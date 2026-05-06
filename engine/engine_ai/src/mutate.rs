use rand::{RngExt, rng};

use crate::model::Model;

pub trait Mutateable: Default {
    fn generate(sub_name: String) -> Self;

    fn crossover(parent1: &Self, parent2: &Self, sub_name: String) -> Self;

    fn name(&self) -> &str;
}

impl Mutateable for Model {
    fn generate(name: String) -> Self {
        let a = || rng().random_range(-999..1000);
        Model {
            back_to_back: a(),
            bumpiness: a(),
            bumpiness_sq: a(),
            row_transitions: a(),
            height: a(),
            top_half: a(),
            top_quarter: a(),
            jeopardy: a(),
            cavity_cells: a(),
            cavity_cells_sq: a(),
            overhang_cells: a(),
            overhang_cells_sq: a(),
            covered_cells: a(),
            covered_cells_sq: a(),
            well_depth: a(),
            max_well_depth: a(),
            well_column: [a(); 10],
            b2b_clear: a(),
            clear: [a(); 4],
            spin: [a(); 4],
            spin_mini: [a(); 4],
            perfect_clear: a(),
            combo_garbage: a(),
            waste: [a(); 7],
            incoming_garbage: a(),
            outgoing_garbage: a(),
            b2b_cap: a(),
            broke_surge: a(),
            name,
        }
    }

    fn crossover(parent1: &Self, parent2: &Self, name: String) -> Self {
        Self {
            back_to_back: crossover_gene(parent1.back_to_back, parent2.back_to_back),
            bumpiness: crossover_gene(parent1.bumpiness, parent2.bumpiness),
            bumpiness_sq: crossover_gene(parent1.bumpiness_sq, parent2.bumpiness_sq),
            row_transitions: crossover_gene(parent1.row_transitions, parent2.row_transitions),
            height: crossover_gene(parent1.height, parent2.height),
            top_half: crossover_gene(parent1.top_half, parent2.top_half),
            top_quarter: crossover_gene(parent1.top_quarter, parent2.top_quarter),
            jeopardy: crossover_gene(parent1.jeopardy, parent2.jeopardy),
            cavity_cells: crossover_gene(parent1.cavity_cells, parent2.cavity_cells),
            cavity_cells_sq: crossover_gene(parent1.cavity_cells_sq, parent2.cavity_cells_sq),
            overhang_cells: crossover_gene(parent1.overhang_cells, parent2.overhang_cells),
            overhang_cells_sq: crossover_gene(parent1.overhang_cells_sq, parent2.overhang_cells_sq),
            covered_cells: crossover_gene(parent1.covered_cells, parent2.covered_cells),
            covered_cells_sq: crossover_gene(parent1.covered_cells_sq, parent2.covered_cells_sq),
            well_depth: crossover_gene(parent1.well_depth, parent2.well_depth),
            max_well_depth: crossover_gene(parent1.max_well_depth, parent2.max_well_depth),
            well_column: crossover_many_genes(parent1.well_column, parent2.well_column),
            b2b_clear: crossover_gene(parent1.b2b_clear, parent2.b2b_clear),
            clear: crossover_many_genes(parent1.clear, parent2.clear),
            spin: crossover_many_genes(parent1.spin, parent2.spin),
            spin_mini: crossover_many_genes(parent1.spin_mini, parent2.spin_mini),
            perfect_clear: crossover_gene(parent1.perfect_clear, parent2.perfect_clear),
            combo_garbage: crossover_gene(parent1.combo_garbage, parent2.combo_garbage),
            waste: crossover_many_genes(parent1.waste, parent2.waste),
            incoming_garbage: crossover_gene(parent1.incoming_garbage, parent2.incoming_garbage),
            outgoing_garbage: crossover_gene(parent1.outgoing_garbage, parent2.outgoing_garbage),
            b2b_cap: crossover_gene(parent1.b2b_cap, parent2.b2b_cap),
            broke_surge: crossover_gene(parent1.broke_surge, parent2.broke_surge),
            name,
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn crossover_gene(v1: i32, v2: i32) -> i32 {
    let v = match rng().random_range(0..100) {
        0..=41 => v1,             // 42%
        42..=83 => v2,            // 42%
        84..=98 => (v1 + v2) / 2, // 15%
        _ => rng().random_range(-999..1000),
    } + rng().random_range(-10..11);
    if v < -999 {
        -999
    } else if v > 999 {
        999
    } else {
        v
    }
}

fn crossover_many_genes<const N: usize>(parent1: [i32; N], parent2: [i32; N]) -> [i32; N] {
    let mut result = [0; N];
    for i in 0..N {
        result[i] = crossover_gene(parent1[i], parent2[i]);
    }
    result
}
