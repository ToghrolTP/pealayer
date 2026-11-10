// Loopback-only visual harness. Does not contact remote hosts, play media or
// mutate configuration. Simulated preview responses exercise the real component.
import {createServer} from 'vite';
const server=await createServer({server:{host:'127.0.0.1',port:18194,strictPort:true},plugins:[{
  name:'remote-location-review-fixture',configureServer(server){
    server.middlewares.use((request,response,next)=>{
      const url=new URL(request.url,'http://127.0.0.1:18194');
      if(url.pathname!=='/api/remote/thumbnail')return next();
      const target=url.searchParams.get('url')??'';
      if(target.endsWith('/pending.mkv')){response.statusCode=202;response.setHeader('Content-Type','application/json');response.end('{"status":"pending"}');return;}
      if(target.endsWith('/unavailable.mkv')){response.statusCode=400;response.setHeader('Content-Type','application/json');response.end('{"error":"Fixture preview unavailable"}');return;}
      response.setHeader('Content-Type','image/svg+xml');
      response.end('<svg xmlns="http://www.w3.org/2000/svg" width="360" height="200"><rect width="360" height="200" fill="#253349"/><path d="M0 170L105 65L200 155L285 75L360 165V200H0Z" fill="#456b72"/><circle cx="275" cy="50" r="18" fill="#c4b58c"/></svg>');
    });
  }
}]});
await server.listen();
console.log('Remote Location component fixture: http://127.0.0.1:18194/scripts/fixtures/remote-location.html');
