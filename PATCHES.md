# Boundless patches

Base: imap-next 0.3.4, upstream `9938818b74a4114f73b0a99c31bdd7c8991ebe0c`.

Boundless drives the client state machine over its own transport: it owns the
socket, TLS, native authentication, deadlines, resource budgets and public
delivery, while this library keeps command literals, AUTHENTICATE and IDLE
state. Each patch below says why Boundless needs it and which tests keep it.
Fork tests run with `cargo test` and with
`cargo test --no-default-features --features compress`; Boundless tests live
under its `protocols::imap` module.

## Kept patches

### Codec pin (Cargo.toml)

imap-codec and imap-types come from the XiiTang imap-codec fork's `boundless`
branch, one revision for the library and for Boundless (which patches
crates.io imap-codec to the same revision), so their types are one crate. The
`ext_urlauth` feature is forwarded to it. Move this pin whenever Boundless moves
its imap-codec pin. `integration-test` still names crates.io imap-codec and does
not build against the fork.

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
`internally_handled_literal_continuation_is_accounted_once`. Boundless:
`append_literal_rejection_never_sends_the_rejected_message`,
`fragmented_fetch_literal_is_preserved_with_its_tagged_completion`.

### Security-layer handoff (058cc00)

`Client::take_unparsed_after_message` returns the bytes after a completely
received response, through imap-codec's fragmentizer, so a SASL security layer
or COMPRESS installed after the tagged OK receives them instead of the
plaintext parser. Evidence: imap-codec's
`handoff_requires_a_complete_message_and_retains_next_layer_bytes`; Boundless:
`independent_zlib_compression_boundary_refusal_invalid_data_and_cancel`.

### COMPRESS=DEFLATE transport (ede0c44)

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
