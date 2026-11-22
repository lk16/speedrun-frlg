import glob, os, json
SP = __import__('os').environ.get('CAL_DIR', '/tmp/cal') + '/'
idx=[]
for f in glob.glob(SP+'cal-[ptf]-*.csv'):
    with open(f,'rb') as fh:
        fh.readline(); first=fh.readline()
        if not first: continue
        fh.seek(max(0,os.path.getsize(f)-300)); last=fh.read().splitlines()[-1]
    idx.append((int(first.split(b',')[0]), int(last.split(b',')[0]), os.path.basename(f)))
idx.sort()
json.dump(idx, open(SP+'wft/csvidx.json','w'))
for x in idx: print(x)
