#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SpinType {
    None = 0,
    Mini = 1,
    Full = 2,
}

impl SpinType {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            0 => SpinType::None,
            1 => SpinType::Mini,
            2 => SpinType::Full,
            _ => panic!("invalid SpinType discriminant"),
        }
    }
}

pub const SPIN_NB: usize = 3;