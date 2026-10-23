#!/usr/bin/env python3
"""Simulator vlastitog normaliziranog JSON-a; nije izvorni Campbell protokol."""
import argparse, datetime, json, os, pathlib, urllib.request, urllib.error
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--url',default='http://localhost:8080')
p.add_argument('--source',choices=['A','B'],help='Procitaj demo token iz lokalne .env datoteke')
p.add_argument('--message-id');p.add_argument('--value',type=float,default=12.8)
a=p.parse_args();token=os.environ.get('SOURCE_TOKEN','')
if a.source:
    path=pathlib.Path(__file__).resolve().parents[1]/'.env'
    config=dict(line.split('=',1) for line in path.read_text().splitlines() if line and not line.startswith('#') and '=' in line)
    token=config.get('DEMO_TOKEN_'+a.source,'')
if len(token)<32:raise SystemExit('Nedostaje token. Koristi --source A ili --source B, ili postavi SOURCE_TOKEN. Sudo nije potreban.')
now=datetime.datetime.now(datetime.timezone.utc)
body={'schema_version':1,'message_id':a.message_id or 'sim:'+now.isoformat(),'observed_at':now.isoformat(),'measurements':[{'key':'battery_voltage','value':a.value,'unit':'V'}]}
req=urllib.request.Request(a.url+'/api/v1/ingest/push',data=json.dumps(body,allow_nan=False).encode(),headers={'Content-Type':'application/json','Authorization':'Bearer '+token})
try:
    with urllib.request.urlopen(req,timeout=10) as response:print(response.read().decode())
except urllib.error.HTTPError as error:
    print(f'HTTP {error.code}: {error.read().decode()}')
    if error.code==401:print('Provjeri token i pokreni seed ako demo izvori jos nisu kreirani.')
    raise SystemExit(1)
except urllib.error.URLError as error:raise SystemExit(f'API nije dostupan: {error.reason}')
