#!/usr/bin/env python3
"""Real CLI, SQLite and MCP integration verification against disposable local data."""
from __future__ import annotations
import json,os,pathlib,subprocess,tempfile,sys
root=pathlib.Path(__file__).resolve().parents[1]
cli=os.environ.get('LIFE_CLI','/tmp/life-manager-target/release/life')
assert pathlib.Path(cli).is_file(),f'Missing built CLI: {cli}'
checks=[]
with tempfile.TemporaryDirectory(prefix='life-manager-verified-') as temp:
    db=str(pathlib.Path(temp)/'life.sqlite')
    def call(action, payload=None):
        cmd=[cli,'--db',db,'--json','call',action,'--payload',json.dumps(payload or {})]
        p=subprocess.run(cmd,text=True,capture_output=True,timeout=20)
        if p.returncode:
            raise RuntimeError(f'{action} failed: {p.stderr}\n{p.stdout}')
        result=json.loads(p.stdout)
        assert result['ok'],result
        return result['data']
    def check(label,expr):
        assert expr,label
        checks.append(label)
    area=call('areas.create',{'name':'Personal research','color':'#266C56'})
    check('create and query custom area',any(v['id']==area['id'] for v in call('areas.list')))
    updated=call('areas.update',{'id':area['id'],'name':'Research and learning'})
    check('rename existing life area',updated['name']=='Research and learning')
    task=call('tasks.create',{'title':'Implement the whole workflow','priority':1,'area_id':area['id'],'request_id':'qa-cli-unique'})
    same=call('tasks.create',{'title':'Implement the whole workflow','priority':1,'area_id':area['id'],'request_id':'qa-cli-unique'})
    check('idempotent repeat returns original task',same['id']==task['id'])
    call('tasks.delete',{'id':task['id']})
    check('archived task hidden from active backlog',all(v['id']!=task['id'] for v in call('tasks.list')))
    check('archived task retained for recovery',any(v['id']==task['id'] for v in call('tasks.archived')))
    restored=call('tasks.restore',{'id':task['id']})
    check('task restored with revision',restored['id']==task['id'] and restored['revision']>1)
    project=call('projects.create',{'name':'Life Manager QA'})
    check('project creation',project['name']=='Life Manager QA')
    event=call('events.create',{'title':'Review focus','start_at':'2026-11-15T09:00:00','end_at':'2026-11-15T09:30:00'})
    check('event creation',event['title']=='Review focus')
    conflicts=call('calendar.conflicts',{'start_at':'2026-11-15T09:15:00','end_at':'2026-11-15T10:00:00'})
    check('calendar detects overlapping commitments',len(conflicts)>=1)
    habit=call('habits.create',{'title':'Daily read'})
    check('habit create and log',call('habits.log',{'habit_id':habit['id'],'day':'2026-11-15','value':20}) is not None)
    focus=call('focus.start',{'task_id':task['id']})
    stopped=call('focus.stop',{'id':focus['id']})
    check('persisted focus session start/stop',stopped['status']=='completed')
    first=call('backup.create')
    second=call('backup.create')
    check('automatically unique backup destinations',first['path']!=second['path'])
    check('verified SQLite backup',call('backup.verify',{'path':first['path']})['verified'] is True)
    created_after=call('tasks.create',{'title':'Snapshot should remove this'})
    check('new task exists before restore',any(t['id']==created_after['id'] for t in call('tasks.list')))
    restore=call('backup.restore',{'path':first['path'],'confirm':True})
    check('restore preserves pre-restore backup',pathlib.Path(restore['pre_restore_backup']).is_file())
    check('restoration accurately reverts later changes',all(t['id']!=created_after['id'] for t in call('tasks.list')))
    call('areas.delete',{'id':area['id']})
    check('area archival preserves task reference',call('tasks.get',{'id':task['id']})['area_id']==area['id'])
    mcp_request='\n'.join(json.dumps(item) for item in [
        {'jsonrpc':'2.0','id':1,'method':'initialize','params':{}},
        {'jsonrpc':'2.0','id':2,'method':'tools/list','params':{}},
        {'jsonrpc':'2.0','id':3,'method':'tools/call','params':{'name':'life_areas_list','arguments':{}}},
    ])+'\n'
    p=subprocess.run([cli,'--db',db,'mcp','serve','--stdio'],input=mcp_request,text=True,capture_output=True,timeout=15)
    assert p.returncode==0,p.stderr
    replies=[json.loads(line) for line in p.stdout.splitlines() if line.strip()]
    check('MCP initializes with handshake',replies[0]['result']['serverInfo']['name']=='life-manager')
    toolnames={t['name'] for t in replies[1]['result']['tools']}
    check('MCP tools include new area and restore operations',{'life_areas_create','life_tasks_restore','life_backup_verify','life_backup_restore'}<=toolnames)
    check('MCP executes a typed operation',replies[2]['result']['content'][0]['type']=='text')
print(json.dumps({'status':'PASS','checks':checks,'count':len(checks)},indent=2))
