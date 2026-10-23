# Marivect — skeleton 0.1

Multitenant platforma za nadzor udaljene opreme. Rust/Axum/Tokio + SQLx/PostgreSQL 17 + React/TypeScript/Vite. Docker i Docker Compose obvezni su dio ovog projekta.

Ovo je **razvojni skeleton s uskom funkcionalnom push vertikalom**, a ne gotov MVP iz pune specifikacije. Nema korisničkog logina niti administracije. Konzola koristi servisni token samo za njegov registrirani uređaj. Ne izlagati ovaj razvojni stack javno.

## Pokretanje

Preduvjeti: Docker Engine/Desktop s Compose v2 i Python 3. Node i Rust na računalu nisu potrebni.

```sh
python3 scripts/init-env.py
docker compose up --build -d
docker compose --profile tools run --rm seed
```

Na Linuxu/macOS-u sve tri naredbe možeš pokrenuti i s `sh scripts/dev-up.sh`.

Otvori **http://localhost:8080**. Token `DEMO_TOKEN_A` ili `DEMO_TOKEN_B` iz generirane `.env` datoteke unesi u razvojnu konzolu. Podataka nema dok ne pošalješ mjerenje. Za demonstracijski unos:

```sh
# Lokalno generirana .env sadrži samo sigurne hex vrijednosti i broj porta.
set -a
. ./.env
set +a
SOURCE_TOKEN="$DEMO_TOKEN_A" python3 scripts/simulate-push.py
```

PowerShell: postavi `$env:SOURCE_TOKEN` na vrijednost `DEMO_TOKEN_A`, zatim `python scripts/simulate-push.py`.

Pregled logova i zaustavljanje:

```sh
docker compose logs -f api worker
docker compose down
```

Podaci ostaju u named volumeu `pgdata`. `docker compose down -v` trajno briše razvojnu bazu. Postgres lozinke i inicijalne uloge postavljaju se samo pri prvom stvaranju baze; promjena `.env` sama ne rotira postojeće DB lozinke. `seed` ponovno izvođenje rotira demo tokene na vrijednosti iz `.env`.

## Što je implementirano

- Compose: PostgreSQL, jednokratna migracija, API, zaseban worker i Nginx frontend; dodatni tools profili za seed/provjere.
- Baza i API nemaju host portove. Nginx služi frontend i proxy za API na istom originu, vezan na localhost.
- Runtime uloga nije vlasnik tablica, superuser niti BYPASSRLS. Tenant kontekst postavlja se samo unutar transakcije.
- Dva demonstracijska tenanta s po jednim uređajem i `battery_voltage` senzorom.
- Hashirani servisni tokeni određuju tenant i uređaj. Tijelo zahtjeva ne bira tenant.
- Normalizirani push, provjera registriranog senzora/jedinice, trajni događaj i brojčana mjerenja u jednoj transakciji. HTTP 200 znači **obrađeno i commitano**; ovo nije asinkroni 202 ingest.
- Idempotentnost po tenant/uređaj/message_id. Isti ID s promijenjenim sadržajem daje 409. Zakašnjeli zapis ne zamjenjuje novije latest stanje. Kod jednakog vremena zadržava se prvi zapis.
- Pregled latest vrijednosti, stanje greške/praznih podataka i osvježavanje svakih 15 s. Token ostaje samo u memoriji; cache se čisti kod promjene izvora.
- Adapter ugovor i zaseban worker s DB heartbeatom. Worker još ne izvršava poslove.

## Struktura

```text
backend/src/bin/     api, worker, migrate, seed
backend/src/         zajednička validacija, tenant transakcije, adapter ugovor
backend/migrations/  SQL migracije
frontend/src/        React razvojna konzola
infra/postgres/      inicijalna runtime DB uloga
scripts/             generiranje .env i push simulator
tests/               integracijske provjere stvarnog stacka
docs/                puna specifikacija, API ugovor, status i plan
compose.yaml         lokalni sustav
```

## Provjere

```sh
docker compose config --quiet
docker compose --profile tools run --build --rm checks
python3 tests/smoke.py
```

`checks` radi Rust fmt, Clippy i unit testove. Smoke test prethodno zahtijeva pokrenut stack i seed; upisuje testna mjerenja u demo tenant A. Provjerava duplikate, konflikt ID-a, zakašnjelo mjerenje, vjerodajnice, odbijanje tenant polja i RLS reset na istoj konekciji. Nije potpuni test poola, RBAC-a ni budućih endpointa.

Frontend provjera bez lokalnih alata: `docker compose build frontend` pokreće `npm ci` i TypeScript/build. Frontend nema zaseban lint alat u ovoj verziji.

## Sigurnosne granice

Migrator u lokalnom Composeu jest privilegirana bootstrap uloga potrebna za kreiranje auth uloge; API i worker dobivaju samo ograničenu runtime vjerodajnicu. Produkcija treba zaseban kontrolirani provisioning i ograničenog vlasnika migracija. Tablice tenant podataka koriste FORCE RLS i kompozitne strane ključeve. Credentials tablica ima posebnu RLS SELECT politiku samo za NOLOGIN auth ulogu radi pronalaska izvora prije tenant konteksta. `tenants` i `source_credentials` nemaju runtime table grant; uska SECURITY DEFINER funkcija provjerava hash bez izlaganja hashova. Njezina NOLOGIN uloga ima samo SELECT nad credentials. Ne vraćaju se proizvoljni tenant podaci.

Ograničenje zahtjeva je 64 KiB / 128 mjerenja. Nisu implementirani rate limiting, korisničke sesije, CSRF, lifecycle tokena, audit ni produkcijski TLS. Token konzola služi provjeri ove vertikale, a ne zamjeni za RBAC. Nepoznat senzor/jedinica odbija cijeli zahtjev. Neispravan JSON vraća standardni Axum rejection (nije još usklađen s vlastitim error envelopeom).

## Sljedeći koraci

1. Korisnici, Argon2id login, server-side sesije, CSRF, članstva i RBAC; tenant siguran CRUD lokacija/uređaja/senzora.
2. Trajni ingest queue sa statusima i reprocessingom; typed boolean/text mjerenja, history API i retencija.
3. Pull scheduler s leaseom, retryjem i cursorom u PostgreSQL-u; simulator kvarova i restartova.
4. Campbell HTTP adapter nakon potvrde modela, firmwarea, protokola i stvarnog odgovora; SSRF profili i šifrirane tajne.
5. ChirpStack webhook nakon potvrde verzije i encodinga, stvarni fixture i DevEUI/application allowlist. MQTT kasnije.
6. Alarmi, grafovi, karta s atribucijom, audit i administracija; capability kontrolirane naredbe tek uz potvrđen transport.
7. Produkcijski TLS, rate limiting, rotacija tajni, backup/restore, retencija i CI izolacijski/E2E testovi.

Nijedan fizički Campbell model, PakBus, ChirpStack webhook ili downlink nije verificiran niti implementiran. Simulator šalje vlastiti dokumentirani normalizirani JSON. Ne dodavati izmišljene URL-ove ni prazne uspješne stubove.

## Ovisnosti i reproducibilnost

Odabrane kompatibilne linije: Axum 0.8, Tokio 1, SQLx 0.8, React 19, Vite 7, TypeScript 5.9; frontend container koristi Node 22. Dokumentacija konzultirana: https://docs.rs/axum/latest/axum/serve/fn.serve.html , https://docs.rs/sqlx/latest/sqlx/macro.migrate.html i https://vite.dev/guide/ .

`package-lock.json` i `Cargo.lock` zaključavaju ovisnosti; Rust Docker build koristi `--locked`. Docker image tagovi nisu digest-pinani; prije produkcije pinati provjerene digestove i definirati proces ažuriranja.

Status stvarno izvršenih provjera pri isporuci nalazi se u `docs/STATUS.md`.
