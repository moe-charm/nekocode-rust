use nekocode_core::symbol_context::SymbolSession;
use nekocode_core::{NekocodeError, Result, SymbolContextRequest};
use serde_json::json;
use std::io::{BufRead, Read, Write};
use std::path::PathBuf;

pub fn run(path: Option<PathBuf>) -> Result<()> {
    let root = path.unwrap_or_else(|| ".".into()).canonicalize()?;
    let mut session = SymbolSession::default();
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    loop {
        let mut line = Vec::new();
        let size = input.by_ref().take(65537).read_until(b'\n', &mut line)?;
        if size == 0 {
            break;
        }
        if size > 65536 {
            return Err(NekocodeError::Config(
                "session request exceeds 64 KiB".into(),
            ));
        }
        let mut backend_reused = false;
        let mut reuse = serde_json::json!({"acquisition":"not_observed", "reasons":[], "retained":false, "retention_reasons":[]});
        let result = serde_json::from_slice::<SymbolContextRequest>(&line)
            .map_err(NekocodeError::from)
            .and_then(|mut request| {
                if request.path.is_some() {
                    return Err(NekocodeError::Config(
                        "session workspace is fixed; omit path".into(),
                    ));
                }
                request.path = Some(root.clone());
                let result = session.investigate(&request);
                backend_reused = session.backend_reused();
                reuse = serde_json::to_value(session.reuse_report())
                    .expect("reuse report serialization");
                result
            });
        let response = match result {
            Ok(context) => {
                json!({"contract_version":"symbol-session-v1", "backend_reused":backend_reused, "reuse":reuse, "context":context, "error":null})
            }
            Err(error) => {
                json!({"contract_version":"symbol-session-v1", "backend_reused":backend_reused, "reuse":reuse, "context":null, "error":error.to_string()})
            }
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}
