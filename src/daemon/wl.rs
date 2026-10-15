//! Wayland event plumbing: registry, outputs, layer surfaces, callbacks.

use super::{Out, State, debug};
use std::time::Instant;
use wayland_client::protocol::{
    wl_buffer::{self, WlBuffer},
    wl_callback::{self, WlCallback},
    wl_compositor::WlCompositor,
    wl_output::{self, WlOutput},
    wl_region::WlRegion,
    wl_registry::{self, WlRegistry},
    wl_shm::WlShm,
    wl_shm_pool::WlShmPool,
    wl_surface::WlSurface,
};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum, delegate_noop};
use wayland_protocols::wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter};
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::ZwlrLayerShellV1,
    zwlr_layer_surface_v1::{self, ZwlrLayerSurfaceV1},
};

delegate_noop!(State: ignore WlCompositor);
delegate_noop!(State: ignore WlShm);
delegate_noop!(State: ignore WlShmPool);
delegate_noop!(State: ignore WlSurface);
delegate_noop!(State: ignore WlRegion);
delegate_noop!(State: ignore WpViewporter);
delegate_noop!(State: ignore WpViewport);
delegate_noop!(State: ignore ZxdgOutputManagerV1);
delegate_noop!(State: ignore ZwlrLayerShellV1);

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        st: &mut Self,
        reg: &WlRegistry,
        ev: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match ev {
            wl_registry::Event::Global { name, interface, version } => match interface.as_str() {
                "wl_compositor" if version >= 4 => {
                    st.g.compositor = Some(reg.bind(name, version.min(5), qh, ()));
                }
                "wl_shm" => st.g.shm = Some(reg.bind(name, 1, qh, ())),
                "zwlr_layer_shell_v1" => {
                    st.g.layer_shell = Some(reg.bind(name, version.min(4), qh, ()));
                }
                "wp_viewporter" => st.g.viewporter = Some(reg.bind(name, 1, qh, ())),
                "zxdg_output_manager_v1" => {
                    let m: ZxdgOutputManagerV1 = reg.bind(name, version.min(3), qh, ());
                    for o in &mut st.outputs {
                        if o.xdg.is_none() {
                            o.xdg = Some(m.get_xdg_output(&o.wl, qh, o.name));
                        }
                    }
                    st.g.xdg_mgr = Some(m);
                }
                "wl_output" if version >= 2 => {
                    let wl: WlOutput = reg.bind(name, version.min(4), qh, name);
                    let xdg = st.g.xdg_mgr.as_ref().map(|m| m.get_xdg_output(&wl, qh, name));
                    st.outputs.push(Out::new(name, wl, xdg));
                    debug(format_args!("output global {name} added"));
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(i) = st.outputs.iter().position(|o| o.name == name) {
                    let mut o = st.outputs.remove(i);
                    debug(format_args!("output {} ({name}) removed", o.label()));
                    o.teardown();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, u32> for State {
    fn event(
        st: &mut Self,
        _: &WlOutput,
        ev: wl_output::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(o) = st.outputs.iter_mut().find(|o| o.name == *name) else { return };
        match ev {
            wl_output::Event::Geometry { transform: WEnum::Value(t), .. } => o.transform = t,
            wl_output::Event::Mode { flags: WEnum::Value(f), width, height, .. }
                if f.contains(wl_output::Mode::Current) =>
            {
                o.mode = (width, height);
            }
            wl_output::Event::Name { name } => o.oname = name,
            wl_output::Event::Done => {
                o.info_done = true;
                // a closed surface comes back if the output is reconfigured
                o.closed = false;
            }
            _ => {}
        }
    }
}

impl Dispatch<ZxdgOutputV1, u32> for State {
    fn event(
        st: &mut Self,
        _: &ZxdgOutputV1,
        ev: zxdg_output_v1::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(o) = st.outputs.iter_mut().find(|o| o.name == *name) else { return };
        match ev {
            zxdg_output_v1::Event::LogicalPosition { x, y } => o.pos = (x, y),
            zxdg_output_v1::Event::Name { name } if o.oname.is_empty() => o.oname = name,
            _ => {}
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, u32> for State {
    fn event(
        st: &mut Self,
        ls: &ZwlrLayerSurfaceV1,
        ev: zwlr_layer_surface_v1::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(o) = st.outputs.iter_mut().find(|o| o.name == *name) else { return };
        match ev {
            zwlr_layer_surface_v1::Event::Configure { serial, width, height } => {
                ls.ack_configure(serial);
                if let Some(s) = o.surf.as_mut() {
                    s.logical = (width.max(1) as i32, height.max(1) as i32);
                    s.configured = true;
                    s.needs_commit = true;
                }
                debug(format_args!("{}: configure {width}x{height}", o.label()));
            }
            zwlr_layer_surface_v1::Event::Closed => {
                debug(format_args!("{}: layer surface closed", o.label()));
                o.drop_surface();
                o.closed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCallback, u32> for State {
    fn event(
        st: &mut Self,
        _: &WlCallback,
        ev: wl_callback::Event,
        name: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = ev {
            let now = Instant::now();
            if let Some(o) = st.outputs.iter_mut().find(|o| o.name == *name) {
                if let Some(s) = o.surf.as_mut() {
                    if let Some(t) = s.frame_pending.take() {
                        let waited = now.duration_since(t);
                        if st.debug_frames > 0 || waited.as_secs() >= 1 {
                            st.debug_frames = st.debug_frames.saturating_sub(1);
                            debug(format_args!(
                                "{}: frame callback after {} ms",
                                o.oname,
                                waited.as_millis()
                            ));
                        }
                    }
                }
            }
        }
    }
}

impl Dispatch<WlBuffer, ()> for State {
    fn event(
        st: &mut Self,
        buf: &WlBuffer,
        ev: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = ev {
            // The compositor keeps its texture copy; our pixels can go. The
            // pool was destroyed right after create_buffer, so this frees the
            // memfd once the compositor unmaps it.
            buf.destroy();
            st.released += 1;
            debug(format_args!("wl_buffer released -> destroyed ({} so far)", st.released));
        }
    }
}
