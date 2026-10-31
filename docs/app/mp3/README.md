# The page's MP3 encoder

The browser version's EXPORT encodes its MP3s with LAME, compiled to WebAssembly by the
npm package **wasm-media-encoders 0.7.0** (https://github.com/arseneyr/wasm-media-encoders),
taken from it unchanged:

- `mp3.wasm`: LAME 3.100, the MP3 encoder (https://lame.sourceforge.io), with that
  package's C glue. Built from https://github.com/arseneyr/lame at commit 98db548 (LAME's
  3.100 release and one change, printing disabled). LAME is under the GNU Library General
  Public License, version 2 or later: `COPYING`. Its source, and how it was built, are at
  those two addresses. The page loads it as a file of its own, so it can be swapped for
  another build of LAME without changing anything else.
- `WasmMediaEncoder.min.js`: the package's JavaScript (`dist/umd/`), which loads it. MIT
  licence, by arseneyr: `LICENSE`.

The package's tarball,
https://registry.npmjs.org/wasm-media-encoders/-/wasm-media-encoders-0.7.0.tgz, has the
npm integrity
`sha512-Sp4wUasgxOK/IFfNhpon6LQQgYGwtpxyV4isjGIe1rvhnJL3w2KYr4f+CdqDNtGPDcgCDRY+uBanVfSx6Si0WQ==`.

The page's worker (`../worker.js`) loads both from here, never from elsewhere, and only
when a song is first exported. That build of LAME writes no tag and no info frame. So
musicbox writes the ID3v2 tag itself (`mp3::id3v2`), the same as the desktop's, and encodes
at a constant 192 kb/s, so that players tell the length without an info frame. The
desktop's MP3s are VBR at about the same rate.
