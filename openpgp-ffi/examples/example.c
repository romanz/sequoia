#define _GNU_SOURCE
#include <errno.h>
/* Roughly glibc compatible error reporting.  */
#define error(S, E, F, ...) do {                        \
  fprintf (stderr, (F), __VA_ARGS__);                   \
  int s = (S), e = (E);                                 \
  if (e) { fprintf (stderr, ": %s", strerror (e)); }    \
  fprintf (stderr, "\n");                               \
  fflush (stderr);                                      \
  if (s) { exit (s); }                                  \
  } while (0)
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <sequoia/openpgp.h>

int
main (int argc, char **argv)
{
  pgp_error_t err;
  pgp_tpk_t tpk;

  if (argc != 2)
    error (1, 0, "Usage: %s <file>", argv[0]);

  tpk = pgp_tpk_from_file (&err, argv[1]);
  if (tpk == NULL)
    error (1, 0, "pgp_tpk_from_file: %s", pgp_error_to_string (err));

  char *debug = pgp_tpk_debug (tpk);
  printf ("%s", debug);
  free (debug);

  pgp_tpk_free (tpk);
  return 0;
}
