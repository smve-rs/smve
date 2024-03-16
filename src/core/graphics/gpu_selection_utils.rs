use log::info;
use wgpu::{Adapter, Backend, DeviceType};

const FEATURE_SCORE_WEIGHT: i8 = 3;
const TYPE_SCORE_WEIGHT: i8 = 2;
const BACKEND_SCORE_WEIGHT: i8 = 1;

pub type Score = i8;
pub type Index = usize;

pub fn get_best_gpu(mut adapters:  Vec<Adapter>) -> Adapter {
    let mut adapter_scores: Vec<(Index, Score)> = adapters.iter().enumerate()
        .map(|(i, adapter)| (i, get_gpu_score(adapter)))
        .collect();

    // Sort adapters based on score
    adapter_scores.sort_by(|a, b| b.1.cmp(&a.1));
    
    // Log scores
    for (i, score) in adapter_scores.iter() {
        info!("GPU: {}; Score: {}", adapters[*i].get_info().name, score);
    }

    // Choose the one with the highest score
    adapters.remove(adapter_scores[0].0)
}

pub fn get_gpu_score(adapter: &Adapter) -> Score {
    get_feature_score(adapter) * FEATURE_SCORE_WEIGHT + get_type_score(adapter) * TYPE_SCORE_WEIGHT + get_backend_score(adapter) * BACKEND_SCORE_WEIGHT
}

fn get_feature_score(_adapter: &Adapter) -> Score {
    0
}

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

fn get_type_score(adapter: &Adapter) -> Score {
    match adapter.get_info().device_type {
        DeviceType::Other => 0,
        DeviceType::Cpu => 1,
        // Integrated GPUs are ranked the same as Virtual GPUs
        DeviceType::IntegratedGpu => 2,
        DeviceType::VirtualGpu => 2,
        DeviceType::DiscreteGpu => 3,
    }
}