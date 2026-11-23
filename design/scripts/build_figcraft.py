#!/usr/bin/env python3
"""Build the editable Life Manager screen system in native FigCraft JSON.

Design tooling only; no application logic is implemented here.

The palette comes from ``tokens.py``, so the design master, the exported CSS
and the running app share one set of design tokens.  Icons are real Lucide
vector geometry read from the ``lucide-svelte`` package the app already
depends on.
"""
import copy, csv, json, pathlib, re
import tokens
import grid
from svgpath import path_network
import lucide

ROOT=pathlib.Path(__file__).resolve().parents[2]
OUT=ROOT/'design'
BASE=json.load(open(pathlib.Path(__file__).with_name('figcraft-template.json')))
W,H=int(tokens.CANVAS['width']),int(tokens.CANVAS['height'])

# Token table -> short palette keys used by the view code.
P={}
for _n,(_l,_d,_x) in tokens.COLORS.items(): P[_n]=_l
P.update({
 'paper':P['surface'],'ink':P['text'],'ink2':P['text-secondary'],'sub':P['text-muted'],
 'faint':P['text-faint'],'nav':P['surface-inverse'],'nav2':P['surface-inverse-raised'],
 'mint':P['accent-muted'],'green':P['accent'],'green2':P['accent-soft-text'],
 'rust':P['warning'],'peach':P['warning-muted'],'yellow':P['yellow-muted'],
 'blue':P['info'],'violet':P['violet-muted'],'red':P['danger'],'bg':P['canvas'],
 'line':P['border'],'paper2':P['surface-raised'],
})
# Palette key -> design token name, for binding painted layers to a token.
TOKEN_FOR={
 'paper':'surface','ink':'text','ink2':'text-secondary','sub':'text-muted',
 'faint':'text-faint','nav':'surface-inverse','nav2':'surface-inverse-raised',
 'mint':'accent-muted','green':'accent','green2':'accent-soft-text','rust':'warning',
 'peach':'warning-muted','yellow':'yellow-muted','blue':'info','violet':'violet-muted',
 'red':'danger','bg':'canvas','line':'border','paper2':'surface-raised',
}

def rgba(h): return tokens.hex_to_rgba(h)
def paint(c,opacity=1.0): return tokens.paint(c,opacity)
def token_of(key): return TOKEN_FOR.get(key,key)

class Doc:
    def __init__(self):
        self.j=copy.deepcopy(BASE);self.j['nodes']={};self.j['pages']=[];self.j['current_page']=None
        self.j['name']='Life Manager — Experience Master';self.n=1;self.frames=[];self.page_names={}
        self.instances={};self.icon_net={}
    def add(self,node,parent=None):
        i=self.n;self.n+=1;node['id']=i
        if parent is not None:
            node['parent']=parent;self.j['nodes'][str(parent)].setdefault('children',[]).append(i)
        self.j['nodes'][str(i)]=node
        return i
    def page(self,name):
        n=copy.deepcopy(BASE['nodes']['1']);n['name']=name;n['children']=[]
        p=self.add(n);self.j['pages'].append(p);self.page_names[name]=p
        if self.j['current_page'] is None:self.j['current_page']=p
        return p
    def frame(self,parent,name,x,y):
        n={'type':'frame','name':name,'width':W,'height':H,'children':[],'clip':True,
           'fills':paint(P['bg']),'transform':[1.0,0.0,0.0,1.0,float(x),float(y)]}
        fid=self.add(n,parent);self.frames.append({'id':fid,'page':parent,'name':name,'x':x,'y':y,'nodes':[]});return fid
    def rect(self,parent,x,y,w,h,c,r=0,name='',stroke=None):
        n={'type':'rectangle','name':name or 'Shape','width':float(w),'height':float(h),
           'fills':paint(c),'transform':[1.0,0.0,0.0,1.0,float(x),float(y)]}
        if r:n['corner_radius']=float(r)
        if stroke:n['strokes']=paint(stroke);n['stroke_weight']=1.0
        return self.add(n,parent)
    def ellipse(self,parent,x,y,w,h,c,name='Dot'):
        n={'type':'ellipse','name':name,'width':float(w),'height':float(h),
           'fills':paint(c),'transform':[1.0,0.0,0.0,1.0,float(x),float(y)]}
        return self.add(n,parent)
    def text(self,parent,s,x,y,w,h=30,size=14,color=None,weight='Regular',name=None,align='left'):
        n=copy.deepcopy(BASE['nodes']['4']);n.update({'type':'text','name':name or ('Text · '+str(s)[:24]),
           'characters':str(s),'width':float(w),'height':float(h),
           'transform':[1.0,0.0,0.0,1.0,float(x),float(y)],'fills':paint(color or P['ink'])})
        n['style']['font_family']='Helvetica Neue';n['style']['font_size']=float(size);n['style']['font_style']=weight
        n['auto_resize']='none'
        if align!='left':n['text_align']=align
        return self.add(n,parent)
    def icon(self,parent,name,x,y,size=18.0,color='ink',weight=1.8,label=None):
        """A real Lucide icon as an editable vector layer.

        The geometry is scaled so the icon fills a square box of ``size``
        pixels, so an icon is optically the same size as its neighbours
        whatever shape Lucide draws it in.  The layer's origin is the box's
        top-left corner, which is how FigCraft places a vector.
        """
        key=(name,round(size,2))
        if key not in self.icon_net:
            data=lucide.icon_path(name)
            net,bbox=path_network(data,normalize=False)
            # Scale the icon's own box onto the requested square, so every
            # icon reads at the same visual weight.
            sx=size/bbox[2] if bbox[2] else 1.0
            sy=size/bbox[3] if bbox[3] else 1.0
            for v in net['vertices']:
                v['x']*=sx;v['y']*=sy
            for s in net['segments']:
                s['ta']=[s['ta'][0]*sx,s['ta'][1]*sy]
                s['tb']=[s['tb'][0]*sx,s['tb'][1]*sy]
            # FigCraft stores a vector with its vertices relative to the
            # shape's own bounding-box origin and the transform carrying
            # that origin.  The geometry is centred in the requested square
            # and the origin it is written with is the square's own corner,
            # so the layer's box is the whole square.
            off_x=(size-bbox[2]*sx)/2.0
            off_y=(size-bbox[3]*sy)/2.0
            for v in net['vertices']:
                v['x']+=off_x-bbox[0]*sx;v['y']+=off_y-bbox[1]*sy
            self.icon_net[key]=(net,size,size,0.0,0.0)
        net,w,h,ox,oy=self.icon_net[key]
        n={'type':'vector','name':label or ('Icon · '+name),'network':net,
           'width':float(w),'height':float(h),
           'transform':[1.0,0.0,0.0,1.0,float(x)+ox,float(y)+oy],
           'fills':[],'strokes':paint(P.get(color,color)),
           'stroke':{'weight':float(weight),'align':'center','sides':None,'cap_start':'round',
                     'cap_end':'round','join':'round','miter_limit':4.0,'dashes':[]}}
        return self.add(n,parent)

    def icon_at(self,parent,name,text_x,text_y,text_font,size,color='ink',weight=1.8,gap=12):
        """An icon placed beside a text label, on the label's cap height.

        This is the rule the alignment audit checks: the icon's centre and
        the label's cap centre sit on the same horizontal line.
        """
        x=text_x-gap-size
        y=grid.icon_y(text_y,0,size,text_font)
        return self.icon(parent,name,x,y,size,color,weight)
    def chip(self,p,s,x,y,w=None,c='mint',ink='green',icon=None):
        w=w or max(64,18+len(s)*7);self.rect(p,x,y,w,27,P.get(c,c),13)
        offset=11
        if icon:
            self.icon(p,icon,x+9,y+6,15,ink,1.8);offset=30
        self.text(p,s,x+offset,y+6,w-offset-6,20,11,P.get(ink,ink),'Bold');return w
    def rule(self,p,x,y,w):self.rect(p,x,y,w,1,P['line'])
    def block(self,p,x,y,w,h,title,subtitle=None):
        self.rect(p,x,y,w,h,P['paper'],14,stroke=P['line']);self.text(p,title,x+22,y+19,w-44,29,17,P['ink'],'Bold')
        if subtitle:self.text(p,subtitle,x+22,y+48,w-44,22,11,P['sub'])
    def label(self,p,s,x,y,w=350):self.text(p,s.upper(),x,y,w,18,10,P['sub'],'Bold')
    def bar(self,p,x,y,w,fraction,accent='green'):
        self.rect(p,x,y,w,7,P['line'],3);self.rect(p,x,y,max(8,w*fraction),7,P[accent],3)
    def save(self,path):
        self.j['next_id']=self.n
        with open(path,'w') as f:json.dump(self.j,f,ensure_ascii=False,separators=(',',':'))

D=Doc()
NAV=['Today','Inbox','Tasks','Calendar','Projects','Goals','Focus','Habits','Notes','Reviews','Agents','Settings']
NAV_ICONS={'Today':'layout-dashboard','Inbox':'inbox','Tasks':'list-todo','Calendar':'calendar-days',
 'Projects':'folder-kanban','Goals':'target','Focus':'timer','Habits':'repeat-2','Notes':'notebook-pen',
 'Reviews':'chart-no-axes-combined','Agents':'bot','Settings':'settings-2'}
MODULE_CONTENT={
'Today':['Plan the day with intention','Review top priorities','Protect a deep-work block','Finish the most important task','Process new captures'],
'Inbox':['Review captured ideas','Clarify vague action','Assign a project or area','Schedule or save for later','Archive processed items'],
'Tasks':['Review design specifications','Implement data migration tests','Prepare technical interview','Study Kubernetes scheduling','Check upcoming obligations'],
'Projects':['Life Manager desktop application','Dhamma Echo redesign','Jet Engine Lab digital twin','Yangon City Simulator','Portfolio improvement'],
'Goals':['Ship Life Manager v1','Maintain consistent deep work','Complete CKAD preparation','Strengthen career portfolio','Review quarterly direction'],
'Calendar':['Daily planning','Deep work · Architecture','Lunch and reset','Project design review','Evening review'],
'Planning':['Choose the three priorities','Allocate protected work sessions','Confirm calendar constraints','Reserve break and transition time','Carry over unfinished work'],
'Focus':['Architecture specification','Design system refinement','Deep work session','Interruptions and recovery','Weekly focus review'],
'Habits':['Morning meditation','Read for 20 minutes','Take an evening walk','Close the day intentionally','Prepare tomorrow'],
'Notes':['Product vision and principles','Architecture research','Weekly reflections','Meeting notes','Useful ideas to revisit'],
'Reviews':['Review wins and difficulties','Compare plan against reality','Reflect on time allocation','Identify next-week blockers','Set one deliberate improvement'],
'Life':['Career application follow-up','System design coursework','Monthly finance review','Health and movement routine','Personal administrative tasks'],
'Agents':['Pi created two follow-up tasks','A draft schedule is awaiting review','CLI exported weekly report','Agent completed a project search','Review changes before applying'],
'Settings':['Appearance and density','Language and time zone','Notification preferences','Backup and recovery','Agent permissions'],
'States':['Resolve the open item','Preserve your current work','Review the suggested recovery','Retry the last operation','Open diagnostics'],
'System':['Today dashboard','Global command palette','Navigation and layout','Keyboard shortcuts','Compact density'],
'Onboarding':['Set up your life areas','Choose a daily routine','Configure your working hours','Review privacy settings','Start your first plan'],
}
def sidebar(fid,active='Today'):
    D.rect(fid,0,0,252,H,P['nav'],name='Sidebar · persistent')
    D.ellipse(fid,23,25,30,30,P['green2'],'Brand mark');D.rect(fid,37,22,7,35,P['nav'],2)
    D.text(fid,'LIFE',67,24,120,26,17,'#F8FCF7','Bold');D.text(fid,'/ MANAGER',117,28,110,18,10,'#A8BFB7','Bold')
    D.text(fid,'PERSONAL WORKSPACE',23,91,200,22,10,'#91ACA3','Bold')
    y=124
    for k in NAV:
        if k in ('Projects','Focus','Agents'):
            D.text(fid,'PLAN & TRACK' if k=='Projects' else ('RHYTHM' if k=='Focus' else 'SYSTEM'),24,y,195,17,9,'#72958A','Bold');y+=24
        on=k==active
        if on:D.rect(fid,12,y-4,228,34,'#335A4B',8)
        # The icon is centred on the label's cap height, so the row reads as
        # one horizontal line.
        D.text(fid,k,58,y+1,174,22,13,('#FFFFFF' if on else '#C3D6CF'),'Bold' if on else 'Regular')
        D.icon(fid,NAV_ICONS[k],27,grid.icon_y(y+1,22,20,13),20,'#ECF7F1' if on else '#7CA69A',1.7)
        y+=39
    D.rule(fid,23,815,206)
    D.ellipse(fid,22,835,28,28,P['rust'],'Profile avatar')
    D.text(fid,'Personal workspace',60,832,169,20,11,'#E6F1EA','Bold')
    D.text(fid,'Offline · All changes local',60,851,172,20,10,'#91AFA4')
def shell(fid,title,subtitle,section='Today',tag='PERSONAL COMMAND CENTER',badge=None):
    sidebar(fid,section)
    D.text(fid,'WORKSPACE  /  '+section.upper(),grid.LEFT,27,530,21,11,P['sub'],'Bold')
    D.rect(fid,1035,16,247,38,P['paper'],9,stroke=P['line'])
    D.icon(fid,'search',1050,25,19,'faint',1.8)
    D.text(fid,'Search anything...',1078,26,150,21,11,P['faint'])
    D.chip(fid,'⌘ K',1232,23,38,'bg','sub')
    D.ellipse(fid,1357,20,30,30,P['peach'],'User')
    D.rule(fid,252,73,1188)
    D.text(fid,tag,grid.LEFT,105,800,19,10,P['green'],'Bold')
    D.text(fid,title,grid.LEFT,129,1005,52,32,P['ink'],'Bold')
    D.text(fid,subtitle,grid.LEFT,183,1030,30,13,P['sub'])
    if badge:D.chip(fid,badge,1260,139,137,'peach','rust')
def metric(fid,x,y,w,k,v,note,accent='green'):
    D.rect(fid,x,y,w,115,P['paper'],14,stroke=P['line'])
    D.label(fid,k,x+21,y+17,w-42);D.text(fid,v,x+21,y+41,w-42,44,33,P['ink'],'Bold');D.text(fid,note,x+21,y+89,w-42,19,11,P[accent])
def rows(fid,x,y,w,items,maxn=5,context='task'):
    for i,s in enumerate(items[:maxn]):
        yy=y+i*61
        if i:D.rule(fid,x,yy-4,w)
        D.ellipse(fid,x,yy+12,19,19,'#FFFFFF','Checkbox');D.ellipse(fid,x+1.5,yy+13.5,16,16,P['line'],'Checkbox outline');D.ellipse(fid,x+4.5,yy+16.5,10,10,P['paper'],'Checkbox interior')
        D.text(fid,s,x+35,yy+3,w-154,25,13,P['ink'],'Bold')
        D.text(fid,['Today · 45m','Tomorrow · 30m','No due date','Friday · 1h','Next week · 25m'][i%5],x+35,yy+26,w-164,19,10,P['sub'])
        D.chip(fid,['P1','P2','P3','P2','P4'][i%5],x+w-70,yy+14,47,('peach' if i==0 else 'bg'),('rust' if i==0 else 'sub'))
def footer(fid,caption):
    D.rule(fid,288,866,1113)
    D.text(fid,caption,290,873,975,18,10,P['faint'])
    D.text(fid,'LOCAL  •  PRIVATE  •  AGENT-READY',1184,873,222,19,9,P['sub'],'Bold')

def view_dashboard(f,m,title,state):
    mx=grid.metric_row()
    metric(f,mx[0][0],242,mx[0][1],'TODAY COMPLETED','4 / 7','Three priorities still open')
    metric(f,mx[1][0],242,mx[1][1],'FOCUS TIME','3h 20m','68% of daily intention')
    metric(f,mx[2][0],242,mx[2][1],'AVAILABLE CAPACITY','2h 15m','Protected until 18:00','rust')
    bx=grid.body_columns()
    D.block(f,bx[0][0],373,bx[0][1],467,'Your next meaningful actions','Selected for impact, not just urgency')
    rows(f,bx[0][0]+21,453,bx[0][1]-42,MODULE_CONTENT.get(m,MODULE_CONTENT['Today']))
    D.rect(f,bx[0][0]+22,770,bx[0][1]-44,46,P['mint'],10)
    D.icon(f,'plus',bx[0][0]+43,782,20,'green',2.0)
    D.text(f,'Capture a task, thought, or commitment',bx[0][0]+72,783,bx[0][1]-106,25,13,P['green'],'Bold')
    D.block(f,bx[1][0],373,bx[1][1],225,'The shape of your day','Time blocks and commitments')
    for i,(time,label) in enumerate([('09:00','Plan & prioritise'),('10:30','Protected focus'),('15:00','Project review')]):
        yy=443+i*48;D.text(f,time,bx[1][0]+21,yy,66,20,11,P['sub'],'Bold')
        D.rect(f,bx[1][0]+90,yy-2,4,28,[P['green'],P['rust'],P['green2']][i],2)
        D.text(f,label,bx[1][0]+106,yy,bx[1][1]-128,24,12,P['ink'])
    D.block(f,bx[1][0],611,bx[1][1],229,'Keep the rhythm','A sustainable day is a productive day')
    D.text(f,'Next break',bx[1][0]+22,684,190,22,12,P['sub'])
    D.text(f,'11:45',bx[1][0]+22,712,190,43,30,P['ink'],'Bold')
    D.chip(f,'12 minutes',bx[1][0]+bx[1][1]-135,713,113,'mint','green')
    D.bar(f,bx[1][0]+22,781,bx[1][1]-44,.63)
def view_list(f,m,title,state):
    D.chip(f,'All',grid.LEFT,238,56);D.chip(f,'Today',grid.LEFT+67,238,73,'bg','sub')
    D.chip(f,'Upcoming',grid.LEFT+152,238,92,'bg','sub');D.chip(f,'Completed',grid.LEFT+255,238,103,'bg','sub')
    D.rect(f,grid.RIGHT-159,233,159,36,P['green'],8)
    D.icon(f,'plus',grid.RIGHT-141,243,17,'#FFFFFF',2.2)
    D.text(f,'New item',grid.RIGHT-118,242,110,22,12,'#FFFFFF','Bold')
    bx=grid.body_columns()
    D.block(f,bx[0][0],292,bx[0][1],548,'Priority queue','Clear next actions with their own context and dates')
    D.label(f,'ITEM',bx[0][0]+21,365,342);D.label(f,'WHEN',bx[0][0]+599,365,110);D.rule(f,bx[0][0]+21,388,bx[0][1]-46)
    items=(MODULE_CONTENT.get(m,MODULE_CONTENT['Tasks'])*2)[:7]
    for i,s in enumerate(items):
        yy=403+i*59
        D.ellipse(f,bx[0][0]+22,yy+10,18,18,P['line']);D.ellipse(f,bx[0][0]+25,yy+13,12,12,P['paper'])
        D.text(f,s,bx[0][0]+58,yy+2,bx[0][1]-183,26,13,P['ink'],'Bold')
        D.text(f,['Architecture · Deep work','Learning · 30m','Personal · Today','Project · 60m','Later · Flexible'][i%5],bx[0][0]+59,yy+29,bx[0][1]-192,20,10,P['sub'])
        D.text(f,['Today','Tomorrow','Thu, Nov 12','Next week','No date'][i%5],bx[0][0]+588,yy+12,136,20,11,P['sub'])
        if i<6:D.rule(f,bx[0][0]+22,yy+53,bx[0][1]-48)
    D.block(f,bx[1][0],292,bx[1][1],252,'Task lens','Make large backlogs feel smaller')
    for i,(lab,v) in enumerate([('P1 · Critical','03'),('Blocked','02'),('Overdue','01')]):
        yy=368+i*50;D.text(f,lab,bx[1][0]+20,yy,bx[1][1]-76,24,12,P['ink'])
        D.text(f,v,bx[1][0]+bx[1][1]-81,yy,46,24,15,P['green'],'Bold')
    D.block(f,bx[1][0],557,bx[1][1],283,'Saved views','One click to useful perspectives')
    for i,s in enumerate(['Deep work · 60+ min','Quick wins · under 20m','Waiting on others','No deadline assigned']):
        D.text(f,s,bx[1][0]+22,633+i*44,bx[1][1]-46,25,12,P['ink2'])
def view_board(f,m,title,state):
    statuses=[('PLANNED',P['blue']),('IN PROGRESS',P['peach']),('READY / DONE',P['mint'])]
    data=MODULE_CONTENT.get(m,MODULE_CONTENT['Projects'])
    for col,(status,colr) in enumerate(statuses):
        x,w=grid.columns(3)[col]
        D.rect(f,x,250,w,590,'#EFF2F0',14)
        D.ellipse(f,x+19,273,10,10,colr)
        D.text(f,status,x+38,270,w-98,24,11,P['ink'],'Bold');D.chip(f,str([4,3,5][col]),x+w-54,266,37,'paper','sub')
        for i in range(3):
            y=315+i*162;D.rect(f,x+15,y,w-30,147,P['paper'],12,stroke=P['line'])
            t=data[(col*3+i)%len(data)]
            D.text(f,t,x+30,y+23,w-70,54,15,P['ink'],'Bold')
            D.text(f,['Small and focused','Needs design review','Waiting on a milestone'][i],x+30,y+82,w-70,24,11,P['sub'])
            D.chip(f,['P1','P2','P3'][i],x+30,y+111,42,'peach' if i==0 else 'bg','rust' if i==0 else 'sub')
            D.text(f,['2 subtasks','45 min','Due Fri'][i],x+w-125,y+117,102,20,10,P['sub'])
        D.icon(f,'plus',x+28,804,16,'green',2.0);D.text(f,'Add card',x+51,805,180,22,12,P['green'],'Bold')
def view_details(f,m,title,state):
    D.chip(f,'P1 · FOCUS',grid.LEFT,243,112,'peach','rust');D.chip(f,'IN PROGRESS',grid.LEFT+123,243,135,'mint','green')
    bx=grid.body_columns()
    D.block(f,bx[0][0],289,bx[0][1],551,'Work through the essentials','An actionable task, not a wall of metadata')
    main=MODULE_CONTENT.get(m,MODULE_CONTENT['Tasks'])[0]
    D.text(f,main,bx[0][0]+24,362,bx[0][1]-48,39,23,P['ink'],'Bold')
    D.text(f,'Outcome: a clear, testable result with an explicit next step.',bx[0][0]+24,415,bx[0][1]-48,50,14,P['ink2'])
    D.label(f,'CHECKLIST',bx[0][0]+24,487)
    for i,s in enumerate(['Clarify scope and expected outcome','Gather supporting information','Execute the smallest meaningful step','Verify and record the result']):
        yy=524+i*62;D.rect(f,bx[0][0]+24,yy,22,22,P['paper'],5,stroke=P['line'])
        D.text(f,s,bx[0][0]+61,yy,bx[0][1]-110,30,13,P['ink'])
    D.rect(f,bx[0][0]+23,780,bx[0][1]-48,39,P['mint'],9)
    D.text(f,'Mark complete',bx[0][0]+44,790,bx[0][1]-90,22,12,P['green'],'Bold')
    D.block(f,bx[1][0],289,bx[1][1],295,'Details','Editable from the app, CLI, or MCP')
    for i,(a,b) in enumerate([('Project',m),('Due','Friday · 17:00'),('Estimate','45 minutes'),('Context','Deep work')]):
        yy=369+i*48;D.text(f,a,bx[1][0]+22,yy,bx[1][1]-101,22,11,P['sub'])
        D.text(f,b,bx[1][0]+155,yy,bx[1][1]-179,22,12,P['ink'],'Bold')
    D.block(f,bx[1][0],599,bx[1][1],241,'Related work','Never lose dependencies')
    D.text(f,'○  Draft architecture',bx[1][0]+23,670,bx[1][1]-46,22,12,P['ink'])
    D.text(f,'→  Blocks the implementation phase',bx[1][0]+23,714,bx[1][1]-46,36,12,P['sub'])
    D.chip(f,'View activity',bx[1][0]+20,786,131,'mint','green')
def view_calendar(f,m,title,state):
    D.chip(f,'‹',290,240,38,'paper','ink');D.chip(f,'›',333,240,38,'paper','ink');D.text(f,'November 9–15, 2026',395,241,525,31,18,P['ink'],'Bold')
    D.chip(f,'Day',1152,240,55,'bg','sub');D.chip(f,'Week',1215,240,70,'mint','green');D.chip(f,'Month',1296,240,96,'bg','sub')
    x0,y0=290,296;wi,hh=1109,528;step=158
    D.rect(f,x0,y0,wi,hh,P['paper'],12,stroke=P['line'])
    for c,name in enumerate(['MON 09','TUE 10','WED 11','THU 12','FRI 13','SAT 14','SUN 15']):
        xx=x0+56+c*150
        D.text(f,name,xx,y0+18,139,25,11,P['green'] if c==0 else P['sub'],'Bold')
        if c>0:D.rect(f,x0+48+c*150,y0+55,1,hh-60,P['line'])
    for i,time in enumerate(['09:00','10:00','11:00','12:00','13:00','14:00','15:00','16:00']):
        yy=y0+70+i*56;D.text(f,time,x0+9,yy,45,18,10,P['faint']);D.rect(f,x0+49,yy+20,wi-60,1,P['line'])
    events=[(0,0,'Daily plan',P['mint'],P['green']),(0,1,'Deep work',P['blue'],P['ink']),(1,3,'Review backlog',P['peach'],P['rust']),(2,1,'Study session',P['violet'],P['ink2']),(3,4,'Project sync',P['mint'],P['green']),(4,2,'Weekly review',P['yellow'],P['ink'])]
    for day,slot,lab,bg,fg in events:
        xx=x0+57+day*150;yy=y0+82+slot*56
        D.rect(f,xx,yy,135,48,bg,8);D.text(f,lab,xx+9,yy+7,117,22,11,fg,'Bold');D.text(f,['45 min','90 min','30 min'][slot%3],xx+9,yy+27,115,17,9,fg)
def view_planning(f,m,title,state):
    bx=grid.body_columns()
    D.block(f,bx[0][0],246,bx[0][1],594,'Time budget','A day that acknowledges reality')
    for i,(name,amt,fr) in enumerate([('Deep work','3h 00m',.83),('Meetings','1h 30m',.42),('Admin','45m',.2),('Breaks','1h 00m',.28)]):
        y=320+i*113;D.text(f,name,bx[0][0]+22,y,bx[0][1]-150,24,13,P['ink'],'Bold')
        D.text(f,amt,bx[0][0]+bx[0][1]-112,y,84,24,12,P['sub'])
        D.bar(f,bx[0][0]+21,y+36,bx[0][1]-42,fr,'green' if i<2 else 'rust')
    D.block(f,bx[1][0],246,bx[1][1],594,'Schedule proposal','Review and apply; existing appointments are protected')
    for i,(time,lab,descr) in enumerate([('08:30','Plan the day','20 min · Organise'),('09:00','Architecture deep work','90 min · Protected'),('10:45','Break and reset','15 min · Recovery'),('11:00','Project design review','60 min · Focus'),('13:00','Study and practice','45 min · Learning'),('15:00','Administrative follow-up','30 min · Routine')]):
        yy=329+i*73;D.text(f,time,bx[1][0]+23,yy,85,25,13,P['sub'],'Bold')
        D.rect(f,bx[1][0]+119,yy-5,5,49,P['green'] if i%2==0 else P['rust'],3)
        D.text(f,lab,bx[1][0]+142,yy-3,bx[1][1]-165,26,14,P['ink'],'Bold')
        D.text(f,descr,bx[1][0]+142,yy+23,bx[1][1]-165,22,11,P['sub'])
        if i<5:D.rule(f,bx[1][0]+23,yy+55,bx[1][1]-46)
    D.rect(f,bx[1][0]+bx[1][1]-239,782,210,39,P['green'],9)
    D.text(f,'Review changes',bx[1][0]+bx[1][1]-222,793,182,24,12,'#FFFFFF','Bold')
def view_focus(f,m,title,state):
    D.rect(f,289,244,1110,595,'#DDECE3',18)
    D.text(f,'ONE THING AT A TIME',334,291,680,25,13,P['green'],'Bold')
    D.text(f,'Architecture specification',335,336,970,49,30,P['ink'],'Bold')
    D.ellipse(f,514,405,377,377,'#B7D7C8','Focus ring');D.ellipse(f,537,428,331,331,'#E9F4ED','Focus center')
    D.text(f,'38:24',591,515,310,91,64,P['ink'],'Bold')
    D.text(f,'SESSION 02 / 04',630,614,250,29,13,P['green'],'Bold')
    D.chip(f,'Pause',608,688,91,'paper','ink');D.chip(f,'Complete',711,688,111,'green','#FFFFFF')
    D.text(f,'PROTECTED SESSION',1004,446,309,26,12,P['green'],'Bold')
    D.text(f,'Focus is a decision.\nThe interface stays quiet.',1004,499,322,128,22,P['ink'],'Bold')
    D.text(f,'No automatic task switching. Progress\nis recorded when you finish.',1004,660,340,58,12,P['sub'])
def view_habits(f,m,title,state):
    D.block(f,290,243,1109,598,'Your routines','Track consistency without turning life into a scoreboard')
    for i,s in enumerate(MODULE_CONTENT['Habits']):
        yy=345+i*91;D.text(f,s,314,yy,327,27,15,P['ink'],'Bold');D.text(f,['Daily · morning','Daily · any time','Mon/Wed/Fri','Daily · evening','Every weekday'][i],314,yy+29,290,20,11,P['sub'])
        for day in range(7):
            x=672+day*93
            D.ellipse(f,x,yy+7,30,30,(P['green'] if (i+day)%6!=1 else P['line']))
            if (i+day)%6!=1:D.text(f,'✓',x+7,yy+11,20,23,16,'#FFFFFF','Bold')
        if i<4:D.rule(f,313,yy+73,1067)
    for i,s in enumerate('MTWTFSS'):D.text(f,s,683+i*93,304,38,21,12,P['sub'],'Bold')
def view_analytics(f,m,title,state):
    metric(f,290,240,350,'WEEKLY COMPLETION','83%','+9 points vs last week')
    metric(f,654,240,350,'TOTAL FOCUS','16h 45m','+2h 15m vs last week')
    metric(f,1018,240,381,'ROUTINE CONSISTENCY','76%','Building steady momentum')
    D.block(f,290,370,756,470,'Effort across the week','Time recorded from actual sessions')
    D.rect(f,326,442,683,322,'#F8FAF8',12)
    vals=[.46,.82,.62,.92,.76,.24,.38]
    for i,v in enumerate(vals):
        xx=356+i*91;hh=247*v
        D.rect(f,xx,707-hh,53,hh,P['green'] if i<5 else '#AACBBC',8)
        D.text(f,['MON','TUE','WED','THU','FRI','SAT','SUN'][i],xx,726,57,23,10,P['sub'],'Bold')
    D.block(f,1060,370,339,470,'Where the hours went','Transparent, not a magic score')
    for i,(s,v) in enumerate([('Projects',.81),('Learning',.52),('Personal',.42),('Admin',.19)]):
        yy=466+i*82;D.text(f,s,1081,yy,281,23,13,P['ink'],'Bold');D.bar(f,1081,yy+32,282,v)
def view_editor(f,m,title,state):
    D.block(f,290,246,786,594,'Product principles','Last edited today · local Markdown')
    D.label(f,'NOTE',313,323)
    D.text(f,'Design for a life, not a backlog.',314,358,724,61,27,P['ink'],'Bold')
    D.rule(f,313,428,736)
    paragraphs=['A useful system makes the next action obvious.','Capture quickly. Decide intentionally. Protect time to act.','Make projects visible without making every minute feel managed.','Agents suggest and execute; the human retains context.','Keep data portable and the offline experience complete.']
    for i,s in enumerate(paragraphs):
        D.text(f,'•  '+s,315,461+i*62,727,51,14,P['ink2'])
    D.block(f,1090,246,309,594,'Connections','Context, references, backlinks')
    for i,s in enumerate(['Linked project · Life Manager','Linked goal · Ship v1','Linked task · UI review','Related note · Architecture','Edited by · You']):
        yy=332+i*75;D.text(f,s,1111,yy,261,47,12,P['ink']);D.rule(f,1111,yy+57,262)
def view_settings(f,m,title,state):
    D.block(f,290,246,739,594,'Preferences','Explicit controls with clear consequences')
    choices=['General and language','Appearance and information density','Calendar and time-zone settings','Notifications and quiet hours','Privacy, permissions, and agents','Export, backup, and restore']
    for i,s in enumerate(choices):
        yy=322+i*79
        D.text(f,s,315,yy,581,24,14,P['ink'],'Bold')
        D.text(f,['Local-first','System / Light / Dark','Asia/Bangkok','Configurable','Review before destructive action','Daily snapshots'][i],315,yy+26,540,22,11,P['sub'])
        D.icon(f,'chevron-right',971,yy+7,20,'green',1.8)
        if i<5:D.rule(f,313,yy+64,689)
    D.block(f,1044,246,355,280,'System state','Visible and predictable')
    D.chip(f,'All data on this Mac',1067,327,206,'mint','green')
    D.text(f,'Database integrity',1067,391,210,23,12,P['sub']);D.text(f,'Healthy',1067,419,230,32,22,P['green'],'Bold')
    D.block(f,1044,540,355,300,'Account and integrations','Sign-in is never required locally')
    for i,s in enumerate(['SQLite · local','MCP · local stdio','GitHub · not connected','External calendars · optional']):D.text(f,'•  '+s,1067,619+i*47,303,30,12,P['ink2'])
def view_agent(f,m,title,state):
    D.block(f,grid.LEFT,244,688,596,'Agent activity','Every change is inspectable, attributed, and reversible where possible')
    items=['Pi • Created task from a note','Codex • Linked a project milestone','Pi • Suggested tomorrow’s schedule','CLI • Exported report as JSON','System • Verified local database']
    for i,s in enumerate(items):
        yy=324+i*87
        D.text(f,s,grid.LEFT+67,yy,602,28,14,P['ink'],'Bold')
        D.text(f,['09:18 · Applied','09:04 · Applied','08:59 · Awaiting review','Yesterday · Completed','Yesterday · Successful'][i],grid.LEFT+67,yy+30,590,19,11,P['sub'])
        D.ellipse(f,grid.LEFT+23,yy+6,29,29,P['mint'])
        D.icon(f,'bot',grid.LEFT+28,grid.icon_y(yy,28,19,14),19,'green',1.8)
        if i<4:D.rule(f,grid.LEFT+23,yy+70,672)
    bx=grid.body_columns()
    D.block(f,bx[1][0],244,bx[1][1],345,'CLI · shared source of truth','Headless commands work when UI is closed')
    D.rect(f,bx[1][0]+20,315,bx[1][1]-40,215,P['nav'],10)
    cmds=['$ life today --json','{  "priorities": 3,','   "focus_minutes": 180,','   "next": "Architecture"','}','$ life task list --status todo']
    for i,s in enumerate(cmds):D.text(f,s,bx[1][0]+36,338+i*30,304,25,11,'#D8F1E1' if i<2 else '#B0CEC2')
    D.block(f,bx[1][0],606,bx[1][1],234,'Automation rules','Local rules · bounded execution')
    for i,s in enumerate(['Daily plan · 08:30','Weekly review · Friday','Backup · every evening']):
        D.text(f,'●  '+s,bx[1][0]+24,679+i*45,322,27,12,P['ink'])
def view_life(f,m,title,state):
    cards=[('CAREER','Applications & follow-ups','2 interviews to prepare',P['peach']),('LEARNING','Study and certifications','6 focused sessions planned',P['blue']),('MONEY','Bills and money goals','1 upcoming reminder',P['yellow']),('HEALTH','Habits and movement','4 routines active',P['mint'])]
    for i,(tag,lab,sub,c) in enumerate(cards):
        x=290+(i%2)*563;y=256+(i//2)*280
        D.rect(f,x,y,541,255,P['paper'],15,stroke=P['line']);D.rect(f,x+22,y+21,54,54,c,14)
        D.text(f,tag,x+95,y+29,422,24,12,P['green'],'Bold');D.text(f,lab,x+24,y+100,482,50,22,P['ink'],'Bold')
        D.text(f,sub,x+24,y+156,482,26,13,P['sub']);D.rule(f,x+23,y+209,495)
        D.text(f,'Open workspace',x+26,y+223,435,25,12,P['green'],'Bold')
def view_wizard(f,m,title,state):
    D.rect(f,0,0,W,H,P['nav']);D.ellipse(f,97,92,48,48,P['green2'])
    D.text(f,'LIFE / MANAGER',168,97,467,56,27,'#EDF5F0','Bold')
    D.text(f,'Build a day worth living.',97,274,589,81,44,'#FFFFFF','Bold')
    D.text(f,'One quiet place for plans, commitments, and meaningful work.',97,384,549,123,21,'#BFD2CA')
    D.rect(f,750,132,588,622,P['paper'],23)
    D.label(f,'STEP '+str((state%6)+1)+' OF 6',790,174)
    D.text(f,title,790,207,503,85,27,P['ink'],'Bold')
    for i,s in enumerate(MODULE_CONTENT['Onboarding']):
        yy=325+i*68;D.rect(f,791,yy,499,51,'#F4F7F4',10)
        D.ellipse(f,810,yy+14,21,21,P['mint']);D.text(f,s,852,yy+13,412,26,13,P['ink'])
    D.rect(f,1069,684,217,45,P['green'],9);D.text(f,'Continue',1118,697,158,25,14,'#FFFFFF','Bold')
    D.text(f,'Offline by design · No account required',791,785,493,23,12,P['sub'])
def view_system(f,m,title,state):
    D.block(f,290,246,1109,594,'Design system · foundations','Type, surfaces, spacing, states, and reusable components')
    D.text(f,'Aa',328,333,188,110,82,P['ink'],'Bold')
    D.text(f,'Helvetica Neue / UI family',538,359,714,32,17,P['ink'],'Bold')
    D.text(f,'32 / 24 / 17 / 14 / 12 / 10 px',538,405,611,25,13,P['sub'])
    swatches=[('Ink',P['nav']),('Forest',P['green']),('Clay',P['rust']),('Mist',P['mint']),('Paper',P['paper']),('Border',P['line'])]
    for i,(n,c) in enumerate(swatches):
        x=326+i*174;D.rect(f,x,498,152,112,c,12,stroke=P['line']);D.text(f,n,x+3,629,143,24,13,P['ink'],'Bold');D.text(f,c,x+3,657,146,23,11,P['sub'])
    D.chip(f,'Primary action',329,739,135,'green','#FFFFFF');D.chip(f,'Needs attention',479,739,152,'peach','rust');D.chip(f,'Completed',645,739,122,'mint','green')
    D.rect(f,790,733,245,39,P['paper'],8,stroke=P['line']);D.text(f,'A keyboard-first input',807,744,223,20,11,P['sub'])
def view_state(f,m,title,state):
    view_list(f,'States',title,state)
    overlay=D.rect(f,252,74,1188,792,P['nav']);D.j['nodes'][str(overlay)]['opacity']=0.43
    D.rect(f,603,280,516,325,P['paper'],17)
    kind=title.lower()
    urgent=any(x in kind for x in ('error','conflict','failed','locked','destructive','permission'))
    D.ellipse(f,828,317,63,63,P['peach'] if urgent else P['mint'])
    D.icon(f,'triangle-alert' if urgent else 'check',846,331,32,P['rust'] if urgent else P['green'],2.2)
    D.text(f,title,647,410,428,68,23,P['ink'],'Bold',align='center')
    D.text(f,'Your information remains local. Review the next step before continuing.',648,486,425,60,13,P['sub'],align='center')
    D.rect(f,772,551,183,37,P['green'],8);D.text(f,'Review options',807,561,139,22,12,'#FFFFFF','Bold')

MODULES=[
('01 Foundation','System',[
 ('System overview','system','Primary visual language, grid and typographic hierarchy'),
 ('App shell · spacious','dashboard','Sidebar, top bar, content cards and persistent actions'),
 ('App shell · compact','list','Denser layout and priority-first scanning'),
 ('Component states','system','Buttons, inputs, badges, focus, error and disabled'),
 ('Color and type tokens','system','Semantic color system for light and dark themes'),
 ('Global command palette','agent','Keyboard-driven command and search entry'),
 ('Quick-add overlay','state','Natural language capture with review before save'),
 ('Responsive small window','list','Prioritized navigation with reduced width'),
 ('Dark appearance','dashboard','Contrast and calm dark theme sample'),
 ('Accessibility inspection','system','Visible focus, keyboard order and type scaling'),
 ('Accessibility audit','system','Focus rings, contrast, order and reduced motion')]),
('02 First-run setup','Onboarding',[
 ('Welcome to Life Manager','wizard','Local-first onboarding and agency'),
 ('Choose your life areas','wizard','Customizable default areas'),
 ('Select initial priorities','wizard','Explicit focus commitments'),
 ('Choose your time zone','wizard','IANA time zone and display formats'),
 ('Configure availability','wizard','Working, personal and protected hours'),
 ('Choose reminders','wizard','Notification permission and quiet hours'),
 ('Create first task','wizard','A useful first action before exiting onboarding'),
 ('Ready to plan','wizard','Review configuration and launch dashboard')]),
('03 Today and Inbox','Today',[
 ('Today · default','dashboard','Top priorities, capacity and agenda'),
 ('Today · busy day','dashboard','Over-capacity warning and next action'),
 ('Today · nothing due','state','Truly empty day without false urgency'),
 ('Next seven days','calendar','Upcoming commitments and deadlines'),
 ('Universal inbox','list','All captured items in one place'),
 ('Inbox · capture workflow','details','Triaging a newly captured idea'),
 ('Inbox · processed','state','Zero-inbox calm completion state'),
 ('Create task from text','state','Parse and confirm natural-language inputs'),
 ('Notification center','list','Reminder events and outstanding actions'),
 ('Evening wind-down','dashboard','Review achievements without pressure'),
 ('Today · delayed work','list','Overdue tasks do not silently shift'),
 ('Tomorrow preview','dashboard','Plan tomorrow against actual capacity')]),
('04 Task management','Tasks',[
 ('Tasks · all','list','Searchable task table with status filters'),
 ('Tasks · Today filter','list','Due or planned for today'),
 ('Tasks · Priority filter','list','P1 and P2 action queue'),
 ('Task detail · open','details','Full details, metadata and checklist'),
 ('Task detail · subtasks','details','Deep nesting and progress'),
 ('Task detail · activity','details','History, provenance and undo'),
 ('Entity activity timeline','details','The full recorded history of one record'),
 ('Task dependencies','board','Blocked and blocking relationship flow'),
 ('Recurring task editor','state','Recurrence rules, exceptions and time zones'),
 ('Recurrence exception editor','details','Skip or move one occurrence without changing the series'),
 ('Task templates','board','Reusable task bundles'),
 ('Bulk-edit selection','state','Atomic multi-item changes and review'),
 ('Task archive','list','Completed and cancelled task history'),
 ('Task restore','state','Restore from recoverable deletion'),
 ('Overdue task decision','state','Explicit reschedule, snooze or keep overdue'),
 ('Overdue decision sheet','state','Choose what happens to late work'),
 ('Smart saved views','list','Persistent task filters and search'),
 ('Saved view runner','list','Execute a stored filter and read the matching rows'),
 ('Task scheduling detail','details','Start, planned, due and completed fields')]),
('05 Projects and milestones','Projects',[
 ('Project portfolio','life','Overview of active work and commitments'),
 ('Projects · all','list','All projects across life areas'),
 ('Project · summary','dashboard','Project scope, next steps and progress'),
 ('Project · Kanban','board','Custom status sections and task cards'),
 ('Project · timeline','planning','Milestones and dependencies against time'),
 ('Project · milestones','list','First-class milestone tracking'),
 ('Project · blockers','board','Cross-project dependency visibility'),
 ('Blocker resolution','board','See exactly what blocks each task'),
 ('Project health report','analytics','Blocked work, progress and schedule risk'),
 ('Project templates','board','Reusable workflows and sections'),
 ('Project notes','editor','Context linked to project work'),
 ('Paused and archived projects','list','Inactive work remains searchable'),
 ('Project completion review','state','Archive, record outcome or reopen')]),
('06 Goals and direction','Goals',[
 ('Goals overview','analytics','Annual and quarterly direction'),
 ('Goal detail','details','Outcome, motivation, progress and linked work'),
 ('Quarterly priorities','board','Intentionally limited active commitments'),
 ('Annual direction','planning','Long-term milestones and themes'),
 ('OKR objective details','details','Optional measurable key results'),
 ('Goal progress history','analytics','Progress over time and evidence'),
 ('Goal monthly review','details','Assess relevance and choose the next action'),
 ('Monthly goal review','details','Assess relevance and next action'),
 ('Goal completed','state','Achievement record and reflection'),
 ('Goal paused or abandoned','state','Preserve history and explanation'),
 ('Active-goal limit','state','Warn against unrealistic simultaneous focus')]),
('07 Calendar','Calendar',[
 ('Calendar · day','calendar','Single-day scheduling and personal commitments'),
 ('Calendar · three days','calendar','Focused short-term capacity'),
 ('Calendar · week','calendar','Time-grid view with time blocks'),
 ('Calendar · month','calendar','Long-horizon calendar scan'),
 ('Calendar · agenda','calendar','Accessible chronological list'),
 ('Create an event','state','Event type, location and time zone'),
 ('Edit an event','details','Metadata and date/time changes'),
 ('Recurring event','state','Edit one, future or all occurrences'),
 ('All-day commitments','calendar','All-day events separate from timed work'),
 ('Calendar conflict','state','Overlaps and possible alternatives'),
 ('Time-zone travel','calendar','Keep wall-clock and instant semantics clear'),
 ('External calendar overlay','calendar','Read-only imported and subscribed events'),
 ('ICS import preview','state','Inspect duplicates and time zones before applying'),
 ('Calendar · no events','state','Empty time is free time, not an error')]),
('08 Scheduling and planning','Planning',[
 ('Daily planning session','planning','Set realistic top-three priorities'),
 ('Daily planning recorder','planning','Record the three priorities of the day'),
 ('Available capacity','analytics','Protect available hours and breaks'),
 ('Weekly time budget','planning','Allocate planned effort across areas'),
 ('Protected time editor','settings','Reserve recurring time that must not be moved'),
 ('Time-block schedule','calendar','Schedule tasks without cloning entities'),
 ('Automatic schedule proposal','planning','Deterministic candidate time slots'),
 ('Proposal approval','state','Approve a suggested change set'),
 ('Conflict resolution','state','Warn and resolve overlaps'),
 ('Protected time','calendar','Do not move locked time without review'),
 ('Unfinished work carryover','list','Explicitly choose next treatment'),
 ('Travel and buffer time','planning','Respect transitions between blocks'),
 ('Time budget overrun','state','Excess planned hours and recovery'),
 ('Next-week preparation','planning','Balanced advance planning')]),
('09 Deep work and time','Focus',[
 ('Focus · choose task','details','Enter a distraction-reduced session'),
 ('Focus · active','focus','Large readable timer and purpose'),
 ('Focus · paused','focus','Resume without double counting'),
 ('Focus · short break','focus','Recovery interval, not another task'),
 ('Focus · completed','analytics','Actual effort recorded on the task'),
 ('Focus · Pomodoro','focus','Four configurable cycles'),
 ('Focus presets and manual entry','state','Timed cycles plus honest manual records'),
 ('Focus · stopwatch','focus','Open-ended elapsed tracking'),
 ('Idle activity review','state','Classify or exclude inactive time'),
 ('Interrupted session','state','Recover after a restart or crash'),
 ('Manual time entry','state','Correct a work session with audit trail'),
 ('Time log history','list','Exact intervals with overlaps handled'),
 ('Focus trends','analytics','Planned versus actual concentration')]),
('10 Habits and routines','Habits',[
 ('Habits · weekly','habits','Visual week-by-week completion'),
 ('Habits · daily checklist','list','Current applicable actions'),
 ('Habit detail','details','Frequency, unit, streak and statistics'),
 ('Create a habit','state','Boolean, count and duration targets'),
 ('Routine builder','board','Order steps with optional timers'),
 ('Morning routine','habits','Guided sequence for morning'),
 ('Evening routine','habits','Guided sequence for wind-down'),
 ('Habit skip decision','state','Intentional rest without false failure'),
 ('Consistency analysis','analytics','Weekly patterns and trends'),
 ('Habit reminders','settings','Scheduling window and quiet hours'),
 ('Routine completion','state','Celebrate consistency without addictive scoring')]),
('11 Notes and discovery','Notes',[
 ('Notes library','list','Searchable local knowledge entries'),
 ('Note editor','editor','Markdown and readable typography'),
 ('Note links and backlinks','editor','Cross-reference projects, goals and tasks'),
 ('Attached files','details','Local references and file integrity'),
 ('Global search','list','Search all entity types with FTS5'),
 ('Filtered search results','list','Multi-field and saved filters'),
 ('Search with no results','state','Clear empty search action'),
 ('Quick capture palette','state','Capture without navigation overhead'),
 ('Capture parser','state','Read a date, an urgency and a tag from plain text'),
 ('Inbox processing workflow','details','Clarify a note into an action'),
 ('Saved search detail','list','A reusable personal lens'),
 ('Saved search manager','list','Create, run and remove reusable searches')]),
('12 Reviews and insights','Reviews',[
 ('Morning planning review','dashboard','Choose intentional priorities'),
 ('Evening reflection','editor','Complete unfinished thoughts'),
 ('Weekly review workflow','details','Wins, blockers and next decisions'),
 ('Monthly review workflow','details','Goal direction and workload'),
 ('Productivity analytics','analytics','Explicit interpretable statistics'),
 ('Plan versus actual','analytics','Capacity accuracy and actual effort'),
 ('Goal achievement report','analytics','Meaningful progress summaries'),
 ('Goal active limit','state','Warn against unrealistic simultaneous focus'),
 ('Area time distribution','analytics','Where attention actually went'),
 ('Journal archive','list','Private reflections over time'),
 ('Journal archive browser','list','Read the private record by period'),
 ('Review export','state','Portable report and data selection')]),
('13 Personal life areas','Life',[
 ('Personal overview','life','Career, learning, money and wellbeing'),
 ('Career pipeline','board','Applications and interview stages'),
 ('Upcoming interviews','calendar','Preparation blocks and commitments'),
 ('Interview scheduler','state','Schedule an interview and note the preparation'),
 ('Career follow-up detail','details','Named next action with reminder'),
 ('Learning dashboard','life','Courses, study sessions and projects'),
 ('Course progress','details','Lessons and evidence of completion'),
 ('Certification deadline','calendar','Milestones and revision time'),
 ('Certification tracker','state','Track an exam date, a deadline and study time'),
 ('Financial reminders','list','Bills and budgets without a banking ledger'),
 ('Savings goal','analytics','A simple manually entered goal'),
 ('Savings goal tracker','analytics','A manually entered target and what is saved'),
 ('Health routine planner','habits','User-defined wellness habits'),
 ('Sleep and rest intentions','habits','Respect a healthy schedule'),
 ('Rest intention planner','state','Record when rest is meant to happen'),
 ('Personal administration','list','Renewals, paperwork and errands'),
 ('Administration tracker','list','Renewals, paperwork and errands with dates'),
 ('Life area detail','dashboard','Contextual status and next actions'),
 ('Area configuration','settings','Rename, recolor, hide and archive')]),
('14 Agents and automations','Agents',[
 ('Agent command center','agent','Observe agents across the shared domain layer'),
 ('CLI quick reference','agent','Human output and structured JSON modes'),
 ('MCP local server','agent','Document stdio setup and tool capabilities'),
 ('Agent-generated task','details','Clear attribution and change history'),
 ('Agent scheduling proposal','planning','Human review for calendar mutations'),
 ('Agent change approval','state','Review changes before applying'),
 ('Agent permission matrix','settings','Allow or deny action patterns per actor'),
 ('Agent activity ledger','agent','Auditable successful and failed mutations'),
 ('Agent permissions','settings','Same-user local scopes and safeguards'),
 ('Automation gallery','agent','Recurring event and task workflows'),
 ('Rule editor','details','Triggers, conditions and actions'),
 ('Automation execution history','list','Deterministic event outcomes'),
 ('Automation scheduler','settings','Daily and weekly triggers with due state'),
 ('Failed agent execution','state','Recoverable error with structured code'),
 ('CLI structured error','state','Stable exit code and machine-readable details'),
 ('Agent undo workflow','state','Reversible changes and recovery boundaries')]),
('15 Settings and integrations','Settings',[
 ('General preferences','settings','Formats, defaults and application behavior'),
 ('Appearance and density','system','System theme, typography and layouts'),
 ('Keyboard shortcuts','settings','Conflict-aware shortcut customization'),
 ('Shortcut map editor','settings','Bind a key combination and detect conflicts'),
 ('Notification and quiet hours','settings','Windows and OS permissions'),
 ('Apple Calendar connection','settings','Optional calendar read access'),
 ('Google Calendar connection','settings','Optional OAuth integration'),
 ('Calendar source management','settings','Visibility, priority and direction'),
 ('Calendar source manager','settings','Local and imported sources with read-only state'),
 ('GitHub integration','settings','Repository and work item linking'),
 ('GitHub connection boundary','settings','State plainly that no repository is connected'),
 ('Local projects and branches','list','Link repository paths to tasks'),
 ('ICS and CSV imports','state','Preview and validate external data'),
 ('CSV import classification','state','Classify valid, duplicate and invalid CSV rows'),
 ('CSV import preview','state','Classify valid, duplicate and invalid CSV rows'),
 ('JSON data export','state','Readable portable backup format'),
 ('Backup history','list','Consistent snapshots and retention'),
 ('Restore database','state','Preview integrity and explicit replacement'),
 ('Security and local permissions','settings','Protect sensitive operations'),
 ('Remote MCP clients','settings','Issue and revoke remote agent tokens'),
 ('Keyboard shortcut map','settings','Conflict-aware shortcut customization'),
 ('Diagnostics and storage','settings','Database health and sizes'),
 ('Database diagnostics report','settings','Integrity, schema, size and row counts'),
 ('About and version','settings','Build identifiers and local paths')]),
('16 Edge states and recovery','States',[
 ('First run · empty database','state','Provide a safe starting point'),
 ('Today loading skeleton','state','No misleading interactive placeholders'),
 ('Database temporarily locked','state','Preserve edits, retry without spin'),
 ('Migration failed','state','Do not rewrite data or discard the backup'),
 ('Backup restore confirmation','state','Destructive changes require preview'),
 ('Delete multiple projects','state','Display scope and undo limitations'),
 ('Scheduling conflict warning','state','Explain overlapping commitments'),
 ('Agent proposal rejected','state','Log rejection and preserve prior data'),
 ('Offline integration state','state','Core remains usable without network'),
 ('External provider boundary','state','State plainly that provider sync is not connected'),
 ('Import partial failure','state','Specific rows failed, valid rows preserved'),
 ('Permission denied','state','Explain the required local permission'),
 ('Long content stress','list','Handle large titles and notes gracefully'),
 ('High-volume backlog','list','Preserve rendering performance and navigation'),
 ('Time-zone boundary','calendar','DST transitions and local-midnight rules')]),
]
VIEW={
'dashboard':view_dashboard,'list':view_list,'board':view_board,'details':view_details,
'calendar':view_calendar,'planning':view_planning,'focus':view_focus,
'habits':view_habits,'analytics':view_analytics,'editor':view_editor,
'settings':view_settings,'agent':view_agent,'life':view_life,
'wizard':view_wizard,'system':view_system,'state':view_state,
}
def build():
    index=[];seq=0
    for page_name,domain,screens in MODULES:
        page=D.page(page_name)
        for i,(title,view,intent) in enumerate(screens):
            seq+=1
            name=f'{seq:03d} · {title}'
            fid=D.frame(page,name,(i%3)*1536,(i//3)*1016)
            if view=='wizard':
                from visual_variants import render_special
                if not render_special(D,fid,title,view,domain,i,P,MODULE_CONTENT): VIEW[view](fid,domain,title,i)
            else:
                active=domain if domain in NAV else ('Projects' if domain=='Planning' else 'Today' if domain in ('Life','System','States') else 'Settings' if domain=='Settings' else 'Today')
                # The dark board is a full appearance sample, so it brings its
                # own navigation rather than the standard shell.
                if 'dark appearance' not in title.lower():
                    shell(fid,title,intent,active,tag=page_name.upper(),badge='DESIGN SPECIMEN' if domain=='System' else None)
                from visual_variants import render_special
                if not render_special(D,fid,title,view,domain,i,P,MODULE_CONTENT): VIEW[view](fid,domain,title,i)
                footer(fid,'DESIGN SYSTEM  /  ' + page_name.upper() + '   ·   Illustrative data only')
            index.append({'board_id':seq,'node_id':fid,'page':page_name,'title':title,'view':view,'scenario':intent,'preview':f'previews/{page_name.lower().replace(" ","-")}/{name}.png'})
    D.save(OUT/'life-manager-master.json')
    with open(OUT/'screen-inventory.json','w') as f:json.dump(index,f,indent=2,ensure_ascii=False)
    with open(OUT/'screen-inventory.csv','w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(index[0]));w.writeheader();w.writerows(index)
    print('BUILT',len(index),'editable boards on',len(MODULES),'pages; nodes',D.n-1,'source',OUT/'life-manager-master.json')
    return index
if __name__=='__main__': build()
