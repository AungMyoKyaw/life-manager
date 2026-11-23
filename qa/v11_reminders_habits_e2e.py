#!/usr/bin/env python3
"""Release CLI/MCP integration against disposable SQLite: gentle habits and reminder leases."""
import json,os,subprocess,tempfile,pathlib
cli=os.environ.get('LIFE_CLI',str(pathlib.Path(__file__).resolve().parents[1]/'dist/life'))
checks=[]
with tempfile.TemporaryDirectory(prefix='life-v11-e2e-') as temp:
 db=str(pathlib.Path(temp)/'life.sqlite')
 def call(action,payload=None):
  proc=subprocess.run([cli,'--db',db,'--json','call',action,'--payload',json.dumps(payload or {})],text=True,capture_output=True,timeout=20)
  if proc.returncode:raise RuntimeError(f'{action}: {proc.stderr}')
  result=json.loads(proc.stdout)
  assert result['ok'],result
  return result['data']
 def verify(label,condition):assert condition,label;checks.append(label)
 h=call('habits.create',{'title':'Gentle exercise'})
 conf=call('habits.configure',{'id':h['id'],'quota_period':'weekly','weekly_quota':2,'rest_days':[5,6]})
 verify('custom weekly cadence',conf['weekly_quota']==2)
 call('habits.log',{'habit_id':h['id'],'day':'2026-11-09','value':1})
 call('habits.skip',{'habit_id':h['id'],'day':'2026-11-10','status':'rest'})
 progress=call('habits.weekly',{'day':'2026-11-11'})[0]
 verify('weekly completion count',progress['completed_days']==1)
 verify('rest counted separately',progress['rest_days_taken']==1)
 verify('rest never counted as missed completion',progress['skipped_days']==0)
 call('habits.log',{'habit_id':h['id'],'day':'2026-11-10','value':1})
 verify('later completion replaces rest',call('habits.weekly',{'day':'2026-11-11'})[0]['fulfilled'] is True)
 call('settings.set',{'key':'notification.preferences','value':{'enabled':True,'quiet_start':'22:00','quiet_end':'08:00'}})
 task=call('tasks.create',{'title':'Reminder smoke','due_date':'2026-11-09'})
 reminder_key='task:'+task['id']+':2026-11-09'
 due=call('reminders.due',{'now':'2026-11-09T09:00:00','forward_minutes':0,'lookback_minutes':0})
 verify('task deadline creates local due reminder',any(v['key']==reminder_key for v in due))
 verify('quiet hours suppress reminder',not call('reminders.due',{'now':'2026-11-09T07:00:00','lookback_minutes':1440}))
 lease=call('reminders.claim',{'key':reminder_key,'now':'2026-11-09T09:00:00','forward_minutes':0,'lookback_minutes':0})
 verify('atomic claim succeeds',lease['claimed'] is True)
 twice=call('reminders.claim',{'key':reminder_key,'now':'2026-11-09T09:00:00','forward_minutes':0,'lookback_minutes':0})
 verify('concurrent duplicate claim prevented',twice['claimed'] is False)
 call('reminders.delivered',{'key':reminder_key,'now':'2026-11-09T09:00:00'})
 verify('delivered reminder hidden',not call('reminders.due',{'now':'2026-11-09T09:00:00','lookback_minutes':60}))
 call('reminders.snooze',{'key':reminder_key,'now':'2026-11-09T09:00:00','until':'2026-11-10T11:00:00'})
 verify('snooze preserves reminder for future',len(call('reminders.upcoming',{'now':'2026-11-10T10:00:00','forward_minutes':120,'lookback_minutes':1440}))>=1)
 verify('delivery history audited',len(call('reminders.history'))>=1)
 messages='\n'.join(json.dumps(x) for x in [
  {'jsonrpc':'2.0','id':1,'method':'initialize','params':{}},
  {'jsonrpc':'2.0','id':2,'method':'tools/list','params':{}},
  {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'life_habits_weekly','arguments':{'day':'2026-11-09'}}},
  {'jsonrpc':'2.0','id':4,'method':'tools/call','params':{'name':'life_reminders_settings','arguments':{}}}
 ])+'\n'
 p=subprocess.run([cli,'--db',db,'mcp','serve','--stdio'],input=messages,text=True,capture_output=True,timeout=15)
 assert p.returncode==0,p.stderr
 replies=[json.loads(x) for x in p.stdout.splitlines() if x.strip()]
 names={t['name']:t for t in replies[1]['result']['tools']}
 verify('MCP exposes reminders and flexible habits',{'life_habits_configure','life_reminders_upcoming','life_reminders_snooze'}<=set(names))
 verify('MCP habit configure schema requires typed fields',set(names['life_habits_configure']['inputSchema']['required'])=={'id','quota_period','weekly_quota','rest_days'})
 verify('MCP snooze schema requires until',set(names['life_reminders_snooze']['inputSchema']['required'])=={'key','until'})
 verify('MCP executes weekly habit tool',replies[2]['result']['content'][0]['type']=='text')
 verify('MCP executes reminder settings tool',replies[3]['result']['content'][0]['type']=='text')
print(json.dumps({'status':'PASS','count':len(checks),'checks':checks},indent=2))
