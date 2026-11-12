# usage: python3 -I tmpentries.py fixed.img unchanged.img ; lists every differing byte inside the two deleted tmp LFN entries
import sys
a=open(sys.argv[1],'rb').read(); b=open(sys.argv[2],'rb').read()
for e in (217248,221408):
    print('entry@%d fixed=%s unchanged=%s'%(e,a[e:e+32].hex(),b[e:e+32].hex()))
    print('  differing offsets:',[e+i for i in range(32) if a[e+i]!=b[e+i]])
    print('  name chars fixed:',a[e+1:e+11].decode('utf-16-le','ignore')+a[e+14:e+26].decode('utf-16-le','ignore'),' unchanged:',b[e+1:e+11].decode('utf-16-le','ignore')+b[e+14:e+26].decode('utf-16-le','ignore'))
