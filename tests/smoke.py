#!/usr/bin/env python3
"""Integration tests against a real Compose stack, seeded with both demo sources."""
import datetime, json, pathlib, subprocess, unittest, urllib.request, urllib.error
ROOT=pathlib.Path(__file__).resolve().parents[1]
env=dict(line.split('=',1) for line in (ROOT/'.env').read_text().splitlines() if line and not line.startswith('#'))
URL='http://localhost:'+env.get('HTTP_PORT','8080')
def call(path,token,payload=None):
    request=urllib.request.Request(URL+path,data=None if payload is None else json.dumps(payload).encode(),headers={'Authorization':'Bearer '+token,'Content-Type':'application/json'})
    try:
        with urllib.request.urlopen(request,timeout=10) as r: return r.status,json.load(r)
    except urllib.error.HTTPError as r: return r.code,None
class Integration(unittest.TestCase):
    def test_01_push_isolation_duplicates_and_late_data(self):
        now=datetime.datetime.now(datetime.timezone.utc)
        stamp=now.isoformat()
        event={'schema_version':1,'message_id':'test:'+stamp,'observed_at':stamp,'measurements':[{'key':'battery_voltage','value':13.2,'unit':'V'}]}
        status,first=call('/api/v1/ingest/push',env['DEMO_TOKEN_A'],event)
        self.assertEqual(status,200)
        status,second=call('/api/v1/ingest/push',env['DEMO_TOKEN_A'],event)
        self.assertEqual(status,200);self.assertTrue(second['duplicate']);self.assertEqual(first['event_id'],second['event_id'])
        event['measurements'][0]['value']=14
        self.assertEqual(call('/api/v1/ingest/push',env['DEMO_TOKEN_A'],event)[0],409)
        event['message_id']+=':old';event['observed_at']=(now-datetime.timedelta(days=1)).isoformat()
        self.assertEqual(call('/api/v1/ingest/push',env['DEMO_TOKEN_A'],event)[0],200)
        _,a=call('/api/v1/source/latest',env['DEMO_TOKEN_A'])
        _,b=call('/api/v1/source/latest',env['DEMO_TOKEN_B'])
        self.assertNotEqual(a['tenant_id'],b['tenant_id']);self.assertNotEqual(a['device_id'],b['device_id'])
        self.assertEqual(a['measurements'][0]['value'],13.2)
        self.assertEqual(call('/api/v1/source/latest','x'*64)[0],401)
        event['tenant_id']=b['tenant_id']
        self.assertEqual(call('/api/v1/ingest/push',env['DEMO_TOKEN_A'],event)[0],422)
    def test_02_rls_without_context_and_transaction_reset(self):
        sql="""BEGIN;
SELECT set_config('app.tenant_id','00000000-0000-0000-0000-000000000001',true);
DO $$ BEGIN
 IF (SELECT count(*) FROM devices) <> 1 THEN RAISE EXCEPTION 'wrong tenant visibility'; END IF;
 IF EXISTS(SELECT 1 FROM devices WHERE tenant_id='00000000-0000-0000-0000-000000000002') THEN RAISE EXCEPTION 'tenant leak'; END IF;
END $$;
COMMIT;
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM devices) THEN RAISE EXCEPTION 'transaction context leaked'; END IF;
 IF (SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user) THEN RAISE EXCEPTION 'unsafe runtime role'; END IF;
END $$;
"""
        result=subprocess.run(['docker','compose','exec','-T','db','sh','-c','PGPASSWORD="$APP_DB_PASSWORD" psql -h 127.0.0.1 -U marivect_app -d marivect -v ON_ERROR_STOP=1'],input=sql,text=True,cwd=ROOT,capture_output=True)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
if __name__=='__main__': unittest.main()
