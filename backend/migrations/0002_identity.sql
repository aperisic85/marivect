-- Migration 0001 is immutable. Upgrade packages deliberately preserve the user's copy.
CREATE TABLE users (
 id uuid PRIMARY KEY, email text NOT NULL UNIQUE CHECK(email=lower(email)),
 password_hash text NOT NULL, active boolean NOT NULL DEFAULT true,
 created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE memberships (
 tenant_id uuid NOT NULL REFERENCES tenants(id), user_id uuid NOT NULL REFERENCES users(id),
 role text NOT NULL CHECK(role IN ('tenant_admin','operator','viewer')),
 PRIMARY KEY(tenant_id,user_id)
);
CREATE INDEX memberships_by_user ON memberships(user_id);
CREATE TABLE sessions (
 token_hash text PRIMARY KEY, user_id uuid NOT NULL REFERENCES users(id),
 csrf_token text NOT NULL, expires_at timestamptz NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX sessions_expiry ON sessions(expires_at);
-- Global identity tables have no runtime grants. Memberships additionally use FORCE RLS.
REVOKE ALL ON users,memberships,sessions FROM PUBLIC,marivect_app;
GRANT SELECT ON users,memberships,tenants TO marivect_auth;
GRANT SELECT,INSERT,DELETE ON sessions TO marivect_auth;
ALTER TABLE memberships ENABLE ROW LEVEL SECURITY;
ALTER TABLE memberships FORCE ROW LEVEL SECURITY;
CREATE POLICY identity_lookup ON memberships FOR SELECT TO marivect_auth USING (true);

CREATE FUNCTION auth_lookup(address text) RETURNS TABLE(id uuid,password_hash text)
 LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT u.id,u.password_hash FROM public.users u WHERE u.email=address AND u.active
$$;
CREATE FUNCTION auth_open(uid uuid, candidate text, csrf text) RETURNS void
 LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 INSERT INTO public.sessions(token_hash,user_id,csrf_token,expires_at)
 SELECT candidate,u.id,csrf,now()+interval '12 hours' FROM public.users u WHERE u.id=uid AND u.active
$$;
CREATE FUNCTION auth_session(candidate text) RETURNS TABLE(user_id uuid,email text,csrf_token text)
 LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT u.id,u.email,s.csrf_token FROM public.sessions s JOIN public.users u ON u.id=s.user_id
 WHERE s.token_hash=candidate AND s.expires_at>now() AND u.active
$$;
CREATE FUNCTION auth_memberships(candidate text) RETURNS TABLE(tenant_id uuid,name text,role text)
 LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT m.tenant_id,t.name,m.role FROM public.sessions s
 JOIN public.users u ON u.id=s.user_id JOIN public.memberships m ON m.user_id=u.id
 JOIN public.tenants t ON t.id=m.tenant_id
 WHERE s.token_hash=candidate AND s.expires_at>now() AND u.active ORDER BY t.name,m.tenant_id
$$;
CREATE FUNCTION auth_close(candidate text) RETURNS void
 LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 DELETE FROM public.sessions WHERE token_hash=candidate
$$;
ALTER FUNCTION auth_lookup(text) OWNER TO marivect_auth;
ALTER FUNCTION auth_open(uuid,text,text) OWNER TO marivect_auth;
ALTER FUNCTION auth_session(text) OWNER TO marivect_auth;
ALTER FUNCTION auth_memberships(text) OWNER TO marivect_auth;
ALTER FUNCTION auth_close(text) OWNER TO marivect_auth;
REVOKE ALL ON FUNCTION auth_lookup(text),auth_open(uuid,text,text),auth_session(text),auth_memberships(text),auth_close(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION auth_lookup(text),auth_open(uuid,text,text),auth_session(text),auth_memberships(text),auth_close(text) TO marivect_app;

-- Device creation is authorized by the API tenant_admin membership before this function.
-- Credential writes additionally require the matching transaction-scoped tenant context.
DROP POLICY credential_lookup ON source_credentials;
CREATE POLICY credential_lookup ON source_credentials FOR SELECT TO marivect_auth USING (true);
CREATE POLICY credential_create ON source_credentials FOR INSERT TO marivect_auth
 WITH CHECK (tenant_id=nullif(current_setting('app.tenant_id',true),'')::uuid);
GRANT INSERT ON source_credentials TO marivect_auth;
CREATE FUNCTION register_source(tid uuid,did uuid,candidate text) RETURNS void
 LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 INSERT INTO public.source_credentials(tenant_id,device_id,token_hash) VALUES(tid,did,candidate)
$$;
ALTER FUNCTION register_source(uuid,uuid,text) OWNER TO marivect_auth;
REVOKE ALL ON FUNCTION register_source(uuid,uuid,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION register_source(uuid,uuid,text) TO marivect_app;
GRANT INSERT ON devices,sensors TO marivect_app;
