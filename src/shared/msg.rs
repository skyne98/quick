#[derive(Debug, Clone)]
pub enum QuickMsg {
    NoOp,
    String(String),
    Binary(Vec<u8>),
}
