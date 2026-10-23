# Status isporuke 0.2 — 23.10.2026.

Ova verzija dodaje ljudsku autentikaciju i tenant konzolu na postojeću push vertikalu. Nije još potpuni MVP iz specifikacije.

## Izvršene provjere

| Provjera | Rezultat |
| --- | --- |
| Rust build/check svih targeta | Prošlo |
| Rust fmt | Prošlo |
| Clippy sa zabranom upozorenja | Prošlo |
| Rust unit testovi | 8 prošlo: ingest validacija, Argon2id, cookie/email i senzorska validacija |
| Frontend TypeScript + Vite production build | Prošlo |
| SQLx migracije 0001 i 0002 na praznoj PGlite bazi | Prošlo |
| SQL identity izolacijski scenariji | Prošlo: bez tenant konteksta nema uređaja, tuđi uređaj nije vidljiv, memberships su ograničeni na korisnika, runtime ne čita sessions, credential insert odbija tuđi tenant, istek/opoziv sesije |
| HTTP auth/RBAC scenariji kroz stvarni Rust API | 2 testna slučaja prošla, s više provjera: login, HttpOnly/SameSite cookie, CSRF/Origin odbijanje, A/B izolacija, viewer zabrana kreiranja, admin kreiranje uređaja, novi servisni token → push → latest, logout i replay opozvanog cookieja |
| Installer nadogradnje | Prošlo: čuva točne bajtove stare migracije 0001 i .env te radi backup prepisanih datoteka |
| Python sintaksa, YAML/JSON struktura | Prošlo |

DB provjere ovdje koriste **PGlite 0.5.11 / PostgreSQL 18.6 u WASM-u**. HTTP testovi koristili su PGlite socket, eksplicitni `SET ROLE marivect_app`, pool s jednom konekcijom i isključeni statement cache zbog ograničenja PGlite protokola. Bootstrap/migrator prethodno su radili privilegirano. Ovo provjerava stvarni SQL i API autorizaciju, ali **ne potvrđuje ponašanje višekonekcijskog poola, konkurentnost ni Docker PostgreSQL 17 deployment**. Produkcijski runtime zadržava zadani pool od 10 konekcija i SQLx statement cache.

## Preostala verifikacija

Docker Engine/Compose i pravi PostgreSQL 17 server nisu dostupni u ovom okruženju. Docker build/start, Nginx rate limiting i testovi na ciljnom Compose stacku ovdje nisu pokrenuti. GitHub CI konfiguracija uključuje PostgreSQL 17 migracije, SQL testove i HTTP auth testove, ali taj workflow ovdje nije izvršen.

Pravi browser E2E nije izvršen; frontend je verificiran TypeScriptom i buildom. Vite je prijavio neblokirajuća upozorenja o ignoriranim `use client` direktivama TanStack Queryja. Nema provjere fizičke opreme.

## Važno pri nadogradnji

`install-upgrade.py` čuva korisnikovu primijenjenu datoteku 0001 jer je ručno ispravljena tijekom pokretanja prve verzije. Ne usklađuje checksumove preko baze i ne briše SQLx povijest. Dodaje se samo nova migracija 0002. Sigurnosna kopija izmijenjenih ostalih datoteka ostaje lokalno u `.marivect-backups`.

## Još nije implementirano

Web administracija korisnika/članstava, samostalno kreiranje novih organizacija, lokacije, edit/delete uređaja i senzora, lifecycle servisnih tokena, audit, history/grafovi, puni ingest queue, boolean/text mjerenja, retencija, Campbell pull, ChirpStack, alarmi i naredbe. Vanjski transporti ostaju označeni kao neimplementirani. Worker je i dalje heartbeat proces.
