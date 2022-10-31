#![feature(allocator_api)]
#![feature(slice_ptr_get)]

use std::{alloc::{Allocator, Global}, marker::PhantomData};

// TODO: Store a std::mem::Layout instead

pub struct ErasedVec<A: Allocator> {
	element_type: std::any::TypeId,
	element_size: usize,
	element_align: usize,
	ptr: *mut u8,
	len: usize,
	cap: usize,
	allocator: A
}

impl ErasedVec<Global> {
	pub fn new<T: 'static>() -> ErasedVec<Global> {
		ErasedVec::new_in::<T>(Global)
	}

	pub fn with_capacity<T: 'static>(capacity: usize) -> ErasedVec<Global> {
		ErasedVec::with_capacity_in::<T>(capacity, Global)
	}
}

impl<A: Allocator> ErasedVec<A> {
	pub fn new_in<T: 'static>(allocator: A) -> ErasedVec<A> {
		ErasedVec {
			element_type: std::any::TypeId::of::<T>(),
			element_size: std::mem::size_of::<T>(),
			element_align: std::mem::align_of::<T>(),
			ptr: std::ptr::null_mut(),
			len: 0,
			cap: 0,
			allocator
		}
	}

	pub fn with_capacity_in<T: 'static>(capacity: usize, allocator: A) -> ErasedVec<A> {
		let layout = std::alloc::Layout::array::<T>(capacity).unwrap();
		let mem = allocator.allocate(layout).unwrap();

		ErasedVec {
			element_type: std::any::TypeId::of::<T>(),
			element_size: std::mem::size_of::<T>(),
			element_align: std::mem::align_of::<T>(),
			ptr: mem.as_mut_ptr(),
			len: 0,
			cap: capacity,
			allocator
		}
	}

	/// Grow the vec into memory double the size
	pub fn grow(&mut self) {
		if self.cap == 0 {
			// If the ErasedVec is empty we allocate space for one element and return
			let layout = std::alloc::Layout::from_size_align(self.element_size, self.element_align).unwrap();
			let mem = self.allocator.allocate(layout).unwrap();
			self.ptr = mem.as_mut_ptr();
			self.cap = 1;
		}

		let old_layout = std::alloc::Layout::from_size_align(self.cap * self.element_size, self.element_align).unwrap();
		let new_layout = std::alloc::Layout::from_size_align(2 * self.cap * self.element_size, self.element_align).unwrap();

		// 1. Allocate new memory
		let mem = self.allocator.allocate(new_layout).unwrap();

		// 2. Copy the elements into the new array
		unsafe {
			std::ptr::copy_nonoverlapping(self.ptr, mem.as_mut_ptr(), old_layout.size());
		}

		// 3. Deallocate the old array
		unsafe {
			self.allocator.deallocate(std::ptr::NonNull::new(self.ptr).unwrap(), old_layout);
		}

		// 4. Update the struct
		self.ptr = mem.as_mut_ptr();
		self.cap = 2 * self.cap;
	}

	pub fn push<T: 'static>(&mut self, value: T) {
		assert_eq!(std::any::TypeId::of::<T>(), self.element_type);

		// 1. Grow if the array is too small
		if self.len == self.cap {
			self.grow();
		}

		// 2. Copy the value to the front of the array
		unsafe {
			std::ptr::copy_nonoverlapping(
				&value as *const T as *const u8,
				self.ptr.offset(self.len as isize * self.element_size as isize),
				self.element_size
			);
		}

		// 3. Increment the length
		self.len += 1;
	}

	pub fn pop(&mut self) {
		assert!(self.len > 0, "ErasedVec#pop() must not be called on an empty vector");

		self.len -= 1;
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn get<T: 'static>(&self, index: usize) -> Option<&T> {
		assert_eq!(std::any::TypeId::of::<T>(), self.element_type);

		if index >= self.len {
			None
		} else {
			unsafe {
				Some(&*(self.ptr.offset(index as isize * self.element_size as isize) as *const u8 as *const T))
			}
		}
	}

	/// Erases an element from the ErasedVec
	/// 
	/// If you want the removed element, use [remove] instead
	pub fn erase(&mut self, index: usize) {
		if index < self.len - 1 {
		    unsafe {
		    	std::ptr::copy(self.ptr.offset((index as isize + 1) * self.element_size as isize), self.ptr.offset(index as isize * self.element_size as isize), self.len - 1 - index)
		    }
		}

		self.len -= 1;
	}

	/// Removes an element from the ErasedVec
	/// 
	/// If you don't need the removed element, use [erase] instead
	pub fn remove<T: 'static>(&mut self, index: usize) -> T {
		assert!(index < self.len);

		let val = unsafe {
			//std::mem::transmute_copy(&(self.ptr.offset(index as isize * self.element_size as isize) as *const T))
			std::ptr::read(self.ptr.offset(index as isize * self.element_size as isize) as *const T)
		};

		self.erase(index);

		val
	}

	pub fn into_vec<T: 'static>(&self) -> Vec<T> {
		assert_eq!(std::any::TypeId::of::<T>(), self.element_type);

		let mut vec = Vec::<T>::with_capacity(self.len);

		if self.len > 0 {
		    // Copy the data into the vec
		    unsafe {
		    	std::ptr::copy_nonoverlapping(self.ptr, vec.as_mut_ptr() as *mut u8, self.len * self.element_size);
		    }
		}	

		vec
	}
}

impl<'a, A: Allocator> ErasedVec<A> {
	pub fn iter<T: 'static>(&'a self) -> IntoIter<'a, A, T> {
		IntoIter {
			i: 0,
			vec: self,
			_marker: PhantomData
		}
	}
}

impl<A: Allocator> Drop for ErasedVec<A> {
	fn drop(&mut self) {
		if self.cap > 0 {
			unsafe {
		    	self.allocator.deallocate(
		    		std::ptr::NonNull::new(self.ptr).unwrap(),
		    		std::alloc::Layout::from_size_align(self.cap * self.element_size, self.element_align).unwrap()
		    	)
		    }
		}
	}
}

impl<A: Allocator + Clone> Clone for ErasedVec<A> {
	fn clone(&self) -> Self {
		let mem = self.allocator.allocate(std::alloc::Layout::from_size_align(self.cap * self.element_size, self.element_align).unwrap()).unwrap();

		unsafe {
			std::ptr::copy_nonoverlapping(self.ptr, mem.as_mut_ptr(), self.len * self.element_size);
		}

		ErasedVec {
			element_type: self.element_type,
			element_size: self.element_size,
			element_align: self.element_align,
			ptr: mem.as_mut_ptr(),
			len: self.len,
			cap: self.cap,
			allocator: self.allocator.clone()
		}
	}
}

pub struct IntoIter<'a, A: Allocator + 'static, T: 'a> {
	i: usize,
	vec: &'a ErasedVec<A>,
	_marker: PhantomData<T>
}

impl<'a, A: Allocator + 'static, T: 'static> Iterator for IntoIter<'a, A, T> {
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item> {
		let val = self.vec.get::<T>(self.i);
		self.i += 1;
		val
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone, Debug, PartialEq, Eq)]
	struct Foo {
		u8: u8,
		u16: u16,
		u32: u32,
		u64: u64,
		usize: usize
	}

	#[test]
	fn test_i32() {
		println!("std::any::TypeId::of::<i32>() == {:?}", std::any::TypeId::of::<i32>());
		
		let mut vec = ErasedVec::with_capacity::<i32>(4);

		vec.push(4i32);
		vec.push(12i32);
		vec.push(36i32);

		assert_eq!(vec.get::<i32>(0), Some(&4));
		assert_eq!(vec.get::<i32>(1), Some(&12));
		assert_eq!(vec.get::<i32>(2), Some(&36));
		assert_eq!(vec.get::<i32>(3), None);

		vec.push(64i32);
		vec.push(64i32);
		vec.push(64i32);

		drop(vec);
	}

	#[test]
	fn test_struct() {
		println!("std::any::TypeId::of::<Foo>() == {:?}", std::any::TypeId::of::<Foo>());

		let mut vec = ErasedVec::with_capacity::<Foo>(16);

		let foo1 = Foo { u8: 0x24, u16: 0x6f86, u32: 0xf4a342f4, u64: 0x7a7285f4f8fbd5eb, usize: 0x7d2772139c20fd39 };
		let foo2 = Foo { u8: 0xa9, u16: 0x68e4, u32: 0xb8cb2579, u64: 0x12c933706af64ccd, usize: 0x7d2772139c20fd39 };

		vec.push(foo1.clone());
		vec.push(foo2.clone());

		assert_eq!(vec.get(0), Some(&foo1));
		assert_eq!(vec.get(1), Some(&foo2));

		drop(vec);
	}

	#[test]
	fn test_zero_cap() {
		let mut vec = ErasedVec::new::<i32>();

		vec.push(4);
		vec.push(12);
		
		assert_eq!(vec.get::<i32>(0), Some(&4));
		assert_eq!(vec.get::<i32>(1), Some(&12));
	}

	#[test]
	fn test_pop() {
		let mut vec = ErasedVec::new::<i32>();

		vec.push(42);
		vec.push(68);

		vec.pop();

		assert_eq!(vec.get::<i32>(1), None);
		assert_eq!(vec.len(), 1);

		vec.pop();

		assert_eq!(vec.len(), 0);
	}

	#[test]
	#[should_panic]
	fn test_pop_empty() {
		let mut vec = ErasedVec::new::<i32>();

		vec.pop();
	}

	#[test]
	fn test_remove() {
		let mut vec = ErasedVec::new::<Foo>();

		let foo = Foo { u8: 0x24, u16: 0x6f86, u32: 0xf4a342f4, u64: 0x7a7285f4f8fbd5eb, usize: 0x7d2772139c20fd39 };

		vec.push(foo.clone());

		assert_eq!(vec.remove::<Foo>(0), foo);
	}

	#[test]
	fn test_iter() {
		let mut vec = ErasedVec::with_capacity::<i32>(4);

		vec.push(4i32);
		vec.push(12i32);
		vec.push(36i32);

		assert_eq!(vec.iter::<i32>().map(|i| *i).collect::<Vec<_>>(), vec![4, 12, 36]);
	}
}
