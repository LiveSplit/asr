use core::ffi::c_int;

// This is mostly copied from the `compiler_builtins` crate:
// https://github.com/rust-lang/compiler-builtins/blob/dbb244a34075d622d821deaa7f8e7f322bed958a/src/mem/mod.rs

// Unfortunately the crate does not work on stable Rust and it is not part of
// `core` when compiling for WASI.

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    copy_forward(dest, src, n);
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let delta = (dest as usize).wrapping_sub(src as usize);
    if delta >= n {
        // We can copy forwards because either dest is far enough ahead of src,
        // or src is ahead of dest (and delta overflowed).
        copy_forward(dest, src, n);
    } else {
        copy_backward(dest, src, n);
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: c_int, n: usize) -> *mut u8 {
    set_bytes(s, c as u8, n);
    s
}

#[no_mangle]
pub const unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub const unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return 1;
        }
        i += 1;
    }
    0
}

#[inline]
#[no_mangle]
pub extern "C" fn fmod(x: f64, y: f64) -> f64 {
    libm::fmod(x, y)
}

#[inline]
#[no_mangle]
pub extern "C" fn fmodf(x: f32, y: f32) -> f32 {
    libm::fmodf(x, y)
}

const WORD_SIZE: usize = core::mem::size_of::<usize>();
const WORD_MASK: usize = WORD_SIZE - 1;

// If the number of bytes involved exceed this threshold we will opt in word-wise copy.
// The value here selected is max(2 * WORD_SIZE, 16):
// * We need at least 2 * WORD_SIZE bytes to guarantee that at least 1 word will be copied through
//   word-wise copy.
// * The word-wise copy logic needs to perform some checks so it has some small overhead.
//   ensures that even on 32-bit platforms we have copied at least 8 bytes through
//   word-wise copy so the saving of word-wise copy outweights the fixed overhead.
const WORD_COPY_THRESHOLD: usize = if 2 * WORD_SIZE > 16 {
    2 * WORD_SIZE
} else {
    16
};

#[inline(always)]
pub unsafe fn copy_forward(mut dest: *mut u8, mut src: *const u8, mut n: usize) {
    #[inline(always)]
    unsafe fn copy_forward_bytes(mut dest: *mut u8, mut src: *const u8, n: usize) {
        let dest_end = dest.add(n);
        while dest < dest_end {
            *dest = *src;
            dest = dest.add(1);
            src = src.add(1);
        }
    }

    #[inline(always)]
    unsafe fn copy_forward_aligned_words(dest: *mut u8, src: *const u8, n: usize) {
        let mut dest_usize = dest as *mut usize;
        let mut src_usize = src as *mut usize;
        let dest_end = dest.add(n) as *mut usize;

        while dest_usize < dest_end {
            *dest_usize = *src_usize;
            dest_usize = dest_usize.add(1);
            src_usize = src_usize.add(1);
        }
    }

    #[inline(always)]
    unsafe fn copy_forward_misaligned_words(dest: *mut u8, src: *const u8, n: usize) {
        let mut dest_usize = dest as *mut usize;
        let mut src_usize = src as *mut usize;
        let dest_end = dest.add(n) as *mut usize;

        while dest_usize < dest_end {
            *dest_usize = read_usize_unaligned(src_usize);
            dest_usize = dest_usize.add(1);
            src_usize = src_usize.add(1);
        }
    }

    if n >= WORD_COPY_THRESHOLD {
        // Align dest
        // Because of n >= 2 * WORD_SIZE, dst_misalignment < n
        let dest_misalignment = (dest as usize).wrapping_neg() & WORD_MASK;
        copy_forward_bytes(dest, src, dest_misalignment);
        dest = dest.add(dest_misalignment);
        src = src.add(dest_misalignment);
        n -= dest_misalignment;

        let n_words = n & !WORD_MASK;
        let src_misalignment = src as usize & WORD_MASK;
        if src_misalignment == 0 {
            copy_forward_aligned_words(dest, src, n_words);
        } else {
            copy_forward_misaligned_words(dest, src, n_words);
        }
        dest = dest.add(n_words);
        src = src.add(n_words);
        n -= n_words;
    }
    copy_forward_bytes(dest, src, n);
}

#[inline(always)]
pub unsafe fn copy_backward(dest: *mut u8, src: *const u8, mut n: usize) {
    // The following backward copy helper functions uses the pointers past the end
    // as their inputs instead of pointers to the start!
    #[inline(always)]
    unsafe fn copy_backward_bytes(mut dest: *mut u8, mut src: *const u8, n: usize) {
        let dest_start = dest.sub(n);
        while dest_start < dest {
            dest = dest.sub(1);
            src = src.sub(1);
            *dest = *src;
        }
    }

    #[inline(always)]
    unsafe fn copy_backward_aligned_words(dest: *mut u8, src: *const u8, n: usize) {
        let mut dest_usize = dest as *mut usize;
        let mut src_usize = src as *mut usize;
        let dest_start = dest.sub(n) as *mut usize;

        while dest_start < dest_usize {
            dest_usize = dest_usize.sub(1);
            src_usize = src_usize.sub(1);
            *dest_usize = *src_usize;
        }
    }

    #[inline(always)]
    unsafe fn copy_backward_misaligned_words(dest: *mut u8, src: *const u8, n: usize) {
        let mut dest_usize = dest as *mut usize;
        let mut src_usize = src as *mut usize;
        let dest_start = dest.sub(n) as *mut usize;

        while dest_start < dest_usize {
            dest_usize = dest_usize.sub(1);
            src_usize = src_usize.sub(1);
            *dest_usize = read_usize_unaligned(src_usize);
        }
    }

    let mut dest = dest.add(n);
    let mut src = src.add(n);

    if n >= WORD_COPY_THRESHOLD {
        // Align dest
        // Because of n >= 2 * WORD_SIZE, dst_misalignment < n
        let dest_misalignment = dest as usize & WORD_MASK;
        copy_backward_bytes(dest, src, dest_misalignment);
        dest = dest.sub(dest_misalignment);
        src = src.sub(dest_misalignment);
        n -= dest_misalignment;

        let n_words = n & !WORD_MASK;
        let src_misalignment = src as usize & WORD_MASK;
        if src_misalignment == 0 {
            copy_backward_aligned_words(dest, src, n_words);
        } else {
            copy_backward_misaligned_words(dest, src, n_words);
        }
        dest = dest.sub(n_words);
        src = src.sub(n_words);
        n -= n_words;
    }
    copy_backward_bytes(dest, src, n);
}

#[inline(always)]
pub unsafe fn set_bytes(mut s: *mut u8, c: u8, mut n: usize) {
    #[inline(always)]
    pub unsafe fn set_bytes_bytes(mut s: *mut u8, c: u8, n: usize) {
        let end = s.add(n);
        while s < end {
            *s = c;
            s = s.add(1);
        }
    }

    #[inline(always)]
    pub unsafe fn set_bytes_words(s: *mut u8, c: u8, n: usize) {
        let mut broadcast = c as usize;
        let mut bits = 8;
        while bits < WORD_SIZE * 8 {
            broadcast |= broadcast << bits;
            bits *= 2;
        }

        let mut s_usize = s as *mut usize;
        let end = s.add(n) as *mut usize;

        while s_usize < end {
            *s_usize = broadcast;
            s_usize = s_usize.add(1);
        }
    }

    if n >= WORD_COPY_THRESHOLD {
        // Align s
        // Because of n >= 2 * WORD_SIZE, dst_misalignment < n
        let misalignment = (s as usize).wrapping_neg() & WORD_MASK;
        set_bytes_bytes(s, c, misalignment);
        s = s.add(misalignment);
        n -= misalignment;

        let n_words = n & !WORD_MASK;
        set_bytes_words(s, c, n_words);
        s = s.add(n_words);
        n -= n_words;
    }
    set_bytes_bytes(s, c, n);
}

unsafe fn read_usize_unaligned(x: *const usize) -> usize {
    // Do not use `core::ptr::read_unaligned` here, since it calls `copy_nonoverlapping` which
    // is translated to memcpy in LLVM.
    let x_read = (x as *const [u8; core::mem::size_of::<usize>()]).read();
    core::mem::transmute(x_read)
}
