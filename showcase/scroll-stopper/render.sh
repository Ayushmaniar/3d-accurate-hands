set -e
cd "/c/Users/Rajat/Documents/3D Accurate Hands/showcase/scroll-stopper"
P=../../starters/fframes-clip/target/release/fframes-clip.exe
python - <<'PY' > jobs.txt
import json
for k,(a,b,look,bg,sp) in enumerate(json.load(open('segs.json'))):
    print(k, json.dumps(look,separators=(',',':')), bg, sp, ' '.join(f"{(i+0.25)/30:.4f}s" for i in range(a,b)))
PY
while read k look bg sp times; do
  $P --clip clip.json --assets ../../assets frame $times --look-json "$look" --background "$bg" --spotlight "$sp" -o raw/s$k > /dev/null 2>&1
  echo "seg $k done $(date +%T)"
done < jobs.txt
