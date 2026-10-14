#![cfg(feature = "experimental")]

use std::{
  borrow::Cow,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use rolldown::{
  Bundler, BundlerOptions, DevModeOptions, ExperimentalOptions, InputItem, SourceMapType,
};
use rolldown_common::{BundleMode, ScanMode};
use rolldown_plugin::{
  HookTransformArgs, HookTransformOutput, HookTransformOutputMap, HookTransformReturn, HookUsage,
  Plugin, SharedTransformPluginContext,
};
use rolldown_sourcemap::OwnedSourceMap;

#[derive(Debug)]
struct TransformMap;

impl Plugin for TransformMap {
  fn name(&self) -> Cow<'static, str> {
    "transform-map".into()
  }

  fn register_hook_usage(&self) -> HookUsage {
    HookUsage::Transform
  }

  async fn transform(
    &self,
    _ctx: SharedTransformPluginContext,
    args: &HookTransformArgs<'_>,
  ) -> HookTransformReturn {
    if !args.id.ends_with("entry.js") {
      return Ok(None);
    }
    Ok(Some(HookTransformOutput {
      code: Some(args.code.to_string()),
      map: HookTransformOutputMap::from(OwnedSourceMap::from_json_string(
        r#"{"version":3,"sources":["original.js"],"sourcesContent":["console.log('original input');"],"names":[],"mappings":"AAAA"}"#,
      )?.into_inner()),
      ..Default::default()
    }))
  }
}

#[derive(Debug)]
struct CombinedMapConsumer(Arc<AtomicUsize>);

impl Plugin for CombinedMapConsumer {
  fn name(&self) -> Cow<'static, str> {
    "combined-map-consumer".into()
  }

  fn register_hook_usage(&self) -> HookUsage {
    HookUsage::Transform
  }

  async fn transform(
    &self,
    ctx: SharedTransformPluginContext,
    args: &HookTransformArgs<'_>,
  ) -> HookTransformReturn {
    if args.id.ends_with("entry.js")
      && ctx.get_combined_sourcemap().get_source_content(0)
        == Some("console.log('original input');")
    {
      self.0.fetch_add(1, Ordering::Relaxed);
    }
    Ok(None)
  }
}

#[tokio::test(flavor = "multi_thread")]
async fn sourcemap_chain_lives_through_hooks_and_only_needed_module_caches() {
  for (sourcemap, dev_mode) in [(false, false), (true, false), (false, true), (true, true)] {
    let reads = Arc::new(AtomicUsize::new(0));
    let mut bundler = Bundler::with_plugins(
      BundlerOptions {
        input: Some(vec![InputItem { name: None, import: "./entry.js".into() }]),
        cwd: Some(
          concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/rolldown/plugin/plugin_context/sourcemap_broken_warning"
          )
          .into(),
        ),
        sourcemap: sourcemap.then_some(SourceMapType::File),
        experimental: Some(ExperimentalOptions {
          incremental_build: Some(true),
          native_magic_string: Some(true),
          dev_mode: dev_mode.then(DevModeOptions::default),
          ..Default::default()
        }),
        ..Default::default()
      },
      vec![Arc::new(TransformMap), Arc::new(CombinedMapConsumer(Arc::clone(&reads)))],
    )
    .unwrap();
    let mut entry_id = None;
    for step in 0..3 {
      let scan_mode = match step {
        0 => ScanMode::Full,
        1 => ScanMode::Partial(vec![]),
        _ => ScanMode::Partial(vec![entry_id.clone().unwrap()]),
      };
      let bundle_mode =
        if step == 0 { BundleMode::IncrementalFullBuild } else { BundleMode::IncrementalBuild };
      entry_id = Some(
        bundler
          .with_cached_bundle_experimental(bundle_mode, async |bundle| {
            let output = bundle.scan_modules(scan_mode).await?;
            let entry = output
              .module_table
              .modules
              .iter()
              .filter_map(|module| module.as_normal())
              .find(|module| module.id.ends_with("entry.js"))
              .unwrap();
            assert_eq!(
              !entry.sourcemap_chain.is_empty(),
              sourcemap || dev_mode,
              "retention on scan {step}, sourcemap={sourcemap}, dev_mode={dev_mode}"
            );
            let id = entry.id.as_arc_str().clone();
            bundle.bundle_generate(output).await?;
            Ok(id)
          })
          .await
          .unwrap(),
      );
      assert_eq!(reads.load(Ordering::Relaxed), if step == 2 { 2 } else { 1 });
    }
  }
}
