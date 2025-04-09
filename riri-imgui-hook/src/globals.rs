#![allow(dead_code, improper_ctypes)]
// This file was automatically generated from riri-imgui-hook-globals.
#[link(name = "cri_adx_globals", kind = "raw-dylib")]
unsafe extern "C" {
   /// Set the pointer to the memory location containing the beginning of TEST_STATIC.
    /// This method must only be called once, otherwise it will panic.
    pub(crate) unsafe fn set_test_static(ptr: *mut u8);
   /// Get a possible reference to TEST_STATIC. This checks to see if `set_test_static`
    /// was called previously and if either you or the hooked process have allocated the instance of it.
    pub(crate) unsafe fn get_test_static() -> Option<& 'static u8>;
   /// Like `get_test_static_mut`, but a mutable reference is created instead.
    pub(crate) unsafe fn get_test_static_mut() -> Option<& 'static mut u8>;
   /// An unchecked version of `get_test_static`. This assumes that TEST_STATIC
    /// is set and it's initialized.
    pub(crate) unsafe fn get_test_static_unchecked() -> & 'static u8;
   /// An unchecked version of `get_test_static_mut`. This assumes that TEST_STATIC
    /// is set and it's initialized.
    pub(crate) unsafe fn get_test_static_unchecked_mut() -> & 'static mut u8;

}

