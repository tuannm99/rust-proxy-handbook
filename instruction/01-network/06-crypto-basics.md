# Crypto Basics: Encryption, Hashing, Signatures, PKI

Part of the from-scratch fundamentals series — see `01-network/01-fundamentals.md`
for the full index. `01-network/13-tls.md` opens with "negotiate a
cipher suite, exchange (EC)DHE key shares, derive session keys" and
`07-security/02-jwt.md`/`07-security/03-mtls.md`/`07-security/01-auth.md`
all lean on "signature," "public key," and "certificate chain" — none of
that is readable without this. This file doesn't teach cryptography as a
field; it teaches the five building blocks well enough that TLS's
handshake and JWT's signature scheme stop being magic.

## What to learn

### Symmetric encryption: one shared secret, fast
**Symmetric** encryption uses the *same* key to encrypt and decrypt.
It's fast — cheap enough to run on every byte of every connection's
traffic — and that speed is exactly why it's what actually protects the
bulk of data on a TLS connection (AES and ChaCha20 are the two you'll see
named in a TLS cipher suite).

The catch is key distribution: both sides need the *same* secret key
*before* they can talk securely, which is a chicken-and-egg problem over
a network two strangers just connected on — how do you agree on a secret
without an eavesdropper on the wire seeing it too? Solving exactly this
problem is what the next section is for.

### Asymmetric (public-key) encryption: two keys, slow, no shared secret needed
**Asymmetric** encryption uses a mathematically related *pair* of keys: a
**public key** (safe to hand out to anyone, including attackers) and a
**private key** (never shared, kept secret by its owner). Data encrypted
with the public key can only be decrypted with the matching private key.
This solves the distribution problem above — no secret needs to travel
over the wire at all — at the cost of being computationally much more
expensive than symmetric encryption, too slow to use for bulk traffic.

**Key exchange** algorithms (Diffie-Hellman and its elliptic-curve
variant, ECDHE — the "(EC)DHE" in `13-tls.md`'s opening line) are a
clever asymmetric-adjacent trick: both sides exchange public values over
the open wire, and each independently *computes* the same shared secret
from their own private value and the other side's public value — an
eavesdropper who sees both public values cannot derive the shared secret
without one side's private value. This is how TLS gets a fresh symmetric
key for every connection, without ever transmitting that key itself.

### The hybrid approach every TLS connection actually uses
Neither building block alone is both fast and distribution-friendly, so
TLS uses both, in sequence: **asymmetric** key exchange (ECDHE) during
the handshake to agree on a shared secret without transmitting it, then
**symmetric** encryption (AES/ChaCha20) using that derived secret for all
the actual request/response traffic. This is why `13-tls.md`'s cipher
suite name has multiple parts (e.g.
`TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`) — it's naming the key-exchange
algorithm, the signature algorithm (next section), and the symmetric
cipher, all three negotiated together.

### Hashing: one-way fingerprinting
A **hash function** takes input of any size and produces a fixed-size
output (a "digest") with three properties that matter here: the same
input always produces the same output, a tiny change in input produces a
completely different output, and it's computationally infeasible to go
*backward* from the digest to something that produces it (one-way).
SHA-256 (producing 256 bits, 32 bytes) is the one you'll see named most
often in this handbook.

Hashing alone gives you *integrity checking* (did this data change?) but
not *authenticity* (did it come from who I think it did? — anyone can
hash anything). `13-algorithms/count-min-sketch.md` and
`13-algorithms/hashmap.md`'s hash functions solve a different problem
entirely (distributing keys across buckets) and are explicitly *not*
held to cryptographic one-way-ness — don't conflate a hash function used
for a hash table with a *cryptographic* hash function used for security;
the security ones are much slower and chosen specifically to resist the
attacks a fast hash table hash doesn't need to resist.

### HMAC: a hash plus a secret, for authenticity
An **HMAC** (hash-based message authentication code) combines a
cryptographic hash function with a secret key, producing a digest that
only someone holding the same secret could have produced — this is what
gives you authenticity on top of plain hashing's integrity. HMAC is what
"HS256" means in a JWT's algorithm name
(`07-security/02-jwt.md`): HMAC using SHA-256, with the shared secret
being whatever key your service and the token issuer agreed on out of
band.

### Digital signatures: authenticity without a shared secret
A **digital signature** does for asymmetric keys what HMAC does for a
shared secret: the *private* key holder signs data (a computation
involving the data and the private key), and *anyone* holding the
matching *public* key can verify the signature is valid — without ever
having access to the private key themselves. This is what "RS256" and
"ES256" mean in a JWT's algorithm name: RSA or ECDSA signatures,
verified with a public key rather than a shared secret.

The practical difference from HMAC that matters for
`07-security/02-jwt.md`'s algorithm-confusion attack: an HMAC secret must
stay equally secret on both the signer's and verifier's side (whoever
can verify can also forge), while a signature's public key is *meant* to
be public — anyone can verify, only the private key holder can sign.
Mixing the two up (treating a public key as if it were an HMAC secret) is
exactly the vulnerability that attack exploits.

### Certificates and PKI: binding a public key to an identity
Asymmetric crypto solves "how do we get a shared secret without
transmitting it," but leaves a gap: when your browser gets a public key
from a server, how does it know that key actually belongs to
`example.com` and not to an attacker in the middle? A **certificate**
answers this: it's a public key plus an identity (a hostname, in
`13-tls.md`'s SNI-matched case) plus a **digital signature** — signed not
by the server itself, but by a **Certificate Authority (CA)**, a third
party your browser/OS already trusts.

This forms a **chain of trust**: your OS ships with a built-in list of
trusted **root CA** public keys. A root CA signs **intermediate CA**
certificates, which sign the actual **leaf** (server) certificates your
proxy presents. Verifying a certificate means walking this chain — leaf
signed by intermediate, intermediate signed by a root you already
trust — using the signature-verification mechanism from the previous
section at every step. `07-security/03-mtls.md`'s "chains to a trusted
CA is weaker than it sounds" gotcha is entirely about this: the CA
vouches for *an identity*, not for *authorization* — anyone that CA will
sign for gets a valid chain.

**PKI** (public key infrastructure) is just the umbrella term for this
whole system: the CAs, the certificates, the chain-of-trust verification,
and the tooling (ACME/Let's Encrypt, referenced in `13-tls.md`'s
certificate management section) that issues and renews them.

## Practice
1. Run `openssl s_client -connect example.com:443 -servername
   example.com` and read the certificate chain it prints — identify the
   leaf certificate's subject (the hostname), then trace up through any
   intermediate to the root, and note which fields carry the issuer's
   identity vs the signed subject's identity.
2. Run `openssl x509 -in <a cert file> -noout -text` on a real
   certificate (export one from step 1 with `openssl s_client ... |
   openssl x509`) and locate the public key, the signature, and the
   issuer/subject fields discussed above.
3. Compute `sha256sum` on a file, change one byte, and recompute — confirm
   the digest changes completely rather than by a small amount (the
   avalanche property of a good hash function).
4. Read `07-security/02-jwt.md`'s "Algorithm confusion, concretely"
   section now that HMAC vs signature is fresh, and explain in your own
   words why treating an RSA public key as an HMAC secret lets an
   attacker forge a token — tie it back to "anyone can verify a
   signature; only the secret holder can compute an HMAC" from this file.
5. Generate an RSA or Ed25519 keypair (`openssl genpkey` or `ssh-keygen`),
   sign a small file with the private key, and verify the signature with
   the public key — then try verifying with a *different* keypair's
   public key and confirm it fails.
