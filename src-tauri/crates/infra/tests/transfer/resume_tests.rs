use super::*;

#[test]
fn bitmap_should_mark_and_find_missing() {
    let mut bitmap = empty_bitmap(10);
    mark_chunk_done(bitmap.as_mut_slice(), 0).expect("mark 0");
    mark_chunk_done(bitmap.as_mut_slice(), 3).expect("mark 3");
    mark_chunk_done(bitmap.as_mut_slice(), 9).expect("mark 9");

    let missing = missing_chunks(bitmap.as_slice(), 10);
    assert_eq!(missing, vec![1, 2, 4, 5, 6, 7, 8]);
}

#[test]
fn completed_bytes_should_handle_tail_chunk() {
    let chunk_size = 1024;
    let chunk_count = chunk_count(2500, chunk_size);
    let mut bitmap = empty_bitmap(chunk_count);
    mark_chunk_done(bitmap.as_mut_slice(), 0).expect("mark 0");
    mark_chunk_done(bitmap.as_mut_slice(), 2).expect("mark 2");

    let bytes = completed_bytes(bitmap.as_slice(), chunk_count, chunk_size, 2500);
    assert_eq!(bytes, 1476);
}
