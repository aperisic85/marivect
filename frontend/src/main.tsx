import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query';
import './style.css';
const client=new QueryClient({defaultOptions:{queries:{retry:false}}});
type Member={tenant_id:string;name:string;role:'tenant_admin'|'operator'|'viewer'};
type Me={user:{email:string;csrf_token:string};memberships:Member[]};
type Device={id:string;name:string;capabilities:string[]};
type Reading={key:string;unit:string;value:number|null;observed_at:string|null;received_at:string|null;quality:string};
const roles={tenant_admin:'Administrator',operator:'Operater',viewer:'Pregled'};
async function api<T>(url:string,options:RequestInit={},signal?:AbortSignal):Promise<T>{
 const r=await fetch(url,{...options,signal,credentials:'same-origin'});
 if(!r.ok){const b=await r.json().catch(()=>null);const code=b?.error?.code;
 if(r.status===401&&!url.endsWith('/login')){client.removeQueries({queryKey:['tenant']});void client.invalidateQueries({queryKey:['me']});}
 const messages:Record<string,string>={invalid_login:'E-mail ili lozinka nisu ispravni.',invalid_origin:'Otvori aplikaciju preko localhost adrese iz README-a.',csrf_required:'Osvježi stranicu i pokušaj ponovno.',admin_required:'Potreban je administrator.',tenant_access_denied:'Nemaš pristup organizaciji.',invalid_device:'Provjeri naziv i ključeve senzora.',login_required:'Sesija je istekla. Prijavi se ponovno.'};
 throw new Error(messages[code]??(r.status===429?'Previše pokušaja. Pričekaj.':`Zahtjev nije uspio (${r.status}).`));}
 return r.status===204?undefined as T:r.json();
}
function post(body:unknown,csrf?:string):RequestInit{return {method:'POST',headers:{'Content-Type':'application/json',...(csrf?{'X-CSRF-Token':csrf}:{})},body:JSON.stringify(body)};}
function Login({done}:{done:()=>void}){
 const [email,setEmail]=useState(''),[password,setPassword]=useState(''),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 return <section className="login"><div className="eyebrow">DOBRO DOŠLI</div><h2>Prijava u Marivect</h2><p>Organizacije i udaljena oprema na jednom mjestu.</p><form onSubmit={async e=>{e.preventDefault();setBusy(true);setError('');try{await api('/api/v1/auth/login',post({email,password}));setPassword('');done();}catch(e){setError((e as Error).message);}finally{setBusy(false);}}}>
 <label htmlFor="email">E-mail</label><input id="email" type="email" autoComplete="username" required maxLength={254} value={email} onChange={e=>setEmail(e.target.value)}/>
 <label htmlFor="password">Lozinka</label><input id="password" type="password" autoComplete="current-password" required maxLength={1024} value={password} onChange={e=>setPassword(e.target.value)}/>
 {error&&<p role="alert">{error}</p>}<button disabled={busy}>{busy?'Prijava…':'Prijavi se'}</button></form><p className="hint">Prvi korisnik: <code>python3 scripts/create-user.py --email tvoj@email.hr</code></p></section>;
}
function Values({tenant,device}:{tenant:string;device:Device}){
 const q=useQuery<{measurements:Reading[]}>({queryKey:['tenant',tenant,'device',device.id],queryFn:({signal})=>api(`/api/v1/tenants/${tenant}/devices/${device.id}/latest`,{},signal),refetchInterval:15000});
 return <section><div className="section-title"><div><div className="eyebrow">DETALJ UREĐAJA</div><h2>{device.name}</h2></div><span className="pill">Push API</span></div><p className="hint">{device.id}</p>
 {q.isPending&&<p role="status">Učitavam mjerenja…</p>}{q.isError&&<p role="alert">{q.error.message}</p>}{q.data&&<><p className="hint">Osvježeno {new Date(q.dataUpdatedAt).toLocaleTimeString('hr-HR')} · svakih 15 s · {Intl.DateTimeFormat().resolvedOptions().timeZone}</p><div className="grid">{q.data.measurements.map(m=><article key={m.key}><span>{m.key}</span><h2>{m.value??'—'} <small>{m.unit}</small></h2><p>{m.observed_at?new Date(m.observed_at).toLocaleString('hr-HR'):'Podaci još nisu primljeni.'}</p>{m.received_at&&<p className="hint">Primljeno: {new Date(m.received_at).toLocaleString('hr-HR')}</p>}<p className="hint">Kvaliteta: {m.quality}</p>{m.observed_at&&Date.now()-Date.parse(m.observed_at)>300000&&<b>Zastarjelo mjerenje (&gt; 5 min, razvojni prag)</b>}</article>)}</div></>}
 </section>;
}
function Create({tenant,csrf,done}:{tenant:string;csrf:string;done:()=>void}){
 const [name,setName]=useState(''),[sensors,setSensors]=useState('battery_voltage,V'),[token,setToken]=useState(''),[error,setError]=useState(''),[busy,setBusy]=useState(false);
 return <section><h2>Dodaj uređaj</h2><p>Brojčani senzori, normalizirani push API. Ključevi: slova, brojevi i donja crta.</p>{token?<div className="token-box"><b>Spremi servisni token — prikazuje se samo sada.</b><p>Token pripada upravo kreiranom uređaju.</p><textarea aria-label="Servisni token" readOnly value={token}/><button className="secondary" onClick={()=>setToken('')}>Spremio sam token, zatvori</button></div>:
 <form onSubmit={async e=>{e.preventDefault();setBusy(true);setError('');try{const parsed=sensors.split('\n').filter(l=>l.trim()).map(l=>{const p=l.split(',');if(p.length!==2)throw new Error('Svaki red mora biti: ključ,jedinica');return {key:p[0].trim(),unit:p[1].trim()};});const r=await api<{source_token:string}>(`/api/v1/tenants/${tenant}/devices`,post({name,sensors:parsed},csrf));setToken(r.source_token);setName('');done();}catch(e){setError((e as Error).message);}finally{setBusy(false);}}}>
 <label htmlFor="name">Naziv uređaja</label><input id="name" required maxLength={120} value={name} onChange={e=>setName(e.target.value)}/><label htmlFor="sensors">Senzori — jedan ključ,jedinica po retku</label><textarea id="sensors" required value={sensors} onChange={e=>setSensors(e.target.value)}/><p className="hint">Primjer: battery_voltage,V ili air_temperature,degC</p>{error&&<p role="alert">{error}</p>}<button disabled={busy}>{busy?'Spremanje…':'Kreiraj uređaj i token'}</button></form>}</section>;
}
function Board({member,csrf}:{member:Member;csrf:string}){
 const tenant=member.tenant_id;const [selected,setSelected]=useState(''),[adding,setAdding]=useState(false);
 const q=useQuery<{devices:Device[];limit:number}>({queryKey:['tenant',tenant,'devices'],queryFn:({signal})=>api(`/api/v1/tenants/${tenant}/devices`,{},signal),refetchInterval:30000});
 const device=q.data?.devices.find(d=>d.id===selected)??q.data?.devices[0];
 return <><section><div className="section-title"><div><div className="eyebrow">UDALJENA OPREMA</div><h2>{member.name}</h2></div>{member.role==='tenant_admin'&&<button className="secondary" onClick={()=>setAdding(!adding)}>{adding?'Zatvori obrazac':'+ Dodaj uređaj'}</button>}</div>
 {q.isPending&&<p role="status">Učitavam uređaje…</p>}{q.isError&&<p role="alert">{q.error.message}</p>}{q.data&&<><p className="hint">Uloga: {roles[member.role]} · uređaji: {q.data.devices.length}{q.data.devices.length===q.data.limit?' · prikaz ograničen na 200':''}</p>{!q.data.devices.length&&<p>Nema uređaja u ovoj organizaciji.</p>}<div className="device-list">{q.data.devices.map(d=><button key={d.id} className={d.id===device?.id?'device selected':'device'} onClick={()=>setSelected(d.id)}><strong>{d.name}</strong><span>{d.capabilities.join(' · ')}</span></button>)}</div></>}
 </section>{adding&&member.role==='tenant_admin'&&<Create tenant={tenant} csrf={csrf} done={()=>void client.invalidateQueries({queryKey:['tenant',tenant,'devices']})}/>}{device&&<Values key={device.id} tenant={tenant} device={device}/>}</>;
}
function App(){
 const [tenant,setTenant]=useState(''),[error,setError]=useState('');
 const me=useQuery<Me|null>({queryKey:['me'],queryFn:async({signal})=>{const r=await fetch('/api/v1/auth/me',{signal,credentials:'same-origin'});if(r.status===401)return null;if(!r.ok)throw new Error('Poslužitelj nije dostupan.');return r.json();},refetchInterval:60000});
 const active=me.data?.memberships.find(m=>m.tenant_id===tenant)??me.data?.memberships[0];
 function refresh(){client.removeQueries({queryKey:['tenant']});client.setQueryData(['me'],null);setTenant('');void client.invalidateQueries({queryKey:['me']});}
 return <main><header><div className="mark">M</div><div><h1>Marivect</h1><p>Nadzor udaljene opreme</p></div><span className="badge">RAZVOJ 0.2</span></header>
 {me.isPending?<section role="status">Provjeravam sesiju…</section>:me.isError?<section role="alert">{me.error.message}<button onClick={()=>void me.refetch()}>Pokušaj ponovno</button></section>:!me.data?<Login done={refresh}/>:
 <><section className="toolbar"><div><strong>{me.data.user.email}</strong><p className="hint">{active?roles[active.role]:'Nema članstva'}</p></div><div><label htmlFor="tenant">Organizacija</label><select id="tenant" value={active?.tenant_id??''} onChange={e=>{void client.cancelQueries({queryKey:['tenant']});client.removeQueries({queryKey:['tenant']});setTenant(e.target.value);}}>{me.data.memberships.map(m=><option key={m.tenant_id} value={m.tenant_id}>{m.name}</option>)}</select></div><button className="secondary" onClick={async()=>{setError('');try{await api('/api/v1/auth/logout',post({},me.data!.user.csrf_token));refresh();}catch(e){setError((e as Error).message);}}}>Odjava</button>{error&&<p role="alert">{error}</p>}</section>
 {active?<Board key={active.tenant_id} member={active} csrf={me.data.user.csrf_token}/>:<section>Nema članstava. Administrator treba dodijeliti organizaciju.</section>}</>}
 <footer>Marivect · sesije i tenant izolacija · Campbell pull, ChirpStack i alarmi slijede u idućim fazama.</footer></main>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><QueryClientProvider client={client}><App/></QueryClientProvider></React.StrictMode>);
