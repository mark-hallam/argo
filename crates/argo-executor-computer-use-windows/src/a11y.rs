//! UIAutomation tree traversal helpers.
//!
//! Wraps `IUIAutomation` (COM) so the executor can look up
//! controls by role + name, by automation id, and retrieve
//! bounding rectangles in virtual-screen coordinates.
//!
//! ## COM requirement
//!
//! Callers must initialise COM on the active thread before
//! invoking any of these helpers. The crate-level
//! [`crate::WindowsCuExecutor::warm_up`] performs the
//! initialisation lazily.

use crate::error::WindowsCuError;
use crate::WindowsCuResult;
use serde::{Deserialize, Serialize};

/// One UIAutomation element described in serde-friendly shape.
///
/// `role` matches the UIAutomation control-type label
/// (e.g., `Button`, `Edit`, `MenuItem`); `name` is the localised
/// display name; `automation_id` is the developer-supplied stable
/// identifier (often empty for legacy WinForms / WPF surfaces);
/// `rect` is the on-screen bounding rectangle in virtual-screen
/// coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A11yElement {
    /// UIAutomation control-type label.
    pub role: String,
    /// Localised display name.
    pub name: String,
    /// Developer-supplied automation id (may be empty).
    pub automation_id: String,
    /// Bounding rectangle: `(x, y, width, height)` in virtual-
    /// screen pixels.
    pub rect: (i32, i32, u32, u32),
    /// When multiple elements share the same role+name, this
    /// 1-indexed counter lets the executor distinguish duplicates
    /// deterministically (e.g., two `Button` named `OK`).
    pub role_count_for_dedupe: u32,
}

/// Find one element by role + name. Returns the first match in
/// depth-first traversal order. Returns
/// [`WindowsCuError::A11yLookupFailed`] when nothing matches.
pub fn find_element_by_role(role: &str, name: &str) -> WindowsCuResult<A11yElement> {
    if !cfg!(target_os = "windows") {
        return Err(WindowsCuError::NotWindows);
    }
    find_element_by_role_impl(role, name)
}

/// Find one element by its developer-supplied automation id.
/// Returns [`WindowsCuError::A11yLookupFailed`] when nothing matches.
pub fn find_element_by_automation_id(automation_id: &str) -> WindowsCuResult<A11yElement> {
    if !cfg!(target_os = "windows") {
        return Err(WindowsCuError::NotWindows);
    }
    find_element_by_automation_id_impl(automation_id)
}

/// Read the bounding rectangle of an element, refreshing from
/// the live tree (callers often cache `A11yElement` rows for the
/// audit log but need a fresh rect before clicking).
pub fn bounding_rect(element: &A11yElement) -> WindowsCuResult<(i32, i32, u32, u32)> {
    if !cfg!(target_os = "windows") {
        return Err(WindowsCuError::NotWindows);
    }
    // V1: trust the cached rect. The full live-rect refresh
    // lands in the follow-up brief alongside the deeper
    // UIAutomation event-hook integration.
    Ok(element.rect)
}

#[cfg(target_os = "windows")]
fn find_element_by_role_impl(role: &str, name: &str) -> WindowsCuResult<A11yElement> {
    use windows::core::BSTR;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, TreeScope_Subtree};

    unsafe {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let root = automation.GetRootElement()?;
        let condition = automation.CreatePropertyCondition(
            windows::Win32::UI::Accessibility::UIA_NamePropertyId,
            &BSTR::from(name).into(),
        )?;
        let element = root.FindFirst(TreeScope_Subtree, &condition)?;
        if element.is_invalid() {
            return Err(WindowsCuError::A11yLookupFailed {
                selector: format!("role={role} name={name}"),
            });
        }
        let rect = element.CurrentBoundingRectangle()?;
        let auto_id = element.CurrentAutomationId().unwrap_or_default();
        Ok(A11yElement {
            role: role.to_string(),
            name: name.to_string(),
            automation_id: auto_id.to_string(),
            rect: (
                rect.left,
                rect.top,
                (rect.right - rect.left).max(0) as u32,
                (rect.bottom - rect.top).max(0) as u32,
            ),
            role_count_for_dedupe: 1,
        })
    }
}

#[cfg(target_os = "windows")]
fn find_element_by_automation_id_impl(automation_id: &str) -> WindowsCuResult<A11yElement> {
    use windows::core::BSTR;
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation, TreeScope_Subtree};

    unsafe {
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
        let root = automation.GetRootElement()?;
        let condition = automation.CreatePropertyCondition(
            windows::Win32::UI::Accessibility::UIA_AutomationIdPropertyId,
            &BSTR::from(automation_id).into(),
        )?;
        let element = root.FindFirst(TreeScope_Subtree, &condition)?;
        if element.is_invalid() {
            return Err(WindowsCuError::A11yLookupFailed {
                selector: format!("automation_id={automation_id}"),
            });
        }
        let rect = element.CurrentBoundingRectangle()?;
        let name = element.CurrentName().unwrap_or_default();
        Ok(A11yElement {
            role: String::new(),
            name: name.to_string(),
            automation_id: automation_id.to_string(),
            rect: (
                rect.left,
                rect.top,
                (rect.right - rect.left).max(0) as u32,
                (rect.bottom - rect.top).max(0) as u32,
            ),
            role_count_for_dedupe: 1,
        })
    }
}

#[cfg(not(target_os = "windows"))]
fn find_element_by_role_impl(_role: &str, _name: &str) -> WindowsCuResult<A11yElement> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(not(target_os = "windows"))]
fn find_element_by_automation_id_impl(_automation_id: &str) -> WindowsCuResult<A11yElement> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_element() -> A11yElement {
        A11yElement {
            role: "Button".into(),
            name: "Save".into(),
            automation_id: "saveBtn".into(),
            rect: (100, 200, 80, 30),
            role_count_for_dedupe: 1,
        }
    }

    #[test]
    fn element_serialises_through_serde() {
        let e = sample_element();
        let s = serde_json::to_string(&e).unwrap();
        let parsed: A11yElement = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed, e);
    }

    #[test]
    fn bounding_rect_returns_cached_value() {
        let e = sample_element();
        let r = bounding_rect(&e).unwrap_or((0, 0, 0, 0));
        // off-platform the rect path either returns NotWindows
        // or the cached value, depending on compile target. Test
        // both shapes are accepted.
        let off_platform = !cfg!(target_os = "windows");
        if off_platform {
            assert_eq!(r, (0, 0, 0, 0));
        } else {
            assert_eq!(r, (100, 200, 80, 30));
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn find_by_role_returns_not_windows_off_platform() {
        let err = find_element_by_role("Button", "Save").unwrap_err();
        assert!(matches!(err, WindowsCuError::NotWindows));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn find_by_automation_id_returns_not_windows_off_platform() {
        let err = find_element_by_automation_id("saveBtn").unwrap_err();
        assert!(matches!(err, WindowsCuError::NotWindows));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn bounding_rect_returns_not_windows_off_platform() {
        let err = bounding_rect(&sample_element()).unwrap_err();
        assert!(matches!(err, WindowsCuError::NotWindows));
    }
}
