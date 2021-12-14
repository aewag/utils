use crate::{InOut, InOutBuf};
use core::slice;
use generic_array::{ArrayLength, GenericArray, typenum::NonZero};

/// The enum which controls which slice to use as input in the
/// [`ChunkProc`] trait.
pub enum InCtrl {
    /// Use input slice as input.
    In,
    /// Use temporary slice as input.
    Tmp,
}

/// Trait for processing data in chunks of fixed size.
// TODO: on GAT stabilization use it to ensure that
// length of slices in `BodyFn` is equal
pub trait ChunkProc<T>: Sized + sealed::Sealed {
    /// Split data into chunks of length `N` and tail. Process them
    /// using `proc_chunk` and `proc_tail` respectively.
    fn process_chunks<N, S, GenInFn, BodyFn, ProcChunkFn, ProcTailFn>(
        self,
        state: S,
        gen_in: GenInFn,
        body: BodyFn,
        proc_chunk: ProcChunkFn,
        proc_tail: ProcTailFn,
    ) where
        T: Default,
        N: ArrayLength<T> + NonZero,
        GenInFn: FnMut(&mut [T]) -> InCtrl,
        BodyFn: FnMut(Self, &mut [T]),
        ProcChunkFn: FnMut(&mut S, InOut<'_, GenericArray<T, N>>),
        ProcTailFn: FnMut(&mut S, InOutBuf<'_, T>);

    /// Split data into chunks of length `N` and tail. Process them
    /// using `f` together with chunks generated with `gen_chunk` and
    /// `gen_single`.
    fn process_gen_chunks<N, S, BodyFn, GenChunkFn, GenSliceFn>(
        self,
        state: S,
        body: BodyFn,
        mut gen_chunk: GenChunkFn,
        mut gen_slice: GenSliceFn,
    ) where
        T: Default,
        N: ArrayLength<T> + NonZero,
        BodyFn: FnMut(Self, &mut [T]),
        GenChunkFn: FnMut(&mut S, &mut GenericArray<T, N>),
        GenSliceFn: FnMut(&mut S, &mut [T]),
    {
        self.process_chunks::<N, _, _, _, _, _>(
            state,
            |_| InCtrl::Tmp,
            body,
            |state, chunk| gen_chunk(state, chunk.get_out()),
            |state, tail| gen_slice(state, tail.get_out()),
        )
    }
}

impl<'a, T> sealed::Sealed for InOutBuf<'a, T> {}

impl<'a, T> ChunkProc<T> for InOutBuf<'a, T> {
    fn process_chunks<N, S, GenInFn, BodyFn, ProcChunkFn, ProcTailFn>(
        self,
        mut state: S,
        mut gen_in: GenInFn,
        mut body: BodyFn,
        mut proc_chunk: ProcChunkFn,
        mut proc_tail: ProcTailFn,
    ) where
        T: Default,
        N: ArrayLength<T> + NonZero,
        GenInFn: FnMut(&mut [T]) -> InCtrl,
        BodyFn: FnMut(Self, &mut [T]),
        ProcChunkFn: FnMut(&mut S, InOut<'_, GenericArray<T, N>>),
        ProcTailFn: FnMut(&mut S, InOutBuf<'_, T>),
    {
        let (chunks, mut tail) = self.into_chunks::<N>();
        for mut chunk in chunks {
            let mut tmp = GenericArray::<T, N>::default();
            let in_val = chunk.reborrow().get_in();
            let proc_val = match gen_in(&mut tmp) {
                InCtrl::In => InOut::from((in_val, &mut tmp)),
                InCtrl::Tmp => InOut::from(&mut tmp),
            };
            proc_chunk(&mut state, proc_val);
            body(chunk.into_buf(), &mut tmp);
        }

        if tail.is_empty() {
            return;
        }
        let n = tail.len();
        let mut tmp = GenericArray::<T, N>::default();
        {
            let tmp = &mut tmp[..n];
            let in_val = tail.reborrow().get_in();
            let proc_val = match gen_in(tmp) {
                InCtrl::In => InOutBuf::new(in_val, tmp).unwrap(),
                InCtrl::Tmp => InOutBuf::from(tmp),
            };
            proc_tail(&mut state, proc_val);
        }
        body(tail, &mut tmp[..n]);
    }
}

impl<'a, T> sealed::Sealed for &'a [T] {}

impl<'a, T> ChunkProc<T> for &'a [T] {
    fn process_chunks<N, S, GenInFn, BodyFn, ProcChunkFn, ProcTailFn>(
        self,
        mut state: S,
        mut gen_in: GenInFn,
        mut body: BodyFn,
        mut proc_chunk: ProcChunkFn,
        mut proc_tail: ProcTailFn,
    ) where
        T: Default,
        N: ArrayLength<T> + NonZero,
        GenInFn: FnMut(&mut [T]) -> InCtrl,
        BodyFn: FnMut(Self, &mut [T]),
        ProcChunkFn: FnMut(&mut S, InOut<'_, GenericArray<T, N>>),
        ProcTailFn: FnMut(&mut S, InOutBuf<'_, T>),
    {
        let (chunks, tail) = into_chunks::<_, N>(self);
        for chunk in chunks {
            let mut tmp = GenericArray::<T, N>::default();
            let proc_val = match gen_in(&mut tmp) {
                InCtrl::In => InOut::from((chunk, &mut tmp)),
                InCtrl::Tmp => InOut::from(&mut tmp),
            };
            proc_chunk(&mut state, proc_val);
            body(chunk, &mut tmp);
        }

        if tail.is_empty() {
            return;
        }
        let n = tail.len();
        let mut tmp = GenericArray::<T, N>::default();
        {
            let tmp = &mut tmp[..n];
            let proc_val = match gen_in(tmp) {
                InCtrl::In => InOutBuf::new(tail, tmp).unwrap(),
                InCtrl::Tmp => InOutBuf::from(tmp),
            };
            proc_tail(&mut state, proc_val);
        }
        body(tail, &mut tmp[..n]);
    }
}

impl<'a, T> sealed::Sealed for &'a mut [T] {}

impl<'a, T> ChunkProc<T> for &'a mut [T] {
    fn process_chunks<N, S, GenInFn, BodyFn, ProcChunkFn, ProcTailFn>(
        self,
        mut state: S,
        mut gen_in: GenInFn,
        mut body: BodyFn,
        mut proc_chunk: ProcChunkFn,
        mut proc_tail: ProcTailFn,
    ) where
        T: Default,
        N: ArrayLength<T> + NonZero,
        GenInFn: FnMut(&mut [T]) -> InCtrl,
        BodyFn: FnMut(Self, &mut [T]),
        ProcChunkFn: FnMut(&mut S, InOut<'_, GenericArray<T, N>>),
        ProcTailFn: FnMut(&mut S, InOutBuf<'_, T>),
    {
        let (chunks, tail) = into_chunks_mut::<_, N>(self);
        for chunk in chunks {
            let mut tmp = GenericArray::<T, N>::default();
            let proc_val = match gen_in(&mut tmp) {
                InCtrl::In => InOut::from((&*chunk, &mut tmp)),
                InCtrl::Tmp => InOut::from(&mut tmp),
            };
            proc_chunk(&mut state, proc_val);
            body(chunk, &mut tmp);
        }

        if tail.is_empty() {
            return;
        }
        let n = tail.len();
        let mut tmp = GenericArray::<T, N>::default();
        {
            let tmp = &mut tmp[..n];
            let proc_val = match gen_in(tmp) {
                InCtrl::In => InOutBuf::new(tail, tmp).unwrap(),
                InCtrl::Tmp => InOutBuf::from(tmp),
            };
            proc_tail(&mut state, proc_val);
        }
        body(tail, &mut tmp[..n]);
    }
}

fn into_chunks<'a, T, N: ArrayLength<T>>(buf: &'a [T]) -> (&'a [GenericArray<T, N>], &'a [T]) {
    let chunks = buf.len() / N::USIZE;
    let tail_pos = N::USIZE * chunks;
    let tail_len = buf.len() - tail_pos;
    unsafe {
        let chunks = slice::from_raw_parts(buf.as_ptr() as *const GenericArray<T, N>, chunks);
        let tail = slice::from_raw_parts(buf.as_ptr().add(tail_pos), tail_len);
        (chunks, tail)
    }
}

fn into_chunks_mut<'a, T, N: ArrayLength<T>>(
    buf: &'a mut [T],
) -> (&'a mut [GenericArray<T, N>], &'a mut [T]) {
    let chunks = buf.len() / N::USIZE;
    let tail_pos = N::USIZE * chunks;
    let tail_len = buf.len() - tail_pos;
    unsafe {
        let chunks = slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut GenericArray<T, N>, chunks);
        let tail = slice::from_raw_parts_mut(buf.as_mut_ptr().add(tail_pos), tail_len);
        (chunks, tail)
    }
}

mod sealed {
    /// Sealed trait for `ChunkProc`.
    pub trait Sealed {}
}
