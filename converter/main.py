# main.py: main application, base borrowed from TheEvilSkeleton/Upscaler
#
# Copyright (C) 2022 Khaleel Al-Adhami / adhami3310
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, version 3.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: GPL-3.0-only

import gi

gi.require_version('Gtk', '4.0')
gi.require_version('Adw', '1')

import sys
from gi.repository import Adw, Gtk, Gio, GLib
from .window import ConverterWindow
from gettext import gettext as _


class ConverterApplication(Adw.Application):
    """The main application singleton class."""

    def __init__(self):
        super().__init__(application_id='io.gitlab.adhami3310.Converter',
                         flags=Gio.ApplicationFlags.HANDLES_COMMAND_LINE)
        self.settings = Gio.Settings("io.gitlab.adhami3310.Converter")
        popular = Gio.SimpleAction.new_stateful(
            'popular', None, GLib.Variant('b', self.settings.get_boolean('show-less-popular'))
        )
        popular.connect('change-state', self.__on_popular)
        self.add_action(popular)
        self.set_accels_for_action('app.popular', ['<Primary>H'])
        self.create_action('quit', self.__quit, ['<primary>q'])
        self.create_action('about', self.__about_action)
        self.create_action('open', self.__open_file, ['<primary>o'])
        self.create_action('paste', self.__paste, ['<primary>v'])
        self.files = None

    def do_activate(self):
        """Called when the application is activated.

        We raise the application's main window, creating it if
        necessary.
        """
        self.win = self.props.active_window
        if not self.win:
            self.win = ConverterWindow(application=self)
        self.win.present()
        if self.files is not None:
            self.win.load_file(self.files)

    def do_command_line(self, command_line):
        args = command_line.get_arguments()
        if len(args) > 1:
            self.files = [command_line.create_file_for_arg(arg).get_path() for arg in args[1:]]
        self.activate()
        return 0

    def __paste(self, *args):
        self.win.load_cb()

    def __open_file(self, *args):
        self.win.open_file()

    def __on_popular(self, action, value, *args):
        action.props.state = value
        self.win.toggle_datatype()

    def __about_action(self, *args):
        """Callback for the app.about action."""
        about = Adw.AboutWindow(transient_for=self.props.active_window,
                                application_name='Converter',
                                application_icon='io.gitlab.adhami3310.Converter',
                                developer_name='Khaleel Al-Adhami',
                                version='1.5.0',
                                copyright='Copyright © 2022 Khaleel Al-Adhami',
                                license_type=Gtk.License.GPL_3_0_ONLY,
                                website='https://gitlab.com/adhami3310/Converter',
                                issue_url='https://gitlab.com/adhami3310/Converter/-/issues')
        about.set_translator_credits(translators())
        about.set_developers(developers())
        about.add_acknowledgement_section(
            _("Code and Design Borrowed from"),
            [
                "GIMP https://gitlab.gnome.org/GNOME/gimp",
                "Upscaler https://gitlab.com/TheEvilSkeleton/Upscaler",
                "Avvie https://github.com/Taiko2k/Avvie",
                "Bottles https://github.com/bottlesdevs/Bottles",
                "Loupe https://gitlab.gnome.org/BrainBlasted/loupe",
                "Totem https://gitlab.gnome.org/GNOME/totem",
            ]
        )
        about.add_acknowledgement_section(
            _("Sample Image from"),
            [
                "Samuel Custodio https://github.com/samuelcust/flappy-bird-assets"
            ]
        )
        about.add_legal_section(
            title='ImageMagick',
            copyright='Copyright © 2022 ImageMagick',
            license_type=Gtk.License.MIT_X11,
        )
        about.present()

    def create_action(self, name, callback, shortcuts=None):
        """Add an application action.

        Args:
            name: the name of the action
            callback: the function to be called when the action is
              activated
            shortcuts: an optional list of accelerators
        """
        action = Gio.SimpleAction.new(name, None)
        action.connect("activate", callback)
        self.add_action(action)
        if shortcuts:
            self.set_accels_for_action(f"app.{name}", shortcuts)

    """ Quit application. """
    def __quit(self, _args, *args):
        self.win.destroy()

def translators():
    """ Translators list. To add yourself into the list, add '\n', followed by
        your name/username, and optionally an email or URL:

        Name only:    \nKhaleel Al-Adhami
        Name + URL:   \nKhaleel Al-Adhami https://adhami3310.gitlab.io
        Name + Email: \nKhaleel Al-Adhami <khaleel.aladhami@gmail.com>
    """
    return _('Jürgen Benvenuti <gastornis@posteo.org>\nIrénée Thirion <irenee.thirion@e.email>\nMattia Borda <mattiagiovanni.borda@icloud.com>\nHeimen Stoffels\nSergio <sergiovg01@outlook.com>\nÅke Engelbrektson <eson@svenskasprakfiler.se>\nSabri Ünal <libreajans@gmail.com>\nAzat Zinnetullin')

def developers():
    """ Developers/Contributors list. If you have contributed code, feel free
        to add yourself into the Python list:

        Name only:    \nKhaleel Al-Adhami
        Name + URL:   \nKhaleel Al-Adhami https://adhami3310.gitlab.io
        Name + Email: \nKhaleel Al-Adhami <khaleel.aladhami@gmail.com>
    """
    return ['Khaleel Al-Adhami <khaleel.aladhami@gmail.com>']

def main(version):
    """The application's entry point."""
    app = ConverterApplication()
    return app.run(sys.argv)

