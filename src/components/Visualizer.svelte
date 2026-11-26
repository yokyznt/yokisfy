<script>
  import { untrack } from 'svelte'
  import { levels, useLevels } from '../lib/levels.js'

  let { colors = [], preset = 0, modes = [0, 1, 2, 3, 4, 5, 6], every = 22, dim = 0 } = $props()
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
float h(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float n(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(h(i),h(i+vec2(1,0)),f.x),mix(h(i+vec2(0,1)),h(i+vec2(1,1)),f.x),f.y);}
float fbm(vec2 p){float v=0.,a=.5;for(int i=0;i<5;i++){v+=a*n(p);p=p*2.03+vec2(1.7,9.2);a*=.5;}return v;}
vec3 pal(float x){x=fract(x);return x<.33?mix(c1,c2,x*3.):x<.66?mix(c2,c3,(x-.33)*3.):mix(c3,c1,(x-.66)*3.);}
void main(){
  vec2 uv=(gl_FragCoord.xy-.5*r)/min(r.x,r.y);
  vec3 col;
  if(mode==0){
    vec2 q=vec2(fbm(uv*1.6+t*.05),fbm(uv*1.6-t*.04+4.));
    vec2 w=vec2(fbm(uv*2.+q*2.+t*.12+lo*.8),fbm(uv*2.+q*2.-t*.1+2.));
    float f=fbm(uv*1.8+w*(1.6+lv*1.4));
    col=pal(f*1.3+t*.03+lo*.15)*(.35+f*1.1+bt*.35);
    col+=pow(f,4.)*hi*1.6;
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
      col+=pal(fi*.33+x*.08+t*.02)*band*rays*(.7+hi*.8+bt*.3);
    }
    col+=pal(.6)*.06*(1.-uv.y);
  }else if(mode==4){
    float a=atan(uv.y,uv.x),d=length(uv);
    float nb=64.;
    float id=floor((a/6.2832+.5)*nb);
    float sp=fbm(vec2(id*.37,t*1.4))*(.35+lo*.9)+hi*.25*h(vec2(id,floor(t*12.)));
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
    vec2 cs=vec2(cos(ang),sin(ang));float wave=fbm(cs*1.6+vec2(t*.3,t*.5))*(.08+hi*.25);
    float ringd=abs(d-r0-wave);
    float glow=.012/(ringd+.004)*(.6+lv);
    float st=0.;
    for(int i=0;i<3;i++){vec2 g=uv*(6.+float(i)*5.)+t*(.05+float(i)*.03);vec2 id=floor(g);vec2 f=fract(g)-.5;float s=h(id+float(i));st+=smoothstep(.06,0.,length(f-vec2(h(id+1.),h(id+2.))+.5)*.9)*step(.88,s)*(.4+hi);}
    col=pal(cs.y*.25+cs.x*.1+t*.05)*glow+vec3(st)*.8+pal(.3)*.12/(d+.4);
  }
  col=pow(max(col,0.),vec3(.9));
  float vig=1.-smoothstep(.4,1.25,length(uv*vec2(.9,1.1)));
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
      const s = gl.createShader(type)
      gl.shaderSource(s, src)
      gl.compileShader(s)
      return s
    }
    const prog = gl.createProgram()
    gl.attachShader(prog, sh(gl.VERTEX_SHADER, vert))
    gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, frag))
    gl.linkProgram(prog)
    gl.useProgram(prog)
    const buf = gl.createBuffer()
    gl.bindBuffer(gl.ARRAY_BUFFER, buf)
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW)
    const loc = gl.getAttribLocation(prog, 'p')
    gl.enableVertexAttribArray(loc)
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0)
    const u = (n) => gl.getUniformLocation(prog, n)
    const U = { r: u('r'), t: u('t'), lv: u('lv'), lo: u('lo'), hi: u('hi'), bt: u('bt'), c1: u('c1'), c2: u('c2'), c3: u('c3'), mode: u('mode') }
    const stop = useLevels()
    let raf
    let clock = 0
    let last = performance.now()
    const scale = Math.min(window.devicePixelRatio || 1, 1.5) * (window.innerWidth < 760 ? 0.6 : 0.75)
    const resize = () => {
      canvas.width = Math.max(1, Math.floor(canvas.clientWidth * scale))
      canvas.height = Math.max(1, Math.floor(canvas.clientHeight * scale))
      gl.viewport(0, 0, canvas.width, canvas.height)
    }
    resize()
    window.addEventListener('resize', resize)
    let modeNow = 0
    const draw = (now) => {
      modeNow = curMode()
      const l = levels()
      clock += ((now - last) / 1000) * (0.6 + l.rms * 0.9)
      last = now
      const [a, b, c] = [rgb(colors[0]), rgb(colors[1]), rgb(colors[2])]
      gl.uniform2f(U.r, canvas.width, canvas.height)
      gl.uniform1f(U.t, clock)
      gl.uniform1f(U.lv, l.rms)
      gl.uniform1f(U.lo, l.low)
      gl.uniform1f(U.hi, l.high)
      gl.uniform1f(U.bt, l.beat)
      gl.uniform3f(U.c1, ...a)
      gl.uniform3f(U.c2, ...b)
      gl.uniform3f(U.c3, ...c)
      gl.uniform1i(U.mode, modeNow)
      gl.drawArrays(gl.TRIANGLES, 0, 3)
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
