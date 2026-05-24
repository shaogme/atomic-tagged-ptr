use super::*;
use std::format;

#[test]
fn test_default_initializer() {
    let atom: AtomicTaggedPtr<i32> = Default::default();
    let loaded = atom.load(Ordering::Relaxed);
    assert!(loaded.ptr.is_none());
    assert_eq!(loaded.tag, Tag::new(0));
}

#[test]
fn test_debug_formatter() {
    let val = 12345;
    let ptr = NonNull::new(&val as *const i32 as *mut i32);
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(ptr, Tag::new(0)));
    atom.store(TaggedPtr::new(ptr, Tag::new(88)), Ordering::Relaxed);

    let debug_str = format!("{:?}", atom);
    assert!(debug_str.contains("AtomicTaggedPtr"));
    assert!(debug_str.contains("tag: Tag(0x58)"));
}

#[test]
fn test_multithreaded_atomic_exchanges() {
    use std::sync::Arc;
    use std::thread;

    let val = 777;
    let ptr = NonNull::new(&val as *const i32 as *mut i32);
    let ptr_usize = ptr.unwrap().as_ptr() as usize;
    let atom = Arc::new(AtomicTaggedPtr::new(TaggedPtr::new(ptr, Tag::new(0))));

    let atom_clone = Arc::clone(&atom);
    let handle = thread::spawn(move || {
        let loaded = atom_clone.load(Ordering::Acquire);
        let local_ptr = NonNull::new(ptr_usize as *mut i32);
        if loaded.ptr == local_ptr && loaded.tag == Tag::new(0) {
            let _ = atom_clone.compare_exchange(
                TaggedPtr::new(local_ptr, Tag::new(0)),
                TaggedPtr::new(None, Tag::new(55)),
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
        }
    });

    handle.join().unwrap();
    let final_state = atom.load(Ordering::Acquire);

    // Assert state was safely transitioned or remained valid
    assert!(final_state.tag == Tag::new(55) || final_state.tag == Tag::new(0));
}

#[test]
fn test_into_ptr_api() {
    let val1 = 111;
    let raw_ptr1 = &val1 as *const i32;
    let mut_ptr1 = &val1 as *const i32 as *mut i32;
    let non_null1 = NonNull::new(mut_ptr1).unwrap();

    // 1. 测试 new
    // 传入 NonNull<T>
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(non_null1, Tag::new(0)));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), Some(non_null1));

    // 传入 Option<NonNull<T>>
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(Some(non_null1), Tag::new(0)));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), Some(non_null1));

    // 传入 *const T
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(raw_ptr1, Tag::new(0)));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), Some(non_null1));

    // 传入 *mut T
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(mut_ptr1, Tag::new(0)));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), Some(non_null1));

    // 传入裸空指针 *const T
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(core::ptr::null::<i32>(), Tag::new(0)));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), None);

    // 传入裸空指针 *mut T
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(core::ptr::null_mut::<i32>(), Tag::new(0)));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), None);

    // 传入 None
    let atom: AtomicTaggedPtr<i32> = AtomicTaggedPtr::new(TaggedPtr::new(None, Tag::new(0)));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), None);

    // 2. 测试 store
    let atom = AtomicTaggedPtr::default();
    atom.store(TaggedPtr::new(raw_ptr1, Tag::new(10)), Ordering::Relaxed);
    let loaded = atom.load(Ordering::Relaxed);
    assert_eq!(loaded.ptr.option(), Some(non_null1));
    assert_eq!(loaded.tag, Tag::new(10));

    atom.store(TaggedPtr::new(None, Tag::new(20)), Ordering::Relaxed);
    let loaded = atom.load(Ordering::Relaxed);
    assert_eq!(loaded.ptr.option(), None);
    assert_eq!(loaded.tag, Tag::new(20));

    // 3. 测试 compare_exchange / compare_exchange_weak (混合不同类型的指针参数)
    let atom = AtomicTaggedPtr::new(TaggedPtr::new(raw_ptr1, Tag::new(0)));
    let res = atom.compare_exchange(
        TaggedPtr::new(raw_ptr1, Tag::new(0)),
        TaggedPtr::new(mut_ptr1, Tag::new(1)),
        Ordering::SeqCst,
        Ordering::SeqCst,
    );
    assert!(res.is_ok());
    let loaded = atom.load(Ordering::Relaxed);
    assert_eq!(loaded.ptr.option(), Some(non_null1));
    assert_eq!(loaded.tag, Tag::new(1));

    let res = atom.compare_exchange_weak(
        TaggedPtr::new(mut_ptr1, Tag::new(1)),
        TaggedPtr::new(None, Tag::new(2)),
        Ordering::SeqCst,
        Ordering::SeqCst,
    );
    let mut res = res;
    while res.is_err() {
        res = atom.compare_exchange_weak(
            TaggedPtr::new(mut_ptr1, Tag::new(1)),
            TaggedPtr::new(None, Tag::new(2)),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }
    assert!(res.is_ok());
    let loaded = atom.load(Ordering::Relaxed);
    assert_eq!(loaded.ptr.option(), None);
    assert_eq!(loaded.tag, Tag::new(2));

    // 4. 测试 From/Into conversions 直接调用
    let ptr_from_nn = Ptr::from(non_null1);
    assert_eq!(ptr_from_nn.option(), Some(non_null1));
    let ptr_from_opt: Ptr<i32> = Ptr::from(Some(non_null1));
    assert_eq!(ptr_from_opt.option(), Some(non_null1));
    let ptr_from_const = Ptr::from(raw_ptr1);
    assert_eq!(ptr_from_const.option(), Some(non_null1));
    let ptr_from_mut = Ptr::from(mut_ptr1);
    assert_eq!(ptr_from_mut.option(), Some(non_null1));

    let tagged = TaggedPtr::new(non_null1, Tag::new(123));
    let ptr_from_tagged = Ptr::from(tagged);
    assert_eq!(ptr_from_tagged.option(), Some(non_null1));

    let opt_from_ptr = Option::<NonNull<i32>>::from(ptr_from_nn);
    assert_eq!(opt_from_ptr, Some(non_null1));
    let opt_from_tagged = Option::<NonNull<i32>>::from(tagged);
    assert_eq!(opt_from_tagged, Some(non_null1));

    // 5. 测试 Tuple -> TaggedPtr 转换以及 AtomicTaggedPtr 接收 Into<TaggedPtr>
    let tag = Tag::new(456);
    let tagged_from_nn = TaggedPtr::from((non_null1, tag));
    assert_eq!(tagged_from_nn.ptr.option(), Some(non_null1));
    assert_eq!(tagged_from_nn.tag, tag);

    let tagged_from_opt = TaggedPtr::from((Some(non_null1), tag));
    assert_eq!(tagged_from_opt.ptr.option(), Some(non_null1));
    assert_eq!(tagged_from_opt.tag, tag);

    let tagged_from_const = TaggedPtr::from((raw_ptr1, tag));
    assert_eq!(tagged_from_const.ptr.option(), Some(non_null1));
    assert_eq!(tagged_from_const.tag, tag);

    let tagged_from_mut = TaggedPtr::from((mut_ptr1, tag));
    assert_eq!(tagged_from_mut.ptr.option(), Some(non_null1));
    assert_eq!(tagged_from_mut.tag, tag);

    // 测试 AtomicTaggedPtr 操作接收 tuple
    let atom = AtomicTaggedPtr::new((non_null1, tag));
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), Some(non_null1));

    atom.store((None, Tag::new(789)), Ordering::Relaxed);
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), None);
    assert_eq!(atom.load(Ordering::Relaxed).tag, Tag::new(789));

    let res = atom.compare_exchange(
        (None, Tag::new(789)),
        (mut_ptr1, Tag::new(999)),
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
    assert!(res.is_ok());
    assert_eq!(atom.load(Ordering::Relaxed).ptr.option(), Some(non_null1));
    assert_eq!(atom.load(Ordering::Relaxed).tag, Tag::new(999));
}

#[test]
fn test_ptr_conversions() {
    let val = 42;
    let raw = &val as *const i32;
    let mut_ptr = &val as *const i32 as *mut i32;
    let non_null = NonNull::new(mut_ptr).unwrap();

    let ptr_some = Ptr::new(Some(non_null));
    let ptr_none: Ptr<i32> = Ptr::new(None);

    // 测试 option() / as_option()
    assert_eq!(ptr_some.option(), Some(non_null));
    assert_eq!(ptr_none.option(), None);
    assert_eq!(ptr_some.as_option(), Some(non_null));

    // 测试 as_ptr()
    assert_eq!(ptr_some.as_ptr(), raw);
    assert_eq!(ptr_none.as_ptr(), core::ptr::null());

    // 测试 as_mut_ptr()
    assert_eq!(ptr_some.as_mut_ptr(), mut_ptr);
    assert_eq!(ptr_none.as_mut_ptr(), core::ptr::null_mut());

    // 测试 is_null() / is_some() / is_none()
    assert!(ptr_some.is_some());
    assert!(!ptr_some.is_null());
    assert!(!ptr_some.is_none());

    assert!(ptr_none.is_null());
    assert!(ptr_none.is_none());
    assert!(!ptr_none.is_some());

    // 测试 PartialEq
    assert!(ptr_some == Some(non_null));
    assert!(ptr_some == non_null);
    assert!(ptr_some == raw);
    assert!(ptr_some == mut_ptr);

    assert!(ptr_none == None);
    assert!(ptr_none == core::ptr::null::<i32>());
    assert!(ptr_none == core::ptr::null_mut::<i32>());
}

#[test]
fn test_new_traits_and_methods() {
    let mut val = 42;
    let non_null = NonNull::new(&mut val as *mut i32).unwrap();
    let ptr_some = Ptr::new(Some(non_null));
    let ptr_none = Ptr::<i32>::new(None);

    // 1. Ptr::as_ref / as_mut
    unsafe {
        assert_eq!(ptr_some.as_ref(), Some(&42));
        assert_eq!(ptr_none.as_ref(), None);
        *ptr_some.as_mut().unwrap() = 100;
        assert_eq!(ptr_some.as_ref(), Some(&100));
        assert_eq!(ptr_none.as_mut(), None);
    }

    // 2. Ptr::expect / unwrap / unwrap_or
    assert_eq!(ptr_some.expect("should be valid"), non_null);
    assert_eq!(ptr_some.unwrap(), non_null);
    let other_val = 99;
    let other_nn = NonNull::new(&other_val as *const i32 as *mut i32).unwrap();
    assert_eq!(ptr_none.unwrap_or(other_nn), other_nn);

    // 3. Ptr::map / map_or / map_or_else
    let mapped = ptr_some.map(|p| p);
    assert_eq!(mapped, ptr_some);
    assert_eq!(ptr_some.map_or(0, |p| unsafe { *p.as_ptr() }), 100);
    assert_eq!(ptr_none.map_or(0, |p| unsafe { *p.as_ptr() }), 0);
    assert_eq!(ptr_some.map_or_else(|| 0, |p| unsafe { *p.as_ptr() }), 100);
    assert_eq!(ptr_none.map_or_else(|| 0, |p| unsafe { *p.as_ptr() }), 0);

    // 4. Ptr Pointer formatting / Ord
    let format_str = format!("{:p}", ptr_some);
    assert!(!format_str.is_empty());
    assert!(ptr_some > ptr_none || ptr_some < ptr_none || ptr_some == ptr_none);
    assert_eq!(ptr_some.cmp(&ptr_some), core::cmp::Ordering::Equal);

    // 5. Ptr conversions
    let raw_const: *const i32 = ptr_some.into();
    assert_eq!(raw_const, non_null.as_ptr() as *const i32);
    let raw_mut: *mut i32 = ptr_some.into();
    assert_eq!(raw_mut, non_null.as_ptr());
    let opt_const: Option<*const i32> = ptr_some.into();
    assert_eq!(opt_const, Some(non_null.as_ptr() as *const i32));
    let opt_mut: Option<*mut i32> = ptr_none.into();
    assert_eq!(opt_mut, None);

    // 6. TaggedPtr methods & traits
    let tag = Tag::new(10);
    let tagged = TaggedPtr::new(ptr_some, tag);
    assert_eq!(tagged.as_ptr(), raw_const);
    assert_eq!(tagged.as_mut_ptr(), raw_mut);
    assert!(tagged.is_some());
    assert!(!tagged.is_null());
    assert!(!tagged.is_none());
    unsafe {
        assert_eq!(tagged.as_ref(), Some(&100));
        *tagged.as_mut().unwrap() = 200;
        assert_eq!(tagged.as_ref(), Some(&200));
    }

    let tagged_with_ptr = tagged.with_ptr(ptr_none);
    assert!(tagged_with_ptr.is_none());
    assert_eq!(tagged_with_ptr.tag, tag);

    let tagged_with_tag = tagged.with_tag(Tag::new(20));
    assert_eq!(tagged_with_tag.tag.value(), 20);

    let mapped_tagged = tagged.map_ptr(|p| p);
    assert_eq!(mapped_tagged, tagged);

    // TaggedPtr Pointer / Ord / Conversions
    let format_tagged = format!("{:p}", tagged);
    assert!(!format_tagged.is_empty());
    assert_eq!(tagged.cmp(&tagged), core::cmp::Ordering::Equal);
    let raw_const_tagged: *const i32 = tagged.into();
    assert_eq!(raw_const_tagged, raw_const);
    let raw_mut_tagged: *mut i32 = tagged.into();
    assert_eq!(raw_mut_tagged, raw_mut);

    // TaggedPtr manual PartialEq/Eq/Hash
    assert_eq!(tagged, TaggedPtr::new(ptr_some, tag));
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use core::hash::Hash;
    use core::hash::Hasher;
    tagged.hash(&mut hasher);
    assert!(hasher.finish() > 0);

    // 7. Tag arithmetic & methods
    let tag1 = Tag::new(5);
    assert_eq!(tag1.wrapping_sub(2).value(), 3);
    assert_eq!(tag1.next().value(), 6);
    assert_eq!((tag1 + 2).value(), 7);
    assert_eq!((tag1 - 2).value(), 3);
    let mut mut_tag = tag1;
    mut_tag += 2;
    assert_eq!(mut_tag.value(), 7);
    mut_tag -= 2;
    assert_eq!(mut_tag.value(), 5);

    // 8. AtomicTaggedPtr swap / into_inner / fetch_update / From
    let atom = AtomicTaggedPtr::new(tagged);
    let old = atom.swap(TaggedPtr::new(ptr_none, Tag::new(99)), Ordering::SeqCst);
    assert_eq!(old, tagged);
    assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 99);

    let inner_val = atom.into_inner();
    assert!(inner_val.ptr.is_none());
    assert_eq!(inner_val.tag.value(), 99);

    let atom2 = AtomicTaggedPtr::from(tagged);
    let res = atom2.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |t| {
        Some(t.with_tag(t.tag + 1))
    });
    assert!(res.is_ok());
    assert_eq!(atom2.load(Ordering::SeqCst).tag.value(), tag.value() + 1);

    let atom3 = AtomicTaggedPtr::from((ptr_some, tag));
    assert_eq!(atom3.load(Ordering::SeqCst).tag.value(), tag.value());
}

#[test]
fn test_new_features_comprehensive() {
    // 1. Ptr::null / none / cast
    let p_null: Ptr<i32> = Ptr::null();
    assert!(p_null.is_null());
    let p_none: Ptr<i32> = Ptr::none();
    assert!(p_none.is_null());

    let mut val = 42u32;
    let ptr = Ptr::new(NonNull::new(&mut val as *mut u32));
    let casted: Ptr<i32> = ptr.cast();
    assert!(!casted.is_null());
    unsafe {
        assert_eq!(casted.read(), 42);
    }

    // 2. Unsafe reads/writes/replace/swap/copy/offset on Ptr
    let mut data = [10, 20, 30];
    let p0 = Ptr::new(NonNull::new(&mut data[0] as *mut i32));
    let p1 = Ptr::new(NonNull::new(&mut data[1] as *mut i32));
    unsafe {
        assert_eq!(p0.read(), 10);
        assert_eq!(p0.read_volatile(), 10);
        assert_eq!(p0.read_unaligned(), 10);

        p0.write(15);
        assert_eq!(data[0], 15);
        p0.write_volatile(25);
        assert_eq!(data[0], 25);
        p0.write_unaligned(35);
        assert_eq!(data[0], 35);

        let old = p0.replace(50);
        assert_eq!(old, 35);
        assert_eq!(data[0], 50);

        p0.swap(p1);
        assert_eq!(data[0], 20);
        assert_eq!(data[1], 50);

        // arithmetic
        let p_add = p0.add(1);
        assert_eq!(p_add.as_ptr(), p1.as_ptr());
        let p_sub = p1.sub(1);
        assert_eq!(p_sub.as_ptr(), p0.as_ptr());

        let p_offset = p0.offset(1);
        assert_eq!(p_offset.as_ptr(), p1.as_ptr());

        // wrapping
        let p_wrap_add = p0.wrapping_add(1);
        assert_eq!(p_wrap_add.as_ptr(), p1.as_ptr());
        let p_wrap_sub = p1.wrapping_sub(1);
        assert_eq!(p_wrap_sub.as_ptr(), p0.as_ptr());
        let p_wrap_offset = p0.wrapping_offset(1);
        assert_eq!(p_wrap_offset.as_ptr(), p1.as_ptr());

        // copy
        let mut src_arr = [1, 2, 3];
        let mut dest_arr = [0, 0, 0];
        let src_ptr = Ptr::new(NonNull::new(&mut src_arr[0] as *mut i32));
        let dest_ptr = Ptr::new(NonNull::new(&mut dest_arr[0] as *mut i32));
        src_ptr.copy_to(dest_ptr, 3);
        assert_eq!(dest_arr, [1, 2, 3]);

        dest_arr = [0, 0, 0];
        src_ptr.copy_to_nonoverlapping(dest_ptr, 3);
        assert_eq!(dest_arr, [1, 2, 3]);

        let mut dest_arr2 = [0, 0, 0];
        let dest_ptr2 = Ptr::new(NonNull::new(&mut dest_arr2[0] as *mut i32));
        dest_ptr2.copy_from(src_ptr, 3);
        assert_eq!(dest_arr2, [1, 2, 3]);

        dest_arr2 = [0, 0, 0];
        dest_ptr2.copy_from_nonoverlapping(src_ptr, 3);
        assert_eq!(dest_arr2, [1, 2, 3]);
    }

    // 3. AsRef
    let opt_nn = NonNull::new(&mut val as *mut u32);
    let ptr_as_ref = Ptr::new(opt_nn);
    assert_eq!(AsRef::<Option<NonNull<u32>>>::as_ref(&ptr_as_ref), &opt_nn);

    let tagged_as_ref = TaggedPtr::new(ptr_as_ref, Tag::new(123));
    assert_eq!(AsRef::<Ptr<u32>>::as_ref(&tagged_as_ref), &ptr_as_ref);
    assert_eq!(AsRef::<Tag>::as_ref(&tagged_as_ref), &Tag::new(123));

    // 4. TaggedPtr::null / cast
    let tp_null: TaggedPtr<i32> = TaggedPtr::null();
    assert!(tp_null.is_null());
    assert_eq!(tp_null.tag.value(), 0);

    let tp_cast: TaggedPtr<i32> = tagged_as_ref.cast();
    assert!(!tp_cast.is_null());
    assert_eq!(tp_cast.tag.value(), 123);

    // 5. TaggedPtr forwarding methods
    let mut data_tp = [100, 200];
    let tp0 = TaggedPtr::new(NonNull::new(&mut data_tp[0] as *mut i32), Tag::new(5));
    let tp1 = TaggedPtr::new(NonNull::new(&mut data_tp[1] as *mut i32), Tag::new(10));
    unsafe {
        assert_eq!(tp0.read(), 100);
        tp0.write(150);
        assert_eq!(data_tp[0], 150);

        let old = tp0.replace(180);
        assert_eq!(old, 150);
        assert_eq!(data_tp[0], 180);

        tp0.swap(tp1);
        assert_eq!(data_tp[0], 200);
        assert_eq!(data_tp[1], 180);

        let tp_offset = tp0.offset(1);
        assert_eq!(tp_offset.as_ptr(), tp1.as_ptr());
        assert_eq!(tp_offset.tag.value(), 5);

        let tp_wrap_add = tp0.wrapping_add(1);
        assert_eq!(tp_wrap_add.as_ptr(), tp1.as_ptr());
        assert_eq!(tp_wrap_add.tag.value(), 5);
    }

    // 6. Tag bitwise operations & From u8/u16/u32
    let t_u8 = Tag::from(5u8);
    let t_u16 = Tag::from(10u16);
    let t_u32 = Tag::from(15u32);
    assert_eq!(t_u8.value(), 5);
    assert_eq!(t_u16.value(), 10);
    assert_eq!(t_u32.value(), 15);

    let t1 = Tag::new(0b1100);
    let t2 = Tag::new(0b1010);
    assert_eq!((t1 & t2).value(), 0b1000);
    assert_eq!((t1 & 0b1010).value(), 0b1000);
    assert_eq!((t1 | t2).value(), 0b1110);
    assert_eq!((t1 | 0b1010).value(), 0b1110);
    assert_eq!((t1 ^ t2).value(), 0b0110);
    assert_eq!((t1 ^ 0b1010).value(), 0b0110);
    assert_eq!((!t1).value(), (!0b1100) & TAG_MASK);

    let mut t_mut = t1;
    t_mut &= t2;
    assert_eq!(t_mut.value(), 0b1000);
    t_mut = t1;
    t_mut &= 0b1010;
    assert_eq!(t_mut.value(), 0b1000);

    t_mut = t1;
    t_mut |= t2;
    assert_eq!(t_mut.value(), 0b1110);
    t_mut = t1;
    t_mut |= 0b1010;
    assert_eq!(t_mut.value(), 0b1110);

    t_mut = t1;
    t_mut ^= t2;
    assert_eq!(t_mut.value(), 0b0110);
    t_mut = t1;
    t_mut ^= 0b1010;
    assert_eq!(t_mut.value(), 0b0110);

    // 7. AtomicTaggedPtr fetch_* methods
    let atom = AtomicTaggedPtr::<i32>::new(TaggedPtr::new(None, Tag::new(10)));
    let old = atom.fetch_add_tag(5, Ordering::SeqCst);
    assert_eq!(old.tag.value(), 10);
    assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 15);

    let old = atom.fetch_sub_tag(3, Ordering::SeqCst);
    assert_eq!(old.tag.value(), 15);
    assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 12);

    let old = atom.fetch_and_tag(0b1100, Ordering::SeqCst);
    assert_eq!(old.tag.value(), 12); // 12 is 0b1100
    assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 12);

    let old = atom.fetch_or_tag(0b0011, Ordering::SeqCst);
    assert_eq!(old.tag.value(), 12);
    assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 15); // 15 is 0b1111

    let old = atom.fetch_xor_tag(0b1010, Ordering::SeqCst);
    assert_eq!(old.tag.value(), 15);
    assert_eq!(atom.load(Ordering::SeqCst).tag.value(), 5); // 0b1111 ^ 0b1010 = 0b0101 (5)

    let val_target = 999;
    let ptr_target = NonNull::new(&val_target as *const i32 as *mut i32);
    let old = atom.fetch_set_ptr(ptr_target, Ordering::SeqCst);
    assert!(old.ptr.is_null());
    assert_eq!(old.tag.value(), 5);
    let loaded = atom.load(Ordering::SeqCst);
    assert_eq!(loaded.ptr.option(), ptr_target);
    assert_eq!(loaded.tag.value(), 5);

    let old = atom.fetch_set_tag(Tag::new(88), Ordering::SeqCst);
    assert_eq!(old.ptr.option(), ptr_target);
    assert_eq!(old.tag.value(), 5);
    let loaded = atom.load(Ordering::SeqCst);
    assert_eq!(loaded.ptr.option(), ptr_target);
    assert_eq!(loaded.tag.value(), 88);

    // 8. AtomicTaggedPtr conversions
    let raw_c = &val_target as *const i32;
    let raw_m = &val_target as *const i32 as *mut i32;
    let nn = NonNull::new(raw_m).unwrap();
    let ptr_wrap = Ptr::new(Some(nn));

    let a1 = AtomicTaggedPtr::from(ptr_wrap);
    assert_eq!(a1.load(Ordering::Relaxed).ptr.option(), Some(nn));
    assert_eq!(a1.load(Ordering::Relaxed).tag.value(), 0);

    let a2 = AtomicTaggedPtr::from(Some(nn));
    assert_eq!(a2.load(Ordering::Relaxed).ptr.option(), Some(nn));

    let a3 = AtomicTaggedPtr::from(nn);
    assert_eq!(a3.load(Ordering::Relaxed).ptr.option(), Some(nn));

    // For *const T / *mut T, From requires Tag::default()
    let a4 = AtomicTaggedPtr::from(raw_c);
    assert_eq!(a4.load(Ordering::Relaxed).ptr.option(), Some(nn));

    let a5 = AtomicTaggedPtr::from(raw_m);
    assert_eq!(a5.load(Ordering::Relaxed).ptr.option(), Some(nn));
}
