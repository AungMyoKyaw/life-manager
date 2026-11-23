#!/usr/bin/env python3
"""Life Manager brand page generator. Append vector logo artwork to the canonical
design JSON after the shared token-binding pass; rebuild_master.py invokes this
so future FigCraft regenerations do not erase the identity page.
"""
from __future__ import annotations
import json
from pathlib import Path

import tokens
from build_figcraft import Doc
from svgpath import path_network

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "design/life-manager-master.json"
BRAND = ROOT / "design/brand"
PAGE = "17 Brand Identity"

# Each symbol is authored as independent closed vector contours in a 120x120
# coordinate system. True enclosed vector regions survive SVG/PDF export.
CONCEPTS = {
    "A": {
        "title": "Convergent Bands",
        "tag": "FLOW / CONTINUITY",
        "description": "Two deliberate ribbons move from separate origins toward",
        "description2": "one shared direction. Calm, focused, confidently minimal.",
        "paths": [
            "M 18 21 C 18 16 22 12 28 12 C 53 12 57 38 74 38 H 95 C 102 38 107 43 107 50 C 107 57 102 62 95 62 H 73 C 48 62 46 36 27 36 C 22 36 18 30 18 25 Z",
            "M 18 99 C 18 104 22 108 28 108 C 53 108 57 82 74 82 H 95 C 102 82 107 77 107 70 C 107 63 102 58 95 58 H 73 C 48 58 46 84 27 84 C 22 84 18 90 18 95 Z",
        ],
    },
    "B": {
        "title": "Offset Channels",
        "tag": "FOCUS / OPENNESS",
        "description": "Two independent channels change direction without",
        "description2": "colliding. An open center lets the mind find its way.",
        "paths": [
            "M 24 12 C 31 12 36 17 36 24 V 61 C 36 74 45 81 59 81 H 72 C 79 81 84 86 84 93 C 84 100 79 105 72 105 H 59 C 31 105 12 87 12 61 V 24 C 12 17 17 12 24 12 Z",
            "M 96 108 C 89 108 84 103 84 96 V 59 C 84 46 75 39 61 39 H 48 C 41 39 36 34 36 27 C 36 20 41 15 48 15 H 61 C 89 15 108 33 108 59 V 96 C 108 103 103 108 96 108 Z",
        ],
    },
    "C": {
        "title": "Joined Modules",
        "tag": "STRUCTURE / AGENCY",
        "description": "Three purposeful pieces arrange a considered place",
        "description2": "to focus. Modular, architectural and structurally stable.",
        "paths": [
            "M 20 12 H 47 C 55 12 60 17 60 24 C 60 31 55 36 47 36 H 36 V 47 C 36 55 31 60 24 60 C 17 60 12 55 12 47 V 20 C 12 15 15 12 20 12 Z",
            "M 75 12 H 100 C 105 12 108 15 108 20 V 48 C 108 55 103 60 96 60 C 89 60 84 55 84 48 V 36 H 75 C 68 36 63 31 63 24 C 63 17 68 12 75 12 Z",
            "M 26 76 C 26 69 31 64 38 64 H 82 C 89 64 94 69 94 76 V 100 C 94 105 91 108 86 108 H 34 C 29 108 26 105 26 100 Z",
        ],
    },
}
SELECTED = "B"
EXPORTS = {
    "app_icon": "LM · App Icon · 1024",
    "standalone": "LM · Standalone · 512",
    "horizontal": "LM · Horizontal Wordmark · 1200",
    "dark": "LM · Dark · 512",
    "black": "LM · Black · 512",
    "white": "LM · White · 512",
    "menu": "LM · Menu Bar · 128",
}

def main():
    obj = json.loads(SOURCE.read_text())
    # Idempotent: discard the previously generated brand page, if any.
    old_pages = [pid for pid in obj["pages"] if obj["nodes"][str(pid)]["name"] == PAGE]
    if old_pages:
        doomed = set()
        def walk(nid):
            if nid in doomed: return
            doomed.add(nid)
            for child in obj["nodes"][str(nid)].get("children", []):
                walk(child)
        for pid in old_pages: walk(pid)
        obj["pages"] = [p for p in obj["pages"] if p not in old_pages]
        for pid in doomed: obj["nodes"].pop(str(pid), None)
    d = Doc()
    d.j = obj
    d.n = max(int(obj.get("next_id",1)), max(int(x) for x in obj["nodes"]) + 1)
    vars_by_name = {v["name"]:v for v in obj["variables"]["variables"]}
    colors = {name:spec[0] for name,spec in tokens.COLORS.items()}

    def bound(nid, token):
        if token:
            d.j["nodes"][str(nid)].setdefault("bound", {})["fills/0/color"] = vars_by_name[token]["id"]
        return nid

    def rect(parent,x,y,w,h,token="surface",r=0,name="Surface"):
        return bound(d.rect(parent,x,y,w,h,colors[token],r,name), token)

    def text(parent,s,x,y,w,h,size=18,token="text",weight="Regular",name=None,spacing=None):
        nid=d.text(parent,s,x,y,w,h,size,colors[token],weight,name)
        if spacing is not None:
            d.j["nodes"][str(nid)]["style"]["letter_spacing"]={"unit":"percent","value":spacing}
        return bound(nid,token)

    def frame(parent,name,x,y,w=1440,h=900,bg="canvas",transparent=False):
        n={"type":"frame","name":name,"width":float(w),"height":float(h),
           "children":[],"clip":True,"transform":[1.0,0.0,0.0,1.0,float(x),float(y)]}
        if not transparent: n["fills"]=tokens.paint(colors[bg])
        nid=d.add(n,parent)
        if not transparent: bound(nid,bg)
        return nid

    def mark(parent,kind,x,y,size,token="accent",prefix="Brand symbol"):
        k=CONCEPTS[kind]
        for i,path in enumerate(k["paths"],1):
            net,_=path_network(path,normalize=False)
            net["regions"]=[{"loops":[list(range(len(net["segments"])))],"winding":"nonZero"}]
            n={"type":"vector","name":f"{prefix} · {kind} · contour {i}",
               "network":net,"width":120.0,"height":120.0,
               "fills":tokens.paint(colors[token]),"strokes":[],
               "transform":[size/120.0,0,0,size/120.0,float(x),float(y)]}
            bound(d.add(n,parent),token)

    def line(parent,x,y,w,color="border"):
        rect(parent,x,y,w,1,color,0,"Hairline")

    def wordmark(parent,x,y,token="text",size=46,width=390):
        text(parent,"Life Manager",x,y,width,size*1.35,size,token,"Medium","Life Manager · primary wordmark",spacing=-0.6)

    def icon_square(parent,kind,x,y,sz=254):
        rect(parent,x,y,sz,sz,"surface-inverse",sz*0.225,"macOS app icon · squircle")
        padding=sz*0.17
        mark(parent,kind,x+padding,y+padding,sz-2*padding,"accent-quiet","App icon geometry")

    page=d.page(PAGE)
    # All concept frames are 1440x900, arranged in a horizontal row like
    # an art-director's comparative contact sheet.
    for index,(kind,k) in enumerate(CONCEPTS.items()):
        f=frame(page,f"LM · Concept {kind} · {k['title']}",index*1560,0)
        rect(f,0,0,1440,11,"accent",0,"Top identity rule")
        text(f,"LIFE MANAGER  /  BRAND IDENTITY",84,66,780,24,14,"text-secondary","Bold",spacing=2)
        text(f,"CONCEPT "+kind,84,140,470,90,74,"text","Medium",spacing=-1.3)
        text(f,k["title"],86,236,580,70,48,"text","Regular",spacing=-0.7)
        text(f,k["tag"],88,321,510,24,13,"accent","Bold",spacing=2)
        text(f,k["description"],88,393,630,32,21,"text-secondary")
        text(f,k["description2"],88,426,630,32,21,"text-secondary")
        line(f,86,490,595)
        text(f,"OPEN CENTER    ·    EDITABLE VECTORS    ·    FLAT COLOR",86,515,680,22,13,"text-secondary","Medium",spacing=0.45)
        # Hero on clear background, illustration large enough to examine curves.
        rect(f,826,94,514,570,"surface",32,"Concept sample / white")
        mark(f,kind,932,174,310,"accent")
        wordmark(f,922,529,"text",39,390)
        # Icon treatment beneath the conceptual notes
        icon_square(f,kind,86,619,198)
        rect(f,306,619,196,198,"surface",20,"Standalone on white")
        mark(f,kind,335,650,139,"accent")
        rect(f,524,619,196,198,"surface-inverse",20,"Monochrome on dark")
        mark(f,kind,553,650,139,"accent-quiet")
        text(f,"ICON",87,837,180,22,12,"text-secondary","Bold",spacing=1)
        text(f,"LIGHT",308,837,180,22,12,"text-secondary","Bold",spacing=1)
        text(f,"DARK",526,837,180,22,12,"text-secondary","Bold",spacing=1)
        line(f,818,715,524)
        text(f,"CUSTOM GEOMETRY   /   NATIVE FIGCRAFT",826,738,524,28,13,"text-secondary","Medium",spacing=0.8)
        text(f,"120 UNIT SYSTEM / OPTICAL ADJUSTMENT",826,772,524,28,13,"text-muted")
        text(f,"LIFE MANAGER BRAND EXPLORATION 2026",826,840,524,20,11,"text-muted")

    # Master selection board; selected 'B' after evaluating the relative
    # distinctiveness and small-scale open center.
    winner=CONCEPTS[SELECTED]
    f=frame(page,"LM · Final Brand System",4680,0)
    rect(f,0,0,1440,12,"accent",0,"Top identity rule")
    text(f,"IDENTITY SYSTEM  /  SELECTED DIRECTION",78,56,700,24,14,"accent","Bold",spacing=1.4)
    text(f,"Offset Channels",78,112,690,85,68,"text","Medium",spacing=-1.1)
    text(f,"A compact mark about independent paths, human agency and clarity.",82,208,875,36,22,"text-secondary")
    rect(f,78,298,618,460,"surface",30,"Primary identity / white")
    mark(f,SELECTED,154,351,312,"accent")
    wordmark(f,126,668,"text",45,470)
    rect(f,728,298,628,460,"surface-inverse",30,"Dark identity system")
    mark(f,SELECTED,765,356,280,"accent-quiet")
    wordmark(f,1040,455,"text-inverse",40,308)
    text(f,"MACOS / LIGHT",84,777,310,22,12,"text-secondary","Bold",spacing=1.3)
    text(f,"DARK APPEARANCE",736,777,315,22,12,"text-secondary","Bold",spacing=1.3)
    line(f,78,824,1278)
    text(f,"DESIGN SOURCE · ONE FIGCRAFT MASTER · REUSABLE EDITABLE PATHS",78,844,1280,22,13,"text-secondary","Medium",spacing=0.3)

    x0=6240
    app=frame(page,EXPORTS["app_icon"],x0,0,1024,1024,transparent=True)
    # Explicit filled vector squircle retains transparent corner alpha.
    squircle='M 44 0 H 76 C 105 0 120 15 120 44 V 76 C 120 105 105 120 76 120 H 44 C 15 120 0 105 0 76 V 44 C 0 15 15 0 44 0 Z'
    net,_=path_network(squircle,normalize=False)
    net['regions']=[{'loops':[list(range(len(net['segments'])))],'winding':'nonZero'}]
    bg=d.add({'type':'vector','name':'App icon background · editable squircle',
              'network':net,'width':120.0,'height':120.0,'fills':tokens.paint(colors['surface-inverse']),
              'strokes':[],'transform':[1024/120,0,0,1024/120,0.0,0.0]},app)
    bound(bg,'surface-inverse')
    mark(app,SELECTED,156,156,712,"accent-quiet","Application icon mark")

    single=frame(page,EXPORTS["standalone"],x0+1160,0,512,512,transparent=True)
    mark(single,SELECTED,27,27,458,"accent","Primary standalone mark")
    horizontal=frame(page,EXPORTS["horizontal"],x0+1820,0,1200,300,transparent=True)
    mark(horizontal,SELECTED,35,25,250,"accent","Wordmark symbol")
    wordmark(horizontal,322,80,"text",100,855)
    dark=frame(page,EXPORTS["dark"],x0+1820,420,512,512,"surface-inverse")
    mark(dark,SELECTED,27,27,458,"accent-quiet","Dark mark")
    black=frame(page,EXPORTS["black"],x0+2450,420,512,512,transparent=True)
    mark(black,SELECTED,27,27,458,"text","Black mark")
    white=frame(page,EXPORTS["white"],x0+3080,420,512,512,transparent=True)
    mark(white,SELECTED,27,27,458,"text-inverse","White mark")
    menu=frame(page,EXPORTS["menu"],x0+3330,0,128,128,transparent=True)
    # Simplified small-scale version keeps both channel silhouettes.
    mark(menu,SELECTED,4,4,120,"text","Menu bar mark")

    d.j["next_id"]=d.n
    SOURCE.write_text(json.dumps(d.j,ensure_ascii=False,separators=(",",":")))
    BRAND.mkdir(parents=True,exist_ok=True)
    (BRAND/"concepts.json").write_text(json.dumps({
        "page":PAGE,"selected":SELECTED,
        "selection_reason":"Offset Channels keeps a recognizable open-center silhouette and avoids checkmark/calendar symbolism while creating the strongest distinctive counterflow.",
        "concepts":{k:{"title":v["title"],"tag":v["tag"],"paths":v["paths"]} for k,v in CONCEPTS.items()},
        "frames":EXPORTS,"source":"design/scripts/build_brand_identity.py",
        "authoritative_master":"design/life-manager-master.figcraft",
        "tokens":{"primary":"accent","dark_icon":"accent-quiet","icon_background":"surface-inverse"}
    },indent=2)+"\n")
    print(f"BRAND: added page {PAGE}, three concepts, one selected brand system, {len(EXPORTS)} export frames; total nodes {d.n-1}")

if __name__=="__main__": main()
