#!/usr/bin/env python3
"""Check new FigCraft-aligned domain flows using a disposable SQLite file and actual release CLI/MCP."""
import json,subprocess,tempfile,pathlib,os
BIN=os.environ.get('LIFE_CLI','/tmp/life-manager-target/release/life')
checks=[]
with tempfile.TemporaryDirectory(prefix='life-figcraft-qa-') as work:
 db=str(pathlib.Path(work)/'figcraft.sqlite')
 def call(action,payload=None):
  argv=[BIN,'--db',db,'--json','--actor','mcp','call',action,'--payload',json.dumps(payload or {})]
  p=subprocess.run(argv,capture_output=True,text=True,timeout=15)
  if p.returncode:raise AssertionError(f'{action} failed: {p.stderr} {p.stdout}')
  doc=json.loads(p.stdout);assert doc['ok'],doc
  return doc['data']
 def check(name,condition):assert condition,name;checks.append(name)
 task=call('tasks.create',{'title':'Review build','priority':2,'actor':'mcp'})
 change=next(r for r in call('agents.changes') if r['action']=='tasks.create')
 check('Audited creation includes actor and after snapshot',change['actor']=='mcp' and change['after_json'] is not None)
 update=call('tasks.update',{'id':task['id'],'title':'Review live build','actor':'mcp'})
 change=next(r for r in call('agents.changes') if r['action']=='tasks.update')
 check('Audited update stores before and after',json.loads(change['before_json'])['title']=='Review build' and json.loads(change['after_json'])['title']=='Review live build')
 call('agents.undo',{'id':change['id']})
 check('Guarded undo recovers former task',call('tasks.get',{'id':task['id']})['title']=='Review build')
 tpl=call('templates.create',{'name':'Weekly review','title':'Review commitments','priority':2})
 made=call('templates.apply',{'id':tpl['id']})
 check('Template application produces a real task',made['title']=='Review commitments')
 call('tasks.bulk',{'ids':[task['id'],made['id']],'fields':{'priority':1}})
 check('Atomic bulk priority persisted',all(t['priority']==1 for t in call('tasks.list') if t['id'] in [task['id'],made['id']]))
 view=call('saved_views.create',{'name':'Urgent work','query':{'priority':1}})
 check('Saved filter created',view['name']=='Urgent work')
 area=call('areas.create',{'name':'FigCraft QA'})
 check('Area dashboard uses canonical entities',call('areas.dashboard',{'id':area['id']})['area']['name']=='FigCraft QA')
 rules=call('availability.replace',{'rules':[{'weekday':0,'start_time':'09:00','end_time':'17:00'}]})
 check('Weekly availability persisted',len(rules)==1)
 goal=call('goals.create',{'title':'Learning goal'})
 kr=call('key_results.create',{'goal_id':goal['id'],'title':'Modules','target':8,'baseline':0})
 call('key_results.update',{'id':kr['id'],'current':4})
 check('Key-result progression recorded',len(call('goals.history',{'goal_id':goal['id']}))==1)
 note=call('notes.create',{'title':'Research context'})
 linked=call('links.add',{'source_kind':'notes','source_id':note['id'],'target_kind':'tasks','target_id':task['id']})
 check('Notes/tasks backlink query works',call('links.list',{'kind':'notes','id':note['id']})[0]['id']==linked['id'])
 items=[{'type':'tasks','title':'Imported task'},{'type':'notes','title':'Imported note'}]
 preview=call('import.preview',{'items':items});check('Import dry run classifies both rows',len(preview['valid'])==2)
 applied=call('import.apply',{'items':items,'confirm':True});check('Atomic import applies both rows',len(applied['created'])==2)
 check('Import detects repeated records',len(call('import.preview',{'items':items})['duplicates'])==2)
 event=call('events.create',{'title':'Calendar block','start_at':'2026-11-15T09:00:00','end_at':'2026-11-15T10:00:00'})
 ics=call('calendar.export_ics');check('ICS export includes event',event['id'] in ics['content'])
 summary=call('reports.summary',{'from':'2026-11-01','to':'2026-11-30'});check('Report has source explanation','SQLite' in summary['source'])
 project=call('projects.create',{'name':'Timeline test'})
 check('Project timeline returns a project',call('projects.timeline',{'id':project['id']})['project']['id']==project['id'])
 messages='\n'.join(json.dumps(v) for v in [
 {'jsonrpc':'2.0','id':1,'method':'initialize','params':{}},
 {'jsonrpc':'2.0','id':2,'method':'tools/list','params':{}},
 {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'life_key_results_list','arguments':{'goal_id':goal['id']}}},
 {'jsonrpc':'2.0','id':4,'method':'tools/call','params':{'name':'life_agents_changes','arguments':{}}},
 ])+'\n'
 p=subprocess.run([BIN,'--db',db,'mcp','serve','--stdio'],input=messages,capture_output=True,text=True,timeout=15)
 check('MCP server exits successfully',p.returncode==0)
 responses=[json.loads(s) for s in p.stdout.splitlines() if s.strip()]
 names={t['name'] for t in responses[1]['result']['tools']}
 check('MCP advertises new feature tools',{'life_templates_apply','life_import_preview','life_agents_undo','life_availability_replace'}<=names)
 check('MCP maps underscore resource names correctly','Modules' in responses[2]['result']['content'][0]['text'])
 check('MCP executes change history tool',responses[3]['result']['content'][0]['type']=='text')
print(json.dumps({'status':'PASS','count':len(checks),'checks':checks},indent=2))
