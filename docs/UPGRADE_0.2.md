# Nadogradnja postojeće instalacije na 0.2

Sačuvaj postojeći projekt i bazu. Novi ZIP raspakiraj **u zasebnu mapu**, npr. `/tmp/marivect-v02`. Ne prepisuj ručno prvu SQL migraciju.

```sh
unzip marivect-skeleton.zip -d /tmp/marivect-v02
python3 /tmp/marivect-v02/marivect/scripts/install-upgrade.py --target ~/Development/marivect
cd ~/Development/marivect
docker compose build api frontend
docker compose up -d --pull never
```

Installer čuva postojeću `.env` i `backend/migrations/0001_foundation.sql` bez izmjena. Prije prepisivanja ostalih izmijenjenih datoteka radi kopije u `.marivect-backups/<vrijeme>/`. Mapa nije za git. Izvršava samo kopiranje, ne mijenja bazu niti pokreće Docker.

**Zašto se čuva 0001?** Prvu migraciju već si ručno ispravio i SQLx pamti checksum njezina točnog sadržaja. Čak i drugo formatiranje iste SQL naredbe mijenja checksum. Nova migracija 0002 dodaje identitet, članstva i sesije; postojeća mjerenja i servisni tokeni ostaju dostupni. Nemoj brisati `_sqlx_migrations`, zaobilaziti checksum niti koristiti `down -v` za nadogradnju.

Nakon uspješne migracije kreiraj svojeg korisnika:

```sh
python3 scripts/create-user.py --email tvoj@email.hr
```

Upiši lozinku od najmanje 12 znakova, dvaput. Zadano se daje `tenant_admin` u demo organizaciji A. E-mail je korisničko ime za lokalni login; ništa se ne šalje e-mailom. Ako organizacija A još ne postoji, prvo pokreni:

```sh
docker compose --profile tools run --rm --pull never seed
```

Za pristup i organizaciji B ponovi s **istim e-mailom i istom lozinkom**:

```sh
python3 scripts/create-user.py --email tvoj@email.hr --tenant 00000000-0000-0000-0000-000000000002 --role viewer
```

Isti alat može dodijeliti `operator` ili `tenant_admin` ulogu. Postojećem korisniku ne resetira lozinku. Uloge se mijenjaju samo namjernim lokalnim provisioningom; web administracija članstava još nije implementirana.

Otvori `http://localhost:8080`, prijavi se, odaberi organizaciju i pregledaj uređaje. U organizaciji gdje si administrator možeš dodati uređaj i njegove brojčane senzore. Token se prikazuje jednom — spremi ga prije zatvaranja obrasca.

Simulator više ne zahtijeva učitavanje `.env` u terminal:

```sh
python3 scripts/simulate-push.py --source A
python3 scripts/simulate-push.py --source B --value 13.1
```

Za novi uređaj postavi `SOURCE_TOKEN` na njegov token. Početni obrazac kreira senzor `battery_voltage,V`, koji simulator šalje.

Ako se javi greška:

```sh
docker compose logs --no-color migrate api
```

Ako se pojavi **checksum mismatch**, provjeri da je u postojećem projektu ostala upravo tvoja prethodno primijenjena datoteka 0001. Možeš je vratiti iz vlastite kopije projekta; ne mijenjaj evidenciju baze.

Za izlazak iz lokalnog HTTP načina trebaš HTTPS reverse proxy i Compose override s `COOKIE_SECURE: "true"` te `APP_ORIGIN` postavljenim na točan origin preglednika. Default je namjerno `http://localhost:<HTTP_PORT>`; pristup preko IP adrese bez izmjene origina odbija login zbog CSRF zaštite.
