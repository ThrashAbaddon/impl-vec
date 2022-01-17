use std::alloc::{alloc, dealloc, realloc, Layout};
use std::ptr;
use std::ptr::NonNull;

// `NonNull` is like raw mutable pointer, nonzero and covarant. It can never be null.

pub struct MyVec<T> {
    /// Pinter to the first element in the vector. It will **always** point to that position,
    /// we don't need to offset it during usage.
    pointer: NonNull<T>,
    /// Returns number of elements currently inside the vector.
    length: usize,
    /// Allocated size for the vector without new allocation. After `length` surpasses `capacity`
    /// new allocation is necessary.
    capacity: usize,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        Self {
            // when `length` is zero we shouldn't user `pointer` because it dangling
            pointer: ptr::NonNull::dangling(),
            length: 0,
            capacity: 0, // no allocation for empty vector
        }
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn push(&mut self, element: T) {
        //  mem::size_of::<T> == 0 returns Err from Vec, and also sets `capacity` to maximum
        assert_ne!(std::mem::size_of::<T>(), 0, "No zero sized types");

        // NOTE: After this point we know that type `T` has to have a size in memory.
        if self.capacity == 0 {
            let layout = Layout::array::<T>(4).expect("Couldn't allocate"); // 4 elements

            // layout is 4 * size_of::<T>
            // size_of::<T> > 0
            let pointer = unsafe { alloc(layout) } as *mut T;
            let pointer = NonNull::new(pointer).expect("Couldn't allocate.");
            // NOTE: `pointer` is not null and we have freshly allocated space.
            unsafe { pointer.as_ptr().write(element) };
            self.pointer = pointer;
            self.capacity = 4;
            self.length = 1;
        } else if self.length < self.capacity {
            // NOTE: We have enough space to add new element without new allocation
            let offset = self
                .length
                .checked_mul(std::mem::size_of::<T>())
                .expect("Can't reach memory location");
            assert!(offset < isize::MAX as usize, "Wrapped isize");
            // Offset can't wrap around and `pointer` is pointing to valid memory
            // writing to an offset at `self.length` is valid

            unsafe { self.pointer.as_ptr().add(self.length).write(element) };
            self.length += 1;
        } else {
            debug_assert!(self.length == self.capacity);

            // NOTE: We don't have enough space, we need new allocation
            let align = std::mem::align_of::<T>();

            let size = std::mem::size_of::<T>() * self.capacity;
            let size = size
                .checked_add(size % align) // maybe: align - size % align
                .expect("isize wrapped");
            let new_capacity = self.capacity.checked_mul(2).expect("capacity wrapped");
            let new_size_in_bytes = std::mem::size_of::<T>() * new_capacity;
            let pointer = unsafe {
                let layout = Layout::from_size_align_unchecked(size, align);
                realloc(self.pointer.as_ptr() as *mut u8, layout, new_size_in_bytes)
            };
            // NOTE: We can panic here because old `length`, `capacity` and `pointer` are still valid.
            let pointer = NonNull::new(pointer as *mut T).expect("Couldn't reallocate.");
            unsafe {
                pointer.as_ptr().add(self.length).write(element);
            }
            self.pointer = pointer;
            self.length += 1;
            self.capacity = new_capacity;
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.length {
            return None;
        }

        Some(unsafe { self.pointer.as_ptr().add(index).as_ref().unwrap() })
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        unsafe {
            // NOTE: We deallocate elements inside the vector.
            let to_drop = std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.length);
            std::ptr::drop_in_place(to_drop);
            // we could have also iterated over the elements and dropped each one one-by-one.

            // NOTE: We deallocate part of memory for the vector where the elements were held.
            let size = std::mem::size_of::<T>() * self.capacity;
            let align = std::mem::align_of::<T>();
            let layout = Layout::from_size_align_unchecked(size, align);
            dealloc(self.pointer.as_ptr() as *mut u8, layout);
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::MyVec;

    #[test]
    fn push_to_vec() {
        let mut vec: MyVec<usize> = MyVec::new();
        vec.push(1_usize);
        vec.push(2);
        vec.push(3);
        vec.push(4);
        vec.push(5);
        assert_eq!(vec.capacity(), 8);
        assert_eq!(vec.len(), 5);

        assert_eq!(vec.get(3), Some(&4));
    }

    #[derive(Debug, PartialEq)]
    struct A(usize);

    impl Drop for A {
        fn drop(&mut self) {
            println!("Dropped");
        }
    }

    #[test]
    fn heap_dealloc() {
        let mut vec = MyVec::new();
        vec.push(A(1));
        vec.push(A(2));
        vec.push(A(3));

        assert_eq!(vec.get(0), Some(&A(1)));
        assert_eq!(vec.get(1), Some(&A(2)));
        assert_eq!(vec.get(2), Some(&A(3)));
        assert_eq!(vec.get(3), None);
    }
}
