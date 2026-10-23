-- Run as migration/bootstrap role against a migrated database. Always rolls back fixtures.
BEGIN;
INSERT INTO tenants(id,name) VALUES
 ('10000000-0000-0000-0000-000000000001','identity test A'),
 ('10000000-0000-0000-0000-000000000002','identity test B');
INSERT INTO users(id,email,password_hash) VALUES
 ('10000000-0000-0000-0000-000000000010','identity-test@example.invalid','not-a-real-hash');
INSERT INTO memberships(tenant_id,user_id,role) VALUES
 ('10000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000010','viewer');
INSERT INTO devices(tenant_id,id,name) VALUES
 ('10000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000101','test A'),
 ('10000000-0000-0000-0000-000000000002','10000000-0000-0000-0000-000000000102','test B');
SET LOCAL ROLE marivect_app;
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM devices) THEN RAISE EXCEPTION 'missing-context RLS failed'; END IF;
 PERFORM auth_open('10000000-0000-0000-0000-000000000010','test-session-hash','test-csrf');
 IF (SELECT count(*) FROM auth_session('test-session-hash'))<>1 THEN RAISE EXCEPTION 'session lookup failed'; END IF;
 IF (SELECT count(*) FROM auth_session('unknown'))<>0 THEN RAISE EXCEPTION 'unknown session accepted'; END IF;
 IF (SELECT count(*) FROM auth_memberships('test-session-hash'))<>1 THEN RAISE EXCEPTION 'membership count wrong'; END IF;
 IF EXISTS(SELECT 1 FROM auth_memberships('test-session-hash') WHERE tenant_id='10000000-0000-0000-0000-000000000002') THEN RAISE EXCEPTION 'cross-tenant membership leak'; END IF;
 BEGIN
  PERFORM * FROM sessions;
  RAISE EXCEPTION 'runtime can read sessions directly';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
 PERFORM set_config('app.tenant_id','10000000-0000-0000-0000-000000000001',true);
 IF EXISTS(SELECT 1 FROM devices WHERE tenant_id='10000000-0000-0000-0000-000000000002') THEN RAISE EXCEPTION 'cross-tenant device leak'; END IF;
 PERFORM register_source('10000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000101','test-source-hash-a');
 IF (SELECT tenant_id FROM authenticate_source('test-source-hash-a'))<>'10000000-0000-0000-0000-000000000001'::uuid THEN RAISE EXCEPTION 'source binding wrong'; END IF;
 BEGIN
  PERFORM register_source('10000000-0000-0000-0000-000000000002','10000000-0000-0000-0000-000000000102','test-source-hash-b');
  RAISE EXCEPTION 'cross-tenant credential creation allowed';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
 PERFORM auth_close('test-session-hash');
 IF EXISTS(SELECT 1 FROM auth_session('test-session-hash')) THEN RAISE EXCEPTION 'revoked session valid'; END IF;
END $$;
RESET ROLE;
INSERT INTO sessions(token_hash,user_id,csrf_token,expires_at) VALUES
 ('expired-test-hash','10000000-0000-0000-0000-000000000010','csrf',now()-interval '1 minute');
SET LOCAL ROLE marivect_app;
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM auth_session('expired-test-hash')) THEN RAISE EXCEPTION 'expired session valid'; END IF;
 IF EXISTS(SELECT 1 FROM auth_memberships('expired-test-hash')) THEN RAISE EXCEPTION 'expired session membership leak'; END IF;
END $$;
ROLLBACK;
