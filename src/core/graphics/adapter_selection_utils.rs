//! Utilities for selecting the best adapter for the current system
//! 
//! This module contains functions that help in selecting the best adapter for the current system based on the following criteria:
//! - Feature support (currently none)
//! - Type of adapter (CPU, Integrated GPU, Discrete GPU, etc.)
//! - Backend (Vulkan, DX12, Metal, etc.)

use log::info;
use wgpu::{Adapter, Backend, DeviceType};

/// Used to weight the importance of feature support
/// 
/// Feature support is the most important since it determines if the adapter can be used at all.
/// 
/// # Examples
/// You can use these weights to determine the feature score of an adapter like so:
/// ```rust
/// get_feature_score(adapter) * FEATURE_SCORE_WEIGHT
/// ```
const FEATURE_SCORE_WEIGHT: i8 = 3;

/// Used to weight the importance of the type of adapter
/// 
/// The type of adapter is the second most important since it determines the performance of the adapter.
/// 
/// # Examples
/// You can use these weights to determine the type score of an adapter like so:
/// ```rust
/// get_type_score(adapter) * TYPE_SCORE_WEIGHT
/// ```
const TYPE_SCORE_WEIGHT: i8 = 2;

/// Used to weight the importance of the backend
/// 
/// The backend is the least important since it only determines the API used.
/// 
/// # Examples
/// You can use these weights to determine the backend score of an adapter like so:
/// ```rust
/// get_backend_score(adapter) * BACKEND_SCORE_WEIGHT
/// ```
const BACKEND_SCORE_WEIGHT: i8 = 1;

/// Type alias for the score of an adapter
pub type Score = i8;

/// Type alias for the index of an adapter in a vector
pub type Index = usize;

/// Sorts the adapters based on their scores
/// 
/// # Arguments
/// * `adapters` - The list of adapters to choose from
/// 
/// # Returns
/// The best adapter based on the scores
/// 
/// # Notes
/// This function takes ownership of the adapters vector and returns ownership of the best adapter.
/// 
/// # Examples
/// This gets the best adapter from the adapters wgpu found:
/// ```rust
/// let adapters = instance.enumerate_adapters(Backends::all());
/// let adapter = get_best_adapter(adapters);
/// ```
pub fn get_best_adapter(adapters: Vec<Adapter>) -> Adapter {
    let mut adapters = filter_unwanted_adapters(adapters);

    let mut adapter_scores: Vec<(Index, Score)> = adapters
        .iter()
        .enumerate()
        .map(|(i, adapter)| (i, get_adapter_score(adapter)))
        .collect();

    // Sort adapters based on score
    adapter_scores.sort_by(|a, b| b.1.cmp(&a.1));

    // Log scores
    for (i, score) in adapter_scores.iter() {
        info!(
            "Adapter: {} with {:?}; Score: {}",
            adapters[*i].get_info().name,
            adapters[*i].get_info().backend,
            score
        );
    }

    // Choose the one with the highest score
    adapters.remove(adapter_scores[0].0)
}

/// Gets the score of an individual adapter based on the criteria
/// 
/// # Arguments
/// * `adapter` - The adapter to get the score of
/// 
/// # Returns
/// The score of the adapter
pub fn get_adapter_score(adapter: &Adapter) -> Score {
    get_feature_score(adapter) * FEATURE_SCORE_WEIGHT
        + get_type_score(adapter) * TYPE_SCORE_WEIGHT
        + get_backend_score(adapter) * BACKEND_SCORE_WEIGHT
}

/// Filters out any unwanted adapters
/// 
/// In this case, all CPU adapters are removed.
/// 
/// # Arguments
/// * `adapters` - The list of adapters to filter
/// 
/// # Returns
/// The list of adapters without any CPU adapters
/// 
/// # Notes
/// This function takes ownership of the adapters vector and returns ownership of the filtered vector.
/// 
/// # Examples
/// This filters out any CPU adapters from the adapters wgpu found:
/// ```rust
/// let adapters = instance.enumerate_adapters(Backends::all());
/// let adapters = filter_unwanted_adapters(adapters);
/// ```
fn filter_unwanted_adapters(adapters: Vec<Adapter>) -> Vec<Adapter> {
    adapters
        .into_iter()
        .filter(|adapter| {
            // Remove any CPU adapters
            adapter.get_info().device_type != DeviceType::Cpu
        })
        .collect()
}

/// Gets the unweighted score of an adapter based on feature support
/// 
/// Currently, it always returns 0 since there are no features to check for.
/// 
/// # Arguments
/// * `adapter` - The adapter to get the feature score of
/// 
/// # Returns
/// The unweighted feature score of the adapter
fn get_feature_score(_adapter: &Adapter) -> Score {
    0
}

/// Gets the unweighted score of an adapter based on the backend
/// 
/// # Arguments
/// * `adapter` - The adapter to get the backend score of
/// 
/// # Returns
/// The unweighted backend score of the adapter
/// or 0 when the backend is not supported on the current platform
fn get_backend_score(adapter: &Adapter) -> Score {
    let backend = adapter.get_info().backend;

    #[cfg(target_os = "windows")]
    match backend {
        Backend::Empty => 0,
        Backend::BrowserWebGpu => 0,
        Backend::Metal => 0,
        Backend::Gl => 1,
        Backend::Vulkan => 2,
        Backend::Dx12 => 3,
    }

    #[cfg(target_os = "macos")]
    match backend {
        Backend::Empty => 0,
        Backend::BrowserWebGpu => 0,
        Backend::Dx12 => 0,
        Backend::Gl => 1,
        Backend::Vulkan => 2,
        Backend::Metal => 3,
    }

    #[cfg(target_os = "linux")]
    match backend {
        Backend::Empty => 0,
        Backend::BrowserWebGpu => 0,
        Backend::Dx12 => 0,
        Backend::Metal => 0,
        Backend::Gl => 1,
        Backend::Vulkan => 2,
    }
}

/// Gets the unweighted score of an adapter based on the type of adapter
/// 
/// # Arguments
/// * `adapter` - The adapter to get the type score of
/// 
/// # Returns
/// The unweighted type score of the adapter
/// 
/// # Notes
/// The value for CPU adapters is arbitrary since they wouldn't go through anyway.
fn get_type_score(adapter: &Adapter) -> Score {
    match adapter.get_info().device_type {
        DeviceType::Other => 1,
        DeviceType::Cpu => -16, // CPU renderers wouldn't go through anyway so this value is arbitrary
        // Integrated GPUs are ranked the same as Virtual GPUs
        DeviceType::IntegratedGpu => 2,
        DeviceType::VirtualGpu => 2,
        DeviceType::DiscreteGpu => 3,
    }
}
