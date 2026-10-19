#![cfg(unix)]

mod output_failure_support;
mod permission_support;

use output_failure_support::{Delivery, Stream};
use permission_support::Setup;

fn assert_anchored_trace(trace: &output_failure_support::LifecycleTrace) {
    use output_failure_support::LifecycleEvent;
    let events = trace.borrow();
    eprintln!("ANCHOR lifecycle: {events:?}");
    let sealed = events
        .iter()
        .position(|event| *event == LifecycleEvent::SignalsFinished)
        .unwrap();
    let reaped = events
        .iter()
        .position(|event| *event == LifecycleEvent::Reaped)
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == LifecycleEvent::SignalsFinished)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == LifecycleEvent::Reaped)
            .count(),
        1
    );
    assert!(sealed < reaped, "{events:?}");
    assert!(
        events[..sealed]
            .iter()
            .all(|event| matches!(event, LifecycleEvent::GroupSignal(_))),
        "{events:?}"
    );
    assert_eq!(events[sealed - 1], LifecycleEvent::GroupSignal(9));
    assert!(
        !events[sealed..]
            .iter()
            .any(|event| matches!(event, LifecycleEvent::GroupSignal(_))),
        "a signal occurred after ownership was sealed/reaped: {events:?}"
    );
}

#[test]
fn shell_signal_status_is_not_accepted_as_an_ordinary_completion_exit() {
    assert_eq!(
        output_failure_support::command_status(1).unwrap().code(),
        Some(1)
    );
    for code in [-1, 128, 137, 141, 255, 256] {
        assert!(
            output_failure_support::command_status(code).is_err(),
            "{code}"
        );
    }
}

#[test]
fn group_ownership_requires_final_signals_before_reaping_and_never_reopens() {
    use output_failure_support::{GroupOwnership, LifecycleEvent, LifecycleTrace};
    let mut premature = GroupOwnership::default();
    assert!(
        premature.record_reaped().is_err(),
        "the old reap-before-final-signal ordering must be rejected"
    );
    let trace = LifecycleTrace::default();
    let mut ownership = GroupOwnership::new(trace.clone());
    ownership.record_signal(15).unwrap();
    ownership.record_signal(9).unwrap();
    ownership.finish_signals();
    assert!(ownership.record_signal(9).is_err());
    ownership.record_reaped().unwrap();
    assert!(ownership.record_signal(9).is_err());
    assert_eq!(
        *trace.borrow(),
        [
            LifecycleEvent::GroupSignal(15),
            LifecycleEvent::GroupSignal(9),
            LifecycleEvent::SignalsFinished,
            LifecycleEvent::Reaped
        ]
    );
}
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

#[test]
fn synthetic_unsupported_probe_is_visible_and_restores_original_permissions() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("layer.yaml");
    fs::write(&path, "name: app\n").unwrap();
    let before = fs::metadata(&path).unwrap().permissions().mode();
    match permission_support::deny_read_with_probe(&path, || Ok(())).unwrap() {
        Setup::Unsupported(reason) => eprintln!(
            "SKIP synthetic-permission-control (simulated successful read, not a root run): {reason}"
        ),
        Setup::Ready(_) => panic!("successful probe incorrectly established denial"),
    }
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode(), before);
    assert_eq!(fs::read_to_string(&path).unwrap(), "name: app\n");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn permission_setup_errors_are_not_skips_and_cleanup_runs_on_unwind() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("layer.yaml");
    fs::write(&path, "name: app\n").unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    let error =
        permission_support::deny_read_with_probe(&path, || Err(io::ErrorKind::NotFound.into()))
            .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode(), mode);
    let panic = std::panic::catch_unwind(|| {
        let _ =
            permission_support::deny_read_with_probe(&path, || panic!("synthetic probe failure"));
    });
    assert!(panic.is_err());
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode(), mode);
}

#[test]
fn native_read_and_create_denial_probes_restore_and_leave_no_probe_artifacts() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("layer.yaml");
    fs::write(&path, "name: app\n").unwrap();
    let file_mode = fs::metadata(&path).unwrap().permissions().mode();
    let directory_mode = fs::metadata(root.path()).unwrap().permissions().mode();
    for (label, result) in [
        ("read", permission_support::deny_read(&path)),
        ("create", permission_support::deny_create(root.path())),
    ] {
        match result.unwrap() {
            Setup::Ready(guard) => {
                eprintln!("CAPABILITY {label}: native PermissionDenied verified");
                drop(guard);
            }
            Setup::Unsupported(reason) => eprintln!("SKIP native-{label}: {reason}"),
        }
    }
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode(), file_mode);
    assert_eq!(
        fs::metadata(root.path()).unwrap().permissions().mode(),
        directory_mode
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn fifo_setup_exit_and_timeout_reap_the_private_reader() {
    for tail in ["exit 97", "wait \"$reader\"", "kill -KILL $$"] {
        let root = tempfile::tempdir().unwrap();
        let pid = root.path().join("reader-pid");
        let setup = format!("printf '%s' \"$reader\" > '{}'\n{tail}", pid.display());
        let trace = output_failure_support::LifecycleTrace::default();
        let result = output_failure_support::run_with_trace(
            root.path(),
            &["--version"],
            Stream::Stdout,
            Delivery::BrokenPipe,
            Duration::from_secs(1),
            Some(&setup),
            trace.clone(),
        );
        assert_anchored_trace(&trace);
        if tail.starts_with("exit") {
            assert_eq!(result.unwrap().output.status.code(), Some(97));
        } else {
            assert_eq!(result.err().unwrap().kind(), io::ErrorKind::TimedOut);
        }
        let reader = fs::read_to_string(pid).unwrap().parse().unwrap();
        assert!(
            !output_failure_support::process_exists(reader),
            "private reader {reader} survived {tail}"
        );
    }
}

#[test]
fn normal_delivery_modes_finish_all_group_signals_before_reaping_the_anchor() {
    for delivery in [
        Delivery::BrokenPipe,
        Delivery::LiveReader,
        Delivery::ClosedDescriptor,
    ] {
        let root = tempfile::tempdir().unwrap();
        let trace = output_failure_support::LifecycleTrace::default();
        let capture = output_failure_support::run_with_trace(
            root.path(),
            &["--version"],
            Stream::Stdout,
            delivery,
            Duration::from_secs(1),
            None,
            trace.clone(),
        )
        .unwrap();
        assert_anchored_trace(&trace);
        let expected = if matches!(delivery, Delivery::BrokenPipe) {
            1
        } else {
            0
        };
        assert_eq!(capture.output.status.code(), Some(expected));
    }
}

#[test]
fn live_fifo_reader_is_reaped_even_when_the_cli_returns_an_error() {
    let root = tempfile::tempdir().unwrap();
    let pid = root.path().join("reader-pid");
    let setup = format!("printf '%s' \"$reader\" > '{}'", pid.display());
    let trace = output_failure_support::LifecycleTrace::default();
    let capture = output_failure_support::run_with_trace(
        root.path(),
        &["init", "--bogus"],
        Stream::Stdout,
        Delivery::LiveReader,
        Duration::from_secs(1),
        Some(&setup),
        trace.clone(),
    )
    .unwrap();
    assert_anchored_trace(&trace);
    assert_eq!(capture.output.status.code(), Some(2));
    assert!(!capture.output.stderr.is_empty());
    assert!(capture.consumed.is_empty());
    let reader = fs::read_to_string(pid).unwrap().parse().unwrap();
    assert!(!output_failure_support::process_exists(reader));
    assert!(!root.path().join("layer.yaml").exists());
}
