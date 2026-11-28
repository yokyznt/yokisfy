use std::ffi::c_void;
use windows::core::{interface, IUnknown, IUnknown_Vtbl, Interface, GUID, HRESULT, HSTRING};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{eConsole, eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};
use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
use windows::Win32::System::WinRT::RoGetActivationFactory;

#[interface("ab3d4648-e242-459f-b02f-541c70306324")]
unsafe trait IPolicy: IUnknown {
    unsafe fn i0(&self) -> HRESULT;
    unsafe fn i1(&self) -> HRESULT;
    unsafe fn i2(&self) -> HRESULT;
    unsafe fn m0(&self) -> HRESULT;
    unsafe fn m1(&self) -> HRESULT;
    unsafe fn m2(&self) -> HRESULT;
    unsafe fn m3(&self) -> HRESULT;
    unsafe fn m4(&self) -> HRESULT;
    unsafe fn m5(&self) -> HRESULT;
    unsafe fn m6(&self) -> HRESULT;
    unsafe fn m7(&self) -> HRESULT;
    unsafe fn m8(&self) -> HRESULT;
    unsafe fn m9(&self) -> HRESULT;
    unsafe fn m10(&self) -> HRESULT;
    unsafe fn m11(&self) -> HRESULT;
    unsafe fn m12(&self) -> HRESULT;
    unsafe fn m13(&self) -> HRESULT;
    unsafe fn m14(&self) -> HRESULT;
    unsafe fn m15(&self) -> HRESULT;
    unsafe fn m16(&self) -> HRESULT;
    unsafe fn m17(&self) -> HRESULT;
    unsafe fn m18(&self) -> HRESULT;
    unsafe fn set(&self, pid: u32, flow: i32, role: i32, id: *mut c_void) -> HRESULT;
}

#[derive(serde::Serialize, Clone)]
pub struct Output {
    pub id: String,
    pub name: String,
    pub default: bool,
}

fn com<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        f()
    })
    .join()
    .unwrap()
}

pub fn outputs() -> Vec<Output> {
    com(|| unsafe {
        let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return vec![] };
        let def = en.GetDefaultAudioEndpoint(eRender, eMultimedia).ok().and_then(|d| d.GetId().ok()).map(|p| {
            let s = p.to_string().unwrap_or_default();
            CoTaskMemFree(Some(p.0 as *const c_void));
            s
        });
        let Ok(col) = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else { return vec![] };
        let n = col.GetCount().unwrap_or(0);
        let mut out = Vec::new();
        for i in 0..n {
            let Ok(d) = col.Item(i) else { continue };
            let Ok(p) = d.GetId() else { continue };
            let id = p.to_string().unwrap_or_default();
            CoTaskMemFree(Some(p.0 as *const c_void));
            let name = d.OpenPropertyStore(STGM_READ).ok().and_then(|st| st.GetValue(&PKEY_Device_FriendlyName).ok()).map(|v| v.to_string()).unwrap_or_default();
            if !name.is_empty() {
                out.push(Output { default: def.as_deref() == Some(id.as_str()), id, name });
            }
        }
        out
    })
}

fn family() -> Vec<u32> {
    unsafe {
        let me = std::process::id();
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return vec![me] };
        let mut all: Vec<(u32, u32)> = Vec::new();
        let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snap, &mut e).is_ok();
        while ok {
            all.push((e.th32ProcessID, e.th32ParentProcessID));
            ok = Process32NextW(snap, &mut e).is_ok();
        }
        let _ = windows::Win32::Foundation::CloseHandle(snap);
        let mut fam = vec![me];
        let mut i = 0;
        while i < fam.len() {
            let p = fam[i];
            for (c, par) in &all {
                if *par == p && !fam.contains(c) {
                    fam.push(*c);
                }
            }
            i += 1;
        }
        fam
    }
}

pub fn route(id: Option<String>) -> usize {
    com(move || unsafe {
        let cls = HSTRING::from("Windows.Media.Internal.AudioPolicyConfig");
        let f: IPolicy = match RoGetActivationFactory::<IUnknown>(&cls).and_then(|u| u.cast::<IPolicy>()) {
            Ok(f) => f,
            Err(_) => {
                let mut raw: *mut c_void = std::ptr::null_mut();
                let iid = GUID::from_u128(0x2a59116d_6c4f_45e0_a74f_707e3fef9258);
                match RoGetActivationFactory::<IUnknown>(&cls) {
                    Ok(u) if u.query(&iid, &mut raw).is_ok() && !raw.is_null() => IPolicy::from_raw(raw),
                    _ => return 0,
                }
            }
        };
        let h = id.map(|x| HSTRING::from(format!("\\\\?\\SWD#MMDEVAPI#{x}#{{e6327cad-dcec-4949-ae8a-991e976a79d2}}")));
        let ptr: *mut c_void = h.as_ref().map(|h| std::mem::transmute_copy::<HSTRING, *mut c_void>(h)).unwrap_or(std::ptr::null_mut());
        let mut n = 0;
        let fam = family();
        for pid in &fam {
            let q = if *pid == std::process::id() { std::ptr::null_mut() } else { ptr };
            let a = f.set(*pid, eRender.0, eConsole.0, q);
            let b = f.set(*pid, eRender.0, eMultimedia.0, q);
            if a.is_ok() && b.is_ok() && *pid != std::process::id() {
                n += 1;
            }
        }
        n
    })
}
