# Note For Ethproofs: How To Verify Submitted Venus Proofs

This note explains how to verify the Venus proofs we submit to ethproofs.org and why the low-level verification must go through a conversion step.

## 1. Why not just use `verify_vadcop_final_proof()` directly?

There is a function in Venus:

- `verifier/src/verifier.rs`
- `verify_vadcop_final_proof(zisk_proof: &[u8], vadcop_final_vk: &[u8]) -> bool`

At first glance it looks like the correct API, but it is **not** the right verifier entrypoint for the proof files we submit to ethproofs.org.

The reason is a format mismatch.

The function comment says it expects:

- `[compressed(8)][pubs_len(8)][pubs][proof_bytes]`

But the implementation only does this:

1. read the first 8 bytes as `compressed`
2. take `zisk_proof[8..]`
3. pass that directly into:
   - `verify_vadcop_final_bytes(...)`, or
   - `verify_vadcop_final_compressed_bytes(...)`

So it does **not** actually parse:

- `pubs_len`
- `pubs`

That means it is not a correct verifier for the wrapped proof format we submit.

Also, `verify_vadcop_final_bytes(...)` expects a different low-level format entirely:

- a raw `u64`-aligned proof stream
- effectively:
  - `[n_publics][public_values][proof]`
- where all fields are packed as 8-byte words

So `verify_vadcop_final_proof()` is not suitable as a drop-in verifier for the submitted proof blob.

Inside Venus, the only real caller of `verify_vadcop_final_proof()` is:

- `ziskos/entrypoint/src/io.rs`

That is a different packaged-proof path, not the same as verifying the proof artifacts we currently submit.

## 2. What proof format do we actually submit?

The proof we submit to ethproofs.org is the bytes of:

- `vadcop_final_proof.bin`

In our proving flow:

1. we read the file as bytes
2. base64-encode those bytes
3. submit them as the `proof` field

So the submitted `proof` is a wrapped proof blob, not the raw low-level STARK byte stream expected by `verify_vadcop_final_bytes(...)`.

Concretely, the submitted proof corresponds to the wrapped proof object format we handle as:

- `ZiskProofWithPublicValues`

This wrapper contains:

- `proof`
- `publics`
- `program_vk`

Where:

- `proof` is the final proof bytes
- `publics` are the public values
- `program_vk` is the guest/program VK commitment

## 3. What does `verify_vadcop_final_bytes()` expect?

The low-level verifier:

- `verify_vadcop_final_bytes(proof, vk)`

expects:

- `proof` to already be the raw packed proof stream
- `vk` to be:
  - `vadcop_final.verkey.bin`

The raw proof format is the one produced by:

- `VadcopFinalProof::proof_with_publics()`

That packed format is:

- `[n_publics as u64][public_values][proof]`

This is why the low-level verifier requires `u64`-aligned input.

## 4. How should submitted proofs be verified?

The correct verification flow is:

1. decode the submitted proof bytes
2. deserialize them as the wrapped proof format
3. reconstruct a `VadcopFinalProof`
4. convert it into the raw packed format with:
   - `proof_with_publics()`
5. call:
   - `verify_vadcop_final_bytes(raw, vadcop_final_vk)`
   - or compressed variant if needed

So the real conversion is:

- submitted wrapped proof
  -> `ZiskProofWithPublicValues`
  -> `VadcopFinalProof`
  -> `proof_with_publics()`
  -> `verify_vadcop_final_bytes(...)`

## 5. How we implemented this in the demo

This repo contains a standalone demo that uses:

- `proofman-util`
- `proofman-verifier`

from:

- `https://github.com/ethproofs/venus`

The demo does the following:

1. load the submitted proof file
2. deserialize it as `ZiskProofWithPublicValues`
3. rebuild `VadcopFinalProof` by combining:
   - `proof`
   - `program_vk`
   - `publics`
4. call:
   - `vadcop.proof_with_publics()`
5. verify with:
   - `verify_vadcop_final_bytes(...)`
   - or `verify_vadcop_final_compressed_bytes(...)`

The key conversion logic is:

```rust
let wrapped = ZiskProofWithPublicValues::load(&proof_path)?;
let vadcop = wrapped.get_vadcop_final_proof()?;
let raw = vadcop.proof_with_publics();
let vk = std::fs::read(&vk_path)?;

let ok = if vadcop.compressed {
    verify_vadcop_final_compressed_bytes(&raw, &vk)
} else {
    verify_vadcop_final_bytes(&raw, &vk)
};
```

And the reconstruction is:

```rust
match &self.proof {
    ZiskProof::VadcopFinal(proof_bytes) | ZiskProof::VadcopFinalCompressed(proof_bytes) => {
        let compressed = matches!(self.proof, ZiskProof::VadcopFinalCompressed(_));
        let mut pubs = self.program_vk.vk.clone();
        pubs.extend(self.publics.public_bytes());
        Ok(VadcopFinalProof::new(proof_bytes.clone(), pubs, compressed))
    }
    _ => Err(...)
}
```

This is the important step:

- `public_values = program_vk || publics`

Then `proof_with_publics()` packs it into the raw format the low-level verifier wants.

## 6. How to use this demo

Repo:

- `https://github.com/cysic-labs/venus_proof_verify_demo`

From a clean checkout:

```bash
git clone git@github.com:cysic-labs/venus_proof_verify_demo.git
cd venus_proof_verify_demo
make install
make verify
```

This downloads:

- the submitted proof file
- the matching `vadcop_final.verkey.bin`

and verifies it.

## 7. Summary

- Do **not** directly use `verify_vadcop_final_proof()` for our submitted proof blobs.
- Our submitted proof is a wrapped proof format, not the raw format expected by `verify_vadcop_final_bytes()`.
- The correct flow is:
  - deserialize wrapped proof
  - reconstruct `VadcopFinalProof`
  - convert with `proof_with_publics()`
  - verify with `verify_vadcop_final_bytes()` using `vadcop_final.verkey.bin`
