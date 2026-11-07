/* scratch tool: which HYP nodes show a picture (ESC 50 in the leading ESC block), using HypView's own lh5d.c */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "/Volumes/Daten/Development/PiRusTOS/sdk/apps/hypview/hyp/lh5d.c"
static unsigned be32(unsigned char*p){return (p[0]<<24)|(p[1]<<16)|(p[2]<<8)|p[3];}
int main(int argc,char**argv){
  FILE*f=fopen(argv[1],"rb"); fseek(f,0,SEEK_END); long n=ftell(f); fseek(f,0,SEEK_SET);
  unsigned char*d=malloc(n); fread(d,1,n,f);
  int num=(d[8]<<8)|d[9]; /* HDOC size(4) num(2) at 4..9 */ num=(d[8]<<8)|d[9];
  unsigned char*e[4096]; int o=12; 
  for(int i=0;i<num;i++){e[i]=d+o;o+=d[o];}
  unsigned char*end=d+o; (void)end;
  /* entry: len,type,seek(4),comp_diff(4),next(2),prev(2),toc(2),name */
  int encrypted=0; /* st_guide_flags: try both */
  unsigned seek[4097]; for(int i=0;i<num;i++) seek[i]=be32(e[i]+2);
  /* last entry's size: file end */ seek[num]=n;
  for(int i=0;i<num;i++){
    int ty=e[i][1]; if(ty!=0) continue;
    unsigned cdiff=(e[i][6]<<8)|e[i][7]; long csz=seek[i+1]-seek[i]; long usz=csz+cdiff;
    unsigned char*u=malloc(usz+8); 
    if(cdiff){ lh5_packedMem=(char*)d+seek[i]; lh5_packedLen=csz; lh5_unpackedMem=(char*)u; lh5_unpackedLen=usz; lh5_decode(1);} else memcpy(u,d+seek[i],usz);
    /* decrypted? crude: if there is no ESC at start try xor 127 */
    if(u[0]!=27 && u[0]!=0 ){ for(long k=0;k<usz;k++) u[k]^=127; }
    int p=0; int pics=0; int piclist[64];
    while(p<usz && u[p]==27){ int code=u[p+1];
      if(code==50){ piclist[pics++]=(u[p+2]-1)+ (u[p+3]-1)*223; p+=8; /* approx, only for listing */ if(pics>60)break; continue;}
      if(code==35){ p+=3; while(p<usz&&u[p]) p++; p++; continue;}
      if(code==49){p+=11;continue;}
      if(code>=40&&code<=48){p+=u[p+2]? 2+u[p+2]-1: 3; /* length byte semantic: src+= src[1]-1 */ p= p; }
      break;
    }
    /* also whole-node scan for ESC 50 */
    int any=0; for(long k=0;k+1<usz;k++) if(u[k]==27&&u[k+1]==50) any++;
    printf("node %3d  %-24s  usz %6ld  lead-pics %d  esc50-anywhere %d\n", i, (char*)e[i]+14, usz, pics, any);
    free(u);
  }
  return 0;
}
