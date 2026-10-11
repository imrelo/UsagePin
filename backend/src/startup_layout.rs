use tauri::{PhysicalPosition, PhysicalRect, PhysicalSize, WebviewWindow};

const ASPECT_RATIO: f64 = 32.0 / 14.0;
const WIDTH_FRACTION: f64 = 0.10;
const RIGHT_MARGIN: f64 = 16.0;
const TOP_MARGIN: f64 = 64.0;

struct StartupLayout {
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
}

fn calculate(work_area: &PhysicalRect<i32, u32>, scale: f64) -> StartupLayout {
    let right_margin = (RIGHT_MARGIN * scale).round() as u32;
    let top_margin = (TOP_MARGIN * scale).round() as u32;
    let minimum_height = scale.ceil();
    let minimum_width = scale.ceil().max(minimum_height * ASPECT_RATIO);
    let maximum_width = f64::from(work_area.size.width.saturating_sub(right_margin)).min(
        f64::from(work_area.size.height.saturating_sub(top_margin)) * ASPECT_RATIO,
    );
    // Compute in physical pixels to avoid applying another monitor's DPI during startup.
    let width = (f64::from(work_area.size.width) * WIDTH_FRACTION)
        .max(minimum_width)
        .min(maximum_width.max(minimum_width))
        .round() as u32;
    let height = (f64::from(width) / ASPECT_RATIO).round() as u32;

    StartupLayout {
        position: PhysicalPosition::new(
            work_area.position.x + work_area.size.width as i32 - right_margin as i32 - width as i32,
            work_area.position.y + top_margin as i32,
        ),
        size: PhysicalSize::new(width, height),
    }
}

pub fn apply(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    let monitor = window.primary_monitor()?.ok_or("Primary monitor is unavailable")?;
    let layout = calculate(monitor.work_area(), monitor.scale_factor());
    window.set_position(layout.position)?;
    window.set_size(layout.size)?;
    // DPI changes can adjust window bounds after moving between monitors.
    window.set_position(layout.position)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work_area(x: i32, y: i32, width: u32, height: u32) -> PhysicalRect<i32, u32> {
        PhysicalRect {
            position: PhysicalPosition::new(x, y),
            size: PhysicalSize::new(width, height),
        }
    }

    #[test]
    fn scales_offsets_without_double_scaling_widget_size() {
        let area = work_area(0, 0, 1920, 1040);
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0] {
            let layout = calculate(&area, scale);
            assert_eq!(layout.size, PhysicalSize::new(192, 84));
            assert_eq!(layout.position.y, (64.0 * scale).round() as i32);
            assert_eq!(1920 - layout.position.x - 192, (16.0 * scale).round() as i32);
        }
    }

    #[test]
    fn respects_work_area_origin_and_high_resolution() {
        let area = work_area(-3840, 48, 3840, 2064);
        let layout = calculate(&area, 2.0);
        assert_eq!(layout.size, PhysicalSize::new(384, 168));
        assert_eq!(layout.position, PhysicalPosition::new(-416, 176));
    }

    #[test]
    fn small_displays_use_ten_percent_and_stay_inside_work_area() {
        let area = work_area(40, 32, 800, 560);
        let scale = 2.0;
        let layout = calculate(&area, scale);
        assert_eq!(layout.size, PhysicalSize::new(80, 35));
        assert_eq!(layout.position, PhysicalPosition::new(728, 160));
        assert!(layout.position.x >= area.position.x);
        assert!(layout.position.y + layout.size.height as i32 <= 592);
    }

    #[test]
    fn wide_displays_fit_vertically_and_preserve_aspect_ratio() {
        let area = work_area(0, 0, 7680, 300);
        let layout = calculate(&area, 1.0);
        assert!(layout.size.height <= 236);
        assert!((f64::from(layout.size.height) - f64::from(layout.size.width) / ASPECT_RATIO).abs() <= 0.5);
        assert_eq!(layout.position.y, 64);
        assert_eq!(layout.position.x + layout.size.width as i32 + 16, 7680);
    }
}
