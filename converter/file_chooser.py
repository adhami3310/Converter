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
from pathlib import PurePath
from converter.threading import RunAsync
from gi.repository import Adw, Gtk, Gio, GdkPixbuf, GLib
from converter.filters import get_format_filters, supported_filters, image_filters, output_image_filters, set_formats_from_extensions, is_extenstion_output, extention_to_mime
from gettext import gettext as _

class FileChooser():

    """ Open and load file. """
    def open_file(self, *args):
        def load_file(_dialog, response):

            """ Run if the user selects an image. """
            if response == -3:

                """ Do nothing if opened image is the same as selected image. """

                try:
                    if self.input_file_path == dialog.get_file().get_path():
                        self.stack_converter.set_visible_child_name('stack_convert')
                        self.button_back.show()
                        return
                except AttributeError:
                    pass

                """ Run in a separate thread. """
                def run():

                    """ Declare variables. """
                    self.input_file_path = dialog.get_file().get_path()
                    """ Confirm file is a valid image. """
                    displayed = True
                    try:
                        print(f'Input file: {self.input_file_path}')
                        image_file = GdkPixbuf.Pixbuf.new_from_file(self.input_file_path)
                    except GLib.GError:
                        print(f'Invalid image file path')
                        self.stack_converter.set_visible_child_name('stack_invalid_image')
                        displayed = False
#                        return

                    if displayed:
                        self.image_size = GdkPixbuf.Pixbuf.get_file_info(self.input_file_path)

                        """ Display image. """
                        self.action_image_size.set_subtitle(f'{self.image_size[1]} × {self.image_size[2]}')
                        self.image.set_pixbuf(image_file)
                    else:
                        self.image.set_pixbuf(None)
                        self.action_image_size.set_subtitle('Unknown')
                        self.image_size = [0, '', '']
                    self.input_ext = str(PurePath(self.input_file_path).suffix)[1:]
                    self.action_image_type.set_subtitle(f'{self.input_ext.upper()} ({extention_to_mime[self.input_ext]})')
                    self.filetype.grab_focus()

                    """ Reset widgets. """
                    # self.spin_scale.set_value(default_value)
                    self.label_output.set_label('(None)')
                    self.button_convert.set_sensitive(False)
                    self.button_convert.set_has_tooltip(True)
                    self.resize_scale_height_value.set_text("100")
                    self.resize_scale_width_value.set_text("100")
                    self.ratio_width_value.set_text("1")
                    self.ratio_height_value.set_text("1")
                    self.resize_width_value.set_text(str(self.image_size[1]))
                    self.resize_height_value.set_text(str(self.image_size[1]))
                    self.resize_minmax_width_value.set_text(str(self.image_size[1]))
                    self.resize_minmax_height_value.set_text(str(self.image_size[1]))
                    self.filetype_changed()
                    self.stack_converter.set_visible_child_name('stack_convert')

                """ Run when run() function finishes. """
                def callback(*args):
                    self.spinner_loading.stop()

                """ Run functions asynchronously. """
                RunAsync(run, callback)
                self.stack_converter.set_visible_child_name('stack_loading')
                self.button_back.show()
                self.spinner_loading.start()

        dialog = Gtk.FileChooserNative.new(
            title=_('Select an image'),
            parent=self,
            action=Gtk.FileChooserAction.OPEN
        )
        dialog.set_modal(True)
        dialog.set_transient_for(self)
        dialog.connect('response', load_file)
        dialog.add_filter(supported_filters())
        dialog.show()

    def check_supported_output(self, ext):
        if not is_extenstion_output(ext):
            self.toast.add_toast(Adw.Toast.new(_('’{}’ is not a supported format'.format(ext))))
            return False
        return True

    """ Select output location. """
    def output_file(self, *args):

        ext = self.filetype.get_text()
        ext = ext[1:] if ext[0] == '.' else ext

        if not FileChooser.check_supported_output(self, ext):
            return

        def convert_content(_dialog, response):

            """ Set output file path if user selects a location. """
            if response == -3:

                path = PurePath(dialog.get_file().get_path())

                """ Check if output file has a file extension or format is supported. """
                if '.' not in str(path.name):
                    self.toast.add_toast(Adw.Toast.new(_('No file extension was specified')))
                    return

                file_ext = str(path.suffix)[1:]
                print(ext)
                if file_ext != ext:
                    self.toast.add_toast(Adw.Toast.new(_('’{}’ is of the wrong format'.format(file_ext))))
                    return

                """ Set output path. """
                self.output_file_path = str(path)
                print(f'Output file: {self.output_file_path}')

                """ Update widgets. """
                self.label_output.set_label(basename(self.output_file_path))
                self.button_convert.set_sensitive(True)
                self.button_convert.set_has_tooltip(False)
                self.button_convert.grab_focus()

        dialog = Gtk.FileChooserNative.new(
            title=_('Select output location'),
            parent=self,
            action=Gtk.FileChooserAction.SAVE
        )

        dialog.set_modal(True)
        dialog.set_transient_for(self)
        dialog.connect('response', convert_content)
        dialog.add_filter(set_formats_from_extensions([ext], ext))
        dialog.set_current_name(str(PurePath(self.input_file_path).with_suffix(f'.{ext}').name))
        dialog.show()

