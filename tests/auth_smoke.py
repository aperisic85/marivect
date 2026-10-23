#!/usr/bin/env python3
"""HTTP identity/RBAC tests. Provision isolated test users using Compose bootstrap first."""
import datetime, http.cookiejar, json, os, pathlib, secrets, subprocess, unittest, urllib.request, urllib.error, uuid
ROOT=pathlib.Path(__file__).resolve().parents[1]
CONFIG=dict(line.split('=',1) for line in (ROOT/'.env').read_text().splitlines() if line and not line.startswith('#') and '=' in line) if (ROOT/'.env').exists() else {}
URL=os.environ.get('TEST_API_URL','http://localhost:'+CONFIG.get('HTTP_PORT','8080'))
ORIGIN=os.environ.get('TEST_APP_ORIGIN',URL)
A='00000000-0000-0000-0000-000000000001';B='00000000-0000-0000-0000-000000000002'
def provision(email,password,tenant,role):
    if os.environ.get('TEST_BOOTSTRAP_BINARY'):
        cmd=[os.environ['TEST_BOOTSTRAP_BINARY']]
    else:cmd=['docker','compose','--profile','tools','run','--rm','--no-deps','--pull','never','-T','-e','BOOTSTRAP_EMAIL','-e','BOOTSTRAP_TENANT_ID','-e','BOOTSTRAP_ROLE','bootstrap']
    env=os.environ.copy();env.update(BOOTSTRAP_EMAIL=email,BOOTSTRAP_TENANT_ID=tenant,BOOTSTRAP_ROLE=role)
    subprocess.run(cmd,input=password+'\n',text=True,env=env,cwd=ROOT,check=True,capture_output=True)
class Browser:
    def __init__(self):self.cookies=http.cookiejar.CookieJar();self.opener=urllib.request.build_opener(urllib.request.HTTPCookieProcessor(self.cookies))
    def call(self,path,body=None,csrf=None,origin=ORIGIN):
        headers={'Content-Type':'application/json','Origin':origin}
        if csrf:headers['X-CSRF-Token']=csrf
        r=urllib.request.Request(URL+path,data=None if body is None else json.dumps(body).encode(),headers=headers)
        try:
            with self.opener.open(r,timeout=15) as response:
                return response.status,(json.load(response) if response.status!=204 else None),response.headers
        except urllib.error.HTTPError as response:
            return response.code,None,response.headers
class AuthSmoke(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if os.environ.get('TEST_ADMIN_EMAIL'):
            cls.admin=os.environ['TEST_ADMIN_EMAIL'];cls.viewer=os.environ['TEST_VIEWER_EMAIL'];cls.password=os.environ['TEST_USER_PASSWORD'];return
        cls.password=secrets.token_hex(20)
        cls.admin='test-admin-'+uuid.uuid4().hex+'@example.invalid'
        cls.viewer='test-viewer-'+uuid.uuid4().hex+'@example.invalid'
        provision(cls.admin,cls.password,A,'tenant_admin');provision(cls.viewer,cls.password,B,'viewer')
    def login(self,address):
        browser=Browser();status,body,headers=browser.call('/api/v1/auth/login',{'email':address,'password':self.password})
        self.assertEqual(status,200);self.assertIn('HttpOnly',headers['Set-Cookie']);self.assertIn('SameSite=Strict',headers['Set-Cookie'])
        return browser,body['csrf_token']
    def test_isolation_and_csrf_and_logout(self):
        b,csrf=self.login(self.admin)
        status,me,_=b.call('/api/v1/auth/me');self.assertEqual(status,200)
        self.assertEqual([m['tenant_id'] for m in me['memberships']],[A])
        self.assertEqual(b.call(f'/api/v1/tenants/{A}/devices')[0],200)
        self.assertEqual(b.call(f'/api/v1/tenants/{B}/devices')[0],403)
        self.assertEqual(b.call(f'/api/v1/tenants/{A}/devices/00000000-0000-0000-0000-000000000102/latest')[0],404)
        device={'name':'HTTP test '+uuid.uuid4().hex,'sensors':[{'key':'battery_voltage','unit':'V'}]}
        self.assertEqual(b.call(f'/api/v1/tenants/{A}/devices',device)[0],403)
        self.assertEqual(b.call(f'/api/v1/tenants/{A}/devices',device,csrf,origin='http://untrusted.invalid')[0],403)
        status,created,_=b.call(f'/api/v1/tenants/{A}/devices',device,csrf);self.assertEqual(status,201)
        self.assertGreaterEqual(len(created['source_token']),32)
        # User-created service token must work on the existing push path.
        event={'schema_version':1,'message_id':'http:'+uuid.uuid4().hex,'observed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'measurements':[{'key':'battery_voltage','value':12.7,'unit':'V'}]}
        req=urllib.request.Request(URL+'/api/v1/ingest/push',data=json.dumps(event).encode(),headers={'Authorization':'Bearer '+created['source_token'],'Content-Type':'application/json'})
        with urllib.request.urlopen(req,timeout=15) as response:self.assertEqual(response.status,200)
        status,latest,_=b.call(f"/api/v1/tenants/{A}/devices/{created['device_id']}/latest")
        self.assertEqual(status,200);self.assertEqual(latest['measurements'][0]['value'],12.7)
        viewer,vcsrf=self.login(self.viewer)
        self.assertEqual(viewer.call(f'/api/v1/tenants/{A}/devices')[0],403)
        self.assertEqual(viewer.call(f'/api/v1/tenants/{B}/devices')[0],200)
        self.assertEqual(viewer.call(f'/api/v1/tenants/{B}/devices',device,vcsrf)[0],403)
        old_cookie='; '.join(f'{c.name}={c.value}' for c in b.cookies)
        self.assertEqual(b.call('/api/v1/auth/logout',{},csrf)[0],204)
        replay=urllib.request.Request(URL+'/api/v1/auth/me',headers={'Cookie':old_cookie})
        with self.assertRaises(urllib.error.HTTPError) as expired:
            urllib.request.urlopen(replay,timeout=15)
        self.assertEqual(expired.exception.code,401)
        self.assertEqual(b.call('/api/v1/auth/me')[0],401)
    def test_login_origin_and_wrong_password(self):
        b=Browser();self.assertEqual(b.call('/api/v1/auth/login',{'email':self.admin,'password':self.password},origin='http://untrusted.invalid')[0],403)
        self.assertEqual(b.call('/api/v1/auth/login',{'email':self.admin,'password':'wrong-password'})[0],401)
if __name__=='__main__':unittest.main()
