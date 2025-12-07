//! Integration tests for security features: timeouts and cancellation
//!
//! These tests verify that the security mechanisms work correctly and
//! are not false positives.

use anthropic_agent_sdk::hooks::{HookManager, HookMatcherBuilder};
use anthropic_agent_sdk::types::{HookContext, HookEvent, HookOutput, ToolPermissionContext};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

// ============================================================================
// Timeout Tests - Verify timeouts actually prevent blocking
// ============================================================================

#[tokio::test]
async fn test_timeout_actually_prevents_blocking() {
    // This test verifies the timeout is REAL, not a false positive.
    // Without timeout, this would take 10 seconds. With timeout, ~100ms.

    let mut manager = HookManager::new();

    let slow_hook = HookManager::callback(|_data, _tool, _ctx| async move {
        // This would block for 10 seconds without timeout
        tokio::time::sleep(Duration::from_secs(10)).await;
        Ok(HookOutput {
            system_message: Some("should never see this".to_string()),
            ..Default::default()
        })
    });

    let matcher = HookMatcherBuilder::new(Some("*"))
        .timeout(Duration::from_millis(100))
        .add_hook(slow_hook)
        .build();
    manager.register_for_event(HookEvent::PreToolUse, matcher);

    let start = Instant::now();
    let result = manager
        .invoke(
            HookEvent::PreToolUse,
            serde_json::json!({}),
            Some("test".to_string()),
            HookContext::default(),
        )
        .await;
    let elapsed = start.elapsed();

    // CRITICAL: If timeout didn't work, this would be ~10s
    assert!(
        elapsed < Duration::from_millis(500),
        "Timeout failed! Took {:?} (expected < 500ms)",
        elapsed
    );

    // Verify we got a result (timeout returns default, doesn't error)
    assert!(result.is_ok());

    // Verify timed-out hook returned default (no message)
    let output = result.unwrap();
    assert!(
        output.system_message.is_none(),
        "Timed out hook should return default output"
    );
}

#[tokio::test]
async fn test_timeout_boundary_just_under() {
    // Hook that completes just before timeout should succeed
    let mut manager = HookManager::new();

    let hook = HookManager::callback(|_data, _tool, _ctx| async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(HookOutput {
            system_message: Some("completed".to_string()),
            ..Default::default()
        })
    });

    let matcher = HookMatcherBuilder::new(Some("*"))
        .timeout(Duration::from_millis(200))
        .add_hook(hook)
        .build();
    manager.register_for_event(HookEvent::PreToolUse, matcher);

    let result = manager
        .invoke(
            HookEvent::PreToolUse,
            serde_json::json!({}),
            Some("test".to_string()),
            HookContext::default(),
        )
        .await
        .unwrap();

    // Should complete, not timeout
    assert_eq!(result.system_message, Some("completed".to_string()));
}

#[tokio::test]
async fn test_timeout_boundary_just_over() {
    // Hook that completes just after timeout should be cancelled
    let mut manager = HookManager::new();

    let hook = HookManager::callback(|_data, _tool, _ctx| async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(HookOutput {
            system_message: Some("should not see".to_string()),
            ..Default::default()
        })
    });

    let matcher = HookMatcherBuilder::new(Some("*"))
        .timeout(Duration::from_millis(50))
        .add_hook(hook)
        .build();
    manager.register_for_event(HookEvent::PreToolUse, matcher);

    let start = Instant::now();
    let result = manager
        .invoke(
            HookEvent::PreToolUse,
            serde_json::json!({}),
            Some("test".to_string()),
            HookContext::default(),
        )
        .await
        .unwrap();
    let elapsed = start.elapsed();

    // Should timeout quickly, not wait 200ms
    assert!(
        elapsed < Duration::from_millis(150),
        "Should have timed out at 50ms, took {:?}",
        elapsed
    );
    assert!(result.system_message.is_none());
}

#[tokio::test]
async fn test_multiple_hooks_timeout_independently() {
    // Each hook in a matcher should be timed independently
    let mut manager = HookManager::new();
    let call_count = Arc::new(AtomicU32::new(0));

    // First hook: fast
    let count1 = call_count.clone();
    let fast_hook = HookManager::callback(move |_data, _tool, _ctx| {
        let count = count1.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(HookOutput::default())
        }
    });

    // Second hook: slow (will timeout)
    let count2 = call_count.clone();
    let slow_hook = HookManager::callback(move |_data, _tool, _ctx| {
        let count = count2.clone();
        async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            count.fetch_add(1, Ordering::SeqCst);
            Ok(HookOutput::default())
        }
    });

    // Third hook: fast (should still run after slow times out)
    let count3 = call_count.clone();
    let fast_hook2 = HookManager::callback(move |_data, _tool, _ctx| {
        let count = count3.clone();
        async move {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(HookOutput::default())
        }
    });

    let matcher = HookMatcherBuilder::new(Some("*"))
        .timeout(Duration::from_millis(100))
        .add_hook(fast_hook)
        .add_hook(slow_hook)
        .add_hook(fast_hook2)
        .build();
    manager.register_for_event(HookEvent::PreToolUse, matcher);

    let start = Instant::now();
    let _ = manager
        .invoke(
            HookEvent::PreToolUse,
            serde_json::json!({}),
            Some("test".to_string()),
            HookContext::default(),
        )
        .await;
    let elapsed = start.elapsed();

    // Should complete in ~200ms (100ms timeout for slow hook + fast hooks)
    assert!(
        elapsed < Duration::from_secs(1),
        "Multiple hooks should not block: {:?}",
        elapsed
    );

    // Fast hooks ran, slow hook was cancelled mid-execution
    // So we expect 2 completions (first fast + third fast), slow was cancelled
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

// ============================================================================
// Cancellation Token Tests - Verify tokens actually work
// ============================================================================

#[tokio::test]
async fn test_cancellation_token_propagates_to_hook() {
    let mut manager = HookManager::new();
    let received_token = Arc::new(AtomicU32::new(0));
    let received_clone = received_token.clone();

    let hook = HookManager::callback(move |_data, _tool, ctx| {
        let received = received_clone.clone();
        async move {
            if ctx.cancellation_token.is_some() {
                received.fetch_add(1, Ordering::SeqCst);
            }
            Ok(HookOutput::default())
        }
    });

    let matcher = HookMatcherBuilder::new(Some("*")).add_hook(hook).build();
    manager.register_for_event(HookEvent::PreToolUse, matcher);

    // Invoke with a real cancellation token
    let token = CancellationToken::new();
    let ctx = HookContext::new(Some("session".to_string()), None, Some(token));

    let _ = manager
        .invoke(
            HookEvent::PreToolUse,
            serde_json::json!({}),
            Some("test".to_string()),
            ctx,
        )
        .await;

    assert_eq!(
        received_token.load(Ordering::SeqCst),
        1,
        "Hook should receive cancellation token"
    );
}

#[tokio::test]
async fn test_cancellation_token_reflects_cancelled_state() {
    let token = CancellationToken::new();
    let ctx = HookContext::new(None, None, Some(token.clone()));

    assert!(!ctx.is_cancelled(), "Should not be cancelled initially");

    token.cancel();

    assert!(ctx.is_cancelled(), "Should be cancelled after cancel()");
}

#[tokio::test]
async fn test_permission_context_cancellation() {
    let token = CancellationToken::new();
    let ctx = ToolPermissionContext::with_cancellation(vec![], token.clone());

    assert!(!ctx.is_cancelled());

    token.cancel();

    assert!(ctx.is_cancelled());
}

// ============================================================================
// Default Timeout Constant Test
// ============================================================================

#[test]
fn test_default_timeout_matches_typescript_sdk() {
    // TypeScript SDK uses 60 seconds as default
    assert_eq!(
        HookManager::DEFAULT_HOOK_TIMEOUT,
        Duration::from_secs(60),
        "Default timeout should match TypeScript SDK (60 seconds)"
    );
}
