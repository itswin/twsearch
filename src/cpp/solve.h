#ifndef SOLVE_H
#include "canon.h"
#include "generatingset.h"
#include "prunetable.h"
#include "puzdef.h"
#include "threads.h"
/*
 *   Routines to use iterated depth-first searching to solve a particular
 *   position (and the required code to distribute the work across
 *   multiple threads).
 */
extern ull solutionsfound, solutionsneeded;
extern int noearlysolutions;
extern int phase2;
extern int optmindepth;
extern int onlyimprovements;
extern int randomstart;
extern string lastsolution;
struct solvestate {
  int st, mi;
  ull mask, skipbase;
};
struct solveworker {
  vector<allocsetval> posns;
  vector<solvestate> solvestates;
  vector<int> movehist;
  long long lookups, checktarget, checkincrement;
  int d, id;
  setval *looktmp;
  char padding[256]; // kill false sharing
  void init(const puzdef &pd, int d_, int id_, const setval &p);
  int solveiter(const puzdef &pd, prunetable &pt, int togo, int sp, int st);
  ull innersetup(prunetable &pt, int sp);
  int innerfetch(const puzdef &pd, prunetable &pt, int &togo, int &sp, int &st,
                 ull h);
  int possibsolution(const puzdef &pd, int sp);
  int solvestart(const puzdef &pd, prunetable &pt, int w);
  void dowork(const puzdef &pd, prunetable &pt);
};
extern solveworker solveworkers[MAXTHREADS];
extern int maxdepth, didprepass;
void setsolvecallback(int (*)(setval &, const vector<int> &, int, int),
                      int (*)(int));
int solve(const puzdef &pd, prunetable &pt, const setval p,
          generatingset *gs = 0);
#define SOLVE_H
#endif
