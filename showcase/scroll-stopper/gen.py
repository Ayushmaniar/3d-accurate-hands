import json
snaps=[1.0,2.0,2.9,3.6,4.2,4.75,5.3]
PF=[-1,0,1]
PLACE={"screen":[0.5,0.62],"depth":0.45,"palm_facing":PF,"fingers_toward":[0,1,0],"elbow_screen":[0.7,1.4]}
SNAP={"thumb":{"cmc_abd":30,"cmc_flex":10,"cmc_rot":55,"mcp_flex":-15,"ip_flex":-15},"middle":{"mcp_flex":95,"pip_flex":95,"dip_flex":60}}
keys=[{"t":-0.25,"hands":{"right":{"from":"relaxed","place":{"screen":[0.56,1.05],"depth":0.5,"palm_facing":PF,"fingers_toward":[-0.25,1,0],"elbow_screen":[0.72,1.6]}}}},
      {"t":0.6,"hands":{"right":{"from":"snap_ready","place":PLACE}}}]
for i,s in enumerate(snaps):
    keys.append({"t":s,"move":0.07,"ease":"snappy","hands":{"right":SNAP}})
    if i<len(snaps)-1:
        nxt=snaps[i+1]
        keys.append({"t":min(s+0.38,nxt-0.12),"move":0.25,"hands":{"right":{"from":"snap_ready"}}})
keys.append({"t":6.0,"move":0.4,"ease":"snappy","hands":{"right":{"from":"spread_palm","place":{"screen":[0.5,0.8],"depth":0.52,"palm_facing":[-0.3,0,1],"fingers_toward":[0,1,0],"elbow_screen":[0.66,1.4]}}}})
clip={"version":1,"fps":30,"duration":7.0,
 "camera":{"position":[0,0.1,0.6],"target":[0,0.1,0],"fov":32},
 "look":{"style":"realistic","skin":"medium","motion_blur":4},
 "keys":keys}
json.dump(clip,open("clip.json","w"),indent=1)
