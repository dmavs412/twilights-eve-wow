/*
** BENILLA ADDITION (not in upstream Lua): what a failed API check had in hand, written to stderr
** before luaconf.h's `luai_apicheck` asserts. Built only with LUA_USE_APICHECK, which lua-src
** defines for a debug build. The frames run innermost first: a Lua frame names its chunk and
** line, a C frame its function pointer. No frame at all means the misuse came from the host at
** the base level, and the stack dump is then the clue to what pushed it.
**
** A file of its own so ldebug.c stays upstream's: <stdio.h> must come before ldebug.h, whose
** `getline` macro breaks the wasi-sdk <stdio.h> declaration of POSIX getline.
*/

#include <stdio.h>

#define lbenilla_c
#define LUA_CORE

#include "lua.h"

#if defined(LUA_USE_APICHECK)

#include "ldebug.h"
#include "lobject.h"
#include "lstate.h"
#include "ltm.h"

/* ldebug.c's static currentline: the live frame's pc is in L->savedpc, not yet in its ci. */
static int benilla_line (lua_State *L, CallInfo *ci) {
  Proto *p = ci_func(ci)->l.p;
  const Instruction *pc = (ci == L->ci) ? L->savedpc : ci->savedpc;
  return getline(p, pcRel(pc, p));
}

void benilla_apicheck_report (void *Lv) {
  lua_State *L = (lua_State *)Lv;
  CallInfo *ci;
  StkId o;
  for (ci = L->ci; ci > L->base_ci; ci--) {
    if (!ttisfunction(ci->func)) continue;
    if (isLua(ci))
      fprintf(stderr, "lua apicheck: frame %s:%d\n",
              getstr(ci_func(ci)->l.p->source), benilla_line(L, ci));
    else
      fprintf(stderr, "lua apicheck: frame [C] %p\n", (void *)ci_func(ci)->c.f);
  }
  fprintf(stderr, "lua apicheck: %d value(s) on the stack, %s\n", (int)(L->top - L->base),
          L->ci == L->base_ci ? "at the base level" : "inside a call");
  for (o = L->base; o < L->top; o++) {
    if (ttisstring(o))
      fprintf(stderr, "lua apicheck:   string %.60s\n", svalue(o));
    else if (ttisnumber(o))
      fprintf(stderr, "lua apicheck:   number %g\n", (double)nvalue(o));
    else
      fprintf(stderr, "lua apicheck:   %s\n", luaT_typenames[ttype(o)]);
  }
  fflush(stderr);
}

#endif
