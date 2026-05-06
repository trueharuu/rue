use serde::Serialize;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub enum Spin {
    None,
    Full,
    Mini,
}
