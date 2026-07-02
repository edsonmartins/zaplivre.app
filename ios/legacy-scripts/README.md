# Legacy scripts

Scripts antigos de build/bindings mantidos apenas para referência.
O pipeline atual é: `build-all.sh` → `build-rust.sh` + `generate-bindings.sh` (uniffi 0.31) → `xcodegen generate` → `xcodebuild`.
Não use estes scripts.
