project('converter',
  version: '1.5.2',
  meson_version: '>= 0.59.0',
  license: 'GPLv3',
  default_options: [ 'warning_level=2', 'werror=false', ],
)

application_name = 'Converter'
app_id = 'io.gitlab.adhami3310.Converter'
profile = get_option('profile')
app_version = meson.project_version()
app_icon = app_id

if profile == 'development'
  app_version += '-' + run_command(
    'git', 'rev-parse', '--short', 'HEAD'
  ).stdout().strip()
  application_name += ' (Development)'
  app_icon = app_id + '.' + 'Devel'  
endif

i18n = import('i18n')
gnome = import('gnome')

subdir('data')
subdir('converter')
subdir('po')

gnome.post_install(
     glib_compile_schemas: true,
    gtk_update_icon_cache: true,
  update_desktop_database: true,
)
