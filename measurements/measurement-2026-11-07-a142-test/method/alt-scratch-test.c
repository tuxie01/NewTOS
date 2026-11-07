/* scratch test program (not product): cflib alerts with every icon, then a form with check box+radio via cflib alert only */
#include <gem.h>
#include <cflib.h>
int main(void)
{
	init_app(NULL);
	do_walert(1, 0, "[0][icon 0 info][OK]", "A142 test");
	do_walert(1, 0, "[1][icon 1 exclam][OK]", "A142 test");
	do_walert(1, 0, "[2][icon 2 question][OK]", "A142 test");
	do_walert(1, 0, "[3][icon 3 stop][OK]", "A142 test");
	exit_app(0);
	return 0;
}
