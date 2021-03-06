// Copyright 2016 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[deny(missing_docs)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate objc;
extern crate cocoa;
extern crate winit;
extern crate metal;
extern crate gfx_core;
extern crate gfx_device_metal;

use winit::os::macos::WindowExt;

use objc::runtime::{Object, Class, BOOL, YES, NO};

use cocoa::base::id as cocoa_id;
use cocoa::base::{selector, class};
use cocoa::foundation::{NSUInteger, NSSize};
use cocoa::appkit::{NSApp,
                    NSApplication, NSApplicationActivationPolicyRegular,
                    NSWindow, NSTitledWindowMask, NSBackingStoreBuffered,
                    NSMenu, NSMenuItem, NSRunningApplication, NSView,
                    NSApplicationActivateIgnoringOtherApps};

use gfx_core::tex::Size;
use gfx_core::format::{RenderFormat, Format};
use gfx_core::handle::{RawRenderTargetView, RenderTargetView};

use gfx_device_metal::{Device, Factory, Resources};

use metal::*;

use winit::{Window};

use std::ops::Deref;
use std::cell::Cell;
use std::mem;

pub struct MetalWindow {
    window: winit::Window,
    layer: CAMetalLayer,
    drawable: *mut CAMetalDrawable,
    backbuffer: *mut MTLTexture
}

impl Deref for MetalWindow {
    type Target = winit::Window;

    fn deref(&self) -> &winit::Window {
        &self.window
    }
}

impl MetalWindow {
    pub fn swap_buffers(&self) -> Result<(), ()> {
        // FIXME: release drawable before swapping
        // TODO: did we fail to swap buffers?
        // TODO: come up with alternative to this hack

        unsafe {
            //self.pool.get().drain();
            //self.pool.set(NSAutoreleasePool::alloc().init());

            let drawable = self.layer.next_drawable().unwrap();
            //drawable.retain();

            if !(*self.drawable).is_null() {
                (*self.drawable).release();
            }

            *self.drawable = drawable;

            *self.backbuffer = drawable.texture();
        }

        Ok(())
    }
}


#[derive(Copy, Clone, Debug)]
pub enum InitError {
    /// Unable to create a window.
    Window,
    /// Unable to map format to Metal.
    Format(Format),
    /// Unable to find a supported driver type.
    DriverType,
}

pub fn init<C: RenderFormat>(title: &str, requested_width: u32, requested_height: u32)
        -> Result<(MetalWindow, Device, Factory, RenderTargetView<Resources, C>), InitError>
{
    use gfx_core::factory::Typed;

    init_raw(title, requested_width, requested_height, C::get_format())
        .map(|(window, device, factory, color)| (window, device, factory, Typed::new(color)))
}

/// Initialize with a given size. Raw format version.
pub fn init_raw(title: &str, requested_width: u32, requested_height: u32, color_format: Format)
        -> Result<(MetalWindow, Device, Factory, RawRenderTargetView<Resources>), InitError>
{
    let winit_window = winit::WindowBuilder::new()
        .with_dimensions(requested_width, requested_height)
        .with_title(title.to_string()).build().unwrap();

    unsafe {
        let wnd: cocoa_id = mem::transmute(winit_window.get_nswindow());

        let layer = CAMetalLayer::layer();
        layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        /*layer.set_pixel_format(match gfx_device_metal::map_format(color_format, true) {
            Some(fm) => fm,
            None => return Err(InitError::Format(color_format)),
        });*/
        let draw_size = winit_window.get_inner_size().unwrap();
        layer.set_drawable_size(NSSize::new(draw_size.0 as f64, draw_size.1 as f64));
        layer.set_presents_with_transaction(false);
        layer.remove_all_animations();

        let view = wnd.contentView();
        view.setWantsLayer(YES);
        view.setLayer(mem::transmute(layer.0));

        let (mut device, factory, color, daddr, addr) = gfx_device_metal::create(color_format, draw_size.0, draw_size.1).unwrap();
        layer.set_device(device.device);

        let drawable = layer.next_drawable().unwrap();


        let window = MetalWindow {
            window: winit_window,
            layer: layer,
            drawable: daddr,
            backbuffer: addr
        };

        (*daddr).0 = drawable.0;
        (*addr).0 = drawable.texture().0;

        Ok((window, device, factory, color))
    }
}
