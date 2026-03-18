"use strict";var J=Object.defineProperty;var me=Object.getOwnPropertyDescriptor;var ye=Object.getOwnPropertyNames;var he=Object.prototype.hasOwnProperty;var re=e=>{throw TypeError(e)};var ge=(e,t)=>{for(var n in t)J(e,n,{get:t[n],enumerable:!0})},be=(e,t,n,s)=>{if(t&&typeof t=="object"||typeof t=="function")for(let r of ye(t))!he.call(e,r)&&r!==n&&J(e,r,{get:()=>t[r],enumerable:!(s=me(t,r))||s.enumerable});return e};var _e=e=>be(J({},"__esModule",{value:!0}),e);var Y=(e,t,n)=>t.has(e)||re("Cannot "+n);var u=(e,t,n)=>(Y(e,t,"read from private field"),n?n.call(e):t.get(e)),O=(e,t,n)=>t.has(e)?re("Cannot add the same private member more than once"):t instanceof WeakSet?t.add(e):t.set(e,n),C=(e,t,n,s)=>(Y(e,t,"write to private field"),s?s.call(e,n):t.set(e,n),n),L=(e,t,n)=>(Y(e,t,"access private method"),n);var K=(e,t,n,s)=>({set _(r){C(e,t,r,n)},get _(){return u(e,t,s)}});var lt={};ge(lt,{live:()=>ut});module.exports=_e(lt);function U(e){let t=e.length;for(let n=e.length-1;n>=0;n--){let s=e.charCodeAt(n);s>127&&s<=2047?t++:s>2047&&s<=65535&&(t+=2),s>=56320&&s<=57343&&n--}return t}var A,T,k,j,V,S,W,F,se,P=class{constructor(t=256){this.size=t;O(this,S);O(this,A);O(this,T,5);O(this,k,!1);O(this,j,new TextEncoder);O(this,V,0);C(this,A,L(this,S,W).call(this,t))}addInt32(t){return L(this,S,F).call(this,4),u(this,A).setInt32(u(this,T),t,u(this,k)),C(this,T,u(this,T)+4),this}addInt16(t){return L(this,S,F).call(this,2),u(this,A).setInt16(u(this,T),t,u(this,k)),C(this,T,u(this,T)+2),this}addCString(t){return t&&this.addString(t),L(this,S,F).call(this,1),u(this,A).setUint8(u(this,T),0),K(this,T)._++,this}addString(t=""){let n=U(t);return L(this,S,F).call(this,n),u(this,j).encodeInto(t,new Uint8Array(u(this,A).buffer,u(this,T))),C(this,T,u(this,T)+n),this}add(t){return L(this,S,F).call(this,t.byteLength),new Uint8Array(u(this,A).buffer).set(new Uint8Array(t),u(this,T)),C(this,T,u(this,T)+t.byteLength),this}flush(t){let n=L(this,S,se).call(this,t);return C(this,T,5),C(this,A,L(this,S,W).call(this,this.size)),new Uint8Array(n)}};A=new WeakMap,T=new WeakMap,k=new WeakMap,j=new WeakMap,V=new WeakMap,S=new WeakSet,W=function(t){return new DataView(new ArrayBuffer(t))},F=function(t){if(u(this,A).byteLength-u(this,T)<t){let s=u(this,A).buffer,r=s.byteLength+(s.byteLength>>1)+t;C(this,A,L(this,S,W).call(this,r)),new Uint8Array(u(this,A).buffer).set(new Uint8Array(s))}},se=function(t){if(t){u(this,A).setUint8(u(this,V),t);let n=u(this,T)-(u(this,V)+1);u(this,A).setInt32(u(this,V)+1,n,u(this,k))}return u(this,A).buffer.slice(t?0:5,u(this,T))};var g=new P,Ee=e=>{g.addInt16(3).addInt16(0);for(let s of Object.keys(e))g.addCString(s).addCString(e[s]);g.addCString("client_encoding").addCString("UTF8");let t=g.addCString("").flush(),n=t.byteLength+4;return new P().addInt32(n).add(t).flush()},we=()=>{let e=new DataView(new ArrayBuffer(8));return e.setInt32(0,8,!1),e.setInt32(4,80877103,!1),new Uint8Array(e.buffer)},Ae=e=>g.addCString(e).flush(112),Te=(e,t)=>(g.addCString(e).addInt32(U(t)).addString(t),g.flush(112)),Re=e=>g.addString(e).flush(112),Se=e=>g.addCString(e).flush(81),Ie=[],Ce=e=>{let t=e.name??"";t.length>63&&(console.error("Warning! Postgres only supports 63 characters for query names."),console.error("You supplied %s (%s)",t,t.length),console.error("This can cause conflicts and silent errors executing queries"));let n=g.addCString(t).addCString(e.text).addInt16(e.types?.length??0);return e.types?.forEach(s=>n.addInt32(s)),g.flush(80)},G=new P;var Ne=(e,t)=>{for(let n=0;n<e.length;n++){let s=t?t(e[n],n):e[n];if(s===null)g.addInt16(0),G.addInt32(-1);else if(s instanceof ArrayBuffer||ArrayBuffer.isView(s)){let r=ArrayBuffer.isView(s)?s.buffer.slice(s.byteOffset,s.byteOffset+s.byteLength):s;g.addInt16(1),G.addInt32(r.byteLength),G.add(r)}else g.addInt16(0),G.addInt32(U(s)),G.addString(s)}},Le=(e={})=>{let t=e.portal??"",n=e.statement??"",s=e.binary??!1,r=e.values??Ie,c=r.length;return g.addCString(t).addCString(n),g.addInt16(c),Ne(r,e.valueMapper),g.addInt16(c),g.add(G.flush()),g.addInt16(s?1:0),g.flush(66)},ve=new Uint8Array([69,0,0,0,9,0,0,0,0,0]),De=e=>{if(!e||!e.portal&&!e.rows)return ve;let t=e.portal??"",n=e.rows??0,s=U(t),r=4+s+1+4,c=new DataView(new ArrayBuffer(1+r));return c.setUint8(0,69),c.setInt32(1,r,!1),new TextEncoder().encodeInto(t,new Uint8Array(c.buffer,5)),c.setUint8(s+5,0),c.setUint32(c.byteLength-4,n,!1),new Uint8Array(c.buffer)},Oe=(e,t)=>{let n=new DataView(new ArrayBuffer(16));return n.setInt32(0,16,!1),n.setInt16(4,1234,!1),n.setInt16(6,5678,!1),n.setInt32(8,e,!1),n.setInt32(12,t,!1),new Uint8Array(n.buffer)},Z=(e,t)=>{let n=new P;return n.addCString(t),n.flush(e)},Me=g.addCString("P").flush(68),xe=g.addCString("S").flush(68),Be=e=>e.name?Z(68,`${e.type}${e.name??""}`):e.type==="P"?Me:xe,Pe=e=>{let t=`${e.type}${e.name??""}`;return Z(67,t)},$e=e=>g.add(e).flush(100),Ue=e=>Z(102,e),Q=e=>new Uint8Array([e,0,0,0,4]),Fe=Q(72),ke=Q(83),Ve=Q(88),Ge=Q(99),q={startup:Ee,password:Ae,requestSsl:we,sendSASLInitialResponseMessage:Te,sendSCRAMClientFinalMessage:Re,query:Se,parse:Ce,bind:Le,execute:De,describe:Be,close:Pe,flush:()=>Fe,sync:()=>ke,end:()=>Ve,copyData:$e,copyDone:()=>Ge,copyFail:Ue,cancel:Oe};var St=new ArrayBuffer(0);var We=1,je=4,pn=We+je,fn=new ArrayBuffer(0);var Qe=globalThis.JSON.parse,He=globalThis.JSON.stringify,ie=16,ae=17;var oe=20,ze=21,Xe=23;var H=25,Je=26;var ue=114;var Ye=700,Ke=701;var Ze=1042,et=1043,tt=1082;var nt=1114,ce=1184;var rt=3802;var st={string:{to:H,from:[H,et,Ze],serialize:e=>{if(typeof e=="string")return e;if(typeof e=="number")return e.toString();throw new Error("Invalid input for string type")},parse:e=>e},number:{to:0,from:[ze,Xe,Je,Ye,Ke],serialize:e=>e.toString(),parse:e=>+e},bigint:{to:oe,from:[oe],serialize:e=>e.toString(),parse:e=>{let t=BigInt(e);return t<Number.MIN_SAFE_INTEGER||t>Number.MAX_SAFE_INTEGER?t:Number(t)}},json:{to:ue,from:[ue,rt],serialize:e=>typeof e=="string"?e:He(e),parse:e=>Qe(e)},boolean:{to:ie,from:[ie],serialize:e=>{if(typeof e!="boolean")throw new Error("Invalid input for boolean type");return e?"t":"f"},parse:e=>e==="t"},date:{to:ce,from:[tt,nt,ce],serialize:e=>{if(typeof e=="string")return e;if(typeof e=="number")return new Date(e).toISOString();if(e instanceof Date)return e.toISOString();throw new Error("Invalid input for date type")},parse:e=>new Date(e)},bytea:{to:ae,from:[ae],serialize:e=>{if(!(e instanceof Uint8Array))throw new Error("Invalid input for bytea type");return"\\x"+Array.from(e).map(t=>t.toString(16).padStart(2,"0")).join("")},parse:e=>{let t=e.slice(2);return Uint8Array.from({length:t.length/2},(n,s)=>parseInt(t.substring(s*2,(s+1)*2),16))}}},le=it(st),An=le.parsers,Tn=le.serializers;function it(e){return Object.keys(e).reduce(({parsers:t,serializers:n},s)=>{let{to:r,from:c,serialize:i,parse:b}=e[s];return n[r]=i,n[s]=i,t[s]=b,Array.isArray(c)?c.forEach(y=>{t[y]=b,n[y]=i}):(t[c]=b,n[c]=i),{parsers:t,serializers:n}},{parsers:{},serializers:{}})}function de(e){let t=e.find(n=>n.name==="parameterDescription");return t?t.dataTypeIDs:[]}var Mn=typeof process=="object"&&typeof process.versions=="object"&&typeof process.versions.node=="string";var ee=()=>{if(globalThis.crypto?.randomUUID)return globalThis.crypto.randomUUID();let e=new Uint8Array(16);if(globalThis.crypto?.getRandomValues)globalThis.crypto.getRandomValues(e);else for(let n=0;n<e.length;n++)e[n]=Math.floor(Math.random()*256);e[6]=e[6]&15|64,e[8]=e[8]&63|128;let t=[];return e.forEach(n=>{t.push(n.toString(16).padStart(2,"0"))}),t.slice(0,4).join("")+"-"+t.slice(4,6).join("")+"-"+t.slice(6,8).join("")+"-"+t.slice(8,10).join("")+"-"+t.slice(10).join("")};async function te(e,t,n,s){if(!n||n.length===0)return t;s=s??e;let r=[];try{await e.execProtocol(q.parse({text:t}),{syncToFs:!1}),r.push(...(await e.execProtocol(q.describe({type:"S"}),{syncToFs:!1})).messages)}finally{r.push(...(await e.execProtocol(q.sync(),{syncToFs:!1})).messages)}let c=de(r),i=t.replace(/\$([0-9]+)/g,(y,l)=>"%"+l+"L");return(await s.query(`SELECT format($1, ${n.map((y,l)=>`$${l+2}`).join(", ")}) as query`,[i,...n],{paramTypes:[H,...c]})).rows[0].query}function ne(e){let t,n=!1,s=async()=>{if(!t){n=!1;return}n=!0;let{args:r,resolve:c,reject:i}=t;t=void 0;try{let b=await e(...r);c(b)}catch(b){i(b)}finally{s()}};return async(...r)=>{t&&t.resolve(void 0);let c=new Promise((i,b)=>{t={args:r,resolve:i,reject:b}});return n||s(),c}}var at=5,ot=async(e,t)=>{let n=new Set,s={async query(r,c,i){let b,y,l;if(typeof r!="string"&&(b=r.signal,c=r.params,i=r.callback,y=r.offset,l=r.limit,r=r.query),y===void 0!=(l===void 0))throw new Error("offset and limit must be provided together");let a=y!==void 0&&l!==void 0,R;if(a&&(typeof y!="number"||isNaN(y)||typeof l!="number"||isNaN(l)))throw new Error("offset and limit must be numbers");let _=i?[i]:[],m=ee().replace(/-/g,""),D=!1,I,M,$=async()=>{await e.transaction(async o=>{let d=c&&c.length>0?await te(e,r,c,o):r;await o.exec(`CREATE OR REPLACE TEMP VIEW live_query_${m}_view AS ${d}`);let E=await pe(o,`live_query_${m}_view`);await fe(o,E,n),a?(await o.exec(`
              PREPARE live_query_${m}_get(int, int) AS
              SELECT * FROM live_query_${m}_view
              LIMIT $1 OFFSET $2;
            `),await o.exec(`
              PREPARE live_query_${m}_get_total_count AS
              SELECT COUNT(*) FROM live_query_${m}_view;
            `),R=(await o.query(`EXECUTE live_query_${m}_get_total_count;`)).rows[0].count,I={...await o.query(`EXECUTE live_query_${m}_get(${l}, ${y});`),offset:y,limit:l,totalCount:R}):(await o.exec(`
              PREPARE live_query_${m}_get AS
              SELECT * FROM live_query_${m}_view;
            `),I=await o.query(`EXECUTE live_query_${m}_get;`)),M=await Promise.all(E.map(w=>o.listen(`"table_change__${w.schema_oid}__${w.table_oid}"`,async()=>{N()})))})};await $();let N=ne(async({offset:o,limit:d}={})=>{if(!a&&(o!==void 0||d!==void 0))throw new Error("offset and limit cannot be provided for non-windowed queries");if(o&&(typeof o!="number"||isNaN(o))||d&&(typeof d!="number"||isNaN(d)))throw new Error("offset and limit must be numbers");y=o??y,l=d??l;let E=async(w=0)=>{if(_.length!==0){try{a?I={...await e.query(`EXECUTE live_query_${m}_get(${l}, ${y});`),offset:y,limit:l,totalCount:R}:I=await e.query(`EXECUTE live_query_${m}_get;`)}catch(h){let p=h.message;if(p.startsWith(`prepared statement "live_query_${m}`)&&p.endsWith("does not exist")){if(w>at)throw h;await $(),E(w+1)}else throw h}if(z(_,I),a){let h=(await e.query(`EXECUTE live_query_${m}_get_total_count;`)).rows[0].count;h!==R&&(R=h,N())}}};await E()}),x=o=>{if(D)throw new Error("Live query is no longer active and cannot be subscribed to");_.push(o)},f=async o=>{o?_=_.filter(d=>d!==d):_=[],_.length===0&&!D&&(D=!0,await e.transaction(async d=>{await Promise.all(M.map(E=>E(d))),await d.exec(`
              DROP VIEW IF EXISTS live_query_${m}_view;
              DEALLOCATE live_query_${m}_get;
            `)}))};return b?.aborted?await f():b?.addEventListener("abort",()=>{f()},{once:!0}),z(_,I),{initialResults:I,subscribe:x,unsubscribe:f,refresh:N}},async changes(r,c,i,b){let y;if(typeof r!="string"&&(y=r.signal,c=r.params,i=r.key,b=r.callback,r=r.query),!i)throw new Error("key is required for changes queries");let l=b?[b]:[],a=ee().replace(/-/g,""),R=!1,_=1,m,D,I=async()=>{await e.transaction(async f=>{let o=await te(e,r,c,f);await f.query(`CREATE OR REPLACE TEMP VIEW live_query_${a}_view AS ${o}`);let d=await pe(f,`live_query_${a}_view`);await fe(f,d,n);let E=[...(await f.query(`
                SELECT column_name, data_type, udt_name
                FROM information_schema.columns 
                WHERE table_name = 'live_query_${a}_view'
              `)).rows,{column_name:"__after__",data_type:"integer"}];await f.exec(`
            CREATE TEMP TABLE live_query_${a}_state1 (LIKE live_query_${a}_view INCLUDING ALL);
            CREATE TEMP TABLE live_query_${a}_state2 (LIKE live_query_${a}_view INCLUDING ALL);
          `);for(let w of[1,2]){let h=w===1?2:1;await f.exec(`
              PREPARE live_query_${a}_diff${w} AS
              WITH
                prev AS (SELECT LAG("${i}") OVER () as __after__, * FROM live_query_${a}_state${h}),
                curr AS (SELECT LAG("${i}") OVER () as __after__, * FROM live_query_${a}_state${w}),
                data_diff AS (
                  -- INSERT operations: Include all columns
                  SELECT 
                    'INSERT' AS __op__,
                    ${E.map(({column_name:p})=>`curr."${p}" AS "${p}"`).join(`,
`)},
                    ARRAY[]::text[] AS __changed_columns__
                  FROM curr
                  LEFT JOIN prev ON curr.${i} = prev.${i}
                  WHERE prev.${i} IS NULL
                UNION ALL
                  -- DELETE operations: Include only the primary key
                  SELECT 
                    'DELETE' AS __op__,
                    ${E.map(({column_name:p,data_type:B,udt_name:X})=>p===i?`prev."${p}" AS "${p}"`:`NULL${B==="USER-DEFINED"?`::${X}`:""} AS "${p}"`).join(`,
`)},
                      ARRAY[]::text[] AS __changed_columns__
                  FROM prev
                  LEFT JOIN curr ON prev.${i} = curr.${i}
                  WHERE curr.${i} IS NULL
                UNION ALL
                  -- UPDATE operations: Include only changed columns
                  SELECT 
                    'UPDATE' AS __op__,
                    ${E.map(({column_name:p,data_type:B,udt_name:X})=>p===i?`curr."${p}" AS "${p}"`:`CASE 
                              WHEN curr."${p}" IS DISTINCT FROM prev."${p}" 
                              THEN curr."${p}"
                              ELSE NULL${B==="USER-DEFINED"?`::${X}`:""}
                              END AS "${p}"`).join(`,
`)},
                      ARRAY(SELECT unnest FROM unnest(ARRAY[${E.filter(({column_name:p})=>p!==i).map(({column_name:p})=>`CASE
                              WHEN curr."${p}" IS DISTINCT FROM prev."${p}" 
                              THEN '${p}' 
                              ELSE NULL 
                              END`).join(", ")}]) WHERE unnest IS NOT NULL) AS __changed_columns__
                  FROM curr
                  INNER JOIN prev ON curr.${i} = prev.${i}
                  WHERE NOT (curr IS NOT DISTINCT FROM prev)
                )
              SELECT * FROM data_diff;
            `)}D=await Promise.all(d.map(w=>f.listen(`"table_change__${w.schema_oid}__${w.table_oid}"`,async()=>{M()})))})};await I();let M=ne(async()=>{if(l.length===0&&m)return;let f=!1;for(let o=0;o<5;o++)try{await e.transaction(async d=>{await d.exec(`
                INSERT INTO live_query_${a}_state${_} 
                  SELECT * FROM live_query_${a}_view;
              `),m=await d.query(`EXECUTE live_query_${a}_diff${_};`),_=_===1?2:1,await d.exec(`
                TRUNCATE live_query_${a}_state${_};
              `)});break}catch(d){if(d.message===`relation "live_query_${a}_state${_}" does not exist`){f=!0,await I();continue}else throw d}ct(l,[...f?[{__op__:"RESET"}]:[],...m.rows])}),$=f=>{if(R)throw new Error("Live query is no longer active and cannot be subscribed to");l.push(f)},N=async f=>{f?l=l.filter(o=>o!==o):l=[],l.length===0&&!R&&(R=!0,await e.transaction(async o=>{await Promise.all(D.map(d=>d(o))),await o.exec(`
              DROP VIEW IF EXISTS live_query_${a}_view;
              DROP TABLE IF EXISTS live_query_${a}_state1;
              DROP TABLE IF EXISTS live_query_${a}_state2;
              DEALLOCATE live_query_${a}_diff1;
              DEALLOCATE live_query_${a}_diff2;
            `)}))};return y?.aborted?await N():y?.addEventListener("abort",()=>{N()},{once:!0}),await M(),{fields:m.fields.filter(f=>!["__after__","__op__","__changed_columns__"].includes(f.name)),initialChanges:m.rows,subscribe:$,unsubscribe:N,refresh:M}},async incrementalQuery(r,c,i,b){let y;if(typeof r!="string"&&(y=r.signal,c=r.params,i=r.key,b=r.callback,r=r.query),!i)throw new Error("key is required for incremental queries");let l=b?[b]:[],a=new Map,R=new Map,_=[],m=!0,{fields:D,unsubscribe:I,refresh:M}=await s.changes(r,c,i,x=>{for(let d of x){let{__op__:E,__changed_columns__:w,...h}=d;switch(E){case"RESET":a.clear(),R.clear();break;case"INSERT":a.set(h[i],h),R.set(h.__after__,h[i]);break;case"DELETE":{let p=a.get(h[i]);a.delete(h[i]),p.__after__!==null&&R.delete(p.__after__);break}case"UPDATE":{let p={...a.get(h[i])??{}};for(let B of w)p[B]=h[B],B==="__after__"&&R.set(h.__after__,h[i]);a.set(h[i],p);break}}}let f=[],o=null;for(let d=0;d<a.size;d++){let E=R.get(o),w=a.get(E);if(!w)break;let h={...w};delete h.__after__,f.push(h),o=E}_=f,m||z(l,{rows:f,fields:D})});m=!1,z(l,{rows:_,fields:D});let $=x=>{l.push(x)},N=async x=>{x?l=l.filter(f=>f!==f):l=[],l.length===0&&await I()};return y?.aborted?await N():y?.addEventListener("abort",()=>{N()},{once:!0}),{initialResults:{rows:_,fields:D},subscribe:$,unsubscribe:N,refresh:M}}};return{namespaceObj:s}},ut={name:"Live Queries",setup:ot};async function pe(e,t){return(await e.query(`
      WITH RECURSIVE view_dependencies AS (
        -- Base case: Get the initial view's dependencies
        SELECT DISTINCT
          cl.relname AS dependent_name,
          n.nspname AS schema_name,
          cl.oid AS dependent_oid,
          n.oid AS schema_oid,
          cl.relkind = 'v' AS is_view
        FROM pg_rewrite r
        JOIN pg_depend d ON r.oid = d.objid
        JOIN pg_class cl ON d.refobjid = cl.oid
        JOIN pg_namespace n ON cl.relnamespace = n.oid
        WHERE
          r.ev_class = (
              SELECT oid FROM pg_class WHERE relname = $1 AND relkind = 'v'
          )
          AND d.deptype = 'n'

        UNION ALL

        -- Recursive case: Traverse dependencies for views
        SELECT DISTINCT
          cl.relname AS dependent_name,
          n.nspname AS schema_name,
          cl.oid AS dependent_oid,
          n.oid AS schema_oid,
          cl.relkind = 'v' AS is_view
        FROM view_dependencies vd
        JOIN pg_rewrite r ON vd.dependent_name = (
          SELECT relname FROM pg_class WHERE oid = r.ev_class AND relkind = 'v'
        )
        JOIN pg_depend d ON r.oid = d.objid
        JOIN pg_class cl ON d.refobjid = cl.oid
        JOIN pg_namespace n ON cl.relnamespace = n.oid
        WHERE d.deptype = 'n'
      )
      SELECT DISTINCT
        dependent_name AS table_name,
        schema_name,
        dependent_oid AS table_oid,
        schema_oid
      FROM view_dependencies
      WHERE NOT is_view; -- Exclude intermediate views
    `,[t])).rows.map(s=>({table_name:s.table_name,schema_name:s.schema_name,table_oid:s.table_oid,schema_oid:s.schema_oid}))}async function fe(e,t,n){let s=t.filter(r=>!n.has(`${r.schema_oid}_${r.table_oid}`)).map(r=>`
      CREATE OR REPLACE FUNCTION "_notify_${r.schema_oid}_${r.table_oid}"() RETURNS TRIGGER AS $$
      BEGIN
        PERFORM pg_notify('table_change__${r.schema_oid}__${r.table_oid}', '');
        RETURN NULL;
      END;
      $$ LANGUAGE plpgsql;
      CREATE OR REPLACE TRIGGER "_notify_trigger_${r.schema_oid}_${r.table_oid}"
      AFTER INSERT OR UPDATE OR DELETE ON "${r.schema_name}"."${r.table_name}"
      FOR EACH STATEMENT EXECUTE FUNCTION "_notify_${r.schema_oid}_${r.table_oid}"();
      `).join(`
`);s.trim()!==""&&await e.exec(s),t.map(r=>n.add(`${r.schema_oid}_${r.table_oid}`))}var z=(e,t)=>{for(let n of e)n(t)},ct=(e,t)=>{for(let n of e)n(t)};0&&(module.exports={live});
//# sourceMappingURL=index.cjs.map