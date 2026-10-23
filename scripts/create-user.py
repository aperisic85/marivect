#!/usr/bin/env python3
"""Provision a user/membership through the privileged local tools service."""
import argparse, getpass, os, pathlib, subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--email',required=True)
p.add_argument('--tenant',default='00000000-0000-0000-0000-000000000001')
p.add_argument('--role',choices=['tenant_admin','operator','viewer'],default='tenant_admin')
a=p.parse_args()
password=getpass.getpass('Lozinka (najmanje 12 znakova; za postojeceg korisnika ista lozinka): ')
if not 12<=len(password.encode())<=1024: raise SystemExit('Lozinka mora imati 12–1024 bajta.')
if password!=getpass.getpass('Ponovi lozinku: '):raise SystemExit('Lozinke nisu jednake.')
env=os.environ.copy();env.update(BOOTSTRAP_EMAIL=a.email,BOOTSTRAP_TENANT_ID=a.tenant,BOOTSTRAP_ROLE=a.role)
result=subprocess.run(['docker','compose','--profile','tools','run','--rm','--no-deps','--pull','never','-T','-e','BOOTSTRAP_EMAIL','-e','BOOTSTRAP_TENANT_ID','-e','BOOTSTRAP_ROLE','bootstrap'],input=password+'\n',text=True,env=env,cwd=pathlib.Path(__file__).resolve().parents[1])
raise SystemExit(result.returncode)
