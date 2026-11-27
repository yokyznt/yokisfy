<script>
  import { untrack } from 'svelte'
  import { levels, useLevels } from '../lib/levels.js'

  let { colors = [], preset = 0, modes = [7, 0, 8, 1, 9, 3, 10, 2, 11, 4, 12, 5, 13, 6], every = 22, dim = 0 } = $props()
  let canvas = $state()
  let slot = $state(0)
  let swap = $state(false)
  let mode = $derived(modes[slot % modes.length])
  let first = true

  function change(to) {
    swap = true
    setTimeout(() => {
      slot = to
      swap = false
    }, 650)
  }

  $effect(() => {
    preset
    if (first) {
      first = false
      slot = Math.floor(Math.random() * modes.length)
      return
    }
    untrack(() => change(slot + 1))
  })

  $effect(() => {
    if (!every) return
    const id = setInterval(() => change(slot + 1 + Math.floor(Math.random() * Math.max(1, modes.length - 1))), every * 1000)
    return () => clearInterval(id)
  })

  const vert = `attribute vec2 p;void main(){gl_Position=vec4(p,0.,1.);}`
  const frag = `precision highp float;
uniform vec2 r;uniform float t;uniform float lv;uniform float lo;uniform float hi;uniform float bt;uniform vec3 c1;uniform vec3 c2;uniform vec3 c3;uniform int mode;
uniform float ce;uniform float vo;uniform float fx;uniform float mo;uniform vec4 sa;uniform vec4 sb;
float h(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float n(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(h(i),h(i+vec2(1,0)),f.x),mix(h(i+vec2(0,1)),h(i+vec2(1,1)),f.x),f.y);}
float fbm(vec2 p){float v=0.,a=.5;for(int i=0;i<5;i++){v+=a*n(p);p=p*2.03+vec2(1.7,9.2);a*=.5;}return v;}
float spec(float x){x=clamp(x,0.,1.)*7.;return dot(sa,max(1.-abs(vec4(0.,1.,2.,3.)-x),0.))+dot(sb,max(1.-abs(vec4(4.,5.,6.,7.)-x),0.));}
vec3 pal(float x){x=fract(x+ce*.4);return x<.33?mix(c1,c2,x*3.):x<.66?mix(c2,c3,(x-.33)*3.):mix(c3,c1,(x-.66)*3.);}
void main(){
  vec2 uv=(gl_FragCoord.xy-.5*r)/min(r.x,r.y);
  vec3 col;
  if(mode==0){
    vec2 q=vec2(fbm(uv*1.6+t*.05),fbm(uv*1.6-t*.04+4.));
    vec2 w=vec2(fbm(uv*2.+q*2.+t*.12+lo*.8),fbm(uv*2.+q*2.-t*.1+2.));
    float f=fbm(uv*1.8+w*(1.6+lv*.9+vo*1.3));
    col=pal(f*1.3+t*.03+lo*.15)*(.35+f*1.1+bt*.35);
    col+=pow(f,4.)*(hi+fx)*1.3;
  }else if(mode==1){
    float a=atan(uv.y,uv.x),d=length(uv);
    float k=6.;a=mod(a,6.2832/k);a=abs(a-3.1416/k);
    vec2 p=vec2(cos(a),sin(a))*d;
    float z=1./(d+.15)+t*(.6+lo*1.4);
    float f=fbm(vec2(p.x*3.,z)+t*.1);
    float ring=smoothstep(.02,.0,abs(fract(z*.5)-.5)-.45+lo*.04);
    col=pal(f+z*.08)*(.25+f)*(.6+1.2/(d*3.+.6));
    col+=pal(z*.1+.5)*ring*(.4+bt);
    col*=smoothstep(0.,.25,d+lv*.1);
  }else if(mode==3){
    float x=uv.x*1.4;
    float y=uv.y+.35;
    col=vec3(0.);
    for(int i=0;i<3;i++){
      float fi=float(i);
      float w=sin(x*(1.4+fi*.6)+t*(.35+fi*.12))*.18+fbm(vec2(x*1.2+fi*3.,t*.15))*.35;
      float band=exp(-pow((y-w-fi*.12)*(5.-lo*1.5),2.));
      float rays=.6+.4*fbm(vec2(x*8.+fi,t*.6+y*2.));
      col+=pal(fi*.33+x*.08+t*.02)*band*rays*(.4+hi*.5+bt*.3+spec(fi*.3+.1)*1.1);
    }
    col+=pal(.6)*.06*(1.-uv.y);
  }else if(mode==4){
    float a=atan(uv.y,uv.x),d=length(uv);
    float nb=64.;
    float id=floor((a/6.2832+.5)*nb);
    float sp=spec(abs(id/nb*2.-1.))*(.85+.3*h(vec2(id,floor(t*10.))))+fbm(vec2(id*.37,t*1.4))*.1;
    float r0=.22+bt*.03;
    float bar=step(r0,d)*step(d,r0+sp*.45)*step(.18,fract((a/6.2832+.5)*nb));
    float glow=.006/abs(d-r0)+.0;
    col=pal(id/nb+t*.03)*bar*1.2+pal(.2)*glow*(.4+lv);
    col+=pal(id/nb)*.05/(d+.3);
  }else if(mode==5){
    vec2 p=uv*3.;
    float v=sin(p.x+t)+sin(p.y*1.3+t*.8)+sin((p.x+p.y)*.7+t*1.1)+sin(length(p+vec2(sin(t*.3),cos(t*.4))*2.)*1.8-t*1.5);
    v=v*.25+lo*.4;
    col=pal(v+t*.04)*(.55+.45*sin(v*6.2832+t))*(.6+lv*.8);
  }else if(mode==6){
    col=vec3(0.);
    for(int i=0;i<4;i++){
      float fi=float(i);
      float z=fract(t*(.08+lo*.12)+fi*.25);
      vec2 g=uv/(z*1.6+.05)*6.+fi*13.;
      vec2 id=floor(g);vec2 f=fract(g)-.5;
      float s=h(id+fi);
      float star=smoothstep(.12*(1.-z)+.02,0.,length(f-(vec2(h(id+3.),h(id+7.))-.5)*.6))*step(.7,s);
      col+=pal(s+fi*.2)*star*z*(1.2+hi);
    }
    col+=pal(.5)*.08/(length(uv)+.5)*(1.+bt);
  }else{
    float d=length(uv);
    float r0=.28+lo*.18+bt*.06;
    float ang=atan(uv.y,uv.x);
    vec2 cs=vec2(cos(ang),sin(ang));float wave=fbm(cs*1.6+vec2(t*.3,t*.5))*(.06+hi*.15)+spec(abs(ang)/3.1416)*.13;
    float ringd=abs(d-r0-wave);
    float glow=.012/(ringd+.004)*(.6+lv);
    float st=0.;
    for(int i=0;i<3;i++){vec2 g=uv*(6.+float(i)*5.)+t*(.05+float(i)*.03);vec2 id=floor(g);vec2 f=fract(g)-.5;float s=h(id+float(i));st+=smoothstep(.06,0.,length(f-vec2(h(id+1.),h(id+2.))+.5)*.9)*step(.88,s)*(.4+hi);}
    col=pal(cs.y*.25+cs.x*.1+t*.05)*glow+pal(.6)*st*.9+pal(.3)*.12/(d+.4);
  }
  col*=.7+mo*.5;
  col/=max(max(max(col.r,col.g),col.b),1.);
  col=pow(max(col,0.),vec3(.9));
  float vig=1.-smoothstep(.4,1.25,length(uv*vec2(.9,1.1)));
  gl_FragColor=vec4(col*vig,1.);
}`

  const fbFrag = `precision highp float;
uniform sampler2D prev;uniform vec2 r;uniform float t;uniform float lv;uniform float lo;uniform float hi;uniform float bt;uniform vec3 c1;uniform vec3 c2;uniform vec3 c3;uniform int mode;
uniform float ce;uniform float vo;uniform float fx;uniform float mo;uniform vec4 sa;uniform vec4 sb;
float h(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float n(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(h(i),h(i+vec2(1,0)),f.x),mix(h(i+vec2(0,1)),h(i+vec2(1,1)),f.x),f.y);}
float fbm(vec2 p){float v=0.,a=.5;for(int i=0;i<4;i++){v+=a*n(p);p=p*2.03+vec2(1.7,9.2);a*=.5;}return v;}
float spec(float x){x=clamp(x,0.,1.)*7.;return dot(sa,max(1.-abs(vec4(0.,1.,2.,3.)-x),0.))+dot(sb,max(1.-abs(vec4(4.,5.,6.,7.)-x),0.));}
vec3 pal(float x){x=fract(x+ce*.4);return x<.33?mix(c1,c2,x*3.):x<.66?mix(c2,c3,(x-.33)*3.):mix(c3,c1,(x-.66)*3.);}
vec3 vivid(vec3 c){float m=max(max(c.r,c.g),c.b);return clamp(c/max(m,.2),0.,1.);}
mat2 rot(float a){float c=cos(a),s=sin(a);return mat2(c,-s,s,c);}
vec3 hue(vec3 c,float a){vec3 k=vec3(.57735);float ca=cos(a);return c*ca+cross(k,c)*sin(a)+k*dot(k,c)*(1.-ca);}
void main(){
  vec2 asp=vec2(r.x/r.y,1.);
  vec2 p=(gl_FragCoord.xy/r-.5)*asp;
  float d=length(p),a=atan(p.y,p.x);
  vec2 q=p;
  float decay=.955;
  if(mode==7){
    q=rot((.025+hi*.05+sin(t*.2)*.02)*(ce>.45?1.:-1.))*p*(.955-lo*.06-bt*.05);
    q+=.008*vec2(sin(p.y*7.+t*1.7),cos(p.x*7.+t*1.3))*(1.+lo*3.);
  }else if(mode==8){
    q=rot((.08+lo*.18+vo*.12+bt*.1)/(d+.3))*p*(.992-bt*.03);
  }else if(mode==9){
    q=p*(.982-lo*.03);
    q+=.012*vec2(fbm(p*3.+t*.25)-.5,fbm(p*3.-t*.25+5.)-.5)*(1.+lo*5.);
    q=rot(.006+lv*.03)*q;
  }else if(mode==10){
    q=rot(-.012-hi*.02)*p*(1.025+bt*.12+lo*.04);
    decay=.94;
  }else if(mode==11){
    vec2 f=vec2(fbm(p*2.2+vec2(0.,t*.35))-.5,fbm(p*2.2+vec2(7.,t*.35))-.5);
    q=p+vec2(f.x*.02,.009+f.y*.012)*(1.+lo*2.5);
    decay=.94;
  }else if(mode==12){
    q=abs(p)*(1.03+lo*.08+bt*.05)-vec2(.07+.05*sin(t*.31),.05+.04*cos(t*.23));
    q=rot(t*.09+hi*.2)*q;
    decay=.93;
  }else{
    q=rot(.01)*vec2(p.x+.004*sin(p.y*20.+t*3.),p.y)*(.97-lo*.05);
    q.y+=.004*sin(t+p.x*3.);
    decay=.95;
  }
  vec2 su=q/asp+.5;
  vec3 old=texture2D(prev,su).rgb*step(0.,su.x)*step(su.x,1.)*step(0.,su.y)*step(su.y,1.);
  old=hue(old,sin(t*.4)*.006+fx*.018+bt*.012);
  float lum=dot(old,vec3(.3333));
  old=mix(old,vivid(pal(lum*1.4+t*.02+a*.08))*lum*1.5,.11);
  old=max(old*decay-.018,0.);
  old=max(mix(vec3(dot(old,vec3(.3333))),old,1.05),0.);
  float pw=(.25+lv*.6+bt*.6+fx*.5)*(.6+mo*.6)*.3;
  vec3 e=vec3(0.);
  if(mode==7||mode==12){
    float r0=.18+lo*.22+bt*.06;
    float w=sin(a*5.+t*2.)*.03+sin(a*11.-t*3.1)*.02*(1.+hi*3.)+(fbm(vec2(a*2.,t*1.5))-.5)*.08*(.4+hi*2.)+spec(abs(a)/3.1416)*.12;
    e+=vivid(pal(a/6.2832+t*.07))*smoothstep(.014,0.,abs(d-r0-w))*pw;
  }
  if(mode==8||mode==11){
    float y=(sin(p.x*4.+t*1.8)*.12*(.4+lv*1.2)+(fbm(vec2(p.x*3.,t*1.2))-.5)*.25*(.3+hi*1.5))*(.4+spec(p.x*.45+.5)*1.4);
    e+=vivid(pal(p.x*.3+t*.05))*smoothstep(.012,0.,abs(p.y-y))*pw;
    e+=vivid(pal(.5+p.x*.2))*smoothstep(.01,0.,abs(p.y+y*.7+.18*sin(t*.4)))*pw*.6;
  }
  if(mode==9||mode==13){
    for(int i=0;i<7;i++){
      float fi=float(i);
      vec2 o=vec2(sin(t*(.7+fi*.23)+fi*1.7),cos(t*(.9+fi*.17)+fi))*(.18+.22*(.5+.5*sin(t*.3+fi)))*(.7+lv*.9);
      float g=.00035*(.6+bt*1.5+spec(fi/6.)*2.5)/(dot(p-o,p-o)+.0006);
      e+=vivid(pal(fi/7.+t*.05))*min(g,1.)*1.2*pw;
    }
  }
  if(mode==10){
    float rays=pow(abs(sin(a*6.+t*.7)),24.)+pow(abs(sin(a*9.-t*1.1)),40.);
    e+=vivid(pal(t*.1+d))*rays*smoothstep(.35,.0,d)*(bt*1.2+lo*.4+vo*.5)*.6;
    e+=vivid(pal(t*.13))*(.0012/(d*d+.006))*(.2+bt*1.2);
  }
  if(mode==13){
    float g=fbm(p*4.+t*.4);
    e+=vivid(pal(g+t*.06))*smoothstep(.01,0.,abs(g-.5-.08*sin(t)))*pw*.8;
  }
  vec3 o=old+e;
  gl_FragColor=vec4(o/max(max(max(o.r,o.g),o.b),1.),1.);
}`

  const outFrag = `precision highp float;
uniform sampler2D cur;uniform vec2 r;uniform float t;uniform float bt;uniform float lo;uniform int mode;
void main(){
  vec2 asp=vec2(r.x/r.y,1.);
  vec2 uv=gl_FragCoord.xy/r;
  vec2 p=(uv-.5)*asp;
  if(mode==9||mode==13){
    float k=mode==9?6.:8.;
    float a=atan(p.y,p.x)+t*.05,d=length(p);
    a=mod(a,6.2832/k);a=abs(a-3.1416/k);
    p=vec2(cos(a),sin(a))*d;
    uv=p/asp+.5;
  }
  vec2 off=(uv-.5)*(.004+bt*.018+lo*.006);
  vec3 col=vec3(texture2D(cur,uv+off).r,texture2D(cur,uv).g,texture2D(cur,uv-off).b);
  col=col/(1.+col*.35)*1.25;
  col*=1.+bt*.25;
  float vig=1.-smoothstep(.45,1.3,length((gl_FragCoord.xy/r-.5)*asp*vec2(.9,1.1)));
  gl_FragColor=vec4(col*vig,1.);
}`

  function curMode() {
    return mode
  }

  function rgb(hex) {
    const m = /^#?([0-9a-f]{6})$/i.exec(hex || '')
    if (!m) return [0.5, 0.4, 1]
    const v = parseInt(m[1], 16)
    return [((v >> 16) & 255) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255]
  }

  $effect(() => {
    if (!canvas) return
    const gl = canvas.getContext('webgl', { antialias: false, premultipliedAlpha: false })
    if (!gl) return
    const sh = (type, src) => {
      const x = gl.createShader(type)
      gl.shaderSource(x, src)
      gl.compileShader(x)
      return x
    }
    const par = gl.getExtension('KHR_parallel_shader_compile')
    const make = (fs, names) => {
      const pr = gl.createProgram()
      gl.attachShader(pr, sh(gl.VERTEX_SHADER, vert))
      gl.attachShader(pr, sh(gl.FRAGMENT_SHADER, fs))
      gl.bindAttribLocation(pr, 0, 'p')
      gl.linkProgram(pr)
      const o = { pr, U: null, ok: false }
      o.ready = () => {
        if (o.U) return o.ok
        if (par && !gl.getProgramParameter(pr, par.COMPLETION_STATUS_KHR)) return false
        o.ok = !!gl.getProgramParameter(pr, gl.LINK_STATUS)
        o.U = {}
        for (const k of names) o.U[k] = gl.getUniformLocation(pr, k)
        return o.ok
      }
      return o
    }
    const common = ['r', 't', 'lv', 'lo', 'hi', 'bt', 'c1', 'c2', 'c3', 'mode', 'ce', 'vo', 'fx', 'mo', 'sa', 'sb']
    const P0 = make(frag, common)
    const PFs = {}
    const pf = (m) => (PFs[m] ??= make(fbFrag.replace('uniform int mode;', `const int mode=${m};`), [...common, 'prev']))
    const PO = make(outFrag, ['cur', 'r', 't', 'bt', 'lo', 'mode'])
    window.__ykViz = { par: !!par }
    const buf = gl.createBuffer()
    gl.bindBuffer(gl.ARRAY_BUFFER, buf)
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW)
    gl.enableVertexAttribArray(0)
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0)
    const hf = gl.getExtension('OES_texture_half_float')
    const hfl = gl.getExtension('OES_texture_half_float_linear')
    gl.getExtension('EXT_color_buffer_half_float')
    let ttype = hf && hfl ? hf.HALF_FLOAT_OES : gl.UNSIGNED_BYTE
    let tex = []
    let fbo = []
    let ping = 0
    const alloc = () => {
      for (const x of tex) gl.deleteTexture(x)
      for (const x of fbo) gl.deleteFramebuffer(x)
      tex = []
      fbo = []
      for (let i = 0; i < 2; i++) {
        const tx = gl.createTexture()
        gl.bindTexture(gl.TEXTURE_2D, tx)
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR)
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR)
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE)
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE)
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, canvas.width, canvas.height, 0, gl.RGBA, ttype, null)
        const fb = gl.createFramebuffer()
        gl.bindFramebuffer(gl.FRAMEBUFFER, fb)
        gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tx, 0)
        if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE && ttype !== gl.UNSIGNED_BYTE) {
          gl.bindFramebuffer(gl.FRAMEBUFFER, null)
          gl.deleteTexture(tx)
          gl.deleteFramebuffer(fb)
          ttype = gl.UNSIGNED_BYTE
          return alloc()
        }
        gl.clearColor(0, 0, 0, 1)
        gl.clear(gl.COLOR_BUFFER_BIT)
        tex.push(tx)
        fbo.push(fb)
      }
      gl.bindFramebuffer(gl.FRAMEBUFFER, null)
      window.__ykViz.half = ttype !== gl.UNSIGNED_BYTE
    }
    const stop = useLevels()
    let raf
    let clock = 0
    let last = performance.now()
    const scale = Math.min(window.devicePixelRatio || 1, 1.5) * (window.innerWidth < 760 ? 0.6 : 0.75)
    const resize = () => {
      canvas.width = Math.max(1, Math.floor(canvas.clientWidth * scale))
      canvas.height = Math.max(1, Math.floor(canvas.clientHeight * scale))
      alloc()
    }
    resize()
    window.addEventListener('resize', resize)
    const setCommon = (U, l, cols, m) => {
      gl.uniform2f(U.r, canvas.width, canvas.height)
      gl.uniform1f(U.t, clock)
      gl.uniform1f(U.lv, l.rms)
      gl.uniform1f(U.lo, l.low)
      gl.uniform1f(U.hi, l.high)
      gl.uniform1f(U.bt, l.beat)
      gl.uniform3f(U.c1, ...cols[0])
      gl.uniform3f(U.c2, ...cols[1])
      gl.uniform3f(U.c3, ...cols[2])
      gl.uniform1i(U.mode, m)
      gl.uniform1f(U.ce, l.tone)
      gl.uniform1f(U.vo, l.mid)
      gl.uniform1f(U.fx, l.flux)
      gl.uniform1f(U.mo, l.mood)
      gl.uniform4f(U.sa, l.spec[0], l.spec[1], l.spec[2], l.spec[3])
      gl.uniform4f(U.sb, l.spec[4], l.spec[5], l.spec[6], l.spec[7])
    }
    const draw = (now) => {
      const m = curMode()
      const l = levels()
      clock += ((now - last) / 1000) * (0.35 + l.mood * 0.5 + l.rms * 0.6)
      last = now
      const cols = [rgb(colors[0]), rgb(colors[1]), rgb(colors[2])]
      const PF = m >= 7 ? pf(m) : null
      const fbOn = !!PF && PF.ready() && PO.ready()
      if (!fbOn) {
        if (!P0.ready()) {
          raf = requestAnimationFrame(draw)
          return
        }
        gl.useProgram(P0.pr)
        gl.bindFramebuffer(gl.FRAMEBUFFER, null)
        gl.viewport(0, 0, canvas.width, canvas.height)
        setCommon(P0.U, l, cols, m < 7 ? m : 0)
        gl.drawArrays(gl.TRIANGLES, 0, 3)
      } else {
        const src = ping
        const dst = 1 - ping
        gl.useProgram(PF.pr)
        gl.bindFramebuffer(gl.FRAMEBUFFER, fbo[dst])
        gl.viewport(0, 0, canvas.width, canvas.height)
        gl.activeTexture(gl.TEXTURE0)
        gl.bindTexture(gl.TEXTURE_2D, tex[src])
        gl.uniform1i(PF.U.prev, 0)
        setCommon(PF.U, l, cols, m)
        gl.drawArrays(gl.TRIANGLES, 0, 3)
        gl.useProgram(PO.pr)
        gl.bindFramebuffer(gl.FRAMEBUFFER, null)
        gl.bindTexture(gl.TEXTURE_2D, tex[dst])
        gl.uniform1i(PO.U.cur, 0)
        gl.uniform2f(PO.U.r, canvas.width, canvas.height)
        gl.uniform1f(PO.U.t, clock)
        gl.uniform1f(PO.U.bt, l.beat)
        gl.uniform1f(PO.U.lo, l.low)
        gl.uniform1i(PO.U.mode, m)
        gl.drawArrays(gl.TRIANGLES, 0, 3)
        ping = dst
        window.__ykViz.fb = true
      }
      raf = requestAnimationFrame(draw)
    }
    raf = requestAnimationFrame(draw)
    return () => {
      cancelAnimationFrame(raf)
      window.removeEventListener('resize', resize)
      stop()
      gl.getExtension('WEBGL_lose_context')?.loseContext()
    }
  })
</script>

<canvas class="viz" class:swap bind:this={canvas} style={dim ? `filter:brightness(${1 - dim})` : ''} aria-hidden="true"></canvas>
