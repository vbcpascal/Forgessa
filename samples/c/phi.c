//test multiple function calls
#include <stdio.h>
#define WriteLine() printf("\n");
#define WriteLong(x) printf(" %lld", (long)x);
#define ReadLong(a) if (fscanf(stdin, "%lld", &a) != 1) a = 0;
#define long long long

void main()
{
  long a, b, c;

  a = 2;
  b = 1;

  while (a > b) {
    c = 4 * a + a;
    b = b + 1;
  }

  WriteLong(b);
}
