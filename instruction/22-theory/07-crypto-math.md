# The Math Behind TLS's Cryptography

## What to learn

### Symmetric encryption as a keyed permutation
A block cipher like AES is, mathematically, a family of permutations of a fixed-size block, indexed by key — for each key, encryption is a bijection on the set of possible blocks, and decryption is its inverse. AES runs the block through multiple rounds (10/12/14 depending on key size) of substitution (a fixed S-box lookup, providing confusion) and permutation (byte shuffling across the block, providing diffusion) — [`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md)'s "symmetric encryption" already names this; here is the actual round structure that makes it secure.

```text
AES round (simplified): SubBytes -> ShiftRows -> MixColumns -> AddRoundKey
repeated N rounds, each round mixing key material further into the block
```
Gotcha: block cipher *mode* matters as much as the cipher itself — encrypting each block independently (ECB mode) leaks patterns (identical plaintext blocks produce identical ciphertext blocks), which is why TLS uses modes like GCM, which encrypts a per-block counter (CTR mode) under a unique nonce so identical plaintext blocks never produce identical ciphertext, and additionally provides authentication (AEAD — authenticated encryption with associated data).

### Diffie-Hellman: shared secrets over a public channel, from a hard problem
Two parties each pick a private random number, exchange a public value derived from it (`g^a mod p` and `g^b mod p`), and each can compute the same shared secret (`g^(ab) mod p`) from the other's public value and their own private one — without ever transmitting the secret itself. Security rests on the discrete logarithm problem being computationally hard: recovering `a` from `g^a mod p` (with a large enough `p`) is infeasible with known algorithms, even though computing `g^a mod p` from `a` is easy.

```text
Alice: picks a, sends A = g^a mod p
Bob:   picks b, sends B = g^b mod p
Alice computes: B^a mod p = g^(ba) mod p
Bob computes:   A^b mod p = g^(ab) mod p   -- same value, never transmitted directly
```
This is exactly what happens inside every TLS 1.3 handshake ([`01-network/13-tls.md`](../01-network/13-tls.md)) via elliptic-curve Diffie-Hellman (ECDHE) — the same mathematical shape, over elliptic curve points instead of modular exponentiation, for equivalent security with much smaller keys.

### RSA: trapdoor functions from the hardness of factoring
RSA's security rests on a different hard problem: given `n = p * q` for two large primes, factoring `n` back into `p` and `q` is computationally infeasible, even though multiplying `p * q` to get `n` is trivial. Key generation picks `p`, `q`, computes `n = pq` and a public/private exponent pair `(e, d)` satisfying `e*d ≡ 1 mod φ(n)`; encryption is `c = m^e mod n`, decryption is `m = c^d mod n` — the trapdoor is that computing `d` from `e` requires knowing `φ(n)`, which requires knowing `p` and `q`, which requires factoring `n`.

Gotcha: TLS 1.3 removed RSA key exchange entirely — every TLS 1.3 handshake uses (EC)DHE, because it gives forward secrecy (compromising a long-term key later doesn't expose past session keys, which static RSA key exchange in TLS 1.2 did not provide). RSA survives in TLS 1.3 only as a certificate *signature* algorithm, never for deriving the session key.

### Hashing: one-way, and why that's the whole point
A cryptographic hash function must be practically impossible to invert (given `H(x)`, finding any `x` is infeasible) and collision-resistant (finding any two different inputs with the same output is infeasible). HMAC ([`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md)'s vocabulary) builds a *keyed* hash from an unkeyed one specifically so that computing a valid HMAC requires the secret key, not just knowledge of the hash algorithm — this is what makes an HMAC usable as a message-authentication code, and a plain hash unusable as one.

### Why this handbook stops at "basics," and why this file exists at all
[`proxy`](../../proxy) should never implement any cryptographic primitive by hand — [`01-network/06-crypto-basics.md`](../01-network/06-crypto-basics.md) and [`01-network/13-tls.md`](../01-network/13-tls.md) are both explicit that this is `rustls`'s job, and hand-rolled crypto is a well-known source of catastrophic, silent vulnerabilities. This file's value is purely being able to read a TLS handshake trace or a security advisory and understand *why* a given primitive or mode is safe or unsafe — never to write one yourself.

## Practice
1. By hand, or with a calculator for small numbers, run through a toy Diffie-Hellman exchange with small `p`, `g`, and private values, and confirm both sides derive the same shared secret independently.
2. By hand, generate a toy RSA key pair with two small primes (e.g. `p=61, q=53`, the textbook example), compute `n`, `φ(n)`, and a valid `(e, d)` pair, then encrypt and decrypt a small message number.
3. Capture a real TLS 1.3 handshake (`openssl s_client -connect ... -tls1_3` or Wireshark) and identify the key-exchange group in use (e.g. `x25519`) — confirm it's elliptic-curve Diffie-Hellman, not RSA key exchange.
4. Encrypt the same repeated block pattern with AES in ECB mode and again in GCM mode (using a crypto library, not hand-rolled AES) and visually compare the ciphertext patterns — confirm ECB leaks the repetition and GCM doesn't.
5. Read `rustls`'s default cipher suite list and, for each entry, identify the key-exchange algorithm, the bulk cipher, and the mode — connect each back to a section in this file.
