CREATE TABLE tenants (id uuid PRIMARY KEY, name text NOT NULL);
CREATE TABLE devices (
 id uuid NOT NULL, tenant_id uuid NOT NULL REFERENCES tenants(id), name text NOT NULL,
 capabilities text[] NOT NULL DEFAULT '{}', PRIMARY KEY(tenant_id,id)
);
CREATE TABLE sensors (
 id uuid NOT NULL, tenant_id uuid NOT NULL, device_id uuid NOT NULL,
 key text NOT NULL, unit text NOT NULL, value_type text NOT NULL DEFAULT 'number' CHECK(value_type='number'),
 PRIMARY KEY(tenant_id,id), UNIQUE(tenant_id,device_id,key), UNIQUE(tenant_id,device_id,id),
 FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id)
);
CREATE TABLE source_credentials (
 tenant_id uuid NOT NULL, device_id uuid NOT NULL, token_hash text NOT NULL UNIQUE,
 revoked boolean NOT NULL DEFAULT false,
 PRIMARY KEY(tenant_id,device_id), FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id)
);
CREATE TABLE ingestion_events (
 id uuid NOT NULL, tenant_id uuid NOT NULL, device_id uuid NOT NULL,
 message_id text NOT NULL, payload jsonb NOT NULL, status text NOT NULL CHECK(status IN ('processed','failed')),
 received_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(tenant_id,id),
 UNIQUE(tenant_id,device_id,message_id), UNIQUE(tenant_id,device_id,id),
 FOREIGN KEY(tenant_id,device_id) REFERENCES devices(tenant_id,id)
);
CREATE TABLE measurements (
 id bigint GENERATED ALWAYS AS IDENTITY, tenant_id uuid NOT NULL, device_id uuid NOT NULL,
 sensor_id uuid NOT NULL, event_id uuid NOT NULL, observed_at timestamptz NOT NULL,
 received_at timestamptz NOT NULL DEFAULT now(), value_number double precision NOT NULL,
 quality text NOT NULL DEFAULT 'unknown' CHECK(quality IN ('good','suspect','invalid','unknown')),
 PRIMARY KEY(tenant_id,id), UNIQUE(tenant_id,event_id,sensor_id),
 FOREIGN KEY(tenant_id,device_id,sensor_id) REFERENCES sensors(tenant_id,device_id,id),
 FOREIGN KEY(tenant_id,device_id,event_id) REFERENCES ingestion_events(tenant_id,device_id,id)
);
CREATE INDEX measurements_history ON measurements(tenant_id,device_id,sensor_id,observed_at DESC);
CREATE TABLE device_latest_values (
 tenant_id uuid NOT NULL, device_id uuid NOT NULL, sensor_id uuid NOT NULL,
 observed_at timestamptz NOT NULL, received_at timestamptz NOT NULL DEFAULT now(), value_number double precision NOT NULL,
 PRIMARY KEY(tenant_id,sensor_id), FOREIGN KEY(tenant_id,device_id,sensor_id) REFERENCES sensors(tenant_id,device_id,id)
);
-- Narrow credential lookup, separate from tenant data; table is never granted to runtime.
-- SECURITY DEFINER function owns only SELECT and exposes no credential material.
CREATE ROLE marivect_auth NOLOGIN NOSUPERUSER NOBYPASSRLS;
GRANT USAGE ON SCHEMA public TO marivect_auth;
GRANT SELECT ON source_credentials TO marivect_auth;
ALTER TABLE source_credentials ENABLE ROW LEVEL SECURITY;
ALTER TABLE source_credentials FORCE ROW LEVEL SECURITY;
CREATE POLICY credential_lookup ON source_credentials TO marivect_auth USING (true);
CREATE FUNCTION authenticate_source(candidate text) RETURNS TABLE(tenant_id uuid,device_id uuid)
 LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
 SELECT c.tenant_id,c.device_id FROM public.source_credentials c WHERE c.token_hash=candidate AND NOT c.revoked
$$;
ALTER FUNCTION authenticate_source(text) OWNER TO marivect_auth;
REVOKE ALL ON FUNCTION authenticate_source(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION authenticate_source(text) TO marivect_app;
REVOKE ALL ON source_credentials FROM PUBLIC, marivect_app;
DO $$ DECLARE tab text; BEGIN
 FOREACH tab IN ARRAY ARRAY['devices','sensors','ingestion_events','measurements','device_latest_values'] LOOP
  EXECUTE format('ALTER TABLE %I ENABLE ROW LEVEL SECURITY',tab);
  EXECUTE format('ALTER TABLE %I FORCE ROW LEVEL SECURITY',tab);
  EXECUTE format('CREATE POLICY tenant_isolation ON %I USING (tenant_id = nullif(current_setting(''app.tenant_id'',true),)::uuid) WITH CHECK (tenant_id = nullif(current_setting(app.tenant_id,true),)::uuid)',tab);
 END LOOP;
END $$;
GRANT USAGE ON SCHEMA public TO marivect_app;
GRANT SELECT ON devices,sensors,ingestion_events,measurements,device_latest_values TO marivect_app;
GRANT INSERT ON ingestion_events,measurements,device_latest_values TO marivect_app;
GRANT UPDATE ON device_latest_values TO marivect_app;
GRANT USAGE ON ALL SEQUENCES IN SCHEMA public TO marivect_app;
