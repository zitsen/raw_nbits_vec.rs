#![feature(alloc)]

extern crate alloc;
extern crate num;

use alloc::raw_vec::RawVec;
use num::PrimInt;
use std::cmp;
use std::fmt::{self, Debug};
use std::mem;
use std::ptr;
use std::marker::PhantomData;

pub trait Nbits {
    fn bits() -> usize;

    #[inline]
    fn mask() -> usize {
        (0..).take(Self::bits()).fold(0, |mask, _x| mask << 1 | 1)
    }
}

macro_rules! nbits_set {
    ($(($t: ident, $size: expr)),*) => (
        $(
            /// Struct for each NBits
            pub struct $t;
            impl Nbits for $t {
                #[inline]
                fn bits() -> usize {
                    $size
                }
            }
        )*
    )
}

nbits_set! {
    (As1bits, 1),
    (As2bits, 2),
    (As3bits, 3),
    (As4bits, 4)
}

pub struct NbitsVec<T: Nbits, B: PrimInt = usize> {
    buf: RawVec<B>,
    len: usize,
    _marker: PhantomData<T>,
}

impl<
T:  Nbits,
B:  PrimInt,
> Default for NbitsVec<T, B> {
    fn default() -> Self {
        NbitsVec {
            buf: RawVec::new(),
            len: 0,
            _marker: PhantomData,
        }
    }
}

impl<T: Nbits, B: PrimInt + fmt::LowerHex> Debug for NbitsVec<T, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f,
                    "NbitsVec<{}> {{ len: {}, buf: RawVec {{ cap: {}, [",
                    T::bits(),
                    self.len,
                    self.buf.cap()));
        let ptr = self.buf.ptr();
        for i in 0..self.buf.cap() {
            unsafe {
                try!(write!(f, "{:#x}, ", ptr::read(ptr.offset(i as isize))));
            }
        }
        write!(f, "] }}")
    }
}

impl<
T:  Nbits,
B:  PrimInt
> NbitsVec<T, B> {
    /// Constructs a new, empty NbitsVec<T>
    ///
    /// The vector will not allocate until elements are pushed onto it.
    ///
    /// # Examples
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::new();
    /// # }
    /// ```
    #[inline]
    pub fn new() -> Self {
        NbitsVec {
            buf: RawVec::new(),
            len: 0,
            _marker: PhantomData,
        }
    }
    /// Constructs a new, empty Vec<T> with the specified capacity.
    ///
    /// The vector will be able to hold exactly capacity elements without reallocating. If capacity
    /// is 0, the vector will not allocate.
    ///
    /// It is important to note that this function does not specify the length of the returned
    /// vector, but only the capacity.
    ///
    /// # Examples
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// assert!(vec.capacity() >= 10);
    /// # }
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        NbitsVec {
            buf: RawVec::with_capacity(Self::capacity_to_buf(capacity)),
            len: 0,
            _marker: PhantomData,
        }
    }

    pub unsafe fn from_raw_parts(ptr: *mut B, length: usize, capacity: usize) -> Self {
        NbitsVec {
            buf: RawVec::from_raw_parts(ptr, Self::capacity_to_buf(capacity)),
            len: length,
            _marker: PhantomData,
        }
    }

    /// Returns the number of elements the vector can hold without reallocating.
    ///
    /// # Examples
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::{NbitsVec, As1bits};
    /// # fn main() {
    /// let v: NbitsVec<As1bits> = NbitsVec::with_capacity(10);
    /// assert!(v.capacity() >= 10);
    /// assert_eq!(v.capacity(), std::mem::size_of::<usize>() * 8);
    /// # }
    /// ```
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        Self::capacity_from_buf(self.buf.cap())
    }

    /// Reserves capacity for at least additional more elements to be inserted in the given
    /// NbitsVec<T>.
    /// The collection may reserve more space to avoid frequent reallocations.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows usize.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut v: NbitsVec<As2bits> = NbitsVec::new();
    /// assert!(v.capacity() == 0);
    /// v.reserve(100);
    /// assert!(v.capacity() >= 100);
    /// # }
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        let required_cap = self.len().checked_add(additional).expect("capacity overflow");
        let used_cap = Self::capacity_to_buf(self.len());
        let need_extra_cap = Self::capacity_to_buf(required_cap);
        self.buf.reserve(used_cap, need_extra_cap);
    }

    /// Reserves the minimum capacity for exactly additional more elements to be inserted in the
    /// given `NbitsVec<T>`. Does nothing if the capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows usize.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut v: NbitsVec<As2bits> = NbitsVec::new();
    /// assert!(v.capacity() == 0);
    /// v.reserve_exact(64);
    /// assert_eq!(v.capacity(), 64);
    /// v.reserve_exact(127);
    /// assert!(v.capacity() >= 127);
    /// v.reserve_exact(128);
    /// assert_eq!(v.capacity(), 128);
    /// # }
    /// ```
    pub fn reserve_exact(&mut self, additional: usize) {
        let required_cap = self.len().checked_add(additional).expect("capacity overflow");
        let used_cap = Self::capacity_to_buf(self.len());
        let need_extra_cap = Self::capacity_to_buf(required_cap);
        self.buf.reserve_exact(used_cap, need_extra_cap);
    }
    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator may still inform the
    /// vector that there is space for a few more elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// vec.shrink_to_fit();
    /// assert_eq!(vec.capacity(), 0);
    /// # }
    /// ```
    ///
    pub fn shrink_to_fit(&mut self) {
        let fit_len = Self::capacity_to_buf(self.len());
        self.buf.shrink_to_fit(fit_len);
    }

    /// Shorten a vector to be `len` elements long, dropping excess elements.
    ///
    /// If `len` is greater than the vector's current length, this has no effect.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(2);
    /// unsafe { vec.set_len(2) }
    /// vec.truncate(3);
    /// assert_eq!(vec.len(), 2);
    /// vec.truncate(1);
    /// assert_eq!(vec.len(), 1);
    /// # }
    /// ```
    pub fn truncate(&mut self, len: usize) {
        if self.len() > len {
            self.len = len;
            self.shrink_to_fit();
        }
    }
    pub fn as_raw_slice(&self) -> &[B] {
        unimplemented!();
    }
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unimplemented!();
    }
    pub fn into_boxed_slice(self) -> Box<[T]> {
        unimplemented!();
    }

    /// Sets the length of a vector.
    ///
    /// This will explicitly set the size of the vector, without actually modifying its buffers or
    /// reserving additional capacity as needed, so it is up to the caller to ensure that the vector
    /// is actually the specified size.
    ///
    /// Recommend to use [resize()](#method.resize) when you actually want to `resize` the vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut v: NbitsVec<As2bits> = NbitsVec::new();
    /// unsafe {
    ///     v.set_len(3);
    /// }
    /// assert_eq!(v.len(), 3);
    /// assert_eq!(v.capacity(), 0); // as documented, the capacity will not change
    /// unsafe {
    ///     v.set_len(1)
    /// }
    /// assert_eq!(v.len(), 1);
    /// # }
    /// ```
    #[inline]
    pub unsafe fn set_len(&mut self, len: usize) {
        self.len = len;
    }

    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut v: NbitsVec<As2bits> = NbitsVec::new();
    /// v.push(0b01);
    /// v.push(0b10);
    /// assert_eq!(v.len(), 2);
    /// v.insert(1, 0b11);
    /// assert_eq!(v.get(1), 0b11);
    /// assert_eq!(v.get(2), 0b10);
    /// # }
    pub fn insert(&mut self, index: usize, element: B) {
        self.align(index, index + 1);
        self.set(index, element);
    }

    /// Removes and returns the element at position `index` within the vector, shifting all elements
    /// after position `index` one position to the left.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut v: NbitsVec<As2bits> = NbitsVec::new();
    /// v.push(0b01);
    /// v.push(0b10);
    /// assert_eq!(v.remove(0), 0b01);
    /// # }
    /// ```
    pub fn remove(&mut self, index: usize) -> B {
        if index >= self.len {
            panic!("index is out of bounds");
        }
        if self.is_empty() {
            panic!("vector is empty");
        }
        if self.len() == 1 {
            return self.pop().expect("swap removed with one element");
        }
        let removed = self.get(index);
        self.align(index + 1, index);
        removed
    }

    /// Removes an element from anywhere in the vector and return it, replacing it with the last
    /// element.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    /// Panics if vector is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut v: NbitsVec<As2bits> = NbitsVec::new();
    /// v.push(0b01);
    /// v.push(0b10);
    /// v.push(0b11);
    /// assert_eq!(v.len(), 3);
    /// println!("{:?}", v);
    /// assert_eq!(v.swap_remove(0), 0b01);
    /// println!("{:?}", v);
    /// assert_eq!(v.len(), 2);
    /// assert_eq!(v.get(0), 0b11);
    /// assert_eq!(v.get(1), 0b10);
    /// println!("{:?}", v);
    /// assert_eq!(v.swap_remove(0), 0b11);
    /// # }
    /// ```
    pub fn swap_remove(&mut self, index: usize) -> B {
        if index >= self.len {
            panic!("index is out of bounds");
        }
        if self.is_empty() {
            panic!("vector is empty");
        }
        if self.len() == 1 {
            return self.pop().expect("swap removed with one element");
        }
        let value = self.get(index);
        let last = self.pop().expect("swap removed with last element");
        self.set(index, last);
        value
    }

    /// Moves all the elements of `other` into `Self`, leaving `other` empty.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the vector overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::new();
    /// let mut other: NbitsVec<As2bits> = NbitsVec::new();
    /// other.resize(2, 0b10);
    /// vec.append(&mut other);
    /// assert_eq!(vec.len(), 2);
    /// assert_eq!(other.len(), 0);
    /// # assert_eq!(vec.get(0), 0b10);
    /// # assert_eq!(vec.get(1), 0b10);
    /// # }
    /// ```
    pub fn append(&mut self, other: &mut Self) {
        let other_len = other.len();
        self.reserve_exact(other_len);
        for i in 0..other_len {
            let v = other.get(i);
            self.push(v);
        }
        unsafe { other.set_len(0) }
    }

    /// Unimplements
    pub fn retain<F>(&mut self, f: F)
        where F: FnMut(&T) -> bool
    {
        unimplemented!();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the number of bits in current length.
    ///
    /// It is related to the element numbers - not the capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// assert_eq!(vec.bits(), 0);
    /// # }
    /// ```
    #[inline]
    pub fn bits(&self) -> usize {
        self.len() * Self::unit_bits()
    }

    /// Total bits in buf.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// assert_eq!(vec.buf_bits(), std::mem::size_of::<usize>() * 8);
    /// # }
    /// ```
    pub fn buf_bits(&self) -> usize {
        self.buf.cap() * Self::buf_unit_bits()
    }

    /// Returns whether or not the vector is empty.
    ///
    /// Alias to `len() == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// assert!(vec.is_empty());
    /// # }
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn split_off(&mut self, at: usize) -> Self {
        unimplemented!();
    }

    pub fn push_all(&mut self, other: &[T]) {
        unimplemented!();
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the vector overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::new();
    /// vec.push(0b10);
    /// vec.push(0b01);
    /// assert_eq!(vec.len(), 2);
    /// # }
    /// ```
    pub fn push(&mut self, value: B) {
        let len = self.len();
        let new_len = len.checked_add(1).expect("usize added overflows");
        self.reserve(1);
        self.len = new_len;
        self.set(len, value);
    }

    /// Removes the last element from a vector and returns it, or `None` if it is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::new();
    /// vec.push(0b10);
    /// vec.push(0b11);
    /// assert_eq!(vec.pop(), Some(0b11));
    /// assert_eq!(vec.pop(), Some(0b10));
    /// assert_eq!(vec.len(), 0);
    /// # }
    /// ```
    pub fn pop(&mut self) -> Option<B> {
        let len = self.len();
        if self.is_empty() {
            return None;
        }
        let new_len = len - 1;
        let last = self.get(new_len);
        self.len = new_len;
        Some(last)
    }

    /// Resizes the Vec in-place so that len() is equal to new_len.
    ///
    /// If new_len is greater than len(), the Vec is extended by the difference, with each
    /// additional slot filled with value. If new_len is less than len(), the Vec is simply
    /// truncated. Note that `resize` expand memeory will use `reserve_exact` method to
    /// fit size.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::new();
    /// vec.resize(10, 0);
    /// assert_eq!(vec.capacity(), std::mem::size_of::<usize>() * 8 / 2);
    /// # }
    /// ```
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: B) {
        let len = self.len();
        if len < new_len {
            let n = new_len - len;
            self.reserve_exact(n);
            unsafe {
                self.fill_buf(len, n, value);
                self.len = new_len;
            }
        } else {
            self.truncate(new_len);
        }
    }

    /// ## Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits, u8> = NbitsVec::new();
    /// vec.resize(24, 0);
    /// unsafe {
    ///     vec.fill_buf(0, 12, 1);
    ///     vec.fill_buf(12, 12, 2);
    /// }
    /// println!("{:?}", vec);
    /// // Left align will reduce the length.
    /// vec.align(1, 0);
    /// assert_eq!(vec.len(), 23);
    /// assert!((0..).take(11).all(|x| vec.get(x) == 1));
    /// assert!((11..).take(12).all(|x| vec.get(x) == 2));
    ///
    /// vec.align(11, 3);
    /// assert_eq!(vec.len(), 23 - 8);
    /// assert!((0..).take(3).all(|x| vec.get(x) == 1));
    /// assert!((3..vec.len()).all(|x| vec.get(x) == 2));
    /// // Right align will expand the length.
    /// vec.align(6, 7);
    /// assert_eq!(vec.len(), 23 - 8 + 1);
    /// assert!((6..7).all(|x| vec.get(x) == 0));
    /// assert!((7..vec.len()).all(|x| vec.get(x) == 2));
    ///
    /// vec.align(13, 33);
    /// assert_eq!(vec.len(), 23 - 8 + 1 + 33 - 13);
    /// assert!((13..33).all(|x| vec.get(x) == 0));
    /// assert!((33..vec.len()).all(|x| vec.get(x) == 2));
    /// println!("{:?}", vec);
    /// # }
    /// ```
    pub fn align(&mut self, offset: usize, to: usize) {
        let unit = Self::unit_bits();
        let buf_unit = Self::buf_unit_bits();
        let unit_cap = buf_unit / unit;
        if offset > to {
            // Reduce `interval` length.
            let interval = offset - to;
            // e.g. N = 2, B = u8, interval = 4
            if buf_unit % unit == 0 && interval % unit_cap == 0 {
                // Copy previous offset * unit % buf_unit values.
                let extra = offset % unit_cap;
                let (offset, to) = (0..extra).fold((offset, to), |(offset, to), _i| {
                    let value = self.get(offset);
                    self.set(to, value);
                    (offset + 1, to + 1)
                });
                unsafe {
                    let ptr = self.buf.ptr();
                    let src = offset / unit_cap;
                    let dst = to / unit_cap;
                    let count = self.len() / unit_cap - src + 1;
                    ptr::copy(ptr.offset(src as isize), ptr.offset(dst as isize), count);
                }
            } else {
                for offset in offset..self.len() {
                    let value = self.get(offset);
                    self.set(offset - interval, value);
                }
            }
            self.len = self.len - interval;
        } else {
            // Expand with `interval` length values.
            let interval = to - offset;
            let len = self.len();
            self.reserve_exact(interval);
            if buf_unit % unit == 0 && interval % unit_cap == 0 {
                unsafe {
                    let ptr = self.buf.ptr();
                    let src = offset / unit_cap;
                    let dst = to / unit_cap;
                    let count = len / unit_cap - src + 1;
                    ptr::copy(ptr.offset(src as isize), ptr.offset(dst as isize), count);
                    self.fill_buf(offset, interval, B::zero());
                    self.len = self.len() + interval;
                }
            } else {
                self.len = len + interval;
                for offset in (offset..len).rev() {
                    let value = self.get(offset);
                    self.set(offset + interval, value);
                }
                unsafe {
                    self.fill_buf(offset, interval, B::zero());
                }
            }
        }
    }

    /// Fill vector buf as `value` from `index` with size `length`.
    ///
    /// ## Unsafety
    ///
    /// The method doesnot check the index validation of the vector.
    ///
    /// ## Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits, u8> = NbitsVec::new();
    /// vec.resize(24, 0);
    /// println!("{:?}", vec);
    /// unsafe {
    ///     vec.fill_buf(1, 2, 2); // length < buf_unit
    ///     assert!((1..).take(2).all(|x| vec.get(x) == 2));
    ///     vec.fill_buf(0, 8, 1); // offset: 0, 0
    ///     assert!((0..).take(8).all(|x| vec.get(x) == 1));
    ///     vec.fill_buf(7, 10, 2); // offset: n, n
    ///     assert!((7..).take(10).all(|x| vec.get(x) == 2));
    ///     vec.fill_buf(8, 11, 1); // offset: 0,n
    ///     assert!((8..).take(11).all(|x| vec.get(x) == 1));
    /// }
    /// # }
    /// ```
    #[inline]
    pub unsafe fn fill_buf(&mut self, index: usize, length: usize, value: B) {
        let unit = Self::unit_bits();
        if length == 1 {
            return self.set_buf_bits(index * unit, unit, value);
        }
        let buf_unit = Self::buf_unit_bits();
        if (length <= buf_unit / unit) || buf_unit % unit != 0 {
            for i in (index..).take(length) {
                self.set_buf_bits(i * unit, unit, value);
            }
        }

        let mul = buf_unit / unit;
        let item = (0..mul).fold(B::zero(), |v, _x| v << unit | value);
        let ptr = self.buf.ptr();
        let write_buf = |start: usize, end: usize| {
            (start..end).fold(ptr.offset(start as isize), |ptr, _x| {
                ptr::write(ptr, item);
                ptr.offset(1)
            });
        };
        match Self::index_range_to_buf(index, length) {
            ((start_idx, start_offset), (end_idx, end_offset)) if start_offset == 0 &&
                                                                  end_offset == 0 => {
                write_buf(start_idx, end_idx)
            }
            ((start_idx, start_offset), (end_idx, end_offset)) if start_offset == 0 => {
                write_buf(start_idx, end_idx);
                self.set_buf_unit_bits(end_idx * buf_unit, end_offset, item);
            }
            ((start_idx, start_offset), (end_idx, end_offset)) if end_offset == 0 => {
                self.set_buf_unit_bits(index * unit, buf_unit - start_offset, item);
                write_buf(start_idx + 1, end_idx);
            }
            ((start_idx, start_offset), (end_idx, end_offset)) => {
                self.set_buf_unit_bits(index * unit, buf_unit - start_offset, item);
                self.set_buf_unit_bits(end_idx * buf_unit, end_offset, item);
                write_buf(start_idx + 1, end_idx);
            }
        }
    }

    /// ## Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// unsafe { vec.set_len(2) }
    /// vec.set(0, 0b11);
    /// assert_eq!(vec.get(0), 0b11);
    /// # }
    /// ```
    #[inline]
    pub fn set(&mut self, index: usize, value: B) {
        if index >= self.len {
            panic!("attempt to set at {} but only {}", index, self.len);
        }
        unsafe {
            let unit = Self::unit_bits();
            self.set_buf_bits(index * unit, unit, value);
        }
    }

    /// Set `bit` at `index`.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// vec.reserve(10);
    /// unsafe { vec.set_len(7) };
    /// vec.set_bit(0, true);
    /// # }
    /// ```
    ///
    #[inline]
    pub fn set_bit(&mut self, index: usize, bit: bool) {
        let bits = self.bits();
        if index >= bits {
            panic!("attempt to set bit out of bounds");
        }
        unsafe {
            self.set_buf_unit_bit(index, bit);
        }
    }

    /// Get `bit` at some bit index.
    ///
    /// Returns `None` if required index is out of bounds, else return `bool` for bit value.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// vec.reserve(10);
    /// assert!(vec.get_bit(0).is_none());
    /// vec.resize(10, 0);
    /// println!("{:?}", vec);
    /// for i in 0..8 {
    ///     vec.set_bit(i, true);
    ///     println!("Set at {} as true", i);
    ///     println!("{:?}", vec);
    ///     assert_eq!(vec.get_bit(i), Some(true));
    /// }
    /// for i in 0..8 {
    ///     vec.set_bit(i, false);
    ///     assert_eq!(vec.get_bit(i), Some(false));
    /// }
    /// # }
    /// ```
    #[inline]
    pub fn get_bit(&self, at: usize) -> Option<bool> {
        let bits = self.bits();
        if at >= bits {
            return None;
        } else {
            unsafe { Some(self.get_buf_unit_bit(at) == B::one()) }
        }
    }

    /// Set `length` bits of buf at `offset`th bit as `value`.
    ///
    /// ## Unsafety
    ///
    /// `set_buf_bits` will not check the `offset`. Users should ensure to do this manually.
    ///
    /// ## Panics
    ///
    /// This method should panic while required `length` is longer than the buf unit bits size.
    ///
    /// ## Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    ///
    /// unsafe {
    ///     println!("Set buf 0 as 1");
    ///     vec.set_buf_bits(0, 1, 1);
    ///     println!("Set buf bits [1, 2] as `10`");
    ///     vec.set_buf_bits(1, 2, 2);
    ///     println!("Set buf bits [3, 6] as `1010`");
    ///     vec.set_buf_bits(3, 4, 0b1010);
    /// }
    /// println!("{:?}", vec);
    /// unsafe {
    ///     assert_eq!(vec.get_buf_bits(0, 1), 1);
    ///     assert_eq!(vec.get_buf_bits(1, 2), 2);
    ///     assert_eq!(vec.get_buf_bits(3, 4), 0b1010);
    /// }
    /// # }
    /// ```
    #[inline]
    pub unsafe fn set_buf_bits(&mut self, offset: usize, length: usize, value: B) {
        let buf_unit = Self::buf_unit_bits();
        if length > buf_unit {
            panic!("set {} buf bits longer than buf unit bits {}",
                   length,
                   buf_unit);
        }
        if length == 1 {
            return self.set_buf_unit_bit(offset, value & B::one() == B::one());
        }
        match Self::unit_bits() {
            unit if unit == buf_unit => {
                // NOTE: maybe unreachable!() is better.
                self.set_buf_unit_bits(offset, length, value);
            }
            unit if unit < buf_unit && buf_unit % unit == 0 => {
                self.set_buf_unit_bits(offset, length, value);
            }
            _ => {
                let mut v = value;
                for x in offset..cmp::min(offset + length, self.buf_bits()) {
                    self.set_buf_unit_bit(x, v & B::one() == B::one());
                    v = v >> 1;
                }
            }
        }
    }

    /// Mask buf element of `index` at offset `(from, to)` as zero.
    #[inline]
    unsafe fn zero_buf_unit_bits(&mut self, offset: usize, length: usize) {
        self.set_buf_unit_bits(offset, length, B::zero());
    }

    /// Set buf element of `index` at offset `from` to `to` as `value`.
    #[inline]
    unsafe fn set_buf_unit_bits(&mut self, offset: usize, length: usize, value: B) {
        let (index, offset) = Self::bit_index_to_buf(offset);
        let mask = (offset..)
                       .take(length)
                       .fold(B::zero(), |mask, _x| mask << 1 | B::one()) <<
                   offset;
        let ptr = self.buf.ptr().offset(index as isize);
        let cur = ptr::read(ptr);
        let new = mask & (value << offset);
        let old = mask & cur;
        if old != new {
            ptr::write(ptr, cur & !mask | new);
        }
    }

    /// Set buf unit bit at `index`th unit of `offset`bit.
    #[inline]
    unsafe fn set_buf_unit_bit(&mut self, offset: usize, bit: bool) {
        let (index, offset) = Self::bit_index_to_buf(offset);
        let mask = B::one() << offset;
        let ptr = self.buf.ptr().offset(index as isize);
        let cur = ptr::read(ptr);
        let old = cur >> offset & B::one();
        match (old == B::one(), bit) {
            (lhs, rhs) if lhs == rhs => (),
            (_, true) => ptr::write(ptr, cur | mask),
            (_, false) => ptr::write(ptr, cur & !mask),
        }
    }

    /// Get `N` bits value as `B`.
    ///
    /// ## TODO
    ///
    /// ?? Is a `Nbits` object is better than `B` ??
    ///
    /// ## Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::with_capacity(10);
    /// unsafe { vec.set_len(2) }
    /// vec.set(0, 0b11);
    /// assert_eq!(vec.get(0), 0b11);
    /// # }
    /// ```
    pub fn get(&self, index: usize) -> B {
        if index >= self.len {
            panic!("attempt to get at {} but only {}", index, self.len);
        }
        let unit = Self::unit_bits();
        unsafe { self.get_buf_bits(index * unit, unit) }
    }

    /// Get `length` bits of buf at `offset`th bit.
    ///
    /// # Unsafety
    ///
    /// `get_buf_bits` will not check the `offset`. Users should ensure to do this manually.
    ///
    /// # Panics
    ///
    /// This method should panic while required `length` is longer than the buf unit bits size.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate raw_nbits_vec;
    /// # use raw_nbits_vec::*;
    /// # fn main() {
    /// let mut vec: NbitsVec<As2bits> = NbitsVec::new();
    /// vec.resize(10, 0);
    /// println!("{:?}", vec);
    /// for i in 0..8 {
    ///     vec.set_bit(i, if i % 2 == 0 { true } else { false });
    /// }
    /// println!("{:?}", vec);
    /// unsafe {
    ///     println!("Get buf bits at 0 with length 1");
    ///     assert_eq!(vec.get_buf_bits(0, 1), 1);
    ///     println!("Get buf bits at 1 with length 2");
    ///     assert_eq!(vec.get_buf_bits(1, 2), 2);
    ///     println!("Get buf bits at 3 with length 4");
    ///     assert_eq!(vec.get_buf_bits(3, 4), 0b1010);
    /// }
    /// # }
    /// ```
    #[inline]
    pub unsafe fn get_buf_bits(&self, offset: usize, length: usize) -> B {
        let buf_unit = Self::buf_unit_bits();
        if length > buf_unit {
            panic!("get {} buf bits longer than buf unit bits {}",
                   length,
                   buf_unit);
        }
        if length == 1 {
            return self.get_buf_unit_bit(offset);
        }
        match (Self::unit_bits(), Self::buf_unit_bits()) {
            (unit, buf_unit) if unit == buf_unit => {
                // NOTE: maybe unreachable!() is better
                self.get_buf_unit_bits(offset, length)
            }
            (unit, buf_unit) if unit < buf_unit && buf_unit % unit == 0 => {
                self.get_buf_unit_bits(offset, length)
            }
            (_, _) => {
                (offset..cmp::min(offset + length, self.buf_bits()))
                    .map(|x| self.get_buf_unit_bit(x))
                    .fold(B::zero(), |v, x| v << 1 | x)
            }
        }
    }

    /// Get buf unit bit at `index`th unit of `offset`bit.
    #[inline]
    unsafe fn get_buf_unit_bit(&self, offset: usize) -> B {
        let (index, offset) = Self::bit_index_to_buf(offset);
        let ptr = self.buf.ptr().offset(index as isize);
        ptr::read(ptr) >> offset & B::one()
    }

    /// Get buf `length` bits of unit at `index`th unit's `offset`th bit
    #[inline]
    unsafe fn get_buf_unit_bits(&self, offset: usize, length: usize) -> B {
        let offset = Self::bit_index_to_buf(offset);
        let ptr = self.buf.ptr().offset(offset.0 as isize);
        let unit = Self::buf_unit_bits();
        (ptr::read(ptr) << (unit - offset.1 - length)) >> (unit - length)
    }
    /// Converts capacity to storage size
    #[inline]
    fn capacity_to_buf(capacity: usize) -> usize {
        if capacity == 0 {
            0
        } else {
            (capacity * Self::unit_bits() - 1) / (Self::buf_unit_bits()) + 1
        }
    }

    /// Converts the storage size to capacity.
    #[inline]
    fn capacity_from_buf(buf_cap: usize) -> usize {
        buf_cap * Self::buf_unit_bits() / Self::unit_bits()
    }

    /// Converts the vector index to buf `(index, offset)` tuple.
    #[inline]
    fn index_to_buf(index: usize) -> (usize, usize) {
        let elem_bits = Self::buf_unit_bits();
        let bits_index = index * Self::unit_bits();
        (bits_index / elem_bits, bits_index % elem_bits)
    }

    /// Converts the vector index range to buf `(index, offset)` range tuple.
    #[inline]
    fn index_range_to_buf(index: usize, length: usize) -> ((usize, usize), (usize, usize)) {
        (Self::index_to_buf(index),
         Self::index_to_buf(index + length))
    }

    /// Converts bit index to buf `(index, offset)` tuple.
    #[inline]
    fn bit_index_to_buf(index: usize) -> (usize, usize) {
        let unit = Self::buf_unit_bits();
        (index / unit, index % unit)
    }

    /// Returns size of `B`.
    #[inline]
    fn buf_unit_bits() -> usize {
        mem::size_of::<B>() * 8
    }

    /// Returns unit of bits - that is `NbitsVec`'s `N`.
    #[inline]
    fn unit_bits() -> usize {
        T::bits()
    }
}
