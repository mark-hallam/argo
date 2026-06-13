//! Tests for `process_registry` — extracted to a sibling file to keep
//! process_registry.rs under the 500-LOC ceiling (the portable cross-platform
//! spawn fix, win-process-registry-spawn-portability, pushed it to 549).

    use super::*;

    fn ok_dir() -> PathBuf {
        std::env::temp_dir()
    }

    fn t1() -> TenantId {
        TenantId(uuid::Uuid::from_u128(1))
    }

    // Platform-portable test commands: production spawns through the native
    // shell ([`shell_command`]: `bash -c` on unix, `cmd /C` on Windows), so the
    // test command-line syntax must match the active shell.

    /// Prints `line-1`..`line-3`, each on its own line.
    fn echo_lines_cmd() -> &'static str {
        #[cfg(windows)]
        {
            "echo line-1&&echo line-2&&echo line-3"
        }
        #[cfg(not(windows))]
        {
            "for i in 1 2 3; do echo line-$i; done; sleep 0.2"
        }
    }

    /// Exits 0 immediately (no-op).
    fn noop_exit0_cmd() -> &'static str {
        #[cfg(windows)]
        {
            "exit 0"
        }
        #[cfg(not(windows))]
        {
            "true"
        }
    }

    /// Runs long enough to be observed as "running" before a kill.
    fn long_running_cmd() -> &'static str {
        #[cfg(windows)]
        {
            // `ping` is a cmd-native long-runner needing no stdin (unlike
            // `timeout`); `-n 6` ≈ 5s — comfortably longer than the 2s kill
            // grace, short enough not to orphan a 30s process. stdout dropped.
            "ping -n 6 127.0.0.1 >nul"
        }
        #[cfg(not(windows))]
        {
            "sleep 30"
        }
    }

    #[tokio::test]
    async fn spawn_list_and_kill_round_trip() {
        let reg = ProcessRegistry::new();
        let id = reg
            .spawn(
                SpawnRequest {
                    command: long_running_cmd().into(),
                    working_dir: ok_dir(),
                },
                t1(),
            )
            .await
            .unwrap();
        let live = reg.list(t1()).await;
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].id, id);
        assert_eq!(live[0].state, "running");
        assert!(live[0].pid.is_some());

        reg.kill(&id, t1(), Duration::from_secs(2)).await.unwrap();
        assert_eq!(reg.list(t1()).await.len(), 0);
    }

    #[tokio::test]
    async fn tail_returns_recent_stdout() {
        let reg = ProcessRegistry::new();
        let id = reg
            .spawn(
                SpawnRequest {
                    command: echo_lines_cmd().into(),
                    working_dir: ok_dir(),
                },
                t1(),
            )
            .await
            .unwrap();
        // Give the pump time to drain.
        tokio::time::sleep(Duration::from_millis(400)).await;
        let tail = reg.tail(&id, t1()).await.unwrap();
        assert!(tail.stdout.contains("line-1"));
        assert!(tail.stdout.contains("line-3"));
        // Cleanup (process exits naturally; reap_finished drops it).
        reg.reap_finished(t1()).await;
    }

    #[tokio::test]
    async fn reap_finished_drops_exited_entries() {
        let reg = ProcessRegistry::new();
        let _ = reg
            .spawn(
                SpawnRequest {
                    command: noop_exit0_cmd().into(),
                    working_dir: ok_dir(),
                },
                t1(),
            )
            .await
            .unwrap();
        // Wait for `true` to exit + the pumps to close.
        tokio::time::sleep(Duration::from_millis(200)).await;
        reg.reap_finished(t1()).await;
        assert!(reg.is_empty(t1()).await);
    }

    #[tokio::test]
    async fn unknown_kill_returns_tool_error() {
        let reg = ProcessRegistry::new();
        let err = reg
            .kill(
                &ProcessId("does-not-exist".into()),
                t1(),
                Duration::from_secs(1),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ArgoError::Tool { .. }));
    }

    #[test]
    fn process_id_is_unique() {
        let a = ProcessId::new();
        let b = ProcessId::new();
        assert_ne!(a, b);
    }
