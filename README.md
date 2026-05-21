# YBOS

> **Status**: Pre-MVP. Planning în desfășurare. Faza Y1 (L0 generalization) e următoarea pe execuție.

YBOS este un sistem de operare AI-native, **mobile-first** cu **laptop companion**, în care un agent LLM principal orchestrează agenți specializați (calendar, business trip, learning, market intel, news) și permite utilizatorului să-și creeze proprii agenți pentru orice task. Datele user-ului rămân pe device prin design.

## Caracteristici cheie

- **AI Executive Assistant OS** — agentul LLM principal e și asistent, și paznic privacy
- **Per-user identity** — fiecare device personalizat la primul boot (nume + PIN + biometric opțional)
- **Privacy by design** — firewall în 3 layere: capabilities + eBPF redactor + LLM judge
- **Agent Builder Framework** — user creează agenți noi pentru orice task, fără release nou
- **User-Context Memory** — sistemul învață preferințe și recurrențe (zboruri, calendar, contacts)
- **Laptop Companion** — Tauri app cross-platform, session-based (ca WhatsApp Web), folosește RAM/GPU laptop pentru LLM mari
- **Android compatibility** — Google Play apps merg nativ (baza AOSP)
- **Rust everywhere** — userland nou și kernel modules pentru performance + safety

## Status implementare

Vezi `docs/ROADMAP.md`. Detaliu pentru Y1 (next), restul fazelor headline-only.

## Pentru dezvoltatori

- `YBOSClaude.md` — instrucțiuni Claude Code (source of truth context)
- `docs/ARCHITECTURE.md` — arhitectură detailed (3-layer brain + laptop companion + user-context)
- `docs/HARDWARE.md` — device test specs (flexibile)
- `docs/L0_SACRED.md` — protocol securitate L0
- `docs/ROADMAP.md` — faze (Y1 detaliat, restul TBD)
- `l0/` — Rust daemon kernel-adjacent (portat din [RemusOS3](https://github.com/PGC22/RemusOS3))

## Licență

TBD. Acest repo e public ca planning în desfășurare; license-ul final va fi stabilit înainte de release.

---

*Code implemented with help from AI Agents Claude, Codex, Jules.*
