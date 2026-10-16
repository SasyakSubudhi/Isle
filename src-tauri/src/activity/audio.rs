//! Output loudness for the audio-reactive wave. Reads only the peak meter of
//! the default speakers (one number, like the Windows volume mixer bar); no
//! audio is captured, recorded or stored. Runs only while the pill asks for it.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use windows::Win32::{
    Media::Audio::{
        eConsole, eRender, Endpoints::IAudioMeterInformation, IMMDeviceEnumerator,
        MMDeviceEnumerator,
    },
    System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED},
};

#[derive(Clone, Default)]
pub struct Meter(pub Arc<AtomicBool>);

fn open() -> Result<IAudioMeterInformation, String> {
    // SAFETY: COM is initialized on this thread before `open` is called.
    unsafe {
        let devices: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let device = devices
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|e| e.to_string())?;
        device
            .Activate::<IAudioMeterInformation>(CLSCTX_ALL, None)
            .map_err(|e| e.to_string())
    }
}

/// Peak to a 0..1 level that reads well on screen: quiet music still moves.
pub fn level(peak: f32) -> f32 {
    if !peak.is_finite() || peak <= 0.0 {
        return 0.0;
    }
    peak.min(1.0).sqrt()
}

pub fn start(app: AppHandle, meter: Meter) {
    std::thread::spawn(move || {
        // SAFETY: this worker owns its MTA; the meter interface never leaves it.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let mut device: Option<(IAudioMeterInformation, Instant)> = None;
        let mut last = -1.0f32;
        loop {
            if !meter.0.load(Ordering::Relaxed) {
                device = None;
                if last != 0.0 {
                    last = 0.0;
                    let _ = app.emit("audio://level", 0.0f32);
                }
                std::thread::sleep(Duration::from_millis(250));
                continue;
            }
            // Reopen every few seconds so a headphone or speaker switch is followed.
            if device
                .as_ref()
                .is_none_or(|(_, opened)| opened.elapsed() > Duration::from_secs(4))
            {
                device = open().ok().map(|m| (m, Instant::now()));
            }
            // SAFETY: the interface was created on this thread.
            let peak = device
                .as_ref()
                .and_then(|(m, _)| unsafe { m.GetPeakValue() }.ok())
                .unwrap_or(0.0);
            let next = (level(peak) * 100.0).round() / 100.0;
            if (next - last).abs() >= 0.02 {
                last = next;
                let _ = app.emit("audio://level", next);
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::level;
    #[test]
    fn levels_stay_in_range() {
        assert_eq!(level(0.0), 0.0);
        assert_eq!(level(f32::NAN), 0.0);
        assert_eq!(level(4.0), 1.0);
        assert!((level(0.25) - 0.5).abs() < 1e-6);
    }
}
