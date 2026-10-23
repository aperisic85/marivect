# Status isporuke — 23.10.2026.

Ovo je skeleton s implementiranom uskom normaliziranom push vertikalom. Nije potpuni MVP iz priložene specifikacije.

## Izvršeno u radnom okruženju

| Provjera | Rezultat |
| --- | --- |
| Rust `cargo check --workspace --all-targets` | Prošlo |
| Rust `cargo fmt --all -- --check` | Prošlo |
| Rust `cargo clippy --locked --workspace --all-targets -- -D warnings` | Prošlo |
| Rust `cargo test --locked --workspace` | Prošlo, 4 unit testa |
| Frontend `npm ci --ignore-scripts` | Prošlo |
| Frontend `npm run build` (TypeScript + Vite) | Prošlo |
| Compose/OpenAPI YAML i package-lock JSON parsiranje | Prošlo; nije zamjena za Compose runtime validaciju |
| Python skripte — kompilacija | Prošlo |
| Generiranje .env i odbijanje overwritea | Prošlo u privremenoj mapi; tajne nisu uključene u isporuku |
| Shell skripte — `sh -n` | Prošlo |

Rust provjere izvršene su privremeno instaliranim Rust 1.99.1 alatom. Frontend build koristi Vite 7.3.7 iz lock datoteke. Vite je prijavio neblokirajuća upozorenja o ignoriranom `use client` direktivama TanStack Query paketa.

## Nije izvršeno

Docker Engine/Compose i PostgreSQL server nisu dostupni u ovom okruženju. Nisu izvršeni Docker image build, `docker compose config`, primjena migracija na praznoj bazi ni `tests/smoke.py`. Njihova uspješnost nije potvrđena. CI konfiguracija uključuje te provjere za GitHub runner; workflow ovdje nije izvršen. Izvršen `cargo check` ne validira SQL upite nad stvarnom bazom.

Nema testova ni potvrde rada s fizičkom opremom. Campbell transport, ChirpStack, pull scheduler, ljudska autentikacija/RBAC, history API, alarmi, naredbe i produkcijski hardening ostaju za sljedeće faze. Detaljan plan je u README-u i punoj specifikaciji.

## Početne odluke

- Naziv projekta: Marivect, prema dogovorenom nazivu.
- Početni transport je vlastiti JSON push, nevezan uz proizvođača.
- Prijam i normalizacija su sinkroni u jednoj transakciji; API vraća 200 tek nakon commita.
- Jedan servisni token odgovara jednom registriranom uređaju. Lifecycle i granularni scopeovi dolaze s upravljanjem izvorima.
- Zaštita latest vrijednosti koristi strogo novije vrijeme; pri jednakom vremenu pobjeđuje prvi spremljeni zapis. Više izvora po senzoru i push/pull deduplikacija još nisu implementirani.
- Brojčana mjerenja prva su vertikala. Boolean i tekst slijede u zasebnoj migraciji.
- Worker je stvarni zaseban proces s heartbeatom, bez lažnih poslova i rezultata.
- Nema hardkodiranih demo lozinki/tokena. `.env` generira slučajne vrijednosti na korisnikovu računalu.
