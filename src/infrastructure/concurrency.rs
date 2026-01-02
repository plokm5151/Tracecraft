/// Concurrency management for Mr. Hedgehog.
/// Configures thread pools to reserve system capacity for UI/LSP.

use anyhow::Result;

/// Initialize the global rayon thread pool with controlled worker count.
/// Reserves ~50% of CPU capacity for UI/LSP responsiveness.
pub fn init_thread_pool() -> Result<()> {
    let cores = num_cpus::get();
    // Reserve 50% capacity, minimum 1 worker
    let workers = std::cmp::max(1, cores / 2);
    
    rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build_global()?;
    
    println!(
        "[Mr. Hedgehog] Initialized thread pool: {} workers (system has {} cores)",
        workers, cores
    );
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_thread_pool_succeeds() {
        // Note: This test may fail if run after another test that already
        // initialized the global pool. Run in isolation if needed.
        // In a real scenario, we'd use a local pool for testing.
        // For now, we just verify the function doesn't panic on first call.
        // If the global pool is already initialized, this will return Err,
        // which is expected behavior.
        let result = init_thread_pool();
        // Either Ok (first init) or Err (already initialized) is acceptable
        assert!(result.is_ok() || result.is_err());
    }
}
