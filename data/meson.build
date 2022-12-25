conf = configuration_data()
conf.set('application_name', application_name)
conf.set('app_id', app_id)
conf.set('app_icon', app_icon)


desktop_file = configure_file(
        input:  app_id + '.desktop.in',
       output:  app_id + '.desktop.i18n.in',
    configuration: conf)


desktop = i18n.merge_file(
       input:  desktop_file,
       output:  app_id + '.desktop',
         type: 'desktop',
       po_dir: '../po',
      install: true,
  install_dir: join_paths(get_option('datadir'), 'applications')
)



desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop])
endif

appstream_file = i18n.merge_file(
        input: app_id + '.metainfo.xml.in',
       output: app_id + '.metainfo.xml',
       po_dir: '../po',
      install: true,
  install_dir: join_paths(get_option('datadir'), 'metainfo')
)

appstream_util = find_program('appstream-util', required: false)
if appstream_util.found()
  test('Validate appstream file', appstream_util, args: ['validate', appstream_file])
endif

install_data(app_id + '.gschema.xml',
  install_dir: join_paths(get_option('datadir'), 'glib-2.0/schemas')
)

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file',
       compile_schemas,
       args: ['--strict', '--dry-run', meson.current_source_dir()])
endif

subdir('icons')
