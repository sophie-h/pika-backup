project(
    run_command(
        find_program(meson.source_root() / 'build-aux/meson-cargo-manifest.py'),
        'package', 'name',
        check: true
    ).stdout(),
    'rust',
    version: run_command(
        find_program(meson.source_root() / 'build-aux/meson-cargo-manifest.py'),
        'package', 'version',
        check: true
    ).stdout()
)

i18n = import ('i18n')
gnome = import ('gnome')

dependency('gtk4', version: '>= 4.6.0')
dependency('libadwaita-1', version: '>= 1.1.alpha.0')
dependency('libsecret-1', version: '>= 0.18.0')

bindir = get_option('prefix') / get_option('bindir')
etcdir = get_option('prefix') / get_option('sysconfdir')
usrdir = get_option('prefix') / get_option('datadir')

meson.add_dist_script('build-aux/meson-dist-vendor.sh')

subdir('data')
subdir('help')
subdir('po')
subdir('src')
