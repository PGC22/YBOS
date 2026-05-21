# L0 SACRED — Protocol de securitate inviolabil

> Versiune: 0.1 (portat din RemusOS3 `docs/L0_SACRED.md`, adaptat pentru YBOS multi-user)
> Data: 2026-05-21

---

## Principiul de bază

**L0 SACRED files sunt fișiere care nu pot fi modificate de niciun cod automat, în nicio circumstanță, nici cu autorizare biometrică sau YubiKey. Singura cale de modificare: utilizatorul, manual, după ce-a dezactivat protecția SELinux + immutable bit ca root.**

Asta NU e o cerere de autorizare, e un **refuz sintactic**. Codul self-improvement / OTA update / agent skill discovery primește `Error::SacredViolation` și se oprește.

---

## Lista L0 SACRED (YBOS)

Hardcodată în `l0/src/identity/sacred.rs`:

```rust
const L0_SACRED: &[&str] = &[
    // Daemon Rust sources — codul care VERIFICĂ sacred files
    "l0/src/identity/sacred.rs",     // includ și asta — nu poate modifica lista
    "l0/src/identity/paths.rs",
    "l0/src/identity/blob.rs",
    "l0/src/identity/mod.rs",
    "l0/src/main.rs",

    // Identity blob per-user (generat la onboarding)
    "config/identity_core.bin",      // nucleul identitar semnat HMAC
    "config/identity_core.salt",     // salt pentru Argon2id envelope A
    "config/bip39.lock",             // marker că paper backup a fost afișat (NU mnemonic-ul!)

    // Cheia master K wrapped (envelope sealed pe TEE)
    "config/k_envelope_a.bin",       // Argon2id-wrapped K
    "config/k_envelope_b.bin",       // TEE-sealed K (pe StrongBox / Hexagon)
    "config/k_envelope_c.bin",       // YubiKey-wrapped K (opt)

    // Boot integrity manifest
    "config/l0_sacred.hashes.json",  // hashes ale L0 sacred files, verificate la boot
];
```

---

## Cum se aplică

### La compile time
`l0/src/identity/sacred.rs` exportă funcția:
```rust
pub fn is_l0_sacred(path: &Path) -> bool {
    // Normalizat (resolve symlinks), anti-relative, case-sensitive pe Linux
}
```

### La runtime în orice cod self-modification
```rust
use l0::identity::sacred::is_l0_sacred;

fn apply_proposed_change(path: &Path, new_content: &[u8]) -> Result<()> {
    if is_l0_sacred(path) {
        return Err(Error::SacredViolation(path.to_owned()));
    }
    // ... continue with normal flow (snapshot, test, apply, rollback)
}
```

### La boot
1. Citește `config/l0_sacred.hashes.json`
2. Re-hash fiecare L0 SACRED file actual
3. Compară cu manifesta
4. Mismatch → boot blocat cu alertă la user
5. Verifică hash al listei L0_SACRED ÎNSEȘI (anti-tamper pe `sacred.rs`)

---

## Protecție Layer 2: file system

### Pe Linux dev (RemusOS3 a folosit `chattr +i`)
```bash
sudo chattr +i config/identity_core.bin
sudo chmod 0400 config/identity_core.bin
sudo chown root:root config/identity_core.bin
```

### Pe Android (YBOS production)
- **SELinux policy** `restrict_l0_sacred`: doar `ybos-l0` daemon poate citi, nimeni nu poate scrie
- **fs-verity** (Linux 5.4+): file marked read-only at FS level, cu Merkle tree pentru integrity
- **Immutable bit** via `libfsverity_enable_file()`
- **dm-verity** pe partiția /data dacă feasible (AOSP support)

---

## Singura cale de modificare L0 SACRED

Utilizatorul, manual:

1. Boot device în recovery mode (volume_down + power la Pixel)
2. `adb shell` ca root
3. `setenforce 0` (dezactivare SELinux temporar)
4. `chattr -i config/identity_core.bin`
5. Modificare manuală
6. **Re-generare hash manifest** (`tools/regen_sacred_hashes.sh`)
7. `chattr +i config/identity_core.bin`
8. `setenforce 1`
9. Reboot

**Asta e by design.** Dacă atacatorul are root pe device + acces fizic, e oricum game over. Protecția e pentru tot ce ține de remote / runtime / automated.

---

## Ce NU sunt L0 SACRED (deci modificabile, dar cu permisiuni)

- Identitate per-user vizibilă (display name, avatar) — `config/profile.json`, modifiable cu PIN
- Agent manifests — `agents/*/manifest.toml`, modifiable cu PIN
- Settings — `config/settings.json`, modifiable cu PIN
- Vector DB content — `data/memory.db`, modifiable de orchestrator

---

## Tripwire la boot (ybos-l0)

```rust
fn boot_integrity_check() -> Result<()> {
    let manifest = read_sacred_hashes()?;

    // 1. Verifică ÎNSĂȘI lista L0_SACRED (anti edit pe sacred.rs)
    let self_hash = sha256(L0_SACRED_BYTES);
    if self_hash != manifest.sacred_list_hash {
        return Err(Error::SacredListTampered);
    }

    // 2. Verifică fiecare sacred file
    for path in L0_SACRED {
        let actual = sha256(read(path)?);
        let expected = manifest.files[path];
        if actual != expected {
            return Err(Error::SacredFileTampered(path.into()));
        }
    }

    Ok(())
}
```

Pe boot failure: device boot stops, alert pe UI: *"Integritate L0 compromisă. Contactează suport. Code: SACRED_TAMPER_<file>"*. User trebuie să facă recovery manual sau wipe.

---

## Audit recommendations

- L0_SACRED list **trebuie review-uită** când se adaugă fișier nou în identity-critical path
- Orice PR care atinge `l0/src/identity/*` are CODEOWNERS = George + Claude review obligatoriu
- Penetration testing periodic (post-MVP): încearcă să modifici L0 SACRED prin: OTA update simulat, agent malițios, escaladare privilege

---

## Diferențe față de Remus (pentru istoric)

| Aspect | Remus (RemusOS3) | YBOS |
|---|---|---|
| Owner | Hardcoded George | Per-user enrollment la onboarding |
| Storage protection | `chattr +i` pe Linux/NixOS | SELinux + fs-verity pe Android |
| Tripwire | Python `core/paths.py is_l0_sacred()` | Rust `l0/src/identity/sacred.rs` |
| YubiKey | Mandatoriu envelope C | Opt-in (NFC sau USB-C) |
| TPM | Discrete TPM pe T460 | StrongBox (Pixel) / Hexagon TEE (Snapdragon) |
| Recovery | Bootabil din Live USB | Recovery mode Android (adb root) |

Conceptul rămâne identic. Implementarea se adaptează la platforma mobilă.
