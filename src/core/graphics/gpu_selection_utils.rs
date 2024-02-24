use std::collections::HashMap;
use wgpu::{Adapter, Backend, DeviceType};

pub fn eliminate_gpu_on_unsupported_feats(adapters: Vec<Adapter>) -> Vec<Adapter> {
    adapters
}

pub fn select_gpu_on_type(mut adapters: Vec<Adapter>) -> Vec<Adapter> {
    get_highest_scored_adapters(&mut adapters, get_type_score)
}

pub fn select_gpu_on_backend(mut adapters: Vec<Adapter>) -> Vec<Adapter> {
    get_highest_scored_adapters(&mut adapters, get_backend_score)
}

fn get_backend_score(adapter: &Adapter) -> u8 {
    let backend = adapter.get_info().backend;

    #[cfg(target_os = "windows")]
    match backend {
        Backend::Empty => { 0 }
        Backend::BrowserWebGpu => { 0 }
        Backend::Metal => { 0 }
        Backend::Gl => { 1 }
        Backend::Vulkan => { 2 }
        Backend::Dx12 => { 3 }
    }

    #[cfg(target_os = "macos")]
    match backend {
        Backend::Empty => { 0 }
        Backend::BrowserWebGpu => { 0 }
        Backend::Dx12 => { 0 }
        Backend::Gl => { 1 }
        Backend::Vulkan => { 2 }
        Backend::Metal => { 3 }
    }

    #[cfg(target_os = "linux")]
    match backend {
        Backend::Empty => { 0 }
        Backend::BrowserWebGpu => { 0 }
        Backend::Dx12 => { 0 }
        Backend::Metal => { 0 }
        Backend::Gl => { 1 }
        Backend::Vulkan => { 2 }
    }
}

fn get_type_score(adapter: &Adapter) -> u8 {
    match adapter.get_info().device_type {
        DeviceType::Other => { 0 }
        DeviceType::Cpu => { 1 }
        // Integrated GPUs are ranked the same as Virtual GPUs
        DeviceType::IntegratedGpu => { 2 }
        DeviceType::VirtualGpu => { 2 }
        DeviceType::DiscreteGpu => { 3 }
    }
}

fn sort_scores(map: HashMap<usize, u8>) -> Vec<(usize, u8)> {
    let mut key_value_pairs: Vec<_> = map.into_iter().collect();
    key_value_pairs.sort_by(|a, b| {
        b.1.cmp(&a.1)
    });
    key_value_pairs
}

fn get_sorted_gpu_scores<F>(adapters: &[Adapter], score_function: F) -> Vec<(usize, u8)> where F: Fn(&Adapter) -> u8 {
    // Keep track of the scores based on the index of the adapter in the vector
    let mut gpu_scores: HashMap<usize, u8> = HashMap::new();

    // Give adapters scores based on their scores
    for (i, adapter) in adapters.iter().enumerate() {
        gpu_scores.insert(i, score_function(adapter));
    }

    sort_scores(gpu_scores)
}

fn get_highest_scored_adapters<F>(adapters: &mut Vec<Adapter>, score_function: F) -> Vec<Adapter> where F: Fn(&Adapter) -> u8 {
    let sorted = get_sorted_gpu_scores(adapters, score_function);

    // Add the GPUs from the top score into a results vector
    let mut results = vec![];
    for pair in sorted.iter() {
        // If the score is different from the top, stop iterating
        if pair.1 != sorted.first().unwrap().1 {
            break;
        }

        results.push(adapters.remove(pair.0));
    }

    results
}
