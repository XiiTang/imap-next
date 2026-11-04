# Boundless patches

Base: imap-next 0.3.4, upstream `9938818b74a4114f73b0a99c31bdd7c8991ebe0c`.

Boundless drives the client state machine over its own transport: it owns the
socket, TLS, native authentication, deadlines, resource budgets and public
delivery, while this library keeps command literals, AUTHENTICATE and IDLE
state. Each patch below says why Boundless needs it and which tests keep it.
Fork tests run with `cargo test --workspace` (the library and upstream's
`integration-test` suite) and with
`cargo test --no-default-features --features compress`; Boundless tests live
under its `protocols::imap` module.

## Kept patches

### Codec pin (Cargo.toml)

imap-codec and imap-types come from the XiiTang imap-codec fork's `boundless`
branch, one revision for the library and for Boundless (which patches
crates.io imap-codec to the same revision), so their types are one crate. The
revision is declared once, in `[workspace.dependencies]`, and both the library
and `integration-test` take it from there. The `ext_urlauth` feature is
forwarded to it. Move this pin whenever Boundless moves its imap-codec pin.

### Received bytes, consumed input and NO literal rejection (dd70977)

- `codec_quirks` makes imap-codec's default and `quirk_crlf_relaxed` features a
  default feature of this crate, so an application that disables default
  features gets strict CRLF decoding.
- `Client::received_message_bytes` returns the exact bytes of the last received
  message, and `take_consumed_input` the complete-message bytes consumed since
  the previous call, internally handled literal continuations included. An
  external driver bounds unread input with them without decoding twice.
- A tagged NO, as well as BAD, rejects a pending synchronizing literal, so the
  command ends without its payload being sent.

Fork tests (`tests/runtime_io.rs`): `exact_received_bytes_and_consumption_follow_each_message`,
`no_rejects_a_pending_literal_without_sending_its_payload`,
`internally_handled_literal_continuation_is_accounted_once`; `integration-test`:
`login_with_literal_and_unexpected_status` (another command's status leaves the
literal pending). Boundless:
`append_literal_rejection_never_sends_the_rejected_message`,
`fragmented_fetch_literal_is_preserved_with_its_tagged_completion`.

### Security-layer handoff (058cc00)

`Client::take_unparsed_after_message` returns the bytes after a completely
received response, through imap-codec's fragmentizer, so a SASL security layer
or COMPRESS installed after the tagged OK receives them instead of the
plaintext parser. Evidence: imap-codec's
`handoff_requires_a_complete_message_and_retains_next_layer_bytes`; Boundless:
`independent_zlib_compression_boundary_refusal_invalid_data_and_cancel`.

### Fork codec and COMPRESS=DEFLATE transport (ede0c44)

ede0c44 moved the library onto the fork's IMAP4rev2 codec. Evidence: upstream's
`integration-test` suite (`client`, `server` and `client_server`) passes
against it with `cargo test --workspace`.

The `compress` feature's `compress::Compressed` wraps an established stream in
RFC 4978 raw DEFLATE after a positive tagged COMPRESS response. It creates no
task, socket, timer or negotiation. Boundless: `independent_dovecot_binary_literals_compression_and_rejections`
and `independent_zlib_compression_boundary_refusal_invalid_data_and_cancel`.

### Literal lookalikes at a response boundary (part of c97715f)

When a line announces a literal, the receiver first asks imap-codec's
`Fragmentizer::complete_if_decodable` whether the line is already a complete
response, so `* OK text {123}` does not swallow the next response as its
literal. Fork test: `text_ending_in_literal_syntax_does_not_consume_the_next_response`.

## Removed (2026-10-10)

Boundless holds every response and APPEND literal whole within its size limit,
so response literal streaming (6e6a62c) and streamed APPEND uploads (from
c97715f) are gone.

## Fixed locked certificate verification (#312)

Update rustls-webpki 0.103.9 to 0.103.16 for RUSTSEC-2026-0382. Pin the
codec fork's lockfile-only advisory refresh, aa8bfd5fad71a9936a5b244b7b5e8c5ec9f30718,
in the manifest and lockfile together. Codec implementation and types are
unchanged. Regression coverage: `cargo test --workspace --all-features
--locked`, including the integration-test suite, and Boundless's complete
IMAP library tests.

## Codec pin for the BINARY response origin (2026-11-01)

Pin the codec fork's 91700a3ea7be32a1a5905bc65273200bb0de661c, which accepts the origin of a partial BINARY fetch
response (`BINARY[1]<0> ~{5}`, as Dovecot sends) and adds
`origin: Option<u64>` to `MessageDataItem::Binary`, in the manifest and
lockfile together. imap-next does not name that variant, so its code is
unchanged; Boundless needs the one codec revision in both crates (#432).
Regression coverage: `cargo test --workspace --all-features --locked`,
including the integration-test suite, and Boundless's IMAP library tests.

## Codec pin after retiring capability ownership (Boundless #43)

Pin `ef9945132c47989d5bb48d716309846ab7d1fb9e` in the workspace manifest
and lockfile. The codec removes its duplicate `ClientCapabilities` module;
Boundless `src/protocols/imap/capabilities.rs` owns command admission,
capability aliases, ENABLE state and incoming UTF-8 checks. imap-next never
used that module and needs no production change. Boundless
`src/protocols/imap/{wire,session}.rs` still relies on this library's protocol
flows and its shared codec types. Keep the one codec pin for both consumers,
including the retained rev2, URLAUTH and partial BINARY syntax.

Regression coverage: `cargo test --workspace --locked` (including
`integration-test`) and `cargo test --no-default-features --features compress
--locked`, followed by Boundless's IMAP and SMTP tests.
