import math

WALLS = (250, 1566, 70, 1054)
CROSS = (783, 924, 435, 576)
CAR = (1000, 608)
BALLS = [(438,186),(365,865),(424,721),(363,179)]
NEARWALL = [(300,560),(320,140),(280,930),(1500,560)]  # left-50, corner, bottom-left, right-50
ALL = BALLS + NEARWALL

def dtt(p,t): return (p[0]-t[0])**2+(p[1]-t[1])**2
def dtr(p,b):
    minx,maxx,miny,maxy=b
    dx=minx-p[0] if p[0]<minx else (p[0]-maxx if p[0]>maxx else 0)
    dy=miny-p[1] if p[1]<miny else (p[1]-maxy if p[1]>maxy else 0)
    return dx*dx+dy*dy
def nw(p):
    minx,maxx,miny,maxy=WALLS
    return min(abs(minx-p[0]),abs(maxx-p[0]),abs(miny-p[1]),abs(maxy-p[1]))

def calc(car,target,wb,cb,wexp,wwt,cexp,cwt):
    op={car:{'g':0,'h':dtt(car,target),'parent':None}}
    cl={}; i=0; found=False
    while True:
        i+=1
        if i>1000: break
        if not op: break
        cp=min(op,key=lambda k:(op[k]['g']+op[k]['h'],op[k]['h']))
        cur=op[cp]; del op[cp]; cl[cp]=cur
        if dtt(cp,target)<80*80: found=True; break
        for d in [(20,20),(-20,20),(20,-20),(-20,-20),(20,0),(-20,0),(0,-20),(0,20)]:
            n=(cp[0]+d[0],cp[1]+d[1])
            if n in cl: continue
            oc=0
            w_=nw(n)
            if w_<wb:
                nm=(wb-w_)/wb; oc+=int((nm**wexp)*wwt)
            dist=math.sqrt(dtr(n,CROSS))
            if dist<cb:
                nm=(cb-dist)/cb; oc+=int((nm**cexp)*cwt)
            ng=cur['g']+dtt(n,cp)+oc
            if n not in op:
                op[n]={'g':ng,'h':dtt(n,target),'parent':cp}
            elif ng<op[n]['g']:
                op[n]['g']=ng; op[n]['parent']=cp
    return op,cl,i,found

def chain_of(op,cl):
    if not cl: return []
    pos=min(cl,key=lambda k:(cl[k]['g']+cl[k]['h']))
    ch=[pos]; node=cl[pos]; seen=set()
    while node['parent'] is not None and node['parent'] not in seen:
        seen.add(node['parent']); ch.append(node['parent'])
        node=cl.get(node['parent']) or op.get(node['parent'])
        if node is None: break
    return ch

def report(label,wb,cb,wexp,wwt,cexp,cwt):
    print(f"\n=== {label}: wall(buf={wb},exp={wexp},w={wwt}) cross(buf={cb},exp={cexp},w={cwt}) ===")
    gmw=1e9; gmc=1e9; gwe=0; gmi=0; cap=False
    for ball in ALL:
        op,cl,i,f=calc(CAR,ball,wb,cb,wexp,wwt,cexp,cwt)
        ch=chain_of(op,cl)
        endpos=ch[0] if ch else None
        ed=math.sqrt(dtt(endpos,ball)) if endpos else 1e9
        mw=min((nw(p) for p in ch),default=0)
        mc=min((math.sqrt(dtr(p,CROSS)) for p in ch),default=0)
        gmw=min(gmw,mw); gmc=min(gmc,mc); gwe=max(gwe,ed); gmi=max(gmi,i)
        if not f: cap=True
        st="FOUND" if f else "*CAP*"
        print(f"  {str(ball):<12} {st} iters={i:<4} chainlen={len(ch):<3} minWall={mw:<4} minCross={round(mc,1):<6} endDist={round(ed,1)}")
    print(f"  GLOBAL: minWall={gmw} minCross={round(gmc,1)} worstEnd={round(gwe,1)} maxIter={gmi} capped={cap}")

report("BASELINE (current on-disk)", 350,400, 4.0,50000.0, 4.0,50000.0)
report("PROPOSED quadratic 120000/450 (verifier said FAILS)", 450,480, 2.0,120000.0, 2.0,120000.0)
report("FINAL CHOSEN", 350,400, 4.0,70000.0, 4.0,60000.0)
