#!/usr/bin/env python3
"""Export every FigCraft frame for visual QA and build per-page contact sheets."""
import pathlib,subprocess,json,re,collections,concurrent.futures,os,sys
from PIL import Image,ImageOps,ImageDraw,ImageFont,ImageStat
ROOT=pathlib.Path(__file__).resolve().parents[2]
DES=ROOT/'design'; master=DES/'life-manager-master.figcraft'
index=json.load(open(DES/'screen-inventory.json'))
groups=collections.defaultdict(list)
for e in index:groups[e['page']].append(e)

def export_group(item):
    page,items=item
    slug=page.lower().replace(' ','-')
    dest=DES/'previews'/slug
    dest.mkdir(parents=True,exist_ok=True)
    p=subprocess.run(['figcraft-cli','export',str(master),'--page',page,'--out',str(dest),'--scale','0.5'],capture_output=True,text=True,timeout=180)
    files=list(dest.glob('*.png'))
    return page,len(files),p.returncode,p.stderr[-280:]

def contact_sheet(page,items):
    slug=page.lower().replace(' ','-'); dest=DES/'previews'/slug
    images=[]; errors=[]
    for e in items:
        name=f"{e['board_id']:03d} · {e['title']}.png"
        p=dest/name
        if not p.exists():errors.append(name);continue
        e['preview']=str(p.relative_to(DES))
        im=Image.open(p).convert('RGB')
        if im.size!=(720,450):errors.append(name+' wrong dimensions '+str(im.size))
        images.append((e,im))
    thumbW,thumbH=480,300
    cellW,cellH=500,339
    rows=(len(images)+2)//3
    sheet=Image.new('RGB',(cellW*3,cellH*rows+52),(227,234,231))
    d=ImageDraw.Draw(sheet)
    d.rectangle((0,0,sheet.width,52),fill=(22,41,41))
    d.text((18,16),f'{page.upper()}    /    {len(images)} SCREENS',fill=(242,250,246))
    for i,(entry,im) in enumerate(images):
        x=(i%3)*cellW;y=52+(i//3)*cellH
        sheet.paste(im.resize((thumbW,thumbH),Image.Resampling.LANCZOS),(x+10,y+9))
        d.text((x+12,y+311),f"{entry['board_id']:03d}  {entry['title'][:53]}",fill=(35,53,53))
    dest=DES/'contact-sheets';dest.mkdir(parents=True,exist_ok=True)
    path=dest/(slug+'.jpg');sheet.save(path,quality=81,optimize=True)
    return errors,path

def main():
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as ex:
        result=list(ex.map(export_group,groups.items()))
    errors=[]
    for page,count,code,err in result:
        if code or count!=len(groups[page]):errors.append(f'{page} export={count}/{len(groups[page])} code={code} {err}')
    for page,items in groups.items():
        err,_=contact_sheet(page,items);errors+=err
    with open(DES/'screen-inventory.json','w') as f:json.dump(index,f,indent=2,ensure_ascii=False)
    report={'pages':len(groups),'boards':len(index),'exported':sum(x[1] for x in result),'contact_sheets':len(list((DES/'contact-sheets').glob('[0-9][0-9]-*.jpg'))),'errors':errors}
    with open(DES/'visual-qa.json','w') as f:json.dump(report,f,indent=2)
    print(json.dumps(report,indent=2)[:5000])
    if errors:sys.exit(1)
if __name__=='__main__':main()
