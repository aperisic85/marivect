#!/usr/bin/env python3
"""Copy this release into an existing checkout, preserving .env and migration 0001."""
import argparse, datetime, pathlib, shutil
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--target',required=True);a=p.parse_args()
source=pathlib.Path(__file__).resolve().parents[1];target=pathlib.Path(a.target).expanduser().resolve()
if source==target or target.is_relative_to(source) or source.is_relative_to(target):raise SystemExit('Raspakiraj novu verziju u zasebnu mapu, izvan postojeceg projekta.')
if not (target/'compose.yaml').is_file() or not (target/'backend/migrations/0001_foundation.sql').is_file():raise SystemExit('Cilj nije postojeci Marivect projekt.')
backup=target/'.marivect-backups'/datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
excluded={'node_modules','target','dist','__pycache__','.git','.marivect-backups'}
changed=0
for src in sorted(source.rglob('*')):
    relative=src.relative_to(source)
    if any(part in excluded for part in relative.parts) or not src.is_file() or src.is_symlink():continue
    if str(relative) in {'.env','backend/migrations/0001_foundation.sql'}:continue
    dst=target/relative
    if dst.is_symlink():raise SystemExit(f'Odbijam prepisati simbolicku poveznicu: {relative}')
    if any(parent.is_symlink() for parent in dst.parents if parent.is_relative_to(target)):raise SystemExit(f'Putanja sadrzi simbolicku poveznicu: {relative}')
    if dst.is_file() and dst.read_bytes()==src.read_bytes():continue
    if dst.exists():
        saved=backup/relative;saved.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(dst,saved)
    dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst);changed+=1
print(f'Nadogradeno datoteka: {changed}. .env i migracija 0001 ostale su nepromijenjene.')
if backup.exists():print(f'Prethodne datoteke: {backup}')
print('Sada u ciljnoj mapi pokreni: docker compose build api frontend && docker compose up -d --pull never')
