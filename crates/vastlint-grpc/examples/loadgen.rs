//! Load harness for the adaptive concurrency limiter.
//!
//! Exists because phase 3 without a measurement is a claim. The server can
//! expose a shed counter and a latency histogram and still be telling nobody
//! anything: the question is whether the limiter actually bounds tail latency
//! when offered load exceeds capacity, and the only way to know is to offer
//! more load than capacity and look.
//!
//! ## What this measures
//!
//! A closed-loop ramp. At each level, N workers each issue `Validate` calls
//! back to back for a fixed window, so offered concurrency is exactly N. That
//! is the right shape for this question: as N passes what the server can
//! actually do, an unprotected server's latency grows without bound because the
//! excess sits in queues, while a protected one refuses the excess and keeps
//! its served requests fast.
//!
//! Latency is measured client-side, which is the number that matters. A server
//! that reports a healthy internal p99 while callers wait in a connection
//! backlog is reporting on the wrong thing.
//!
//! ## What it does not measure
//!
//! Throughput here is not a benchmark of the validator. The client, the server,
//! and the load generator share one machine, so absolute numbers are worth less
//! than the comparison between two runs of the same shape on the same box. The
//! comparison is the point.
//!
//! ```text
//! cargo run --release --example loadgen -- --label with-limiter
//! cargo run --release --example loadgen -- --target http://127.0.0.1:50051 --csv out.csv
//! ```

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use tonic::transport::Channel;
use tonic::Code;
use vastlint_grpc::proto::vastlint_service_client::VastlintServiceClient;
use vastlint_grpc::proto::ValidateRequest;

/// Concurrency levels to walk through.
///
/// Doubling rather than stepping, because the interesting behaviour is a knee
/// and a linear ramp spends most of its time far away from it.
const LEVELS: &[usize] = &[1, 2, 4, 8, 16, 32, 64, 128, 256];

/// How long to hold each level.
const DEFAULT_WINDOW: Duration = Duration::from_secs(3);

/// Discarded before measurement starts at each level, so the numbers describe
/// steady state rather than the moment the limiter was still adapting.
const WARMUP: Duration = Duration::from_millis(500);

struct Args {
    target: String,
    label: String,
    window: Duration,
    csv: Option<String>,
    /// Concurrency levels to run. Defaults to the full ramp.
    levels: Vec<usize>,
}

fn parse_args() -> Args {
    let mut args = Args {
        target: "http://127.0.0.1:50051".to_string(),
        label: "run".to_string(),
        window: DEFAULT_WINDOW,
        csv: None,
        levels: LEVELS.to_vec(),
    };

    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        match flag.as_str() {
            "--target" => args.target = argv.next().expect("--target needs a value"),
            "--label" => args.label = argv.next().expect("--label needs a value"),
            "--csv" => args.csv = Some(argv.next().expect("--csv needs a value")),
            // Useful for isolating one point on the curve, for example when
            // comparing client-observed latency against the server's own
            // histogram at a single concurrency.
            "--levels" => {
                args.levels = argv
                    .next()
                    .expect("--levels needs a comma-separated list")
                    .split(',')
                    .map(|level| level.trim().parse().expect("--levels must be numbers"))
                    .collect();
            }
            "--window-secs" => {
                let secs: u64 = argv
                    .next()
                    .expect("--window-secs needs a value")
                    .parse()
                    .expect("--window-secs must be a number");
                args.window = Duration::from_secs(secs);
            }
            other => panic!("unknown flag {other}"),
        }
    }

    args
}

/// A light tag: one ad, complete and realistic.
///
/// Not a minimal tag. A stripped-down document validates in tens of
/// microseconds, which would put the corpus spread near 50x and make the whole
/// ramp unreadable. Real single-ad traffic carries quartile tracking and
/// several renditions, so that is what this carries, landing near the published
/// 363µs light benchmark.
fn light_document() -> String {
    let mut xml = String::from(
        r#"<VAST version="4.1"><Ad id="1"><InLine><AdSystem version="1.0">Example</AdSystem><AdTitle>Example Ad</AdTitle><AdServingId>serving-1</AdServingId><Advertiser>Example Brand</Advertiser><Pricing model="CPM" currency="USD">15.00</Pricing>"#,
    );

    for tracker in 0..3 {
        xml.push_str(&format!(
            r#"<Impression id="imp-{tracker}"><![CDATA[https://t.example.com/i?n={tracker}]]></Impression>"#
        ));
    }

    xml.push_str(
        r#"<Creatives><Creative id="c1" sequence="1"><UniversalAdId idRegistry="ad-id.org">UID-0001</UniversalAdId><Linear><Duration>00:00:15</Duration><TrackingEvents>"#,
    );
    for event in [
        "start",
        "firstQuartile",
        "midpoint",
        "thirdQuartile",
        "complete",
    ] {
        xml.push_str(&format!(
            r#"<Tracking event="{event}"><![CDATA[https://t.example.com/{event}]]></Tracking>"#
        ));
    }
    xml.push_str("</TrackingEvents><MediaFiles>");
    for (width, height) in [(640, 360), (1280, 720), (1920, 1080)] {
        xml.push_str(&format!(
            r#"<MediaFile delivery="progressive" type="video/mp4" width="{width}" height="{height}" bitrate="2000"><![CDATA[https://cdn.example.com/a-{width}.mp4]]></MediaFile>"#
        ));
    }
    xml.push_str(
        r#"</MediaFiles><VideoClicks><ClickThrough><![CDATA[https://example.com/landing]]></ClickThrough><ClickTracking><![CDATA[https://t.example.com/click]]></ClickTracking></VideoClicks></Linear></Creative></Creatives></InLine></Ad></VAST>"#,
    );

    xml
}

/// A heavy tag: a pod with several creatives and full tracking.
///
/// The mix matters, and so does its spread. A uniform corpus of light tags
/// would never saturate the server from one machine. But an absurdly heavy one
/// is just as useless in the other direction: the first version of this
/// generator built a 40-ad document that took 340ms, so p999 measured "did this
/// request draw the heavy document" and said nothing about queueing at all. The
/// tail has to come from load, not from corpus variance.
///
/// Sized to the *ratio* of the published per-tag benchmarks, 363µs light to
/// 2,104µs heavy, roughly 6x. The absolute numbers are not reproduced and are
/// not meant to be: they depend on the machine. What has to hold is that the
/// heavy document is expensive enough to saturate the server and cheap enough
/// that drawing it is not itself the tail. `calibrate` prints what it actually
/// costs on the machine under test, so a drift here is visible rather than
/// assumed.
fn heavy_document() -> String {
    let mut xml = String::from(r#"<VAST version="4.2">"#);

    for ad in 0..1 {
        xml.push_str(&format!(r#"<Ad id="{ad}" sequence="{}"><InLine>"#, ad + 1));
        xml.push_str("<AdSystem>Example</AdSystem><AdTitle>Heavy</AdTitle>");
        xml.push_str(&format!("<AdServingId>serving-{ad}</AdServingId>"));

        for tracker in 0..10 {
            xml.push_str(&format!(
                r#"<Impression id="i{tracker}"><![CDATA[https://t.example.com/i?ad={ad}&n={tracker}]]></Impression>"#
            ));
        }

        xml.push_str("<Creatives>");
        for creative in 0..4 {
            xml.push_str(&format!(
                r#"<Creative id="c{creative}"><UniversalAdId idRegistry="ad-id.org">UID-{ad}-{creative}</UniversalAdId><Linear><Duration>00:00:30</Duration><TrackingEvents>"#
            ));
            for event in [
                "start",
                "firstQuartile",
                "midpoint",
                "thirdQuartile",
                "complete",
                "pause",
                "resume",
                "mute",
            ] {
                xml.push_str(&format!(
                    r#"<Tracking event="{event}"><![CDATA[https://t.example.com/{event}?a={ad}]]></Tracking>"#
                ));
            }
            xml.push_str("</TrackingEvents><MediaFiles>");
            for media in 0..4 {
                xml.push_str(&format!(
                    r#"<MediaFile delivery="progressive" type="video/mp4" width="1920" height="1080"><![CDATA[https://cdn.example.com/{ad}-{creative}-{media}.mp4]]></MediaFile>"#
                ));
            }
            xml.push_str("</MediaFiles></Linear></Creative>");
        }
        xml.push_str("</Creatives></InLine></Ad>");
    }

    xml.push_str("</VAST>");
    xml
}

/// One corpus entry, with its class recorded rather than inferred.
///
/// Classifying by document length worked until the light tag grew realistic
/// enough to cross the threshold, at which point the calibration output would
/// have quietly reported ten heavy documents and no light ones.
struct Document {
    heavy: bool,
    xml: String,
}

#[derive(Default)]
struct Counts {
    ok: AtomicU64,
    shed: AtomicU64,
    other: AtomicU64,
}

struct LevelResult {
    concurrency: usize,
    ok: u64,
    shed: u64,
    other: u64,
    goodput: f64,
    p50_ms: f64,
    p99_ms: f64,
    p999_ms: f64,
    max_ms: f64,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args();

    // One 80/20 light-to-heavy corpus, built once. Rebuilding per request would
    // measure string formatting as much as validation.
    let corpus: Arc<Vec<Document>> = Arc::new(
        (0..10)
            .map(|i| {
                if i < 8 {
                    Document {
                        heavy: false,
                        xml: light_document(),
                    }
                } else {
                    Document {
                        heavy: true,
                        xml: heavy_document(),
                    }
                }
            })
            .collect(),
    );

    println!("target      {}", args.target);
    println!("label       {}", args.label);
    println!(
        "window      {:?} per level, after {WARMUP:?} warm-up",
        args.window
    );
    println!(
        "corpus      {} documents, {} light and {} heavy",
        corpus.len(),
        corpus.iter().filter(|d| !d.heavy).count(),
        corpus.iter().filter(|d| d.heavy).count()
    );
    calibrate(&corpus);
    println!();

    let mut results = Vec::new();

    for &concurrency in &args.levels {
        let result = run_level(&args, Arc::clone(&corpus), concurrency).await?;
        print_row(&result, results.is_empty());
        results.push(result);
    }

    if let Some(path) = &args.csv {
        write_csv(path, &args.label, &results)?;
        println!("\nwrote {path}");
    }

    summarise(&args.label, &results);

    Ok(())
}

/// Measures what the corpus costs locally, with no transport involved.
///
/// This is the denominator for everything that follows. Without it, a p999 of
/// 40ms could be queueing or could be one expensive document, and there would
/// be no way to tell from the ramp alone. It also catches the failure that
/// broke the first version of this harness: a corpus whose spread is so wide
/// that tail latency tracks document choice rather than load.
fn calibrate(corpus: &[Document]) {
    let mut light = Vec::new();
    let mut heavy = Vec::new();

    for document in corpus {
        // Warm the allocator and any lazily built state first, so the first
        // document measured is not also paying for initialisation.
        let _ = vastlint_core::validate(&document.xml);

        let mut samples = Vec::new();
        for _ in 0..20 {
            let started = Instant::now();
            let _ = vastlint_core::validate(&document.xml);
            samples.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        samples.sort_by(|a, b| a.partial_cmp(b).expect("no NaN in timings"));
        let median = samples[samples.len() / 2];

        if document.heavy {
            heavy.push(median);
        } else {
            light.push(median);
        }
    }

    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len().max(1) as f64;
    let light_ms = mean(&light);
    let heavy_ms = mean(&heavy);

    println!(
        "cost        light {light_ms:.3} ms, heavy {heavy_ms:.3} ms, spread {:.1}x (measured here, no transport)",
        heavy_ms / light_ms.max(f64::MIN_POSITIVE)
    );

    // A corpus whose slowest document is orders of magnitude past its fastest
    // makes the tail unreadable: p999 becomes a statement about which document
    // was drawn. Warn rather than fail, because the run is still informative if
    // the reader knows.
    if heavy_ms / light_ms.max(f64::MIN_POSITIVE) > 20.0 {
        println!(
            "            WARNING: spread is wide enough that tail latency may track document \
             choice rather than load"
        );
    }
}

async fn run_level(
    args: &Args,
    corpus: Arc<Vec<Document>>,
    concurrency: usize,
) -> Result<LevelResult, Box<dyn std::error::Error>> {
    let counts = Arc::new(Counts::default());
    // 3 significant figures out to a minute. Enough resolution for a p999 in
    // the microsecond range without an enormous histogram.
    let histogram: Arc<std::sync::Mutex<Histogram<u64>>> = Arc::new(std::sync::Mutex::new(
        Histogram::new_with_bounds(1, 60_000_000, 3)?,
    ));

    let deadline = Instant::now() + WARMUP + args.window;
    let measure_from = Instant::now() + WARMUP;

    let mut workers = Vec::with_capacity(concurrency);
    for worker in 0..concurrency {
        // A channel per worker rather than a shared one. tonic multiplexes over
        // HTTP/2, so a single channel would make the client's own stream
        // concurrency limit part of what is being measured.
        let channel = Channel::from_shared(args.target.clone())?.connect().await?;
        let mut client = VastlintServiceClient::new(channel);

        let counts = Arc::clone(&counts);
        let histogram = Arc::clone(&histogram);
        let corpus = Arc::clone(&corpus);

        workers.push(tokio::spawn(async move {
            let mut index = worker;

            while Instant::now() < deadline {
                let document = corpus[index % corpus.len()].xml.clone();
                index += 1;

                let started = Instant::now();
                let outcome = client
                    .validate(ValidateRequest {
                        document,
                        context: None,
                    })
                    .await;
                let elapsed = started.elapsed();

                let measured = started >= measure_from;

                match outcome {
                    Ok(_) => {
                        if measured {
                            counts.ok.fetch_add(1, Ordering::Relaxed);
                            let micros = elapsed.as_micros().max(1) as u64;
                            let _ = histogram.lock().unwrap().record(micros);
                        }
                    }
                    Err(status) if status.code() == Code::ResourceExhausted => {
                        if measured {
                            counts.shed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        if measured {
                            counts.other.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }
        }));
    }

    for worker in workers {
        worker.await?;
    }

    let histogram = histogram.lock().unwrap();
    let ok = counts.ok.load(Ordering::Relaxed);

    Ok(LevelResult {
        concurrency,
        ok,
        shed: counts.shed.load(Ordering::Relaxed),
        other: counts.other.load(Ordering::Relaxed),
        // Goodput, not throughput: shed responses are fast and counting them
        // would make an overloaded server look productive.
        goodput: ok as f64 / args.window.as_secs_f64(),
        p50_ms: histogram.value_at_quantile(0.50) as f64 / 1_000.0,
        p99_ms: histogram.value_at_quantile(0.99) as f64 / 1_000.0,
        p999_ms: histogram.value_at_quantile(0.999) as f64 / 1_000.0,
        max_ms: histogram.max() as f64 / 1_000.0,
    })
}

fn print_row(result: &LevelResult, first_row: bool) {
    if first_row {
        println!(
            "{:>5}  {:>9}  {:>8}  {:>8}  {:>8}  {:>8}  {:>7}  {:>7}",
            "conc", "goodput/s", "p50 ms", "p99 ms", "p999 ms", "max ms", "ok", "shed"
        );
        println!("{}", "-".repeat(76));
    }

    println!(
        "{:>5}  {:>9.0}  {:>8.2}  {:>8.2}  {:>8.2}  {:>8.2}  {:>7}  {:>7}{}",
        result.concurrency,
        result.goodput,
        result.p50_ms,
        result.p99_ms,
        result.p999_ms,
        result.max_ms,
        result.ok,
        result.shed,
        if result.other > 0 {
            format!("  ({} errors)", result.other)
        } else {
            String::new()
        }
    );
}

fn write_csv(
    path: &str,
    label: &str,
    results: &[LevelResult],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut csv = String::from(
        "label,concurrency,goodput_per_s,p50_ms,p99_ms,p999_ms,max_ms,ok,shed,errors\n",
    );

    for result in results {
        csv.push_str(&format!(
            "{label},{},{:.1},{:.3},{:.3},{:.3},{:.3},{},{},{}\n",
            result.concurrency,
            result.goodput,
            result.p50_ms,
            result.p99_ms,
            result.p999_ms,
            result.max_ms,
            result.ok,
            result.shed,
            result.other
        ));
    }

    std::fs::write(path, csv)?;
    Ok(())
}

/// Prints the one comparison the run exists to produce.
fn summarise(label: &str, results: &[LevelResult]) {
    let Some(first) = results.first() else { return };
    let Some(last) = results.last() else { return };

    println!();
    println!("{label}:");
    println!(
        "  p999 went from {:.2} ms at concurrency {} to {:.2} ms at concurrency {} ({:.1}x)",
        first.p999_ms,
        first.concurrency,
        last.p999_ms,
        last.concurrency,
        last.p999_ms / first.p999_ms.max(0.001),
    );

    let shed: u64 = results.iter().map(|r| r.shed).sum();
    let ok: u64 = results.iter().map(|r| r.ok).sum();
    if shed == 0 {
        println!("  nothing was shed: every request was admitted");
    } else {
        println!(
            "  {shed} of {} requests shed ({:.1}%)",
            shed + ok,
            100.0 * shed as f64 / (shed + ok) as f64
        );
    }

    let errors: u64 = results.iter().map(|r| r.other).sum();
    if errors > 0 {
        println!("  {errors} requests failed for other reasons, which is worth explaining before trusting the rest");
    }
}
