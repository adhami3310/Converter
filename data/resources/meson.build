blueprints = custom_target('blueprints',
  input: files(
    'blueprints/window.blp',
    'blueprints/image-thumbnail.blp',
    'blueprints/image-rest.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

scalable_dir = 'icons' / 'hicolor' / 'scalable' / 'apps'
install_data(
  [
    scalable_dir / ('@0@.svg').format(base_id),
    scalable_dir / ('@0@.svg').format(application_id)
  ],
  install_dir: datadir /scalable_dir
)

symbolic_dir = 'icons' / 'hicolor' / 'symbolic' / 'apps'
install_data(
  symbolic_dir / ('@0@-symbolic.svg').format(base_id),
  install_dir: datadir / symbolic_dir
)

# Resources
resources = gnome.compile_resources(
    'resources',
    'resources.gresource.xml',
    gresource_bundle: true,
    source_dir: meson.current_build_dir(),
    install: true,
    install_dir: pkgdatadir,
    dependencies: [blueprints, appdata_file],
)