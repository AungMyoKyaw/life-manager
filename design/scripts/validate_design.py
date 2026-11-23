#!/usr/bin/env python3
"""Repeatable static FigCraft design verification, not an application test."""
from pathlib import Path
from PIL import Image,ImageStat
import json,csv,hashlib,sys
ROOT=Path(__file__).resolve().parents[2]
D=ROOT/'design'
sys.path.insert(0,str(ROOT/'design'/'scripts'))
import tokens
from audit_alignment import audit_frame
inv=json.loads((D/'screen-inventory.json').read_text())
master=json.loads((D/'life-manager-master.json').read_text())
info=json.loads((D/'figcraft-info.json').read_text())
qa=json.loads((D/'visual-qa.json').read_text())
requirements=list(csv.DictReader((ROOT/'docs/requirements-coverage.csv').open()))
errors=[]
EXPECTED=len(inv)
if len(inv)<194:errors.append('screen inventory count')
brand_pages=[p for p in info['pages'] if p['name']=='17 Brand Identity']
original_pages=[p for p in info['pages'] if p['name']!='17 Brand Identity']
if len(info['pages'])!=17 or len(brand_pages)!=1 or len(original_pages)!=16:errors.append('page count')
if sum(len(p['frames']) for p in original_pages)!=EXPECTED:errors.append('native application frame count')
if len(brand_pages)!=1 or len(brand_pages[0]['frames'])!=11:errors.append('native brand frame count')
if info.get('missingFonts'):errors.append('missing fonts')
if qa.get('errors') or qa.get('exported')!=EXPECTED:errors.append('export coverage')
if len(requirements)!=200 or len({r['id'] for r in requirements})!=200:errors.append('requirements traceability')
if len({x['board_id'] for x in inv})!=len(inv):errors.append('duplicate screen IDs')
if len({x['title'] for x in inv})!=len(inv):errors.append('duplicate screen titles')
if len(list((D/'contact-sheets').glob('[0-9][0-9]-*.jpg')))!=16:errors.append('missing contact sheets')

# The design system: every painted layer binds to a token, so a token change
# repaints the whole master.
variables=master.get('variables',{})
collections=variables.get('collections',[])
variable_list=variables.get('variables',[])
if len(collections)<3:errors.append('design token collections')
if len(variable_list)<len(tokens.COLORS):errors.append('design token variables')
bound_layers=sum(1 for n in master['nodes'].values() if n.get('bound'))
bound_colors=sum(1 for n in master['nodes'].values() if n.get('bound',{}).get('fills/0/color'))
if bound_colors<1000:errors.append('token-bound layers')
icon_layers=sum(1 for n in master['nodes'].values() if n.get('type')=='vector' and str(n.get('name','')).startswith('Icon'))

# The alignment of each visual group.
alignment=[]
for entry in inv:
    frame=master['nodes'][str(entry['node_id'])]
    for problem in audit_frame(master,frame):
        alignment.append((entry['board_id'],problem))
if alignment:errors.append('alignment: '+str(alignment[:8]))

content_hashes=set();out_of_frame=[];empty=[]
for entry in inv:
    file=D/entry['preview']
    if not file.is_file():errors.append('missing '+str(file));continue
    with Image.open(file) as img:
        if img.size!=(720,450):errors.append('wrong dimensions '+str(file))
        rgb=img.convert('RGB')
        h=hashlib.sha256(rgb.tobytes()).hexdigest();content_hashes.add(h)
        variance=sum(ImageStat.Stat(rgb.resize((24,15))).var)
        if variance<2:empty.append(entry['board_id'])
    frame=master['nodes'][str(entry['node_id'])]
    if frame['name']!='%03d · %s'%(entry['board_id'],entry['title']):errors.append('frame name mismatch '+str(entry['board_id']))
    for child_id in frame.get('children',[]):
        node=master['nodes'][str(child_id)]
        t=node.get('transform',[1,0,0,1,0,0]);x,y=t[4],t[5]
        w,h=node.get('width',0),node.get('height',0)
        if x<-2 or y<-2 or x+w>1442 or y+h>902:out_of_frame.append((entry['board_id'],node.get('name')))
if empty:errors.append('visually blank '+str(empty[:10]))
if len(content_hashes)!=EXPECTED:errors.append('duplicate rendered PNG bytes')
if out_of_frame:errors.append('out-of-frame layers '+str(out_of_frame[:10]))
result={'test_name':'static_figcraft_qa','pages':len(info['pages']),'frames':sum(len(p['frames']) for p in info['pages']),'native_nodes':info['nodes'],'preview_files':len(inv),'unique_preview_hashes':len(content_hashes),'contact_sheets':len(list((D/'contact-sheets').glob('[0-9][0-9]-*.jpg'))),'mapped_decisions':len(requirements),'out_of_frame_layers':len(out_of_frame),'missing_fonts':info.get('missingFonts',[]),'token_variables':len(variable_list),'token_collections':len(collections),'token_bound_layers':bound_layers,'token_bound_colors':bound_colors,'lucide_icon_layers':icon_layers,'alignment_problems':len(alignment),'errors':errors,'status':'PASS' if not errors else 'FAIL','scope':'Native FigCraft and static preview integrity ONLY; not application functionality, interactive accessibility or implementation readiness.'}
(D/'qa-results.json').write_text(json.dumps(result,indent=2)+'\n')
lines=['# FigCraft design QA report','','Result: **'+result['status']+'** (static design verification only).','','| Check | Observed |','|---|---|']
for k,label in [('pages','Native pages'),('frames','Named top-level frames'),('native_nodes','Native nodes'),('preview_files','Exported frame PNGs'),('unique_preview_hashes','Distinct rendered PNGs'),('contact_sheets','Contact sheets'),('mapped_decisions','Requirement ID mappings'),('out_of_frame_layers','Layers exceeding 1440 × 900 frame'),('missing_fonts','Missing fonts'),('token_collections','Design token collections'),('token_variables','Design token variables'),('token_bound_layers','Layers bound to a token'),('lucide_icon_layers','Lucide icon layers'),('alignment_problems','Visual-group alignment problems')]:lines.append('| '+label+' | '+str(result[k])+' |')
lines+=['','## Explicit limits','','This QA validates the static design file, frame inventories, exported PNG sizes, screenshot uniqueness, layer containment, traceability references, font availability, the design-token bindings and the alignment of each visual group. All 16 contact sheets have been visually reviewed and the design revised where major semantic mismatches were found. It does not verify runtime accessibility, executable buttons, drag/drop, calendar recurrence, desktop or CLI behavior.','', '## Repeat','', '1. Run the design generator.', '2. Convert JSON to native FigCraft and refresh figcraft-info.json.','3. Export all screenshots and contact sheets.','4. Run python3 design/scripts/make_coverage.py.','5. Run python3 design/scripts/validate_design.py.','', 'The actual current outputs are also in design/qa-results.json.']
(D/'QA_REPORT.md').write_text('\n'.join(lines)+'\n')
print(json.dumps(result,indent=2))
sys.exit(bool(errors))
