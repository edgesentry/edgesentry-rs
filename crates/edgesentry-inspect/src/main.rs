//! CLI entry point for `edgesentry-inspect`.
//!
//! Usage:
//!   edgesentry-inspect scan --config config.toml

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use trilink_core::{
    BBox2D, CameraIntrinsics, Point3D, PointCloud, Transform4x4,
    bridge::{project_to_depth_map, unproject},
};

use edgesentry_inspect::{
    config::{InferenceMode, load_config},
    deviation::{compute_deviation, per_point_deviations_mm},
    heatmap::{render_heatmap, write_heatmap_png},
    ifc::load_ifc_points,
    inference::{depth_map_to_png, http_infer, mock_infer, onnx_infer},
    ply::load_ply_points,
    points::{write_points, PointsJson},
    report::write_report,
};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "edgesentry-inspect",
    about = "Edge-first 3D scan vs. reference deviation tool",
    long_about = "Compare an as-built point cloud scan against an IFC design model.\n\
                  Computes per-point deviation, renders a colour heatmap, and writes\n\
                  a JSON report and points file for the Inspect App viewer."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare a scan against an IFC design and write report.json, heatmap.png, points.json.
    ///
    /// Reads the IFC reference model and the PLY scan point cloud specified in
    /// the config file, computes per-point deviation, optionally runs AI defect
    /// detection, renders a colour heatmap, and writes three output files.
    Scan {
        /// Path to the TOML configuration file (see config.example.toml).
        #[arg(long)]
        config: PathBuf,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sep() {
    println!("{}", "─".repeat(60));
}

fn ok(msg: &str) {
    println!("      ✓  {msg}");
}

fn skip(msg: &str) {
    println!("      –  {msg}");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Scan { config: config_path } => {
            // ── Load config ──────────────────────────────────────────────
            let cfg = match load_config(&config_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: could not load config from {}: {e}", config_path.display());
                    process::exit(1);
                }
            };

            let threshold = cfg.output.threshold_mm;

            // ── Header ───────────────────────────────────────────────────
            println!();
            println!("EdgeSentry Inspect — scan pipeline");
            sep();
            println!();
            println!("Inputs");
            println!(
                "  IFC design  : {}",
                cfg.ifc_path.display()
            );
            println!(
                "               (planned reference geometry — what the design specifies)"
            );
            println!(
                "  Scan cloud  : {}",
                cfg.scan_path.display()
            );
            println!(
                "               (as-built measurements — what was actually constructed)"
            );
            println!("  Threshold   : {threshold:.1} mm");
            println!(
                "               (scan points within this distance of the design are compliant)"
            );
            println!(
                "  AI mode     : {}",
                match cfg.inference.mode {
                    InferenceMode::Off => "off (deviation report only)",
                    InferenceMode::Mock => "mock (built-in demo detections)",
                    InferenceMode::Onnx => "onnx (local model file, in-process)",
                    InferenceMode::Http => "http (POST depth map to inference server)",
                }
            );
            println!("  Output dir  : {}", cfg.output.dir.display());
            println!();

            // ── Step 1: IFC ───────────────────────────────────────────────
            println!("[1/5] Loading IFC reference model …");
            let reference = match load_ifc_points(&cfg.ifc_path) {
                Ok(pts) => pts,
                Err(e) => {
                    eprintln!("      error: {e}");
                    process::exit(1);
                }
            };
            ok(&format!(
                "{} reference points extracted from IFC geometry",
                reference.len()
            ));
            println!("      These are the planned surface positions from the design model.");
            println!();

            // ── Step 2: PLY ───────────────────────────────────────────────
            println!("[2/5] Loading scan point cloud …");
            let scan = match load_ply_points(&cfg.scan_path) {
                Ok(pts) => pts,
                Err(e) => {
                    eprintln!("      error: {e}");
                    process::exit(1);
                }
            };
            ok(&format!(
                "{} scan points loaded from PLY file",
                scan.len()
            ));
            println!("      Each point is a 3D (x, y, z) measurement from the LiDAR/ToF sensor.");
            println!();

            // ── Step 3: Depth map + AI inference ─────────────────────────
            println!(
                "[3/5] Projecting scan to depth map ({} × {}) …",
                cfg.camera.width, cfg.camera.height
            );
            let k = CameraIntrinsics {
                fx: cfg.camera.fx,
                fy: cfg.camera.fy,
                cx: cfg.camera.cx,
                cy: cfg.camera.cy,
            };
            let pose = Transform4x4::identity();
            let cloud =
                PointCloud { capture_ts_us: 0, points: scan.clone(), intensities: None };
            let depth_map =
                project_to_depth_map(&cloud, &pose, &k, cfg.camera.width, cfg.camera.height);
            ok("depth map produced");
            println!(
                "      The 3D scan is projected to a 2D image so the AI model can \
                 detect defects."
            );
            println!();

            println!("[4/5] Running AI inference …");
            let detections: Vec<BBox2D> = match &cfg.inference.mode {
                InferenceMode::Off => {
                    skip("skipped (inference.mode = \"off\" in config)");
                    println!(
                        "      Set inference.mode = \"mock\" for a built-in demo, \
                         \"onnx\" with model_path for a local model, or \
                         \"http\" with an endpoint for a remote model."
                    );
                    vec![]
                }
                InferenceMode::Mock => {
                    let dets = mock_infer();
                    ok(&format!("{} demo defect bounding boxes (built-in mock)", dets.len()));
                    println!(
                        "      Each bounding box is back-projected to a 3D world \
                         coordinate and shown as a sphere in the Inspect App."
                    );
                    dets
                }
                InferenceMode::Onnx => {
                    let model_path = match cfg.inference.model_path.as_deref() {
                        Some(p) => p,
                        None => {
                            eprintln!(
                                "      error: inference.model_path is required when \
                                 inference.mode = \"onnx\""
                            );
                            process::exit(1);
                        }
                    };
                    println!("      Loading ONNX model from {} …", model_path.display());
                    match onnx_infer(model_path, &depth_map) {
                        Ok(dets) => {
                            ok(&format!("{} defect bounding boxes from ONNX model", dets.len()));
                            println!(
                                "      Each bounding box is back-projected to a 3D world \
                                 coordinate and shown as a sphere in the Inspect App."
                            );
                            dets
                        }
                        Err(e) => {
                            eprintln!("      error: ONNX inference failed: {e}");
                            process::exit(1);
                        }
                    }
                }
                InferenceMode::Http => {
                    let endpoint = match cfg.inference.endpoint.as_deref() {
                        Some(e) => e,
                        None => {
                            eprintln!(
                                "      error: inference.endpoint is required when \
                                 inference.mode = \"http\""
                            );
                            process::exit(1);
                        }
                    };
                    println!("      POSTing depth map PNG to {} …", endpoint);
                    let png = match depth_map_to_png(&depth_map) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("      error: {e}");
                            process::exit(1);
                        }
                    };
                    match http_infer(endpoint, &png) {
                        Ok(dets) => {
                            ok(&format!("{} defect bounding boxes returned", dets.len()));
                            println!(
                                "      Each bounding box is back-projected to a 3D world \
                                 coordinate and shown as a sphere in the Inspect App."
                            );
                            dets
                        }
                        Err(e) => {
                            eprintln!("      error: inference request failed: {e}");
                            process::exit(1);
                        }
                    }
                }
            };
            println!();

            // Back-project detections to 3D
            let world_detections: Vec<Point3D> = detections
                .iter()
                .map(|bbox| {
                    unproject(bbox, None, cfg.inference.fallback_depth_m, &k, &pose)
                })
                .collect();

            // ── Step 4: Deviation ─────────────────────────────────────────
            println!("[5/5] Computing per-point deviation, rendering heatmap, writing outputs …");
            let deviations_mm = per_point_deviations_mm(&scan, &reference);
            let report = compute_deviation(&scan, &reference, threshold);

            let img = render_heatmap(
                &scan,
                &deviations_mm,
                &pose,
                &k,
                cfg.camera.width,
                cfg.camera.height,
                threshold,
            );

            if let Err(e) = std::fs::create_dir_all(&cfg.output.dir) {
                eprintln!("      error: could not create output directory: {e}");
                process::exit(1);
            }

            let report_path = cfg.output.dir.join("report.json");
            let heatmap_path = cfg.output.dir.join("heatmap.png");
            let points_path = cfg.output.dir.join("points.json");

            for (label, result) in [
                ("report.json", write_report(&report, &report_path).map_err(|e| e.to_string())),
                (
                    "heatmap.png",
                    write_heatmap_png(&img, &heatmap_path).map_err(|e| e.to_string()),
                ),
                (
                    "points.json",
                    write_points(
                        &PointsJson::new(&scan, &deviations_mm, &world_detections),
                        &points_path,
                    )
                    .map_err(|e| e.to_string()),
                ),
            ] {
                if let Err(e) = result {
                    eprintln!("      error: failed to write {label}: {e}");
                    process::exit(1);
                }
            }

            ok("outputs written");
            println!();

            // ── Results ───────────────────────────────────────────────────
            sep();
            println!("Results");
            println!();

            let non_compliant = report.point_count
                - (report.point_count as f64 * report.compliant_pct / 100.0).round() as usize;

            let compliant_symbol =
                if report.compliant_pct >= 95.0 { "✓  pass" } else { "✗  fail" };
            println!(
                "  Points analysed      : {:>8}",
                report.point_count
            );
            println!(
                "  Compliant ≤ {threshold:.0} mm   : {:>7.1}%   {compliant_symbol}",
                report.compliant_pct
            );
            if non_compliant > 0 {
                println!(
                    "  Non-compliant points : {:>8}  \
                     ({:.1}% of scan above threshold)",
                    non_compliant,
                    100.0 - report.compliant_pct
                );
            }

            let max_symbol = if report.max_deviation_mm <= threshold {
                "✓"
            } else if report.max_deviation_mm <= 2.0 * threshold {
                "⚠"
            } else {
                "✗"
            };
            println!(
                "  Max deviation        : {:>7.2} mm  {} ({:.1}× threshold)",
                report.max_deviation_mm,
                max_symbol,
                report.max_deviation_mm / threshold
            );

            let mean_symbol = if report.mean_deviation_mm <= threshold * 0.5 { "✓" } else { "⚠" };
            println!(
                "  Mean deviation       : {:>7.2} mm  {}",
                report.mean_deviation_mm, mean_symbol
            );

            println!(
                "  AI detections        : {:>8}  {}",
                world_detections.len(),
                if world_detections.is_empty() { "–" } else { "⚠  see orange spheres in viewer" }
            );

            println!();
            println!("Output files");
            println!("  {}  — deviation statistics", report_path.display());
            println!(
                "  {}  — 2D colour map projected from scan",
                heatmap_path.display()
            );
            println!(
                "  {}  — per-point 3D positions and deviations",
                points_path.display()
            );
            println!();
            println!("Colour scale  green  = compliant  (deviation ≤ {threshold:.0} mm)");
            println!("              yellow = warning    (deviation ≤ {:.0} mm)", threshold * 2.0);
            println!("              red    = exceeds    (deviation  > {:.0} mm)", threshold * 2.0);
            println!();
            println!(
                "Open the output folder in EdgeSentry Inspect App to view \
                 the 3D deviation point cloud."
            );
            println!();
        }
    }
}
