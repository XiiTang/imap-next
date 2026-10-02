# External IO integration patch

Base: imap-next 0.3.4, commit
`9938818b74a4114f73b0a99c31bdd7c8991ebe0c`.

- Make codec normalization quirks an explicit feature, enabled by the upstream
  default but removable by an embedding application's `default-features = false`.
- Expose borrowed exact received message bytes and a resettable count of consumed
  complete-message bytes. The latter includes internally handled literal
  continuations and lets an external driver bound unread input without decoding it
  a second time. Authentication replies remain secret data for the caller.
- Treat tagged NO as well as BAD as rejection of a synchronizing command literal,
  allowing the session to continue without sending the rejected payload.

The library continues to own command literals, AUTHENTICATE and IDLE state.
Transport, native authentication, deadlines, resource budgets and public delivery
remain responsibilities of the embedding application.

## UTF-8 codec validation (2026-10-02)

Align the engine dependency with the Boundless imap-codec SHA containing upstream
#730. Keep one codec/types revision throughout the client and adapter; this changes
no imap-next runtime code or public contract. Codec and downstream tests cover
quoted UTF-8 validation and the existing session/literal/compression paths.
