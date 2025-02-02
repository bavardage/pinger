use core_foundation::{base::ToVoid, bundle::CFBundle};

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    pub static kLSSharedFileListSessionLoginItems: *const ::std::os::raw::c_void;
    fn LSSharedFileListCreate(
        allocator: *const ::std::os::raw::c_void,
        list_type: *const ::std::os::raw::c_void,
        options: *const ::std::os::raw::c_void,
    ) -> *const ::std::os::raw::c_void;
    fn LSSharedFileListInsertItemURL(
        list: *const ::std::os::raw::c_void,
        after: *const ::std::os::raw::c_void,
        display_name: *const ::std::os::raw::c_void,
        icon_ref: *const ::std::os::raw::c_void,
        item_url: *const ::std::os::raw::c_void,
        properties: *const ::std::os::raw::c_void,
        propertyKeys: *const ::std::os::raw::c_void,
    ) -> *const ::std::os::raw::c_void;
}

pub fn add_to_login_items() -> Result<(), &'static str> {
    unsafe {
        // Get the main bundle
        let bundle = CFBundle::main_bundle();

        // Get bundle URL
        let bundle_url = bundle.bundle_url();
        if bundle_url.is_none() {
            return Err("Failed to get bundle URL");
        }

        let login_items = LSSharedFileListCreate(
            std::ptr::null(),
            kLSSharedFileListSessionLoginItems,
            std::ptr::null(),
        );

        if login_items.is_null() {
            return Err("Failed to create LSSharedFileList");
        }

        let result = LSSharedFileListInsertItemURL(
            login_items,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            bundle_url.unwrap().to_void() as *const _,
            std::ptr::null(),
            std::ptr::null(),
        );

        if result.is_null() {
            Err("Failed to add login item")
        } else {
            Ok(())
        }
    }
}
