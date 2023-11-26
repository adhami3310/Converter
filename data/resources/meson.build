blueprints = custom_target('blueprints',
  input: files(
    'blueprints/help-overlay.blp',
    'blueprints/window.blp',
    'blueprints/image-thumbnail.blp',
    'blueprints/image-rest.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

scalable_dir = join_paths('icons', 'hicolor', 'scalable', 'apps')
install_data(
  join_paths(scalable_dir, ('@0@.svg').format(application_id)),
  install_dir: join_paths(get_option('datadir'), scalable_dir)
)

symbolic_dir = join_paths('icons', 'hicolor', 'symbolic', 'apps')
install_data(
  join_paths(symbolic_dir, ('@0@-symbolic.svg').format('io.gitlab.adhami3310.Converter')),
  install_dir: join_paths(get_option('datadir'), symbolic_dir)
)

action_dir = join_paths('icons', 'hicolor', 'scalable', 'actions')
action_icons = [
    # each icon must be registered here
    join_paths(action_dir, 'im-symbolic.svg'),
    join_paths(action_dir, 'image-symbolic.svg'),
    join_paths(action_dir, 'view-more-horizontal-symbolic.svg'),
    join_paths(action_dir, 'plus-large-symbolic.svg'),
    join_paths(action_dir, 'user-trash-symbolic.svg'),
    join_paths(action_dir, 'chain-link-loose-symbolic.svg'),
    join_paths(action_dir, 'chain-link-symbolic.svg')
]

install_data(
  action_icons,
  install_dir: join_paths(get_option('datadir'), action_dir)
)


# Resources
resources = gnome.compile_resources(
    'resources',
    'resources.gresource.xml',
    gresource_bundle: true,
    source_dir: meson.current_build_dir(),
    install: true,
    install_dir: pkgdatadir,
    dependencies: blueprints,
)