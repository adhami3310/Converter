pkgdatadir = join_paths(get_option('prefix'), get_option('datadir'), meson.project_name())
moduledir = join_paths(pkgdatadir, 'converter')
gnome = import('gnome')

blueprints = custom_target('blueprints',
  input: files(
    'gtk/help-overlay.blp',
    'gtk/dialog-converting.blp',
    'gtk/window.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('converter',
  'converter.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

python = import('python')

conf = configuration_data()
conf.set('PYTHON', python.find_installation('python3').path())
conf.set('VERSION', meson.project_version())
conf.set('localedir', join_paths(get_option('prefix'), get_option('localedir')))
conf.set('pkgdatadir', pkgdatadir)

configure_file(
  input: 'converter.in',
  output: 'converter',
  configuration: conf,
  install: true,
  install_dir: get_option('bindir')
)

converter_sources = [
  '__init__.py',
  'main.py',
  'window.py',
  'filters.py',
  'threading.py',
  'dialog_converting.py',
  'file_chooser.py',
]

install_data(converter_sources, install_dir: moduledir)
