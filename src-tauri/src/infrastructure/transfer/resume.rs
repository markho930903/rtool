use crate::core::{AppError, AppResult};

pub fn chunk_count(size_bytes: u64, chunk_size: u32) -> u32 {
    if chunk_size == 0 {
        return 0;
    }
    let chunk_size_u64 = u64::from(chunk_size);
    size_bytes.div_ceil(chunk_size_u64) as u32
}

pub fn bitmap_len(chunk_count: u32) -> usize {
    chunk_count.div_ceil(8) as usize
}

pub fn empty_bitmap(chunk_count: u32) -> Vec<u8> {
    vec![0u8; bitmap_len(chunk_count)]
}

pub fn mark_chunk_done(bitmap: &mut [u8], chunk_index: u32) -> AppResult<()> {
    let byte_index = (chunk_index / 8) as usize;
    if byte_index >= bitmap.len() {
        return Err(AppError::new(
            "transfer_bitmap_out_of_range",
            "断点位图越界",
        ));
    }
    let bit_index = (chunk_index % 8) as u8;
    bitmap[byte_index] |= 1u8 << bit_index;
    Ok(())
}

pub fn is_chunk_done(bitmap: &[u8], chunk_index: u32) -> bool {
    let byte_index = (chunk_index / 8) as usize;
    if byte_index >= bitmap.len() {
        return false;
    }
    let bit_index = (chunk_index % 8) as u8;
    bitmap[byte_index] & (1u8 << bit_index) != 0
}

pub fn missing_chunks(bitmap: &[u8], chunk_count: u32) -> Vec<u32> {
    let mut missing = Vec::new();
    for index in 0..chunk_count {
        if !is_chunk_done(bitmap, index) {
            missing.push(index);
        }
    }
    missing
}

pub fn completed_bytes(bitmap: &[u8], chunk_count: u32, chunk_size: u32, total_size: u64) -> u64 {
    let mut completed = 0u64;
    for index in 0..chunk_count {
        if !is_chunk_done(bitmap, index) {
            continue;
        }

        let start = u64::from(index) * u64::from(chunk_size);
        if start >= total_size {
            continue;
        }
        let remaining = total_size - start;
        completed += remaining.min(u64::from(chunk_size));
    }
    completed
}

#[cfg(test)]
mod tests {
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
}
