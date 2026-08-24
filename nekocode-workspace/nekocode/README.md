# nekocode

The canonical NekoCode CLI exposes the two Rust-first use cases:

```text
nekocode snapshot PATH
nekocode context PATH --baseline SNAPSHOT.json
```

The CLI is a thin adapter over `nekocode-core`. See the repository
[README](../../README.md) for the artifact and execution contracts.
