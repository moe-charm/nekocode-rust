use proc_macro::TokenStream;

/// Deliberately emits a compiler error so the fixture proves this proc macro
/// executed during `cargo check`. The error is data for the context layer.
#[proc_macro_derive(ExecutionSentinel)]
pub fn execution_sentinel(_input: TokenStream) -> TokenStream {
    "compile_error!(\"nekocode proc macro execution sentinel\");"
        .parse()
        .expect("sentinel token stream")
}
