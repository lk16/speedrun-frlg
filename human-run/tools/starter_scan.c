// Exact scan of the full 2^32 gRngValue cycle for gift-mon (starter) outcomes.
// Model (decompiled/src/pokemon.c:1778,1836-1852; include/random.h:14-19; verified
// against the ROM by crates/frlg-route/examples/starter-sweep.rs): state s -> Random() x4:
// pid = r0 | r1<<16 ; word1 = r2 (hp 0-4, atk 5-9, def 10-14) ; word2 = r3 (spe, spa, spd).
// Every start state is one (seed, stream offset) pair a human could in principle reach.
// Build: cc -O2 -o starter_scan starter_scan.c && ./starter_scan
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#define MODEST 15
#define MILD 16
#define RASH 19
#define QUIET 17
static inline uint32_t nx(uint32_t s){return s*1103515245u+24691u;}
enum {T0,T1,T1R,T2,T3,T4,NT};
static const char*names[NT]={
 "T0 nature in {Modest,Mild,Rash}, SpA IV>=25",
 "T1 nature in {Modest,Mild,Rash}, SpA IV>=30",
 "T1R nature in {Mild,Rash}, SpA IV>=30",
 "T2 T1 + Spe,HP,Def IV>=20",
 "T3 T1 + Spe,HP,Def IV>=25",
 "T4 T1R + Spe,HP,Def IV>=25"};
static int good(uint32_t pid,uint32_t w1,uint32_t w2,int*g){
  int n=pid%25;int hp=w1&31,def=(w1>>10)&31,spe=w2&31,spa=(w2>>5)&31;
  int m3=(n==MODEST||n==MILD||n==RASH), r2=(n==MILD||n==RASH);
  g[T0]=m3&&spa>=25; g[T1]=m3&&spa>=30; g[T1R]=r2&&spa>=30;
  int e20=hp>=20&&def>=20&&spe>=20, e25=hp>=25&&def>=25&&spe>=25;
  g[T2]=g[T1]&&e20; g[T3]=g[T1]&&e25; g[T4]=g[T1R]&&e25; return 0;}
int main(void){
  uint64_t cnt[NT]={0}, run[NT][9]={{0}}, runlen[NT]={0}, pair[NT][9]={{0}};
  // rolling window of the last 9 'good' flags per tier, for P(good at i+d | good at i)
  uint32_t hist[NT]={0};
  uint64_t nat[25]={0};
  uint32_t s=0; uint16_t r[4]; 
  for(int i=0;i<4;i++){s=nx(s);r[i]=s>>16;}
  uint64_t total=1ULL<<32;
  for(uint64_t i=0;i<total;i++){
    uint32_t pid=r[0]|((uint32_t)r[1]<<16); int g[NT]; good(pid,r[2],r[3],g);
    nat[pid%25]++;
    for(int t=0;t<NT;t++){
      hist[t]=(hist[t]<<1)|g[t];
      if(g[t]){cnt[t]++; runlen[t]++; for(int d=1;d<=8;d++) if((hist[t]>>d)&1) pair[t][d]++;}
      else if(runlen[t]){ run[t][runlen[t]>8?8:runlen[t]]++; runlen[t]=0; }
    }
    s=nx(s); r[0]=r[1];r[1]=r[2];r[2]=r[3];r[3]=s>>16;
  }
  printf("total states %llu\n",(unsigned long long)total);
  for(int t=0;t<NT;t++){
    printf("%s\n  density %.6g  (1 in %.0f)\n  P(good at i+d | good at i), d=1..8:",names[t],(double)cnt[t]/total,(double)total/cnt[t]);
    for(int d=1;d<=8;d++)printf(" %.4f",(double)pair[t][d]/cnt[t]); printf("\n  run-length histogram (len 1..7, 8+):");
    for(int l=1;l<=8;l++)printf(" %llu",(unsigned long long)run[t][l]); printf("\n");
  }
  printf("nature counts:");for(int n=0;n<25;n++)printf(" %d:%llu",n,(unsigned long long)nat[n]);printf("\n");
  return 0;}
