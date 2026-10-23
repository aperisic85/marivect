#!/usr/bin/env python3
"""Generate local development credentials; never overwrite existing .env."""
from pathlib import Path
import secrets
path = Path(__file__).resolve().parents[1] / '.env'
with path.open('x') as stream:
    for key in ['POSTGRES_PASSWORD','APP_DB_PASSWORD','DEMO_TOKEN_A','DEMO_TOKEN_B']:
        stream.write(f'{key}={secrets.token_hex(32)}\n')
    stream.write('HTTP_PORT=8080\n')
path.chmod(0o600)
print('Created .env with random development credentials.')
