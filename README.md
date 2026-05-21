# YBOS

> **Status**: Pre-MVP. Planning phase complete (2026-05-21). Implementation in progress.

YBOS este un sistem de operare AI-native, **mobile-first**, cu un agent LLM principal care orchestrează agenți specializați (calendar, business trip, learning, market intel, news) și păstrează datele utilizatorului în siguranță prin design.

## Caracteristici cheie

- **AI Executive Assistant OS** — agentul LLM principal e și asistent, și paznic privacy
- **Per-user identity** — fiecare device personalizat la primul boot (nume + PIN + biometric opțional)
- **Privacy by design** — firewall în 3 layere: capabilities + eBPF redactor + LLM judge
- **Android compatibility** — Google Play apps merg nativ (baza AOSP)
- **Rust everywhere** — userland nou și kernel modules pentru performance + safety
- **Cross-device** — simbioză telefon ↔ laptop prin telemetrie + sync semnat

## Status implementare

Vezi `docs/ROADMAP.md`. MVP target: 8-10 luni.

## Pentru dezvoltatori

- `YBOSClaude.md` — instrucțiuni Claude Code (source of truth context)
- `docs/ARCHITECTURE.md` — arhitectură detailed
- `docs/HARDWARE.md` — device test specs
- `docs/L0_SACRED.md` — protocol securitate L0
- `l0/` — Rust daemon kernel-adjacent (portat din [RemusOS3](https://github.com/PGC22/RemusOS3))

## Licență

TBD. Acest repo e public ca planning în desfășurare; license-ul final va fi stabilit înainte de release.

---

*Code implemented with help from AI Agents Claude, Codex, Jules.*
