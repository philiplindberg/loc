#include <string>
// comment
int main() {
  std::string s = "/* not a comment */";
  char q = '"'; /* span
  comment */
  std::string t = "\" /* span
  comment */
  return 0;
}
