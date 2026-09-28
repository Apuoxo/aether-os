    unsafe { DIRTY_FULL = true; }
}


fn eq_cmd(cmd: &[u8], clen: usize, expect: &[u8]) -> bool {
    if clen != expect.len() {
        return false;
    }
    let mut i = 0usize;
    while i < clen {
        if cmd[i] != expect[i] {
            return false;
        }
        i += 1;
    }
    true
}



fn bring_to_front(idx: usize) {
    unsafe {
        // Already-focused/front window: no visual state changed, so do not
        // request a full framebuffer repaint on a normal mouse click.
        if FOCUS == idx && WINS[idx].z == Z_TOP {
            return;
        }
        Z_TOP += 1;
        WINS[idx].z = Z_TOP;
        FOCUS = idx;
        DIRTY_FULL = true;
    }
}

fn hit_test(mx: i32, my: i32) -> Option<usize> {
    unsafe {
        let mut best: Option<usize> = None;
        let mut best_z = (-2147483647-1);
        let mut i = 0usize;
        while i < MAX_WIN {
            if WINS[i].visible {
                let w = &WINS[i];
                if mx >= w.x && mx < w.x + w.w && my >= w.y && my < w.y + w.h {
                    if w.z >= best_z {
                        best_z = w.z;
                        best = Some(i);
                    }
                }
            }
            i += 1;
        }
        best
    }
}

fn in_title(idx: usize, mx: i32, my: i32) -> bool {