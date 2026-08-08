# Licensing of this adapter

**MIT, like the rest of `wa-wire`.** No file here is covered by MPL-2.0.

This needs saying because [`DESIGN.md`](../../DESIGN.md) planned otherwise.
D-022 set aside `adapters/hypermeow/` as an MPL-2.0 subdirectory on the
expectation that the adapter would carry patched `whatsmeow` files, since
MPL-2.0 is file-level copyleft and a modified covered file stays covered
wherever it goes.

It turned out not to need any. The two hooks this adapter is built on —
`RawNodeHandler` carrying the frame bytes, and `DecryptedPayloadHandler` —
were contributed to `hypermeow` itself
([polymorfa/hypermeow#5](https://github.com/polymorfa/hypermeow/pull/5)), where
they are MPL-2.0 as every file in that repository is. Nothing was copied here.

What remains is code that *imports* `hypermeow`. MPL-2.0 §3.3 permits
distributing a Larger Work under other terms so long as the Covered Software
keeps its own, which is exactly the arrangement: `hypermeow` stays MPL-2.0 at
its own repository, and this adapter is MIT at ours.

## What would change that

Copying any file from `hypermeow` into this directory, or vendoring it. If a
future change needs an engine patch that upstream will not take, the patched
file arrives here still covered and this directory becomes MPL-2.0 after all —
which is what D-022 was written for, and the reason it is worth keeping rather
than deleting.

## The engine

Built against `hypermeow` at the `frame-bytes-and-plaintext-hooks` branch,
declared as a `replace` in `go.mod` rather than a version. That is deliberate:
a `replace` says plainly that this is not built against anything published, and
disappears the moment the hooks land in a release.
