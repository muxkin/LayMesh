#!/usr/bin/env python3
"""Deterministic editable LayMesh layout; rendering uses the branch Rust CLI."""
from pathlib import Path
import json, math
ROOT=Path(__file__).resolve().parent
S=200/1125
NAVY='#071878'; BLUE='#006aff'; ORANGE='#ff570a'; RED='#d73505'; WHITE='#ffffff'
q=lambda v:json.dumps(v,ensure_ascii=False)
mm=lambda v:f'{v*S:.5f} mm'
out=['# Editable recreation of the user supplied ABACUS workflow infographic.',
 '# Time ranges are reference scenario estimates, not measured performance.',
 '# All labels/arrows/cards/icons are native objects. Only illustrations are bitmaps.',
 f'page=canvas(name="科研探索周期",size=({mm(1998)},200mm),background="#ffffff")']
parent='page'; ox=oy=0; labels=[]
def add(expr,x,y):out.append(f'{parent}.add({expr},offset=({mm(x-ox)},{mm(y-oy)}))')
def grad(a,b):return f'linear_gradient(start=(0,0),end=(1,0),stops=[(0,{q(a)}),(1,{q(b)})])'
def rect(x,y,w,h,c=WHITE,stroke=None,lw=1,r=0,fill=None):
 a=f'size=({mm(w)},{mm(h)}),fill={fill or q(c)},border_width={mm(lw if stroke else 0)}'
 if stroke:a+=f',border_color={q(stroke)}'
 if r:a+=f',border_radius={mm(r)}'
 add('rect('+a+')',x,y)
def pic(x,y,w,h,src,r=0):rect(x,y,w,h,r=r,fill=f'image_fill(src={q("assets/"+src)},fit=cover)')
def circle(x,y,r,c=BLUE,stroke=None,lw=1):
 a=f'size=({mm(2*r)},{mm(2*r)}),fill={q(c)},border_width={mm(lw if stroke else 0)}'
 if stroke:a+=f',border_color={q(stroke)}'
 add('ellipse('+a+')',x-r,y-r)
def text(x,y,t,sz=26,c=NAVY,bold=False,w=None,align='left',font='Noto Sans CJK SC'):
 labels.append(t)
 a=f'{q(t)},font_family={q(font)},font_size={sz*S*72/25.4:.5f}pt,color={q(c)},font_weight={700 if bold else 400}'
 if w:a+=f',size=({mm(w)},auto),align={align}'
 add('text('+a+')',x,y)
def poly(ps,c=None,stroke=None,lw=1):
 x=min(p[0] for p in ps);y=min(p[1] for p in ps)
 points=', '.join(f'({mm(a-x)},{mm(b-y)})' for a,b in ps)
 if c:
  a=f'points=[{points}],fill={q(c)},border_width={mm(lw if stroke else 0)}'
  if stroke:a+=f',border_color={q(stroke)}'
  expr='polygon('+a+')'
 else:expr=f'polyline(points=[{points}],line_color={q(stroke or BLUE)},line_width={mm(lw)},line_join=round,start_cap=round,end_cap=round)'
 add(expr,x,y)
def line(x,y,a,b,c=BLUE,lw=2):poly([(x,y),(a,b)],stroke=c,lw=lw)
def arrow_curve(ps,c,width=16,head=33,both=False):
 # Sample a cubic Bezier and construct one filled editable polygon.
 p0,p1,p2,p3=ps
 def point(t):return tuple((1-t)**3*p0[k]+3*(1-t)**2*t*p1[k]+3*(1-t)*t*t*p2[k]+t**3*p3[k] for k in (0,1))
 path=[point(i/36) for i in range(37)]
 # Shorten the shaft under the triangular head.
 end=path[-1];vx=end[0]-path[-2][0];vy=end[1]-path[-2][1];d=math.hypot(vx,vy);ux,uy=vx/d,vy/d
 last=(end[0]-ux*head,end[1]-uy*head)
 path=[p for p in path[:-1] if (end[0]-p[0])*ux+(end[1]-p[1])*uy>head]+[last]
 left=[];right=[]
 for i,p in enumerate(path):
  a=path[max(0,i-1)];b=path[min(len(path)-1,i+1)];dx=b[0]-a[0];dy=b[1]-a[1];d=math.hypot(dx,dy)
  nx,ny=-dy/d,dx/d
  left.append((p[0]+nx*width/2,p[1]+ny*width/2));right.append((p[0]-nx*width/2,p[1]-ny*width/2))
 base=(end[0]-ux*head,end[1]-uy*head)
 points=left+[(base[0]-uy*head*.55,base[1]+ux*head*.55),end,(base[0]+uy*head*.55,base[1]-ux*head*.55)]+right[::-1]
 poly(points,c,WHITE,1.6)
 if both:
  a=path[0];b=path[1];dx=b[0]-a[0];dy=b[1]-a[1];d=math.hypot(dx,dy);ux,uy=dx/d,dy/d
  poly([a,(a[0]+ux*head-uy*head*.55,a[1]+uy*head+ux*head*.55),(a[0]+ux*head+uy*head*.55,a[1]+uy*head-ux*head*.55)],c,WHITE,1.3)
def section(name,x=0,y=0):
 global parent,ox,oy
 parent=name;ox=x;oy=y;out.extend(['',f'# {name}',f'{name}=group()'])
def finish():
 global parent,ox,oy
 out.append(f'page.add({parent},offset=({mm(ox)},{mm(oy)}))');parent='page';ox=oy=0
def doc(x,y,s=1,c=BLUE):
 rect(x,y,38*s,47*s,WHITE,c,2*s,3*s)
 for yy in [11,20,29,38]:line(x+7*s,y+yy*s,x+29*s,y+yy*s,c,2*s)
def gear(x,y,r=16,c=BLUE):
 pts=[]
 for i in range(48):
  a=i*math.pi/24;rr=r if i%6 in (1,2,3,4) else r*.78;pts.append((x+rr*math.cos(a),y+rr*math.sin(a)))
 poly(pts,c);circle(x,y,r*.46,WHITE);circle(x,y,r*.25,c)
def server(x,y,s=1,c=BLUE):
 for i in range(3):
  rect(x,y+i*15*s,47*s,12*s,'#164374','#10265c',1.5*s,3*s)
  circle(x+8*s,y+(i*15+6)*s,2*s,'#56d6fc');line(x+17*s,y+(i*15+6)*s,x+34*s,y+(i*15+6)*s,'#679cce',2*s)
 gear(x+53*s,y+32*s,16*s,c)
def terminal(x,y,s=1,c=BLUE):
 rect(x,y,55*s,43*s,'#153554',c,2*s,3*s)
 poly([(x+10*s,y+10*s),(x+20*s,y+17*s),(x+10*s,y+24*s)],stroke=WHITE,lw=2.5*s)
 line(x+25*s,y+26*s,x+36*s,y+26*s,WHITE,2*s)
 for i in range(3):rect(x+58*s,y+i*14*s,12*s,10*s,'#286282','#123367',1*s,2*s)
def bulb(x,y,s=1):
 circle(x,y,16*s,'#fff5a8','#ffae00',2*s)
 poly([(x-12*s,y+10*s),(x+12*s,y+10*s),(x+6*s,y+27*s),(x-6*s,y+27*s)],'#ffe575','#ffae00',1*s)
 for i in range(3):line(x-6*s,y+(27+i*4)*s,x+6*s,y+(27+i*4)*s,'#233a54',2*s)
 poly([(x-6*s,y+1*s),(x,y+8*s),(x+6*s,y+1*s)],stroke='#e6a514',lw=1.5*s)
 for a in [-math.pi/2,-math.pi/4,0,math.pi,math.pi*1.25]:line(x+22*s*math.cos(a),y+22*s*math.sin(a),x+29*s*math.cos(a),y+29*s*math.sin(a),'#ffb000',2.3*s)
def magnify(x,y,s=1):
 doc(x,y,s*.85);circle(x+35*s,y+28*s,13*s,'#dcfaff',BLUE,4*s);line(x+44*s,y+38*s,x+57*s,y+50*s,BLUE,6*s)
def pencil(x,y,s=1):
 doc(x,y,s*.9)
 poly([(x+25*s,y+36*s),(x+49*s,y+6*s),(x+58*s,y+13*s),(x+34*s,y+43*s)],'#ffe455','#16239d',2*s)
 poly([(x+25*s,y+36*s),(x+34*s,y+43*s),(x+22*s,y+46*s)],'#f0ba72','#16239d',1.4*s)
 poly([(x+46*s,y+10*s),(x+50*s,y+5*s),(x+59*s,y+12*s),(x+55*s,y+17*s)],'#ff5939','#16239d',1.4*s)
def clock(x,y):
 circle(x,y,15,WHITE,NAVY,3);line(x,y,x,y-9,NAVY,2.5);line(x,y,x+9,y+4,NAVY,2.5)
 line(x-16,y-18,x-9,y-23,NAVY,3);line(x+9,y-23,x+16,y-18,NAVY,3)
def sun(x,y):
 circle(x,y,12,'#ffeb63','#ff9d00',2)
 for i in range(8):
  a=i*math.pi/4;line(x+18*math.cos(a),y+18*math.sin(a),x+24*math.cos(a),y+24*math.sin(a),'#ffad00',2)
def check(x,y,r=18):
 circle(x,y,r,'#08b596');poly([(x-r*.5,y),(x-r*.15,y+r*.35),(x+r*.55,y-r*.4)],stroke=WHITE,lw=4)
def card(x,y,w,h,n,label,icon,c=BLUE,pill=None):
 rect(x,y,w,h,WHITE,'#a2e5f9' if c==BLUE else '#ffd6a0',1.4,36)
 circle(x+31,y+29,26,c,WHITE,2)
 text(x+3,y+6,f'{n:02d}',30,WHITE,True,56,'center',font='DejaVu Sans')
 icon(x+w*.46,y+14,.95)
 text(x,y+h-43-(16 if pill else 0),label,25,c if c==BLUE else '#a52509',True,w,'center')
 if pill:rect(x+20,y+h-31,w-40,30,'#d8f7ff',r=14);text(x+20,y+h-29,pill,20,BLUE,True,w-40,'center')

# Header ribbons: native editable polygons, opaque gradients and strokes.
section('header')
rect(0,0,1998,75,fill=grad('#ccf6fd','#31d6f3'))
for k,(c,base) in enumerate([('#009cdd',-10),('#39e4ec',4),('#78f0f2',14),('#c1f7fb',25)]):
 ps=[(400,0)]+[(400+i*1598/45,max(0,base+43*math.sin(i/45*math.pi)+7*math.sin(i/45*math.pi*3+k))) for i in range(46)]+[(1998,0)]
 poly(ps,c)
poly([(420+i*33,10+58*math.sin(i/48*math.pi)) for i in range(49)],stroke=WHITE,lw=2)
text(41,3,'Codex科研实践',41,NAVY,True)
rect(0,75,1998,77,WHITE)
text(42,65,'科研探索周期：传统方式与Agent协作',60,NAVY,True)
text(1710,95,'ABACUS案例启示',32,'#0736e9')
finish()

section('comparison',27,152)
rect(27,152,1948,208,'#e4f5fd',r=20)
text(56,155,'首次探索：非计算流程时间',38,NAVY,True)
text(550,168,'（含必要学习、操作与核验）',29,NAVY,True)
rect(1662,160,290,49,'#fff1be','#ffe292',1,20)
text(1662,164,'情景估算，非实测',28,'#282539',True,290,'center')
rect(49,215,1904,139,WHITE,r=20)
rect(61,223,426,78,r=20,fill=grad('#fff6e9','#ff5000'))
rect(1057,223,369,78,r=20,fill=grad('#c1f2ff','#0064ff'))
# Person/book and laptop symbols.
circle(179,241,17,ORANGE,WHITE,2)
poly([(148,274),(179,263),(210,274),(210,296),(179,303),(148,296)],ORANGE,WHITE,2)
line(179,278,179,301,WHITE,2)
text(243,234,'人工独立推进',33,WHITE,True)
rect(1112,234,56,39,'#053bfb',WHITE,2,4);rect(1118,240,44,26,'#2092ff',WHITE,1.5)
poly([(1114,277),(1167,277),(1179,292),(1102,292)],'#0a43ff',WHITE,2)
text(1195,235,'Codex协作推进',30,WHITE,True)
text(541,207,'约1–3个工作周',57,'#e5390a',True)
text(655,276,'40–120小时',30,NAVY,True)
text(1480,207,'约1–3个工作日',57,'#072eff',True)
text(1608,276,'8–24小时',30,NAVY,True)
line(990,240,990,295,'#a1b4d7',2)
rect(60,314,1882,44,'#e6f5fc',r=18)
text(244,319,'已有DFT基础，首次接触ABACUS；同以首轮可核验试算为终点。',24,'#213259')
text(1140,319,'按8小时/工作日、5日/工作周折算；另加排队、计算等待与响应间隔。',24,'#213259')
finish()

# Native rounded picture-filled shapes; arrows painted before the foreground cards.
for name,dx,c,asset,title in [('traditional',0,ORANGE,'traditional.png','传统方式：研究者逐项实施'),('agent',974,BLUE,'agent.png','Agent协作：研究者指导与验收')]:
 section(name,33+dx,369)
 pic(33+dx,369,958,429,asset,16)
 rect(47+dx,372,540 if dx==0 else 587,56,r=15,fill=grad('#ff5200' if dx==0 else '#0059fc','#ff9920' if dx==0 else '#00b8ee'),stroke=WHITE,lw=1.5)
 text(77+dx,369,title,38,WHITE,True)
 for ps in [[(644,463),(682,465),(708,482),(731,507)],[(813,613),(813,636),(797,658),(781,677)],[(678,752),(605,791),(427,788),(335,756)],[(197,672),(175,654),(163,637),(177,615)],[(261,514),(300,487),(342,474),(386,464)]]:
  arrow_curve([(x+dx,y) for x,y in ps],c,17,34)
 text(236+dx,464,'进入下一轮',25,'#b82b0b' if dx==0 else '#112ace',True)
 # The original book spine labels remain editable text, over editable books.
 if dx==0:
  for yy,col in [(578,'#6bafdf'),(602,'#654f48'),(626,'#245a7d'),(650,'#705545')]:
   rect(278,yy,131,22,col,'#21354c',1.5,3)
   rect(382,yy+2,27,18,'#f4efe5','#34495f',1,2)
  text(289,601,'文献调研',16,'#fff6e6',True)
  text(289,625,'计算方法',16,'#eaf4ff',True)
  text(289,649,'知识积累',16,'#fff6e6',True)
 # Top idea node: horizontal compact layout.
 rect(394+dx,433,238,64,WHITE,'#ffd3a2' if dx==0 else '#93e9fa',1.4,32)
 circle(427+dx,465,26,c,WHITE,2);text(401+dx,441,'01',30,WHITE,True,52,'center',font='DejaVu Sans')
 bulb(478+dx,459,.86);text(503+dx,447,'提出设想',27,'#aa220a' if dx==0 else NAVY,True)
 if dx:
  rect(1457,483,150,32,'#ffecd1',r=16);text(1457,482,'研究者',22,'#852713',True,150,'center')
 card(704+dx,510,181,103 if dx==0 else 114,2,'准备验证',server,c,'Codex协助' if dx else None)
 card(679+dx,676,195,103 if dx==0 else 118,3,'执行试算',terminal,c,'Codex协助' if dx else None)
 card(150+dx,675,184,104 if dx==0 else 119,4,'分析核验',magnify,c,'人机协作' if dx else None)
 card(98+dx,508,192,103,5,'调整方案',pencil,c)
 if dx==0:
  rect(345,708,302,45,'#fff7e9',WHITE,1,20);text(345,711,'实施与判断均需投入精力',24,'#a72510',True,302,'center')
 else:
  arrow_curve([(1303,731),(1380,752),(1556,754),(1670,731)],BLUE,16,32,True)
  rect(1299,756,388,37,'#effaff',r=18);text(1299,756,'Codex协助实施，研究者把握判断',24,NAVY,True,388,'center')
 finish()

section('continuation',29,799)
rect(29,806,1940,173,'#eef5ff',r=17)
text(59,799,'任务完成后，响应间隔也会拉长周期（示例）',36,NAVY,True)
rect(47,848,944,131,'#fff7ec',WHITE,2,16)
rect(1005,848,953,131,'#e6f6ff',WHITE,2,16)
pic(49,886,321,91,'sleeping.png',5)
rect(49,847,329,40,ORANGE,WHITE,1.4,14);text(66,846,'人工接续（未配置自动化）',27,WHITE,True)
rect(1014,847,451,40,BLUE,WHITE,1.4,14);text(1032,846,'Agent辅助流程（已配置持续执行）',27,WHITE,True)
# Automatic server/moon vignette is built from editable vectors.
rect(1015,887,258,90,fill=grad('#285786','#123669'))
circle(1060,919,20,'#ffe99a');circle(1071,910,19,'#275580')
for x,y in [(1036,911),(1100,952),(1087,901)]:poly([(x,y-5),(x+2,y-2),(x+5,y),(x+2,y+2),(x,y+5),(x-2,y+2),(x-5,y),(x-2,y-2)],'#fff7bb')
server(1150,911,.92);server(1200,912,.86)
rect(389,880,144,57,WHITE,r=20);clock(416,907);text(451,879,'02:00',23,NAVY,True);text(448,905,'任务结束',20,NAVY,True)
rect(580,880,185,57,WHITE,r=20)
# Hourglass.
line(596,891,618,891,NAVY,3);line(596,923,618,923,NAVY,3)
poly([(598,894),(616,894),(609,907),(616,920),(598,920),(605,907),(598,894)],stroke=NAVY,lw=2)
poly([(600,896),(614,896),(607,905)],'#5aa7ff');poly([(600,918),(614,918),(607,909)],'#eec954')
text(633,891,'等待人工响应',21,NAVY,True)
rect(813,880,157,57,WHITE,r=20);sun(845,906);text(880,879,'08:00',23,NAVY,True);text(876,905,'检查接续',20,NAVY,True)
for x in [535,765]:arrow_curve([(x,908),(x+12,908),(x+26,908),(x+39,908)],ORANGE,12,15)
rect(389,942,580,32,'#ffebd5',r=16);text(389,941,'示例：额外等待约6小时',22,'#272d5b',True,580,'center')
rect(1287,880,655,57,WHITE,r=20)
doc(1300,892,.66);text(1344,894,'任务结束',22,NAVY,True)
gear(1518,909,19);text(1548,894,'自动检查',22,NAVY,True)
check(1724,909,19);text(1752,894,'通过预设验收后接续',22,NAVY,True)
for x in [1442,1646]:arrow_curve([(x,910),(x+12,910),(x+26,910),(x+39,910)],BLUE,12,15)
rect(1287,942,655,32,'#d6f0ff',r=16);text(1287,942,'超出规则或需人工判断时，仍等待人工处理',22,NAVY,True,655,'center')
finish()

section('footer',0,978)
rect(27,984,1946,37,'#e1f6ff',r=12)
circle(146,1001,19,BLUE,WHITE,2);text(135,978,'!',30,WHITE,True,font='DejaVu Sans')
text(194,987,'人工响应间隔、计算失败与额外验证，都可能延长实际周期。',24,NAVY,True)
line(902,991,902,1016,'#627dda',1.5);gear(1012,1001,18)
text(1040,987,'传统调度脚本也能自动接续；Agent可协助搭建与维护，科学判断仍需研究者。',23,NAVY,True)
rect(22,1025,1954,47,BLUE,r=13)
text(612,1024,'降低',34,WHITE,True);text(681,1024,'学习与操作门槛',34,'#ffff3a',True)
text(920,1024,'，减少可自动化环节的',34,WHITE,True);text(1288,1024,'等待',34,'#ffff3a',True)
rect(0,1075,1998,50,fill=grad('#164e98','#07397e'))
text(38,1077,'logo',35,WHITE,True,font='DejaVu Sans');text(1865,1078,'第XX页',31,WHITE)
finish()
(ROOT/'research-cycle.lay').write_text('\n'.join(out)+'\n')
(ROOT/'labels.json').write_text(json.dumps(labels,ensure_ascii=False,indent=2)+'\n')
print(f'{len(labels)} source labels; {len(out)} source statements')
