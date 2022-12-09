# file_chooser.py: file chooser dialogs for opening and outputting files
#
# Copyright (C) 2022 Hari Rana / TheEvilSkeleton
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

from os.path import basename
from pathlib import PurePath, Path
from gi.repository import Gtk, Gio, GdkPixbuf, GLib, Gdk
import converter.filters
from gettext import gettext as _

class FileChooser:

    @staticmethod
    def __load_image_done(_obj, result, data):
        callback_good = data[0][0]
        callback_bad = data[0][1]
        input_file_path = data[1]

        try:
            pixbuf = GdkPixbuf.Pixbuf.new_from_stream_finish(result)
        except GLib.Error as error:
            print(f"Unable to load image, {error}")
            callback_bad(error)
            return

        callback_good(input_file_path, pixbuf)

    @staticmethod
    def __open_file_done(file, result, callbacks):
        callback_bad = callbacks[1]

        try:
            input_stream = file.read_finish(result)
        except GLib.Error as error:
            print(f"Unable to open file, {error}")
            callback_bad(error)
            return

        GdkPixbuf.Pixbuf.new_from_stream_async(input_stream,
                                               None,
                                               FileChooser.__load_image_done,
                                               (callbacks, file.get_path()))

    """ Run in a separate thread. """
    @staticmethod
    def load_file(file, callback_good, callback_error):
        print(f"Input file: {file.get_path()}")
        file.read_async(GLib.PRIORITY_DEFAULT,
                        None,
                        FileChooser.__open_file_done,
                        (callback_good, callback_error))


    """ Open and load file. """
    @staticmethod
    def open_file(parent, callback_good, callback_error, *args):
        def load_file(_dialog, response):

            """ Run if the user selects an image. """
            if response != Gtk.ResponseType.ACCEPT:
                callback_error(None)
                return

            file = dialog.get_file()
            FileChooser.load_file(file, callback_good, callback_error)

        dialog = Gtk.FileChooserNative.new(
            title=_('Select an image'),
            parent=parent,
            action=Gtk.FileChooserAction.OPEN
        )
        dialog.set_modal(True)
        dialog.connect('response', load_file)
        dialog.add_filter(converter.filters.get_file_filter(_("Supported image files"), converter.filters.supported_input_formats))
        dialog.show()

    """ Select output location. """
    @staticmethod
    def output_file(parent, default_name, format, default_folder, callback_good, callback_bad, *args):
        def convert_content(_dialog, response):

            """ Set output file path if user selects a location. """
            if response != Gtk.ResponseType.ACCEPT:
                callback_bad(None)
                return

            path = PurePath(dialog.get_file().get_path())

            """ Check if output file has a file extension or format is supported. """
            if '.' not in str(path.name):
                callback_bad(_('No file extension was specified'))
                return

            file_ext = str(path.suffix)[1:]

            if file_ext != format:
                callback_bad(_(f'’{file_ext}’ is of the wrong format'))
                return

            """ Set output path. """
            output_file_path = str(path)
            print(f'Output file: {output_file_path}')
            callback_good(output_file_path)

        dialog = Gtk.FileChooserNative.new(
            title=_('Select output location'),
            parent=parent,
            action=Gtk.FileChooserAction.SAVE
        )

        dialog.set_modal(True)
        dialog.connect('response', convert_content)
        dialog.add_filter(converter.filters.get_file_filter(format, [format]))
        dialog.set_current_name(default_name)
        if default_folder is not None: dialog.set_current_folder(Gio.File.new_for_path(default_folder))
        dialog.show()
