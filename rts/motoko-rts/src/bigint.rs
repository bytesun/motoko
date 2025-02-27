//! Implements big int support:
//!
//! - libtommath memory management
//! - libtommath wrappers
//! - (s)leb128 encoding/decoding for bigints

/*
A libtommath arbitrary precision integer is a struct (`mp_int`) that contains a pointer to a data
array.

 - The libtommath library never allocates the struct, so we are in full control.

   We allocate that struct on the stack if we need it temporarily, or embed it in a TAG_BIGINT
   object when we persist the number.

 - The data array is allocated with mp_calloc() and mp_realloc(), and these functions are _only_ used
   to allocate the data array.

   We provide these calls, allocate TAG_BIGINT objects (leaving space for the mp_int), and pass a
   pointer _into_ this object back to libtommath, which stores it in the `mp_digit* dp`
   field of the struct.

When persisting a `mp_int` (presumably stack-allocated), we know that the `mp_digit` pointer
points to a `TAG_BIGINT` with sufficient space for the `mp_int` data. We copy the `mp_int`
there, and use the overall `TAG_BIGINT` as the bignum object.

This scheme makes the following assumptions:

 - libtommath never modifies the data on the heap.
   (or put differently, we only pass those to libtommath when they are immutable)
 - libtommath uses mp_calloc() and mp_realloc() _only_ to allocate the `mp_digit *` array.
*/

use crate::buf::{read_byte, Buf};
use crate::mem_utils::memcpy_bytes;
use crate::memory::Memory;
use crate::tommath_bindings::*;
use crate::types::{size_of, BigInt, Bytes, Value, TAG_BIGINT};

use motoko_rts_macros::ic_mem_fn;

unsafe fn mp_alloc<M: Memory>(mem: &mut M, size: Bytes<u32>) -> *mut u8 {
    let ptr = mem.alloc_words(size_of::<BigInt>() + size.to_words());
    // NB. Cannot use as_bigint() here as header is not written yet
    let blob = ptr.get_ptr() as *mut BigInt;
    (*blob).header.tag = TAG_BIGINT;
    // libtommath stores the size of the object in alloc as count of mp_digits (u64)
    let size = size.as_usize();
    debug_assert_eq!((size % core::mem::size_of::<mp_digit>()), 0);
    (*blob).mp_int.alloc = (size / core::mem::size_of::<mp_digit>()) as i32;
    blob.payload_addr() as *mut u8
}

#[ic_mem_fn]
pub unsafe fn mp_calloc<M: Memory>(
    mem: &mut M,
    n_elems: usize,
    elem_size: Bytes<usize>,
) -> *mut libc::c_void {
    debug_assert_eq!(elem_size.0, core::mem::size_of::<mp_digit>());
    // Overflow check for the following multiplication
    if n_elems > 1 << 30 {
        bigint_trap();
    }
    let size = Bytes((n_elems * elem_size.0) as u32);
    let payload = mp_alloc(mem, size) as *mut u32;

    // NB. alloc_bytes rounds up to words so we do the same here to set the whole buffer
    for i in 0..size.to_words().as_usize() {
        *payload.add(i as usize) = 0;
    }

    payload as *mut _
}

#[ic_mem_fn]
pub unsafe fn mp_realloc<M: Memory>(
    mem: &mut M,
    ptr: *mut libc::c_void,
    old_size: Bytes<u32>,
    new_size: Bytes<u32>,
) -> *mut libc::c_void {
    let bigint = BigInt::from_payload(ptr as *mut mp_digit);

    debug_assert_eq!((*bigint).header.tag, TAG_BIGINT);
    debug_assert_eq!(bigint.len(), old_size);

    if new_size > bigint.len() {
        let new_ptr = mp_alloc(mem, new_size);
        memcpy_bytes(new_ptr as usize, ptr as usize, old_size);
        new_ptr as *mut _
    } else if new_size == bigint.len() {
        ptr
    } else {
        // libtommath only shrinks via mp_shrink and we do not use that function, so this should not
        // happen.
        panic!("mp_realloc: trying to shrink");
    }
}

#[no_mangle]
pub unsafe extern "C" fn mp_free(_ptr: *mut libc::c_void, _size: u32) {}

/*
Note on libtommath error handling
---------------------------------

Most libtommath operations return an enum to signal error codes. These are (see tommath.h):

   MP_OKAY  = 0,   /* no error */
   MP_ERR   = -1,  /* unknown error */
   MP_MEM   = -2,  /* out of mem */
   MP_VAL   = -3,  /* invalid input */
   MP_ITER  = -4,  /* maximum iterations reached */
   MP_BUF   = -5   /* buffer overflow, supplied buffer too small */

We will never hit MP_MEM, because our allocation functions trap if they cannot allocate. But the
others can happen (e.g. division by 0). In that case, we call a trap function provided by the
compiler.
*/

// Trap function generated by compiler. Originally added in e2ca6a1. I think this could be
// simplified now by calling rts_trap and removing generated code from the compiler.
extern "C" {
    fn bigint_trap() -> !;
}

unsafe fn check(err: mp_err) {
    if err != 0 {
        bigint_trap();
    }
}

unsafe fn mp_get_u32(p: *const mp_int) -> u32 {
    mp_get_i32(p) as u32
}

#[cfg(feature = "ic")]
unsafe fn mp_get_u64(p: *const mp_int) -> u64 {
    mp_get_i64(p) as u64
}

unsafe fn mp_isneg(p: *const mp_int) -> bool {
    (*p).sign == 1
}

unsafe fn mp_iszero(p: *const mp_int) -> bool {
    (*p).used == 0
}

// Allocates a mp_int on the stack
unsafe fn tmp_bigint() -> mp_int {
    let mut i: mp_int = core::mem::zeroed();
    check(mp_init(&mut i));
    i
}

// Persists an mp_int from the stack on the heap
unsafe fn persist_bigint(i: mp_int) -> Value {
    if i.dp == core::ptr::null_mut() {
        panic!("persist_bigint: dp == NULL?");
    }
    let r = BigInt::from_payload(i.dp);
    if (*r).mp_int.alloc != i.alloc {
        panic!("persist_bigint: alloc changed?");
    }
    (*r).mp_int = i;
    Value::from_ptr(r as usize)
}

#[no_mangle]
pub unsafe extern "C" fn bigint_of_word32(w: u32) -> Value {
    let mut i = tmp_bigint();
    mp_set_u32(&mut i, w);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_of_int32(j: i32) -> Value {
    let mut i = tmp_bigint();
    mp_set_i32(&mut i, j);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_to_word32_wrap(p: Value) -> u32 {
    mp_get_u32(p.as_bigint().mp_int_ptr())
}

#[no_mangle]
unsafe extern "C" fn bigint_to_word32_trap(p: Value) -> u32 {
    let mp_int = p.as_bigint().mp_int_ptr();

    if mp_isneg(mp_int) || mp_count_bits(mp_int) > 32 {
        bigint_trap();
    }

    mp_get_u32(mp_int)
}

// a : BigInt, msg : Blob
#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_to_word32_trap_with(p: Value, msg: Value) -> u32 {
    let mp_int = p.as_bigint().mp_int_ptr();

    if mp_isneg(mp_int) || mp_count_bits(mp_int) > 32 {
        crate::rts_trap(msg.as_blob().payload_addr(), msg.as_blob().len());
    }

    mp_get_u32(mp_int)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_to_word64_wrap(p: Value) -> u64 {
    mp_get_u64(p.as_bigint().mp_int_ptr())
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_to_word64_trap(p: Value) -> u64 {
    let mp_int = p.as_bigint().mp_int_ptr();

    if mp_isneg(mp_int) || mp_count_bits(mp_int) > 64 {
        bigint_trap();
    }

    mp_get_u64(mp_int)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_of_word64(w: u64) -> Value {
    let mut i = tmp_bigint();
    mp_set_u64(&mut i, w);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_of_int64(j: i64) -> Value {
    let mut i = tmp_bigint();
    mp_set_i64(&mut i, j);
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_of_float64(j: f64) -> Value {
    if j < 1073741824.0 && j > -1073741825.0 {
        return Value::from_signed_scalar(j as i32);
    }
    let mut i = tmp_bigint();
    check(mp_set_double(&mut i, j));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_to_float64(p: Value) -> f64 {
    if p.is_scalar() {
        p.get_signed_scalar() as f64
    } else {
        let mp_int = p.as_bigint().mp_int_ptr();
        mp_get_double(mp_int)
    }
}

#[no_mangle]
pub unsafe extern "C" fn bigint_eq(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) == 0
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_lt(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) < 0
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_gt(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) > 0
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_le(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) <= 0
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_ge(a: Value, b: Value) -> bool {
    mp_cmp(a.as_bigint().mp_int_ptr(), b.as_bigint().mp_int_ptr()) >= 0
}

#[no_mangle]
pub unsafe extern "C" fn bigint_add(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_add(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
    ));
    persist_bigint(i)
}

#[no_mangle]
pub unsafe extern "C" fn bigint_sub(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_sub(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
    ));
    persist_bigint(i)
}

#[no_mangle]
pub unsafe extern "C" fn bigint_mul(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_mul(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
    ));
    persist_bigint(i)
}

#[no_mangle]
pub unsafe extern "C" fn bigint_pow(a: Value, b: Value) -> Value {
    let exp = bigint_to_word32_trap(b);
    let mut i = tmp_bigint();
    check(mp_expt_u32(a.as_bigint().mp_int_ptr(), exp, &mut i));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_div(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_div(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        &mut i,
        core::ptr::null_mut(),
    ));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_rem(a: Value, b: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_div(
        a.as_bigint().mp_int_ptr(),
        b.as_bigint().mp_int_ptr(),
        core::ptr::null_mut(),
        &mut i,
    ));
    persist_bigint(i)
}

#[no_mangle]
pub unsafe extern "C" fn bigint_neg(a: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_neg(a.as_bigint().mp_int_ptr(), &mut i));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_abs(a: Value) -> Value {
    let mut i = tmp_bigint();
    check(mp_abs(a.as_bigint().mp_int_ptr(), &mut i));
    persist_bigint(i)
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_isneg(a: Value) -> bool {
    mp_isneg(a.as_bigint().mp_int_ptr())
}

#[cfg(feature = "ic")]
#[no_mangle]
unsafe extern "C" fn bigint_lsh(a: Value, b: i32) -> Value {
    let mut i = tmp_bigint();
    check(mp_mul_2d(a.as_bigint().mp_int_ptr(), b, &mut i));
    persist_bigint(i)
}

#[no_mangle]
unsafe extern "C" fn bigint_count_bits(a: Value) -> i32 {
    mp_count_bits(a.as_bigint().mp_int_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn bigint_leb128_size(a: Value) -> u32 {
    if mp_iszero(a.as_bigint().mp_int_ptr()) {
        1
    } else {
        (bigint_count_bits(a) as u32 + 6) / 7 // divide by 7, round up
    }
}

// `add_bit` argument is to make this work for both leb and sleb encoding
unsafe fn bigint_leb128_encode_go(tmp: *mut mp_int, mut buf: *mut u8, add_bit: bool) {
    if mp_isneg(tmp) {
        bigint_trap();
    }

    loop {
        let byte = mp_get_u32(tmp) as u8;
        check(mp_div_2d(tmp, 7, tmp, core::ptr::null_mut()));
        if !mp_iszero(tmp) || (add_bit && byte & (1 << 6) != 0) {
            *buf = byte | (1 << 7);
            buf = buf.add(1);
        } else {
            *buf = byte;
            break;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn bigint_leb128_encode(n: Value, buf: *mut u8) {
    let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
    check(mp_init_copy(&mut tmp, n.as_bigint().mp_int_ptr()));
    bigint_leb128_encode_go(&mut tmp, buf, false)
}

#[no_mangle]
unsafe extern "C" fn bigint_2complement_bits(n: Value) -> u32 {
    let mp_int = n.as_bigint().mp_int_ptr();
    if mp_isneg(mp_int) {
        let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
        check(mp_init_copy(&mut tmp, mp_int));
        check(mp_incr(&mut tmp));
        1 + mp_count_bits(&tmp) as u32
    } else {
        1 + mp_count_bits(mp_int) as u32
    }
}

#[no_mangle]
pub unsafe extern "C" fn bigint_sleb128_size(n: Value) -> u32 {
    (bigint_2complement_bits(n) + 6) / 7 // divide by 7, round up
}

#[no_mangle]
pub unsafe extern "C" fn bigint_sleb128_encode(n: Value, buf: *mut u8) {
    let mut tmp: mp_int = core::mem::zeroed(); // or core::mem::uninitialized?
    check(mp_init_copy(&mut tmp, n.as_bigint().mp_int_ptr()));

    if mp_isneg(&tmp) {
        // Turn negatiave numbers into the two's complement of the right size
        let bytes = bigint_sleb128_size(n);
        let mut big: mp_int = core::mem::zeroed();
        check(mp_init(&mut big));
        check(mp_2expt(&mut big, 7 * bytes as i32));
        check(mp_add(&mut tmp, &big, &mut tmp));
        bigint_leb128_encode_go(&mut tmp, buf, false)
    } else {
        bigint_leb128_encode_go(&mut tmp, buf, true)
    }
}

#[no_mangle]
pub unsafe extern "C" fn bigint_leb128_decode(buf: *mut Buf) -> Value {
    let mut i = tmp_bigint();
    let mut tmp = tmp_bigint();

    let mut shift = 0;
    loop {
        let byte = read_byte(buf);
        mp_set_u32(&mut tmp, (byte & 0b0111_1111) as u32);
        check(mp_mul_2d(&mut tmp, shift, &mut tmp));
        check(mp_add(&mut i, &tmp, &mut i));
        shift += 7;

        if byte & 0b1000_0000 == 0 {
            break;
        }
    }

    persist_bigint(i)
}

#[no_mangle]
pub unsafe extern "C" fn bigint_sleb128_decode(buf: *mut Buf) -> Value {
    let mut i = tmp_bigint();
    let mut tmp = tmp_bigint();

    let mut shift = 0;
    let mut last_sign_bit_set;
    loop {
        let byte = read_byte(buf);
        mp_set_u32(&mut tmp, (byte & 0b0111_1111) as u32);
        check(mp_mul_2d(&mut tmp, shift, &mut tmp));
        check(mp_add(&mut i, &tmp, &mut i));
        last_sign_bit_set = byte & 0b0100_0000 != 0;
        shift += 7;

        if byte & 0b1000_0000 == 0 {
            break;
        }
    }

    if last_sign_bit_set {
        // Negative number, un-2-complement it
        let mut big = tmp_bigint();
        check(mp_2expt(&mut big, shift));
        check(mp_sub(&mut i, &big, &mut i));
    }

    persist_bigint(i)
}
