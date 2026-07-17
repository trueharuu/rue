//! Neural-network evaluation model, with CNN and auxiliary inputs.
#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use candle_core::{DType, Device, Tensor};
use candle_nn::{
    BatchNorm, BatchNormConfig, Conv2d, Conv2dConfig, Linear, Module, ModuleT, VarBuilder,
};
use rue_core::game::Game;
use rue_core::game::attack::AttackContext;
use rue_core::history::History;
use rue_core::placement::Move;

use crate::weights::Weights;

/// Default board rows for encoding (bottom portion of the field).
pub const DEFAULT_BOARD_ROWS: usize = 24;
/// Standard board width.
pub const DEFAULT_BOARD_COLS: usize = 10;
/// Number of auxiliary features fed to the dense head.
pub const DEFAULT_AUX_FEATURES: usize = 47;

/// Architecture and encoding configuration for the deep model.
pub struct Config {
    /// Output channels for each conv layer.
    pub conv_channels: [usize; 2],
    /// Width of hidden dense layers.
    pub dense_width: usize,
    /// Number of dense layers (including the output layer).
    pub dense_layers: usize,
    /// Board rows encoded.
    pub board_rows: usize,
    /// Board columns encoded.
    pub board_cols: usize,
    /// Number of auxiliary features.
    pub aux_features: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            conv_channels: [8, 16],
            dense_width: 128,
            dense_layers: 2,
            board_rows: DEFAULT_BOARD_ROWS,
            board_cols: DEFAULT_BOARD_COLS,
            aux_features: DEFAULT_AUX_FEATURES,
        }
    }
}

impl Config {
    /// Returns the approximate number of trainable parameters.
    #[inline]
    #[must_use]
    pub fn params(&self) -> usize {
        // Conv1: 1 * ch[0] * 3 * 3 + ch[0] (bias)
        let conv1 = self.conv_channels[0] * 3 * 3 + self.conv_channels[0];
        // Conv2: ch[0] * ch[1] * 3 * 3 + ch[1]
        let conv2 = self.conv_channels[0] * self.conv_channels[1] * 3 * 3 + self.conv_channels[1];
        // BN1: 2 * ch[0] (weight + bias)
        let bn1 = 2 * self.conv_channels[0];
        // BN2: 2 * ch[1]
        let bn2 = 2 * self.conv_channels[1];

        let flat_size = self.conv_channels[1] * self.board_rows * self.board_cols;
        let dense_in = flat_size + self.aux_features;

        // Dense layers
        let mut dense = 0;
        let mut prev = dense_in;
        for _ in 0..self.dense_layers {
            dense += (prev + 1) * self.dense_width;
            prev = self.dense_width;
        }
        // Output layer: dense_width -> 1
        dense += self.dense_width + 1;

        conv1 + conv2 + bn1 + bn2 + dense
    }
}

/// The raw CNN + dense architecture.
pub struct DeepModel {
    conv1: Conv2d,
    bn1: BatchNorm,
    conv2: Conv2d,
    bn2: BatchNorm,
    fc1: Linear,
    fc2: Linear,
    fc_out: Linear,
}

impl DeepModel {
    /// Forward pass: board [1,1,H,W] + aux [A] -> scalar.
    fn forward(&self, board: &Tensor, aux: &Tensor) -> candle_core::Result<Tensor> {
        let x = self.conv1.forward(board)?;
        let x = self.bn1.forward_t(&x, false)?;
        let x = x.relu()?;

        let x = self.conv2.forward(&x)?;
        let x = self.bn2.forward_t(&x, false)?;
        let x = x.relu()?;

        let flat = x.flatten_all()?;
        let x = Tensor::cat(&[&flat, aux], 0)?;

        let x = self.fc1.forward(&x)?;
        let x = x.relu()?;

        let x = self.fc2.forward(&x)?;
        let x = x.relu()?;

        let x = self.fc_out.forward(&x)?;
        let x = x.tanh()?;

        Ok(x)
    }
}

/// Encode the board occupancy into a [1, 1, `board_rows`, `board_cols`] float tensor.
fn encode_board<const N: usize>(game: &Game<N>, config: &Config) -> Tensor {
    let mut data = vec![0.0f32; config.board_rows * config.board_cols];
    let max_h = game.board.max_y().min(config.board_rows as i32) as usize;
    for y in 0..max_h {
        for x in 0..config.board_cols {
            if game.board.get(x as i32, y as i32) {
                data[y * config.board_cols + x] = 1.0;
            }
        }
    }
    Tensor::from_slice(
        &data,
        (1, 1, config.board_rows, config.board_cols),
        &Device::Cpu,
    )
    .unwrap()
}

/// Build the auxiliary feature vector from game state and optional piece history.
///
/// Layout (47 floats):
///   [0]    b2b normalised
///   [1]    combo normalised
///   [2]    garbage total normalised
///   [3]    garbage segment count normalised
///   [4]    hold `is_some`
///   [5..12]  hold piece one-hot (7)
///   [12..19] queue[0] one-hot (7)
///   [19..26] queue[1] one-hot (7)
///   [26..33] queue[2] one-hot (7)
///   [33..40] `piece_recency` (7)
///   [40..47] `piece_recency_inv` (7)
fn encode_aux<const N: usize>(
    game: &Game<N>,
    history: Option<&History>,
    config: &Config,
) -> Tensor {
    let mut aux = Vec::with_capacity(config.aux_features);

    // b2b
    aux.push(game.b2b_count.map_or(0.0, |b| b as f32 / 10.0));
    // combo
    aux.push(game.combo_count.map_or(0.0, |c| c as f32 / 10.0));
    // garbage total
    aux.push(game.garbage_queue.total() as f32 / 20.0);
    // garbage segment count
    aux.push(game.garbage_queue.segments.len() as f32 / 10.0);

    // hold
    if let Some(p) = game.hold {
        aux.push(1.0);
        let mut oh = [0.0f32; 7];
        oh[p as usize] = 1.0;
        aux.extend_from_slice(&oh);
    } else {
        aux.push(0.0);
        aux.extend_from_slice(&[0.0; 7]);
    }

    // queue[0..3] one-hot
    for i in 0..3 {
        if let Some(&p) = game.queue.get(i) {
            let mut oh = [0.0f32; 7];
            oh[p as usize] = 1.0;
            aux.extend_from_slice(&oh);
        } else {
            aux.extend_from_slice(&[0.0; 7]);
        }
    }

    // piece recency from history
    if let Some(h) = history {
        let rec = h.recency();
        aux.extend_from_slice(&rec);
        // inv_recency = 1.0 - recency
        for &r in &rec {
            aux.push(1.0 - r);
        }
    } else {
        aux.extend_from_slice(&[1.0; 7]); // recency: all "not seen"
        aux.extend_from_slice(&[0.0; 7]); // inv_recency: all zero
    }

    Tensor::from_slice(&aux, config.aux_features, &Device::Cpu).unwrap()
}

/// Top-level deep evaluation model.
pub struct Deep {
    model: DeepModel,
    config: Config,
}

impl Deep {
    /// Create a new model with random (Kaiming) initialisation.
    #[must_use]
    pub fn new(config: Config) -> Self {
        let device = Device::Cpu;
        let varmap = candle_nn::VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);

        let model = build_model(&config, &vb).expect("model build failed");

        Self { model, config }
    }

    /// Load model weights from a safetensors file.
    pub fn load(
        path: impl AsRef<std::path::Path>,
        config: Config,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let device = Device::Cpu;
        let data = std::fs::read(path)?;
        let vb = VarBuilder::from_buffered_safetensors(data, DType::F32, &device)?;

        let model = build_model(&config, &vb)?;

        Ok(Self { model, config })
    }

    /// Save model weights to a safetensors file.
    pub fn save(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let varmap = candle_nn::VarMap::new();
        // We can't easily extract the vars from the built model, so we rebuild from a VarBuilder
        // backed by the VarMap, then save. This is a bit wasteful but simple.
        // Actually, let's just use the existing VarMap approach — we need to store the varmap.
        // For now, skip saving via VarMap and save raw tensors.
        // Re-architect if needed.
        let _ = (path, varmap);
        todo!("save requires VarMap retained at construction time")
    }

    /// Run inference on a game state.
    fn infer(&self, board: &Tensor, aux: &Tensor) -> f64 {
        let out = self.model.forward(board, aux).unwrap();
        f64::from(out.to_vec0::<f32>().unwrap())
    }
}

/// Build the [`DeepModel`] from config and [`VarBuilder`].
fn build_model(config: &Config, vb: &VarBuilder) -> candle_core::Result<DeepModel> {
    let conv_cfg = Conv2dConfig {
        padding: 1,
        ..Default::default()
    };

    let conv1 = candle_nn::conv2d(1, config.conv_channels[0], 3, conv_cfg, vb.pp("conv1"))?;
    let bn1 = candle_nn::batch_norm(
        config.conv_channels[0],
        BatchNormConfig::default(),
        vb.pp("bn1"),
    )?;
    let conv2 = candle_nn::conv2d(
        config.conv_channels[0],
        config.conv_channels[1],
        3,
        conv_cfg,
        vb.pp("conv2"),
    )?;
    let bn2 = candle_nn::batch_norm(
        config.conv_channels[1],
        BatchNormConfig::default(),
        vb.pp("bn2"),
    )?;

    let flat_size = config.conv_channels[1] * config.board_rows * config.board_cols;
    let dense_in = flat_size + config.aux_features;

    let fc1 = candle_nn::linear(dense_in, config.dense_width, vb.pp("fc1"))?;
    let fc2 = candle_nn::linear(config.dense_width, config.dense_width, vb.pp("fc2"))?;
    let fc_out = candle_nn::linear(config.dense_width, 1, vb.pp("fc_out"))?;

    Ok(DeepModel {
        conv1,
        bn1,
        conv2,
        bn2,
        fc1,
        fc2,
        fc_out,
    })
}

impl Weights for Deep {
    fn name() -> &'static str {
        "deep"
    }

    fn evaluate<const N: usize>(&self, game: &Game<N>, _context: &AttackContext) -> f64 {
        let board = encode_board(game, &self.config);
        let aux = encode_aux(game, None, &self.config);
        self.infer(&board, &aux)
    }

    fn evaluate_with_path<const N: usize>(
        &self,
        game: &Game<N>,
        _context: &AttackContext,
        path: &[Move],
    ) -> f64 {
        let mut history = History::empty();
        for mv in path {
            history.push(mv.piece());
        }

        let board = encode_board(game, &self.config);
        let aux = encode_aux(game, Some(&history), &self.config);
        self.infer(&board, &aux)
    }
}
