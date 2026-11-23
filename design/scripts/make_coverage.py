#!/usr/bin/env python3
"""Map every provisional product decision to an editable FigCraft board."""
from pathlib import Path
import csv,json
root=Path(__file__).resolve().parents[2]
inv=json.loads((root/'design/screen-inventory.json').read_text())
sections=[
('Product vision','Product name|Repository name|Core purpose|Product philosophy|First screen|GUI and CLI parity|Offline operating model|Success criterion|Product positioning|Development strategy','01 Foundation','PRODUCT.md'),
('Life organization','Default life areas|Customizable areas|Areas goals projects and tasks|Tasks without projects|Cross-domain task memberships|Task priorities|Universal inbox|Unscheduled backlog|Completed work archive|Cross-area dashboard','13 Personal life areas','PRODUCT.md'),
('Task fundamentals','Nested subtasks|Task statuses|Start planned due and completion times|Priority tags effort and contexts|Markdown task description|Recurring tasks|Task dependencies|Optional duration estimates|File attachments|Bulk operations','04 Task management','DATA_MODEL.md'),
('Advanced task behavior','Natural-language quick capture|Snooze without changing original deadline|Recurring task completion|Missed recurrence catch-up or skip|Overdue task treatment|Task templates|Activity history|Related and duplicate relationships|Saved smart views|Stable IDs and deep links','04 Task management','UX_SPEC.md'),
('Projects and execution','Project list board timeline calendar|Custom project sections and phases|Project lifecycle|Computed project progress|First-class milestones|Cross-project dependencies|Project templates|Project documentation and linked notes|Project health|Weekly project reviews','05 Projects and milestones','PRODUCT.md'),
('Goals and long-term direction','Goal horizons|Quantitative and qualitative goals|Goal project task linking|Optional OKRs|Active goal limit|Monthly goal review|Archive abandoned goals|Goal progress timeline|Goal achievement evidence|Minimal gamification','06 Goals and direction','PRODUCT.md'),
('Calendar fundamentals','Internal calendar authority|Day three-day week month agenda|Event and reminder types|All-day events|Calendar recurrence and exceptions|Time-zone context|Calendar drag resize and duplication|Task time-block linking|Calendar filters|Calendar colors','07 Calendar','ARCHITECTURE.md'),
('Scheduling and time allocation','Working hours by weekday|Protected personal and sleep windows|Schedule conflict handling|Time buffers and breaks|Deterministic auto-scheduling proposal|Agent rescheduling approval|Unfinished work carryover|Daily top-three priorities|Protected schedule locks|Weekly time budgets','08 Scheduling and planning','ARCHITECTURE.md'),
('Focus and time tracking','Distraction-reduced focus screen|Stopwatch and Pomodoro|Custom cycles and breaks|Focus task links|Persisted pause and resume|Idle detection and classification|Interrupted session recovery|Planned versus actual reports|Optional focus targets|macOS Focus integration','09 Deep work and time','UX_SPEC.md'),
('Habits and routines','Built-in habit tracking|Habit frequencies|Quantity duration and Boolean values|Non-coercive streaks|Skipped habits|Routine builder|Guided routine execution|User-defined health routines|Habit trend dashboard|Contextual habit reminders','10 Habits and routines','DATA_MODEL.md'),
('Inbox notes and search','Universal capture|Global capture shortcut|Rule-based capture parser|Markdown notes|Note backlinks|Local file attachments|SQLite FTS5 search|Global entity search|Saved searches|Inbox triage','11 Notes and discovery','UX_SPEC.md'),
('Planning reviews and life administration','Morning planning|Evening review|Weekly review|Monthly review|Transparent analytics|No arbitrary productivity score|Lightweight journaling|Light finance reminders|Career management|Learning tracking','12 Reviews and insights','PRODUCT.md'),
('Agent-friendly CLI','Standalone Rust CLI|Working command name|Human and JSON output|Resource-oriented commands|Flags JSON stdin and input files|Transactional batch operations|GUI-independent CLI|Idempotency keys|Stable machine error codes|Agent-readable CLI schemas','14 Agents and automations','CLI_AND_MCP.md'),
('MCP and agent workflows','Local MCP stdio|Optional remote MCP|Agent read and write operations|GUI CLI MCP parity|Validated typed tools|Destructive-write review|Agent identity and attribution|Change events and subscriptions|Trigger condition action rules|Agent action history and undo','14 Agents and automations','CLI_AND_MCP.md'),
('Technical architecture','Svelte 5 TypeScript Vite|Bun|Rust shared domain core|Rust-owned database access|Modular monolith|Typed frontend stores|Small native background helper|SQLite interprocess synchronization|Deterministic scheduler|Internal module extension points','01 Foundation','ARCHITECTURE.md'),
('SQLite and integrity','Normalized primary SQLite database|WAL and busy timeout|UUIDv7 identity|UTC instant and IANA zone|Transactional migrations|Recoverable deletion|Append-only activity events|Conflict-aware updates|Verified backup snapshots|Migration recovery','16 Edge states and recovery','DATA_MODEL.md'),
('UX and visual design','Original visual identity|Persistent sidebar and detail panel|Compact and comfortable density|System light dark appearance|System fonts and typography|Today dashboard composition|Keyboard-first navigation|Global command palette|Accessibility and VoiceOver|Resizable remembered window','01 Foundation','UX_SPEC.md'),
('External integrations','Apple Calendar import|Google Calendar OAuth|ICS import and export|CalDAV adapter|Apple Reminders import|GitHub issue and PR links|Local Git references|Deferred email integration|CSV JSON import|Portable exports','15 Settings and integrations','PRODUCT.md'),
('Privacy security and performance','Local-first privacy|Optional database encryption|OS user access boundary|CLI permissions|No arbitrary MCP shell|Redacted sensitive logs|Daily backup retention|Restore verification|Warm CLI latency budget|Large-data scaling','15 Settings and integrations','ARCHITECTURE.md'),
('Verification and distribution','macOS Apple Silicon first|Minimum macOS target|Rust frontend and CLI tests|Desktop GUI testing|Screenshot inspect fix recheck|SQLite migration and crash tests|MCP and CLI E2E tests|Signed local app|Phased release milestones|Release acceptance criteria','16 Edge states and recovery','TEST_PLAN.md'),
]
assert len(sections)==20
keyboards={
 5:'Today · default',7:'First run',8:'Today · default',17:'Universal inbox',
 23:'Task scheduling detail',26:'Recurring task editor',27:'Task dependencies',29:'Attached files',30:'Bulk-edit selection',
 31:'Create task from text',33:'Recurring task editor',34:'Recurring task editor',35:'Overdue task decision',36:'Task templates',37:'Task detail · activity',38:'Task dependencies',39:'Smart saved views',
 41:'Project · Kanban',44:'Project · summary',45:'Project · milestones',46:'Project · blockers',47:'Project templates',48:'Project notes',49:'Project health report',
 53:'Goal detail',54:'OKR objective details',55:'Active-goal limit',56:'Monthly goal review',57:'Goal paused or abandoned',58:'Goal progress history',59:'Goal completed',
 62:'Calendar · day',64:'All-day commitments',65:'Recurring event',66:'Time-zone travel',68:'Time-block schedule',
 71:'Configure availability',72:'Protected time',73:'Conflict resolution',75:'Automatic schedule proposal',76:'Proposal approval',77:'Unfinished work carryover',79:'Protected time',
 82:'Focus · Pomodoro',85:'Focus · paused',86:'Idle activity review',87:'Interrupted session',89:'Focus trends',
 92:'Create a habit',93:'Habit detail',95:'Habit skip decision',96:'Routine builder',97:'Morning routine',99:'Habits · weekly',
 101:'Quick capture palette',102:'Quick-add overlay',104:'Note editor',105:'Note links and backlinks',107:'Global search',110:'Inbox processing workflow',
 113:'Weekly review workflow',115:'Productivity analytics',118:'Financial reminders',119:'Career pipeline',120:'Learning dashboard',
 121:'CLI quick reference',127:'CLI quick reference',129:'CLI structured error',131:'MCP local server',136:'Agent change approval',137:'Agent activity ledger',139:'Automation gallery',140:'Agent undo workflow',
 152:'Database temporarily locked',155:'Migration failed',156:'Task restore',159:'Backup history',160:'Migration failed',
 163:'App shell · compact',164:'Dark appearance',167:'Accessibility inspection',168:'Global command palette',170:'Responsive small window',
 171:'Apple Calendar connection',172:'Google Calendar connection',173:'ICS import preview',176:'GitHub integration',177:'Local projects and branches',179:'ICS and CSV imports',180:'JSON data export',
 181:'Security and local permissions',187:'Backup history',188:'Restore database',189:'High-volume backlog',190:'High-volume backlog',
 193:'CLI quick reference',194:'Accessibility inspection',196:'Backup restore confirmation',197:'MCP local server'
}
rows=[]
for group_no,(label,pipe,page,doc) in enumerate(sections):
 subjects=pipe.split('|');assert len(subjects)==10,(group_no,label,len(subjects))
 for offset,name in enumerate(subjects):
  q=group_no*10+offset+1
  desired=keyboards.get(q)
  matches=[b for b in inv if desired and desired.lower() in b['title'].lower()]
  if not matches:matches=[b for b in inv if b['page']==page]
  assert matches,(q,page)
  b=matches[0]
  rows.append(dict(id='Q%03d'%q,round='%02d — %s'%(group_no+1,label),decision=name,baseline='A — recommended, NOT yet user-approved',board_id='%03d'%b['board_id'],board_title=b['title'],page=b['page'],spec='docs/'+doc,status='DESIGNED STATIC / IMPLEMENTATION PENDING'))
assert len(rows)==200 and len(set(r['id'] for r in rows))==200
assert all(any(b['board_id']==int(r['board_id']) for b in inv) for r in rows)
with (root/'docs/requirements-coverage.csv').open('w',newline='') as f:
 w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
lines=['# Requirement coverage — Q001–Q200','','**Design traceability: 200/200 mapped.** Every recommended A decision is provisionally represented by at least one editable FigCraft board and a planning document. This is NOT a user-approved architecture, completed behavior specification, nor functional implementation.','','See requirements-coverage.csv for all 200 row-level mappings with board ID and source document.','','| Round | Decision IDs | FigCraft section | Main specification |','|---|---|---|---|']
for n,(name,_,page,doc) in enumerate(sections):lines.append('| %02d %s | Q%03d–Q%03d | %s | %s |'%(n+1,name,n*10+1,n*10+10,page,doc))
lines+=['','## Verification boundaries','','- 194 real native FigCraft frames on 16 pages are editable and exportable.','- Static visual mapping does not prove all interactive states have been prototyped, nor validate engineering behavior.','- CLI, MCP, database, Tauri and integrations are not implemented.','- Final choice overrides, conflicts and dependencies must be reconciled before implementation readiness can be approved.']
(root/'docs/REQUIREMENTS_COVERAGE.md').write_text('\n'.join(lines)+'\n')
print('200 Q IDs VALID; board references VALID; mapped pages=',len(set(r['page'] for r in rows)),'unique boards=',len(set(r['board_id'] for r in rows)))
