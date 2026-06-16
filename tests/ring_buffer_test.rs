use hotspot_hub::ring_buffer::RingBuffer;

#[test]
fn keeps_only_newest_items() {
    let mut buffer = RingBuffer::new(3);
    buffer.push(1);
    buffer.push(2);
    buffer.push(3);
    buffer.push(4);

    assert_eq!(buffer.to_vec(), vec![2, 3, 4]);
}

#[test]
fn zero_capacity_stays_empty() {
    let mut buffer = RingBuffer::new(0);
    buffer.push(1);

    assert_eq!(buffer.to_vec(), Vec::<i32>::new());
}
