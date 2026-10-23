#!/usr/bin/env python3
"""Send OUR normalized JSON; not a CRBasic or native Campbell simulator."""
import argparse, datetime, json, os, urllib.request
p=argparse.ArgumentParser();p.add_argument('--url',default='http://localhost:8080');p.add_argument('--message-id');p.add_argument('--value',type=float,default=12.8);args=p.parse_args()
now=datetime.datetime.now(datetime.timezone.utc)
body={'schema_version':1,'message_id':args.message_id or 'sim:'+now.isoformat(),'observed_at':now.isoformat(),'measurements':[{'key':'battery_voltage','value':args.value,'unit':'V'}]}
request=urllib.request.Request(args.url+'/api/v1/ingest/push',data=json.dumps(body).encode(),headers={'Content-Type':'application/json','Authorization':'Bearer '+os.environ['SOURCE_TOKEN']})
with urllib.request.urlopen(request,timeout=10) as response: print(response.read().decode())
