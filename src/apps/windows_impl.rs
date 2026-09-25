#![allow(non_snake_case, dead_code)]
//! WASAPI session enumeration plus the internal `AudioPolicyConfig` interface for per-app routing.
//! The interface layout follows EarTrumpet's reverse engineering: IInspectable's three methods,
//! then 19 unused slots, then the three we call. Only the interface ID changed in Windows 10 21H2.
//! Source: https://github.com/File-New-Project/EarTrumpet/blob/master/EarTrumpet/Interop/MMDeviceAPI/IAudioPolicyConfigFactoryVariantFor21H2.cs
//! and .../IAudioPolicyConfigFactoryVariantForDownlevel.cs; device-id format from
//! .../EarTrumpet/DataModel/WindowsAudio/Internal/AudioPolicyConfigService.cs.
//! This file is only type-checked on macOS; the first run on a real Windows PC is the test.

use super::{AppSession, Endpoint, Flow};
use anyhow::{anyhow, Context, Result};
use windows::core::{IUnknown, IUnknown_Vtbl, Interface, GUID, HRESULT, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{CloseHandle, HANDLE, S_OK};
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow, ERole, IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED, STGM_READ};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::System::WinRT::RoGetActivationFactory;

const POLICY_CLASS: &str = "Windows.Media.Internal.AudioPolicyConfig";
/// The class behind the Sound control panel's "Set as default". Undocumented but unchanged since
/// Vista; every tray mixer uses it. Layout from PolicyConfig.h: IUnknown, then GetMixFormat,
/// GetDeviceFormat, ResetDeviceFormat, SetDeviceFormat, GetProcessingPeriod, SetProcessingPeriod,
/// GetShareMode, SetShareMode, GetPropertyValue, SetPropertyValue, SetDefaultEndpoint.
const POLICY_CONFIG_CLASS: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
const MMDEVAPI_TOKEN: &str = r"\\?\SWD#MMDEVAPI#";
const DEVINTERFACE_AUDIO_RENDER: &str = "#{e6327cad-dcec-4949-ae8a-991e976a79d2}";
const DEVINTERFACE_AUDIO_CAPTURE: &str = "#{2eef81be-33fa-4800-9670-1cd474972c3f}";

#[windows::core::interface("ab3d4648-e242-459f-b02f-541c70306324")]
unsafe trait IAudioPolicyConfigFactory: IUnknown {
    // IInspectable: GetIids, GetRuntimeClassName, GetTrustLevel.
    unsafe fn inspectable0(&self) -> HRESULT;
    unsafe fn inspectable1(&self) -> HRESULT;
    unsafe fn inspectable2(&self) -> HRESULT;
    unsafe fn slot0(&self) -> HRESULT;
    unsafe fn slot1(&self) -> HRESULT;
    unsafe fn slot2(&self) -> HRESULT;
    unsafe fn slot3(&self) -> HRESULT;
    unsafe fn slot4(&self) -> HRESULT;
    unsafe fn slot5(&self) -> HRESULT;
    unsafe fn slot6(&self) -> HRESULT;
    unsafe fn slot7(&self) -> HRESULT;
    unsafe fn slot8(&self) -> HRESULT;
    unsafe fn slot9(&self) -> HRESULT;
    unsafe fn slot10(&self) -> HRESULT;
    unsafe fn slot11(&self) -> HRESULT;
    unsafe fn slot12(&self) -> HRESULT;
    unsafe fn slot13(&self) -> HRESULT;
    unsafe fn slot14(&self) -> HRESULT;
    unsafe fn slot15(&self) -> HRESULT;
    unsafe fn slot16(&self) -> HRESULT;
    unsafe fn slot17(&self) -> HRESULT;
    unsafe fn slot18(&self) -> HRESULT;
    unsafe fn SetPersistedDefaultAudioEndpoint(&self, process_id: u32, flow: EDataFlow, role: ERole, device_id: std::mem::MaybeUninit<HSTRING>) -> HRESULT;
    unsafe fn GetPersistedDefaultAudioEndpoint(&self, process_id: u32, flow: EDataFlow, role: ERole, device_id: *mut std::mem::MaybeUninit<HSTRING>) -> HRESULT;
    unsafe fn ClearAllPersistedApplicationDefaultEndpoints(&self) -> HRESULT;
}

#[windows::core::interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    unsafe fn slot0(&self) -> HRESULT;
    unsafe fn slot1(&self) -> HRESULT;
    unsafe fn slot2(&self) -> HRESULT;
    unsafe fn slot3(&self) -> HRESULT;
    unsafe fn slot4(&self) -> HRESULT;
    unsafe fn slot5(&self) -> HRESULT;
    unsafe fn slot6(&self) -> HRESULT;
    unsafe fn slot7(&self) -> HRESULT;
    unsafe fn slot8(&self) -> HRESULT;
    unsafe fn slot9(&self) -> HRESULT;
    unsafe fn SetDefaultEndpoint(&self, device_id: PCWSTR, role: ERole) -> HRESULT;
}

/// Same layout, the interface ID Windows used before 21H2.
#[windows::core::interface("2a59116d-6c4f-45e0-a74f-707e3fef9258")]
unsafe trait IAudioPolicyConfigFactoryDownlevel: IUnknown {
    // IInspectable: GetIids, GetRuntimeClassName, GetTrustLevel.
    unsafe fn inspectable0(&self) -> HRESULT;
    unsafe fn inspectable1(&self) -> HRESULT;
    unsafe fn inspectable2(&self) -> HRESULT;
    unsafe fn slot0(&self) -> HRESULT;
    unsafe fn slot1(&self) -> HRESULT;
    unsafe fn slot2(&self) -> HRESULT;
    unsafe fn slot3(&self) -> HRESULT;
    unsafe fn slot4(&self) -> HRESULT;
    unsafe fn slot5(&self) -> HRESULT;
    unsafe fn slot6(&self) -> HRESULT;
    unsafe fn slot7(&self) -> HRESULT;
    unsafe fn slot8(&self) -> HRESULT;
    unsafe fn slot9(&self) -> HRESULT;
    unsafe fn slot10(&self) -> HRESULT;
    unsafe fn slot11(&self) -> HRESULT;
    unsafe fn slot12(&self) -> HRESULT;
    unsafe fn slot13(&self) -> HRESULT;
    unsafe fn slot14(&self) -> HRESULT;
    unsafe fn slot15(&self) -> HRESULT;
    unsafe fn slot16(&self) -> HRESULT;
    unsafe fn slot17(&self) -> HRESULT;
    unsafe fn slot18(&self) -> HRESULT;
    unsafe fn SetPersistedDefaultAudioEndpoint(&self, process_id: u32, flow: EDataFlow, role: ERole, device_id: std::mem::MaybeUninit<HSTRING>) -> HRESULT;
    unsafe fn GetPersistedDefaultAudioEndpoint(&self, process_id: u32, flow: EDataFlow, role: ERole, device_id: *mut std::mem::MaybeUninit<HSTRING>) -> HRESULT;
    unsafe fn ClearAllPersistedApplicationDefaultEndpoints(&self) -> HRESULT;
}

fn ensure_com() {
    // Safety: initialising COM for the UI thread. This is never unwound on purpose: the thread
    // lives as long as the app, and eframe may already have initialised COM with its own
    // apartment, in which case this call is a harmless no-op that returns S_FALSE or RPC_E_CHANGED_MODE.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

fn data_flow(flow: Flow) -> EDataFlow {
    match flow {
        Flow::Playback => eRender,
        Flow::Capture => eCapture,
    }
}

fn device_name(device: &IMMDevice) -> Result<String> {
    // Safety: standard property-store read on a live device.
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ)?;
        let value = store.GetValue(&PKEY_Device_FriendlyName)?;
        Ok(value.to_string())
    }
}

fn device_id(device: &IMMDevice) -> Result<String> {
    // Safety: GetId returns a CoTaskMem string that we copy and free.
    unsafe {
        let id: PWSTR = device.GetId()?;
        let text = id.to_string()?;
        CoTaskMemFree(Some(id.0 as *const _));
        Ok(text)
    }
}

fn process_name(pid: u32) -> Option<String> {
    // Safety: limited-information handle, closed before returning.
    unsafe {
        let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(handle);
        if !ok {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        let file = path.rsplit(['\\', '/']).next()?;
        Some(file.trim_end_matches(".exe").trim_end_matches(".EXE").to_string())
    }
}

fn devices(flow: Flow) -> Result<Vec<IMMDevice>> {
    ensure_com();
    // Safety: plain COM enumeration.
    unsafe {
        let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let collection = enumerator.EnumAudioEndpoints(data_flow(flow), DEVICE_STATE_ACTIVE)?;
        (0..collection.GetCount()?).map(|i| collection.Item(i).map_err(Into::into)).collect()
    }
}

/// One pass over every active device: its endpoint entry and the sessions playing on it.
pub fn snapshot() -> Result<(Vec<AppSession>, Vec<Endpoint>)> {
    let mut sessions: Vec<AppSession> = Vec::new();
    let mut endpoints = Vec::new();
    for flow in [Flow::Playback, Flow::Capture] {
        for device in devices(flow)? {
            let name = device_name(&device)?;
            endpoints.push(Endpoint { id: device_id(&device)?, name: name.clone(), flow });
            // Safety: session enumeration on a live device; every object is released on drop.
            unsafe {
                let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
                let enumerator = manager.GetSessionEnumerator()?;
                for i in 0..enumerator.GetCount()? {
                    let control = enumerator.GetSession(i)?;
                    let control2: IAudioSessionControl2 = control.cast()?;
                    // S_OK means "yes, system sounds"; S_FALSE (also non-negative) means an ordinary app.
                    if control2.IsSystemSoundsSession() == S_OK {
                        continue;
                    }
                    let pid = control2.GetProcessId()?;
                    if pid == 0 {
                        continue;
                    }
                    let app = process_name(pid).unwrap_or_else(|| format!("pid {pid}"));
                    if super::is_system_process(&app) || sessions.iter().any(|s| s.pid == pid && s.flow == flow) {
                        continue;
                    }
                    sessions.push(AppSession { pid, name: app, flow, device_name: name.clone() });
                }
            }
        }
    }
    sessions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok((sessions, endpoints))
}

fn policy_device_string(flow: Flow, endpoint_id: &str) -> String {
    let suffix = match flow {
        Flow::Playback => DEVINTERFACE_AUDIO_RENDER,
        Flow::Capture => DEVINTERFACE_AUDIO_CAPTURE,
    };
    format!("{MMDEVAPI_TOKEN}{endpoint_id}{suffix}")
}

pub fn set_app_device(pid: u32, flow: Flow, endpoint_id: Option<&str>) -> Result<()> {
    ensure_com();
    let device = match endpoint_id {
        Some(id) => HSTRING::from(policy_device_string(flow, id)),
        None => HSTRING::new(),
    };
    let class = HSTRING::from(POLICY_CLASS);
    // Safety: activation of an internal WinRT class; both interface layouts are identical apart
    // from the ID, so whichever one Windows hands out is called the same way.
    unsafe {
        let call = |role: ERole| -> Result<()> {
            // Bitwise copy of the handle: the callee borrows it, the owner below frees it.
            let arg: std::mem::MaybeUninit<HSTRING> = std::mem::transmute_copy(&device);
            let hr = match RoGetActivationFactory::<IAudioPolicyConfigFactory>(&class) {
                Ok(factory) => factory.SetPersistedDefaultAudioEndpoint(pid, data_flow(flow), role, arg),
                Err(_) => {
                    let factory = RoGetActivationFactory::<IAudioPolicyConfigFactoryDownlevel>(&class)
                        .context("AudioPolicyConfig is not available on this Windows build")?;
                    factory.SetPersistedDefaultAudioEndpoint(pid, data_flow(flow), role, arg)
                }
            };
            hr.ok().map_err(|e| anyhow!("SetPersistedDefaultAudioEndpoint failed: {e}"))
        };
        call(eMultimedia)?;
        call(eConsole)?;
    }
    Ok(())
}

/// Makes the active device called `name` the Windows default for `flow`, for every role: what
/// apps play to or record from unless they chose otherwise, and what calls use.
pub fn set_default_device(flow: Flow, name: &str) -> Result<()> {
    ensure_com();
    let device = devices(flow)?
        .into_iter()
        .find(|d| device_name(d).ok().as_deref() == Some(name))
        .ok_or_else(|| anyhow!("\"{name}\" is not an active device"))?;
    let id: Vec<u16> = device_id(&device)?.encode_utf16().chain(std::iter::once(0)).collect();
    // Safety: the id buffer outlives every call, and the class is the one the Sound panel uses.
    unsafe {
        let policy: IPolicyConfig = CoCreateInstance(&POLICY_CONFIG_CLASS, None, CLSCTX_ALL).context("PolicyConfig is not available")?;
        for role in [eConsole, eMultimedia, eCommunications] {
            policy.SetDefaultEndpoint(PCWSTR(id.as_ptr()), role).ok().map_err(|e| anyhow!("SetDefaultEndpoint failed: {e}"))?;
        }
    }
    Ok(())
}
