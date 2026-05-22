# Legacy Prototype → YBOS Port Notes

> Sesiunea: 2026-05-21
> Decision-maker: project architect
> Lead Dev: Claude (Opus 4.7)

Acest document explică ce a fost preluat din prototipul inițial la crearea
YBOS, ce a fost lăsat, și de ce.

## Context

Prototipul inițial era un OS AI personal, construit pe Linux kernel, target
laptop, single-user și cu owner hardcoded. YBOS este pivot-ul spre produs
vandabil: mobile-first, multi-user, AOSP base, executive assistant cu
multi-agent + privacy firewall.

## Ce am portat

### 1. `l0/` — întregul crate Rust

Acțiune: copy complet în `YBOS/l0/`, schimbat Cargo.toml la `ybos-l0`.

Conținut portat:

- `Cargo.toml`, `build.rs`, `proto/l0.proto`
- `src/main.rs`, `src/bus/`, `src/grpc/`, `src/hw/`, `src/identity/`, `src/reflex/`
- `README.md`

Status la moment port:

- Scaffold crate
- Identity + boot integrity
- HAL telemetrie statică
- MQTT broker embedded + publisher
- gRPC server cu IdentityService + TelemetryService + ReflexService stub

Adaptări Y1:

- Generalizare `identity/` de la owner hardcoded la enrollment dinamic
- Update paths în `paths.rs` pentru layout `${YBOS_DATA}/identity`
- Adaptare `sacred.rs` pentru fișiere YBOS
- Păstrare MQTT ca transport local de dezvoltare până la decizia Binder/L1

### 2. Conceptul L0 SACRED

Păstrat ca regulă de refuz sintactic pentru fișiere identity-critical și
adaptat pentru multi-user + Android storage protection.

### 3. Conceptul 3-envelope crypto

Adaptare YBOS:

- Envelope A: Argon2id(PIN + biometric_template + device_fingerprint)
- Envelope B: TEE mobil prin trait în Y1, implementare reală în faza AOSP/device
- Envelope C: hardware-key HMAC opt-in prin trait în Y1

### 4. BIP39 paper backup

Păstrat ca strategie. 24 cuvinte afișate o singură dată la onboarding.

### 5. Identity blob signed HMAC

Păstrat. `identity_core.bin` este semnat HMAC cu K-master.

### 6. Tripwire boot integrity

Păstrat. Hash check pe L0 SACRED files + hash check pe lista L0_SACRED însăși.

## Ce am lăsat

- Tot codul Python runtime
- Web UI și TUI vechi
- Pipeline-uri de deploy pentru laptop/Linux
- Feature-uri personale care nu sunt MVP executive-assistant
- Device-specific laptop paths/constants
- Owner hardcoded
- Roluri cross-device vechi

## Update după Y1

Y1 a rescris `l0/src/identity/` pentru enrollment generic, envelope A,
BIP39 one-time display marker, session-token API hook și tripwire pe layout
`${YBOS_DATA}/identity`.
