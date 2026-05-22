# YBOS L0 — The Reflex Layer

Daemon kernel-adjacent scris în Rust. L0 gestionează identitatea per-user,
tripwire-ul de integritate, telemetria hardware și reflexele locale.

## Status Y1

| Component | Status |
|---|---|
| Identity per-user | enrollment scaffold + blob HMAC |
| Envelope A | Argon2id PIN + biometric/device fingerprint |
| Envelope B | trait stub, TEE plan documentat |
| Envelope C | trait stub, hardware-key plan documentat |
| BIP39 backup | 24 cuvinte, afișare o singură dată, marker lock |
| Session tokens | L0-side API hook, in-memory active sessions |
| Tripwire | hash L0 sacred sources + identity artifacts |
| HAL telemetry | Linux backend + non-Linux stub |
| MQTT/gRPC | local-only development services |

## Build & Test

```bash
cd l0
cargo build
cargo test
```

Pentru rulare locală:

```bash
cd l0
YBOS_DATA=.ybos-data cargo run
```

`YBOS_DATA` controlează layout-ul identity:

```text
${YBOS_DATA}/identity/identity_core.bin
${YBOS_DATA}/identity/identity_core.salt
${YBOS_DATA}/identity/bip39.lock
${YBOS_DATA}/identity/k_envelope_a.bin
${YBOS_DATA}/identity/k_envelope_b.bin
${YBOS_DATA}/identity/k_envelope_c.bin
${YBOS_DATA}/identity/l0_sacred.hashes.json
```

## Services

| Direction | Transport | Content |
|---|---|---|
| L0 → L1 | MQTT `ybos/telemetry/{cpu,mem,battery,thermal,backlight,full}` | telemetrie continuă |
| L0 → L1 | MQTT `ybos/status` | online/offline retained |
| L1 ↔ L0 | gRPC `ybos.l0.v1.IdentityService/GetIdentity` | identitate verificată după unlock |
| L1 ↔ L0 | gRPC `ybos.l0.v1.TelemetryService/Subscribe` | stream telemetrie |
| L1 → L0 | gRPC `ybos.l0.v1.ReflexService/RequestAction` | reflex actions placeholder |

Porturi locale:

- MQTT: `127.0.0.1:11883`
- gRPC: `127.0.0.1:50051`

## Cross-compile aarch64

To build `ybos-l0` for Android (aarch64), use the provided wrapper script.
You must have the Android NDK installed and `ANDROID_NDK_HOME` set.

```bash
cd l0
export ANDROID_NDK_HOME=/path/to/your/ndk
./build_android.sh
```

The script will add the necessary Rust target, configure the linker paths,
and produce a release binary in `target/aarch64-linux-android/release/ybos-l0`.

## Security Notes

- K-master nu este scris niciodată în clar pe disk.
- `identity_core.bin` este HMAC-SHA256 semnat cu K-master.
- `bip39.lock` marchează doar că fraza a fost afișată; fraza nu este stocată.
- L0 sacred writes sunt refuzate după sealing; onboarding poate crea artifactele o singură dată.
- TEE și hardware-key integration sunt doar trait-uri în Y1, fără implementare reală.
