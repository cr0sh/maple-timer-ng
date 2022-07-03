use enum_kind::Kind;

#[derive(Kind, PartialEq, Eq)]
#[kind(functions(to_str = "&'static str"))]
#[kind(functions(width = "u32"))]
#[kind(functions(height = "u32"))]
pub enum ScreenDimension {
    #[kind(to_str = "stringify!(1280x720)", width = "1280", height = "720")]
    X1280Y720,
    // #[kind(to_str = "stringify!(1366x768)", width = "1366", height = "768")] // FIXME: buggy
    // X1366Y768,
}

impl Default for ScreenDimension {
    fn default() -> Self {
        Self::X1280Y720
    }
}
