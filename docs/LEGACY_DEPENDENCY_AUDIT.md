# Legacy dependency audit before physical archive

監査日: 2026-08-23

物理移動の前に、旧root package・5バイナリ配布・MCPの三系統を切り替える必要がある。`nekocode-workspace`を先に移動すると、現行の配布・MCP導線が壊れるため、順序を固定する。

## 発見した依存

### Root package

- root `Cargo.toml` は旧単一package `nekocode-rust` を定義する。
- `.github/workflows/build-and-test.yml` と `release.yml` はroot packageをfmt/test/build/releaseする。
- `scripts/update_releases.sh` はrootでbuildするため、Rust-first CLIの配布更新になっていない。
- `README.md`、`nekocode-analysis.yml`、`security.yml`にも旧`analyze`/`nekocode-rust`導線が残る。

### 5-binary workspace

- workspace memberには`nekorefactor`、`nekoimpact`、`nekoinc`、`nekomcp`、`nekosplit_rust`が残る。
- `Makefile`、`build.sh`、`build-and-deploy.sh`は5本を`bin/`/`releases/`へコピーする。
- `releases/setup.py`は`nekocode`と`nekorefactor`を別々に探索する。

### MCP

- `mcp-nekocode-server/mcp_server_real.py`はrelease/workspace/root/C++の複数binaryを探索し、legacyの`analyze`、session、refresh、deadcodeを呼ぶ。
- `mcp_wrapper_5binary.py`は`nekocode`、`nekorefactor`、`nekoimpact`を同時に必要とする。
- `mcp_server_nekocode.py`と`config.json`には開発機固有のbinary path候補も残る。

## Archive gate

1. root workflow/release/security/analysisをRust-first CLIへ切り替えるか、legacy workflowとして明示的に隔離する。
2. MCPに`index`/`context`専用の最小gatewayを追加し、旧5-binary serverをlegacy扱いにする。
3. workspace memberを`nekocode-core` + `nekocode`へ縮退する方針を決める。
4. Makefile/build/setup/release/Dockerの5本コピー前提を切り替える。
5. clean checkoutでcore test、CLI index/context、MCP smokeを通す。
6. その後にroot package、legacy crates、配布ELFをarchive branchまたは`archive/legacy`へ移す。

現時点では依存監査と、コミット済みHEADのclean checkout（core 9 tests、CLI check/index）まで完了。物理移動は、上記導線の切り替えとMCP smokeが完了するまで実施しない。
