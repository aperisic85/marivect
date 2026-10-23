import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query';
import './style.css';
const client = new QueryClient();
type Reading = {key:string;value:number;unit:string;observed_at:string;received_at:string;quality:string};
type Latest = {tenant_id:string;device_id:string;measurements:Reading[]};
function Dashboard({token}:{token:string}) {
  const query = useQuery<Latest>({queryKey:['source',token], queryFn: async ({signal}) => {
    const response = await fetch('/api/v1/source/latest', {signal,headers:{Authorization:`Bearer ${token}`}});
    if (!response.ok) throw new Error(response.status === 401 ? 'Neispravna vjerodajnica.' : `Dohvat nije uspio (${response.status}).`);
    return response.json();
  }, refetchInterval:15000, retry:false});
  if (query.isPending) return <p role="status">Dohvaćam podatke…</p>;
  if (query.isError) return <p role="alert">{query.error.message}</p>;
  return <><p>Organizacija: <code>{query.data.tenant_id}</code></p><p>Uređaj: <code>{query.data.device_id}</code></p>
    <p>Zadnje osvježavanje: {new Date(query.dataUpdatedAt).toLocaleTimeString('hr-HR')} · automatski svakih 15 s</p>
    {!query.data.measurements.length && <p>Nema mjerenja. Pokreni push simulator prema README uputama.</p>}
    <div className="grid">{query.data.measurements.map(m=><article key={m.key}><span>{m.key}</span><h2>{m.value} <small>{m.unit}</small></h2><p>Mjerenje: {new Date(m.observed_at).toLocaleString('hr-HR')}</p><p>Primitak: {new Date(m.received_at).toLocaleString('hr-HR')}</p><p>Kvaliteta: {m.quality}</p>{Date.now()-Date.parse(m.observed_at)>300000 && <b>Zastarjelo mjerenje (&gt; 5 min, demo prag)</b>}</article>)}</div></>;
}
function App(){
  const [token,setToken]=useState(''); const [draft,setDraft]=useState('');
  return <main><header><div className="mark">M</div><div><h1>Marivect</h1><p>Nadzor udaljene opreme</p></div><span className="badge">SKELETON 0.1</span></header>
    <section><h2>Razvojna konzola izvora</h2><p>Ovo je provjera push vertikale pomoću servisnog tokena uređaja. Korisnička prijava i administracija još nisu implementirane.</p>
    <form onSubmit={e=>{e.preventDefault();client.clear();setToken(draft.trim());setDraft('');}}><label htmlFor="token">Servisni token iz lokalne .env datoteke</label><div className="row"><input id="token" type="password" autoComplete="off" value={draft} onChange={e=>setDraft(e.target.value)} required minLength={32}/><button>Otvori izvor</button></div></form>
    {token && <button className="secondary" onClick={()=>{setToken('');client.clear();}}>Zatvori izvor</button>}</section>
    <section>{token ? <Dashboard token={token}/> : <p>Unesi token za pregled podataka pripadajućeg uređaja. Token se čuva samo u memoriji ove stranice.</p>}</section>
    <footer>Campbell pull · ChirpStack · alarmi · upravljanje — predviđeni za sljedeće faze.</footer></main>;
}
createRoot(document.getElementById('root')!).render(<React.StrictMode><QueryClientProvider client={client}><App/></QueryClientProvider></React.StrictMode>);
