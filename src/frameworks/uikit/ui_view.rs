/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIView`.

use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect};
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{
    id, msg, nil, objc_classes, release, retain, Class, ClassExports, HostObject, NSZonePtr,
};

#[derive(Default)]
pub struct State {
    pub(super) views: Vec<id>,
}

#[derive(Default)]
pub(super) struct UIViewHostObject {
    /// CALayer or subclass.
    layer: id,
    /// Subviews in back-to-front order. These are strong references.
    subviews: Vec<id>,
    /// The superview. This is a weak reference.
    superview: id,
}
impl HostObject for UIViewHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIView: UIResponder

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (Class)layerClass {
    env.objc.get_known_class("CALayer", &mut env.mem)
}

// TODO: accessors etc

- (id)init {
    let view_class: Class = msg![env; this class];
    let layer_class: Class = msg![env; view_class layerClass];
    let layer: id = msg![env; layer_class layer];

    // CALayer is not opaque by default, but UIView is
    () = msg![env; layer setDelegate:this];
    () = msg![env; layer setOpaque:true];

    env.objc.borrow_mut::<UIViewHostObject>(this).layer = layer;

    env.framework_state.uikit.ui_view.views.push(this);

    this
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg![env; this init];

    () = msg![env; this setFrame:frame];

    log_dbg!(
        "[(UIView*){:?} initWithFrame:{:?}] => bounds {:?}, center {:?}",
        this,
        frame,
        { let bounds: CGRect = msg![env; this bounds]; bounds },
        { let center: CGPoint = msg![env; this center]; center },
    );

    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this: id = msg![env; this init];

    // TODO: decode the various other UIView properties

    let key_ns_string = get_static_str(env, "UIBounds");
    let bounds: CGRect = msg![env; coder decodeCGRectForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UICenter");
    let center: CGPoint = msg![env; coder decodeCGPointForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UISubviews");
    let subviews: id = msg![env; coder decodeObjectForKey:key_ns_string];
    let subview_count: NSUInteger = msg![env; subviews count];

    log_dbg!(
        "[(UIView*){:?} initWithCoder:{:?}] => bounds {}, center {}, {} subviews",
        this,
        coder,
        bounds,
        center,
        subview_count,
    );

    () = msg![env; this setBounds:bounds];
    () = msg![env; this setCenter:center];

    for i in 0..subview_count {
        let subview: id = msg![env; subviews objectAtIndex:i];
        () = msg![env; this addSubview:subview];
    }

    this
}

// TODO: setMultipleTouchEnabled
- (())setMultipleTouchEnabled:(bool)_enabled {
    // TODO: enable multitouch
}

- (())layoutSubviews {
    // On iOS 5.1 and earlier, the default implementation of this method does nothing.
}

- (id)superview {
    env.objc.borrow::<UIViewHostObject>(this).superview
}
// TODO: subviews accessor

- (())addSubview:(id)view {
    if env.objc.borrow::<UIViewHostObject>(view).superview == this {
        () = msg![env; this bringSubviewToFront:view];
    } else {
        retain(env, view);
        () = msg![env; view removeFromSuperview];
        env.objc.borrow_mut::<UIViewHostObject>(view).superview = this;
        env.objc.borrow_mut::<UIViewHostObject>(this).subviews.push(view);
    }
}

- (())bringSubviewToFront:(id)subview {
    let UIViewHostObject { ref mut subviews, .. } = env.objc.borrow_mut(this);
    let idx = subviews.iter().position(|&subview2| subview2 == subview).unwrap();
    let subview2 = subviews.remove(idx);
    assert!(subview2 == subview);
    subviews.push(subview);
}

- (())removeFromSuperview {
    let UIViewHostObject { ref mut superview, .. } = env.objc.borrow_mut(this);
    let superview = std::mem::take(superview);
    if superview == nil {
        return;
    }

    let UIViewHostObject { ref mut subviews, .. } = env.objc.borrow_mut(superview);
    let idx = subviews.iter().position(|&subview| subview == this).unwrap();
    let subview = subviews.remove(idx);
    assert!(subview == this);
    release(env, this);
}

- (())dealloc {
    let UIViewHostObject {
        layer,
        superview,
        subviews,
    } = std::mem::take(env.objc.borrow_mut(this));

    release(env, layer);
    assert!(superview == nil);
    for subview in subviews {
        env.objc.borrow_mut::<UIViewHostObject>(subview).superview = nil;
        release(env, subview);
    }

    env.framework_state.uikit.ui_view.views.swap_remove(
        env.framework_state.uikit.ui_view.views.iter().position(|&v| v == this).unwrap()
    );

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)layer {
    env.objc.borrow_mut::<UIViewHostObject>(this).layer
}

- (bool)isOpaque {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer isOpaque]
}
- (())setOpaque:(bool)opaque {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setOpaque:opaque]
}

- (CGFloat)alpha {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer opacity]
}
- (())setAlpha:(CGFloat)alpha {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setOpacity:alpha]
}

- (id)backgroundColor {
    nil // this is the actual default (equivalent to transparency)
}
- (())setBackgroundColor:(id)_color { // UIColor*
    // TODO: implement this once views are actually rendered
}

- (CGRect)bounds {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer bounds]
}
- (())setBounds:(CGRect)bounds {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setBounds:bounds]
}
- (CGPoint)center {
    // FIXME: what happens if [layer anchorPoint] isn't (0.5, 0.5)?
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer position]
}
- (())setCenter:(CGRect)center {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setPosition:center]
}
- (CGRect)frame {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer frame]
}
- (())setFrame:(CGRect)frame {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setFrame:frame]
}

@end

@implementation UIAlertView: UIView
- (id)initWithTitle:(id)title
                      message:(id)message
                     delegate:(id)delegate
            cancelButtonTitle:(id)cancelButtonTitle
            otherButtonTitles:(id)otherButtonTitles {

    log!("TODO: [(UIAlertView*){:?} initWithTitle:{:?} message:{:?} delegate:{:?} cancelButtonTitle:{:?} otherButtonTitles:{:?}]", this, title, message, delegate, cancelButtonTitle, otherButtonTitles);

    let msg = to_rust_string(env, message);
    let title = to_rust_string(env, title);

    log!("UIAlertView: title: {:?}, message: {:?}", title, msg);

    let host_object: &mut UIViewHostObject = env.objc.borrow_mut(this);
    let layer = host_object.layer;
    () = msg![env; layer setDelegate:this];

    env.framework_state.uikit.ui_view.views.push(this);

    this
}
- (())show {
    log!("TODO: [(UIAlertView*){:?} show]", this);
}
@end

};
