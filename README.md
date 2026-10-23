# Marivect — razvojna verzija 0.2

Multitenant nadzor udaljene opreme: Rust/Axum/Tokio + SQLx/PostgreSQL 17 + React/TypeScript/Vite. Docker i Docker Compose dio su osnovnog načina rada.

**Postojeća instalacija 0.1:** prati [docs/UPGRADE_0.2.md](docs/UPGRADE_0.2.md). Nadogradnja čuva `.env`, prvu primijenjenu migraciju i postojeće podatke. Ne briši bazu.

## Nova instalacija

Preduvjeti: Docker Engine/Desktop s Compose v2 i Python 3. Lokalni Rust i Node nisu potrebni.

```sh
python3 scripts/init-env.py
docker compose build api frontend
docker compose up -d --pull never
docker compose --profile tools run --rm --pull never seed
python3 scripts/create-user.py --email tvoj@email.hr
```

Otvori **http://localhost:8080** i prijavi se lozinkom koju si postavio. Seed stvara dvije demo organizacije i uređaje, bez lažnih mjerenja. `create-user.py` stvara korisnika/admina organizacije A, a može dodati članstvo i u drugoj organizaciji. Detalji su u uputama za nadogradnju.

```sh
python3 scripts/simulate-push.py --source A
python3 scripts/simulate-push.py --source B --value 13.1
```

Za ove naredbe ne trebaju `sudo`, `source .env` ni korisnički login. Servisni token je zaseban identitet uređaja. Simulator s `--source` čita demo token iz lokalne `.env`; za ručno dodan uređaj koristi `SOURCE_TOKEN` varijablu.

## Implementirano

- Compose servisi: PostgreSQL, jednokratni migrator, API, worker, Nginx/frontend; tools profili za seed, korisnički bootstrap i Rust provjere.
- Servisni image gradi se lokalno. `pull_policy: never` uklanja pokušaj njegova preuzimanja s Docker Huba. Vanjske bazne slike i dalje se preuzimaju tijekom builda.
- Runtime DB uloga nije superuser, vlasnik tablica ni BYPASSRLS. Tenant podaci imaju FORCE RLS i kompozitne strane ključeve; tenant kontekst je transakcijski.
- Argon2id lozinke, server-side sesije od 12 sati, hashirani session tokeni, HttpOnly/SameSite=Strict cookie i CSRF/Origin provjera mutacija. Tokeni se ne spremaju u localStorage.
- Login limit na Nginxu (5/min uz burst 10 po IP-u) i najviše 4 istodobne provjere lozinki u API-ju.
- Lokalni provisioning korisnika i članstava s ulogama `tenant_admin`, `operator`, `viewer`. Nema javne registracije niti hardkodirane administratorske lozinke.
- Web login/logout, odabir organizacije, popis uređaja i latest mjerenja. Promjena tenanta uklanja prethodne tenant queryje i resetira detalj/obrazac.
- Administrator može dodati uređaj, do 128 brojčanih senzora i servisni token prikazan jednom. Viewer i operator imaju samo pregled u ovoj fazi. API provjerava prava neovisno o UI-ju.
- Normalizirani push, validacija ključeva i jedinica, trajni događaji/mjerenja, deduplikacija po tenant/uređaj/message_id i konflikt 409 za drugačiji payload istog ID-a.
- Stariji ili jednako star zapis ne mijenja noviji latest. Povijesni zapisi ostaju u bazi, ali history API i grafovi još nisu dodani.
- Latest osvježavanje svakih 15 s; nepostojeće mjerenje je null, ne nula. Razvojni prag zastarjelosti 5 min nije komunikacijski online/offline status.
- Adapter ugovor i worker heartbeat; nema lažnog dohvata ni naredbi.

## Struktura

- `backend/src/auth.rs`: sesije, lozinke, CSRF i članstva.
- `backend/src/console.rs`: autorizirani pregled i kreiranje uređaja.
- `backend/src/bin/`: API, worker, migrator, demo seed i lokalni bootstrap.
- `backend/migrations/`: nepromjenjivi SQL koraci; nova funkcionalnost ide u nove migracije.
- `frontend/src/`: React konzola na hrvatskom.
- `scripts/`: priprema okruženja, nadogradnja, korisnici i simulator.
- `tests/`: unit testovi u Rustu, SQL izolacijski scenariji i HTTP testovi.
- `docs/`: puna specifikacija, OpenAPI, upgrade i status provjera.

## Provjere

Uz pokrenut i seedan stack:

```sh
docker compose config --quiet
docker compose --profile tools run --build --rm checks
python3 tests/smoke.py
docker compose exec -T db psql -U marivect_migrator -d marivect -v ON_ERROR_STOP=1 < tests/identity.sql
python3 tests/auth_smoke.py
```

`auth_smoke.py` kreira zasebne nasumične testne korisnike i testni uređaj, provjerava login, cookie, CSRF, RBAC, tenant izolaciju, push i logout. Koristi isključivo razvojnu/testnu bazu. SQL test rollbacka fixtureove. GitHub CI uključuje oba skupa testova.

Frontend provjera: `docker compose build frontend` radi `npm ci`, TypeScript i Vite build. Rust checks rade fmt, Clippy i unit testove. Frontend lint i pravi browser E2E nisu dio osnovnog CI-ja. Izvršene provjere i ograničenja su u `docs/STATUS.md`.

## Sigurnost i operacije

Sustav je još razvojna verzija. Compose izlaže samo Nginx na `127.0.0.1:<HTTP_PORT>`; baza i API nemaju host port. Lokalni HTTP cookie ima `COOKIE_SECURE=false` u Composeu. Kod API-ja zadana vrijednost je Secure=true ako varijabla nije eksplicitno false. Produkcija zahtijeva TLS i točan `APP_ORIGIN`. Iza drugog reverse proxyja pravilno definirati login rate limiting; ne vjerovati proizvoljnom klijentskom IP headeru.

Globalne identity tablice nemaju runtime table grant. Uske SECURITY DEFINER funkcije omogućuju samo potrebne operacije. Memberships imaju posebnu auth RLS SELECT politiku radi razrješavanja članstava prije odabira tenanta. Credentials imaju auth SELECT lookup i INSERT politiku koja zahtijeva odgovarajući tenant kontekst. Runtime ne čita hashove servisnih tokena izravno.

Lokalni migrator/bootstrap koristi privilegiranu inicijalnu DB ulogu; produkcija treba odvojeni provisioning i kontrolirano izvođenje migracija. `bootstrap` prima lozinku na stdin. `.env` se ne commita. Generirane lozinke za bazu vrijede pri inicijalizaciji volumea; promjena `.env` ne rotira postojeće DB lozinke. `seed` namjerno rotira demo tokene prema `.env` i ostaje development-only.

Sesije se mogu opozvati logoutom; deaktivacija korisnika ili promjena članstva djeluje pri sljedećem API zahtjevu. Sesije nemaju sliding expiry. Reset lozinke, web upravljanje članstvima, cleanup isteklih sesija, audit, retencija i lifecycle servisnih tokena još nisu implementirani. Ako se odgovor za kreiranje uređaja izgubi nakon commita ili se izgubi prikazani token, UI ga ne može ponovno prikazati; samostalni siguran tok rotacije slijedi u idućoj fazi.

Ingest je sinkron i vraća 200 tek nakon uspješne transakcije, a ne asinkroni 202. Payload je ograničen na 64 KiB / 128 mjerenja. Nije implementiran puni ingest rate limiting. Neispravan JSON koristi standardni Axum rejection; vlastite greške imaju code i request_id. Postojeći source/latest endpoint ostaje radi razvojne kompatibilnosti i dopušta pristup samo uređaju vezanom uz servisni token.

```sh
docker compose logs -f api worker
docker compose down
```

Named volume `pgdata` ostaje nakon `down`. **`down -v` briše bazu.** Backup/restore i test oporavka treba dodati prije produkcije.

## Nastavak

1. Tenant siguran CRUD lokacija i proširena administracija uređaja/senzora; sigurna rotacija i opoziv servisnih tokena, audit.
2. History API i grafovi; tekst/logičke vrijednosti, retencija i asinkroni ingest sa statusima.
3. Trajni scheduler i pull simulator; potvrđeni Campbell HTTP adapter tek nakon poznatog modela/firmwarea, formata i sigurnih mrežnih profila.
4. Potvrđena ChirpStack verzija, webhook fixture, decoder/mapping i DevEUI/application allowlist.
5. Alarmi, karta s atribucijom, obavijesti i provjerene capability naredbe.

Nijedan fizički Campbell model, PakBus, ChirpStack webhook ili downlink nije verificiran niti implementiran. Simulator koristi vlastiti JSON ugovor.

## Ovisnosti

Axum 0.8, Tokio 1, SQLx 0.8, Argon2 0.5.3, React 19, Vite 7 i TypeScript 5.9. Docker frontend koristi Node 22. Cargo/npm lock datoteke su uključene; Rust build koristi `--locked`. Docker slike još nisu digest-pinane.

Službene reference: https://docs.rs/argon2/0.5.3/argon2/ , https://docs.rs/axum/latest/ , https://www.postgresql.org/docs/17/ddl-rowsecurity.html , https://vite.dev/guide/ .
