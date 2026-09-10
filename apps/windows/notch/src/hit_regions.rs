use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn set_hit_regions(app: AppHandle, rects: Vec<[f64; 4]>) -> Result<(), String> {
    let rectangles = validated_rectangles(&rects)?;
    let window = app
        .get_webview_window("notch")
        .ok_or("Notch window is unavailable.")?;
    #[cfg(windows)]
    {
        let handle = window.hwnd().map_err(|error| error.to_string())?;
        native::apply(handle.0 as isize, &rectangles)
    }
    #[cfg(not(windows))]
    {
        let _ = (window, rectangles);
        Err("Hit regions require Windows.".into())
    }
}

fn validated_rectangles(rectangles: &[[f64; 4]]) -> Result<Vec<[i32; 4]>, String> {
    if rectangles.len() > 4 {
        return Err("Too many interactive regions.".into());
    }
    rectangles.iter().map(validated_rectangle).collect()
}

fn validated_rectangle(rectangle: &[f64; 4]) -> Result<[i32; 4], String> {
    let [x, y, width, height] = *rectangle;
    if rectangle.iter().any(|number| !number.is_finite()) || width <= 0.0 || height <= 0.0 {
        return Err("Invalid interactive region.".into());
    }
    let bounds = [
        x.floor(),
        y.floor(),
        (x + width).ceil(),
        (y + height).ceil(),
    ];
    if bounds
        .iter()
        .any(|number| !(-1_000_000.0..=1_000_000.0).contains(number))
    {
        return Err("Interactive region exceeds supported screen coordinates.".into());
    }
    Ok(bounds.map(|number| number as i32))
}

#[cfg(windows)]
mod native {
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, HRGN, RGN_OR},
    };

    struct Region(HRGN);

    impl Region {
        fn rectangle(bounds: [i32; 4]) -> Result<Self, String> {
            let [left, top, right, bottom] = bounds;
            let region = unsafe { CreateRectRgn(left, top, right, bottom) };
            if region.is_invalid() {
                return Err("Could not create a native window region.".into());
            }
            Ok(Self(region))
        }

        fn merge(&self, other: &Region) -> Result<(), String> {
            if unsafe { CombineRgn(self.0, self.0, other.0, RGN_OR) }.0 == 0 {
                return Err("Could not combine native window regions.".into());
            }
            Ok(())
        }
    }

    impl Drop for Region {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteObject(self.0);
            }
        }
    }

    pub fn apply(handle: isize, rectangles: &[[i32; 4]]) -> Result<(), String> {
        let region = Region::rectangle([0, 0, 0, 0])?;
        for bounds in rectangles {
            region.merge(&Region::rectangle(*bounds)?)?;
        }
        if unsafe { SetWindowRgn(HWND(handle as *mut _), region.0, true) } == 0 {
            return Err("Could not apply native window regions.".into());
        }
        // SetWindowRgn transfers ownership to Windows on success.
        std::mem::forget(region);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::validated_rectangles;

    #[test]
    fn fractional_bounds_round_outward_and_invalid_regions_are_rejected() {
        assert_eq!(
            validated_rectangles(&[[10.5, 20.5, 5.2, 6.2]]).unwrap(),
            vec![[10, 20, 16, 27]]
        );
        assert!(validated_rectangles(&[]).unwrap().is_empty());
        for rectangle in [
            [0.0, 0.0, -1.0, 2.0],
            [0.0, 0.0, 1.0, 0.0],
            [f64::NAN, 0.0, 1.0, 2.0],
            [1_000_000.0, 0.0, 1.0, 2.0],
        ] {
            assert!(validated_rectangles(&[rectangle]).is_err());
        }
        assert!(validated_rectangles(&[[0.0, 0.0, 1.0, 1.0]; 5]).is_err());
    }
}
