//test multiple function calls
#include <stdio.h>
#define WriteLine() printf("\n");
#define WriteLong(x) printf(" %lld", (long)x);
#define ReadLong(a) if (fscanf(stdin, "%lld", &a) != 1) a = 0;
#define long long long

void main()
{
  long x, a, b;

  while (1 == 1) {
    x = 1;
  }

  a = 2;
  if (a > 0) {
    a = 4;
  } else {
    a = 4;
  }

  b = a;
  WriteLong(b);
}
