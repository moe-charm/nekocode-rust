// Deliberate E0308 fixture: diagnostics must preserve a real compiler error.
pub fn intentional_type_error() -> u8 {
    "not-a-u8"
}
