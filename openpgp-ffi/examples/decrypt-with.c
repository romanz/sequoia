/* This example demonstrates how to use the low-level interface to
   decrypt a file.  */

#define _GNU_SOURCE
#include <assert.h>
/* Roughly glibc compatible error reporting.  */
#define error(S, E, F, ...) do {                        \
  fprintf (stderr, (F), __VA_ARGS__);                   \
  int s = (S), e = (E);                                 \
  if (e) { fprintf (stderr, ": %s", strerror (e)); }    \
  fprintf (stderr, "\n");                               \
  fflush (stderr);                                      \
  if (s) { exit (s); }                                  \
  } while (0)
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include <sequoia/openpgp.h>

struct decrypt_cookie {
  pgp_tpk_t key;
  int decrypt_called;
};

static pgp_status_t
get_public_keys_cb (void *cookie_raw,
                    pgp_keyid_t *keyids, size_t keyids_len,
                    pgp_tpk_t **tpks, size_t *tpk_len,
                    void (**our_free)(void *))
{
  /* Feed the TPKs to the verifier here.  */
  *tpks = NULL;
  *tpk_len = 0;
  *our_free = free;
  return PGP_STATUS_SUCCESS;
}

static pgp_status_t
check_cb (void *cookie_opaque, pgp_message_structure_t structure)
{
  pgp_message_structure_iter_t iter = pgp_message_structure_iter (structure);

  for (pgp_message_layer_t layer = pgp_message_structure_iter_next (iter);
       layer;
       layer = pgp_message_structure_iter_next (iter)) {
    uint8_t algo;
    uint8_t aead_algo;
    pgp_verification_result_iter_t results;

    switch (pgp_message_layer_variant (layer)) {
    case PGP_MESSAGE_LAYER_COMPRESSION:
      pgp_message_layer_compression (layer, &algo);
      fprintf (stderr, "Compressed using %d\n", algo);
      break;

    case PGP_MESSAGE_LAYER_ENCRYPTION:
      pgp_message_layer_encryption (layer, &algo, &aead_algo);
      if (aead_algo) {
        fprintf (stderr, "Encrypted and protected using %d/%d\n",
                 algo, aead_algo);
      } else {
        fprintf (stderr, "Encrypted using %d\n", algo);
      }
      break;

    case PGP_MESSAGE_LAYER_SIGNATURE_GROUP:
      pgp_message_layer_signature_group (layer, &results);
      for (pgp_verification_result_t result =
             pgp_verification_result_iter_next (results);
           result;
           result = pgp_verification_result_iter_next (results)) {
        pgp_signature_t sig;
        pgp_keyid_t keyid;
        char *keyid_str = NULL;

        switch (pgp_verification_result_variant (result)) {
        case PGP_VERIFICATION_RESULT_GOOD_CHECKSUM:
          pgp_verification_result_good_checksum (result, &sig, NULL,
                                                 NULL, NULL, NULL);
          keyid = pgp_signature_issuer (sig);
          keyid_str = pgp_keyid_to_string (keyid);
          fprintf (stderr, "Good signature from %s\n", keyid_str);
          break;

        case PGP_VERIFICATION_RESULT_MISSING_KEY:
          pgp_verification_result_missing_key (result, &sig);
          keyid = pgp_signature_issuer (sig);
          keyid_str = pgp_keyid_to_string (keyid);
          fprintf (stderr, "No key to check signature from %s\n", keyid_str);
          break;

        case PGP_VERIFICATION_RESULT_BAD_CHECKSUM:
          pgp_verification_result_bad_checksum (result, &sig);
          keyid = pgp_signature_issuer (sig);
          if (keyid) {
            keyid_str = pgp_keyid_to_string (keyid);
            fprintf (stderr, "Bad signature from %s\n", keyid_str);
          } else {
            fprintf (stderr, "Bad signature without issuer information\n");
          }
          break;

        default:
          assert (! "reachable");
        }
        free (keyid_str);
        pgp_signature_free (sig);
        pgp_verification_result_free (result);
      }
      pgp_verification_result_iter_free (results);
      break;

    default:
      assert (! "reachable");
    }

    pgp_message_layer_free (layer);
  }

  pgp_message_structure_iter_free (iter);
  pgp_message_structure_free (structure);

  /* Implement your verification policy here.  */
  return PGP_STATUS_SUCCESS;
}

static pgp_status_t
decrypt_cb (void *cookie_opaque,
            pgp_pkesk_t *pkesks, size_t pkesk_count,
            pgp_skesk_t *skesks, size_t skesk_count,
            pgp_decryptor_do_decrypt_cb_t *decrypt,
            void *decrypt_cookie,
            pgp_fingerprint_t *identity_out)
{
  pgp_status_t rc;
  pgp_error_t err;
  struct decrypt_cookie *cookie = cookie_opaque;

  /* Prevent iterations, we only have one key to offer.  */
  assert (!cookie->decrypt_called);
  cookie->decrypt_called = 1;

  for (int i = 0; i < pkesk_count; i++) {
    pgp_pkesk_t pkesk = pkesks[i];
    pgp_keyid_t keyid = pgp_pkesk_recipient (pkesk);

    pgp_tpk_key_iter_t key_iter = pgp_tpk_key_iter_all (cookie->key);
    pgp_key_t key;
    while ((key = pgp_tpk_key_iter_next (key_iter, NULL, NULL))) {
      pgp_keyid_t this_keyid = pgp_key_keyid (key);
      int match = pgp_keyid_equal (this_keyid, keyid);
      pgp_keyid_free (this_keyid);
      if (match)
        break;
      pgp_key_free (key);
    }
    pgp_tpk_key_iter_free (key_iter);
    pgp_keyid_free (keyid);
    if (! key)
      continue;

    uint8_t algo;
    uint8_t session_key[1024];
    size_t session_key_len = sizeof session_key;
    if (pgp_pkesk_decrypt (&err,
                           pkesk, key, &algo,
                           session_key, &session_key_len)) {
      error (1, 0, "pgp_pkesk_decrypt: %s", pgp_error_to_string (err));
    }
    pgp_key_free (key);

    pgp_session_key_t sk = pgp_session_key_from_bytes (session_key,
                                                       session_key_len);
    rc = decrypt (decrypt_cookie, algo, sk);
    pgp_session_key_free (sk);

    *identity_out = pgp_tpk_fingerprint (cookie->key);
    return rc;
  }

  return PGP_STATUS_UNKNOWN_ERROR;
}

int
main (int argc, char **argv)
{
  pgp_error_t err;
  pgp_tpk_t tpk;
  pgp_reader_t source;
  pgp_reader_t plaintext;
  uint8_t buf[1024];
  ssize_t nread;

  if (argc != 2)
    error (1, 0, "Usage: %s <keyfile> <cipher >plain", argv[0]);

  tpk = pgp_tpk_from_file (&err, argv[1]);
  if (tpk == NULL)
    error (1, 0, "pgp_tpk_from_file: %s", pgp_error_to_string (err));

  source = pgp_reader_from_fd (STDIN_FILENO);
  assert (source);

  struct decrypt_cookie cookie = {
    .key = tpk,
    .decrypt_called = 0,
  };
  plaintext = pgp_decryptor_new (&err, source,
                                 get_public_keys_cb, decrypt_cb,
                                 check_cb, NULL, &cookie, 0);
  if (! plaintext)
    error (1, 0, "pgp_decryptor_new: %s", pgp_error_to_string (err));

  while ((nread = pgp_reader_read (&err, plaintext, buf, sizeof buf)) > 0) {
    write (STDOUT_FILENO, buf, nread);
  }
  if (nread < 0)
    error (1, 0, "pgp_reader_read: %s", pgp_error_to_string (err));

  pgp_reader_free (plaintext);
  pgp_reader_free (source);
  pgp_tpk_free (tpk);
  return 0;
}
