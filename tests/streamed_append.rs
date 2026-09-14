use imap_codec::{
    encode::StreamedAppend,
    imap_types::{
        core::{LiteralMode, Tag},
        mailbox::Mailbox,
    },
};
use imap_next::{
    client::{Client, Event, Options},
    Interrupt, Io, State,
};
fn ready() -> Client {
    let mut c = Client::new(Options::default());
    c.enqueue_input(b"* OK fixture\r\n");
    assert!(matches!(c.next(), Ok(Event::GreetingReceived { .. })));
    c
}
fn spec(length: u64, binary: bool, mode: LiteralMode) -> StreamedAppend {
    StreamedAppend {
        tag: Tag::try_from("A").unwrap(),
        mailbox: Mailbox::Inbox,
        flags: vec![],
        date: None,
        length,
        binary,
        mode,
    }
}
#[test]
fn synchronous_upload_waits_for_permission_checks_chunks_and_requires_explicit_finish() {
    let mut c = ready();
    let h = c
        .enqueue_streamed_append(spec(131072, false, LiteralMode::Sync))
        .unwrap();
    assert!(c.append_chunk(h, vec![1]).is_err());
    assert!(
        matches!(c.next(),Err(Interrupt::Io(Io::Output(v))) if v==b"A APPEND INBOX {131072}\r\n")
    );
    assert!(matches!(c.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
    c.enqueue_input(b"+ go\r\n");
    assert!(matches!(c.next(),Ok(Event::AppendReady{handle}) if handle==h));
    assert!(c.append_finish(h).is_err());
    assert!(c.append_chunk(h, vec![1; 65537]).is_err());
    assert!(c.append_chunk(h, vec![0]).is_err());
    for written in [65536, 131072] {
        c.append_chunk(h, vec![42; 65536]).unwrap();
        assert!(c.append_chunk(h, vec![1]).is_err());
        assert!(matches!(c.next(),Err(Interrupt::Io(Io::Output(v))) if v==vec![42;65536]));
        assert!(
            matches!(c.next(),Ok(Event::AppendChunkSent{handle,written:w}) if handle==h && w==written)
        );
    }
    assert!(c.append_chunk(h, vec![1]).is_err());
    assert!(matches!(c.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
    c.append_finish(h).unwrap();
    assert!(matches!(c.next(),Err(Interrupt::Io(Io::Output(v))) if v==b"\r\n"));
    assert!(matches!(c.next(),Ok(Event::AppendSent{handle}) if handle==h));
    assert!(c.append_finish(h).is_err());
    c.enqueue_input(b"A OK [APPENDUID 7 8] stored\r\n");
    assert!(matches!(c.next(), Ok(Event::StatusReceived { .. })));
}
#[test]
fn rejection_and_handle_generation_never_send_a_rejected_literal() {
    let mut c = ready();
    let h = c
        .enqueue_streamed_append(spec(i64::MAX as u64, false, LiteralMode::Sync))
        .unwrap();
    assert!(matches!(c.next(), Err(Interrupt::Io(Io::Output(_)))));
    assert!(matches!(c.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
    c.enqueue_input(b"A NO quota\r\n");
    assert!(matches!(c.next(),Ok(Event::AppendTerminated{handle,..}) if handle==h));
    assert!(c.append_chunk(h, vec![1]).is_err());
    let mut other = ready();
    let h2 = other
        .enqueue_streamed_append(spec(1, true, LiteralMode::NonSync))
        .unwrap();
    assert!(
        matches!(other.next(),Err(Interrupt::Io(Io::Output(v))) if v==b"A APPEND INBOX ~{1+}\r\n")
    );
    assert!(matches!(other.next(), Ok(Event::AppendReady { .. })));
    assert!(other.append_chunk(h, vec![0]).is_err());
    other.append_chunk(h2, vec![0]).unwrap();
    assert!(matches!(other.next(),Err(Interrupt::Io(Io::Output(v))) if v==[0]));
    assert!(matches!(
        other.next(),
        Ok(Event::AppendChunkSent { written: 1, .. })
    ));
}
#[test]
fn mailbox_literals_and_zero_length_body_have_separate_continuations() {
    let mut c = ready();
    let mut s = spec(0, false, LiteralMode::Sync);
    s.mailbox = Mailbox::try_from("a\nb").unwrap();
    let h = c.enqueue_streamed_append(s).unwrap();
    assert!(matches!(c.next(),Err(Interrupt::Io(Io::Output(v))) if v==b"A APPEND {3}\r\n"));
    assert!(matches!(c.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
    c.enqueue_input(b"+ mailbox\r\n");
    assert!(matches!(c.next(),Err(Interrupt::Io(Io::Output(v))) if v==b"a\nb {0}\r\n"));
    assert!(matches!(c.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
    assert!(c.append_finish(h).is_err());
    c.enqueue_input(b"+ body\r\n");
    assert!(matches!(c.next(), Ok(Event::AppendReady { .. })));
    c.append_finish(h).unwrap();
    assert!(matches!(c.next(),Err(Interrupt::Io(Io::Output(v))) if v==b"\r\n"));
    assert!(matches!(c.next(), Ok(Event::AppendSent { .. })));
}
