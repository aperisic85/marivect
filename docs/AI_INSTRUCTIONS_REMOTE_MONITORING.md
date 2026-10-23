# AI upute: multitenant platforma za nadzor i upravljanje udaljenom opremom

Verzija: 1.0 · Datum: 23. listopada 2026.

> Ovaj dokument je implementacijski zadatak za Codex ili drugog AI programerskog agenta. Može se koristiti kao projektna specifikacija ili kao sadržaj datoteke `AGENTS.md` u korijenu repozitorija. Izradi funkcionalnu aplikaciju, migracije, testove i dokumentaciju prema ovim uputama. Nemoj se zaustaviti na maketi sučelja.

## 1. Cilj i obvezni opseg

Izradi web platformu za nadzor i upravljanje udaljenom opremom. Primarni slučaj uporabe je oprema na moru i obali: plutače, meteorološke i oceanografske postaje, svjetionici, komunikacijski uređaji i pripadajuće napajanje. Model mora ostati primjenjiv i na druge udaljene sustave.

Obvezno podrži:
- Više neovisnih organizacija (tenanata) unutar iste aplikacije.
- Campbell Scientific dataloggere: slanje mjerenja prema API-ju aplikacije (push) i periodički dohvat s uređaja (pull).
- ChirpStack integraciju za prihvat podataka LoRaWAN uređaja, neovisno o Campbell uređajima.
- Dodavanje drugih izvora bez promjene osnovnog modela mjerenja i poslovne logike.
- Aktualna i povijesna mjerenja, grafove, kartu, alarme, status komunikacije i upravljačke naredbe za uređaje koji to stvarno podržavaju.

Push i pull moraju moći postojati istodobno, za različite uređaje, ali i za isti uređaj uz definiranu deduplikaciju. Ne pretpostavljaj da je svaki uređaj dostupan s interneta ili da svaki protokol podržava upravljanje.

## 2. Tehnologije

- Backend: Rust, Axum, Tokio.
- Baza: PostgreSQL; SQLx za upite, migracije i transakcije.
- Frontend: React, TypeScript, Vite.
- HTTP klijent: reqwest; serijalizacija: serde; strukturirani logovi: tracing.
- Frontend dohvat: TanStack Query; obrasci i validacija kroz dosljedno odabrane biblioteke.
- Grafikoni i karta: odaberi održavane biblioteke; karta mora imati zamjenjiv izvor podloge i vidljivu atribuciju.
- Lokalno pokretanje: Docker Compose s bazom, backendom i frontendom.
- API ugovor: OpenAPI, usklađen s implementacijom.

Prije implementacije provjeri službenu dokumentaciju i kompatibilne verzije ovisnosti. Zabilježi ih i koristi lock datoteke. Ne uvodi Redis, Kafka, Kubernetes, TimescaleDB ili mikroservise bez konkretne potrebe. PostgreSQL je početna baza i trajni red poslova. PostGIS i vremensko particioniranje uvodi tek kada ih opravdaju upiti i volumen.

## 3. Način rada AI agenta

1. Pregledaj postojeći repozitorij i postojeće upute; sačuvaj korisnikove promjene.
2. Izradi kratak implementacijski plan i popis pretpostavki. Rutinske odluke donosi samostalno.
3. Implementiraj po fazama, u funkcionalnim vertikalnim cjelinama: migracija → backend → frontend → testovi.
4. Za blokirajuće nepoznanice, poput stvarnog Campbell protokola i formata odgovora, zatraži primjer ili dokumentaciju. U međuvremenu nastavi rad na ugovoru adaptera i simulatoru.
5. Ne izmišljaj Campbell URL-ove, nazive tablica, autentikaciju, PakBus operacije ni ChirpStack sheme. Provjeri dokumentaciju za ciljanu verziju i model.
6. Mock i simulator jasno označi. Ne prikazuj simulaciju kao provjerenu integraciju s fizičkim uređajem.
7. Nakon svake faze navedi što radi, provjerene scenarije i preostala ograničenja.
8. Ne dodaj neimplementirane gumbe i lažne uspješne odgovore. Nedostupne mogućnosti jasno označi.

## 4. Arhitektura

Početno implementiraj modularni monolit. API i pozadinski worker mogu koristiti isti Rust workspace i zasebne procese. Poslovi i naredbe moraju preživjeti restart.

Odvoji module:
- identitet, autentikacija i članstva;
- tenanti i autorizacija;
- lokacije, uređaji i senzori;
- integracije, adapteri i dekoderi;
- prihvat i normalizacija podataka;
- vremenske serije i agregacije;
- scheduler i poslovi;
- alarmi i obavijesti;
- upravljačke naredbe;
- audit i operativni nadzor.

Tok podataka:
`izvor → autentikacija/identifikacija → trajni prijam događaja → dekodiranje i validacija → normalizirana mjerenja → statusi/alarmi → API → frontend`

Svaki adapter prevodi vanjski protokol u zajednički interni ugovor. Jezgra ne smije poznavati Campbell nazive polja ili ChirpStack strukturu poruke. Ne izvoditi proizvoljan korisnički kod za dekodiranje u backend procesu.

## 5. Multitenancy i prava pristupa

Koristi zajedničku bazu i shemu s obveznim `tenant_id` na svim tenant podacima. Globalni korisnici i definicije sustava mogu biti iznimke, izričito dokumentirane.

- Korisnik može biti član više tenanata, s različitim ulogama.
- Aktivni tenant mora biti potvrđen članstvom; vrijednost iz headera, URL-a ili tijela nije dokaz ovlasti.
- Servisni token vezuje se uz tenant i dopuštene uređaje/integracije. Tenant za ingest određuje se iz vjerodajnice i registriranog izvora.
- Svaki repository upit mora imati tenant kontekst.
- PostgreSQL Row Level Security koristi kao dodatnu zaštitu. Tenant kontekst postavi transakcijski (`SET LOCAL` ili ekvivalent) na istoj konekciji na kojoj se izvršavaju upiti.
- Runtime uloga ne smije biti superuser, vlasnik koji zaobilazi politike niti imati `BYPASSRLS`; migracije koriste zasebnu ulogu. Po potrebi koristi `FORCE ROW LEVEL SECURITY`.
- Nedostajući tenant kontekst mora odbiti pristup. Ne ostavljaj kontekst na konekciji u poolu.
- Međusobne veze tenant entiteta zaštiti kompozitnim stranim ključevima `(tenant_id, id)` i odgovarajućim jedinstvenim ograničenjima.
- Izolacija vrijedi za poslove, cache ključeve, SSE događaje, izvoz, alarme, audit i integracijske tajne.

Uloge:
| Uloga | Ovlasti |
| --- | --- |
| tenant_admin | Postavke organizacije, članovi, integracije, uređaji i prava |
| operator | Pregled, potvrda alarma i dopuštene upravljačke naredbe |
| viewer | Pregled podataka i dopušteni izvoz |
| platform_admin | Administracija platforme; pristup tenant podacima samo izričitom, auditiranom procedurom |

Provjeravaj ovlasti na backendu za svaku operaciju. Skrivanje gumba nije autorizacija. Granularno odvoji uređivanje konfiguracije, izvršavanje naredbi i upravljanje tajnama.

## 6. Domenski model

Obuhvati najmanje:
- `tenants`, `users`, `memberships`, sesije i servisne vjerodajnice;
- `sites`: naziv, opis, koordinate, vremenska zona;
- `devices`: lokacija, tip, proizvođač, model, serijski broj, aktivnost i capabilities;
- `sensors`: uređaj, stabilni ključ mjerne veličine, jedinica, tip vrijednosti, granice i metapodaci;
- `integrations`: tip, konfiguracija, stanje, referenca na šifrirane tajne;
- `device_sources`: veza uređaja s integracijom, vanjski identifikator, mapping, prioritet izvora;
- `ingestion_events`: izvorni događaj, identifikator, vrijeme primitka, status obrade i pogreška;
- `measurements`: tenant, uređaj, senzor, izvor, vrijeme mjerenja i primitka, vrijednost i kvaliteta;
- `device_latest_values`: aktualno stanje po senzoru;
- `poll_jobs` i trajni raspored dohvata/cursori;
- `alarm_rules`, `alarm_instances`, prijelazi stanja i potvrde;
- `commands`, pokušaji slanja, potvrde i rezultati;
- `notification_deliveries`, `audit_log`.

Konfiguraciju adaptera i metapodatke možeš spremati kao JSONB, ali validiraj ih prema verzioniranoj shemi. Ključne veze i često filtrirana polja zadrži u relacijskim stupcima. Mjerenja ne pohranjuj samo kao neprozirni JSON.

Podrži brojčane, logičke i tekstualne vrijednosti s jasnim tipom i ograničenjem da je ispunjena odgovarajuća vrijednost. Koristi UTC i `timestamptz`; vremenska zona služi prikazu. Posebno čuvaj `observed_at` i `received_at`.

Primjeri veličina: brzina i smjer vjetra, temperatura zraka i mora, vlaga, tlak, visina/period/smjer valova, brzina/smjer morske struje, napon baterije i panela, struja punjenja/potrošnje, stanje izlaza i GPS položaj. Omogući nove veličine kroz konfiguraciju. Za smjer definiraj referencu i konvenciju; za struje dubinu mjerenja kad je dostupna.

## 7. Zajednički prihvat podataka

Normalizirani događaj sadrži verziju sheme, identitet izvora, identitet vanjskog uređaja, stabilni ID događaja kada postoji, vremena i skup mjerenja. Vanjski nazivi mapiraju se na senzore kroz konfiguraciju.

Primjer ugovora vlastitog push API-ja; ovo nije izvorni Campbell ni ChirpStack format:

```json
{
  "schema_version": 1,
  "message_id": "station-01:boot-7:1842",
  "observed_at": "2026-10-23T08:00:00Z",
  "measurements": [
    {"key": "wind_speed", "value": 7.4, "unit": "m/s"},
    {"key": "air_temperature", "value": 21.3, "unit": "degC"}
  ]
}
```

- Uređaj i tenant određuju se iz ovjerenog endpointa/vjerodajnice. Ne vjeruj proizvoljnom ID-u iz tijela.
- Ograniči veličinu zahtjeva, broj mjerenja, duljinu nizova i vremenski raspon. Odbij NaN/Infinity.
- Konverzije jedinica moraju biti izričite; sačuvaj izvorne podatke prema retenciji.
- Kvaliteta: najmanje `good`, `suspect`, `invalid`, `unknown`. Ne pretvaraj nedostajuće vrijednosti u nulu.
- Neispravan payload zabilježi bez curenja tajni; ne stvaraj tiho nove uređaje iz nepoznatih identifikatora.
- Potvrdi prijam tek nakon trajnog zapisa. `202 Accepted` znači zaprimljeno, ne i uspješno obrađeno. Daj mogućnost provjere statusa.
- Obrada je najmanje jednom, s idempotentnim zapisom i jedinstvenim ograničenjima. Ne obećavaj exactly-once transport.
- ID događaja jedinstven je unutar tenanta/integracije/izvora. Za izvore bez ID-a dokumentiraj izvedeni ključ i ograničenja.
- Push/pull preklapanje ukloni stabilnim identitetom izvornog zapisa gdje je moguć. Nemoj deduplicirati samo po vrijednosti.
- Različite izvore ne spajaj tiho. Odredi autoritativni izvor po senzoru ili dokumentiraj pravilo konflikta.
- Stariji naknadno pristigli zapis ne smije zamijeniti noviju aktualnu vrijednost.

## 8. Campbell Scientific adapteri

### 8.1 Push

Implementiraj HTTPS endpoint za uređaj ili njegov posrednik. Osiguraj zasebne opozive vjerodajnice po uređaju, rotaciju, rate limiting i jasan odgovor za retry. Odredi podržani payload prema stvarnim mogućnostima dataloggera/programa.

Pripremi dokumentiran primjer slanja i simulator. Primjer CRBasic programa izradi tek nakon potvrde ciljanog modela, firmwarea i podržanih funkcija. Ne navodi da isti primjer radi na svim CR300/CR310/CR1000X uređajima bez provjere.

### 8.2 Periodički pull

Po uređaju konfiguriraj: transport/adapter, adresu, autentikaciju, tablicu ili resurs, mapping, interval, timeout, batch limit i cursor.

- Kao prvi stvarni transport implementiraj dokumentirani HTTP(S) pristup ciljnom uređaju ako je podržan. PakBus ostavi zasebnim adapterom, bez tvrdnje da je implementiran.
- Podrži intervale po uređaju, jitter, ograničen broj paralelnih dohvata i samo jedan aktivni dohvat istog izvora.
- Koristi incremental cursor temeljen na stvarnom protokolu; pomakni ga tek nakon trajnog uspješnog zapisa batcha.
- Obradi prekid veze, djelomični odgovor, restart uređaja, reset brojača zapisa, praznu tablicu i zaostale podatke.
- Retry: eksponencijalni backoff s jitterom i konačnim ograničenjem; jedan nedostupan uređaj ne smije blokirati druge.
- Planer mora raditi s više worker instanci: zaključavanje/lease, oporavak nakon isteka i idempotentni završetak.
- Omogući ručni dohvat s istim ovlastima i ograničenjima kao automatski.
- Podrži privatnu mrežu/VPN ili budući lokalni collector koji inicira izlaznu vezu. Ne zahtijevaj javno otvaranje dataloggera.

## 9. ChirpStack / LoRaWAN

Platforma je aplikacijski sloj i ne zamjenjuje ChirpStack LNS ni gateway bridge.

- Za MVP implementiraj HTTP integration/webhook za uplink događaje. MQTT adapter planiraj kao dodatni transport kroz isti ugovor.
- U dokumentaciji navedi točno provjerenu ChirpStack verziju, encoding i oblik događaja. Različite verzije ne pretpostavljaj jednakima.
- Integraciju poveži s tenantom i dopuštenim aplikacijama/DevEUI identifikatorima. LoRaWAN tenant i tenant ove platforme nisu automatski isti pojam.
- Autentificiraj webhook podržanim mehanizmom; tajna mora biti vezana uz integraciju. Za MQTT koristi TLS, ograničene topic ACL-ove i reconnect/backoff.
- Prednost daj dekodiranom objektu ako postoji. Za sirovi payload koristi registriran, verzioniran dekoder i čuvaj fPort/payload potreban za obradu.
- Nepoznat dekoder ili neispravan payload mora završiti jasnim statusom, uz mogućnost ponovne obrade izvornog događaja.
- Čuvaj relevantne radio metapodatke, uključujući RSSI/SNR i gateway podatke kad postoje. To nisu automatski senzorska mjerenja.
- Dedupliciraj prema stabilnom ID-u događaja iz provjerene sheme; ne koristi sam frame counter jer se može resetirati.
- Za LoRaWAN downlink definiraj poseban capability i payload encoder. Red slanja nije potvrda izvršenja, a mrežna potvrda nije dokaz promjene fizičkog izlaza.

## 10. Upravljanje opremom

Svaki uređaj ima eksplicitne capabilities, primjerice `telemetry.push`, `telemetry.pull`, `command.set_output`, `command.read_config`. UI prikazuje samo podržane radnje.

- Naredbe su tipizirane i unaprijed dopuštene; nema proizvoljnog shella, URL-a, CRBasic koda ili proizvoljnog pisanja registara.
- Validiraj parametre, raspon, tenant, ovlasti, aktivnost uređaja i vremenski rok.
- Stanja: `queued`, `dispatching`, `sent`, `acknowledged`, `succeeded`, `failed`, `expired`, `unknown`, `cancelled` uz dokumentirane dopuštene prijelaze.
- Razlikuj potvrdu transporta, potvrdu uređaja i verificiran rezultat. Izostanak potvrde nakon slanja može značiti nepoznat ishod.
- Koristi idempotency key; automatski retry samo za dokazano sigurne/idempotentne operacije. Ne ponavljaj fizičku radnju naslijepo.
- Dodaj TTL i audit: tko, kada, tenant, uređaj, parametri bez tajni, pokušaji i ishod.
- Potencijalno opasne operacije zahtijevaju jasnu potvrdu u UI-ju.
- Campbell write i ChirpStack downlink implementiraj tek uz potvrđen protokol i test. Dok toga nema, capability je isključen; simulator može demonstrirati cijeli tijek.

## 11. Alarmi i stanje uređaja

Podrži pragove, histerezu, minimalno trajanje prekoračenja, izostanak očekivanih podataka, status baterije i grešku integracije. Odvoji komunikacijski status od starosti/kvalitete senzorskih vrijednosti.

Alarm ima pravilo, uređaj/senzor, ozbiljnost, trenutak nastanka, aktivno/razriješeno stanje i zasebnu potvrdu operatera. Potvrda ne znači uklanjanje uzroka. Čuvaj povijest prijelaza; spriječi poplavu duplikata i podrži period održavanja.

Definiraj ponašanje kod zakašnjelih i neispravnih podataka: povijesni unos po zadanim postavkama ne smije otvoriti današnji alarm niti lažno označiti uređaj online. Prag dostupnosti temelji se na očekivanoj učestalosti i uspješnom relevantnom kontaktu, uz prikaz vremena posljednjeg mjerenja i posljednje komunikacije.

Za MVP implementiraj alarme u aplikaciji. Vanjske obavijesti odvoji iza sučelja, s trajnim redom, deduplikacijom i ograničenjem učestalosti. Ne šalji vanjske poruke iz testova.

## 12. API i frontend

Koristi verzionirani API `/api/v1`. Odvoji korisnički API od ingest i integracijskih endpointa. Minimalne cjeline: auth, tenants/memberships, sites, devices, sensors, integrations, measurements/latest/history, alarm rules/instances, polling status, commands, audit.

Za svaki endpoint definiraj autorizaciju, validaciju, statusne kodove i ograničenja. Koristi dosljedan format greške s kodom i request ID-jem, paginaciju i ograničen vremenski raspon. Izvoz mora poštovati tenant, ovlasti i limite. Za velike serije koristi serversku agregaciju; smjerovi trebaju kružnu statistiku, a brojači vlastitu semantiku.

Frontend mora sadržavati:
- prijavu i odabir aktivne organizacije;
- pregled sustava s upozorenjima i vremenom osvježavanja;
- kartu i tablicu lokacija/uređaja s filtrima;
- detalj uređaja: zadnje vrijednosti, grafovi, komunikacija, izvori, alarmi i dopuštene naredbe;
- konfiguraciju integracija i mapiranja polja;
- administraciju korisnika i uloga;
- pregled alarma i audit zapisa prema ovlastima.

Sučelje je responzivno, primarno na hrvatskom, pripremljeno za prijevode. Prikazuj jedinice, vremensku zonu, kvalitetu i starost podataka. Jasno razlikuj loading, prazan rezultat, grešku i zastarjelo stanje. Ne prikazuj tajne nakon spremanja. Kod promjene tenanta očisti/odvoji cache i pretplate kako se podaci prethodnog tenanta ne bi prikazali.

Koristi polling ili SSE za osvježavanje, s tenant autorizacijom i obnovom veze. WebSocket uvodi samo ako postoji potreba za dvosmjernom komunikacijom u pregledniku.

## 13. Sigurnost i pouzdanost

- Ljudske korisnike i uređaje autentificiraj zasebno. Lozinke hashiraj Argon2id ako implementiraš lokalni login.
- Preferiraj server-side sesije s HttpOnly/Secure/SameSite kolačićima; implementiraj CSRF zaštitu za autentificirane mutacije. Ne pohranjuj dugotrajne tokene u localStorage.
- Servisne tokene čuvaj kao hash kada nije potreban povrat izvornog tokena. Vanjske lozinke/API ključeve potrebne za spajanje šifriraj ključem izvan baze.
- Uvedi isteke, opoziv i rotaciju; tajne ne smiju biti u logovima, URL queryju, repozitoriju ili demo podacima.
- Ograniči CORS, zahtjeve, konkurentnost, retry i potrošnju po tenantu.
- Pull konfiguracija je SSRF površina: dopusti samo odobrene transportne profile i ciljeve; kontroliraj DNS, redirecte i pristup metadata/loopback adresama. Legitimne privatne mreže odobravaj kroz eksplicitne administrativne profile.
- TLS provjera ostaje uključena; privatni CA konfigurira se izričito.
- Baza i worker koriste najmanje potrebne ovlasti. Podrži graceful shutdown i oporavak prekinutih poslova.
- Definiraj retenciju sirovih događaja, mjerenja i audita; zadaci čišćenja moraju biti tenant sigurni.
- Logovi i metrike: request ID, job ID, tenant kontekst bez tajni, trajanje dohvata, pogreške, backlog i starost podataka. Izbjegni neograničenu kardinalnost metrika.
- Odvoji liveness i readiness; readiness provjerava potrebne ovisnosti bez otkrivanja internih detalja.

## 14. Testovi i kriteriji prihvata

Obvezno automatiziraj kritične scenarije:
1. Dva tenanta s različitim korisnicima i izvorima; pokušaji čitanja, promjene, naredbe i izvoza tuđeg entiteta su odbijeni.
2. RLS odbija upit bez tenant konteksta i ne propušta kontekst između pool konekcija/transakcija.
3. Push zapis prolazi do baze i UI-ja; ponovljena poruka ne stvara duplikat.
4. Pull simulator daje zapise, prekid, reset i oporavak; cursor ne preskače nezapisane podatke.
5. Dva workera ne izvršavaju isti aktivni dohvat; restart vraća napušten posao.
6. ChirpStack fixture provjerene verzije mapira se u normalizirana mjerenja; nepoznat uređaj i loša vjerodajnica odbijaju se.
7. Preklapanje push/pull i zakašnjeli podaci ne kvare latest vrijednosti.
8. Alarm testira prag, histerezu, trajanje, izostanak podataka, potvrdu i oporavak.
9. Naredbe provjeravaju RBAC, capability, TTL, deduplikaciju i nepoznat ishod bez nesigurnog retryja.
10. Promjena tenanta u frontendu ne ostavlja prethodne podatke; pregled i graf rade na manjim ekranima.

Koristi unit testove za parsiranje i pravila, integracijske testove sa stvarnim PostgreSQL-om za RLS/transakcije i mali broj E2E testova za ključne tijekove. Vanjsku opremu zamijeni determinističkim simulatorom u CI-ju. Prolazak simulatora nije potvrda rada s fizičkim uređajem.

Obvezne provjere: Rust format, Clippy, testovi, frontend typecheck/lint/build te migracije na praznoj bazi. Ne navodi kao izvršenu provjeru koju nisi pokrenuo.

## 15. Faze implementacije

### Faza 1 — Temelj i push vertikala
Tenant izolacija, korisnici/uloge, lokacije, uređaji/senzori, migracije, uređajske vjerodajnice, push API, trajni prijam, normalizacija, latest/history i osnovni frontend. Seed s dva tenanta i izričito označenim demo podacima.

### Faza 2 — Campbell pull
Trajni scheduler, adapter ugovor, simulator, stvarni potvrđeni HTTP(S) adapter, mapping, cursori, retry i status integracije.

### Faza 3 — ChirpStack
Webhook, fixture provjerene verzije, DevEUI mapping, dekodiranje, deduplikacija i radio metapodaci. MQTT po potrebi kao zaseban dodatak.

### Faza 4 — Operativni nadzor i upravljanje
Karta/grafovi, alarmi, audit, trajni red naredbi i simulator naredbi. Aktiviraj stvarne write/downlink mogućnosti samo za provjerene uređaje i protokole.

### Faza 5 — Priprema za produkciju
Opterećenje s dokumentiranim volumenom, retencija, backup i probni restore, migracije pri nadogradnji, operativna dokumentacija, sigurnosne provjere i uklanjanje preostalih demo pretpostavki.

## 16. Isporuka i završna definicija gotovosti

Isporuči izvorni kod, SQL migracije, lock datoteke, `.env.example` bez tajni, Docker/Compose konfiguraciju, OpenAPI, simulatore i fixtureove, testove i README.

Dokumentacija mora opisati:
- lokalno pokretanje jednom dokumentiranom naredbom nakon konfiguracije;
- inicijalnog administratora bez hardkodirane produkcijske lozinke;
- kreiranje tenanta, uređaja, tokena, mappinga i integracije;
- primjere push zahtjeva, pull rasporeda i ChirpStack postavki;
- provjerene modele/verzije/protokole i ograničenja;
- sigurnosni model, retenciju, backup/restore i upravljanje tajnama;
- razliku između simuliranih, implementiranih i fizički verificiranih mogućnosti;
- popis preostalih zadataka i odluka o arhitekturi.

Gotov MVP omogućuje da administrator kreira dvije izolirane organizacije, svaka primi svoje podatke, operater vidi aktualna i povijesna mjerenja te alarm, a uređaji mogu koristiti push, periodički pull ili ChirpStack izvor kroz zajednički model. Upravljanje je dostupno samo tamo gdje capability i stvarni transport to podržavaju.

**Prvi zadatak AI agenta:** pregledaj repozitorij, zabilježi poznate i nepoznate integracijske detalje, predloži minimalnu strukturu i implementiraj fazu 1 s testovima tenant izolacije. Nastavi prema sljedećim fazama uz jasno evidentirane integracijske preduvjete.
