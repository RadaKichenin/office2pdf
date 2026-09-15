#!/usr/bin/env python3
"""Legend row of a native Excel export: key rect height, sample stroke width, and the vector marker box, in chart points (page pt / scale)."""
import subprocess, sys, os, xml.etree.ElementTree as ET
def mat(s):
    v=[float(x) for x in s.split()]; return v if len(v)==6 else v+[0.0,0.0]
def run(pdf,page,scale):
    out=subprocess.run(["mutool","draw","-F","trace","-o","-",pdf,str(page)],capture_output=True,text=True)
    root=ET.fromstring(out.stdout); els=[]; keys=[]; labels=[]
    for el in root.iter():
        if el.tag in ("fill_path","stroke_path"):
            m=mat(el.get("transform")) if el.get("transform") else [1,0,0,1,0,0]
            xs=[];ys=[]
            for p in el.iter():
                if p.tag in ("moveto","lineto","curveto") and p.get("x") is not None:
                    xs.append(m[0]*float(p.get("x"))+m[2]*float(p.get("y"))+m[4]); ys.append(m[1]*float(p.get("x"))+m[3]*float(p.get("y"))+m[5])
                if p.tag=="curveto":
                    for a,b in (("x1","y1"),("x2","y2"),("x3","y3")):
                        if p.get(a): xs.append(m[0]*float(p.get(a))+m[2]*float(p.get(b))+m[4]); ys.append(m[1]*float(p.get(a))+m[3]*float(p.get(b))+m[5])
            if not xs: continue
            b=(min(xs)/scale,min(ys)/scale,max(xs)/scale,max(ys)/scale)
            els.append((el.tag,b,el.get("linewidth")))
            if el.tag=="fill_path" and abs(b[2]-b[0]-19.2)<0.05 and b[3]-b[1]<40: keys.append(b)
        elif el.tag=="fill_text":
            m=mat(el.get("transform"))
            for span in el:
                trm=mat(span.get("trm")) if span.get("trm") else [1,0,0,1,0,0]
                gs=list(span)
                if gs: labels.append(("".join(g.get("unicode") or "" for g in gs),(m[0]*float(gs[0].get("x"))+m[4])/scale,(m[3]*float(gs[0].get("y"))+m[5])/scale,trm[0]/scale))
    if not keys: print(os.path.basename(pdf),"no keys"); return
    ky0=min(k[1] for k in keys)-25; ky1=max(k[3] for k in keys)+25
    kh=keys[0][3]-keys[0][1]
    lab=[l for l in labels if ky0<=l[2]<=ky1 and l[1]>keys[0][0]]
    fs=lab[0][3] if lab else float('nan')
    line=[(b,lw) for t,b,lw in els if t=="stroke_path" and abs(b[2]-b[0]-19.2)<0.05 and ky0<=b[1]<=ky1 and b[0]>keys[-1][2]]
    mark=[b for t,b,lw in els if t=="fill_path" and ky0<=b[1] and b[3]<=ky1 and b[0]>keys[-1][2] and (b[2]-b[0])<19]
    for b,lw in line: print(f"{os.path.basename(pdf):32s} legend_font={fs:.3f} key_h={kh:.3f} line_y={b[1]:.3f} lw={lw} marker={[(round(m[2]-m[0],3),round(m[3]-m[1],3),round(m[0],3),round(m[1],3)) for m in mark]}")
scale=float(sys.argv[1])
for pdf in sys.argv[2:]: run(pdf,2,scale)
