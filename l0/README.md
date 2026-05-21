# Remus L0 — The Reflex Layer

> Daemon **kernel-adjacent** scris în Rust. L0 e *sistemul nervos autonom* al lui Remus: vorbește direct cu hardware-ul, are reflexe sub-ms, e linkul fizic între identitatea Remus și device-ul pe care trăiește.
>
> Vezi `../docs/L0_SACRED.md`, `../CLAUDE.md` §2 (arhitectura 3-layer brain), §6 (Faza 6 — acest crate).

## 1. Cele 3 layere ale minții Sentient

| Layer | Rol | Implementare |
|---|---|---|
| **L2 — Cognitive** | *Conștient* — gândire, limbaj, intenție | LLM (Ollama llama3.2), VLM optional |
| **L1 — Agentic** | *Subconștient* — ReAct loop, memorie semantică, decizii | Python (orchestrator, ChromaDB) |
| **L0 — Reflex** | *Sistem nervos autonom* — reflexe sub-ms, simțirea hardware-ului ca prelungire | **Rust daemon** (acest crate) |

## 2. Status (S6.x sub-sprints)

| # | Sprint | Status |
|---|---|---|
| **S6.0** | Scaffold crate, boot sequence placeholder | ✓ done |
| **S6.1** | Identity + boot integrity (HMAC verify pe `identity_core.bin`, port din Python) | ✓ done |
| **S6.2** | HAL trait + telemetrie statică din `/sys/`, `/proc/`, ACPI | ✓ done |
| **S6.3** | MQTT broker (rumqttd embedded) — topics `remus/telemetry/*` | ✓ done |
| **S6.4** | gRPC server (tonic) — `IdentityService`, `TelemetryService` (streaming), `ReflexService` (placeholder) | ✓ done |
| S6.5 | Reflex actions — CPU throttle, brightness, fan curve, suspend | pending |
| S6.6 | Python L1 client (`core/l0_client.py`) — înlocuiește `l0_simulator.py` | pending |
| S6.7 | systemd service + NixOS integration | pending |

## 3. Target platform

- **Binar production**: Linux only (T460 NixOS + viitoare device-uri Linux Remus). Citește `/sys/class/hwmon`, `/sys/class/power_supply`, `/proc/stat`, ACPI, cpufreq nativ.
- **Dev pe Windows**: `cargo check` + `cargo test` + `cargo build` merg. Codul Linux-specific e behind `#[cfg(target_os = "linux")]`. Binarul rulează (placeholder), dar HAL afișează mesaj „mod degradat" — nu citește hardware real pe Win.
- **Interacțiune cross-system** (iOS, Android, alte Windows, peers Remus) **trăiește în L1 Python**, nu L0 (vezi `../CLAUDE.md` Faza 14).

## 4. Build & run

### 4.1 Pe Linux (target real)

```bash
cd l0
cargo build --release
sudo cp target/release/remus-l0 /usr/local/bin/
sudo cp ../deploy/nixos/remus-l0.service /etc/systemd/system/  # creat în S6.7
sudo systemctl enable --now remus-l0
```

### 4.2 Pe Windows (dev)

Necesar:
- Rust stable, toolchain GNU: `rustup default stable-x86_64-pc-windows-gnu`
- MSYS2 cu mingw-w64 binutils + gcc:
  ```cmd
  winget install --id MSYS2.MSYS2
  C:\msys64\usr\bin\pacman.exe -Sy --noconfirm mingw-w64-x86_64-binutils mingw-w64-x86_64-gcc
  ```
- Adaugă `C:\msys64\mingw64\bin` în PATH (linker `dlltool.exe`).

Apoi:
```bash
cd l0
cargo check     # validare sintaxa, fără linking
cargo build     # debug build
cargo test      # unit tests (nu cele care necesită /sys/)
cargo run       # rulează placeholder-ul L0
```

## 5. Logging

`tracing` cu `tracing-subscriber`. Default level: `info`. Pentru verbose:

```bash
RUST_LOG=info,remus_l0=debug cargo run
RUST_LOG=trace cargo run                 # tot
```

## 6. Structură module

```
l0/
├── Cargo.toml
├── README.md             # acest fișier
├── proto/
│   └── l0.proto          # contract gRPC între L0 (Rust) și L1 (Python)
└── src/
    ├── main.rs           # entry point + boot sequence + reflex loop
    ├── identity/         # S6.1 — HMAC verify identity_core.bin
    ├── hw/               # S6.2 — HAL trait + Linux impl behind cfg
    ├── bus/              # S6.3 — rumqttd embedded MQTT broker
    ├── grpc/             # S6.4 — tonic gRPC server
    └── reflex/           # S6.5 — reflex action loop
```

## 7. Dependențe (Cargo.toml)

Minimal in S6.0 (le adăugăm pe parcurs):

- `tokio` — async runtime multi-threaded
- `tracing` + `tracing-subscriber` — logging structurat
- `anyhow` + `thiserror` — error handling
- `serde` + `serde_json` — config + identity payload
- `hmac` + `sha2` + `hex` — pregătit pentru S6.1 (identity verify)

Adăugări pe sub-sprint-uri:
- S6.3: `rumqttd` + `rumqttc` (broker MQTT embedded + publisher)
- S6.4: `tonic` + `prost` + `tokio-stream` (gRPC + streaming)
- S6.4: `tonic-build` + `protoc-bin-vendored` (build.rs compileaza `.proto` cu protoc vendat — nu cere instalare separată pe Win)

## 8. Securitate

- Binarul rulează sub user `remus` (NU root), cu capabilities granulare:
  - `CAP_SYS_RAWIO` — citire `/sys` și `/dev` non-public
  - `CAP_NET_BIND_SERVICE` — bind pe porturi <1024 (nu folosit acum)
- `identity_core.bin` și `sync_key.bin` au permisiuni 0400, owner root, `chattr +i` (vezi `../docs/L0_SACRED.md`).
- L0 sacred files NU pot fi atinse de pipeline-ul L1 de self-improvement (refuz hard).
- Boot integrity check (S6.1) compară hash-uri SHA256 ale L0 sacred files — blocheaza boot dacă diferă.

## 9. Comunicare cu L1 (Python)

| Direcție | Transport | Conținut |
|---|---|---|
| L0 → L1 | MQTT `remus/telemetry/{cpu,mem,battery,thermal,backlight,full}` | telemetrie continuă (S6.3 ✓) |
| L0 → L1 | MQTT `remus/status` (retain) | online/offline (S6.3 ✓) |
| L0 → L1 | MQTT `remus/hw/event` | udev events (boxă conectată, mic detașat) — Faza 7 |
| L1 → L0 | gRPC `ReflexService.RequestAction` | „set CPU governor performance", „brightness 50%" — wiring ✓ S6.4, semantică S6.5 |
| L1 ↔ L0 | gRPC `IdentityService.GetIdentity` | nucleul identitar verificat (S6.4 ✓) |
| L1 ↔ L0 | gRPC `TelemetryService.Subscribe` | stream alternativ la MQTT (S6.4 ✓) |

**Porturi locale**:
- MQTT: `127.0.0.1:11883`. Standardul e 1883, dar pe Windows TIME_WAIT pe 1883 blochează re-bind între restart-uri rapide. 11883 evită conflict-ul. Pe Linux production se poate trece înapoi la 1883 (config în `bus/broker.rs::BROKER_TOML`).
- gRPC: `127.0.0.1:50051` (fără TLS in S6.4 init — local-only, ca MQTT). TLS + auth mTLS vin în S6.x când permitem peer multi-body cross-device.

**Verificare gRPC end-to-end**: clientul Python L1 (S6.6) va folosi `grpcio` + stub generat din `proto/l0.proto`. Până atunci, smoke test = port-ul 50051 listening + log-ul `[L0/grpc] gRPC server listening on 127.0.0.1:50051`. Pentru probă manuală cu `grpcurl`, vezi mai jos.

### 9.1 Probă rapidă cu grpcurl

```bash
# Lista servicii (necesită feature reflectie — adăugat în S6.x ulterior)
# grpcurl -plaintext 127.0.0.1:50051 list

# Apel direct fără reflectie (cu proto local):
grpcurl -plaintext -proto proto/l0.proto -import-path proto \
    127.0.0.1:50051 remus.l0.v1.IdentityService/GetIdentity

grpcurl -plaintext -proto proto/l0.proto -import-path proto \
    -d '{"interval_ms": 2000}' \
    127.0.0.1:50051 remus.l0.v1.TelemetryService/Subscribe
```
