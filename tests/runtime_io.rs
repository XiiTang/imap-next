use imap_codec::{decode::Decoder, imap_types::IntoStatic, CommandCodec};
use imap_next::{client::{Client, Event, Options}, Interrupt, Io, State};

fn client() -> Client {
    let mut client = Client::new(Options::default());
    client.enqueue_input(b"* OK ready\r\n");
    assert!(matches!(client.next(), Ok(Event::GreetingReceived { .. })));
    assert_eq!(client.received_message_bytes(), b"* OK ready\r\n");
    assert_eq!(client.take_consumed_input(), 12);
    client
}

#[test]
fn exact_received_bytes_and_consumption_follow_each_message() {
    let mut client = client();
    client.enqueue_input(b"* 1 EXISTS\r\n* 2 RECENT\r\n");
    for raw in [b"* 1 EXISTS\r\n", b"* 2 RECENT\r\n"] {
        assert!(matches!(client.next(), Ok(Event::DataReceived { .. })));
        assert_eq!(client.received_message_bytes(), raw);
        assert_eq!(client.take_consumed_input(), raw.len());
        assert_eq!(client.take_consumed_input(), 0);
    }
    assert!(matches!(client.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
}

#[test]
fn no_rejects_a_pending_literal_without_sending_its_payload() {
    let mut client = client();
    let (_, command) = CommandCodec::default().decode(b"A1 APPEND INBOX {4}\r\ntest\r\n").unwrap();
    let handle = client.enqueue_command(command.into_static());
    assert!(matches!(client.next(), Err(Interrupt::Io(Io::Output(bytes))) if bytes == b"A1 APPEND INBOX {4}\r\n"));
    assert!(matches!(client.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
    client.enqueue_input(b"A1 NO quota exceeded\r\n");
    assert!(matches!(client.next(), Ok(Event::CommandRejected { handle: h, .. }) if h == handle));
    assert_eq!(client.received_message_bytes(), b"A1 NO quota exceeded\r\n");
    assert_eq!(client.take_consumed_input(), 22);
    assert!(matches!(client.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
}

#[test]
fn internally_handled_literal_continuation_is_accounted_once() {
    let mut client = client();
    let (_, command) = CommandCodec::default().decode(b"A1 APPEND INBOX {4}\r\ntest\r\n").unwrap();
    client.enqueue_command(command.into_static());
    assert!(matches!(client.next(), Err(Interrupt::Io(Io::Output(_)))));
    assert!(matches!(client.next(), Err(Interrupt::Io(Io::NeedMoreInput))));
    client.enqueue_input(b"+ accepted\r\n");
    assert!(matches!(client.next(), Err(Interrupt::Io(Io::Output(_)))));
    assert_eq!(client.take_consumed_input(), 12);
    assert_eq!(client.take_consumed_input(), 0);
}
