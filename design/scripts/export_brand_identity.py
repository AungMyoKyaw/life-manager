#!/usr/bin/env python3
"""Deterministic, FigCraft-first Life Manager brand asset export and validation."""
from __future__ import annotations
from concurrent.futures import ThreadPoolExecutor
from hashlib import sha256
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont, ImageStat
import json, subprocess, xml.etree.ElementTree as ET
import sys

ROOT=Path(__file__).resolve().parents[2]
DESIGN=ROOT/"design"
MASTER=DESIGN/"life-manager-master.figcraft"
BRAND=DESIGN/"brand"
OUT=BRAND/"exports"
PREVIEW=BRAND/"previews"
OUT.mkdir(parents=True,exist_ok=True)
PREVIEW.mkdir(parents=True,exist_ok=True)

PAGE="17 Brand Identity"
# name, output file, file format, raster scale
JOBS=[
    ("LM · Concept A · Convergent Bands","../previews/concept-a.png","png",0.7),
    ("LM · Concept B · Offset Channels","../previews/concept-b.png","png",0.7),
    ("LM · Concept C · Joined Modules","../previews/concept-c.png","png",0.7),
    ("LM · Final Brand System","../previews/concept-final.png","png",0.7),
    ("LM · App Icon · 1024","LifeManager-1024.png","png",1),
    ("LM · App Icon · 1024","life-manager-app-icon.svg","svg",1),
    ("LM · App Icon · 1024","life-manager-app-icon.pdf","pdf",1),
    ("LM · Standalone · 512","life-manager-symbol.svg","svg",1),
    ("LM · Standalone · 512","life-manager-symbol.png","png",1),
    ("LM · Horizontal Wordmark · 1200","life-manager-wordmark.svg","svg",1),
    ("LM · Horizontal Wordmark · 1200","life-manager-wordmark.png","png",1),
    ("LM · Horizontal Wordmark · 1200","life-manager-wordmark.pdf","pdf",1),
    ("LM · Dark · 512","life-manager-dark.png","png",1),
    ("LM · Black · 512","life-manager-black.svg","svg",1),
    ("LM · Black · 512","life-manager-black.png","png",1),
    ("LM · White · 512","life-manager-white.svg","svg",1),
    ("LM · White · 512","life-manager-white.png","png",1),
    ("LM · Menu Bar · 128","life-manager-menubar.png","png",1),
]
def export(job):
    node,dest,fmt,scale=job
    path=(OUT/dest).resolve()
    cmd=["figcraft-cli","export",str(MASTER),"--page",PAGE,
         "--node",node,"--format",fmt,"--out",str(path)]
    if fmt=="png":cmd+=["--scale",str(scale)]
    r=subprocess.run(cmd,capture_output=True,text=True,timeout=90)
    if r.returncode or not path.is_file():
        raise RuntimeError(f"{node} -> {path}: {r.returncode} {r.stderr[-600:]}")
    return str(path.relative_to(ROOT))

def luminance(c):
    rgb=tuple(int(c[i:i+2],16)/255 for i in (1,3,5))
    a=tuple(x/12.92 if x <=0.04045 else ((x+0.055)/1.055)**2.4 for x in rgb)
    return 0.2126*a[0]+0.7152*a[1]+0.0722*a[2]
def contrast(a,b):
    l1,l2=sorted((luminance(a),luminance(b)),reverse=True)
    return (l1+0.05)/(l2+0.05)

def main():
    if not MASTER.is_file():raise SystemExit("Missing FigCraft master.")
    info=json.loads(subprocess.check_output(["figcraft-cli","info",str(MASTER),"--json"],text=True))
    assert len(info["pages"])==17 and info["pages"][-1]["name"]==PAGE, "Brand page missing"
    assert len(info["pages"][-1]["frames"])==11, "Brand asset frame count changed"
    assert not info["missingFonts"], "Missing wordmark fonts"
    with ThreadPoolExecutor(max_workers=3) as pool:
        outputs=list(pool.map(export,JOBS))
    # App icon native geometry has transparent squircle corners.
    icon=Image.open(OUT/"LifeManager-1024.png").convert("RGBA")
    assert icon.size==(1024,1024),icon.size
    assert icon.getpixel((0,0))[3]==0, f"App icon corners are not transparent: {icon.getpixel((0,0))}"
    assert icon.getpixel((512,512))[3]>240
    # Seven approved PNG sizes. The FigCraft 1024 export is the true source.
    sizes=(1024,512,256,128,64,32,16)
    for px in sizes:
        dest=OUT/f"LifeManager-{px}.png"
        if px!=1024:
            icon.resize((px,px),Image.Resampling.LANCZOS).save(dest,optimize=True)
        assert Image.open(dest).size==(px,px)
    # Standard Apple iconset mapping; use the verified FigCraft render as source.
    iconset=OUT/"LifeManager.iconset"
    iconset.mkdir(exist_ok=True)
    mapping={"icon_16x16.png":16,"icon_16x16@2x.png":32,
             "icon_32x32.png":32,"icon_32x32@2x.png":64,
             "icon_128x128.png":128,"icon_128x128@2x.png":256,
             "icon_256x256.png":256,"icon_256x256@2x.png":512,
             "icon_512x512.png":512,"icon_512x512@2x.png":1024}
    for fname,px in mapping.items():
        (Image.open(OUT/f"LifeManager-{px}.png")).save(iconset/fname,optimize=True)
    subprocess.run(["iconutil","-c","icns","-o",str(OUT/"LifeManager.icns"),str(iconset)],
                   check=True,capture_output=True,text=True,timeout=40)
    outputs.extend([str((OUT/f"LifeManager-{px}.png").relative_to(ROOT)) for px in sizes])
    outputs.append(str((OUT/"LifeManager.icns").relative_to(ROOT)))
    # Small-size and background inspection specimens (actual pixel grids).
    samples=Image.new("RGBA",(1010,390),"#F6F7F5")
    d=ImageDraw.Draw(samples)
    d.text((32,21),"LIFE MANAGER  /  PIXEL & SURFACE TEST",fill="#19292A")
    x=24
    for px in (16,24,32,64,128,256):
        render=icon.resize((px,px),Image.Resampling.LANCZOS)
        # Nearest enlargement preserves literal small pixel structure.
        tile_size=112
        cell=Image.new("RGBA",(tile_size,tile_size),"#DEE9E4")
        if px<=64:
            zoom=Image.Resampling.NEAREST
            w=min(100,px*max(1,100//px))
            render=render.resize((w,w),zoom)
        else: render=render.resize((100,100),Image.Resampling.LANCZOS)
        cell.alpha_composite(render,((112-render.width)//2,(112-render.height)//2))
        samples.alpha_composite(cell,(x,74))
        d.text((x+6,195),f"{px}px",fill="#19292A")
        x+=161
    for j,bg in enumerate(("#F6F7F5","#162929","#FFFFFF","#266C56","#C7D1CD")):
        cx=30+j*193
        d.rounded_rectangle((cx,242,cx+180,360),radius=12,fill=bg)
        small=Image.open(OUT/"life-manager-symbol.png").convert("RGBA").resize((104,104),Image.Resampling.LANCZOS)
        if bg in ("#162929","#266C56"):
            small=Image.open(OUT/"life-manager-white.png").convert("RGBA").resize((104,104),Image.Resampling.LANCZOS)
        samples.alpha_composite(small,(cx+38,249))
    samples.convert("RGB").save(PREVIEW/"size-and-surface-test.jpg","JPEG",quality=90,optimize=True)
    # Concept comparison in a single contact sheet.
    sheet=Image.new("RGB",(1440,326),"#DFE6E2")
    for i,c in enumerate("abc"):
        im=Image.open(PREVIEW/f"concept-{c}.png").convert("RGB")
        im.thumbnail((476,310),Image.Resampling.LANCZOS)
        sheet.paste(im,(i*480+2,8))
    sheet.save(PREVIEW/"concept-comparison.jpg","JPEG",quality=88,optimize=True)
    # Test vector/PDF integrity and metrics; distinguish logo from UI contrast.
    for name in ("life-manager-app-icon.svg","life-manager-symbol.svg",
                 "life-manager-wordmark.svg","life-manager-black.svg","life-manager-white.svg"):
        p=OUT/name
        root=ET.parse(p).getroot()
        assert root.tag.endswith("svg"), name
        assert p.stat().st_size>200, name
    for name in ("life-manager-app-icon.pdf","life-manager-wordmark.pdf"):
        b=(OUT/name).read_bytes()
        assert b.startswith(b"%PDF") and b"%%EOF" in b[-128:],name
    colors={"accent_on_light":contrast("#266C56","#F6F7F5"),
            "mint_on_dark":contrast("#B7D7C8","#162929")}
    assert min(colors.values())>=3,colors
    rgba=lambda p: Image.open(OUT/p).convert("RGBA")
    assert rgba("life-manager-symbol.png").getpixel((0,0))[3]==0
    assert rgba("life-manager-menubar.png").getpixel((0,0))[3]==0
    assert all(Image.open(OUT/f"LifeManager-{n}.png").size==(n,n) for n in sizes)
    assert len(set((OUT/f"LifeManager-{n}.png").read_bytes() for n in sizes))==7
    def fileinfo(path):
        p=ROOT/path
        return {"file":str(path),"bytes":p.stat().st_size,"sha256":sha256(p.read_bytes()).hexdigest()}
    finals=sorted(set(outputs))
    manifest={
        "status":"produced_and_automatically_validated",
        "source_master":str(MASTER.relative_to(ROOT)),
        "selected_concept":"B — Offset Channels",
        "source_design_page":PAGE,
        "counts":{"original_app_pages":16,"app_frames":224,"brand_frames":11,"total_pages":17},
        "contrast_ratios":{k:round(v,2) for k,v in colors.items()},
        "native_editable":True,
        "app_icon_transparent_corners":True,
        "font_missing":info["missingFonts"],
        "exported": [fileinfo(Path(x)) for x in finals]
    }
    (BRAND/"asset-manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
    print(json.dumps({"files":len(finals),"pages":17,"brand_frames":11,
         "colors":manifest["contrast_ratios"],"icns_size":(OUT/"LifeManager.icns").stat().st_size,
         "app_corner_alpha":icon.getpixel((0,0))[3]},indent=2))
if __name__=="__main__":main()
