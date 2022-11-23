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
from converter.threading import RunAsync
from gi.repository import Adw, Gtk, Gio, GdkPixbuf, GLib
from converter.filters import get_format_filters, supported_filters, image_filters
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
                        return
                except AttributeError:
                    pass

                """ Run in a separate thread. """
                def run():

                    """ Declare variables. """
                    default_value = 4
                    self.input_file_path = dialog.get_file().get_path()

                    """ Confirm file is a valid image. """
                    try:
                        print(f'Input file: {self.input_file_path}')
                        image_file = GdkPixbuf.Pixbuf.new_from_file(self.input_file_path)
                    except GLib.GError:
                        print(f'Invalid image file path')
                        self.stack_converter.set_visible_child_name('stack_invalid_image')
                        return

                    self.image_size = GdkPixbuf.Pixbuf.get_file_info(self.input_file_path)

                    """ Display image. """
                    self.action_image_size.set_subtitle(f'{self.image_size[1]} × {self.image_size[2]}')
                    self.action_convert_image_size.set_subtitle(f'{self.image_size[1] * default_value} × {self.image_size[2] * default_value}')
                    self.image.set_pixbuf(image_file)

                    # except GLib.GError:
                        # Display video
                    #     self.video.set_filename(self.input_file_path)
                    #     self.video.set_visible(True)

                        # Display models
                    #     for model in self.model_videos:
                    #         self.string_models.append(model)

                    """ Reset widgets. """
                    # self.spin_scale.set_value(default_value)
                    self.label_output.set_label('(None)')
                    self.button_convert.set_sensitive(False)
                    self.button_convert.set_has_tooltip(True)
                    self.combo_models.set_selected(0)

                    self.stack_converter.set_visible_child_name('stack_convert')

                """ Run when run() function finishes. """
                def callback(*args):
                    self.spinner_loading.stop()

                """ Run functions asynchronously. """
                RunAsync(run, callback)
                self.stack_converter.set_visible_child_name('stack_loading')
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

    """ Select output location. """
    def output_file(self, *args):
        def convert_content(_dialog, response):

            """ Set output file path if user selects a location. """
            if response == -3:

                """ Get all filters. """
                filters = []
                for filter in get_format_filters('image'):
                    filters.append(filter.split('/').pop())

                """ Check if output file has a file extension or format is supported. """
                if '.' not in basename(dialog.get_file().get_path()):
                    self.toast.add_toast(Adw.Toast.new(_('No file extension was specified')))
                    return

                elif basename(dialog.get_file().get_path()).split('.').pop().lower() not in filters:
                    filename = basename(dialog.get_file().get_path()).split('.').pop()
                    self.toast.add_toast(Adw.Toast.new(_('’{}’ is an unsupported format'.format(filename))))
                    return

                """ Set output path. """
                self.output_file_path = dialog.get_file().get_path()
                print(f'Output file: {self.output_file_path}')

                """ Update widgets. """
                self.label_output.set_label(basename(self.output_file_path))
                self.button_convert.set_sensitive(True)
                self.button_convert.set_has_tooltip(False)

        dialog = Gtk.FileChooserNative.new(
            title=_('Select output location'),
            parent=self,
            action=Gtk.FileChooserAction.SAVE
        )
        dialog.set_modal(True)
        dialog.set_transient_for(self)
        dialog.connect('response', convert_content)
        dialog.add_filter(image_filters())
        dialog.set_current_name('.png')
        dialog.show()

