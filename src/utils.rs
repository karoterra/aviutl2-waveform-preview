pub struct NChunks<'a, T> {
    slice: &'a [T],
    chunks: usize,
    index: usize,
    start: usize,
}

impl<'a, T> NChunks<'a, T> {
    pub fn new(slice: &'a [T], chunks: usize) -> Self {
        Self {
            slice,
            chunks,
            index: 0,
            start: 0,
        }
    }
}

impl<'a, T> Iterator for NChunks<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.chunks || self.chunks == 0 {
            return None;
        }

        let len = self.slice.len();
        let base = len / self.chunks;
        let rem = len % self.chunks;

        let size = base + usize::from(self.index < rem);
        let end = self.start + size;

        let chunk = &self.slice[self.start..end];

        self.start = end;
        self.index += 1;

        Some(chunk)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.chunks - self.index;
        (remaining, Some(remaining))
    }
}

impl<T> ExactSizeIterator for NChunks<'_, T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_evenly_when_length_is_divisible_by_chunks() {
        let values = [0, 1, 2, 3, 4, 5];

        let chunks: Vec<&[i32]> = NChunks::new(&values, 3).collect();

        assert_eq!(chunks, vec![&[0, 1][..], &[2, 3][..], &[4, 5][..]]);
    }

    #[test]
    fn distributes_remainder_to_front_chunks() {
        let values = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        let chunks: Vec<&[i32]> = NChunks::new(&values, 3).collect();

        assert_eq!(
            chunks,
            vec![&[0, 1, 2, 3][..], &[4, 5, 6][..], &[7, 8, 9][..]]
        );
    }

    #[test]
    fn returns_exactly_requested_number_of_chunks() {
        let values = [0, 1, 2, 3, 4];

        let chunks: Vec<&[i32]> = NChunks::new(&values, 3).collect();

        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn chunk_lengths_differ_by_at_most_one() {
        let values = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        let chunks: Vec<&[i32]> = NChunks::new(&values, 4).collect();
        let lengths: Vec<usize> = chunks.iter().map(|chunk| chunk.len()).collect();

        assert_eq!(lengths, vec![3, 3, 2, 2]);

        let min = lengths.iter().copied().min().unwrap();
        let max = lengths.iter().copied().max().unwrap();

        assert!(max - min <= 1);
    }

    #[test]
    fn concatenating_chunks_restores_original_slice() {
        let values = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        let restored: Vec<i32> = NChunks::new(&values, 4)
            .flat_map(|chunk| chunk.iter().copied())
            .collect();

        assert_eq!(restored, values);
    }

    #[test]
    fn returns_empty_chunks_when_chunks_exceed_length() {
        let values = [0, 1, 2];

        let chunks: Vec<&[i32]> = NChunks::new(&values, 5).collect();

        assert_eq!(chunks, vec![&[0][..], &[1][..], &[2][..], &[][..], &[][..]]);
    }

    #[test]
    fn empty_slice_returns_requested_number_of_empty_chunks() {
        let values: [i32; 0] = [];

        let chunks: Vec<&[i32]> = NChunks::new(&values, 3).collect();
        let expected: Vec<&[i32]> = vec![&[][..], &[][..], &[][..]];

        assert_eq!(chunks, expected);
    }

    #[test]
    fn size_hint_reports_remaining_chunks() {
        let values = [0, 1, 2, 3, 4];

        let mut chunks = NChunks::new(&values, 3);

        assert_eq!(chunks.size_hint(), (3, Some(3)));
        assert_eq!(chunks.len(), 3);

        assert_eq!(chunks.next(), Some(&[0, 1][..]));

        assert_eq!(chunks.size_hint(), (2, Some(2)));
        assert_eq!(chunks.len(), 2);

        assert_eq!(chunks.next(), Some(&[2, 3][..]));
        assert_eq!(chunks.next(), Some(&[4][..]));
        assert_eq!(chunks.next(), None);

        assert_eq!(chunks.size_hint(), (0, Some(0)));
        assert_eq!(chunks.len(), 0);
    }
}
