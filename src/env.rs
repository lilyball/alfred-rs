//! Accessors for workflow environment variables
//!
//! See https://www.alfredapp.com/help/workflows/script-environment-variables/
//! for more info.

use std::env;
use std::path::PathBuf;

/// Returns the location of the Alfred.alfredpreferences.
///
/// Example output: `"/Users/Crayons/Dropbox/Alfred/Alfred.alfredpreferences"`
pub fn preferences() -> Option<PathBuf> {
    env::var("alfred_preferences").ok().map(PathBuf::from)
}

/// Returns the location of local (Mac-specific) preferences.
///
/// Example output: `"/Users/Crayons/Dropbox/Alfred/Alfred.alfredpreferences/preferences/local/adbd4f66bc3ae8493832af61a41ee609b20d8705"`
pub fn local_preferences() -> Option<PathBuf> {
    match (preferences(),env::var("alfred_preferences_localhash")) {
        (Some(mut prefs),Ok(hash)) => {
            prefs.extend(["preferences","local",&hash].iter());
            Some(prefs)
        }
        _ => None
    }
}

/// Returns the current Alfred theme.
///
/// Example output: `"alfred.theme.yosemite"`
pub fn theme() -> Option<String> {
    env::var("alfred_theme").ok()
}

/// Returns the color of the theme background.
///
/// Example output: `"rgba(255,255,255,0.98)"`
// TODO: can we parse this?
// Is this always in rgba(), or can it be any web color?
pub fn theme_background_str() -> Option<String> {
    env::var("alfred_theme_background").ok()
}

/// Returns the color of the theme's selected item background.
///
/// Example output: `"rgba(255,255,255,0.98)"`
// TODO: see `theme_background_str()`
pub fn theme_selection_background_str() -> Option<String> {
    env::var("alfred_theme_selection_background").ok()
}

/// The subtext mode in the Appearance preferences.
#[derive(Copy,Clone,Debug,PartialEq,Eq,Hash)]
pub enum Subtext {
    /// Always show subtext.
    Always,
    /// Only show subtext for alternative actions.
    AlternativeActions,
    /// Only show subtext for the selected result.
    SelectedResult,
    /// Never show subtext.
    Never
}

/// Returns the subtext mode the user has selected in the Appearance preferences.
pub fn theme_subtext() -> Option<Subtext> {
    match env::var("alfred_theme_subtext").as_ref().map(|s| s.as_ref()) {
        Ok("0") => Some(Subtext::Always),
        Ok("1") => Some(Subtext::AlternativeActions),
        Ok("2") => Some(Subtext::SelectedResult),
        Ok("3") => Some(Subtext::Never),
        _ => None
    }
}

/// Returns the version of Alfred.
///
/// Example output: `"3.2.1"`
pub fn version() -> Option<String> {
    env::var("alfred_version").ok()
}

/// Returns the build of Alfred.
///
/// Example output: `768`
pub fn version_build() -> Option<i32> {
    env::var("alfred_version_build").ok().and_then(|s| s.parse().ok())
}

/// Returns the bundle ID of the current running workflow.
///
/// Example output: `"com.alfredapp.david.googlesuggest"`
pub fn workflow_bundle_id() -> Option<String> {
    env::var("alfred_workflow_bundleid").ok()
}

/// Returns the recommended location for volatile workflow data.
/// Will only be populated if the workflow has a bundle identifier set.
///
/// Example output: `"/Users/Crayons/Library/Caches/com.runningwithcrayons.Alfred-2/Workflow Data/com.alfredapp.david.googlesuggest"`
pub fn workflow_cache() -> Option<PathBuf> {
    env::var("alfred_workflow_cache").ok().map(PathBuf::from)
}

/// Returns the recommended location for non-volatile workflow data.
/// Will only be populated if the workflow has a bundle identifier set.
///
/// Example output: `"/Users/Crayons/Library/Application Support/Alfred 2/Workflow Data/com.alfredapp.david.googlesuggest"`
pub fn workflow_data() -> Option<PathBuf> {
    env::var("alfred_workflow_data").ok().map(PathBuf::from)
}

/// Returns the name of the currently running workflow.
///
/// Example output: `"Google Suggest"`
pub fn workflow_name() -> Option<String> {
    env::var("alfred_workflow_name").ok()
}

/// Returns the unique ID of the currently running workflow.
///
/// Example output: `"user.workflow.B0AC54EC-601C-479A-9428-01F9FD732959"`
pub fn workflow_uid() -> Option<String> {
    env::var("alfred_workflow_uid").ok()
}

/// Returns the version of the currently running workflow.
pub fn workflow_version() -> Option<String> {
    env::var("alfred_workflow_version").ok()
}

/// Returns `true` if the user has the debug panel open for the workflow.
pub fn is_debug() -> bool {
    match env::var("alfred_debug") {
        Ok(val) => val == "1",
        _ => false
    }
}
