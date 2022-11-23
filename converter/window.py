# window.py: main window
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


import os
import subprocess
import re
from os.path import basename
from gi.repository import Adw, Gtk, GLib, Gdk
from converter.dialog_converting import ConvertingDialog
from converter.threading import RunAsync
from converter.file_chooser import FileChooser

@Gtk.Template(resource_path='/io/gitlab/adhami3310/Converter/gtk/window.ui')
class ConverterWindow(Adw.ApplicationWindow):
    __gtype_name__ = 'ConverterWindow'

    """ Declare child widgets. """
    toast = Gtk.Template.Child()
    stack_converter = Gtk.Template.Child()
    button_input = Gtk.Template.Child()
    action_image_size = Gtk.Template.Child()
    action_convert_image_size = Gtk.Template.Child()
    button_convert = Gtk.Template.Child()
    spinner_loading = Gtk.Template.Child()
    image = Gtk.Template.Child()
    # video = Gtk.Template.Child()
    # spin_scale = Gtk.Template.Child()
    button_output = Gtk.Template.Child()
    label_output = Gtk.Template.Child()

    """ Initialize function. """
    def __init__(self, **kwargs):
        super().__init__(**kwargs)

        """ Connect signals. """
        self.button_input.connect('clicked', self.__open_file)
        self.button_convert.connect('clicked', self.__convert)
        self.button_output.connect('clicked', self.__output_location)
        # self.spin_scale.connect('value-changed', self.__update_post_convert_image_size)

        """ Declare variables. """
        self.convert_dialog = None

    """ Open file and display it if the user selected it. """
    def __open_file(self, *args):
        FileChooser.open_file(self)

    """ Select output file location. """
    def __output_location(self, *args):
        FileChooser.output_file(self)

    """ Update progress. """
    def __convert_progress(self, progress):
        if self.convert_dialog:
            self.convert_dialog.set_progress(progress)

    def __convert(self, *args):

        """ Since GTK is not thread safe, prepare some data in the main thread. """
        self.convert_dialog = ConvertingDialog(self)

        """ Run in a separate thread. """
        def run():
            command = ['magick',
                       '-monitor',
                       self.input_file_path,
                       self.output_file_path,
                       ]
            process = subprocess.Popen(command, stderr=subprocess.PIPE, universal_newlines=True)
            print('Running: ', end='')
            print(*command)
            """ Read each line, query the percentage and update the progress bar. """
            for line in iter(process.stderr.readline, ''):
                print(line, end='')
                res = re.match('^(\d*.\d+)%$', line)
                if res:
                    GLib.idle_add(self.__convert_progress, float(res.group(1)))

        """ Run when run() function finishes. """
        def callback(*args):
            self.convert_dialog.close()
            self.convert_dialog = None
            self.converting_completed_dialog()

        """ Run functions asynchronously. """
        RunAsync(run, callback)
        self.convert_dialog.present()

    """ Ask the user if they want to open the file. """
    def converting_completed_dialog(self, *args):
        def response(_widget):
            path = f'file://{self.output_file_path}'
            Gtk.show_uri(self, path, Gdk.CURRENT_TIME)

        toast = Adw.Toast.new(_('Image convertd'))
        toast.set_button_label(_('Open'))
        toast.connect('button-clicked', response)
        self.toast.add_toast(toast)

    """ Update post-convert image size as the user adjusts the spinner. """
    # def __update_post_convert_image_size(self, *args):
    #     convert_image_size = [
    #         self.image_size[1] * int(self.spin_scale.get_value()),
    #         self.image_size[2] * int(self.spin_scale.get_value()),
    #     ]
    #     self.action_convert_image_size.set_subtitle(f'{convert_image_size[0]} × {convert_image_size[1]}')
