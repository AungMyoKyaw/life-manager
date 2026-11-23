"""Screen-specific visual treatments. No application or runtime code."""
import calendar, math

def wizard(D,f,title,index,P):
    D.rect(f,0,0,1440,900,P['nav'])
    D.ellipse(f,95,96,49,49,P['green2']);D.text(f,'LIFE / MANAGER',165,103,433,45,25,'#FFFFFF','Bold')
    D.text(f,'A little more clarity.\nA lot more agency.',96,279,601,161,42,'#F5FAF5','Bold')
    D.text(f,'A quiet workspace for your intentions,\ncommitments and meaningful work.',97,464,578,81,19,'#B9D2C7')
    for i in range(8):D.ellipse(f,97+i*27,814,11,11,P['green2'] if i<=index else '#436057')
    D.rect(f,747,104,598,677,P['paper'],20)
    D.text(f,f'GET STARTED    /    {index+1} OF 8',789,142,510,26,11,P['green'],'Bold')
    D.text(f,title,789,185,516,84,31,P['ink'],'Bold')
    D.text(f,'Your choices stay on this Mac and can be changed later.',790,264,513,33,13,P['sub'])
    def item(label,y,detail='',selected=True):
        D.rect(f,788,y,516,64,'#F2F6F3',10,stroke=P['line']);D.rect(f,805,y+19,24,24,P['green'] if selected else P['paper'],6,stroke=P['green']);
        if selected:D.text(f,'✓',810,y+21,18,20,13,'#FFFFFF','Bold')
        D.text(f,label,846,y+11,440,26,14,P['ink'],'Bold')
        if detail:D.text(f,detail,846,y+35,436,20,11,P['sub'])
    if index==0:
        for i,(s,n) in enumerate([('One place to decide','Capture and clarify every commitment'),('A realistic calendar','Protect capacity, not just deadlines'),('Your agents, your rules','The same local data for the app, CLI and MCP')]):item(s,327+i*84,n)
    elif index==1:
        for i,(s,n) in enumerate([('Projects & work','Build and ship meaningful things'),('Learning & growth','Courses, skills and practice'),('Health & routines','Gentle consistency'),('Personal & finance','Everyday responsibilities')]):item(s,310+i*79,n)
    elif index==2:
        for i,(s,n) in enumerate([('Choose three priorities','Keep the daily plan realistic'),('Protect deep work','Reserve interruption-free time'),('Review each evening','Close the loop, not your self-worth'),('Weekly planning','Adjust commitments before they pile up')]):item(s,310+i*79,n,i!=3)
    elif index==3:
        D.label(f,'YOUR TIME ZONE',789,312)
        D.rect(f,788,343,516,63,'#F2F6F3',10,stroke=P['line']);D.text(f,'Asia/Bangkok   (UTC+07:00)',809,362,466,33,17,P['ink'],'Bold')
        D.label(f,'DATE FORMAT',789,448)
        for i,(a,b) in enumerate([('9 November 2026','Day / Month / Year'),('November 9, 2026','Month / Day / Year')]):item(a,477+i*86,b,i==0)
    elif index==4:
        D.label(f,'WEEKLY AVAILABILITY',790,307)
        for i,s in enumerate(['MON','TUE','WED','THU','FRI','SAT','SUN']):
            y=345+i*48;D.text(f,s,795,y+5,71,27,12,P['ink'],'Bold')
            D.rect(f,874,y,431,38,'#F2F6F3',8)
            D.text(f,'08:30  —  18:00' if i<5 else 'Flexible / rest',893,y+9,344,25,13,P['ink2'])
    elif index==5:
        for i,(a,b) in enumerate([('Task reminders','Before an important deadline'),('Daily planning reminder','A gentle morning prompt'),('Quiet hours','No alerts between 22:00–08:00'),('System notifications','Permission can be granted later')]):item(a,312+i*78,b,i!=3)
    elif index==6:
        D.label(f,'WHAT IS THE FIRST THING TO DO?',790,326)
        D.rect(f,788,358,516,67,'#F2F6F3',10,stroke=P['line']);D.text(f,'Plan my first productive day',808,378,455,31,18,P['ink'],'Bold')
        for i,(a,b) in enumerate([('When','Tomorrow · 09:00'),('Priority','P1 · High'),('Area','Personal'),('Duration','30 minutes')]):
            y=454+i*50;D.text(f,a,794,y,160,26,12,P['sub']);D.text(f,b,956,y,339,26,14,P['ink'],'Bold');D.rect(f,789,y+37,516,1,P['line'])
    elif index==7:
        for i,(a,b) in enumerate([('4 life areas','Selected and editable'),('Asia/Bangkok','Your local time zone'),('Weekday schedule','08:30 — 18:00'),('One starting action','Ready for your dashboard')]):item(a,307+i*78,b)
    D.rect(f,1084,715,221,44,P['green'],10);D.text(f,'Open my day   →' if index==7 else 'Continue    →',1121,727,177,27,14,'#FFFFFF','Bold')
    if index>0:D.text(f,'←   Back',789,727,147,27,13,P['sub'],'Bold')
    return True

def calendar_view(D,f,title,P):
    t=title.lower()
    mode='week'
    if 'three days' in t: mode='three'
    elif 'month' in t:mode='month'
    elif 'agenda' in t:mode='agenda'
    elif 'day' in t and 'all-day' not in t:mode='day'
    D.chip(f,'‹',290,239,40,'paper','ink');D.chip(f,'›',338,239,40,'paper','ink')
    D.text(f,('Monday, November 9, 2026' if mode=='day' else 'November 2026' if mode=='month' else 'November 9–15, 2026'),397,238,577,34,19,P['ink'],'Bold')
    for i,(s,c) in enumerate([('Day',mode=='day'),('3 days',mode=='three'),('Week',mode=='week'),('Month',mode=='month'),('Agenda',mode=='agenda')]):D.chip(f,s,1043+i*73,242,68,'mint' if c else 'bg','green' if c else 'sub')
    if mode=='month':
        D.rect(f,290,293,1112,547,P['paper'],12,stroke=P['line'])
        for i,d in enumerate(['MON','TUE','WED','THU','FRI','SAT','SUN']):D.text(f,d,309+i*157,310,121,26,11,P['sub'],'Bold')
        for row in range(5):
            for col in range(7):
                xx=293+col*158;yy=345+row*98
                D.rect(f,xx,yy,157,97,'#FCFDFC' if col<5 else '#F6F8F7',stroke=P['line'])
                day=row*7+col-5
                if 1<=day<=30:
                    D.text(f,str(day),xx+12,yy+9,37,25,12,P['green'] if day==9 else P['sub'],'Bold')
                    if day in (3,5,9,11,13,17,20,24,27):D.rect(f,xx+11,yy+44,135,27,P['mint'] if day%2 else P['peach'],5);D.text(f,'Focus / meeting' if day%2 else 'Review',xx+17,yy+51,125,18,10,P['ink'])
    elif mode=='agenda':
        D.rect(f,290,292,1112,547,P['paper'],12,stroke=P['line'])
        events=[('MON 09','08:30','Daily plan','20 min'),('MON 09','09:00','Architecture deep work','90 min'),('MON 09','15:30','Design review','45 min'),('TUE 10','10:00','Learning session','60 min'),('WED 11','13:00','Planning and catch-up','30 min')]
        for i,(day,time,lab,dur) in enumerate(events):
            y=331+i*97;D.text(f,day,321,y+8,133,28,12,P['green'],'Bold');D.text(f,time,480,y+8,126,30,16,P['ink'],'Bold');D.rect(f,635,y+3,5,50,P['green'] if i%2 else P['rust'],3);D.text(f,lab,660,y,482,31,17,P['ink'],'Bold');D.text(f,dur,1228,y+13,134,24,12,P['sub']);D.rule(f,315,y+71,1048)
    else:
        cols=1 if mode=='day' else 3 if mode=='three' else 7
        D.rect(f,290,293,1112,548,P['paper'],12,stroke=P['line'])
        avail=1022
        cell=avail/cols
        for i in range(cols):
            x=358+i*cell
            D.text(f,['MON 09','TUE 10','WED 11','THU 12','FRI 13','SAT 14','SUN 15'][i],x+11,313,cell-13,24,12,P['green'] if i==0 else P['sub'],'Bold')
            if i:D.rect(f,x,345,1,480,P['line'])
        if 'all-day' in t:
            D.rect(f,362,349,1024,41,P['yellow'],6);D.text(f,'ALL DAY  •  Personal commitment · No time conflict',375,358,976,27,12,P['ink'],'Bold')
        for row in range(8):
            y=369+row*56;D.text(f,f'{row+9:02d}:00',309,y,47,22,10,P['faint']);D.rect(f,357,y+32,1023,1,P['line'])
        ev=[(0,0,1,'Daily planning'),(0,1,2,'Deep work · Architecture'),(1,3,1,'Design review'),(2,2,2,'Learning session'),(3,5,1,'Project sync'),(4,4,1,'Weekly review')]
        for day,start,length,lab in ev:
            if day>=cols:continue
            x=368+day*cell;y=377+start*56
            w=cell-18;h=length*56-6
            D.rect(f,x,y,w,h,P['mint'] if day%2==0 else P['peach'],7)
            D.text(f,lab,x+11,y+8,w-18,38,12,P['green'] if day%2==0 else P['rust'],'Bold')
            if h>60:D.text(f,'Protected · 90 minutes',x+11,y+49,w-18,22,10,P['ink2'])
        if 'protected' in t or 'boundary' in t:
            D.rect(f,361,739,1022,31,P['yellow'],6);D.text(f,'LOCKED / TIME ZONE SENSITIVE — Review before rescheduling',373,747,1000,22,11,P['rust'],'Bold')
        if 'external' in t:D.chip(f,'Apple Calendar · read only',1038,796,259,'blue','ink')
        if 'time-zone' in t:D.chip(f,'Bangkok +07 / Singapore +08',1005,796,296,'blue','ink')
    return True

def state_view(D,f,title,P):
    t=title.lower()
    no_overlay=any(k in t for k in ['nothing due','no events','no results','processed','empty database','loading skeleton','routine completion','goal completed','offline integration state'])
    D.rect(f,290,237,1112,603,P['paper'],15,stroke=P['line'])
    if 'loading skeleton' in t:
        D.text(f,'Loading your day',339,291,894,33,23,P['ink'],'Bold')
        for i in range(3):
            D.rect(f,338+i*343,369,319,112,'#EEF2EF',12)
            D.rect(f,361+i*343,391,154,14,'#DEE6DF',5)
            D.rect(f,361+i*343,426,216,26,'#E4EAE5',5)
        for i in range(5):
            D.rect(f,338,511+i*54,1005,36,'#F0F4F1',8)
            D.rect(f,360,519+i*54,354+i*71,16,'#DFE8E2',5)
        D.text(f,'Non-interactive skeletons. Never imply content before it has loaded.',341,811,977,18,11,P['sub'])
        return True
    if no_overlay:
        success=any(k in t for k in ['processed','completion','completed'])
        D.ellipse(f,747,339,174,174,P['mint'] if success else P['blue'])
        D.text(f,'✓' if success else '○',804,389,110,91,72,P['green'],'Bold')
        lead={'Today · nothing due':'A genuinely open day.','Inbox · processed':'You have cleared your inbox.','Search with no results':'No matching results.','Calendar · no events':'Room to make your own plans.','First run · empty database':'A blank slate, by design.','Routine completion':'One more routine, thoughtfully done.','Goal completed':'You reached the outcome.','Offline integration state':'Offline, and still fully usable.'}.get(title,title)
        D.text(f,lead,431,546,902,58,31,P['ink'],'Bold',align='center')
        D.text(f,'Capture something meaningful or enjoy the space. Nothing is automatically scheduled.',480,622,812,62,14,P['sub'],align='center')
        D.chip(f,'Open local calendar' if 'offline' in t else 'Create a task',739,719,209 if 'offline' in t else 165,'green','#FFFFFF')
        return True
    overlay=D.rect(f,252,74,1188,792,P['nav']);D.j['nodes'][str(overlay)]['opacity']=0.34
    D.rect(f,604,197,570,550,P['paper'],20)
    urgent=any(k in t for k in ['failed','failure','conflict','locked','denied','error','delete','destructive','overrun','migration'])
    is_form=any(k in t for k in ['create','quick','editor','manual time','capture','import preview','import','ics','recurring','data export','review export','search filter'])
    is_review=any(k in t for k in ['approval','proposal','bulk-edit','restore','rejected','undo','delete','completion review','overdue','rescheduling','reschedule'])
    if urgent:D.ellipse(f,630,227,42,42,P['peach']);D.text(f,'!',644,230,25,26,20,P['rust'],'Bold')
    else:D.ellipse(f,630,227,42,42,P['mint']);D.text(f,'+' if is_form else '✓',640,232,29,26,18,P['green'],'Bold')
    D.text(f,title,688,228,454,52,24,P['ink'],'Bold')
    D.rule(f,630,294,514)
    if is_form:
        fields=[]
        if 'event' in t:fields=[('EVENT NAME','Project review'),('START / END','Nov 12 · 15:00 — 15:45'),('TIME ZONE','Asia/Bangkok (UTC+07:00)'),('REPEAT','Does not repeat')]
        elif 'habit' in t:fields=[('HABIT NAME','Morning meditation'),('FREQUENCY','Monday–Sunday'),('TARGET','15 minutes'),('REMINDER','Between 07:00 and 09:00')]
        elif 'time' in t:fields=[('TASK','Architecture specification'),('STARTED','Today · 09:00'),('FINISHED','Today · 09:45'),('REASON','Correct missed focus session')]
        elif 'recurr' in t:fields=[('REPEAT EVERY','1 week'),('ON','Monday, Wednesday and Friday'),('END','Never'),('MISSED OCCURRENCES','Skip missed repetitions')]
        elif 'import' in t or 'export' in t:fields=[('SOURCE / FORMAT','Calendar ICS / Versioned JSON'),('INCLUDED ITEMS','48 records; 0 duplicates'),('TIME ZONES','Preserve source time zones'),('VALIDATION','Schema and conflicts checked')]
        else:fields=[('TASK / CAPTURE','Prepare application update'),('AREA','Career'),('WHEN','Tomorrow · 10:30'),('PRIORITY / EFFORT','P2 · 30 minutes')]
        for i,(lab,val) in enumerate(fields):
            y=316+i*83;D.label(f,lab,632,y,484);D.rect(f,631,y+23,514,46,'#F5F8F5',8,stroke=P['line']);D.text(f,val,648,y+35,486,25,13,P['ink'],'Bold')
        D.text(f,'Changes are validated locally before they are saved.',633,679,488,23,11,P['sub'])
        D.chip(f,'Cancel',793,707,91,'bg','sub');D.chip(f,'Save changes',894,707,247,'green','#FFFFFF')
    elif is_review:
        info=[('WHAT CHANGES',title),('AFFECTED RECORDS','3 selected items'),('EXISTING COMMITMENTS','Protected events are unchanged'),('RECOVERY','History and undo available when supported')]
        for i,(label,val) in enumerate(info):
            y=326+i*81;D.label(f,label,634,y,490);D.text(f,val,633,y+22,499,35,14,P['ink'],'Bold');D.rule(f,632,y+62,512)
        D.text(f,'Review the complete change set; nothing applies without this action.',632,667,502,33,11,P['sub'])
        D.chip(f,'Keep current',781,706,134,'bg','sub');D.chip(f,'Confirm deletion' if 'delete' in t else 'Apply reviewed changes',927,706,215,'rust' if 'delete' in t else 'green','#FFFFFF')
    elif urgent:
        message={
           'database temporarily locked':'The database is busy with another local writer. Your pending input is retained.',
           'migration failed':'The migration was stopped. The existing database and pre-migration snapshot were preserved.',
           'scheduling conflict warning':'The proposed focus block overlaps an existing locked calendar event.',
           'calendar conflict':'Two scheduled events occupy the same time. Choose which should move.',
           'permission denied':'This operation requires a specific macOS permission. Existing work remains available.',
           'failed agent execution':'The agent operation failed. No partial schedule edits were applied.',
           'cli structured error':'The command returned a stable error code and a retryable classification.'}
        msg=message.get(t,'This operation needs a decision. No previous data has been discarded.')
        D.text(f,msg,636,329,508,100,17,P['ink'],'Bold')
        D.rect(f,633,454,508,136,'#F8F3ED',10)
        D.text(f,'SAFE RECOVERY PATH',652,470,462,22,11,P['rust'],'Bold')
        D.text(f,'1. Inspect the affected item or operation.\n2. Retry only when the conflict is resolved.\n3. Review the activity log before continuing.',652,511,455,70,12,P['ink2'])
        D.chip(f,'Open diagnostics',638,690,182,'bg','sub');D.chip(f,'Review and retry',890,690,250,'green','#FFFFFF')
    else:
        D.text(f,'This action has a recorded state and a clear recovery path.',636,326,506,87,17,P['ink'],'Bold')
        for i,s in enumerate(['Current work remains intact','Changes are attributed to their actor','History is available for inspection']):
            y=443+i*57;D.ellipse(f,642,y+5,22,22,P['mint']);D.text(f,'✓',647,y+7,18,20,12,P['green'],'Bold');D.text(f,s,678,y+5,431,24,13,P['ink'])
        D.chip(f,'Back',786,690,99,'bg','sub');D.chip(f,'Continue',894,690,250,'green','#FFFFFF')
    return True

def focus_view(D,f,title,P):
    t=title.lower();paused='paused' in t;breaking='break' in t;pom='pomodoro' in t;st='stopwatch' in t
    D.rect(f,290,239,1112,601,P['mint'] if not breaking else P['blue'],18)
    D.text(f,'A PROTECTED MOMENT' if not breaking else 'REST IS PART OF FOCUS',330,272,704,30,12,P['green'],'Bold')
    D.text(f,'A quiet break.' if breaking else 'Architecture specification',329,329,901,56,30,P['ink'],'Bold')
    D.ellipse(f,480,402,363,363,'#B9D8CB');D.ellipse(f,504,426,315,315,'#F2F8F3')
    elapsed='05:00' if breaking else '25:00' if pom else '01:27:12' if st else '38:24'
    D.text(f,elapsed,558,513,284,91,63,P['ink'],'Bold')
    D.text(f,'PAUSED' if paused else 'BREAK · 01 / 04' if breaking else 'POMODORO · 02 / 04' if pom else 'STOPWATCH' if st else 'SESSION · 02 / 04',576,616,259,29,12,P['green'],'Bold')
    D.chip(f,'Resume' if paused else 'Skip break' if breaking else 'Pause',563,687,112,'paper','ink')
    D.chip(f,'Finish session',688,687,169,'green','#FFFFFF')
    D.text(f,'SESSION CONTEXT',999,426,321,26,12,P['green'],'Bold')
    for i,(key,val) in enumerate([('Task','Architecture specification'),('Goal','Ship Life Manager v1'),('Elapsed','38 minutes'),('Interruptions','0 recorded')]):
        y=472+i*65;D.text(f,key,999,y,311,22,11,P['sub']);D.text(f,val,999,y+24,335,26,14,P['ink'],'Bold')
    return True

def dark_view(D,f,title,P):
    # The dark appearance uses the design system's Dark mode, so the board
    # is the same layout with the colour mode switched.
    D.j['nodes'][str(f)]['var_modes']={'VariableCollectionId:1':'VariableCollectionId:1/mode2'}
    D.rect(f,0,0,1440,900,'#102124')
    D.rect(f,0,0,252,900,'#0B191C')
    D.ellipse(f,28,26,28,28,'#58A88C');D.text(f,'LIFE / MANAGER',70,30,173,26,14,'#F0F9F4','Bold')
    dark_nav=(('Today','layout-dashboard'),('Inbox','inbox'),('Tasks','list-todo'),
              ('Calendar','calendar-days'),('Projects','folder-kanban'),('Focus','timer'),
              ('Habits','repeat-2'),('Agents','bot'),('Settings','settings-2'))
    for i,(s,icon) in enumerate(dark_nav):
        yy=110+i*63
        if s=='Today':D.rect(f,14,yy-6,219,42,'#274F45',8)
        D.text(f,s,64,yy+4,160,24,14,'#E3F1E8' if s=='Today' else '#9DB9AE','Bold' if s=='Today' else 'Regular')
        import grid as _grid
        D.icon(f,icon,32,_grid.icon_y(yy+4,24,20,14),20,'#E3F1E8' if s=='Today' else '#9DB9AE',1.8)
    D.text(f,'DARK APPEARANCE   /   WCAG-AWARE TOKENS',291,105,868,25,11,'#75B49C','Bold')
    D.text(f,'Make room for what matters.',289,144,960,54,32,'#EFF8F3','Bold')
    for i,(name,number) in enumerate([('Tasks ahead','03'),('Time protected','2h 15m'),('Focus logged','3h 20m')]):
        x=290+i*364;D.rect(f,x,244,348,139,'#1B3537',13)
        D.text(f,name.upper(),x+23,267,302,20,11,'#92BFB0','Bold');D.text(f,number,x+22,304,299,53,32,'#F5FAF6','Bold')
    D.rect(f,290,407,719,414,'#1B3537',14);D.text(f,'Meaningful actions',315,432,621,32,19,'#F2F9F6','Bold')
    for i,s in enumerate(['Refine the planning flow','Review priorities and capacity','Protect the next deep-work block','Close unfinished tasks']):
        y=505+i*66;D.ellipse(f,322,y+3,23,23,'#3D7364');D.text(f,s,362,y,584,34,15,'#D7E9E2')
    D.rect(f,1031,406,369,415,'#1B3537',13);D.text(f,'Today’s rhythm',1056,438,316,34,19,'#E9F5EF','Bold')
    for i,(time,name) in enumerate([('09:00','Daily plan'),('11:00','Deep work'),('15:00','Review')]):
        y=525+i*79;D.text(f,time,1056,y,75,28,14,'#9FC9BA');D.text(f,name,1148,y,219,30,14,'#F3F9F6','Bold')
    return True

def agent_technical(D,f,title,P):
    t=title.lower();is_mcp='mcp' in t;is_palette='palette' in t;is_rules='automation gallery' in t
    if is_palette:
        D.rect(f,290,239,1112,601,P['paper'],14,stroke=P['line']);over=D.rect(f,290,239,1112,601,P['nav']);D.j['nodes'][str(over)]['opacity']=.28
        D.rect(f,491,294,712,468,P['paper'],16)
        D.text(f,'COMMANDS   /   SEARCH ANYTHING',517,323,663,27,11,P['green'],'Bold')
        D.rect(f,517,361,655,52,'#F0F5F1',9)
        D.text(f,'>   create task for tomorrow 09:00',540,373,591,28,17,P['ink'],'Bold')
        for i,(cmd,shortcut) in enumerate([('Create task','⌘ N'),('Open today','G T'),('Search everything','⌘ K'),('Start focus','⌘ F'),('Open agent activity','G A')]):
            y=437+i*54;D.text(f,cmd,541,y,450,27,14,P['ink'],'Bold' if i==0 else 'Regular');D.chip(f,shortcut,1090,y,72,'bg','sub')
        return True
    if is_rules:
        D.rect(f,290,239,1112,601,P['paper'],14,stroke=P['line']);D.text(f,'AUTOMATION RULES',315,267,687,28,12,P['green'],'Bold')
        rules=[('Plan my day','Every weekday at 08:30','Generate suggested priorities'),('Review unfinished work','Every evening at 18:15','Queue outstanding work for review'),('Weekly progress report','Friday at 17:00','Summarize time, goals and blockers'),('Verified backup','Every day at 20:00','Create and verify a SQLite snapshot')]
        for i,(name,trigger,outcome) in enumerate(rules):
            yy=313+i*118;D.rect(f,309,yy,1074,102,'#F5F8F5',11,stroke=P['line']);D.ellipse(f,331,yy+23,34,34,P['mint']);D.text(f,'↻',338,yy+28,26,28,18,P['green'],'Bold');D.text(f,name,387,yy+16,678,28,17,P['ink'],'Bold');D.text(f,trigger+'   →   '+outcome,387,yy+49,835,29,12,P['sub']);D.chip(f,'Enabled',1272,yy+33,84,'mint','green')
        return True
    if is_mcp or 'cli quick' in t:
        D.rect(f,290,240,1110,600,P['paper'],14,stroke=P['line'])
        D.text(f,'LOCAL MCP SERVER' if is_mcp else 'CLI CONTRACT',316,271,676,29,12,P['green'],'Bold')
        D.text(f,'One data model. Every surface.',316,315,896,50,28,P['ink'],'Bold')
        D.text(f,'No window required. Safe typed commands with JSON output and audited mutations.',318,375,989,29,13,P['sub'])
        D.rect(f,318,423,760,350,P['nav'],14)
        cmds=(['$ life mcp serve --stdio','$ life agent permissions --json','$ life task create --json < task.json','$ life calendar conflicts --json','$ life history --actor pi --limit 30'] if is_mcp else ['$ life today --json','$ life task create --title "Write review" --due tomorrow','$ life task list --priority P1 --json','$ life focus start --task <id>','$ life schedule propose --date tomorrow'])
        for i,s in enumerate(cmds):D.text(f,s,343,453+i*56,708,32,13,'#D9F1E5','Bold' if i==0 else 'Regular')
        D.rect(f,1094,424,281,349,'#E9F3EC',13)
        for i,(lab,val) in enumerate([('Transport','stdio · default'),('Data','SQLite · local'),('Authorization','Same OS user'),('Failure','Stable errors'),('Replay','Idempotent writes')]):
            y=452+i*58;D.text(f,lab.upper(),1114,y,228,19,10,P['green'],'Bold');D.text(f,val,1114,y+22,243,27,13,P['ink'],'Bold')
        return True
    return False

def settings_specific(D,f,title,P):
    t=title.lower()
    if not any(s in t for s in ['calendar','github','notification','backup','restore','security','diagnostic','storage','shortcut','integration','area configuration','keyboard','about','version']):return False
    D.rect(f,290,242,1112,596,P['paper'],15,stroke=P['line'])
    D.text(f,'CONFIGURE   /   '+title.upper(),319,268,1011,26,11,P['green'],'Bold')
    if 'calendar' in t or 'github' in t or 'integration' in t:
        provider='Google Calendar' if 'google' in t else 'Apple Calendar' if 'apple' in t else 'GitHub' if 'github' in t else 'Calendar sources'
        D.rect(f,319,328,1047,152,'#F4F8F5',13)
        D.ellipse(f,345,362,76,76,P['blue']);D.text(f,provider[0],364,385,48,40,32,P['ink'],'Bold')
        D.text(f,provider,451,361,776,43,24,P['ink'],'Bold');D.chip(f,'Not connected',1073,376,159,'peach','rust')
        D.text(f,'Optional integration. Your offline calendar remains fully available.',453,416,813,28,13,P['sub'])
        for i,(a,b) in enumerate([('Read scope','Import availability and event details'),('Write scope','Off until explicitly enabled'),('Conflict behavior','Never overwrite local events automatically'),('Recovery','Disconnect without deleting local entries')]):
            yy=517+i*64;D.text(f,a,325,yy,221,26,12,P['sub']);D.text(f,b,590,yy,708,25,13,P['ink'],'Bold');D.rule(f,325,yy+42,1034)
        return True
    if 'backup' in t or 'restore' in t or 'diagnostic' in t or 'storage' in t:
        labels=[('Database integrity','Healthy'),('Latest verified backup','Today · 20:00'),('Previous restore point','Yesterday · 20:00'),('Local retention','30 daily copies'),('Attachments','App-managed storage')]
        for i,(lab,val) in enumerate(labels):
            yy=326+i*86;D.rect(f,316,yy,1049,72,'#F6F8F5',10)
            D.text(f,lab,339,yy+13,471,29,15,P['ink'],'Bold');D.text(f,val,998,yy+17,338,28,13,P['green'],'Bold')
        D.chip(f,'Verify backup',1100,781,199,'mint','green')
        return True
    if 'notification' in t:
        opts=[('Task deadlines','Notify before a due time',True),('Daily planning','Weekdays at 08:30',True),('Quiet hours','22:00–08:00',True),('Missed habit prompts','Do not nag automatically',False)]
    elif 'security' in t:
        opts=[('Local database','Same-user access',True),('Agent mutations','Actor attribution and history',True),('Destructive bulk writes','Require confirmation',True),('Remote MCP access','Disabled by default',False)]
    else:
        opts=[('Global quick capture','⌘ ⇧ Space',True),('Command palette','⌘ K',True),('Focus mode','⌘ ⇧ F',True),('Accessibility shortcuts','Show keyboard reference',True)]
    for i,(a,b,on) in enumerate(opts):
        y=335+i*110;D.rect(f,320,y,1047,96,'#F6F8F5',10)
        D.text(f,a,343,y+17,742,31,18,P['ink'],'Bold');D.text(f,b,343,y+53,755,27,12,P['sub'])
        D.rect(f,1220,y+30,111,39,P['mint'] if on else P['line'],20)
        D.ellipse(f,1296 if on else 1225,y+35,30,30,P['green'] if on else P['sub'])
    return True

def list_special(D,f,title,P,domain):
    t=title.lower()
    if 'backup history' in t or 'time log history' in t or 'automation execution history' in t or 'journal archive' in t:
        D.rect(f,290,244,1112,594,P['paper'],14,stroke=P['line'])
        D.text(f,'HISTORY',313,277,743,28,12,P['green'],'Bold')
        D.text(f,'A clear record of what happened',314,316,890,49,25,P['ink'],'Bold')
        if 'backup' in t:
            records=[('Today · 20:00','Verified snapshot','Healthy · 8.4 MB'),('Yesterday · 20:00','Verified snapshot','Healthy · 8.3 MB'),('Saturday · 20:00','Verified snapshot','Healthy · 8.2 MB'),('Friday · 20:00','Verified snapshot','Healthy · 8.2 MB')]
        elif 'time log' in t:
            records=[('09:00–10:32','Architecture focus','92 minutes'),('11:04–11:35','Design review','31 minutes'),('13:00–14:15','Learning session','75 minutes'),('15:20–15:55','Planning','35 minutes')]
        elif 'automation' in t:
            records=[('08:30','Daily plan suggestion','Completed'),('20:00','Backup verification','Completed'),('Yesterday','Weekly review','Completed'),('Yesterday','Task follow-up','Skipped; condition unmet')]
        else:
            records=[('Today','Evening reflection','Learning and progress'),('Yesterday','Daily review','Three meaningful actions'),('Friday','Weekly review','Plans and actuals'),('Thursday','Quick note','Next project milestone')]
        for i,(time,name,sub) in enumerate(records):
            y=401+i*94;D.ellipse(f,321,y+7,35,35,P['mint']);D.text(f,'✓',331,y+12,21,21,16,P['green'],'Bold');D.text(f,name,377,y,580,30,17,P['ink'],'Bold');D.text(f,sub,377,y+34,621,27,12,P['sub']);D.text(f,time,1100,y+15,231,26,12,P['green'],'Bold');D.rule(f,319,y+75,1034)
        return True
    return False

def board_special(D,f,title,P,domain):
    t=title.lower()
    if 'career pipeline' not in t and 'routine builder' not in t:return False
    phases=(['APPLIED','INTERVIEWING','DECISION'] if 'career' in t else ['MORNING','AFTERNOON','EVENING'])
    data=(['Senior Platform Engineer','Senior AI Engineer','Full-Stack Engineer','Follow-up application','Technical preparation','Review offer'] if 'career' in t else ['Wake and hydrate','Meditate · 15m','Plan priorities','Walk · 30m','Read · 20m','Reflect · 10m'])
    for col,phase in enumerate(phases):
        x=290+col*362;D.rect(f,x,248,346,591,'#EEF3EF',14);D.text(f,phase,x+23,276,291,29,13,P['green'],'Bold')
        for i in range(2):
            y=335+i*189;D.rect(f,x+16,y,314,168,P['paper'],12,stroke=P['line']);D.text(f,data[col*2+i],x+33,y+20,282,60,18,P['ink'],'Bold')
            D.chip(f,'Next action',x+31,y+116,117,'mint','green')
        D.text(f,'+   Add another step',x+20,765,316,28,13,P['green'],'Bold')
    return True

def analytics_special(D,f,title,P):
    t=title.lower()
    if 'savings goal' not in t and 'goal' not in t and 'plan versus actual' not in t:return False
    D.rect(f,290,239,1112,604,P['paper'],14,stroke=P['line'])
    if 'savings' in t:
        D.label(f,'SAVINGS MILESTONE',320,276)
        D.text(f,'62%',320,313,526,110,72,P['ink'],'Bold')
        D.text(f,'Progress toward your manually entered goal',321,429,712,33,15,P['sub'])
        D.bar(f,320,496,1028,.62)
        D.text(f,'Contributions are not automatically linked to bank accounts.',322,545,981,30,13,P['ink2'])
    elif 'plan versus actual' in t:
        D.label(f,'CAPACITY REPORT',322,275)
        D.text(f,'Planned 22h   /   Actual 18h',320,316,928,73,35,P['ink'],'Bold')
        for i,(a,b) in enumerate([('Deep work',.82),('Learning',.61),('Personal',.45),('Admin',.28)]):
            yy=419+i*94;D.text(f,a,322,yy,190,28,14,P['ink'],'Bold');D.bar(f,528,yy+7,747,b)
    else:
        D.label(f,'GOAL PROGRESS',322,276)
        D.text(f,'Build a meaningful rhythm',322,314,946,59,32,P['ink'],'Bold')
        for i,(name,val) in enumerate([('Ship Life Manager v1',.74),('Practice consistently',.62),('Strengthen skills',.48),('Make time for personal life',.81)]):
            yy=416+i*92;D.text(f,name,322,yy,673,30,15,P['ink'],'Bold');D.bar(f,322,yy+40,1004,val)
    return True

def render_special(D,f,title,view,domain,index,P,CONTENT):
    t=title.lower()
    if view=='wizard':return wizard(D,f,title,index,P)
    if view=='state':return state_view(D,f,title,P)
    if view=='calendar':return calendar_view(D,f,title,P)
    if view=='focus':return focus_view(D,f,title,P)
    if 'dark appearance' in t:return dark_view(D,f,title,P)
    if view=='agent' and agent_technical(D,f,title,P):return True
    if view=='settings' and settings_specific(D,f,title,P):return True
    if view=='list' and list_special(D,f,title,P,domain):return True
    if view=='board' and board_special(D,f,title,P,domain):return True
    if view=='analytics' and analytics_special(D,f,title,P):return True
    return False

# Second-pass semantic refinements, based on exported contact-sheet inspection.
BASE_SPECIAL=render_special

def project_portfolio(D,f,P):
    projects=[('Life Manager','DESIGN SYSTEM','194 design specimens',.78),('Dhamma Echo','DESIGN REVIEW','Current master in revision',.64),('Jet Engine Lab','ENGINEERING','Digital twin milestones',.47),('Yangon Simulator','EXPLORATION','Worldbuilding roadmap',.32)]
    for i,(name,kind,sub,p) in enumerate(projects):
        x=290+(i%2)*560;y=246+(i//2)*290
        D.rect(f,x,y,552,265,P['paper'],15,stroke=P['line'])
        D.chip(f,kind,x+23,y+21,170,'mint' if i==0 else 'bg','green')
        D.text(f,name,x+23,y+70,494,47,24,P['ink'],'Bold')
        D.text(f,sub,x+24,y+126,494,31,13,P['sub'])
        D.bar(f,x+24,y+181,504,p)
        D.text(f,f'{int(p*100)}%  •  View project →',x+26,y+212,469,29,12,P['green'],'Bold')
    return True

def learning_dashboard(D,f,P):
    D.rect(f,290,243,1112,597,P['paper'],14,stroke=P['line'])
    D.label(f,'ACTIVE LEARNING',316,273)
    for i,(name,status,sub,p) in enumerate([('CKAD practice','Study plan','Four practice blocks remaining',.56),('System design','Reading','Architecture review and notes',.68),('AI agent orchestration','Applied study','Design a verifiable workflow',.42)]):
        y=324+i*163;D.rect(f,313,y,1060,147,'#F5F8F5',12,stroke=P['line'])
        D.text(f,name,337,y+17,800,39,20,P['ink'],'Bold');D.chip(f,status,1184,y+20,158,'mint','green')
        D.text(f,sub,337,y+67,810,24,13,P['sub']);D.bar(f,337,y+109,997,p)
    return True

def gantt_view(D,f,title,P):
    yearly='annual' in title.lower()
    tasks=['Research and design','Data and domain model','CLI and MCP integration','Desktop UI and testing','Polish and release'] if not yearly else ['Product and focus','Deep work habit','Learning and growth','Career development','Personal wellbeing']
    headers=['JAN','FEB','MAR','APR','MAY','JUN','JUL','AUG','SEP','OCT','NOV','DEC'] if yearly else ['W1','W2','W3','W4','W5','W6','W7','W8']
    D.rect(f,290,242,1112,596,P['paper'],14,stroke=P['line'])
    D.text(f,'MILESTONE ROADMAP' if not yearly else 'YEAR AT A GLANCE',317,271,920,28,11,P['green'],'Bold')
    D.text(f,'Work sequenced against real time',315,308,946,44,25,P['ink'],'Bold')
    left=330;start=674;cell=(680/len(headers))
    for i,h in enumerate(headers):D.text(f,h,start+i*cell,386,cell-5,27,11,P['sub'],'Bold')
    for i,name in enumerate(tasks):
        y=424+i*72;D.text(f,name,left,y+12,322,37,14,P['ink'],'Bold');D.rect(f,654,y,710,62,'#F4F8F5',8)
        for c in range(len(headers)):D.rect(f,start+c*cell,y+2,1,57,P['line'])
        begin=i if not yearly else i*2
        span=3 if not yearly else 4
        if begin>=len(headers):begin=len(headers)-2
        w=min(span,len(headers)-begin)*cell-6
        D.rect(f,start+begin*cell,y+16,w,28,P['mint'] if i%2==0 else P['peach'],7)
        D.text(f,['Research','Milestone','Review','Delivery','Measure'][i],start+begin*cell+11,y+23,w-12,20,11,P['green'] if i%2==0 else P['rust'],'Bold')
    return True

def review_flow(D,f,title,P):
    week='weekly' in title.lower();label='WEEKLY REVIEW' if week else 'MONTHLY REVIEW'
    D.rect(f,290,243,1112,595,P['paper'],14,stroke=P['line'])
    D.label(f,label,317,278)
    D.text(f,'What deserves your attention?',318,315,916,52,29,P['ink'],'Bold')
    stages=[('01','Recognize progress','Three outcomes worth remembering'),('02','Notice friction','Which commitments were unrealistic?'),('03','Check direction','Are the current goals still meaningful?'),('04','Choose changes','Decide what to stop, start, and continue')]
    for i,(number,name,sub) in enumerate(stages):
        y=399+i*98;D.rect(f,316,y,1058,83,'#F4F8F5',11)
        D.ellipse(f,334,y+22,38,38,P['mint']);D.text(f,number,343,y+32,26,21,12,P['green'],'Bold')
        D.text(f,name,397,y+13,754,30,17,P['ink'],'Bold');D.text(f,sub,398,y+44,715,22,12,P['sub'])
        D.text(f,'Open →',1257,y+31,109,20,12,P['green'],'Bold')
    return True

def component_samples(D,f,title,P):
    access='accessibility' in title.lower();responsive='responsive' in title.lower()
    if responsive:
        import build_figcraft as bf
        D.rect(f,0,0,1440,900,'#DDE5E0');D.rect(f,170,52,1050,791,P['paper'],18)
        D.rect(f,170,52,88,791,P['nav']);D.ellipse(f,199,77,27,27,P['green2'])
        nav=('layout-dashboard','inbox','list-todo','calendar-days','folder-kanban',
             'target','timer','repeat-2','notebook-pen','settings-2')
        for i,name in enumerate(nav):
            D.icon(f,name,200,160+i*62,26,'#D4E9DF',1.8)
        D.text(f,'1024 PX  /  COMPACT VIEWPORT',293,89,826,26,11,P['green'],'Bold')
        D.text(f,'Today, without the clutter.',294,141,842,54,32,P['ink'],'Bold')
        for i,(lab,val) in enumerate([('PRIORITIES','03'),('FOCUS','2h 15m')]):
            D.rect(f,290+i*448,235,422,126,'#F3F7F4',12);D.label(f,lab,310+i*448,258);D.text(f,val,310+i*448,290,393,50,32,P['ink'],'Bold')
        D.rect(f,290,393,870,376,'#F3F7F4',12)
        for i,task in enumerate(['Review tomorrow’s priorities','Complete the main task','Protect afternoon learning','Leave space to recover']):D.text(f,'○  '+task,320,434+i*76,783,41,19,P['ink'],'Bold')
        return True
    D.rect(f,290,244,1112,594,P['paper'],14,stroke=P['line'])
    D.label(f,'ACCESSIBILITY SPECIMEN' if access else 'COMPONENT STATES',320,273)
    D.text(f,'Usable in every state.',319,308,973,47,29,P['ink'],'Bold')
    D.text(f,'Visible focus · Discernible states · Predictable keyboard behavior',319,363,934,33,13,P['sub'])
    for i,(label,c,txt) in enumerate([('Primary','#266C56','#FFFFFF'),('Secondary','#DDEEE5','#266C56'),('Warning','#F5E8DD','#C16A4C'),('Disabled','#E4E9E6','#8E9E9D')]):
        x=323+i*268;D.rect(f,x,444,238,52,c,11)
        D.text(f,label,x+23,457,193,30,15,txt,'Bold')
    D.label(f,'KEYBOARD FOCUS',323,553)
    D.rect(f,320,582,484,70,P['paper'],11,stroke=P['green'])
    D.text(f,'Selected by Tab · Enter activates',342,604,435,37,15,P['ink'],'Bold')
    D.label(f,'INPUT LABEL & ERROR',847,552)
    D.rect(f,846,583,517,68,'#F9F1EF',10,stroke=P['rust'])
    D.text(f,'A date is required before saving',868,603,465,31,15,P['red'],'Bold')
    D.text(f,'Text scaling • Reduced motion • High contrast • Screen reader order',323,715,1021,31,13,P['sub'])
    return True

def finance_reminders(D,f,P):
    D.rect(f,290,241,1112,596,P['paper'],14,stroke=P['line']);D.label(f,'UPCOMING PERSONAL PAYMENTS',320,275)
    data=[('Utility reminder','Nov 15 · 2026','Confirm bill due date'),('Monthly budget check','Nov 30 · 2026','Compare planned vs actual'),('Subscriptions review','Dec 01 · 2026','Review renewal options'),('Savings contribution','Dec 05 · 2026','Log a manual transfer')]
    for i,(a,b,c) in enumerate(data):
        y=330+i*117;D.rect(f,315,y,1064,98,'#F5F8F5',11)
        D.text(f,a,339,y+13,717,31,18,P['ink'],'Bold');D.text(f,c,339,y+54,677,27,12,P['sub'])
        D.chip(f,b,1120,y+35,224,'mint','green')
    D.text(f,'Planning only. No bank account or financial institution connection is required.',320,807,1040,23,12,P['sub'])
    return True


def render_special(D,f,title,view,domain,index,P,CONTENT):
    t=title.lower()
    if t in ['project portfolio']:return project_portfolio(D,f,P)
    if t in ['learning dashboard']:return learning_dashboard(D,f,P)
    if 'project · timeline' in t or t=='annual direction':return gantt_view(D,f,title,P)
    if 'weekly review workflow' in t or 'monthly review workflow' in t:return review_flow(D,f,title,P)
    if t in ['component states','accessibility inspection','responsive small window']:return component_samples(D,f,title,P)
    if t=='financial reminders':return finance_reminders(D,f,P)
    if t=='project health report':return analytics_health(D,f,title,P)
    return BASE_SPECIAL(D,f,title,view,domain,index,P,CONTENT)

def analytics_health(D,f,title,P):
    D.rect(f,290,243,1112,595,P['paper'],14,stroke=P['line'])
    for i,(key,val,sub) in enumerate([('DELIVERY','72%','Completed tasks'),('BLOCKERS','02','Require a decision'),('MILESTONES','3 / 5','On track')]):
        x=316+i*353;D.rect(f,x,281,333,126,'#F3F8F4',12);D.label(f,key,x+23,301);D.text(f,val,x+22,330,298,43,31,P['ink'],'Bold');D.text(f,sub,x+22,381,291,24,11,P['sub'])
    D.label(f,'RISK REGISTER',318,458)
    for i,(s,detail) in enumerate([('Task dependencies','Two items waiting on design approval'),('Schedule volatility','Review deadlines at the next milestone'),('Scope decisions','One open implementation question')]):
        y=498+i*93;D.rect(f,315,y,1059,79,'#F6F8F6',10);D.text(f,s,341,y+11,619,29,17,P['ink'],'Bold');D.text(f,detail,341,y+42,841,24,12,P['sub']);D.chip(f,'Review',1261,y+21,88,'peach','rust')
    return True

# Final board-specific repairs identified in the third screenshot inspection.
PREVIOUS_RENDER_SPECIAL=render_special

def area_configuration(D,f,P):
    D.rect(f,290,243,1112,596,P['paper'],14,stroke=P['line'])
    D.label(f,'LIFE AREAS  /  CUSTOMIZE',317,278)
    D.text(f,'Make this workspace yours.',317,317,921,53,28,P['ink'],'Bold')
    for i,(name,icon,desc,color) in enumerate([('Projects','P','Creative and engineering work','mint'),('Learning','L','Skills, study and certifications','blue'),('Personal','H','Habits, health and relationships','peach'),('Finance','F','Goals, bills and administrative reminders','yellow')]):
        y=406+i*96
        D.rect(f,316,y,1057,83,'#F6F8F6',10)
        D.rect(f,335,y+18,46,46,P[color],10)
        D.text(f,icon,351,y+29,29,28,15,P['ink'],'Bold')
        D.text(f,name,399,y+9,616,31,17,P['ink'],'Bold')
        D.text(f,desc,399,y+43,711,27,12,P['sub'])
        D.text(f,'Edit  ·  Archive',1200,y+27,158,29,12,P['green'],'Bold')
    D.text(f,'+   Add a new area',326,799,356,30,14,P['green'],'Bold')
    return True

def about_and_version(D,f,P):
    D.rect(f,290,243,1112,596,P['paper'],14,stroke=P['line'])
    D.ellipse(f,753,295,138,138,P['green'])
    D.text(f,'LIFE',791,349,105,48,26,'#FFFFFF','Bold')
    D.text(f,'Life Manager',618,471,465,64,39,P['ink'],'Bold')
    D.text(f,'Private, local, intentional.',631,546,459,37,16,P['sub'])
    for i,(k,v) in enumerate([('Design edition','FigCraft · native master'),('Platform target','macOS Apple Silicon'),('Planned framework','Tauri 2 / Rust / SQLite'),('Connectivity','Offline first')]):
        x=382+(i%2)*426;y=630+(i//2)*81
        D.label(f,k,x,y,379);D.text(f,v,x,y+24,380,37,14,P['ink'],'Bold')
    return True

def course_progress(D,f,P):
    D.rect(f,290,243,1112,596,P['paper'],14,stroke=P['line'])
    D.chip(f,'LEARNING',319,279,119,'blue','ink')
    D.text(f,'Kubernetes application development',319,326,1021,59,27,P['ink'],'Bold')
    D.text(f,'Course progress and practice milestones',320,397,884,29,14,P['sub'])
    D.bar(f,321,452,1019,.56)
    for i,(name,status) in enumerate([('Core Kubernetes resources','Completed'),('Application configuration','Completed'),('Deployments and networking','In progress'),('Practice exam and review','Upcoming')]):
        y=493+i*79;D.rect(f,315,y,1059,62,'#F6F8F6',9)
        D.ellipse(f,335,y+17,26,26,P['mint'] if i<2 else P['peach'])
        D.text(f,'✓' if i<2 else '○',342,y+22,22,22,13,P['green'],'Bold')
        D.text(f,name,378,y+13,709,32,15,P['ink'],'Bold')
        D.chip(f,status,1190,y+17,157,'mint' if i<2 else 'bg','green' if i<2 else 'sub')
    return True

def project_summary(D,f,P):
    D.rect(f,290,241,1112,596,P['paper'],14,stroke=P['line'])
    D.chip(f,'DESIGN PHASE',317,278,141,'mint','green')
    D.text(f,'Life Manager',317,324,701,57,31,P['ink'],'Bold')
    D.text(f,'Build an agent-first local productivity manager with deliberate, low-friction workflows.',319,385,953,55,15,P['sub'])
    for i,(name,value,sub) in enumerate([('MILESTONES','3 / 5','Design and validation'),('OPEN TASKS','12','Across planned phases'),('BLOCKERS','02','Need design decisions')]):
        x=314+i*355;D.rect(f,x,471,332,132,'#F5F8F5',10)
        D.label(f,name,x+17,489);D.text(f,value,x+17,523,300,42,28,P['ink'],'Bold')
        D.text(f,sub,x+18,573,299,20,11,P['sub'])
    D.label(f,'UPCOMING MILESTONES',320,644)
    for i,s in enumerate(['Finish FigCraft source master','Approve architecture and CLI contracts','Start domain and persistence implementation']):
        D.ellipse(f,333,687+i*43,17,17,P['mint'])
        D.text(f,s,367,681+i*43,935,34,14,P['ink'])
    return True

def recurring_editor(D,f,title,P):
    D.rect(f,290,239,1112,602,P['paper'],14,stroke=P['line'])
    over=D.rect(f,290,239,1112,602,P['nav']);D.j['nodes'][str(over)]['opacity']=.27
    D.rect(f,601,201,579,541,P['paper'],18)
    D.text(f,'RECURRENCE RULE',629,231,503,24,11,P['green'],'Bold')
    D.text(f,title,629,265,507,53,24,P['ink'],'Bold')
    rows=[('Frequency','Every week'),('On days','Mon, Wed, Fri'),('Time zone','Asia/Bangkok'),('Ends','Never'),('Exception handling','Edit one, future or entire series')]
    for i,(lab,v) in enumerate(rows):
        y=331+i*67;D.label(f,lab,630,y,520)
        D.rect(f,628,y+21,520,35,'#F3F7F4',7)
        D.text(f,v,646,y+28,479,29,13,P['ink'],'Bold')
    D.chip(f,'Cancel',797,699,105,'bg','sub')
    D.chip(f,'Review occurrences',911,699,241,'green','#FFFFFF')
    return True

def project_blockers(D,f,P):
    D.rect(f,290,242,1112,597,P['paper'],14,stroke=P['line'])
    D.text(f,'BLOCKED WORK  /  WHAT NEEDS A DECISION',319,275,1059,28,12,P['rust'],'Bold')
    for i,(a,b,c) in enumerate([('Data-model migrations','Waiting on schema approval','BLOCKED'),('Focus-session recovery','Waiting on process behavior review','BLOCKED'),('CLI output contract','Ready after schema freeze','NEXT'),('Calendar conflict rules','Ready for review','NEXT')]):
        y=333+i*116;D.rect(f,315,y,1058,98,'#F6F8F6',11)
        D.text(f,a,338,y+13,697,31,18,P['ink'],'Bold')
        D.text(f,b,338,y+53,755,29,12,P['sub'])
        D.chip(f,c,1228,y+35,120,'peach' if i<2 else 'mint','rust' if i<2 else 'green')
    return True

def render_special(D,f,title,view,domain,index,P,CONTENT):
    low=title.lower()
    if low=='area configuration':return area_configuration(D,f,P)
    if low=='about and version':return about_and_version(D,f,P)
    if low=='course progress':return course_progress(D,f,P)
    if low=='project · summary':return project_summary(D,f,P)
    if low=='project · blockers':return project_blockers(D,f,P)
    if low=='recurring event' or low=='recurring task editor':return recurring_editor(D,f,title,P)
    return PREVIOUS_RENDER_SPECIAL(D,f,title,view,domain,index,P,CONTENT)
