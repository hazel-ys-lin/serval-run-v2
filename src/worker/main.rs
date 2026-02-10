mod executor;
mod result_handler;

use std::sync::Arc;

use tokio::signal;
use tokio::sync::watch;

// Import from the main crate
use serval_run::config::Config;
use serval_run::state::AppState;

use executor::JobExecutor;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting ServalRun Worker...");

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");

    // Initialize application state
    tracing::info!("Connecting to databases...");
    let state = AppState::new(config)
        .await
        .expect("Failed to initialize application state");
    let state = Arc::new(state);
    tracing::info!("Database connections established");

    // Set up graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Spawn shutdown signal handler
    tokio::spawn(async move {
        shutdown_signal().await;
        tracing::info!("Shutdown signal received, stopping worker...");
        let _ = shutdown_tx.send(true);
    });

    // Create job executor
    let executor = JobExecutor::new(state.clone());

    // Worker loop
    tracing::info!("Worker started, waiting for jobs...");
    loop {
        // Check for shutdown
        if *shutdown_rx.borrow() {
            tracing::info!("Shutdown requested, exiting worker loop");
            break;
        }

        // Try to dequeue a job (with 5 second timeout)
        match state.job_queue.dequeue(5).await {
            Ok(Some(job)) => {
                let job_id = job.id;
                let job_type = job.job_type.as_str().to_string();
                tracing::info!(job_id = %job_id, job_type = %job_type, "Processing job");

                // Update status to Running
                if let Err(e) = state
                    .job_queue
                    .update_status(job_id, serval_run::queue::JobStatus::Running)
                    .await
                {
                    tracing::error!(job_id = %job_id, error = %e, "Failed to update job status");
                    continue;
                }

                // Execute the job
                match executor.execute(job).await {
                    Ok(result) => {
                        tracing::info!(
                            job_id = %job_id,
                            passed = result.passed,
                            failed = result.failed,
                            "Job completed successfully"
                        );
                        if let Err(e) = state.job_queue.complete_job(job_id, result).await {
                            tracing::error!(job_id = %job_id, error = %e, "Failed to mark job as complete");
                        }
                    }
                    Err(e) => {
                        let is_retryable = is_retryable_error(&e);
                        tracing::error!(
                            job_id = %job_id,
                            error = %e,
                            retryable = is_retryable,
                            "Job failed"
                        );
                        if let Err(e) = state
                            .job_queue
                            .fail_job(job_id, e.to_string(), is_retryable)
                            .await
                        {
                            tracing::error!(job_id = %job_id, error = %e, "Failed to mark job as failed");
                        }
                    }
                }
            }
            Ok(None) => {
                // No job available, continue loop (dequeue already waited)
            }
            Err(e) => {
                tracing::error!(error = %e, "Error dequeuing job");
                // Brief sleep on error to prevent tight loop
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }

    tracing::info!("Worker shutdown complete");
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Determine if an error is retryable
fn is_retryable_error(error: &serval_run::error::AppError) -> bool {
    match error {
        // Network errors are usually retryable
        serval_run::error::AppError::Internal(msg) => {
            msg.contains("timeout") || msg.contains("connection") || msg.contains("network")
        }
        // Queue errors might be retryable
        serval_run::error::AppError::Queue(msg) => {
            msg.contains("timeout") || msg.contains("connection")
        }
        // Validation errors are not retryable
        serval_run::error::AppError::Validation(_) => false,
        // Auth errors are not retryable
        serval_run::error::AppError::Unauthorized => false,
        // Not found errors are not retryable
        serval_run::error::AppError::NotFound(_) => false,
        // Database errors might be retryable (transient failures)
        serval_run::error::AppError::Database(_) => true,
        // Conflict errors are not retryable
        serval_run::error::AppError::Conflict(_) => false,
        // Auth token errors are not retryable
        serval_run::error::AppError::InvalidCredentials
        | serval_run::error::AppError::InvalidToken
        | serval_run::error::AppError::TokenExpired => false,
    }
}
