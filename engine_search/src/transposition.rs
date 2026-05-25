use engine_core::{board::{BOARD_HEIGHT, COL_NB}, game::Game};

const ZOBRIST_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const GARBAGE_KEYS: usize = 256;

pub(crate) const DEFAULT_TT_SIZE: usize = 65_536;

#[derive(Clone)]
pub(crate) struct ZobristKeys {
    keys: [[u64; BOARD_HEIGHT]; COL_NB],
    garbage: [u64; GARBAGE_KEYS],
}

impl ZobristKeys {
    pub(crate) fn new() -> Self {
        let mut rng = SplitMix64::new(ZOBRIST_SEED);
        let mut keys = [[0u64; BOARD_HEIGHT]; COL_NB];
        for row in keys.iter_mut().take(COL_NB) {
            for key in row.iter_mut().take(BOARD_HEIGHT) {
                *key = rng.next_u64();
            }
        }

        let mut garbage = [0u64; GARBAGE_KEYS];
        for key in &mut garbage {
            *key = rng.next_u64();
        }

        Self { keys, garbage }
    }

    pub(crate) fn hash_game(&self, game: &Game) -> u64 {
        let mut hash = 0u64;
        for y in 0..BOARD_HEIGHT {
            let row = game.board.rows[y];
            for x in 0..COL_NB {
                if row & (1u16 << x) != 0 {
                    hash ^= self.keys[x][y];
                }
            }
        }

        let pending = game.pending_garbage as usize;
        hash ^ self.garbage[pending]
    }
}

impl Default for ZobristKeys {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn get_zobrist_keys() -> &'static ZobristKeys {
    use std::sync::OnceLock;
    static KEYS: OnceLock<ZobristKeys> = OnceLock::new();
    KEYS.get_or_init(ZobristKeys::new)
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
pub(crate) struct TTEntry {
    pub(crate) hash: u64,
    pub(crate) depth: u8,
    pub(crate) score: f64,
}

pub(crate) struct TranspositionTable {
    entries: Box<[TTEntry]>,
}

impl TranspositionTable {
    pub(crate) fn new(size: usize) -> Self {
        let size = size.max(1);
        Self {
            entries: vec![TTEntry::default(); size].into_boxed_slice(),
        }
    }

    #[inline]
    fn index(&self, hash: u64) -> usize {
        (hash as usize) % self.entries.len()
    }

    pub(crate) fn probe(&self, hash: u64, depth: u8) -> Option<f64> {
        let entry = self.entries[self.index(hash)];
        if entry.hash == hash && entry.depth >= depth {
            Some(entry.score)
        } else {
            None
        }
    }

    pub(crate) fn store(&mut self, hash: u64, depth: u8, score: f64) {
        let entry = &mut self.entries[self.index(hash)];
        if depth >= entry.depth {
            *entry = TTEntry { hash, depth, score };
        }
    }

    pub(crate) fn clear(&mut self) {
        self.entries.fill(TTEntry::default());
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(DEFAULT_TT_SIZE)
    }
}
