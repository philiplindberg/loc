#include <stdio.h>
/* block
   comment */
int main(void) {
  char c = '"'; /* span
  comment */
  puts("// not a comment");
  return 0; /* trailing */
}

// end
